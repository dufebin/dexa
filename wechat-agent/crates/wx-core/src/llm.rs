use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::models::ContactProfile;

/// Calls vision-brain's `llm` subcommands as a subprocess.
/// LLM configuration (LLM_PROVIDER, LLM_API_KEY, LLM_MODEL, LLM_API_URL) is
/// read by vision-brain from its own environment — wechat-agent passes nothing.
pub struct VisionBrainClient {
    bin: PathBuf,
}

impl VisionBrainClient {
    pub fn new(bin: impl Into<PathBuf>) -> Self {
        Self { bin: bin.into() }
    }

    async fn call(&self, subcommand: &str, payload: serde_json::Value) -> Result<String> {
        let json_bytes = serde_json::to_vec(&payload)?;

        let mut child = Command::new(&self.bin)
            .args(["llm", subcommand])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .with_context(|| format!("failed to spawn vision-brain at {:?}", self.bin))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(&json_bytes)
                .await
                .context("failed to write to vision-brain stdin")?;
        }

        let output = child
            .wait_with_output()
            .await
            .context("vision-brain process error")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("vision-brain llm {} failed: {}", subcommand, stderr);
        }

        Ok(String::from_utf8(output.stdout)
            .context("vision-brain output is not valid UTF-8")?
            .trim()
            .to_string())
    }

    /// Generate a reply for an incoming WeChat message.
    pub async fn generate_reply(
        &self,
        incoming: &str,
        sender: &str,
        history_text: &str,
        profile: Option<&ContactProfile>,
        max_len: usize,
    ) -> Result<String> {
        let profile_json = profile
            .and_then(|p| serde_json::to_string(p).ok());
        let payload = serde_json::json!({
            "sender":   sender,
            "content":  incoming,
            "history":  history_text,
            "profile":  profile_json,
            "max_len":  max_len,
        });
        self.call("generate-reply", payload).await
    }

    /// Distill a contact's messages into a JSON profile string.
    pub async fn distill_contact(
        &self,
        contact_name: &str,
        messages_text: &str,
    ) -> Result<String> {
        let payload = serde_json::json!({
            "contact":  contact_name,
            "messages": messages_text,
        });
        self.call("distill-contact", payload).await
    }

    /// Distill the user's own messages into a persona Markdown string.
    pub async fn distill_self(&self, my_messages_text: &str) -> Result<String> {
        let payload = serde_json::json!({
            "messages": my_messages_text,
        });
        self.call("distill-self", payload).await
    }
}
