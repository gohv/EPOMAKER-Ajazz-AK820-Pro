/// Last-applied keyboard settings, persisted locally.
/// The keyboard can't report its current sleep time or key response time,
/// but both are sent together in one block, so changing one needs the other.
/// Stored at $XDG_CONFIG_HOME/ak820/settings (default ~/.config/ak820/settings).

use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::protocol::{KeyResponse, SleepTime};

#[derive(Debug, Default, Clone, Copy)]
pub struct StoredSettings {
    pub sleep: Option<SleepTime>,
    pub key_response: Option<KeyResponse>,
}

fn store_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("ak820").join("settings"))
}

impl StoredSettings {
    /// Load stored settings. Missing or unreadable values are None.
    pub fn load() -> Self {
        let mut s = Self::default();
        let Some(text) = store_path().and_then(|p| std::fs::read_to_string(p).ok()) else {
            return s;
        };
        for line in text.lines() {
            match line.split_once('=') {
                Some(("sleep", v)) => s.sleep = SleepTime::from_name(v.trim()),
                Some(("key_response", v)) => {
                    s.key_response = v.trim().parse().ok().and_then(KeyResponse::new)
                }
                _ => {}
            }
        }
        s
    }

    pub fn save(&self) -> Result<()> {
        let path = store_path().context("Neither XDG_CONFIG_HOME nor HOME is set")?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .context(format!("Failed to create {}", dir.display()))?;
        }
        let mut text = String::new();
        if let Some(sleep) = self.sleep {
            text.push_str(&format!("sleep={}\n", sleep.name()));
        }
        if let Some(k) = self.key_response {
            text.push_str(&format!("key_response={}\n", k.level()));
        }
        std::fs::write(&path, text).context(format!("Failed to write {}", path.display()))
    }
}
