pub mod default_presets;

use crate::presets::default_presets::{get_default_presets, Preset};
use std::fs;
use std::path::PathBuf;

pub struct PresetManager {
    pub presets: Vec<Preset>,
    pub current_index: usize,
}

impl PresetManager {
    pub fn new() -> Self {
        let presets = get_default_presets();
        Self {
            presets,
            current_index: 0,
        }
    }

    pub fn current(&self) -> &Preset {
        &self.presets[self.current_index]
    }

    pub fn select(&mut self, index: usize) {
        if index < self.presets.len() {
            self.current_index = index;
        }
    }

    pub fn next(&mut self) {
        self.current_index = (self.current_index + 1) % self.presets.len();
    }

    pub fn prev(&mut self) {
        if self.current_index == 0 {
            self.current_index = self.presets.len() - 1;
        } else {
            self.current_index -= 1;
        }
    }

    pub fn find_by_name(&self, name: &str) -> Option<usize> {
        let name_lower = name.to_lowercase();
        self.presets.iter().position(|p| p.name.to_lowercase().contains(&name_lower))
    }

    pub fn load_user_presets(&mut self, user_presets_path: &PathBuf) {
        if user_presets_path.exists() {
            if let Ok(content) = fs::read_to_string(user_presets_path) {
                if let Ok(user_list) = serde_json::from_str::<Vec<Preset>>(&content) {
                    for p in user_list {
                        self.presets.push(p);
                    }
                }
            }
        }
    }
}
