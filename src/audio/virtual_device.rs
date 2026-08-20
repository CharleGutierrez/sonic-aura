//! Virtual Audio Sink Manager for PipeWire and PulseAudio
//! Creates virtual audio loopback sinks so all computer system audio
//! (Spotify, YouTube, Games, Netflix) routes seamlessly through SonicAura AI.

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

    /// Creates a virtual null-sink for system audio capture
    pub fn create_virtual_sink() -> Result<u32> {
        if !Self::is_pactl_available() {
            anyhow::bail!("PulseAudio/PipeWire 'pactl' utility is not found. Please ensure pipewire-pulse or pulseaudio is installed.");
        }

        // First check if sink already exists
        if let Ok(true) = Self::is_virtual_sink_loaded() {
            println!("Virtual sink '{}' is already active.", Self::SINK_NAME);
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
            anyhow::bail!("pactl command failed: {}", err);
        }

        let module_id_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let module_id = module_id_str.parse::<u32>().unwrap_or(0);
        Ok(module_id)
    }

    /// Checks if the virtual sink is currently loaded
    pub fn is_virtual_sink_loaded() -> Result<bool> {
        let output = Command::new("pactl")
            .args(["list", "sinks", "short"])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.contains(Self::SINK_NAME))
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
