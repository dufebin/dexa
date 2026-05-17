use anyhow::{anyhow, bail, Result};
use enigo::{Button, Key};
use rand::rngs::SmallRng;
use rand::SeedableRng;

use crate::behavior::Mode;
use crate::executor::{Executor, TypingEvent};
use crate::{human, smooth};

pub struct Service {
    executor: Executor,
    rng: SmallRng,
}

impl Service {
    pub fn new() -> Self {
        Self {
            executor: Executor::new(),
            rng: SmallRng::from_entropy(),
        }
    }

    pub fn executor(&self) -> Executor {
        self.executor
    }

    pub async fn mouse_move(&mut self, x: i32, y: i32, ms: u64, mode: Mode) -> Result<()> {
        tracing::debug!("mouse_move x={x} y={y} ms={ms} mode={mode:?}");
        let from = self.executor.mouse_pos().await?;
        let waypoints = self.plan_mouse_move(from, x, y, ms, mode);
        self.executor.run_path(waypoints).await
    }

    pub fn plan_mouse_move(
        &mut self,
        from: (i32, i32),
        x: i32,
        y: i32,
        ms: u64,
        mode: Mode,
    ) -> Vec<(i32, i32, u64)> {
        match mode {
            Mode::Fast => vec![(x, y, 0)],
            Mode::Human => {
                let steps = human::mouse_steps(ms);
                let path = smooth::generate_path(from.0, from.1, x, y, steps, ms, &mut self.rng);
                jittered_waypoints(path, &mut self.rng, Some((x, y)))
            }
        }
    }

    pub async fn mouse_click(&mut self, x: i32, y: i32, button: &str, double: bool) -> Result<()> {
        tracing::debug!("mouse_click x={x} y={y} button={button} double={double}");
        let btn = parse_button(button)?;
        self.executor.click(x, y, btn, double).await
    }

    pub async fn mouse_drag(
        &mut self,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        ms: u64,
        mode: Mode,
    ) -> Result<()> {
        tracing::debug!("mouse_drag ({x1},{y1}) -> ({x2},{y2}) ms={ms} mode={mode:?}");
        let waypoints = self.plan_mouse_drag(x1, y1, x2, y2, ms, mode);
        self.executor.run_drag_path(waypoints).await
    }

    pub fn plan_mouse_drag(
        &mut self,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        ms: u64,
        mode: Mode,
    ) -> Vec<(i32, i32, u64)> {
        match mode {
            Mode::Fast => vec![(x1, y1, 0), (x2, y2, 0)],
            Mode::Human => {
                let steps = human::mouse_steps(ms);
                let path = smooth::generate_path(x1, y1, x2, y2, steps, ms, &mut self.rng);
                jittered_waypoints(path, &mut self.rng, Some((x2, y2)))
            }
        }
    }

    pub async fn mouse_scroll(&mut self, delta: i32) -> Result<()> {
        tracing::debug!("mouse_scroll delta={delta}");
        self.executor.scroll(delta).await
    }

    pub async fn mouse_pos(&mut self) -> Result<(i32, i32)> {
        self.executor.mouse_pos().await
    }

    pub async fn key_type(&mut self, text: &str, mode: Mode) -> Result<()> {
        tracing::debug!("key_type len={} mode={mode:?}", text.len());
        match mode {
            Mode::Fast => self.executor.type_fast(text.to_string()).await,
            Mode::Human => {
                let events = self.plan_key_type(text);
                self.executor.type_events(events).await
            }
        }
    }

    pub fn plan_key_type(&mut self, text: &str) -> Vec<TypingEvent> {
        let mut events = Vec::new();
        let chars: Vec<char> = text.chars().collect();

        for (i, &ch) in chars.iter().enumerate() {
            if human::should_typo(&mut self.rng) {
                events.push(TypingEvent::Char(human::typo_char(&mut self.rng)));
                events.push(TypingEvent::Delay(human::char_delay_ms(&mut self.rng)));
                events.push(TypingEvent::Key(Key::Backspace));
                events.push(TypingEvent::Delay(human::char_delay_ms(&mut self.rng)));
            }

            events.push(TypingEvent::Char(ch));

            if i + 1 < chars.len() {
                events.push(TypingEvent::Delay(delay_after(ch, &mut self.rng)));
            }
        }

        events
    }

    pub async fn key_tap(&mut self, key_str: &str) -> Result<()> {
        tracing::debug!("key_tap key={key_str}");
        let key = parse_key(key_str)?;
        self.executor.tap_key(key).await
    }

    pub async fn key_combo(&mut self, keys_str: &str) -> Result<()> {
        tracing::debug!("key_combo keys={keys_str}");
        let (modifiers, main_key) = parse_key_combo(keys_str)?;
        self.executor.key_combo(modifiers, main_key).await
    }

    /// Write `text` to the system clipboard, then send Ctrl+V to paste.
    /// This is the most reliable way to input CJK and other Unicode text on Windows.
    pub async fn paste_text(&mut self, text: &str) -> Result<()> {
        tracing::debug!("paste_text len={}", text.len());
        let text = text.to_string();
        // Set clipboard on blocking thread (arboard requires it)
        tokio::task::spawn_blocking(move || -> Result<()> {
            let mut clipboard = arboard::Clipboard::new()
                .map_err(|e| anyhow::anyhow!("clipboard init: {e}"))?;
            clipboard.set_text(&text)
                .map_err(|e| anyhow::anyhow!("clipboard set: {e}"))?;
            Ok(())
        })
        .await??;

        // Small delay to let the clipboard settle
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Send Ctrl+V to paste
        let (modifiers, main_key) = parse_key_combo("ctrl+v")?;
        self.executor.key_combo(modifiers, main_key).await
    }
}

pub fn parse_button(s: &str) -> Result<Button> {
    match s.to_lowercase().as_str() {
        "left" | "" => Ok(Button::Left),
        "right" => Ok(Button::Right),
        other => Err(anyhow!(
            "unknown button '{}', expected 'left' or 'right'",
            other
        )),
    }
}

fn jittered_waypoints(
    path: Vec<smooth::Waypoint>,
    rng: &mut SmallRng,
    exact_final: Option<(i32, i32)>,
) -> Vec<(i32, i32, u64)> {
    let mut waypoints: Vec<(i32, i32, u64)> = path
        .into_iter()
        .map(|wp| {
            let (jx, jy) = human::jitter(rng);
            (wp.x + jx, wp.y + jy, wp.delay_ms)
        })
        .collect();

    if let (Some((x, y)), Some(last)) = (exact_final, waypoints.last_mut()) {
        last.0 = x;
        last.1 = y;
    }

    waypoints
}

fn delay_after(ch: char, rng: &mut SmallRng) -> u64 {
    if ch == ' ' {
        human::word_pause_ms(rng)
    } else if matches!(ch, '.' | '!' | '?') {
        human::sentence_pause_ms(rng)
    } else {
        human::char_delay_ms(rng)
    }
}

pub fn parse_key(s: &str) -> Result<Key> {
    match s.to_lowercase().as_str() {
        "ctrl" | "control" => Ok(Key::Control),
        "alt" | "option" => Ok(Key::Alt),
        "shift" => Ok(Key::Shift),
        "meta" | "cmd" | "super" | "win" | "windows" => Ok(Key::Meta),
        "return" | "enter" => Ok(Key::Return),
        "esc" | "escape" => Ok(Key::Escape),
        "tab" => Ok(Key::Tab),
        "backspace" | "back" => Ok(Key::Backspace),
        "delete" | "del" => Ok(Key::Delete),
        "space" => Ok(Key::Space),
        "up" => Ok(Key::UpArrow),
        "down" => Ok(Key::DownArrow),
        "left" => Ok(Key::LeftArrow),
        "right" => Ok(Key::RightArrow),
        "home" => Ok(Key::Home),
        "end" => Ok(Key::End),
        "pageup" | "pgup" => Ok(Key::PageUp),
        "pagedown" | "pgdn" | "pgdown" => Ok(Key::PageDown),
        "f1" => Ok(Key::F1),
        "f2" => Ok(Key::F2),
        "f3" => Ok(Key::F3),
        "f4" => Ok(Key::F4),
        "f5" => Ok(Key::F5),
        "f6" => Ok(Key::F6),
        "f7" => Ok(Key::F7),
        "f8" => Ok(Key::F8),
        "f9" => Ok(Key::F9),
        "f10" => Ok(Key::F10),
        "f11" => Ok(Key::F11),
        "f12" => Ok(Key::F12),
        s if s.chars().count() == 1 => Ok(Key::Unicode(s.chars().next().unwrap())),
        s => bail!("unknown key '{}'", s),
    }
}

pub fn parse_key_combo(keys_str: &str) -> Result<(Vec<Key>, Option<Key>)> {
    let parts: Vec<&str> = keys_str.split('+').collect();
    let all_keys: Vec<Key> = parts
        .iter()
        .map(|s| parse_key(s.trim()))
        .collect::<Result<Vec<_>>>()?;

    let modifier_variants = [Key::Control, Key::Alt, Key::Shift, Key::Meta];
    let modifiers: Vec<Key> = all_keys
        .iter()
        .copied()
        .filter(|k| modifier_variants.contains(k))
        .collect();
    let main_key = all_keys
        .iter()
        .rev()
        .find(|k| !modifier_variants.contains(k))
        .copied();

    Ok((modifiers, main_key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_key_combo_into_modifiers_and_main_key() {
        let (modifiers, main_key) = parse_key_combo("cmd+shift+4").unwrap();

        assert_eq!(modifiers, vec![Key::Meta, Key::Shift]);
        assert_eq!(main_key, Some(Key::Unicode('4')));
    }

    #[test]
    fn empty_text_creates_no_typing_events() {
        let mut service = Service::new();

        assert!(service.plan_key_type("").is_empty());
    }

    #[test]
    fn drag_plan_ends_at_exact_target_in_human_mode() {
        let mut service = Service::new();
        let waypoints = service.plan_mouse_drag(10, 20, 300, 400, 250, Mode::Human);

        assert_eq!(waypoints.last().map(|(x, y, _)| (*x, *y)), Some((300, 400)));
    }
}
