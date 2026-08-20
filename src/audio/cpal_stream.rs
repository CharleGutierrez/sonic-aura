//! Real-Time Low-Latency CPAL Audio Playback & Synthesis Engine

use crate::audio::test_synth::{SynthTone, TestSynth};
use crate::dsp::pipeline::SharedPipeline;
use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Host, Stream, StreamConfig};
use ringbuf::traits::{Consumer, Observer, Producer, Split};
use ringbuf::HeapRb;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[cfg(target_os = "linux")]
fn silence_alsa_logging() {
    unsafe {
        extern "C" fn dummy_handler(
            _file: *const std::ffi::c_char,
            _line: std::ffi::c_int,
            _function: *const std::ffi::c_char,
            _err: std::ffi::c_int,
            _fmt: *const std::ffi::c_char,
        ) {}

        let handle = libc::dlopen(b"libasound.so.2\0".as_ptr() as *const _, libc::RTLD_LAZY);
        if !handle.is_null() {
            let symbol = libc::dlsym(handle, b"snd_lib_error_set_handler\0".as_ptr() as *const _);
            if !symbol.is_null() {
                let set_handler: extern "C" fn(*const ()) -> std::ffi::c_int = std::mem::transmute(symbol);
                set_handler(dummy_handler as *const ());
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn silence_alsa_logging() {}

pub struct AudioEngine {
    _host: Host,
    pub input_device_name: String,
    pub output_device_name: String,
    pub sample_rate: u32,
    pub is_running: Arc<AtomicBool>,
    pub synth_enabled: Arc<AtomicBool>,
    _input_stream: Option<Stream>,
    _output_stream: Option<Stream>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EngineMode {
    LoopbackLive,
    TestSynth,
}

impl AudioEngine {
    pub fn get_available_devices() -> (Vec<String>, Vec<String>) {
        let host = cpal::default_host();
        let inputs = host
            .input_devices()
            .map(|devices| devices.filter_map(|d| d.name().ok()).collect())
            .unwrap_or_default();
        let outputs = host
            .output_devices()
            .map(|devices| devices.filter_map(|d| d.name().ok()).collect())
            .unwrap_or_default();
        (inputs, outputs)
    }

    pub fn start(
        pipeline: SharedPipeline,
        mode: EngineMode,
        preferred_input: Option<String>,
        preferred_output: Option<String>,
        synth_tone: SynthTone,
    ) -> Result<Self> {
        silence_alsa_logging();

        let host = cpal::default_host();

        // 1. Select Physical Output Device
        let output_device = if let Some(ref name) = preferred_output {
            host.output_devices()?
                .find(|d| d.name().map(|n| n == *name).unwrap_or(false))
                .or_else(|| host.default_output_device())
        } else {
            host.default_output_device()
        }
        .context("No audio output device found on system")?;

        let output_device_name = output_device.name().unwrap_or_else(|_| "Default Output".to_string());

        let default_out_config = output_device.default_output_config()?;
        let sample_rate = default_out_config.sample_rate();
        let out_channels = default_out_config.channels() as usize;

        {
            let mut pl = pipeline.lock().unwrap();
            pl.set_sample_rate(sample_rate as f32);
        }

        let is_running = Arc::new(AtomicBool::new(true));
        let synth_enabled = Arc::new(AtomicBool::new(mode == EngineMode::TestSynth));

        let out_config = StreamConfig {
            channels: default_out_config.channels(),
            sample_rate,
            buffer_size: cpal::BufferSize::Fixed(512),
        };

        let ring_buffer = HeapRb::<f32>::new(16384);
        let (mut producer, mut consumer) = ring_buffer.split();

        let mut synth = TestSynth::new(sample_rate as f32);
        synth.set_tone_type(synth_tone);

        let mut _input_stream_opt: Option<Stream> = None;
        let mut input_device_name = "None".to_string();

        if let Some(input_device) = if let Some(ref name) = preferred_input {
            host.input_devices()?
                .find(|d| d.name().map(|n| n == *name).unwrap_or(false))
                .or_else(|| host.default_input_device())
        } else {
            host.default_input_device()
        } {
            input_device_name = input_device.name().unwrap_or_else(|_| "Default Input".to_string());
            if let Ok(default_in_config) = input_device.default_input_config() {
                let in_channels = default_in_config.channels() as usize;
                let in_config = StreamConfig {
                    channels: default_in_config.channels(),
                    sample_rate,
                    buffer_size: cpal::BufferSize::Fixed(512),
                };

                let err_fn_in = |_| {};
                if let Ok(input_stream) = input_device.build_input_stream(
                    &in_config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        for frame in data.chunks_exact(in_channels) {
                            let l = frame[0];
                            let r = if frame.len() > 1 { frame[1] } else { l };
                            let _ = producer.try_push(l);
                            let _ = producer.try_push(r);
                        }
                    },
                    err_fn_in,
                    None,
                ) {
                    let _ = input_stream.play();
                    _input_stream_opt = Some(input_stream);
                }
            }
        }

        let pl_clone = Arc::clone(&pipeline);
        let synth_flag = Arc::clone(&synth_enabled);

        let err_fn_out = |_| {};
        let output_stream = output_device.build_output_stream(
            &out_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let is_synth = synth_flag.load(Ordering::Relaxed);

                if is_synth {
                    if let Ok(mut pl) = pl_clone.lock() {
                        for frame in data.chunks_exact_mut(out_channels) {
                            let (syn_l, syn_r) = synth.next_sample();
                            let (out_l, out_r) = pl.process_stereo_sample(syn_l, syn_r);
                            frame[0] = out_l;
                            if frame.len() > 1 {
                                frame[1] = out_r;
                            }
                            for extra in frame.iter_mut().skip(2) {
                                *extra = 0.0;
                            }
                        }
                    } else {
                        data.fill(0.0);
                    }
                } else {
                    // Check if consumer has audio from input/loopback stream
                    if consumer.occupied_len() >= 2 {
                        if let Ok(mut pl) = pl_clone.lock() {
                            for frame in data.chunks_exact_mut(out_channels) {
                                if let (Some(in_l), Some(in_r)) = (consumer.try_pop(), consumer.try_pop()) {
                                    let (out_l, out_r) = pl.process_stereo_sample(in_l, in_r);
                                    frame[0] = out_l;
                                    if frame.len() > 1 {
                                        frame[1] = out_r;
                                    }
                                    for extra in frame.iter_mut().skip(2) {
                                        *extra = 0.0;
                                    }
                                } else {
                                    frame.fill(0.0);
                                }
                            }
                        } else {
                            data.fill(0.0);
                        }
                    } else {
                        // Consumer is empty: do not overwrite pipeline ai_analyzer with zeros!
                        data.fill(0.0);
                    }
                }
            },
            err_fn_out,
            None,
        )?;

        output_stream.play()?;

        Ok(Self {
            _host: host,
            input_device_name,
            output_device_name,
            sample_rate,
            is_running,
            synth_enabled,
            _input_stream: _input_stream_opt,
            _output_stream: Some(output_stream),
        })
    }
}
