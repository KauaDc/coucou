// Preferences, stored as plain JSON in settings.json under platform::config_dir().
// No secret ever lands here — API keys live in the OS keychain (see secrets.rs).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub sound_enabled: bool,
    pub sound_volume: f64,
    pub auto_close_interval: f64,
    pub absence_interval: f64,
    pub active_integrations: Vec<String>,
    /// "primary" = the main display, "cursor" = whichever display the mouse is on,
    /// "monitor:<name>@<w>x<h>+<x>+<y>" = one display in particular (see island::monitor_id).
    pub screen: String,
    pub autostart: bool,
    pub hooks_installed: bool,
    /// Claude model used by the chat. Changeable in the settings window.
    /// Defaulted explicitly so a settings.json written by an older build still loads.
    #[serde(default = "default_model")]
    pub model: String,
    /// Which API the chat talks to. Missing or unknown → Claude, so an older
    /// settings.json keeps working as before.
    #[serde(default)]
    pub chat_provider: ChatProvider,
    /// Gemini model used by the chat when `chat_provider` is Gemini.
    #[serde(default = "default_gemini_model")]
    pub gemini_model: String,
}

fn default_model() -> String {
    crate::claude::DEFAULT_MODEL.to_string()
}

fn default_gemini_model() -> String {
    crate::gemini::DEFAULT_MODEL.to_string()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatProvider {
    #[default]
    Anthropic,
    Gemini,
}

// Hand-written so a value this build does not know falls back to Claude instead
// of failing the whole file — load() would otherwise reset every preference.
impl<'de> Deserialize<'de> for ChatProvider {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // Value accepts any JSON, so even a number or null cannot fail the parse.
        let value = serde_json::Value::deserialize(d)?;
        Ok(match value.as_str() {
            Some("gemini") => ChatProvider::Gemini,
            _ => ChatProvider::Anthropic,
        })
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sound_enabled: true,
            sound_volume: 0.12,
            auto_close_interval: 15.0,
            absence_interval: 180.0,
            active_integrations: vec![
                "integration_resend".into(),
                "integration_n8n".into(),
                "integration_vercel".into(),
                "integration_github".into(),
            ],
            screen: "primary".into(),
            autostart: false,
            hooks_installed: false,
            model: default_model(),
            chat_provider: ChatProvider::default(),
            gemini_model: default_gemini_model(),
        }
    }
}

pub use crate::platform::{config_dir, local_dir};

pub fn hook_exe_path() -> PathBuf {
    local_dir().join("bin").join(crate::platform::HOOK_EXE)
}

fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

pub fn load() -> Settings {
    match std::fs::read(settings_path()) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

pub fn save(settings: &Settings) -> std::io::Result<()> {
    let dir = config_dir();
    crate::platform::ensure_private_dir(&dir)?;
    let json = serde_json::to_vec_pretty(settings)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(settings_path(), json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn older_settings_file_still_loads() {
        let json = r#"{"soundEnabled":false,"soundVolume":0.5,"autoCloseInterval":20,"absenceInterval":180,
            "activeIntegrations":[],"screen":"primary","autostart":false,"hooksInstalled":true,"model":"claude-sonnet-5"}"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert!(!s.sound_enabled);
        assert_eq!(s.model, "claude-sonnet-5");
        assert_eq!(s.chat_provider, ChatProvider::Anthropic);
        assert_eq!(s.gemini_model, crate::gemini::DEFAULT_MODEL);
    }

    #[test]
    fn unknown_provider_falls_back_without_losing_the_rest() {
        let json = r#"{"soundEnabled":false,"soundVolume":0.5,"autoCloseInterval":20,"absenceInterval":180,
            "activeIntegrations":[],"screen":"cursor","autostart":false,"hooksInstalled":false,"chatProvider":"openai"}"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.chat_provider, ChatProvider::Anthropic);
        assert_eq!(s.screen, "cursor");
    }

    #[test]
    fn provider_round_trips_lowercase() {
        let s = Settings { chat_provider: ChatProvider::Gemini, ..Settings::default() };
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json["chatProvider"], "gemini");
        let back: Settings = serde_json::from_value(json).unwrap();
        assert_eq!(back.chat_provider, ChatProvider::Gemini);
    }
}
