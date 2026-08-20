//! Offline File Audio Processing Engine (WAV batch processing / remastering)

use crate::dsp::pipeline::AudioPipeline;
use crate::presets::default_presets::Preset;
use anyhow::{Context, Result};
use hound::{WavReader, WavSpec, WavWriter};
use std::path::Path;
use std::time::Instant;

pub struct FileProcessor;

impl FileProcessor {
    pub fn process_file(
        input_path: &Path,
        output_path: &Path,
        preset: &Preset,
    ) -> Result<ProcessingReport> {
        let start_time = Instant::now();
        let mut reader = WavReader::open(input_path)
            .with_context(|| format!("Failed to open WAV input file: {:?}", input_path))?;

        let spec = reader.spec();
        let sample_rate = spec.sample_rate as f32;
        let channels = spec.channels as usize;

        if channels != 1 && channels != 2 {
            anyhow::bail!("Only Mono (1) or Stereo (2) WAV files are supported. Found {} channels.", channels);
        }

        // Initialize Audio Pipeline
        let mut pipeline = AudioPipeline::new(sample_rate);
        pipeline.apply_config(&preset.to_pipeline_config());

        // Configure output spec (16-bit high compatibility PCM stereo)
        let out_spec = WavSpec {
            channels: 2, // Always output stereo (spatialized)
            sample_rate: spec.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = WavWriter::create(output_path, out_spec)
            .with_context(|| format!("Failed to create WAV output file: {:?}", output_path))?;

        let mut max_input_peak: f32 = 0.0;
        let mut max_output_peak: f32 = 0.0;
        let mut sample_count: u64 = 0;

        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap_or(0.0)).collect(),
            hound::SampleFormat::Int => {
                let max_val = (1i64 << (spec.bits_per_sample - 1)) as f32;
                reader.samples::<i32>().map(|s| s.unwrap_or(0) as f32 / max_val).collect()
            }
        };

        if channels == 1 {
            for &s in &samples {
                let abs_in = s.abs();
                if abs_in > max_input_peak { max_input_peak = abs_in; }

                let (out_l, out_r) = pipeline.process_stereo_sample(s, s);
                let abs_out = out_l.abs().max(out_r.abs());
                if abs_out > max_output_peak { max_output_peak = abs_out; }

                let i16_l = (out_l.clamp(-1.0, 1.0) * 32767.0).round() as i16;
                let i16_r = (out_r.clamp(-1.0, 1.0) * 32767.0).round() as i16;
                writer.write_sample(i16_l)?;
                writer.write_sample(i16_r)?;
                sample_count += 1;
            }
        } else {
            for chunk in samples.chunks_exact(2) {
                let in_l = chunk[0];
                let in_r = chunk[1];
                let abs_in = in_l.abs().max(in_r.abs());
                if abs_in > max_input_peak { max_input_peak = abs_in; }

                let (out_l, out_r) = pipeline.process_stereo_sample(in_l, in_r);
                let abs_out = out_l.abs().max(out_r.abs());
                if abs_out > max_output_peak { max_output_peak = abs_out; }

                let i16_l = (out_l.clamp(-1.0, 1.0) * 32767.0).round() as i16;
                let i16_r = (out_r.clamp(-1.0, 1.0) * 32767.0).round() as i16;
                writer.write_sample(i16_l)?;
                writer.write_sample(i16_r)?;
                sample_count += 1;
            }
        }

        writer.finalize()?;

        let elapsed = start_time.elapsed();
        let total_audio_duration_sec = sample_count as f64 / sample_rate as f64;
        let speedup = if elapsed.as_secs_f64() > 0.0 {
            total_audio_duration_sec / elapsed.as_secs_f64()
        } else {
            1.0
        };

        Ok(ProcessingReport {
            input_peak_db: 20.0 * (max_input_peak + 1e-6).log10(),
            output_peak_db: 20.0 * (max_output_peak + 1e-6).log10(),
            duration_seconds: total_audio_duration_sec,
            processing_time_ms: elapsed.as_millis(),
            speedup_factor: speedup,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ProcessingReport {
    pub input_peak_db: f32,
    pub output_peak_db: f32,
    pub duration_seconds: f64,
    pub processing_time_ms: u128,
    pub speedup_factor: f64,
}
