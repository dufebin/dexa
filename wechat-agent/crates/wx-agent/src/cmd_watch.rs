use anyhow::Result;
use std::io::{self, Write as IoWrite};
use std::time::Duration;
use tokio::time::sleep;
use wx_core::{Database, HandClient, LlmClient, PendingMessage, WxClient};

use crate::config::{default_db_path, Config};
use crate::wechat_ui;

pub async fn run(auto: bool, cfg: &Config) -> Result<()> {
    let wx   = WxClient::new(cfg.wx_bin());
    let hand = HandClient::new(cfg.hand_bin());
    let llm  = LlmClient::new(
        &cfg.claude.api_key,
        &cfg.claude.reply_model,
        &cfg.claude.distill_model,
    );
    let db = Database::open(&default_db_path()).await?;

    let auto = auto || cfg.agent.mode == "auto";
    let mode = if auto { "全自动" } else { "半自动（需确认）" };
    println!("wx-agent watch 已启动 [{mode}]，轮询间隔 {}s，按 Ctrl+C 退出。\n",
        cfg.agent.poll_interval);

    loop {
        // 1. Poll for new messages and enqueue them.
        match wx.new_messages().await {
            Ok(new_msgs) => {
                for msg in &new_msgs {
                    if msg.is_self { continue; }
                    let pending = PendingMessage {
                        msg_id:    msg.stable_id(),
                        sender:    msg.display_sender().to_string(),
                        content:   msg.content.clone(),
                        timestamp: msg.timestamp,
                        chat_name: msg.chat_name.clone(),
                        status:    "pending".into(),
                        reply:     String::new(),
                    };
                    let _ = db.enqueue_message(&pending).await;
                }
            }
            Err(e) => {
                tracing::warn!("wx new-messages error: {e}");
            }
        }

        // 2. Process all pending messages.
        let pending = db.pending_messages().await.unwrap_or_default();
        for msg in pending {
            // Skip if profile required but not available.
            if cfg.agent.require_profile {
                let profile = db.get_profile(&msg.sender).await.ok().flatten();
                if profile.is_none() {
                    tracing::debug!(
                        "Skipping message from {} (no profile, require_profile=true)",
                        msg.sender
                    );
                    db.mark_skipped(&msg.msg_id).await.ok();
                    continue;
                }
            }

            if let Err(e) = process_message(&msg, auto, cfg, &wx, &hand, &llm, &db).await {
                tracing::error!("Failed to process message {}: {e}", msg.msg_id);
            }
        }

        sleep(Duration::from_secs(cfg.agent.poll_interval)).await;
    }
}

async fn process_message(
    msg: &PendingMessage,
    auto: bool,
    cfg: &Config,
    wx: &WxClient,
    hand: &HandClient,
    llm: &LlmClient,
    db: &Database,
) -> Result<()> {
    let sender = &msg.sender;

    // Fetch context and profile.
    let history = wx.history(sender, 50).await.unwrap_or_default();
    let history_text = history
        .iter()
        .map(|m| {
            let who = if m.is_self { "我" } else { sender.as_str() };
            format!("{who}: {}", m.content)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let profile = db.get_profile(sender).await.ok().flatten();

    // Generate reply.
    let reply = llm
        .generate_reply(
            &msg.content,
            sender,
            &history_text,
            profile.as_ref(),
            cfg.agent.reply_max_len,
        )
        .await?;

    let final_reply = if auto {
        println!("▶ {sender}: {}", msg.content);
        println!("  ✦ 自动回复: {reply}\n");
        reply.clone()
    } else {
        // Semi-auto: prompt user to confirm.
        println!("┌─────────────────────────────────────────────");
        println!("│ 来自 {sender}: {}", msg.content);
        println!("│ 建议回复: {reply}");
        println!("└─────────────────────────────────────────────");
        print!("[Enter] 发送  [e] 编辑  [s] 跳过: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim();

        match choice {
            "" => reply.clone(),
            "s" => {
                db.mark_skipped(&msg.msg_id).await?;
                println!("已跳过。\n");
                return Ok(());
            }
            "e" => {
                print!("输入回复: ");
                io::stdout().flush()?;
                let mut edited = String::new();
                io::stdin().read_line(&mut edited)?;
                edited.trim().to_string()
            }
            other => {
                // Treat any other input as a custom reply.
                other.to_string()
            }
        }
    };

    if final_reply.is_empty() {
        db.mark_skipped(&msg.msg_id).await?;
        return Ok(());
    }

    // Send via WeChat UI.
    wechat_ui::send_message(
        sender,
        &final_reply,
        hand,
        cfg.wechat_search_key(),
        cfg.wechat.activate_cmd.as_deref(),
    )
    .await?;

    db.mark_replied(&msg.msg_id, &final_reply).await?;

    if !auto {
        println!("已发送。\n");
    }
    Ok(())
}
