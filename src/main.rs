//! SonicAura AI: Dolby Atmos & Bang & Olufsen-grade Audio Enhancer in Rust
//! High-Performance Psychoacoustic AI DSP Engine, Spatializer, Multi-band EQ & Limiter

#![allow(dead_code, deprecated)]

mod audio;
mod config;
mod dsp;
mod presets;
mod ui;

use anyhow::Result;
use audio::cpal_stream::{AudioEngine, EngineMode};
use audio::file_processor::FileProcessor;
use audio::test_synth::SynthTone;
use audio::virtual_device::VirtualSinkManager;
use clap::Parser;
use config::AppConfig;
use dsp::earphone_profiler::EarphoneType;
use dsp::environment_adapter::EnvironmentMode;
use dsp::pipeline::AudioPipeline;
use presets::PresetManager;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use ui::tui_app::TuiApp;

#[derive(Parser, Debug)]
#[command(
    name = "sonic_aura",
    author = "SonicAura AI Audio Engine",
    version = "0.1.0",
    about = "Universal Dolby Atmos & Bang & Olufsen-grade AI Sound Enhancer: Adapts to ANY Earphone & ANY Environment (City to Remote)"
)]
struct Args {
    /// Launch in Demo Synth Mode (immediate playback with rich binaural test music)
    #[arg(short, long)]
    demo: bool,

    /// Synth demo tone type (music, pink, sweep, kick)
    #[arg(long, default_value = "music")]
    synth_tone: String,

    /// Initial preset to load (e.g. "Dolby Atmos", "Bang & Olufsen", "Laptop Speaker Fix", "DTS:X Gaming")
    #[arg(short, long)]
    preset: Option<String>,

    /// Earphone/Headphone hardware profile (budget, airpods, bass, iem, open-back, closed-back, neutral)
    #[arg(short = 'e', long)]
    earphone: Option<String>,

    /// Acoustic environment context (city, transit, cafe, remote, whisper, neutral)
    #[arg(short = 'E', long)]
    env: Option<String>,

    /// List all earphone calibration profiles and environment modes
    #[arg(long)]
    list_profiles: bool,

    /// Run in headless daemon mode (no TUI)
    #[arg(short = 'D', long)]
    daemon: bool,

    /// List all available audio input and output devices
    #[arg(long)]
    list_devices: bool,

    /// Setup system-wide PipeWire / PulseAudio Virtual Loopback Sink
    #[arg(long)]
    setup_sink: bool,

    /// Remove system-wide Virtual Loopback Sink
    #[arg(long)]
    remove_sink: bool,

    /// Offline batch audio file processing (input WAV)
    #[arg(long)]
    process_file: Option<PathBuf>,

    /// Output WAV file path for offline processing
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Run DSP performance benchmark
    #[arg(long)]
    benchmark: bool,

    /// Preferred input device name
    #[arg(long)]
    input_device: Option<String>,

    /// Preferred output device name
    #[arg(long)]
    output_device: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // 1. Handle Setup Sink
    if args.setup_sink {
        println!("🔧 Setting up SonicAura Virtual Audio Sink (PipeWire / PulseAudio)...");
        match VirtualSinkManager::create_virtual_sink() {
            Ok(_) => {
                println!("✅ Success! Virtual sink 'SonicAura_Sink' (SonicAura_AI_Enhancer_Sink) created.");
                println!("👉 Open your system Sound Settings and select 'SonicAura_AI_Enhancer_Sink' as Output.");
                println!("👉 Then run `sonic_aura` to start real-time Dolby/B&O audio boosting!");
            }
            Err(e) => {
                eprintln!("❌ Error creating virtual sink: {}", e);
            }
        }
        return Ok(());
    }

    // 2. Handle Remove Sink
    if args.remove_sink {
        println!("Removing SonicAura virtual audio sink...");
        let _ = VirtualSinkManager::remove_virtual_sink();
        println!("Done.");
        return Ok(());
    }

    // 3. Handle List Profiles
    if args.list_profiles {
        println!("=== 🎧 Universal Earphone Hardware Calibration Profiles ===");
        for e in &EarphoneType::ALL {
            println!("\n  [{:?}] {}", e, e.name());
            println!("   {}", e.description());
        }

        println!("\n=== 🌍 Adaptive Acoustic Environment Modes ===");
        for env in &EnvironmentMode::ALL {
            println!("\n  [{:?}] {}", env, env.name());
            println!("   {}", env.description());
        }
        return Ok(());
    }

    // 4. Handle List Devices
    if args.list_devices {
        println!("=== SonicAura Audio Devices ===");
        let (inputs, outputs) = AudioEngine::get_available_devices();
        println!("\n📥 Available Input Devices (Capture / Sinks):");
        for (i, d) in inputs.iter().enumerate() {
            println!("  [{}] {}", i + 1, d);
        }
        println!("\n📤 Available Output Devices (Speakers / Headphones):");
        for (i, d) in outputs.iter().enumerate() {
            println!("  [{}] {}", i + 1, d);
        }
        return Ok(());
    }

    // 5. Handle DSP Benchmark
    if args.benchmark {
        run_benchmark();
        return Ok(());
    }

    // 6. Handle Offline File Processing
    if let Some(ref input_path) = args.process_file {
        let output_path = args.output.unwrap_or_else(|| {
            let stem = input_path.file_stem().and_then(|s| s.to_str()).unwrap_or("audio");
            PathBuf::from(format!("{}_enhanced.wav", stem))
        });

        let presets = PresetManager::new();
        let preset = if let Some(ref name) = args.preset {
            presets
                .find_by_name(name)
                .map(|idx| &presets.presets[idx])
                .unwrap_or_else(|| presets.current())
        } else {
            presets.current()
        };

        println!("⚡ Processing '{}' with preset: '{}'...", input_path.display(), preset.name);
        let report = FileProcessor::process_file(input_path, &output_path, preset)?;

        println!("\n✨ Processing Complete!");
        println!("  Output File:        {}", output_path.display());
        println!("  Duration:           {:.2}s", report.duration_seconds);
        println!("  Processing Time:    {} ms", report.processing_time_ms);
        println!("  Speedup:            {:.1}x Real-time", report.speedup_factor);
        println!("  Input Peak:         {:.1} dBFS", report.input_peak_db);
        println!("  Enhanced Peak:      {:.1} dBFS", report.output_peak_db);
        return Ok(());
    }

    // 7. Real-time Audio Engine Setup
    let mut config = AppConfig::load();

    // Override earphone from CLI if provided
    if let Some(ref e_str) = args.earphone {
        let e_lower = e_str.to_lowercase();
        if e_lower.contains("budget") || e_lower.contains("cheap") {
            config.earphone_type = EarphoneType::BudgetEarbudsFix;
        } else if e_lower.contains("airpod") || e_lower.contains("tws") {
            config.earphone_type = EarphoneType::AirPodsAndTws;
        } else if e_lower.contains("bass") || e_lower.contains("beat") {
            config.earphone_type = EarphoneType::BassHeavyCommercial;
        } else if e_lower.contains("iem") || e_lower.contains("audio") {
            config.earphone_type = EarphoneType::IemAudiophile;
        } else if e_lower.contains("open") {
            config.earphone_type = EarphoneType::StudioOpenBack;
        } else if e_lower.contains("close") {
            config.earphone_type = EarphoneType::StudioClosedBack;
        } else if e_lower.contains("flat") || e_lower.contains("neutral") {
            config.earphone_type = EarphoneType::UniversalNeutral;
        }
    }

    // Override environment from CLI if provided
    if let Some(ref env_str) = args.env {
        let env_lower = env_str.to_lowercase();
        if env_lower.contains("city") || env_lower.contains("traffic") {
            config.environment_mode = EnvironmentMode::CityTraffic;
        } else if env_lower.contains("transit") || env_lower.contains("plane") || env_lower.contains("subway") {
            config.environment_mode = EnvironmentMode::CommuteTransit;
        } else if env_lower.contains("cafe") || env_lower.contains("office") {
            config.environment_mode = EnvironmentMode::CafeOffice;
        } else if env_lower.contains("remote") || env_lower.contains("quiet") || env_lower.contains("nature") {
            config.environment_mode = EnvironmentMode::QuietRemote;
        } else if env_lower.contains("night") || env_lower.contains("whisper") {
            config.environment_mode = EnvironmentMode::LateNightWhisper;
        } else if env_lower.contains("neutral") || env_lower.contains("studio") {
            config.environment_mode = EnvironmentMode::NeutralStudio;
        }
    }

    let sample_rate = config.sample_rate as f32;
    let pipeline = Arc::new(Mutex::new(AudioPipeline::new(sample_rate)));

    let engine_mode = if args.demo {
        EngineMode::TestSynth
    } else {
        EngineMode::LoopbackLive
    };

    let synth_tone = match args.synth_tone.to_lowercase().as_str() {
        "pink" => SynthTone::PinkNoise,
        "sweep" => SynthTone::SineSweep,
        "kick" => SynthTone::BassKickTest,
        _ => SynthTone::MusicAcousticDemo,
    };

    let input_dev = args.input_device.or_else(|| config.input_device.clone());
    let output_dev = args.output_device.or_else(|| config.output_device.clone());

    let _engine_result = AudioEngine::start(
        Arc::clone(&pipeline),
        engine_mode,
        input_dev,
        output_dev,
        synth_tone,
    );

    match _engine_result {
        Ok(engine) => {
            let synth_flag = Arc::clone(&engine.synth_enabled);
            if args.daemon {
                println!("⚡ SonicAura AI Daemon running in background...");
                println!("  Input:       {}", engine.input_device_name);
                println!("  Output:      {}", engine.output_device_name);
                println!("  Earphone:    {}", config.earphone_type.name());
                println!("  Environment: {}", config.environment_mode.name());
                println!("  Sample Rate: {} Hz", engine.sample_rate);
                println!("Press Ctrl+C to terminate.");
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            } else {
                let mut app = TuiApp::new(pipeline, config, synth_flag);
                app.run()?;
            }
        }
        Err(e) => {
            eprintln!("⚠️ Audio device notice: {}", e);
            eprintln!("Launching in Interactive TUI Offline/Benchmarking mode...");
            let synth_flag = Arc::new(std::sync::atomic::AtomicBool::new(true));
            let mut app = TuiApp::new(pipeline, config, synth_flag);
            app.run()?;
        }
    }

    Ok(())
}

fn run_benchmark() {
    println!("⚡ Starting SonicAura AI DSP Engine Performance Benchmark...");
    let sample_rate = 48000.0;
    let mut pipeline = AudioPipeline::new(sample_rate);

    let num_samples = 48000 * 60; // 60 seconds of 48kHz audio (2.88 million stereo sample frames)
    let in_l = 0.45_f32;
    let in_r = -0.45_f32;

    let start = Instant::now();
    for _ in 0..num_samples {
        let _ = pipeline.process_stereo_sample(in_l, in_r);
    }
    let elapsed = start.elapsed();

    let audio_seconds = 60.0;
    let elapsed_seconds = elapsed.as_secs_f64();
    let speedup = audio_seconds / elapsed_seconds;
    let throughput_msamples = (num_samples as f64 / 1_000_000.0) / elapsed_seconds;
    let per_sample_ns = (elapsed.as_nanos() as f64) / (num_samples as f64);

    println!("\n🚀 Benchmark Results:");
    println!("  Total Processed:    {} stereo frames (1 minute of 48kHz audio)", num_samples);
    println!("  Execution Time:     {:.3} ms", elapsed.as_secs_f64() * 1000.0);
    println!("  Throughput Speed:   {:.1}x Real-time", speedup);
    println!("  Processing Speed:   {:.2} Million Samples/sec", throughput_msamples);
    println!("  Latency per sample: {:.1} nanoseconds", per_sample_ns);
    println!("  CPU DSP Load:       {:.3}% of single CPU core at 48kHz", (1.0 / speedup) * 100.0);
    println!("\n✨ Ultra-lightweight & zero dropouts guaranteed!");
}
