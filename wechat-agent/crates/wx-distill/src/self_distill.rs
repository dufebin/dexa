use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::fs;
use wx_core::{VisionBrainClient, WxClient, WxMessage};

/// Distill the user's own messages across all contacts into a SKILL.md file.
/// If `contact` is Some, only use messages from that conversation.
pub async fn distill_self(
    contact: Option<&str>,
    wx: &WxClient,
    vb: &VisionBrainClient,
    output_path: &PathBuf,
) -> Result<()> {
    let my_messages = if let Some(name) = contact {
        println!("Exporting self-messages from conversation with 「{name}」…");
        let msgs = wx.export_all(name).await?;
        msgs.into_iter().filter(|m| m.is_self).collect::<Vec<_>>()
    } else {
        println!("Fetching self-messages from all recent sessions…");
        let sessions = wx.sessions().await?;
        let mut all: Vec<WxMessage> = Vec::new();
        for session in &sessions {
            if let Ok(msgs) = wx.history(&session.name, 500).await {
                all.extend(msgs.into_iter().filter(|m| m.is_self));
            }
        }
        all
    };

    if my_messages.is_empty() {
        anyhow::bail!("no self-messages found to distill");
    }

    println!("  {} self-messages collected, distilling…", my_messages.len());

    let text = my_messages
        .iter()
        .map(|m| format!("[{}] {}", m.timestamp, m.content))
        .collect::<Vec<_>>()
        .join("\n");

    let persona_md = vb
        .distill_self(&text)
        .await
        .context("vision-brain self-distill failed")?;

    let skill_content = format!(
        r#"---
name: wechat-self
description: 微信自我蒸馏 Skill，模拟用户本人的说话方式和思维模式
metadata:
  type: persona
---

{persona_md}
"#
    );

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(output_path, &skill_content)
        .await
        .with_context(|| format!("failed to write SKILL.md to {:?}", output_path))?;

    println!("  Self persona written to {:?}", output_path);
    Ok(())
}
