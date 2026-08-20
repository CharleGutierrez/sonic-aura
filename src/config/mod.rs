//! Application Configuration Management (TOML persistence)

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub active_preset: String,
    pub input_device: Option<String>,
    pub output_device: Option<String>,
    pub sample_rate: u32,
    pub buffer_size: u32,
    pub ai_boost_enabled: bool,
    pub ai_intensity: f32,
    pub bass_intensity: f32,
    pub air_mix: f32,
    pub stereo_width: f32,
    pub headphone_mode: bool,
    pub custom_eq_10: [f32; 10],
    pub master_gain_db: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            active_preset: "Dolby Atmos Cinema 3D".to_string(),
            input_device: None,
            output_device: None,
            sample_rate: 48000,
            buffer_size: 512,
            ai_boost_enabled: true,
            ai_intensity: 0.85,
            bass_intensity: 0.85,
            air_mix: 0.65,
            stereo_width: 1.35,
            headphone_mode: true,
            custom_eq_10: [0.0; 10],
            master_gain_db: 0.0,
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let dir = PathBuf::from(home).join(".config").join("sonic_aura");
        let _ = fs::create_dir_all(&dir);
        dir.join("config.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(cfg) = toml::from_str::<AppConfig>(&content) {
                    return cfg;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path();
        let toml_str = toml::to_string_pretty(self)?;
        fs::write(path, toml_str)?;
        Ok(())
    }
}
