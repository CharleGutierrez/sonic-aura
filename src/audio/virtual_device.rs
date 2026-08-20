//! Virtual Audio Sink & Auto-Routing Manager for PipeWire and PulseAudio
//! Automatically routes all computer audio (YouTube, Spotify, Games, VLC, Netflix)
//! directly into SonicAura AI, and routes enhanced audio to your physical speakers/headphones.

use anyhow::{Context, Result};
use std::process::Command;

pub struct VirtualSinkManager;

impl VirtualSinkManager {
    pub const SINK_NAME: &'static str = "SonicAura_Sink";
    pub const SINK_DESC: &'static str = "SonicAura_AI_Enhancer_Sink";

    /// Checks if pactl is available in the system
    pub fn is_pactl_available() -> bool {
        Command::new("pactl")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Gets the current system default sink name (ignoring SonicAura_Sink)
    pub fn get_current_default_sink() -> Option<String> {
        if !Self::is_pactl_available() {
            return None;
        }

        if let Ok(output) = Command::new("pactl").arg("get-default-sink").output() {
            if output.status.success() {
                let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !name.is_empty() && !name.contains(Self::SINK_NAME) {
                    return Some(name);
                }
            }
        }

        if let Ok(output) = Command::new("pactl").arg("info").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.starts_with("Default Sink:") {
                    let name = line.replace("Default Sink:", "").trim().to_string();
                    if !name.is_empty() && !name.contains(Self::SINK_NAME) {
                        return Some(name);
                    }
                }
            }
        }

        None
    }

    /// Gets the current system default source name (ignoring SonicAura_Sink)
    pub fn get_current_default_source() -> Option<String> {
        if !Self::is_pactl_available() {
            return None;
        }

        if let Ok(output) = Command::new("pactl").arg("get-default-source").output() {
            if output.status.success() {
                let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !name.is_empty() && !name.contains(Self::SINK_NAME) {
                    return Some(name);
                }
            }
        }

        None
    }

    /// Sets the system default sink
    pub fn set_default_sink(sink_name: &str) -> Result<()> {
        if !Self::is_pactl_available() {
            return Ok(());
        }

        let output = Command::new("pactl")
            .args(["set-default-sink", sink_name])
            .output()
            .context("Failed to execute pactl set-default-sink")?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to set default sink: {}", err);
        }
        Ok(())
    }

    /// Sets the system default source
    pub fn set_default_source(source_name: &str) -> Result<()> {
        if !Self::is_pactl_available() {
            return Ok(());
        }

        let output = Command::new("pactl")
            .args(["set-default-source", source_name])
            .output()
            .context("Failed to execute pactl set-default-source")?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to set default source: {}", err);
        }
        Ok(())
    }

    /// Creates a virtual null-sink for system audio capture
    pub fn create_virtual_sink() -> Result<u32> {
        if !Self::is_pactl_available() {
            anyhow::bail!("PulseAudio/PipeWire 'pactl' utility is not found.");
        }

        // First check if sink already exists
        if let Ok(true) = Self::is_virtual_sink_loaded() {
            return Ok(0);
        }

        let output = Command::new("pactl")
            .args([
                "load-module",
                "module-null-sink",
                &format!("sink_name={}", Self::SINK_NAME),
                &format!("sink_properties=device.description={}", Self::SINK_DESC),
            ])
            .output()
            .context("Failed to execute pactl load-module")?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("pactl load-module failed: {}", err);
        }

        let module_id_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let module_id = module_id_str.parse::<u32>().unwrap_or(0);
        Ok(module_id)
    }

    /// Automatically sets up the virtual sink and routes YouTube/System audio into SonicAura
    pub fn auto_route_system_audio() -> (Option<String>, Option<String>) {
        if !Self::is_pactl_available() {
            return (None, None);
        }

        let original_sink = Self::get_current_default_sink();
        let original_source = Self::get_current_default_source();

        // 1. Create SonicAura_Sink if not present
        let _ = Self::create_virtual_sink();

        // 2. Set SonicAura_Sink as default sink (YouTube/Chrome/Spotify outputs here)
        let _ = Self::set_default_sink(Self::SINK_NAME);

        // 3. Set SonicAura_Sink.monitor as default source (SonicAura captures from here)
        let _ = Self::set_default_source(&format!("{}.monitor", Self::SINK_NAME));

        (original_sink, original_source)
    }

    /// Checks if the virtual sink is currently loaded
    pub fn is_virtual_sink_loaded() -> Result<bool> {
        let output = Command::new("pactl")
            .args(["list", "sinks", "short"])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.contains(Self::SINK_NAME))
    }

    /// Restores original hardware output sink and unloads the virtual sink
    pub fn cleanup_and_restore(original_sink: Option<&str>, original_source: Option<&str>) {
        if !Self::is_pactl_available() {
            return;
        }

        if let Some(orig_s) = original_sink {
            let _ = Self::set_default_sink(orig_s);
        }

        if let Some(orig_src) = original_source {
            let _ = Self::set_default_source(orig_src);
        }

        let _ = Self::remove_virtual_sink();
    }

    /// Unloads any active SonicAura virtual sinks
    pub fn remove_virtual_sink() -> Result<()> {
        if !Self::is_pactl_available() {
            return Ok(());
        }

        let output = Command::new("pactl")
            .args(["list", "modules", "short"])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("module-null-sink") && line.contains(Self::SINK_NAME) {
                if let Some(id_str) = line.split_whitespace().next() {
                    let _ = Command::new("pactl").args(["unload-module", id_str]).output();
                }
            }
        }
        Ok(())
    }

    /// Returns the monitor source name for audio capture
    pub fn get_monitor_source_name() -> String {
        format!("{}.monitor", Self::SINK_NAME)
    }
}
