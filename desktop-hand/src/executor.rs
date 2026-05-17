use anyhow::{anyhow, Result};
use enigo::{Axis, Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use std::time::Duration;

/// Stateless executor — creates a fresh Enigo per operation via spawn_blocking.
/// No randomness here: that's handled by the callers in service.rs.
#[derive(Clone, Copy)]
pub struct Executor;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TypingEvent {
    Char(char),
    Key(Key),
    Delay(u64),
}

impl Executor {
    pub fn new() -> Self {
        Self
    }

    /// Move mouse through a series of (x, y, delay_ms) waypoints in one blocking call.
    pub async fn run_path(&self, waypoints: Vec<(i32, i32, u64)>) -> Result<()> {
        tokio::task::spawn_blocking(move || -> Result<()> {
            let mut en = make_enigo()?;
            for (x, y, delay_ms) in waypoints {
                en.move_mouse(x, y, Coordinate::Abs)
                    .map_err(|e| anyhow!("mouse move: {e:?}"))?;
                if delay_ms > 0 {
                    std::thread::sleep(Duration::from_millis(delay_ms));
                }
            }
            Ok(())
        })
        .await?
    }

    /// Move mouse through waypoints with mouse button held (for drag).
    pub async fn run_drag_path(&self, waypoints: Vec<(i32, i32, u64)>) -> Result<()> {
        tokio::task::spawn_blocking(move || -> Result<()> {
            let mut en = make_enigo()?;
            if let Some(&(fx, fy, _)) = waypoints.first() {
                en.move_mouse(fx, fy, Coordinate::Abs)
                    .map_err(|e| anyhow!("drag move: {e:?}"))?;
            }
            en.button(Button::Left, Direction::Press)
                .map_err(|e| anyhow!("drag press: {e:?}"))?;
            for (x, y, delay_ms) in &waypoints {
                en.move_mouse(*x, *y, Coordinate::Abs)
                    .map_err(|e| anyhow!("drag step: {e:?}"))?;
                if *delay_ms > 0 {
                    std::thread::sleep(Duration::from_millis(*delay_ms));
                }
            }
            en.button(Button::Left, Direction::Release)
                .map_err(|e| anyhow!("drag release: {e:?}"))?;
            Ok(())
        })
        .await?
    }

    pub async fn click(&self, x: i32, y: i32, button: Button, double: bool) -> Result<()> {
        tokio::task::spawn_blocking(move || -> Result<()> {
            let mut en = make_enigo()?;
            en.move_mouse(x, y, Coordinate::Abs)
                .map_err(|e| anyhow!("click move: {e:?}"))?;
            en.button(button, Direction::Click)
                .map_err(|e| anyhow!("click: {e:?}"))?;
            if double {
                std::thread::sleep(Duration::from_millis(50));
                en.button(button, Direction::Click)
                    .map_err(|e| anyhow!("double click: {e:?}"))?;
            }
            Ok(())
        })
        .await?
    }

    pub async fn scroll(&self, delta: i32) -> Result<()> {
        tokio::task::spawn_blocking(move || -> Result<()> {
            let mut en = make_enigo()?;
            en.scroll(delta, Axis::Vertical)
                .map_err(|e| anyhow!("scroll: {e:?}"))?;
            Ok(())
        })
        .await?
    }

    pub async fn mouse_pos(&self) -> Result<(i32, i32)> {
        tokio::task::spawn_blocking(move || -> Result<(i32, i32)> {
            let en = make_enigo()?;
            en.location().map_err(|e| anyhow!("location: {e:?}"))
        })
        .await?
    }

    /// Type a mixed sequence of characters, special keys, and delays.
    pub async fn type_events(&self, events: Vec<TypingEvent>) -> Result<()> {
        tokio::task::spawn_blocking(move || -> Result<()> {
            let mut en = make_enigo()?;
            for event in events {
                match event {
                    TypingEvent::Char(ch) => {
                        en.key(Key::Unicode(ch), Direction::Click)
                            .map_err(|e| anyhow!("type char '{}': {e:?}", ch))?;
                    }
                    TypingEvent::Key(key) => {
                        en.key(key, Direction::Click)
                            .map_err(|e| anyhow!("type key {:?}: {e:?}", key))?;
                    }
                    TypingEvent::Delay(delay_ms) => {
                        if delay_ms > 0 {
                            std::thread::sleep(Duration::from_millis(delay_ms));
                        }
                    }
                }
            }
            Ok(())
        })
        .await?
    }

    /// Type text as fast as possible using the platform text API.
    pub async fn type_fast(&self, text: String) -> Result<()> {
        tokio::task::spawn_blocking(move || -> Result<()> {
            let mut en = make_enigo()?;
            match en.text(&text) {
                Ok(()) => Ok(()),
                Err(_) => {
                    // fallback: char by char with minimal delay
                    for ch in text.chars() {
                        en.key(Key::Unicode(ch), Direction::Click)
                            .map_err(|e| anyhow!("fast type: {e:?}"))?;
                    }
                    Ok(())
                }
            }
        })
        .await?
    }

    pub async fn tap_key(&self, key: Key) -> Result<()> {
        tokio::task::spawn_blocking(move || -> Result<()> {
            let mut en = make_enigo()?;
            en.key(key, Direction::Click)
                .map_err(|e| anyhow!("tap key: {e:?}"))
        })
        .await?
    }

    /// Press modifier keys, tap the main key, release modifiers in reverse order.
    pub async fn key_combo(&self, modifiers: Vec<Key>, main_key: Option<Key>) -> Result<()> {
        tokio::task::spawn_blocking(move || -> Result<()> {
            let mut en = make_enigo()?;
            for &m in &modifiers {
                en.key(m, Direction::Press)
                    .map_err(|e| anyhow!("modifier press: {e:?}"))?;
            }
            if let Some(mk) = main_key {
                en.key(mk, Direction::Click)
                    .map_err(|e| anyhow!("combo main key: {e:?}"))?;
            }
            for &m in modifiers.iter().rev() {
                en.key(m, Direction::Release)
                    .map_err(|e| anyhow!("modifier release: {e:?}"))?;
            }
            Ok(())
        })
        .await?
    }
}

fn make_enigo() -> Result<Enigo> {
    Enigo::new(&Settings::default()).map_err(|e| anyhow!("enigo init: {e:?}"))
}
