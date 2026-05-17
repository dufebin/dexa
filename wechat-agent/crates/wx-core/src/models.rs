use serde::{Deserialize, Serialize};

/// Raw message as returned by wx-cli --json.
/// Field aliases handle possible naming variations across wx-cli versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WxMessage {
    #[serde(default, alias = "msg_svr_id", alias = "msgId")]
    pub msg_id: String,

    /// Sender display name. For new-messages this is the contact name;
    /// for history it may be omitted (everything is from the same contact).
    #[serde(default, alias = "from", alias = "talker", alias = "name")]
    pub sender: String,

    #[serde(alias = "text", alias = "str_content", alias = "msg")]
    pub content: String,

    #[serde(alias = "create_time", alias = "ts")]
    pub timestamp: i64,

    #[serde(default, alias = "chatType")]
    pub chat_type: String,

    #[serde(default, alias = "isSender", alias = "is_sender", alias = "is_self")]
    pub is_self: bool,

    /// Which chat this message belongs to (set by new-messages, not history).
    #[serde(default, alias = "chat", alias = "room", alias = "session")]
    pub chat_name: String,
}

impl WxMessage {
    /// Stable ID for dedup; uses msg_id if non-empty, else derives one.
    pub fn stable_id(&self) -> String {
        if !self.msg_id.is_empty() {
            return self.msg_id.clone();
        }
        let chat = if self.chat_name.is_empty() { &self.sender } else { &self.chat_name };
        format!("{}__{}__{}", self.sender, chat, self.timestamp)
    }

    /// Best-effort display name for who sent this.
    pub fn display_sender(&self) -> &str {
        if !self.sender.is_empty() {
            &self.sender
        } else if !self.chat_name.is_empty() {
            &self.chat_name
        } else {
            "Unknown"
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WxSession {
    #[serde(alias = "chat")]
    pub name: String,
    #[serde(default, alias = "chatType")]
    pub chat_type: String,
    #[serde(default, alias = "latestMessage", alias = "latest_msg", alias = "summary")]
    pub latest_message: String,
    #[serde(default, alias = "unreadCount", alias = "unread")]
    pub unread_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WxContact {
    #[serde(alias = "display")]
    pub name: String,
    #[serde(default, alias = "username")]
    pub wxid: String,
    #[serde(default)]
    pub nickname: String,
}

/// Analyzed persona for a contact, stored in SQLite as JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactProfile {
    pub contact_name: String,
    pub summary: String,
    pub communication_style: String,
    pub topics: Vec<String>,
    pub emotional_pattern: String,
    pub relationship: String,
    pub response_strategy: String,
    pub updated_at: String,
}

/// Incoming message queued for processing.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PendingMessage {
    pub msg_id: String,
    pub sender: String,
    pub content: String,
    pub timestamp: i64,
    pub chat_name: String,
    pub status: String, // "pending" | "replied" | "skipped"
    #[serde(default)]
    pub reply: String,
}
