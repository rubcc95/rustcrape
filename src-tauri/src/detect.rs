use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserInfo {
    pub name: String,
    pub path: String,
    pub version: Option<String>,
    pub browser_type: String,
}

fn check_browser(name: &str, paths: &[String], btype: &str) -> Option<BrowserInfo> {
    for p in paths {
        let path = PathBuf::from(p);
        if path.exists() {
            return Some(BrowserInfo {
                name: name.to_string(),
                path: path.to_string_lossy().to_string(),
                version: None,
                browser_type: btype.to_string(),
            });
        }
    }
    None
}

pub fn detect_browsers() -> Vec<BrowserInfo> {
    let mut browsers = Vec::new();

    if let Some(chrome_path) = std::env::var_os("CHROME_PATH") {
        let path = PathBuf::from(chrome_path);
        if path.exists() {
            browsers.push(BrowserInfo {
                name: "Chrome".into(),
                path: path.to_string_lossy().to_string(),
                version: None,
                browser_type: "chromium".into(),
            });
        }
    }

    let home = std::env::var("HOME").unwrap_or_default();
    let chrome_local = format!("{}/.local/bin/google-chrome", home);

    let chrome_paths = vec![
        "/usr/bin/google-chrome".to_string(),
        "/usr/bin/google-chrome-stable".to_string(),
        "/usr/bin/chromium".to_string(),
        "/usr/bin/chromium-browser".to_string(),
        chrome_local,
    ]; 
    if let Some(b) = check_browser("Google Chrome", &chrome_paths, "chromium") {
        browsers.push(b);
    }

    let edge_paths = vec![
        "/usr/bin/microsoft-edge".to_string(),
        "/usr/bin/microsoft-edge-stable".to_string(),
    ];
    if let Some(b) = check_browser("Microsoft Edge", &edge_paths, "chromium") {
        browsers.push(b);
    }

    let firefox_paths = vec![
        "/usr/bin/firefox".to_string(),
        "/usr/bin/firefox-esr".to_string(),
    ];
    if let Some(b) = check_browser("Firefox", &firefox_paths, "firefox") {
        browsers.push(b);
    }

    let chromium_snap = format!("{}/snap/chromium/current/.local/bin/chromium", home);
    let chromium_paths = vec![
        "/usr/bin/chromium".to_string(),
        "/usr/bin/chromium-browser".to_string(),
        chromium_snap,
    ];
    if !browsers.iter().any(|b| b.browser_type == "chromium") {
        if let Some(b) = check_browser("Chromium", &chromium_paths, "chromium") {
            browsers.push(b);
        }
    }

    let playwright_cache = format!("{}/.cache/ms-playwright", home);
    if let Ok(entries) = std::fs::read_dir(&playwright_cache) {
        for entry in entries.flatten() {
            let p = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("chromium") {
                let chrome_path = p.join("chrome-linux64").join("chrome");
                if chrome_path.exists() {
                    browsers.push(BrowserInfo {
                        name: format!("Playwright {}", name),
                        path: chrome_path.to_string_lossy().to_string(),
                        version: None,
                        browser_type: "chromium".into(),
                    });
                }
            }
            if name.starts_with("firefox") {
                let ff_path = p.join("firefox").join("firefox");
                if ff_path.exists() {
                    browsers.push(BrowserInfo {
                        name: format!("Playwright {}", name),
                        path: ff_path.to_string_lossy().to_string(),
                        version: None,
                        browser_type: "firefox".into(),
                    });
                }
            }
        }
    }

    browsers
}
