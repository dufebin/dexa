use anyhow::Result;
use serde_json::{json, Value};
use std::path::Path;

use crate::{apps, capture, memory::Memory, vision};

pub struct Service {
    memory: Memory,
}

impl Service {
    pub fn new(db_path: &Path) -> Result<Self> {
        Ok(Self {
            memory: Memory::new(db_path)?,
        })
    }

    pub async fn screen_capture(&self) -> Result<Value> {
        let b64 = capture::capture_primary().await?;
        Ok(json!({ "base64": b64 }))
    }

    pub async fn screen_analyze(&self, task: &str, screenshot: Option<&str>) -> Result<Value> {
        let preview: String = task.chars().take(80).collect();
        tracing::info!("screen_analyze task={:?}", preview);
        let b64 = match screenshot {
            Some(s) => s.to_string(),
            None => capture::capture_primary().await?,
        };
        let result = vision::analyze(&b64, task).await?;
        Ok(serde_json::to_value(result)?)
    }

    pub async fn memory_get(&self, key: String) -> Result<Value> {
        tracing::debug!("memory_get key={key}");
        match self.memory.get(key).await? {
            Some(row) => Ok(serde_json::to_value(row)?),
            None => Ok(Value::Null),
        }
    }

    pub async fn memory_set(&self, key: String, steps: Value) -> Result<Value> {
        tracing::debug!("memory_set key={key}");
        self.memory.set(key, steps).await?;
        Ok(json!({ "ok": true }))
    }

    pub async fn memory_delete(&self, key: String) -> Result<Value> {
        tracing::debug!("memory_delete key={key}");
        let deleted = self.memory.delete(key).await?;
        Ok(json!({ "ok": deleted }))
    }

    pub async fn memory_list(&self) -> Result<Value> {
        let list = self.memory.list().await?;
        Ok(serde_json::to_value(list)?)
    }

    pub async fn app_list(&self) -> Result<Value> {
        let apps = tokio::task::spawn_blocking(apps::discover_apps).await?;
        Ok(serde_json::to_value(apps)?)
    }

    pub async fn llm_distill_contact(&self, contact: &str, messages: &str) -> Result<Value> {
        let json_str = vision::distill_contact(contact, messages).await?;
        let val: Value = serde_json::from_str(&json_str)?;
        Ok(val)
    }

    pub async fn llm_generate_reply(
        &self,
        sender: &str,
        content: &str,
        history: &str,
        profile_json: Option<&str>,
        max_len: usize,
    ) -> Result<Value> {
        let reply = vision::generate_reply(sender, content, history, profile_json, max_len).await?;
        Ok(json!({ "reply": reply }))
    }

    pub async fn llm_distill_self(&self, messages: &str) -> Result<Value> {
        let md = vision::distill_self(messages).await?;
        Ok(json!({ "markdown": md }))
    }

    pub async fn app_open(&self, query: String) -> Result<Value> {
        let app_list = tokio::task::spawn_blocking(apps::discover_apps).await?;

        // 1. Fuzzy match
        if let Some(app) = apps::fuzzy_find(&query, &app_list) {
            let path = app.path.clone();
            let name = app.name.clone();
            tracing::info!("app_open fuzzy match: {name} ({path})");
            tokio::task::spawn_blocking(move || apps::launch_app(&path)).await??;
            return Ok(json!({ "ok": true, "app": name, "method": "fuzzy" }));
        }

        // 2. LLM fallback
        tracing::info!("app_open fuzzy miss, trying LLM for {:?}", &query);
        match vision::llm_match_app(&query, &app_list).await {
            Ok(Some(path)) => {
                let name = app_list
                    .iter()
                    .find(|a| a.path == path)
                    .map(|a| a.name.clone())
                    .unwrap_or_else(|| path.clone());
                tracing::info!("app_open LLM match: {name}");
                let path_c = path.clone();
                tokio::task::spawn_blocking(move || apps::launch_app(&path_c)).await??;
                Ok(json!({ "ok": true, "app": name, "method": "llm" }))
            }
            Ok(None) => Ok(json!({ "ok": false, "error": "no matching app found" })),
            Err(e) => Err(e),
        }
    }
}
