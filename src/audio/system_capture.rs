//! Real-Time Dynamic System Output Sound Capture Engine with Hot-Plug & Sink-Switching Watcher
//! Automatically follows whatever output device you select (SonicAura_Sink, Laptop Speakers,
//! Bluetooth Earphones, USB DAC, HDMI) in real time without stopping or needing a restart!

use crate::dsp::pipeline::SharedPipeline;
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct SystemSoundCapture {
    pub is_running: Arc<AtomicBool>,
    pub active_sink_name: Arc<Mutex<String>>,
    capture_thread: Option<JoinHandle<()>>,
    child_process: Option<Arc<Mutex<Option<Child>>>>,
}

impl SystemSoundCapture {
    /// Starts the dynamic auto-capturing engine that tracks the active sound output
    pub fn start_auto_capture(pipeline: SharedPipeline) -> Option<Self> {
        let is_running = Arc::new(AtomicBool::new(true));
        let active_sink_name = Arc::new(Mutex::new(Self::detect_active_output_sink().unwrap_or_else(|| "0".to_string())));

        let is_running_clone = Arc::clone(&is_running);
        let active_sink_clone = Arc::clone(&active_sink_name);
        let child_arc: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));
        let child_clone = Arc::clone(&child_arc);

        let capture_thread = thread::spawn(move || {
            Self::dynamic_capture_loop(pipeline, is_running_clone, active_sink_clone, child_clone);
        });

        Some(Self {
            is_running,
            active_sink_name,
            capture_thread: Some(capture_thread),
            child_process: Some(child_arc),
        })
    }

    /// Detects whichever sink is currently the system default output sink
    pub fn detect_active_output_sink() -> Option<String> {
        // 1. Try pactl get-default-sink (fastest & most accurate)
        if let Ok(output) = Command::new("pactl").arg("get-default-sink").output() {
            if output.status.success() {
                let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !name.is_empty() {
                    return Some(name);
                }
            }
        }

        // 2. Try pactl info fallback
        if let Ok(output) = Command::new("pactl").arg("info").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.starts_with("Default Sink:") {
                    let name = line.replace("Default Sink:", "").trim().to_string();
                    if !name.is_empty() {
                        return Some(name);
                    }
                }
            }
        }

        // 3. Try pactl list sinks short (find first available sink)
        if let Ok(output) = Command::new("pactl").args(["list", "sinks", "short"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    return Some(parts[1].to_string());
                }
            }
        }

        Some("0".to_string())
    }

    /// Spawns a low-latency PCM reader process for a given sink target
    fn spawn_reader_for_sink(sink: &str) -> Option<Child> {
        let has_pw_record = Command::new("pw-record")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if has_pw_record {
            Command::new("pw-record")
                .args([
                    "--target",
                    sink,
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
            let monitor_source = if sink.ends_with(".monitor") {
                sink.to_string()
            } else {
                format!("{}.monitor", sink)
            };
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
        }
    }

    fn dynamic_capture_loop(
        pipeline: SharedPipeline,
        is_running: Arc<AtomicBool>,
        active_sink_name: Arc<Mutex<String>>,
        child_holder: Arc<Mutex<Option<Child>>>,
    ) {
        let mut current_sink = Self::detect_active_output_sink().unwrap_or_else(|| "0".to_string());
        if let Ok(mut lock) = active_sink_name.lock() {
            *lock = current_sink.clone();
        }

        let mut child_opt = Self::spawn_reader_for_sink(&current_sink);
        let mut last_sink_check = std::time::Instant::now();
        let mut raw_buf = [0u8; 1024]; // 256 stereo 16-bit frames = ~5.3ms

        while is_running.load(Ordering::Relaxed) {
            // Check every 300ms if the user switched their output device in Sound Settings
            if last_sink_check.elapsed() >= Duration::from_millis(300) {
                last_sink_check = std::time::Instant::now();
                if let Some(new_sink) = Self::detect_active_output_sink() {
                    if new_sink != current_sink {
                        current_sink = new_sink.clone();
                        if let Ok(mut lock) = active_sink_name.lock() {
                            *lock = current_sink.clone();
                        }

                        // Terminate previous reader and spawn new reader on the newly selected output device
                        if let Some(mut old_child) = child_opt.take() {
                            let _ = old_child.kill();
                            let _ = old_child.wait();
                        }

                        child_opt = Self::spawn_reader_for_sink(&current_sink);
                    }
                }
            }

            // Read from current child stdout pipe
            let mut read_success = false;
            if let Some(ref mut child) = child_opt {
                if let Some(ref mut stdout) = child.stdout {
                    match stdout.read(&mut raw_buf) {
                        Ok(0) => {
                            // Stream paused or idle, sleep briefly
                            thread::sleep(Duration::from_millis(5));
                        }
                        Ok(bytes_read) => {
                            read_success = true;
                            let num_samples = bytes_read / 2;
                            let num_frames = num_samples / 2;

                            if let Ok(mut pl) = pipeline.lock() {
                                for frame_idx in 0..num_frames {
                                    let offset = frame_idx * 4;
                                    let s_l = i16::from_le_bytes([raw_buf[offset], raw_buf[offset + 1]]) as f32 / 32768.0;
                                    let s_r = i16::from_le_bytes([raw_buf[offset + 2], raw_buf[offset + 3]]) as f32 / 32768.0;

                                    // Push to AI analyzer & 32-Band FFT spectrum visualizer
                                    pl.ai_analyzer.push_sample(s_l, s_r);
                                }
                            }
                        }
                        Err(_) => {
                            thread::sleep(Duration::from_millis(5));
                        }
                    }
                }
            }

            // If reader failed or died, attempt respawn
            if !read_success && child_opt.is_none() {
                child_opt = Self::spawn_reader_for_sink(&current_sink);
                thread::sleep(Duration::from_millis(10));
            }
        }

        // Cleanup on shutdown
        if let Some(mut child) = child_opt {
            let _ = child.kill();
            let _ = child.wait();
        }
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
