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

// ── Text-only LLM helper ──────────────────────────────────────────────────────

async fn call_text_llm_full(
    provider: &str,
    api_key: &str,
    system: Option<&str>,
    prompt: &str,
    max_tokens: u32,
) -> Result<String> {
    let model = std::env::var("LLM_MODEL").unwrap_or_else(|_| match provider {
        "openai" => "gpt-4o-mini".to_string(),
        _ => "claude-haiku-4-5-20251001".to_string(),
    });
    let client = reqwest::Client::new();

    let (url, body) = match provider {
        "openai" => {
            let url = std::env::var("LLM_API_URL")
                .unwrap_or_else(|_| "https://api.openai.com/v1/chat/completions".to_string());
            let mut messages = Vec::new();
            if let Some(sys) = system {
                messages.push(json!({"role": "system", "content": sys}));
            }
            messages.push(json!({"role": "user", "content": prompt}));
            let body = json!({
                "model": model,
                "max_tokens": max_tokens,
                "temperature": 0.0,
                "messages": messages,
            });
            (url, body)
        }
        _ => {
            let url = std::env::var("LLM_API_URL")
                .unwrap_or_else(|_| "https://api.anthropic.com/v1/messages".to_string());
            let mut body_obj = json!({
                "model": model,
                "max_tokens": max_tokens,
                "temperature": 0.0,
                "messages": [{"role": "user", "content": prompt}],
            });
            if let Some(sys) = system {
                body_obj["system"] = json!(sys);
            }
            (url, body_obj)
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

async fn call_text_llm(provider: &str, api_key: &str, prompt: &str) -> Result<String> {
    call_text_llm_full(provider, api_key, None, prompt, 256).await
}

// ── WeChat LLM operations ─────────────────────────────────────────────────────

/// Distill a contact's chat history into a structured JSON profile string.
pub async fn distill_contact(contact: &str, messages: &str) -> Result<String> {
    let api_key = std::env::var("LLM_API_KEY")
        .map_err(|_| anyhow!("LLM_API_KEY env var not set"))?;
    let provider = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".to_string());

    let system = "你是一个专业的人格分析专家。请根据聊天记录分析联系人特征，严格输出 JSON，不要有任何额外文字。";
    let prompt = format!(
        r#"分析「{contact}」的聊天记录，输出以下 JSON 格式：
{{
  "summary": "一句话概括这个人",
  "communication_style": "沟通风格：casual|formal|mixed",
  "topics": ["话题1", "话题2", ...（最多10个）],
  "emotional_pattern": "情感模式描述（1-2句）",
  "relationship": "关系类型：close_friend|colleague|acquaintance|family|other",
  "response_strategy": "与此人对话的建议策略（1-2句）"
}}

聊天记录（时间从旧到新）：
{messages}"#
    );

    call_text_llm_full(&provider, &api_key, Some(system), &prompt, 1024).await
}

/// Generate a WeChat reply for an incoming message.
/// `profile_json` is an optional serialized ContactProfile JSON string.
pub async fn generate_reply(
    sender: &str,
    content: &str,
    history: &str,
    profile_json: Option<&str>,
    max_len: usize,
) -> Result<String> {
    let api_key = std::env::var("LLM_API_KEY")
        .map_err(|_| anyhow!("LLM_API_KEY env var not set"))?;
    let provider = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".to_string());

    let profile_block = profile_json
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        .map(|val| {
            let rel      = val["relationship"].as_str().unwrap_or("");
            let style    = val["communication_style"].as_str().unwrap_or("");
            let strategy = val["response_strategy"].as_str().unwrap_or("");
            format!(
                "关于 {sender} 的了解：\n- 关系：{rel}\n- 沟通风格：{style}\n- 回复策略：{strategy}"
            )
        })
        .unwrap_or_else(|| "（暂无该联系人画像）".to_string());

    let system = "你正在帮助用户回复微信消息。请模仿用户本人的自然说话风格，给出简短真实的回复。只输出回复内容本身，不要加引号、解释或前缀。";
    let prompt = format!(
        "{profile_block}\n\n最近对话（从旧到新）：\n{history}\n\n{sender} 刚发来：「{content}」\n\n请用中文回复，不超过 {max_len} 个字。"
    );

    call_text_llm_full(&provider, &api_key, Some(system), &prompt, 256).await
}

/// Distill the user's own messages into a Self Memory + Persona Markdown document.
pub async fn distill_self(messages: &str) -> Result<String> {
    let api_key = std::env::var("LLM_API_KEY")
        .map_err(|_| anyhow!("LLM_API_KEY env var not set"))?;
    let provider = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".to_string());

    let system = "你是一个专业的个人档案分析师。请根据用户自己发出的消息，蒸馏出用户的自我画像，以 Markdown 格式输出。";
    let prompt = format!(
        r#"以下是用户自己发出的微信消息（时间从旧到新）。请分析并生成：

## Part A — Self Memory（自我记忆）
核心经历、价值观、生活习惯、重要偏好、常去的地方等。

## Part B — Persona（人格模型）
- 性格特征（3-5 条）
- 说话风格（口头禅、句式偏好、常用表达）
- 情感模式（高兴/难过/愤怒时的表达方式）
- 决策风格

---
用户消息如下：
{messages}"#
    );

    call_text_llm_full(&provider, &api_key, Some(system), &prompt, 4096).await
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
