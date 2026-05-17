use anyhow::{anyhow, Result};
use serde::Serialize;
use strsim::jaro_winkler;

#[derive(Debug, Clone, Serialize)]
pub struct AppInfo {
    pub name: String,    // localized display name
    pub fs_name: String, // filesystem name (sans .app / .exe)
    pub path: String,    // full path
}

// ── public API ────────────────────────────────────────────────────────────────

pub fn discover_apps() -> Vec<AppInfo> {
    discover_impl()
}

/// Returns the best matching AppInfo for a natural-language query.
///
/// Priority order:
///   1. Exact match (query == name or query == fs_name)
///   2. Query is contained in name/fs_name (name starts with / equals query)
///   3. Name/fs_name is contained in query ("open wechat" contains "wechat")
///   4. Jaro-Winkler similarity ≥ 0.75
///
/// Exact-match priority prevents shorter names (e.g. "WeChat") from losing to
/// longer ones that happen to contain the same substring (e.g. "WeChatWebDevTools").
pub fn fuzzy_find<'a>(query: &str, apps: &'a [AppInfo]) -> Option<&'a AppInfo> {
    let q = normalize(query);
    if q.is_empty() {
        return None;
    }

    // 1. Exact match
    for app in apps {
        if normalize(&app.name) == q || normalize(&app.fs_name) == q {
            return Some(app);
        }
    }

    // 2. name/fs_name contains query  (query is a prefix/full word of the app name)
    for app in apps {
        let name_n = normalize(&app.name);
        let fs_n = normalize(&app.fs_name);
        if name_n.contains(&q) || fs_n.contains(&q) {
            return Some(app);
        }
    }

    // 3. query contains name/fs_name  ("打开微信" contains "微信")
    for app in apps {
        let name_n = normalize(&app.name);
        let fs_n = normalize(&app.fs_name);
        if q.contains(&name_n) || q.contains(&fs_n) {
            return Some(app);
        }
    }

    // 4. Jaro-Winkler similarity
    let mut best: (f64, Option<&AppInfo>) = (0.0, None);
    for app in apps {
        let s1 = jaro_winkler(&normalize(&app.name), &q);
        let s2 = jaro_winkler(&normalize(&app.fs_name), &q);
        let s = s1.max(s2);
        if s > best.0 {
            best = (s, Some(app));
        }
    }

    if best.0 >= 0.75 {
        best.1
    } else {
        None
    }
}

pub fn launch_app(path: &str) -> Result<()> {
    launch_impl(path)
}

fn normalize(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

// ── macOS ─────────────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn discover_impl() -> Vec<AppInfo> {
    use std::collections::HashSet;
    let mut apps = Vec::new();
    for dir in &["/Applications", "/System/Applications"] {
        scan_app_dir(std::path::Path::new(dir), &mut apps);
    }
    if let Ok(home) = std::env::var("HOME") {
        scan_app_dir(
            &std::path::PathBuf::from(home).join("Applications"),
            &mut apps,
        );
    }
    let mut seen = HashSet::new();
    apps.retain(|a| seen.insert(a.path.clone()));
    apps
}

#[cfg(target_os = "macos")]
fn scan_app_dir(dir: &std::path::Path, apps: &mut Vec<AppInfo>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("app") {
            if let Some(info) = read_app_info(&path) {
                apps.push(info);
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn read_app_info(app_path: &std::path::Path) -> Option<AppInfo> {
    let fs_name = app_path.file_stem()?.to_str()?.to_string();
    let plist_path = app_path.join("Contents/Info.plist");

    // English name from main Info.plist (used as fs_name / fallback)
    let english_name = if plist_path.exists() {
        read_plist_display_name(&plist_path).unwrap_or_else(|| fs_name.clone())
    } else {
        fs_name.clone()
    };

    // Localized name from zh-Hans / zh-Hant InfoPlist.strings (used as name)
    let localized = read_localized_name(app_path);
    let name = localized.unwrap_or_else(|| english_name.clone());

    Some(AppInfo {
        name,
        fs_name: english_name,
        path: app_path.to_str()?.to_string(),
    })
}

/// Read CFBundleDisplayName / CFBundleName from Info.plist.
#[cfg(target_os = "macos")]
fn read_plist_display_name(plist_path: &std::path::Path) -> Option<String> {
    let val: plist::Value = plist::from_file(plist_path).ok()?;
    let dict = val.as_dictionary()?;
    dict.get("CFBundleDisplayName")
        .or_else(|| dict.get("CFBundleName"))
        .and_then(|v| v.as_string())
        .map(str::to_string)
}

/// Read localized display name from zh-Hans (or zh-Hant) InfoPlist.strings.
///
/// InfoPlist.strings can be either binary plist OR UTF-16 LE plain text.
/// Try binary plist first; fall back to UTF-16 line parser if that fails.
#[cfg(target_os = "macos")]
fn read_localized_name(app_path: &std::path::Path) -> Option<String> {
    for locale in &["zh-Hans", "zh-Hant", "zh_CN", "zh"] {
        let path = app_path
            .join("Contents/Resources")
            .join(format!("{}.lproj/InfoPlist.strings", locale));
        if !path.exists() {
            continue;
        }
        // Try binary/XML plist format first
        if let Some(name) = read_plist_display_name(&path) {
            return Some(name);
        }
        // Fall back to plain-text .strings format (UTF-16 LE or UTF-8)
        if let Some(name) = read_text_strings_display_name(&path) {
            return Some(name);
        }
    }
    None
}

/// Parse a plain-text Apple .strings file (UTF-16 LE or UTF-8).
/// Returns CFBundleDisplayName or CFBundleName if present.
///
/// Format:  "Key" = "Value";   (standard Apple .strings format)
#[cfg(target_os = "macos")]
fn read_text_strings_display_name(path: &std::path::Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;

    // Decode: UTF-16 LE with BOM (0xFF 0xFE) or plain UTF-8
    let text = if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        let u16s: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16(&u16s).ok()?
    } else {
        String::from_utf8(bytes).ok()?
    };

    for key in &["CFBundleDisplayName", "CFBundleName"] {
        if let Some(val) = extract_strings_value(&text, key) {
            return Some(val);
        }
    }
    None
}

/// Extract a single value from an Apple .strings file text.
/// Handles both key formats used in the wild:
///   "CFBundleDisplayName" = "Value";   (quoted key — WeChat, most apps)
///   CFBundleDisplayName = "Value";     (unquoted key — Lark, some apps)
#[cfg(target_os = "macos")]
fn extract_strings_value(text: &str, key: &str) -> Option<String> {
    let quoted_needle = format!("\"{}\"", key);
    for line in text.lines() {
        let line = line.trim();
        // Accept both quoted and unquoted key forms
        if !line.starts_with(&quoted_needle) && !line.starts_with(key) {
            continue;
        }
        let after_eq = line.splitn(2, '=').nth(1)?.trim();
        if after_eq.starts_with('"') {
            let inner = &after_eq[1..];
            let end = inner.rfind('"')?;
            return Some(inner[..end].to_string());
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn launch_impl(path: &str) -> Result<()> {
    std::process::Command::new("open")
        .arg(path)
        .spawn()
        .map_err(|e| anyhow!("open failed: {e}"))?;
    Ok(())
}

// ── Windows ───────────────────────────────────────────────────────────────────
//
// Scan Start Menu shortcut (.lnk) folders — the canonical source of user-visible
// app names (same as the Windows Start menu). Names are clean; uninstallers and
// helper binaries from Program Files are absent.
// `cmd /C start "" <path>` launches both .lnk shortcuts and plain .exe files.

#[cfg(target_os = "windows")]
fn discover_impl() -> Vec<AppInfo> {
    use std::collections::HashSet;
    let mut apps = Vec::new();

    // System-wide Start Menu
    scan_lnk_dir(
        std::path::Path::new(
            r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs",
        ),
        &mut apps,
    );

    // Per-user Start Menu
    if let Ok(appdata) = std::env::var("APPDATA") {
        scan_lnk_dir(
            &std::path::PathBuf::from(appdata)
                .join(r"Microsoft\Windows\Start Menu\Programs"),
            &mut apps,
        );
    }

    let mut seen = HashSet::new();
    apps.retain(|a| seen.insert(a.path.clone()));
    apps
}

#[cfg(target_os = "windows")]
fn scan_lnk_dir(dir: &std::path::Path, apps: &mut Vec<AppInfo>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_lnk_dir(&path, apps);
        } else if path.extension().and_then(|e| e.to_str()) == Some("lnk") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                apps.push(AppInfo {
                    name: stem.to_string(),
                    fs_name: stem.to_string(),
                    path: path.to_str().unwrap_or("").to_string(),
                });
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn launch_impl(path: &str) -> Result<()> {
    // Works for .lnk shortcuts and plain .exe files alike
    std::process::Command::new("cmd")
        .args(["/C", "start", "", path])
        .spawn()
        .map_err(|e| anyhow!("start failed: {e}"))?;
    Ok(())
}

// ── Other / fallback ──────────────────────────────────────────────────────────

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn discover_impl() -> Vec<AppInfo> {
    vec![]
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn launch_impl(_path: &str) -> Result<()> {
    Err(anyhow!("unsupported platform"))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_feishu() {
        let apps = discover_apps();
        println!("Discovered {} apps", apps.len());
        let app = fuzzy_find("飞书", &apps).expect("飞书 not found in app list");
        println!("Matched: name={:?}  fs={:?}  path={}", app.name, app.fs_name, app.path);
        launch_app(&app.path).expect("launch failed");
        println!("Launch command sent OK");
    }
}
