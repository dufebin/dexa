use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::models::ContactProfile;

#[derive(Clone)]
pub struct LlmClient {
    client: Client,
    api_key: String,
    pub reply_model: String,
    pub distill_model: String,
}

// ── Claude API wire types ────────────────────────────────────────────────────

#[derive(Serialize)]
struct ApiRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<&'a str>,
    messages: Vec<ApiMessage<'a>>,
}

#[derive(Serialize)]
struct ApiMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ApiResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────

impl LlmClient {
    pub fn new(
        api_key: impl Into<String>,
        reply_model: impl Into<String>,
        distill_model: impl Into<String>,
    ) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
            reply_model: reply_model.into(),
            distill_model: distill_model.into(),
        }
    }

    async fn call(
        &self,
        model: &str,
        system: Option<&str>,
        user_prompt: &str,
        max_tokens: u32,
    ) -> Result<String> {
        let req = ApiRequest {
            model,
            max_tokens,
            system,
            messages: vec![ApiMessage {
                role: "user",
                content: user_prompt,
            }],
        };

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&req)
            .send()
            .await
            .context("Claude API request failed")?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Claude API {status}: {body}");
        }

        let api_resp: ApiResponse = resp.json().await.context("failed to parse Claude response")?;
        api_resp
            .content
            .into_iter()
            .find(|b| b.kind == "text")
            .and_then(|b| b.text)
            .context("Claude returned no text content")
    }

    /// Generate a reply for an incoming message.
    pub async fn generate_reply(
        &self,
        incoming: &str,
        sender: &str,
        history_text: &str,
        profile: Option<&ContactProfile>,
        max_len: usize,
    ) -> Result<String> {
        let profile_section = profile.map(|p| {
            format!(
                "关于 {sender} 的了解：\n- 关系：{}\n- 沟通风格：{}\n- 常见话题：{}\n- 回复策略：{}",
                p.relationship, p.communication_style,
                p.topics.join("、"), p.response_strategy
            )
        });

        let system = "你正在帮助用户回复微信消息。请模仿用户本人的自然说话风格，给出简短真实的回复。只输出回复内容本身，不要加引号、解释或前缀。";

        let user_prompt = format!(
            "{profile_block}\n\n最近对话（从旧到新）：\n{history}\n\n{sender} 刚发来：「{incoming}」\n\n请用中文回复，不超过 {max_len} 个字。",
            profile_block = profile_section.as_deref().unwrap_or("（暂无该联系人画像）"),
            history = history_text,
        );

        self.call(&self.reply_model.clone(), Some(system), &user_prompt, 256)
            .await
    }

    /// Distill a contact's messages into a structured JSON profile.
    /// Returns a JSON string matching `ContactProfile` (minus contact_name/updated_at).
    pub async fn distill_contact(
        &self,
        contact_name: &str,
        messages_text: &str,
    ) -> Result<String> {
        let system = "你是一个专业的人格分析专家。请根据聊天记录分析联系人特征，严格输出 JSON，不要有任何额外文字。";

        let user_prompt = format!(
            r#"分析「{contact_name}」的聊天记录，输出以下 JSON 格式：
{{
  "summary": "一句话概括这个人",
  "communication_style": "沟通风格：casual|formal|mixed",
  "topics": ["话题1", "话题2", ...（最多10个）],
  "emotional_pattern": "情感模式描述（1-2句）",
  "relationship": "关系类型：close_friend|colleague|acquaintance|family|other",
  "response_strategy": "与此人对话的建议策略（1-2句）"
}}

聊天记录（{contact_name} 发出的消息，时间从旧到新）：
{messages_text}"#
        );

        self.call(&self.distill_model.clone(), Some(system), &user_prompt, 1024)
            .await
    }

    /// Distill the user's own messages into a Self Memory + Persona markdown.
    pub async fn distill_self(&self, my_messages_text: &str) -> Result<String> {
        let system = "你是一个专业的个人档案分析师。请根据用户自己发出的消息，蒸馏出用户的自我画像，以 Markdown 格式输出。";

        let user_prompt = format!(
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
{my_messages_text}"#
        );

        self.call(&self.distill_model.clone(), Some(system), &user_prompt, 4096)
            .await
    }
}
