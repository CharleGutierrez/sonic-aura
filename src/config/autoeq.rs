use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct ParametricEqBand {
    pub frequency: f32,
    pub gain: f32,
    pub q: f32,
    pub filter_type: String, // "Peaking", "HighShelf", "LowShelf"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AutoEqProfile {
    pub name: String,
    pub bands: Vec<ParametricEqBand>,
}

impl AutoEqProfile {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let profile = serde_json::from_str(&content)?;
        Ok(profile)
    }

    pub fn fetch_from_database(model_name: &str) -> anyhow::Result<Self> {
        let url = format!(
            "https://raw.githubusercontent.com/jaakkopasanen/AutoEq/master/results/{}",
            model_name
        );
        let content = reqwest::blocking::get(&url)?.text()?;
        let profile = serde_json::from_str(&content)?;
        Ok(profile)
    }
}
