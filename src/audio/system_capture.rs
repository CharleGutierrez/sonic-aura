//! Real-Time Non-Blocking Dynamic System Sound Capture Engine
//! Features:
//! - Non-blocking asynchronous pipe I/O (prevents hanging when sinks are suspended or idle)
//! - Fast 150ms sink change polling & debounced switching (immune to rapid toggling)
//! - Auto-recovery watchdog (detects dead or crashed capture processes and respawns cleanly)
//! - Tracks any active sound output (Laptop Speakers, Bluetooth Earbuds, USB Audio, HDMI, Virtual Sink)

use crate::dsp::pipeline::SharedPipeline;
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub struct SystemSoundCapture {
    pub is_running: Arc<AtomicBool>,
    pub active_sink_name: Arc<Mutex<String>>,
    capture_thread: Option<JoinHandle<()>>,
}

impl SystemSoundCapture {
    /// Starts the dynamic auto-capturing engine with non-blocking pipe I/O and rapid sink-switching immunity
    pub fn start_auto_capture(pipeline: SharedPipeline) -> Option<Self> {
        let is_running = Arc::new(AtomicBool::new(true));
        let active_sink_name = Arc::new(Mutex::new(Self::detect_active_output_sink().unwrap_or_else(|| "0".to_string())));

        let is_running_clone = Arc::clone(&is_running);
        let active_sink_clone = Arc::clone(&active_sink_name);

        let capture_thread = thread::spawn(move || {
            Self::supervisor_loop(pipeline, is_running_clone, active_sink_clone);
        });

        Some(Self {
            is_running,
            active_sink_name,
            capture_thread: Some(capture_thread),
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

    /// Spawns a low-latency PCM reader process with non-blocking stdout pipe
    fn spawn_nonblocking_reader(sink: &str) -> Option<Child> {
        let has_pw_record = Command::new("pw-record")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        let mut child = if has_pw_record {
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
        };

        // Set O_NONBLOCK on the child stdout pipe so read() NEVER hangs indefinitely on dead/suspended sinks
        if let Some(ref mut c) = child {
            if let Some(ref stdout) = c.stdout {
                let fd = stdout.as_raw_fd();
                unsafe {
                    let flags = libc::fcntl(fd, libc::F_GETFL, 0);
                    if flags >= 0 {
                        libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
                    }
                }
            }
        }

        child
    }

    /// Cleanly terminates a child process without blocking the thread
    fn kill_child(mut child: Child) {
        let _ = child.kill();
        let _ = child.try_wait();
    }

    fn supervisor_loop(
        pipeline: SharedPipeline,
        is_running: Arc<AtomicBool>,
        active_sink_name: Arc<Mutex<String>>,
    ) {
        let mut current_sink = Self::detect_active_output_sink().unwrap_or_else(|| "0".to_string());
        if let Ok(mut lock) = active_sink_name.lock() {
            *lock = current_sink.clone();
        }

        let mut child_opt = Self::spawn_nonblocking_reader(&current_sink);
        let mut last_sink_poll = Instant::now();
        let mut last_valid_read = Instant::now();
        let mut pending_new_sink: Option<String> = None;
        let mut debounce_timer = Instant::now();
        let mut raw_buf = [0u8; 1024];

        while is_running.load(Ordering::Relaxed) {
            // 1. Poll for sink changes every 150ms with 100ms anti-flapping debounce
            if last_sink_poll.elapsed() >= Duration::from_millis(150) {
                last_sink_poll = Instant::now();
                if let Some(detected) = Self::detect_active_output_sink() {
                    if detected != current_sink {
                        if pending_new_sink.as_ref() == Some(&detected) {
                            if debounce_timer.elapsed() >= Duration::from_millis(100) {
                                // Apply the debounced sink change
                                current_sink = detected.clone();
                                pending_new_sink = None;

                                if let Ok(mut lock) = active_sink_name.lock() {
                                    *lock = current_sink.clone();
                                }

                                if let Some(old_child) = child_opt.take() {
                                    Self::kill_child(old_child);
                                }
                                child_opt = Self::spawn_nonblocking_reader(&current_sink);
                                last_valid_read = Instant::now();
                            }
                        } else {
                            pending_new_sink = Some(detected);
                            debounce_timer = Instant::now();
                        }
                    } else {
                        pending_new_sink = None;
                    }
                }
            }

            // 2. Check health of active child reader
            let mut is_child_dead = false;
            if let Some(ref mut child) = child_opt {
                if let Ok(Some(_)) = child.try_wait() {
                    is_child_dead = true;
                }
            } else {
                is_child_dead = true;
            }

            if is_child_dead {
                if let Some(old_child) = child_opt.take() {
                    Self::kill_child(old_child);
                }
                child_opt = Self::spawn_nonblocking_reader(&current_sink);
                last_valid_read = Instant::now();
                thread::sleep(Duration::from_millis(10));
                continue;
            }

            // 3. Non-blocking read from child stdout
            let mut read_bytes = 0;
            if let Some(ref mut child) = child_opt {
                if let Some(ref mut stdout) = child.stdout {
                    match stdout.read(&mut raw_buf) {
                        Ok(0) => {
                            // Pipe closed or EOF
                            thread::sleep(Duration::from_millis(4));
                        }
                        Ok(n) => {
                            read_bytes = n;
                            last_valid_read = Instant::now();
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            // Non-blocking: no data currently available (sink idle or suspended)
                            thread::sleep(Duration::from_millis(4));
                        }
                        Err(_) => {
                            thread::sleep(Duration::from_millis(4));
                        }
                    }
                }
            }

            // 4. Feed audio data into pipeline to process DSP and update 32-Band FFT spectrum
            if read_bytes > 0 {
                let num_samples = read_bytes / 2;
                let num_frames = num_samples / 2;

                if let Ok(mut pl) = pipeline.lock() {
                    for frame_idx in 0..num_frames {
                        let offset = frame_idx * 4;
                        let s_l = i16::from_le_bytes([raw_buf[offset], raw_buf[offset + 1]]) as f32 / 32768.0;
                        let s_r = i16::from_le_bytes([raw_buf[offset + 2], raw_buf[offset + 3]]) as f32 / 32768.0;

                        let _ = pl.process_stereo_sample(s_l, s_r);
                    }
                }
            }

            // 5. Watchdog: If child has been completely silent for > 3.0s and another sink exists, trigger a clean re-check
            if last_valid_read.elapsed() >= Duration::from_secs(3) {
                last_valid_read = Instant::now();
                if let Some(new_candidate) = Self::detect_active_output_sink() {
                    if new_candidate != current_sink {
                        current_sink = new_candidate.clone();
                        if let Ok(mut lock) = active_sink_name.lock() {
                            *lock = current_sink.clone();
                        }
                        if let Some(old_child) = child_opt.take() {
                            Self::kill_child(old_child);
                        }
                        child_opt = Self::spawn_nonblocking_reader(&current_sink);
                    }
                }
            }
        }

        // Clean up child on shutdown
        if let Some(child) = child_opt.take() {
            Self::kill_child(child);
        }
    }
}

impl Drop for SystemSoundCapture {
    fn drop(&mut self) {
        self.is_running.store(false, Ordering::Relaxed);
    }
}
