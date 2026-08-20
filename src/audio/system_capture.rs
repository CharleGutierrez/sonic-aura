//! Real-Time Automatic System Output Sound Capture Engine
//! Automatically detects whatever audio is playing on your laptop (YouTube, Spotify, Games, VLC)
//! through the laptop speakers, external soundbar, or Bluetooth headphones, and streams it directly
//! into the SonicAura AI DSP and 32-Band FFT Spectrum Analyzer in real time!

use crate::dsp::pipeline::SharedPipeline;
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

pub struct SystemSoundCapture {
    pub is_running: Arc<AtomicBool>,
    pub detected_sink_name: String,
    capture_thread: Option<JoinHandle<()>>,
    child_process: Option<Arc<std::sync::Mutex<Option<Child>>>>,
}

impl SystemSoundCapture {
    /// Automatically detects the active laptop output sink and starts real-time monitor capture
    pub fn start_auto_capture(pipeline: SharedPipeline) -> Option<Self> {
        let detected_sink = Self::detect_active_output_sink()?;
        let is_running = Arc::new(AtomicBool::new(true));

        let is_running_clone = Arc::clone(&is_running);
        let sink_target = detected_sink.clone();

        let child_arc: Arc<std::sync::Mutex<Option<Child>>> = Arc::new(std::sync::Mutex::new(None));
        let child_clone = Arc::clone(&child_arc);

        let capture_thread = thread::spawn(move || {
            Self::capture_loop(sink_target, pipeline, is_running_clone, child_clone);
        });

        Some(Self {
            is_running,
            detected_sink_name: detected_sink,
            capture_thread: Some(capture_thread),
            child_process: Some(child_arc),
        })
    }

    /// Detects active laptop sound output sink (e.g. Built-in Speakers, Bluetooth, or Default Sink)
    pub fn detect_active_output_sink() -> Option<String> {
        // 1. Try pactl get-default-sink
        if let Ok(output) = Command::new("pactl").arg("get-default-sink").output() {
            if output.status.success() {
                let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !name.is_empty() && !name.contains("SonicAura") {
                    return Some(name);
                }
            }
        }

        // 2. Try pactl list sinks short
        if let Ok(output) = Command::new("pactl").args(["list", "sinks", "short"]).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let sink_name = parts[1];
                        if !sink_name.contains("SonicAura") {
                            // Prefer built-in laptop speakers if present
                            if sink_name.contains("analog-stereo") || sink_name.contains("pci") {
                                return Some(sink_name.to_string());
                            }
                        }
                    }
                }
                // Fallback to first available sink
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let sink_name = parts[1];
                        if !sink_name.contains("SonicAura") {
                            return Some(sink_name.to_string());
                        }
                    }
                }
            }
        }

        // 3. Fallback to PipeWire auto target "0"
        Some("0".to_string())
    }

    fn capture_loop(
        sink_target: String,
        pipeline: SharedPipeline,
        is_running: Arc<AtomicBool>,
        child_holder: Arc<std::sync::Mutex<Option<Child>>>,
    ) {
        // Prefer pw-record if available (native PipeWire), fallback to parec
        let has_pw_record = Command::new("pw-record")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        let mut child = if has_pw_record {
            Command::new("pw-record")
                .args([
                    "--target",
                    &sink_target,
                    "--format",
                    "s16",
                    "--rate",
                    "48000",
                    "--channels",
                    "2",
                    "--latency",
                    "256",
                    "-",
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .ok()
        } else {
            let monitor_source = format!("{}.monitor", sink_target);
            Command::new("parec")
                .args([
                    "-d",
                    &monitor_source,
                    "--format=s16le",
                    "--rate=48000",
                    "--channels=2",
                    "--raw",
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .ok()
        };

        if let Some(ref mut _c) = child {
            if let Ok(mut lock) = child_holder.lock() {
                *lock = child.take();
            }
        }

        let stdout_pipe = {
            if let Ok(mut lock) = child_holder.lock() {
                if let Some(ref mut c) = *lock {
                    c.stdout.take()
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(mut reader) = stdout_pipe {
            let mut raw_buf = [0u8; 1024]; // 256 stereo 16-bit samples

            while is_running.load(Ordering::Relaxed) {
                match reader.read(&mut raw_buf) {
                    Ok(0) => {
                        // EOF or paused stream, sleep briefly
                        thread::sleep(std::time::Duration::from_millis(5));
                    }
                    Ok(bytes_read) => {
                        // Convert i16 PCM samples to f32 and feed pipeline
                        let num_samples = bytes_read / 2;
                        let num_frames = num_samples / 2;

                        if let Ok(mut pl) = pipeline.lock() {
                            for frame_idx in 0..num_frames {
                                let offset = frame_idx * 4;
                                let s_l = i16::from_le_bytes([raw_buf[offset], raw_buf[offset + 1]]) as f32 / 32768.0;
                                let s_r = i16::from_le_bytes([raw_buf[offset + 2], raw_buf[offset + 3]]) as f32 / 32768.0;
                                
                                // Push directly to AI analyzer and visualizer
                                pl.ai_analyzer.push_sample(s_l, s_r);
                            }
                        }
                    }
                    Err(_) => {
                        thread::sleep(std::time::Duration::from_millis(5));
                    }
                }
            }
        }

        // Cleanup child process on exit
        if let Ok(mut lock) = child_holder.lock() {
            if let Some(mut c) = lock.take() {
                let _ = c.kill();
                let _ = c.wait();
            }
        }
    }
}

impl Drop for SystemSoundCapture {
    fn drop(&mut self) {
        self.is_running.store(false, Ordering::Relaxed);
        if let Some(ref child_arc) = self.child_process {
            if let Ok(mut lock) = child_arc.lock() {
                if let Some(mut c) = lock.take() {
                    let _ = c.kill();
                    let _ = c.wait();
                }
            }
        }
    }
}
