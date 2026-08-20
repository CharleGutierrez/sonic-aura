//! Real-Time Low-Latency CPAL Audio I/O Engine

use crate::audio::test_synth::{SynthTone, TestSynth};
use crate::dsp::pipeline::SharedPipeline;
use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Host, Stream, StreamConfig};
use ringbuf::traits::{Consumer, Producer, Split};
use ringbuf::HeapRb;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub struct AudioEngine {
    _host: Host,
    pub input_device_name: String,
    pub output_device_name: String,
    pub sample_rate: u32,
    pub is_running: Arc<AtomicBool>,
    _input_stream: Option<Stream>,
    _output_stream: Option<Stream>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EngineMode {
    LoopbackLive, // Process live input/sink monitor -> DSP -> output
    TestSynth,    // Internal test synth -> DSP -> output
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
        let host = cpal::default_host();

        // 1. Select Output Device
        let output_device = if let Some(ref name) = preferred_output {
            host.output_devices()?
                .find(|d| d.name().map(|n| n == *name).unwrap_or(false))
                .or_else(|| host.default_output_device())
        } else {
            host.default_output_device()
        }
        .context("No audio output device found on system")?;

        let output_device_name = output_device.name().unwrap_or_else(|_| "Default Output".to_string());

        // Select Output Config
        let default_out_config = output_device.default_output_config()?;
        let sample_rate = default_out_config.sample_rate();
        let out_channels = default_out_config.channels() as usize;

        // Update pipeline sample rate
        {
            let mut pl = pipeline.lock().unwrap();
            pl.set_sample_rate(sample_rate as f32);
        }

        let is_running = Arc::new(AtomicBool::new(true));

        let out_config = StreamConfig {
            channels: default_out_config.channels(),
            sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let mut _input_stream_opt: Option<Stream> = None;
        let input_device_name: String;

        match mode {
            EngineMode::TestSynth => {
                input_device_name = "Internal Test Synth (Binaural Demo)".to_string();
                let mut synth = TestSynth::new(sample_rate as f32);
                synth.set_tone_type(synth_tone);

                let pl_clone = Arc::clone(&pipeline);
                let err_fn = |err| eprintln!("Output audio stream error: {}", err);

                let output_stream = output_device.build_output_stream(
                    &out_config,
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        if let Ok(mut pl) = pl_clone.try_lock() {
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
                    },
                    err_fn,
                    None,
                )?;

                output_stream.play()?;

                Ok(Self {
                    _host: host,
                    input_device_name,
                    output_device_name,
                    sample_rate,
                    is_running,
                    _input_stream: None,
                    _output_stream: Some(output_stream),
                })
            }
            EngineMode::LoopbackLive => {
                // Select Input Device
                let input_device = if let Some(ref name) = preferred_input {
                    host.input_devices()?
                        .find(|d| d.name().map(|n| n == *name).unwrap_or(false))
                        .or_else(|| host.default_input_device())
                } else {
                    host.default_input_device()
                }
                .context("No audio input device found on system")?;

                input_device_name = input_device.name().unwrap_or_else(|_| "Default Input".to_string());

                let default_in_config = input_device.default_input_config()?;
                let in_channels = default_in_config.channels() as usize;

                let in_config = StreamConfig {
                    channels: default_in_config.channels(),
                    sample_rate,
                    buffer_size: cpal::BufferSize::Default,
                };

                // Ring buffer for bridging input stream to output stream (stereo samples)
                let ring_buffer = HeapRb::<f32>::new(8192);
                let (mut producer, mut consumer) = ring_buffer.split();

                let err_fn_in = |err| eprintln!("Input audio stream error: {}", err);
                let err_fn_out = |err| eprintln!("Output audio stream error: {}", err);

                // Input capture callback
                let input_stream = input_device.build_input_stream(
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
                )?;

                // Output playback callback with DSP processing
                let pl_clone = Arc::clone(&pipeline);
                let output_stream = output_device.build_output_stream(
                    &out_config,
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        if let Ok(mut pl) = pl_clone.try_lock() {
                            for frame in data.chunks_exact_mut(out_channels) {
                                let in_l = consumer.try_pop().unwrap_or(0.0);
                                let in_r = consumer.try_pop().unwrap_or(0.0);
                                let (out_l, out_r) = pl.process_stereo_sample(in_l, in_r);
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
                    },
                    err_fn_out,
                    None,
                )?;

                input_stream.play()?;
                output_stream.play()?;

                _input_stream_opt = Some(input_stream);

                Ok(Self {
                    _host: host,
                    input_device_name,
                    output_device_name,
                    sample_rate,
                    is_running,
                    _input_stream: _input_stream_opt,
                    _output_stream: Some(output_stream),
                })
            }
        }
    }
}
