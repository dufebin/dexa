use anyhow::{Context, Result};
use chrono::Utc;
use wx_core::{ContactProfile, LlmClient, WxClient, WxMessage};

/// Export all messages for `contact`, run LLM distillation, return the profile.
pub async fn distill_contact(
    contact: &str,
    wx: &WxClient,
    llm: &LlmClient,
) -> Result<ContactProfile> {
    println!("Exporting messages for 「{contact}」…");
    let messages = wx
        .export_all(contact)
        .await
        .with_context(|| format!("failed to export messages for {contact}"))?;

    if messages.is_empty() {
        anyhow::bail!("no messages found for contact: {contact}");
    }

    println!("  {} messages fetched, distilling…", messages.len());

    // Separate contact's messages from self messages for richer context.
    let contact_msgs: Vec<&WxMessage> = messages.iter().filter(|m| !m.is_self).collect();
    let self_msgs: Vec<&WxMessage> = messages.iter().filter(|m| m.is_self).collect();

    let contact_text = format_messages_for_llm(&contact_msgs, contact);
    let self_text    = format_messages_for_llm(&self_msgs, "我");

    let combined = format!(
        "=== {contact} 发出的消息 ===\n{contact_text}\n\n=== 我发给 {contact} 的消息 ===\n{self_text}"
    );

    let json_str = llm
        .distill_contact(contact, &combined)
        .await
        .context("LLM distillation failed")?;

    // Parse the JSON that the LLM returned into a serde_json::Value first,
    // so we can inject the contact_name and updated_at fields.
    let mut value: serde_json::Value = serde_json::from_str(&json_str)
        .with_context(|| format!("LLM returned non-JSON: {json_str}"))?;

    value["contact_name"] = serde_json::Value::String(contact.to_string());
    value["updated_at"]   = serde_json::Value::String(Utc::now().to_rfc3339());

    let profile: ContactProfile = serde_json::from_value(value)
        .context("failed to deserialize ContactProfile from LLM output")?;

    Ok(profile)
}

fn format_messages_for_llm(msgs: &[&WxMessage], sender: &str) -> String {
    if msgs.is_empty() {
        return format!("（{sender} 没有发出任何消息）");
    }
    msgs.iter()
        .map(|m| format!("[{}] {}", m.timestamp, m.content))
        .collect::<Vec<_>>()
        .join("\n")
}
