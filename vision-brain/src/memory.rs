use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS memories (
    key        TEXT PRIMARY KEY,
    steps      TEXT NOT NULL,
    hits       INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
";

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryRow {
    pub key: String,
    pub steps: serde_json::Value,
    pub hits: i64,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryMeta {
    pub key: String,
    pub hits: i64,
    pub updated_at: String,
}

pub struct Memory {
    db_path: PathBuf,
}

impl Memory {
    pub fn new(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            db_path: db_path.to_owned(),
        })
    }

    pub async fn get(&self, key: String) -> Result<Option<MemoryRow>> {
        let path = self.db_path.clone();
        tokio::task::spawn_blocking(move || -> Result<Option<MemoryRow>> {
            let conn = Connection::open(&path)?;
            conn.execute(
                "UPDATE memories SET hits = hits + 1, updated_at = datetime('now') WHERE key = ?1",
                params![key],
            )?;
            conn.query_row(
                "SELECT key, steps, hits, updated_at FROM memories WHERE key = ?1",
                params![key],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()?
            .map(|(k, steps_str, hits, updated_at)| {
                let steps =
                    serde_json::from_str(&steps_str).unwrap_or(serde_json::Value::Array(vec![]));
                Ok(MemoryRow {
                    key: k,
                    steps,
                    hits,
                    updated_at,
                })
            })
            .transpose()
        })
        .await?
    }

    pub async fn set(&self, key: String, steps: serde_json::Value) -> Result<()> {
        let path = self.db_path.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let conn = Connection::open(&path)?;
            let steps_str = serde_json::to_string(&steps)?;
            conn.execute(
                "INSERT INTO memories (key, steps, hits, updated_at)
                 VALUES (?1, ?2, 0, datetime('now'))
                 ON CONFLICT(key) DO UPDATE SET steps = ?2, updated_at = datetime('now')",
                params![key, steps_str],
            )?;
            Ok(())
        })
        .await?
    }

    pub async fn delete(&self, key: String) -> Result<bool> {
        let path = self.db_path.clone();
        tokio::task::spawn_blocking(move || -> Result<bool> {
            let conn = Connection::open(&path)?;
            let n = conn.execute("DELETE FROM memories WHERE key = ?1", params![key])?;
            Ok(n > 0)
        })
        .await?
    }

    pub async fn list(&self) -> Result<Vec<MemoryMeta>> {
        let path = self.db_path.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<MemoryMeta>> {
            let conn = Connection::open(&path)?;
            let mut stmt = conn.prepare(
                "SELECT key, hits, updated_at FROM memories ORDER BY updated_at DESC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(MemoryMeta {
                    key: row.get(0)?,
                    hits: row.get(1)?,
                    updated_at: row.get(2)?,
                })
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| anyhow!("{e}"))
        })
        .await?
    }
}
