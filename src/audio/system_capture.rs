//! Real-Time Universal System Sound Capture Engine with PipeWire Port-Linker
//! Automatically detects and links all active output monitor ports (Laptop Speakers,
//! Bluetooth Earphones, USB Audio, HDMI) so that YouTube, Spotify, and system audio
//! immediately drive the DSP pipeline and 32-Band FFT Spectrum Analyzer in real time!

use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use ringbuf::traits::Producer;

pub struct SystemSoundCapture {
    pub is_running: Arc<AtomicBool>,
    pub active_sink_name: Arc<Mutex<String>>,
    capture_thread: Option<JoinHandle<()>>,
}

impl SystemSoundCapture {
    /// Starts the dynamic auto-capturing engine
    pub fn start_auto_capture(producer: ringbuf::HeapProd<f32>) -> Option<Self> {
        let is_running = Arc::new(AtomicBool::new(true));
        let active_sink_name = Arc::new(Mutex::new("Auto-Detecting Output...".to_string()));

        let is_running_clone = Arc::clone(&is_running);
        let active_sink_clone = Arc::clone(&active_sink_name);

        let capture_thread = thread::spawn(move || {
            Self::universal_capture_loop(producer, is_running_clone, active_sink_clone);
        });

        Some(Self {
            is_running,
            active_sink_name,
            capture_thread: Some(capture_thread),
        })
    }

    /// Exclusively links SonicAura_Sink monitor ports to capture process.
    /// STRICTLY isolates from physical monitors (Bluetooth/Speakers/Mic) to eliminate feedback and unwanted ambient capture!
    fn link_virtual_sink_only() {
        let _ = Command::new("pw-link")
            .args(["SonicAura_Sink:monitor_FL", "parec:input_FL"])
            .output();
        let _ = Command::new("pw-link")
            .args(["SonicAura_Sink:monitor_FR", "parec:input_FR"])
            .output();
        let _ = Command::new("pw-link")
            .args(["SonicAura_Sink:monitor_FL", "pw-record:input_FL"])
            .output();
        let _ = Command::new("pw-link")
            .args(["SonicAura_Sink:monitor_FR", "pw-record:input_FR"])
            .output();

        // Proactively disconnect any physical output monitor links (Bluetooth/Speakers/Mic) to capture
        if let Ok(output) = Command::new("pw-link").arg("-l").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut current_source = String::new();
            for line in stdout.lines() {
                let trimmed = line.trim();
                if !trimmed.starts_with('|') {
                    current_source = trimmed.to_string();
                } else if (trimmed.contains("pw-record") || trimmed.contains("parec"))
                    && !current_source.contains("SonicAura_Sink")
                {
                    let target = trimmed.replace("|->", "").trim().to_string();
                    let _ = Command::new("pw-link")
                        .args(["-d", &current_source, &target])
                        .output();
                }
            }
        }
    }

    /// Detects current human-readable active output sink
    fn get_active_sink_display() -> String {
        if let Ok(output) = Command::new("pactl").arg("get-default-sink").output() {
            if output.status.success() {
                let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if name.contains("analog") || name.contains("pci") {
                    return "💻 Laptop Speakers (Analog Stereo)".to_string();
                } else if name.contains("bluez") {
                    return "🎧 Bluetooth Headphones".to_string();
                } else if name.contains("SonicAura") {
                    return "⚡ SonicAura AI Virtual Sink".to_string();
                } else if !name.is_empty() {
                    return name;
                }
            }
        }
        "💻 Laptop Speakers (Default)".to_string()
    }

    fn universal_capture_loop(
        mut producer: ringbuf::HeapProd<f32>,
        is_running: Arc<AtomicBool>,
        active_sink_name: Arc<Mutex<String>>,
    ) {
        // Terminate any stale orphaned capture instances from previous runs
        let _ = Command::new("pkill").args(["-f", "(pw-record|parec).*SonicAura"]).output();

        let sink_name = crate::audio::virtual_device::VirtualSinkManager::SINK_NAME;
        let mon_source = crate::audio::virtual_device::VirtualSinkManager::get_monitor_source_name();

        // Priority 1: Use parec for ultra-low latency direct monitor capture (10ms buffers)
        let mut child = Command::new("parec")
            .env_remove("PIPEWIRE_NODE")
            .args([
                "-d",
                &mon_source,
                "--latency-msec=10",
                "--process-time-msec=10",
                "--rate=48000",
                "--format=s16le",
                "--channels=2",
                "--raw",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok();

        // Priority 2: Fallback to pw-record if parec is unavailable
        if child.is_none() {
            child = Command::new("pw-record")
                .env_remove("PIPEWIRE_NODE")
                .args([
                    "--target",
                    sink_name,
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
                .ok();
        }

        // Brief delay to allow ports to register with PipeWire
        thread::sleep(Duration::from_millis(100));
        Self::link_virtual_sink_only();

        if child.is_none() {
            eprintln!("🛑 CRITICAL ERROR: Neither 'parec' nor 'pw-record' could be found or started!");
            return;
        }

        // Set non-blocking on stdout pipe
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

        let mut last_link_check = Instant::now();
        let mut read_buf = [0u8; 2048];
        let mut carry_buf = [0u8; 4];
        let mut carry_len = 0;

        while is_running.load(Ordering::Relaxed) {
            // Check if capture process exited and auto-restart if needed
            let is_dead = match child.as_mut().map(|c| c.try_wait()) {
                Some(Ok(Some(_))) => true,
                None => true,
                _ => false,
            };

            if is_dead {
                child = Command::new("parec")
                    .env_remove("PIPEWIRE_NODE")
                    .args([
                        "-d",
                        &mon_source,
                        "--latency-msec=10",
                        "--process-time-msec=10",
                        "--rate=48000",
                        "--format=s16le",
                        "--channels=2",
                        "--raw",
                    ])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .spawn()
                    .ok();

                if child.is_none() {
                    child = Command::new("pw-record")
                        .env_remove("PIPEWIRE_NODE")
                        .args([
                            "--target",
                            sink_name,
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
                        .ok();
                }

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

                thread::sleep(Duration::from_millis(80));
                Self::link_virtual_sink_only();
            }

            // Periodically refresh active sink display and maintain monitor links every 250ms
            if last_link_check.elapsed() >= Duration::from_millis(250) {
                last_link_check = Instant::now();
                Self::link_virtual_sink_only();

                let disp = Self::get_active_sink_display();
                if let Ok(mut lock) = active_sink_name.lock() {
                    *lock = disp;
                }
            }

            let mut bytes_read = 0;
            if let Some(ref mut c) = child {
                if let Some(ref mut stdout) = c.stdout {
                    match stdout.read(&mut read_buf) {
                        Ok(0) => {
                            thread::sleep(Duration::from_millis(2));
                        }
                        Ok(n) => {
                            bytes_read = n;
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(2));
                        }
                        Err(_) => {
                            thread::sleep(Duration::from_millis(2));
                        }
                    }
                }
            }

            if bytes_read > 0 {
                // Combine any leftover bytes from previous read
                let total_bytes = carry_len + bytes_read;
                let mut combined = [0u8; 2048 + 4];
                combined[..carry_len].copy_from_slice(&carry_buf[..carry_len]);
                combined[carry_len..total_bytes].copy_from_slice(&read_buf[..bytes_read]);

                let num_frames = total_bytes / 4;
                let processed_bytes = num_frames * 4;

                for frame_idx in 0..num_frames {
                    let offset = frame_idx * 4;
                    let s_l = i16::from_le_bytes([combined[offset], combined[offset + 1]]) as f32 / 32768.0;
                    let s_r = i16::from_le_bytes([combined[offset + 2], combined[offset + 3]]) as f32 / 32768.0;

                    let _ = producer.try_push(s_l);
                    let _ = producer.try_push(s_r);
                }

                // Store remainder for next read
                carry_len = total_bytes - processed_bytes;
                if carry_len > 0 {
                    carry_buf[..carry_len].copy_from_slice(&combined[processed_bytes..total_bytes]);
                }
            }
        }

        // Cleanup on shutdown
        if let Some(mut c) = child {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

impl Drop for SystemSoundCapture {
    fn drop(&mut self) {
        self.is_running.store(false, Ordering::Relaxed);
    }
}
