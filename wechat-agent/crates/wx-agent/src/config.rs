use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub binaries: BinariesConfig,
    pub claude: ClaudeConfig,
    #[serde(default)]
    pub agent: AgentConfig,
    #[serde(default)]
    pub wechat: WechatConfig,
}

#[derive(Debug, Deserialize)]
pub struct BinariesConfig {
    #[serde(default = "default_wx")]
    pub wx: String,
    #[serde(default = "default_hand")]
    pub hand: String,
}

impl Default for BinariesConfig {
    fn default() -> Self {
        Self { wx: default_wx(), hand: default_hand() }
    }
}

fn default_wx() -> String   { "wx".into() }
fn default_hand() -> String { "hand".into() }

#[derive(Debug, Deserialize)]
pub struct ClaudeConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_reply_model")]
    pub reply_model: String,
    #[serde(default = "default_distill_model")]
    pub distill_model: String,
}

fn default_reply_model() -> String   { "claude-haiku-4-5-20251001".into() }
fn default_distill_model() -> String { "claude-sonnet-4-6".into() }

#[derive(Debug, Deserialize)]
pub struct AgentConfig {
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_poll")]
    pub poll_interval: u64,
    #[serde(default = "default_max_len")]
    pub reply_max_len: usize,
    #[serde(default = "default_true")]
    pub require_profile: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            poll_interval: default_poll(),
            reply_max_len: default_max_len(),
            require_profile: true,
        }
    }
}

fn default_mode() -> String { "semi".into() }
fn default_poll() -> u64    { 5 }
fn default_max_len() -> usize { 80 }
fn default_true() -> bool   { true }

#[derive(Debug, Default, Deserialize)]
pub struct WechatConfig {
    /// Override the OS default command to bring WeChat to the foreground.
    pub activate_cmd: Option<String>,
    /// Override the OS default search shortcut (e.g. "cmd+f").
    pub search_key: Option<String>,
}

impl Config {
    /// Load from `./config.toml`, falling back to `~/.wx-agent/config.toml`.
    pub fn load() -> Result<Self> {
        let candidates: Vec<PathBuf> = vec![
            PathBuf::from("config.toml"),
            dirs::home_dir()
                .unwrap_or_default()
                .join(".wx-agent")
                .join("config.toml"),
        ];

        for path in &candidates {
            if path.exists() {
                let raw = std::fs::read_to_string(path)
                    .with_context(|| format!("failed to read {:?}", path))?;
                let mut cfg: Config = toml::from_str(&raw)
                    .with_context(|| format!("failed to parse {:?}", path))?;
                // Environment variable overrides the config file API key.
                if let Ok(key) = std::env::var("WX_AGENT_API_KEY") {
                    cfg.claude.api_key = key;
                }
                return Ok(cfg);
            }
        }

        anyhow::bail!(
            "config.toml not found. Create one at ./config.toml or ~/.wx-agent/config.toml"
        )
    }

    pub fn wx_bin(&self)   -> PathBuf { PathBuf::from(&self.binaries.wx) }
    pub fn hand_bin(&self) -> PathBuf { PathBuf::from(&self.binaries.hand) }

    pub fn wechat_search_key(&self) -> &str {
        self.wechat.search_key.as_deref().unwrap_or_else(|| {
            #[cfg(target_os = "macos")]    { "cmd+f" }
            #[cfg(target_os = "windows")]  { "ctrl+f" }
            #[cfg(not(any(target_os = "macos", target_os = "windows")))] { "ctrl+f" }
        })
    }
}

pub fn default_db_path() -> String {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".wx-agent")
        .join("data.db")
        .to_string_lossy()
        .into_owned()
}

pub fn default_skill_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude")
        .join("skills")
        .join("wechat-self")
        .join("SKILL.md")
}
