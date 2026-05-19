use anyhow::Result;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler, ServiceExt,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::service::Service;

// ── param structs ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ScreenAnalyzeParams {
    pub task: String,
    pub screenshot: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MemoryKeyParams {
    pub key: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MemorySetParams {
    pub key: String,
    pub steps: serde_json::Value,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AppOpenParams {
    /// Natural language description of the app to open, e.g. "打开微信", "open Chrome"
    pub query: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LlmDistillContactParams {
    pub contact: String,
    pub messages: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LlmGenerateReplyParams {
    pub sender: String,
    pub content: String,
    pub history: String,
    pub profile: Option<String>,
    pub max_len: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LlmDistillSelfParams {
    pub messages: String,
}

// ── server ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct VisionServer {
    service: Arc<Service>,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl VisionServer {
    pub fn new(service: Arc<Service>) -> Self {
        Self {
            service,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl VisionServer {
    #[tool(description = "Capture the primary screen. Returns {\"base64\": string}")]
    async fn screen_capture(&self) -> Result<String, McpError> {
        self.service
            .screen_capture()
            .await
            .map(|v| v.to_string())
            .map_err(mcp_err)
    }

    #[tool(
        description = "Analyze a screen and return action steps. task = goal description. screenshot = optional base64 PNG (captures if omitted)."
    )]
    async fn screen_analyze(
        &self,
        Parameters(p): Parameters<ScreenAnalyzeParams>,
    ) -> Result<String, McpError> {
        self.service
            .screen_analyze(&p.task, p.screenshot.as_deref())
            .await
            .map(|v| v.to_string())
            .map_err(mcp_err)
    }

    #[tool(
        description = "Get stored action steps by key. Increments hit count. Returns null if not found."
    )]
    async fn memory_get(
        &self,
        Parameters(p): Parameters<MemoryKeyParams>,
    ) -> Result<String, McpError> {
        self.service
            .memory_get(p.key)
            .await
            .map(|v| v.to_string())
            .map_err(mcp_err)
    }

    #[tool(description = "Store action steps under a key. Upserts on conflict.")]
    async fn memory_set(
        &self,
        Parameters(p): Parameters<MemorySetParams>,
    ) -> Result<String, McpError> {
        self.service
            .memory_set(p.key, p.steps)
            .await
            .map(|v| v.to_string())
            .map_err(mcp_err)
    }

    #[tool(description = "Delete stored steps by key. Returns {\"ok\": bool}.")]
    async fn memory_delete(
        &self,
        Parameters(p): Parameters<MemoryKeyParams>,
    ) -> Result<String, McpError> {
        self.service
            .memory_delete(p.key)
            .await
            .map(|v| v.to_string())
            .map_err(mcp_err)
    }

    #[tool(description = "List all memory keys with hit counts and timestamps.")]
    async fn memory_list(&self) -> Result<String, McpError> {
        self.service
            .memory_list()
            .await
            .map(|v| v.to_string())
            .map_err(mcp_err)
    }

    #[tool(description = "List all installed applications on this computer. Returns [{name, fs_name, path}].")]
    async fn app_list(&self) -> Result<String, McpError> {
        self.service
            .app_list()
            .await
            .map(|v| v.to_string())
            .map_err(mcp_err)
    }

    #[tool(
        description = "Open an application by natural language. Examples: '打开微信', 'open Chrome', 'launch calculator'. Tries fuzzy match first, falls back to LLM. Returns {ok, app, method}."
    )]
    async fn app_open(
        &self,
        Parameters(p): Parameters<AppOpenParams>,
    ) -> Result<String, McpError> {
        self.service
            .app_open(p.query)
            .await
            .map(|v| v.to_string())
            .map_err(mcp_err)
    }

    #[tool(
        description = "Distill a WeChat contact's chat history into a structured JSON profile. Returns {summary, communication_style, topics, emotional_pattern, relationship, response_strategy}."
    )]
    async fn llm_distill_contact(
        &self,
        Parameters(p): Parameters<LlmDistillContactParams>,
    ) -> Result<String, McpError> {
        self.service
            .llm_distill_contact(&p.contact, &p.messages)
            .await
            .map(|v| v.to_string())
            .map_err(mcp_err)
    }

    #[tool(
        description = "Generate a WeChat reply for an incoming message using the contact's profile. Returns {\"reply\": string}."
    )]
    async fn llm_generate_reply(
        &self,
        Parameters(p): Parameters<LlmGenerateReplyParams>,
    ) -> Result<String, McpError> {
        self.service
            .llm_generate_reply(
                &p.sender,
                &p.content,
                &p.history,
                p.profile.as_deref(),
                p.max_len.unwrap_or(80) as usize,
            )
            .await
            .map(|v| v.to_string())
            .map_err(mcp_err)
    }

    #[tool(
        description = "Distill the user's own WeChat messages into a Self Memory + Persona Markdown document. Returns {\"markdown\": string}."
    )]
    async fn llm_distill_self(
        &self,
        Parameters(p): Parameters<LlmDistillSelfParams>,
    ) -> Result<String, McpError> {
        self.service
            .llm_distill_self(&p.messages)
            .await
            .map(|v| v.to_string())
            .map_err(mcp_err)
    }
}

#[tool_handler]
impl ServerHandler for VisionServer {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.instructions =
            Some("Screen perception, UI analysis, and persistent action memory".into());
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info
    }
}

pub async fn run_mcp_server(service: Arc<Service>) -> Result<()> {
    use rmcp::transport::stdio;
    let server = VisionServer::new(service);
    let handle = server.serve(stdio()).await?;
    handle.waiting().await?;
    Ok(())
}

fn mcp_err(e: impl std::fmt::Display) -> McpError {
    McpError::internal_error(e.to_string(), None)
}
