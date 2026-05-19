use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::process::Command;

use crate::models::{WxContact, WxMessage, WxSession};

pub struct WxClient {
    pub bin: PathBuf,
}

impl WxClient {
    pub fn new(bin: impl Into<PathBuf>) -> Self {
        Self { bin: bin.into() }
    }

    async fn run_json<T>(&self, args: &[&str]) -> Result<T>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        let output = Command::new(&self.bin)
            .args(args)
            .output()
            .await
            .with_context(|| format!("failed to spawn: wx {}", args.join(" ")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("wx {} failed: {stderr}", args.join(" "));
        }

        serde_json::from_slice(&output.stdout)
            .with_context(|| format!("parse error for: wx {}", args.join(" ")))
    }

    pub async fn get_local_user(&self) -> String {
        if let Ok(msgs) = self
            .run_json::<Vec<WxMessage>>(&["history", "文件传输助手", "-n", "10", "--format", "json"])
            .await
        {
            for m in msgs {
                if !m.sender.is_empty() {
                    return m.sender;
                }
            }
        }
        String::new()
    }

    async fn post_process(&self, mut msgs: Vec<WxMessage>) -> Vec<WxMessage> {
        let local_user = self.get_local_user().await;
        if local_user.is_empty() {
            return msgs;
        }
        for m in &mut msgs {
            if m.sender == local_user {
                m.is_self = true;
            }
        }
        msgs
    }

    /// New messages since the daemon's last check (incremental).
    pub async fn new_messages(&self) -> Result<Vec<WxMessage>> {
        let msgs: Vec<WxMessage> = self.run_json(&["new-messages", "--format", "json"]).await?;
        Ok(self.post_process(msgs).await)
    }

    /// Recent message history for a specific contact or group.
    pub async fn history(&self, contact: &str, limit: usize) -> Result<Vec<WxMessage>> {
        let n = limit.to_string();
        let msgs: Vec<WxMessage> = self
            .run_json(&["history", contact, "-n", &n, "--format", "json"])
            .await?;
        Ok(self.post_process(msgs).await)
    }

    /// Export all messages for a contact (for distillation).
    pub async fn export_all(&self, contact: &str) -> Result<Vec<WxMessage>> {
        let msgs: Vec<WxMessage> = self
            .run_json(&["export", contact, "--format", "json"])
            .await?;
        Ok(self.post_process(msgs).await)
    }

    /// Export messages with a count limit.
    pub async fn export_n(&self, contact: &str, n: usize) -> Result<Vec<WxMessage>> {
        let count = n.to_string();
        let msgs: Vec<WxMessage> = self
            .run_json(&["export", contact, "-n", &count, "--format", "json"])
            .await?;
        Ok(self.post_process(msgs).await)
    }

    /// Export messages since a date (YYYY-MM-DD).
    pub async fn export_since(&self, contact: &str, since: &str) -> Result<Vec<WxMessage>> {
        let msgs: Vec<WxMessage> = self
            .run_json(&["export", contact, "--since", since, "--format", "json"])
            .await?;
        Ok(self.post_process(msgs).await)
    }

    pub async fn sessions(&self) -> Result<Vec<WxSession>> {
        self.run_json(&["sessions", "--format", "json"]).await
    }

    pub async fn contacts(&self) -> Result<Vec<WxContact>> {
        self.run_json(&["contacts", "--format", "json"]).await
    }
}
