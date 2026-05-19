use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::fs;
use wx_core::{ContactProfile, VisionBrainClient, WxClient, WxMessage};

/// Distill the user's own messages across all contacts into a SKILL.md file.
/// `contact` limits to one conversation; `contacts` injects known relationship context.
pub async fn distill_self(
    contact: Option<&str>,
    wx: &WxClient,
    vb: &VisionBrainClient,
    output_path: &PathBuf,
    contacts: &[ContactProfile],
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

    let contacts_section = build_contacts_section(contacts);
    let skill_content = format!(
        r#"---
name: wechat-self
description: 微信自我蒸馏 Skill，模拟用户本人的说话方式和思维模式
metadata:
  type: persona
---

{persona_md}

## Part C — 可用工具

| 命令 | 用途 |
|------|------|
| `wx-agent send <联系人> <消息>` | 发送微信消息 |
| `wx-agent distill contact <联系人>` | 蒸馏联系人画像并保存到本地 |
| `wx-agent distill self [--from <联系人>]` | 更新此自我画像 |
| `wx-agent distill list` | 列出所有已蒸馏联系人 |
| `wx-agent watch [--auto]` | 启动新消息监听和自动回复守护进程 |
| `wx-agent profile <联系人>` | 查看联系人画像详情 |

## Part D — 已知联系人关系图

{contacts_section}
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

fn build_contacts_section(contacts: &[ContactProfile]) -> String {
    if contacts.is_empty() {
        return "（暂无已蒸馏联系人画像，运行 `wx-agent distill contact <name>` 添加）".to_string();
    }
    contacts
        .iter()
        .map(|p| {
            let topics = if p.topics.is_empty() {
                "—".to_string()
            } else {
                p.topics.join("、")
            };
            format!(
                "- **{}** — {} | 风格：{} | 话题：{} | 策略：{}",
                p.contact_name, p.relationship, p.communication_style, topics, p.response_strategy
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
