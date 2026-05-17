use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::process::Command;

pub struct HandClient {
    pub bin: PathBuf,
}

impl HandClient {
    pub fn new(bin: impl Into<PathBuf>) -> Self {
        Self { bin: bin.into() }
    }

    async fn run(&self, args: &[&str]) -> Result<()> {
        let output = Command::new(&self.bin)
            .args(args)
            .output()
            .await
            .with_context(|| format!("failed to spawn: hand {}", args.join(" ")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("hand {} failed: {stderr}", args.join(" "));
        }
        Ok(())
    }

    /// Press a key combination, e.g. "cmd+f", "ctrl+f", "ctrl+a".
    pub async fn key_combo(&self, keys: &str) -> Result<()> {
        self.run(&["key", "combo", "--keys", keys]).await
    }

    /// Type text. `human = true` uses realistic per-character delays.
    pub async fn key_type(&self, text: &str, human: bool) -> Result<()> {
        let mode = if human { "human" } else { "fast" };
        self.run(&["key", "type", "--text", text, "--mode", mode])
            .await
    }

    /// Tap a single key by name: "return", "escape", "down", "tab", etc.
    pub async fn key_tap(&self, key: &str) -> Result<()> {
        self.run(&["key", "tap", "--key", key]).await
    }

    /// Paste text via clipboard + Ctrl+V (works for CJK/Unicode text).
    pub async fn key_paste(&self, text: &str) -> Result<()> {
        self.run(&["key", "paste", "--text", text]).await
    }

    /// Left-click at (x, y).
    pub async fn mouse_click(&self, x: i32, y: i32) -> Result<()> {
        self.run(&[
            "mouse", "click",
            "--x", &x.to_string(),
            "--y", &y.to_string(),
        ])
        .await
    }

    /// Get current mouse position.
    pub async fn mouse_pos(&self) -> Result<(i32, i32)> {
        let output = Command::new(&self.bin)
            .args(["mouse", "pos"])
            .output()
            .await
            .context("failed to spawn: hand mouse pos")?;

        #[derive(serde::Deserialize)]
        struct Pos {
            x: i32,
            y: i32,
        }
        let pos: Pos = serde_json::from_slice(&output.stdout)
            .context("failed to parse hand mouse pos output")?;
        Ok((pos.x, pos.y))
    }
}
