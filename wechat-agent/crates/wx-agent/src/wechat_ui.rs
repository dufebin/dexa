use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;
use wx_core::HandClient;

/// Bring WeChat to the foreground using the OS-native method.
pub fn activate_wechat(activate_cmd: Option<&str>) -> Result<()> {
    if let Some(cmd) = activate_cmd {
        let mut parts = cmd.splitn(2, ' ');
        let program = parts.next().unwrap();
        let rest    = parts.next().unwrap_or("");

        let mut proc = std::process::Command::new(program);
        if !rest.is_empty() {
            proc.args(rest.split_whitespace());
        }
        proc.status()
            .map_err(|e| anyhow::anyhow!("failed to activate WeChat ({cmd}): {e}"))?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-a")
            .arg("WeChat")
            .status()
            .map_err(|e| anyhow::anyhow!("failed to activate WeChat: {e}"))?;
    }

    #[cfg(target_os = "windows")]
    {
        // Try WeChat.exe first
        let status1 = std::process::Command::new("cmd")
            .arg("/c")
            .arg("start WeChat.exe 2>nul")
            .status()
            .map_err(|e| anyhow::anyhow!("failed to activate WeChat: {e}"))?;

        if !status1.success() {
            // Try Weixin.exe if WeChat.exe failed
            let status2 = std::process::Command::new("cmd")
                .arg("/c")
                .arg("start Weixin.exe 2>nul")
                .status()
                .map_err(|e| anyhow::anyhow!("failed to activate Weixin: {e}"))?;

            if !status2.success() {
                anyhow::bail!("未检测到 WeChat.exe 或 Weixin.exe，请检查微信是否安装或在 config.toml 中配置 activate_cmd");
            }
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        std::process::Command::new("wechat")
            .status()
            .map_err(|e| anyhow::anyhow!("failed to activate WeChat: {e}"))?;
    }

    Ok(())
}

/// Send a message to `contact` via WeChat UI automation (desktop-hand).
///
/// Flow:
///   1. Activate WeChat window
///   2. Press search shortcut to open the search bar
///   3. Type the contact name
///   4. Press Enter to open the conversation
///   5. Type the reply in human mode
///   6. Press Enter to send
pub async fn send_message(
    contact: &str,
    message: &str,
    hand: &HandClient,
    search_key: &str,
    activate_cmd: Option<&str>,
) -> Result<()> {
    // Step 1 — bring WeChat to front
    activate_wechat(activate_cmd)?;
    sleep(Duration::from_millis(1500)).await;  // wait for WeChat window to fully appear

    // Step 2 — open WeChat search bar
    hand.key_combo(search_key).await?;
    sleep(Duration::from_millis(600)).await;

    // Step 3 — clear any previous search, then paste contact name
    // Use clipboard paste so Chinese contact names work reliably
    hand.key_combo("ctrl+a").await?;
    sleep(Duration::from_millis(200)).await;
    hand.key_paste(contact).await?;
    sleep(Duration::from_millis(800)).await;

    // Step 4 — select the first result and open conversation
    // In some Windows WeChat versions, the 1st Enter selects the result list item,
    // and a 2nd Enter is required to actually open the chat and focus the input box.
    hand.key_tap("return").await?;
    sleep(Duration::from_millis(500)).await;
    hand.key_tap("return").await?;
    sleep(Duration::from_millis(800)).await;

    // Step 5 — paste message via clipboard (handles CJK/emoji reliably)
    hand.key_paste(message).await?;
    sleep(Duration::from_millis(400)).await; // Give WeChat time to render text

    // Step 6 — send
    // Send using both Enter and Ctrl+Enter to cover both common WeChat shortcut settings
    hand.key_tap("return").await?;
    sleep(Duration::from_millis(300)).await;
    hand.key_combo("return").await?;
    Ok(())
}
