//! Persistent user settings: a small JSON file in the OS config dir.
//! Deliberately not a plugin — one struct, one file, no surprises.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub lang: String,
    pub level: String,
    pub region: String,
    pub license_key: Option<String>,
    /// Anthropic API key for narration. Stored locally, never sent anywhere
    /// but api.anthropic.com.
    pub anthropic_api_key: Option<String>,
    pub elevate_for_smart: bool,
    /// Team/Enterprise: Threadwise workspace to upload reports to.
    pub cloud_endpoint: Option<String>,
    pub cloud_token: Option<String>,
    /// Upload automatically after every scan (Team/Enterprise).
    pub cloud_auto_upload: bool,
    /// Friendly label for this machine in the team dashboard.
    pub machine_label: Option<String>,
    /// Scheduled rescan + upload cadence while the app is open (Team).
    pub cloud_interval_hours: u32,
}

impl Default for Settings {
    fn default() -> Self {
        let lang = std::env::var("LANG").ok().and_then(|l| l.get(..2).map(String::from)).filter(|l| l == "es").unwrap_or_else(|| "en".into());
        Self {
            lang, level: "informed".into(), region: "us".into(), license_key: None, anthropic_api_key: None, elevate_for_smart: false,
            cloud_endpoint: None, cloud_token: None, cloud_auto_upload: false, machine_label: None, cloud_interval_hours: 24,
        }
    }
}

pub fn path() -> PathBuf {
    let dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("innards");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("settings.json")
}

pub fn load() -> Settings {
    std::fs::read_to_string(path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

pub fn save(s: &Settings) -> Result<(), String> {
    let json = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
    std::fs::write(path(), json).map_err(|e| e.to_string())
}
