use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Step {
    pub action: String,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyzeResult {
    pub steps: Vec<Step>,
}

const SYSTEM_PROMPT: &str = r#"You are a UI automation assistant.
Return ONLY valid JSON. No markdown, no explanation, no code fences.
Schema: {"steps":[{"action":"click"|"type"|"scroll"|"wait","target":"string","x":number|null,"y":number|null,"text":"string"|null,"duration_ms":number|null}]}"#;

pub async fn analyze(screenshot_b64: &str, task: &str) -> Result<AnalyzeResult> {
    let api_key = std::env::var("LLM_API_KEY")
        .map_err(|_| anyhow!("LLM_API_KEY env var not set"))?;
    let provider =
        std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".to_string());

    let mut last_err: Option<anyhow::Error> = None;
    for attempt in 0..3_u8 {
        if attempt > 0 {
            tracing::warn!("LLM JSON parse failed, retry {}/2", attempt);
        }
        match call_llm(&provider, &api_key, screenshot_b64, task).await {
            Ok(r) => return Ok(r),
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap())
}

async fn call_llm(
    provider: &str,
    api_key: &str,
    screenshot_b64: &str,
    task: &str,
) -> Result<AnalyzeResult> {
    let model = std::env::var("LLM_MODEL").unwrap_or_else(|_| match provider {
        "openai" => "gpt-4o".to_string(),
        _ => "claude-opus-4-5".to_string(),
    });

    let prompt = format!(
        "{}\n\nTask: {}",
        SYSTEM_PROMPT, task
    );

    let client = reqwest::Client::new();

    let (url, body) = match provider {
        "openai" => {
            let url = std::env::var("LLM_API_URL")
                .unwrap_or_else(|_| "https://api.openai.com/v1/chat/completions".to_string());
            let body = json!({
                "model": model,
                "max_tokens": 2048,
                "temperature": 0.1,
                "response_format": {"type": "json_object"},
                "messages": [{
                    "role": "user",
                    "content": [
                        {
                            "type": "image_url",
                            "image_url": {
                                "url": format!("data:image/png;base64,{}", screenshot_b64)
                            }
                        },
                        { "type": "text", "text": prompt }
                    ]
                }]
            });
            (url, body)
        }
        _ => {
            // Anthropic Claude
            let url = std::env::var("LLM_API_URL")
                .unwrap_or_else(|_| "https://api.anthropic.com/v1/messages".to_string());
            let body = json!({
                "model": model,
                "max_tokens": 2048,
                "temperature": 0.1,
                "messages": [{
                    "role": "user",
                    "content": [
                        {
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": "image/png",
                                "data": screenshot_b64
                            }
                        },
                        { "type": "text", "text": prompt }
                    ]
                }]
            });
            (url, body)
        }
    };

    let req = client.post(&url).json(&body);
    let req = match provider {
        "openai" => req.header("Authorization", format!("Bearer {}", api_key)),
        _ => req
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json"),
    };

    let resp = req.send().await?.error_for_status()?;
    let resp_json: serde_json::Value = resp.json().await?;

    let text = extract_text(provider, &resp_json)?;
    let trimmed = text.trim();

    serde_json::from_str::<AnalyzeResult>(trimmed).map_err(|e| {
        anyhow!(
            "JSON parse failed: {e} | response snippet: {}",
            &trimmed[..trimmed.len().min(300)]
        )
    })
}

fn extract_text(provider: &str, resp: &serde_json::Value) -> Result<String> {
    match provider {
        "openai" => resp["choices"][0]["message"]["content"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| anyhow!("missing content in OpenAI response")),
        _ => resp["content"][0]["text"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| anyhow!("missing text in Anthropic response")),
    }
}

// ── App matching via LLM ─────────────────────────────────────────────────────

/// Ask the LLM to match a natural-language query to the best app in `apps`.
/// Returns the full path of the matched app, or None if no match or LLM unavailable.
pub async fn llm_match_app(
    query: &str,
    apps: &[crate::apps::AppInfo],
) -> Result<Option<String>> {
    let Ok(api_key) = std::env::var("LLM_API_KEY") else {
        return Ok(None);
    };
    let provider =
        std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".to_string());

    let app_list = apps
        .iter()
        .map(|a| format!("- {} ({}): {}", a.name, a.fs_name, a.path))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        "Available apps:\n{app_list}\n\nUser wants to open: \"{query}\"\n\nReturn ONLY the full path of the best matching app. If no match, return \"none\". No explanation."
    );

    let path = call_text_llm(&provider, &api_key, &prompt).await?;
    let path = path.trim().trim_matches('"').to_string();

    if path.eq_ignore_ascii_case("none") || path.is_empty() {
        Ok(None)
    } else {
        Ok(Some(path))
    }
}

async fn call_text_llm(provider: &str, api_key: &str, prompt: &str) -> Result<String> {
    let model = std::env::var("LLM_MODEL").unwrap_or_else(|_| match provider {
        "openai" => "gpt-4o-mini".to_string(),
        _ => "claude-haiku-4-5-20251001".to_string(),
    });
    let client = reqwest::Client::new();

    let (url, body) = match provider {
        "openai" => {
            let url = std::env::var("LLM_API_URL")
                .unwrap_or_else(|_| "https://api.openai.com/v1/chat/completions".to_string());
            let body = json!({
                "model": model, "max_tokens": 256, "temperature": 0.0,
                "messages": [{"role": "user", "content": prompt}]
            });
            (url, body)
        }
        _ => {
            let url = std::env::var("LLM_API_URL")
                .unwrap_or_else(|_| "https://api.anthropic.com/v1/messages".to_string());
            let body = json!({
                "model": model, "max_tokens": 256, "temperature": 0.0,
                "messages": [{"role": "user", "content": prompt}]
            });
            (url, body)
        }
    };

    let req = client.post(&url).json(&body);
    let req = match provider {
        "openai" => req.header("Authorization", format!("Bearer {}", api_key)),
        _ => req
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json"),
    };

    let resp = req.send().await?.error_for_status()?;
    let resp_json: serde_json::Value = resp.json().await?;
    extract_text(provider, &resp_json)
}
