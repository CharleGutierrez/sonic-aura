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

        if let Ok(output) = Command::new("pactl").arg("list").arg("short").arg("sinks").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // 1. High priority: Bluetooth or USB earphones/headphones
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() > 1 {
                    let name = parts[1].to_string();
                    if !name.contains(Self::SINK_NAME) && (name.contains("bluez") || name.contains("usb") || name.contains("headphone")) {
                        return Some(name);
                    }
                }
            }
            // 2. Secondary priority: Internal laptop speakers
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() > 1 {
                    let name = parts[1].to_string();
                    if !name.contains(Self::SINK_NAME) && (name.contains("analog") || name.contains("pci")) {
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

        if let Ok(output) = Command::new("pactl").arg("list").arg("short").arg("sources").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() > 1 {
                    let name = parts[1].to_string();
                    if !name.contains(Self::SINK_NAME) && !name.contains("monitor") {
                        return Some(name);
                    }
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

        // 4. Force unmute and set volume to 100% to ensure audio isn't lost
        let _ = Command::new("pactl").args(["set-sink-mute", Self::SINK_NAME, "0"]).output();
        let _ = Command::new("pactl").args(["set-sink-volume", Self::SINK_NAME, "100%"]).output();

        (original_sink, original_source)
    }

    /// Continuously monitors audio streams to ensure:
    /// 1. SonicAura's playback stream is ALWAYS routed to target_sink (never trapped in SonicAura_Sink)
    /// 2. When SonicAura_Sink is selected as default, user media (YouTube/Chrome) is automatically moved into SonicAura_Sink
    /// 3. SonicAura_Sink remains unmuted at 100% volume even across GNOME output switches
    pub fn start_output_enforcer(target_sink: String) {
        std::thread::spawn(move || {
            let mut last_synced_vol: Option<String> = None;
            let mut last_synced_mute: Option<bool> = None;
            let mut last_known_physical = target_sink.clone();

            loop {
                // Check what the current default sink is in GNOME / PipeWire
                let current_def = Command::new("pactl")
                    .arg("get-default-sink")
                    .output()
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                    .unwrap_or_default();

                let default_is_sonic_aura = current_def.contains(Self::SINK_NAME);

                // If user selected a physical sink in GNOME (Speakers or Bluetooth), remember it
                if !default_is_sonic_aura && !current_def.is_empty() {
                    last_known_physical = current_def.clone();
                }

                // Dynamically resolve target physical hardware sink (Bluetooth or Speakers)
                let active_target = if default_is_sonic_aura {
                    Self::get_current_default_sink().unwrap_or_else(|| last_known_physical.clone())
                } else {
                    last_known_physical.clone()
                };

                let mut virtual_sink_id = None;
                let mut target_sink_id = None;

                if let Ok(output) = Command::new("pactl").args(["list", "sinks", "short"]).output() {
                    let s = String::from_utf8_lossy(&output.stdout);
                    for line in s.lines() {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() > 1 {
                            let idx = parts[0].parse::<u32>().ok();
                            let name = parts[1];
                            if name.contains(Self::SINK_NAME) {
                                virtual_sink_id = idx;
                            }
                            if name == active_target {
                                target_sink_id = idx;
                            }
                        }
                    }
                }

                // Synchronize volume and mute adjustments made by the user in GNOME / keyboard keys
                if default_is_sonic_aura {
                    if let Ok(vol_out) = Command::new("pactl").args(["get-sink-volume", Self::SINK_NAME]).output() {
                        let s = String::from_utf8_lossy(&vol_out.stdout);
                        if let Some(pct) = s.split('/').nth(1).map(|p| p.trim().to_string()) {
                            if last_synced_vol.as_ref() != Some(&pct) {
                                let _ = Command::new("pactl").args(["set-sink-volume", &active_target, &pct]).output();
                                last_synced_vol = Some(pct);
                            }
                        }
                    }
                    if let Ok(mute_out) = Command::new("pactl").args(["get-sink-mute", Self::SINK_NAME]).output() {
                        let s = String::from_utf8_lossy(&mute_out.stdout);
                        let is_muted = s.contains("yes");
                        if last_synced_mute != Some(is_muted) {
                            let _ = Command::new("pactl").args(["set-sink-mute", &active_target, if is_muted { "1" } else { "0" }]).output();
                            last_synced_mute = Some(is_muted);
                        }
                    }
                }

                if let Ok(output) = Command::new("pactl").args(["list", "sink-inputs"]).output() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    
                    let mut streams: Vec<(u32, Option<u32>, bool)> = Vec::new();
                    let mut cur_id: Option<u32> = None;
                    let mut cur_sink: Option<u32> = None;
                    let mut cur_is_app = false;

                    for line in stdout.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("Sink Input #") {
                            if let Some(id) = cur_id {
                                streams.push((id, cur_sink, cur_is_app));
                            }
                            cur_id = trimmed.replace("Sink Input #", "").parse::<u32>().ok();
                            cur_sink = None;
                            cur_is_app = false;
                        } else if trimmed.starts_with("Sink:") {
                            cur_sink = trimmed.replace("Sink:", "").trim().parse::<u32>().ok();
                        } else {
                            let lower = trimmed.to_lowercase();
                            // Reliably match PipeWire ALSA [sonic_aura], ALSA plug-in [sonic_aura], binary, etc.
                            if (lower.starts_with("application.name =") || lower.starts_with("node.name =") || lower.starts_with("device.description ="))
                                && (lower.contains("sonic_aura") || lower.contains("sonic-aura"))
                            {
                                cur_is_app = true;
                            }
                        }
                    }
                    if let Some(id) = cur_id {
                        streams.push((id, cur_sink, cur_is_app));
                    }

                    for (id, sink_id, is_app) in streams {
                        if is_app {
                            // SonicAura's processed playback must ALWAYS play to physical hardware, never virtual sink!
                            let needs_move = match (sink_id, target_sink_id) {
                                (Some(cur), Some(target)) => cur != target,
                                (Some(cur), None) => Some(cur) == virtual_sink_id,
                                _ => true,
                            };
                            if needs_move {
                                let _ = Command::new("pactl")
                                    .args(["move-sink-input", &id.to_string(), &active_target])
                                    .output();
                            }
                        } else if default_is_sonic_aura {
                            // If SonicAura is selected in GNOME, make sure media apps (Chrome/YouTube) are routed into SonicAura_Sink
                            let is_on_virtual = match (sink_id, virtual_sink_id) {
                                (Some(cur), Some(virt)) => cur == virt,
                                _ => false,
                            };
                            if !is_on_virtual {
                                let _ = Command::new("pactl")
                                    .args(["move-sink-input", &id.to_string(), Self::SINK_NAME])
                                    .output();
                            }
                        }
                    }
                }

                std::thread::sleep(std::time::Duration::from_millis(150));
            }
        });
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
                    let _ = Command::new("pactl")
                        .args(["unload-module", id_str])
                        .output();
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
