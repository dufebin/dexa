use anyhow::Result;
use wx_core::HandClient;

use crate::config::Config;
use crate::wechat_ui;

pub async fn run(contact: &str, message: &str, cfg: &Config) -> Result<()> {
    let hand = HandClient::new(cfg.hand_bin());

    println!("Sending to 「{contact}」: {message}");
    wechat_ui::send_message(
        contact,
        message,
        &hand,
        cfg.wechat_search_key(),
        cfg.wechat.activate_cmd.as_deref(),
    )
    .await?;
    println!("Sent.");
    Ok(())
}
