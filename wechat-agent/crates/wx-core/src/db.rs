use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::{sqlite::SqliteConnectOptions, Pool, Sqlite, SqlitePool};
use std::str::FromStr;

use crate::models::{ContactProfile, PendingMessage};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS contact_profiles (
    contact_name TEXT PRIMARY KEY,
    data         TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS pending_messages (
    msg_id    TEXT PRIMARY KEY,
    sender    TEXT NOT NULL,
    content   TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    chat_name TEXT NOT NULL DEFAULT '',
    status    TEXT NOT NULL DEFAULT 'pending',
    reply     TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL
);
"#;

pub struct Database {
    pool: Pool<Sqlite>,
}

impl Database {
    pub async fn open(path: &str) -> Result<Self> {
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        let opts = SqliteConnectOptions::from_str(&format!("sqlite:{path}"))?
            .create_if_missing(true);
        let pool = SqlitePool::connect_with(opts)
            .await
            .with_context(|| format!("failed to open SQLite at {path}"))?;

        sqlx::query(SCHEMA).execute(&pool).await?;
        Ok(Self { pool })
    }

    // ── contact profiles ────────────────────────────────────────────────────

    pub async fn save_profile(&self, profile: &ContactProfile) -> Result<()> {
        let data = serde_json::to_string(profile)?;
        sqlx::query(
            "INSERT INTO contact_profiles (contact_name, data, updated_at)
             VALUES (?, ?, ?)
             ON CONFLICT(contact_name) DO UPDATE SET data = excluded.data, updated_at = excluded.updated_at",
        )
        .bind(&profile.contact_name)
        .bind(&data)
        .bind(&profile.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_profile(&self, contact_name: &str) -> Result<Option<ContactProfile>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT data FROM contact_profiles WHERE contact_name = ?")
                .bind(contact_name)
                .fetch_optional(&self.pool)
                .await?;

        match row {
            Some((data,)) => Ok(Some(serde_json::from_str(&data)?)),
            None => Ok(None),
        }
    }

    pub async fn list_profiles(&self) -> Result<Vec<String>> {
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT contact_name FROM contact_profiles ORDER BY updated_at DESC")
                .fetch_all(&self.pool)
                .await?;
        Ok(rows.into_iter().map(|(n,)| n).collect())
    }

    // ── pending messages ─────────────────────────────────────────────────────

    /// Insert a new message if not already present (idempotent).
    pub async fn enqueue_message(&self, msg: &PendingMessage) -> Result<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO pending_messages
             (msg_id, sender, content, timestamp, chat_name, status, reply, created_at)
             VALUES (?, ?, ?, ?, ?, 'pending', '', ?)",
        )
        .bind(&msg.msg_id)
        .bind(&msg.sender)
        .bind(&msg.content)
        .bind(msg.timestamp)
        .bind(&msg.chat_name)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Load all messages with status = 'pending', ordered by timestamp.
    pub async fn pending_messages(&self) -> Result<Vec<PendingMessage>> {
        let rows = sqlx::query_as::<_, PendingMessage>(
            "SELECT msg_id, sender, content, timestamp, chat_name, status, reply
             FROM pending_messages
             WHERE status = 'pending'
             ORDER BY timestamp ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn mark_replied(&self, msg_id: &str, reply: &str) -> Result<()> {
        sqlx::query(
            "UPDATE pending_messages SET status = 'replied', reply = ? WHERE msg_id = ?",
        )
        .bind(reply)
        .bind(msg_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_skipped(&self, msg_id: &str) -> Result<()> {
        sqlx::query(
            "UPDATE pending_messages SET status = 'skipped' WHERE msg_id = ?",
        )
        .bind(msg_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
