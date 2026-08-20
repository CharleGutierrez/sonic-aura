//! Real-Time Universal System Sound Capture Engine with PipeWire Port-Linker
//! Automatically detects and links all active output monitor ports (Laptop Speakers,
//! Bluetooth Earphones, USB Audio, HDMI) so that YouTube, Spotify, and system audio
//! immediately drive the 32-Band FFT Spectrum Analyzer and VU Meters in real time!

use crate::dsp::pipeline::SharedPipeline;
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::process::{Command, Stdio};
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
    /// Starts the dynamic auto-capturing engine
    pub fn start_auto_capture(pipeline: SharedPipeline) -> Option<Self> {
        let is_running = Arc::new(AtomicBool::new(true));
        let active_sink_name = Arc::new(Mutex::new("Auto-Detecting Output...".to_string()));

        let is_running_clone = Arc::clone(&is_running);
        let active_sink_clone = Arc::clone(&active_sink_name);

        let capture_thread = thread::spawn(move || {
            Self::universal_capture_loop(pipeline, is_running_clone, active_sink_clone);
        });

        Some(Self {
            is_running,
            active_sink_name,
            capture_thread: Some(capture_thread),
        })
    }

    /// Links all available output monitor ports to pw-record
    fn link_all_monitors() {
        if let Ok(output) = Command::new("pw-link").arg("-o").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let port = line.trim();
                if port.contains("monitor") && !port.contains("pw-record") {
                    if port.contains("FL") || port.contains("left") || port.ends_with("_1") {
                        let _ = Command::new("pw-link")
                            .args([port, "pw-record:input_FL"])
                            .output();
                    } else if port.contains("FR") || port.contains("right") || port.ends_with("_2") {
                        let _ = Command::new("pw-link")
                            .args([port, "pw-record:input_FR"])
                            .output();
                    } else {
                        let _ = Command::new("pw-link")
                            .args([port, "pw-record:input_FL"])
                            .output();
                        let _ = Command::new("pw-link")
                            .args([port, "pw-record:input_FR"])
                            .output();
                    }
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
        pipeline: SharedPipeline,
        is_running: Arc<AtomicBool>,
        active_sink_name: Arc<Mutex<String>>,
    ) {
        // Spawn pw-record process with unlinked target (target 0)
        let mut child = Command::new("pw-record")
            .args([
                "--target",
                "0",
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

        // Fallback to parec if pw-record is unavailable
        if child.is_none() {
            child = Command::new("parec")
                .args([
                    "-d",
                    "@DEFAULT_SINK@.monitor",
                    "--format=s16le",
                    "--rate=48000",
                    "--channels=2",
                    "--raw",
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .ok();
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

        // Brief delay to allow pw-record ports to register with PipeWire
        thread::sleep(Duration::from_millis(80));
        Self::link_all_monitors();

        let mut last_link_check = Instant::now();
        let mut raw_buf = [0u8; 1024];

        while is_running.load(Ordering::Relaxed) {
            // Periodically refresh links and active sink display every 400ms
            if last_link_check.elapsed() >= Duration::from_millis(400) {
                last_link_check = Instant::now();
                Self::link_all_monitors();

                let disp = Self::get_active_sink_display();
                if let Ok(mut lock) = active_sink_name.lock() {
                    *lock = disp;
                }
            }

            let mut read_bytes = 0;
            if let Some(ref mut c) = child {
                if let Some(ref mut stdout) = c.stdout {
                    match stdout.read(&mut raw_buf) {
                        Ok(0) => {
                            thread::sleep(Duration::from_millis(3));
                        }
                        Ok(n) => {
                            read_bytes = n;
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(3));
                        }
                        Err(_) => {
                            thread::sleep(Duration::from_millis(3));
                        }
                    }
                }
            }

            // Feed captured samples directly into DSP pipeline & 32-band FFT analyzer
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
