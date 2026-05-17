use anyhow::Result;
use wx_core::{Database, LlmClient, WxClient};
use wx_distill::{distill_contact, distill_self};

use crate::config::{default_db_path, default_skill_path, Config};

pub async fn run_contact(contact: &str, cfg: &Config) -> Result<()> {
    let wx  = WxClient::new(cfg.wx_bin());
    let llm = LlmClient::new(
        &cfg.claude.api_key,
        &cfg.claude.reply_model,
        &cfg.claude.distill_model,
    );
    let db = Database::open(&default_db_path()).await?;

    let profile = distill_contact(contact, &wx, &llm).await?;

    println!("\n=== 联系人画像：{contact} ===");
    println!("关系    ：{}", profile.relationship);
    println!("风格    ：{}", profile.communication_style);
    println!("话题    ：{}", profile.topics.join("、"));
    println!("情感    ：{}", profile.emotional_pattern);
    println!("策略    ：{}", profile.response_strategy);
    println!("概括    ：{}", profile.summary);

    db.save_profile(&profile).await?;
    println!("\n画像已保存到本地数据库。");
    Ok(())
}

pub async fn run_self(contact: Option<&str>, cfg: &Config) -> Result<()> {
    let wx  = WxClient::new(cfg.wx_bin());
    let llm = LlmClient::new(
        &cfg.claude.api_key,
        &cfg.claude.reply_model,
        &cfg.claude.distill_model,
    );
    let out = default_skill_path();

    distill_self(contact, &wx, &llm, &out).await?;
    println!("\n自我蒸馏完成，可在 Hermes/OpenClaw 中用 /wechat-self 调用。");
    Ok(())
}

pub async fn run_list(_cfg: &Config) -> Result<()> {
    let db = Database::open(&default_db_path()).await?;
    let profiles = db.list_profiles().await?;

    if profiles.is_empty() {
        println!("（暂无已蒸馏联系人）");
    } else {
        println!("已蒸馏联系人：");
        for name in &profiles {
            println!("  • {name}");
        }
    }
    Ok(())
}
