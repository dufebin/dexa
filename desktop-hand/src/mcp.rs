use anyhow::Result;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler, ServiceExt,
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::service::Service;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MouseMoveParams {
    pub x: i32,
    pub y: i32,
    pub ms: Option<u64>,
    pub mode: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MouseClickParams {
    pub x: i32,
    pub y: i32,
    pub button: Option<String>,
    pub double: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MouseDragParams {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
    pub ms: Option<u64>,
    pub mode: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MouseScrollParams {
    pub delta: i32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct KeyTypeParams {
    pub text: String,
    pub mode: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct KeyTapParams {
    pub key: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct KeyComboParams {
    pub keys: String,
}

#[derive(Clone)]
pub struct HandServer {
    service: Arc<Mutex<Service>>,
    action_lock: Arc<Mutex<()>>,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

// `new()` must be in a separate impl block — NOT inside #[tool_router].
impl HandServer {
    pub fn new() -> Self {
        Self {
            service: Arc::new(Mutex::new(Service::new())),
            action_lock: Arc::new(Mutex::new(())),
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl HandServer {
    #[tool(description = "Move mouse to (x, y). ms = duration ms, mode = human|fast")]
    async fn mouse_move(
        &self,
        Parameters(p): Parameters<MouseMoveParams>,
    ) -> Result<String, McpError> {
        let mode = p
            .mode
            .as_deref()
            .unwrap_or("human")
            .parse()
            .map_err(mcp_err)?;
        let executor = self.service.lock().await.executor();
        let _action = self.action_lock.lock().await;
        let from = executor.mouse_pos().await.map_err(mcp_err)?;
        let waypoints =
            self.service
                .lock()
                .await
                .plan_mouse_move(from, p.x, p.y, p.ms.unwrap_or(300), mode);
        executor
            .run_path(waypoints)
            .await
            .map(|_| "ok".to_string())
            .map_err(mcp_err)
    }

    #[tool(description = "Click mouse at (x, y). button = left|right, double = true|false")]
    async fn mouse_click(
        &self,
        Parameters(p): Parameters<MouseClickParams>,
    ) -> Result<String, McpError> {
        let executor = self.service.lock().await.executor();
        let button =
            crate::service::parse_button(p.button.as_deref().unwrap_or("left")).map_err(mcp_err)?;
        let _action = self.action_lock.lock().await;
        executor
            .click(p.x, p.y, button, p.double.unwrap_or(false))
            .await
            .map(|_| "ok".to_string())
            .map_err(mcp_err)
    }

    #[tool(description = "Drag mouse from (x1,y1) to (x2,y2). ms = duration, mode = human|fast")]
    async fn mouse_drag(
        &self,
        Parameters(p): Parameters<MouseDragParams>,
    ) -> Result<String, McpError> {
        let mode = p
            .mode
            .as_deref()
            .unwrap_or("human")
            .parse()
            .map_err(mcp_err)?;
        let executor = self.service.lock().await.executor();
        let waypoints = self.service.lock().await.plan_mouse_drag(
            p.x1,
            p.y1,
            p.x2,
            p.y2,
            p.ms.unwrap_or(400),
            mode,
        );
        let _action = self.action_lock.lock().await;
        executor
            .run_drag_path(waypoints)
            .await
            .map(|_| "ok".to_string())
            .map_err(mcp_err)
    }

    #[tool(description = "Scroll mouse wheel. delta > 0 = scroll up, delta < 0 = scroll down")]
    async fn mouse_scroll(
        &self,
        Parameters(p): Parameters<MouseScrollParams>,
    ) -> Result<String, McpError> {
        let executor = self.service.lock().await.executor();
        let _action = self.action_lock.lock().await;
        executor
            .scroll(p.delta)
            .await
            .map(|_| "ok".to_string())
            .map_err(mcp_err)
    }

    #[tool(description = "Get current mouse position. Returns JSON {\"x\": int, \"y\": int}")]
    async fn mouse_pos(&self) -> Result<String, McpError> {
        let executor = self.service.lock().await.executor();
        let (x, y) = executor.mouse_pos().await.map_err(mcp_err)?;
        Ok(format!("{{\"x\":{x},\"y\":{y}}}"))
    }

    #[tool(description = "Type text. mode = human (char-by-char with delays) | fast")]
    async fn key_type(&self, Parameters(p): Parameters<KeyTypeParams>) -> Result<String, McpError> {
        let mode = p
            .mode
            .as_deref()
            .unwrap_or("human")
            .parse()
            .map_err(mcp_err)?;
        let executor = self.service.lock().await.executor();
        match mode {
            crate::behavior::Mode::Fast => {
                let _action = self.action_lock.lock().await;
                executor
                    .type_fast(p.text)
                    .await
                    .map(|_| "ok".to_string())
                    .map_err(mcp_err)
            }
            crate::behavior::Mode::Human => {
                let events = self.service.lock().await.plan_key_type(&p.text);
                let _action = self.action_lock.lock().await;
                executor
                    .type_events(events)
                    .await
                    .map(|_| "ok".to_string())
                    .map_err(mcp_err)
            }
        }
    }

    #[tool(description = "Tap a single key by name (Return, Escape, F1, ctrl, a, ...)")]
    async fn key_tap(&self, Parameters(p): Parameters<KeyTapParams>) -> Result<String, McpError> {
        let executor = self.service.lock().await.executor();
        let key = crate::service::parse_key(&p.key).map_err(mcp_err)?;
        let _action = self.action_lock.lock().await;
        executor
            .tap_key(key)
            .await
            .map(|_| "ok".to_string())
            .map_err(mcp_err)
    }

    #[tool(description = "Press a key combination, e.g. 'ctrl+c' or 'cmd+shift+4'")]
    async fn key_combo(
        &self,
        Parameters(p): Parameters<KeyComboParams>,
    ) -> Result<String, McpError> {
        let executor = self.service.lock().await.executor();
        let (modifiers, main_key) = crate::service::parse_key_combo(&p.keys).map_err(mcp_err)?;
        let _action = self.action_lock.lock().await;
        executor
            .key_combo(modifiers, main_key)
            .await
            .map(|_| "ok".to_string())
            .map_err(mcp_err)
    }
}

#[tool_handler]
impl ServerHandler for HandServer {
    fn get_info(&self) -> ServerInfo {
        // ServerInfo is #[non_exhaustive]; use field mutation instead of struct literal.
        let mut info = ServerInfo::default();
        info.instructions = Some("Human-like desktop mouse and keyboard control".into());
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info
    }
}

pub async fn run_mcp_server() -> Result<()> {
    use rmcp::transport::stdio;
    let server = HandServer::new();
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

fn mcp_err(e: impl std::fmt::Display) -> McpError {
    McpError::internal_error(e.to_string(), None)
}
