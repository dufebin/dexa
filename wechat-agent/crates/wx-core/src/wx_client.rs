use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::process::Command;

use crate::models::{WxContact, WxMessage, WxSession};

pub struct WxClient {
    pub bin: PathBuf,
}

#[derive(serde::Deserialize)]
struct ExportWrapper {
    #[serde(default)]
    messages: Vec<WxMessage>,
}

impl WxClient {
    pub fn new(bin: impl Into<PathBuf>) -> Self {
        Self { bin: bin.into() }
    }

    async fn run_json<T>(&self, args: &[&str]) -> Result<T>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        let mut all_args: Vec<&str> = args.to_vec();
        all_args.push("--json");

        let output = Command::new(&self.bin)
            .args(&all_args)
            .output()
            .await
            .with_context(|| format!("failed to spawn: wx {}", all_args.join(" ")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("wx {} failed: {stderr}", all_args.join(" "));
        }

        // wx-cli may return an empty array if there is nothing to report;
        // parse normally and let the caller decide what to do.
        serde_json::from_slice(&output.stdout)
            .with_context(|| format!("parse error for: wx {}", all_args.join(" ")))
    }

    pub async fn get_local_user(&self) -> String {
        if let Ok(msgs) = self.run_json::<Vec<WxMessage>>(&["history", "文件传输助手", "--limit", "10"]).await {
            for m in msgs {
                if !m.sender.is_empty() {
                    return m.sender;
                }
            }
        }
        "王彬℘࿐".to_string()
    }

    async fn post_process(&self, mut msgs: Vec<WxMessage>) -> Vec<WxMessage> {
        let local_user = self.get_local_user().await;
        for m in &mut msgs {
            if m.sender == local_user {
                m.is_self = true;
            }
        }
        msgs
    }

    /// New messages since the daemon's last check (incremental).
    pub async fn new_messages(&self) -> Result<Vec<WxMessage>> {
        let msgs: Vec<WxMessage> = self.run_json(&["new-messages"]).await?;
        Ok(self.post_process(msgs).await)
    }

    /// Recent message history for a specific contact or group.
    pub async fn history(&self, contact: &str, limit: usize) -> Result<Vec<WxMessage>> {
        let limit_str = limit.to_string();
        let msgs: Vec<WxMessage> = self.run_json(&["history", contact, "--limit", &limit_str]).await?;
        Ok(self.post_process(msgs).await)
    }

    /// Export all messages for a contact (for distillation).
    /// Falls back to a large history if `export` subcommand is unavailable.
    pub async fn export_all(&self, contact: &str) -> Result<Vec<WxMessage>> {
        let msgs = match self.run_json::<ExportWrapper>(&["export", contact, "--format", "json"]).await {
            Ok(wrapper) => wrapper.messages,
            Err(_) => match self.run_json::<Vec<WxMessage>>(&["export", contact, "--format", "json"]).await {
                Ok(msgs) => msgs,
                Err(_) => self.run_json::<Vec<WxMessage>>(&["history", contact, "--limit", "2000"]).await?,
            },
        };
        Ok(self.post_process(msgs).await)
    }

    pub async fn sessions(&self) -> Result<Vec<WxSession>> {
        self.run_json(&["sessions"]).await
    }

    pub async fn contacts(&self) -> Result<Vec<WxContact>> {
        self.run_json(&["contacts"]).await
    }
}
