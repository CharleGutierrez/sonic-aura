//! Real-Time Low-Latency CPAL Audio Playback & Synthesis Engine
//! Strictly isolates output playback from microphone capture to eliminate 100% of background hiss,
//! fan noise, electrical hum, and acoustic feedback.

use crate::audio::test_synth::{SynthTone, TestSynth};
use crate::dsp::pipeline::SharedPipeline;
use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Host, Stream, StreamConfig};
use ringbuf::traits::{Consumer, Observer};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use rubato::{Resampler, SincFixedIn, SincInterpolationType, SincInterpolationParameters, WindowFunction};

#[cfg(target_os = "linux")]
fn silence_alsa_logging() {
    unsafe {
        extern "C" fn dummy_handler(
            _file: *const std::ffi::c_char,
            _line: std::ffi::c_int,
            _function: *const std::ffi::c_char,
            _err: std::ffi::c_int,
            _fmt: *const std::ffi::c_char,
        ) {
        }

        let handle = libc::dlopen(b"libasound.so.2\0".as_ptr() as *const _, libc::RTLD_LAZY);
        if !handle.is_null() {
            let symbol = libc::dlsym(handle, b"snd_lib_error_set_handler\0".as_ptr() as *const _);
            if !symbol.is_null() {
                let set_handler: extern "C" fn(*const ()) -> std::ffi::c_int =
                    std::mem::transmute(symbol);
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
        mut consumer: ringbuf::HeapCons<f32>,
    ) -> Result<Self> {
        silence_alsa_logging();

        let host = cpal::default_host();

        // 1. Select Output Device (Speakers / Headphones)
        let output_device = if let Some(ref name) = preferred_output {
            host.output_devices()?
                .find(|d| d.name().map(|n| n == *name).unwrap_or(false))
                .or_else(|| host.default_output_device())
        } else {
            host.default_output_device()
        }
        .context("No audio output device found on system")?;

        let output_device_name = output_device
            .name()
            .unwrap_or_else(|_| "Default Output".to_string());

        let default_out_config = output_device.default_output_config()?;
        let sample_rate = if let Ok(configs) = output_device.supported_output_configs() {
            if configs.into_iter().any(|c| c.min_sample_rate() <= 48000 && c.max_sample_rate() >= 48000) {
                48000
            } else {
                default_out_config.sample_rate()
            }
        } else {
            default_out_config.sample_rate()
        };
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

        let mut synth = TestSynth::new(sample_rate as f32);
        synth.set_tone_type(synth_tone);

        let mut _input_stream_opt: Option<Stream> = None;
        let mut input_device_name = "System Output Monitor (pw-record)".to_string();

        if let Some(ref name) = preferred_input {
            input_device_name = name.clone();
        }

        let pl_clone = Arc::clone(&pipeline);
        let synth_flag = Arc::clone(&synth_enabled);

        let err_fn_out = |_| {};
        
        let mut resampler = SincFixedIn::<f32>::new(
            sample_rate as f64 / 48000.0,
            2.0,
            SincInterpolationParameters {
                sinc_len: 128,
                f_cutoff: 0.95,
                interpolation: SincInterpolationType::Linear,
                oversampling_factor: 128,
                window: WindowFunction::BlackmanHarris2,
            },
            256,
            2,
        ).unwrap();
        let mut resampler_in = vec![vec![0.0; 256]; 2];
        let mut resampler_out = resampler.output_buffer_allocate(true);
        let mut resampler_out_idx = 0;
        let mut resampler_out_len = 0;
        let mut resampler_filled = 0;

        let output_stream = output_device.build_output_stream(
            &out_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let is_synth = synth_flag.load(Ordering::Relaxed);

                if is_synth {
                    // Play synthesized spatial test audio with zero extraneous noise
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
                    let is_48k = sample_rate == 48000;
                    if let Ok(mut pl) = pl_clone.lock() {
                        for frame in data.chunks_exact_mut(out_channels) {
                            let (in_l, in_r) = if is_48k {
                                if let (Some(l), Some(r)) = (consumer.try_pop(), consumer.try_pop()) {
                                    (l, r)
                                } else {
                                    (0.0, 0.0)
                                }
                            } else {
                                // Resample if hardware requires non-48kHz
                                if resampler_out_idx >= resampler_out_len {
                                    while resampler_filled < 256 && consumer.occupied_len() >= 2 {
                                        if let (Some(l), Some(r)) = (consumer.try_pop(), consumer.try_pop()) {
                                            resampler_in[0][resampler_filled] = l;
                                            resampler_in[1][resampler_filled] = r;
                                            resampler_filled += 1;
                                        } else {
                                            break;
                                        }
                                    }
                                    
                                    if resampler_filled == 256 {
                                        if let Ok((_in_len, out_len)) = resampler.process_into_buffer(&resampler_in, &mut resampler_out, None) {
                                            resampler_out_len = out_len;
                                            resampler_out_idx = 0;
                                        }
                                        resampler_filled = 0;
                                    }
                                }

                                if resampler_out_idx < resampler_out_len {
                                    let l = resampler_out[0][resampler_out_idx];
                                    let r = resampler_out[1][resampler_out_idx];
                                    resampler_out_idx += 1;
                                    (l, r)
                                } else {
                                    (0.0, 0.0)
                                }
                            };

                            if in_l != 0.0 || in_r != 0.0 {
                                let (out_l, out_r) = pl.process_stereo_sample(in_l, in_r);
                                if out_channels == 1 {
                                    frame[0] = (out_l + out_r) * 0.5;
                                } else {
                                    frame[0] = out_l;
                                    if frame.len() > 1 {
                                        frame[1] = out_r;
                                    }
                                    for extra in frame.iter_mut().skip(2) {
                                        *extra = 0.0;
                                    }
                                }
                            } else {
                                frame.fill(0.0);
                            }
                        }
                    } else {
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
