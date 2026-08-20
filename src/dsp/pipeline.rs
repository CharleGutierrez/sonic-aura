//! Master Audio Processing Engine Pipeline
//! Unifies EQ, AI Analysis, Earphone Calibration, Environmental Adaptation,
//! Psychoacoustic Bass, Harmonic Excitation, 3D Spatializer, Multiband Compressor, and True-Peak Limiter.
//! Features Time-Aligned De-Clicking Crossfade and 18Hz Infrasonic DC/Rumble Blocking.

use crate::dsp::ai_analyzer::AiSpectralAnalyzer;
use crate::dsp::biquad::{Biquad, FilterType};
use crate::dsp::compressor::MultibandCompressor;
use crate::dsp::earphone_profiler::EarphoneType;
use crate::dsp::environment_adapter::EnvironmentMode;
use crate::dsp::equalizer::Equalizer;
use crate::dsp::exciter::HarmonicExciter;
use crate::dsp::limiter::Limiter;
use crate::dsp::psychoacoustic_bass::PsychoacousticBass;
use crate::dsp::spatializer::{SpatialMode, Spatializer};
use crate::dsp::transient_shaper::TransientShaper;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub enabled: bool,
    pub master_gain_db: f32,
    pub ai_boost_enabled: bool,
    pub ai_intensity: f32,
    pub bass_boost_intensity: f32,
    pub bass_speaker_protect: bool,
    pub exciter_air_mix: f32,
    pub exciter_drive: f32,
    pub spatial_width: f32,
    pub spatial_depth: f32,
    pub spatial_crossfeed: f32,
    pub spatial_mode: SpatialMode,
    pub earphone_type: EarphoneType,
    pub environment_mode: EnvironmentMode,
    pub transient_attack: f32,
    pub compressor_intensity: f32,
    pub dynamic_loudness: f32,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            master_gain_db: 0.0,
            ai_boost_enabled: true,
            ai_intensity: 0.85,
            bass_boost_intensity: 0.8,
            bass_speaker_protect: false,
            exciter_air_mix: 0.55,
            exciter_drive: 0.6,
            spatial_width: 1.35,
            spatial_depth: 0.35,
            spatial_crossfeed: 0.45,
            spatial_mode: SpatialMode::HeadphonesBinaural,
            earphone_type: EarphoneType::AirPodsAndTws,
            environment_mode: EnvironmentMode::CityTraffic,
            transient_attack: 0.45,
            compressor_intensity: 0.65,
            dynamic_loudness: 0.5,
        }
    }
}

pub struct AudioPipeline {
    sample_rate: f32,
    pub config: PipelineConfig,
    pub eq: Equalizer,
    pub psycho_bass: PsychoacousticBass,
    pub transient_shaper: TransientShaper,
    pub exciter: HarmonicExciter,
    pub spatializer: Spatializer,
    pub compressor: MultibandCompressor,
    pub limiter: Limiter,
    pub ai_analyzer: AiSpectralAnalyzer,
    pub user_eq_gains: [f32; 10],

    // Infrasonic DC / Sub-sonic Rumble Filter (18Hz HPF) to eliminate speaker flutter/thump
    dc_blocker_l: Biquad,
    dc_blocker_r: Biquad,

    // Time-aligned delay buffer for clickless bypass crossfading (matches limiter lookahead)
    dry_delay_l: Vec<f32>,
    dry_delay_r: Vec<f32>,
    dry_delay_idx: usize,
    dry_delay_len: usize,

    // Smooth de-clicking crossfade gain for [Space] bypass toggle (0.0 = bypass, 1.0 = active)
    current_fade: f32,
    fade_step: f32,
}

impl AudioPipeline {
    pub fn new(sample_rate: f32) -> Self {
        let config = PipelineConfig::default();
        let dc_blocker_l = Biquad::new(FilterType::HighPass, 18.0, 0.707, 0.0, sample_rate);
        let dc_blocker_r = Biquad::new(FilterType::HighPass, 18.0, 0.707, 0.0, sample_rate);

        // 1.5ms lookahead match
        let lookahead_samples = ((1.5 * 0.001 * sample_rate) as usize).max(4);
        let fade_samples = (0.015 * sample_rate).max(64.0); // 15ms smooth crossfade
        let fade_step = 1.0 / fade_samples;

        let mut pipeline = Self {
            sample_rate,
            config: config.clone(),
            eq: Equalizer::new_10_band(sample_rate),
            psycho_bass: PsychoacousticBass::new(sample_rate),
            transient_shaper: TransientShaper::new(sample_rate),
            exciter: HarmonicExciter::new(sample_rate),
            spatializer: Spatializer::new(sample_rate),
            compressor: MultibandCompressor::new(sample_rate),
            limiter: Limiter::new(sample_rate, 1.5, -0.1, 50.0),
            ai_analyzer: AiSpectralAnalyzer::new(sample_rate),
            user_eq_gains: [0.0; 10],
            dc_blocker_l,
            dc_blocker_r,
            dry_delay_l: vec![0.0; lookahead_samples],
            dry_delay_r: vec![0.0; lookahead_samples],
            dry_delay_idx: 0,
            dry_delay_len: lookahead_samples,
            current_fade: 1.0,
            fade_step,
        };
        pipeline.apply_config(&config);
        pipeline
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.eq.set_sample_rate(sample_rate);
        self.psycho_bass.set_sample_rate(sample_rate);
        self.transient_shaper.set_sample_rate(sample_rate);
        self.exciter.set_sample_rate(sample_rate);
        self.spatializer.set_sample_rate(sample_rate);
        self.compressor.set_sample_rate(sample_rate);
        self.limiter.set_sample_rate(sample_rate);
        self.ai_analyzer.set_sample_rate(sample_rate);
        self.dc_blocker_l = Biquad::new(FilterType::HighPass, 18.0, 0.707, 0.0, sample_rate);
        self.dc_blocker_r = Biquad::new(FilterType::HighPass, 18.0, 0.707, 0.0, sample_rate);

        let lookahead_samples = ((1.5 * 0.001 * sample_rate) as usize).max(4);
        self.dry_delay_len = lookahead_samples;
        self.dry_delay_l.resize(lookahead_samples, 0.0);
        self.dry_delay_r.resize(lookahead_samples, 0.0);
        self.dry_delay_idx = 0;

        let fade_samples = (0.015 * sample_rate).max(64.0);
        self.fade_step = 1.0 / fade_samples;
    }

    pub fn set_user_eq_gain(&mut self, band_idx: usize, gain_db: f32) {
        if band_idx < 10 {
            self.user_eq_gains[band_idx] = gain_db;
            self.update_combined_eq();
        }
    }

    pub fn set_all_user_eq_gains(&mut self, gains: &[f32; 10]) {
        self.user_eq_gains = *gains;
        self.update_combined_eq();
    }

    pub fn set_earphone_type(&mut self, earphone: EarphoneType) {
        self.config.earphone_type = earphone;
        self.update_combined_eq();
        self.apply_config(&self.config.clone());
    }

    pub fn set_environment_mode(&mut self, env: EnvironmentMode) {
        self.config.environment_mode = env;
        self.update_combined_eq();
        self.apply_config(&self.config.clone());
    }

    fn update_combined_eq(&mut self) {
        let earphone_offsets = self.config.earphone_type.eq_offsets();
        let env_offsets = self.config.environment_mode.eq_offsets();

        for i in 0..10 {
            let combined = self.user_eq_gains[i] + earphone_offsets[i] * 0.75 + env_offsets[i] * 0.75;
            self.eq.set_band_gain(i, combined.clamp(-24.0, 24.0));
        }
    }

    pub fn apply_config(&mut self, config: &PipelineConfig) {
        self.config = config.clone();

        let (e_bass, e_air, e_width, e_cross, e_trans) = config.earphone_type.dsp_modifiers();
        let (env_bass, env_air, env_width, env_comp, env_loud) = config.environment_mode.dsp_modifiers();

        let effective_bass = config.bass_boost_intensity * e_bass * env_bass;
        let effective_air = config.exciter_air_mix * e_air * env_air;
        let effective_width = config.spatial_width * e_width * env_width;
        let effective_cross = config.spatial_crossfeed * e_cross;
        let effective_trans = config.transient_attack * e_trans;
        let effective_comp = (config.compressor_intensity * env_comp).min(1.0);
        let effective_loud = config.dynamic_loudness * env_loud;

        self.psycho_bass.set_intensity(effective_bass);
        self.psycho_bass.set_speaker_protection(config.bass_speaker_protect);
        self.exciter.set_air_mix(effective_air);
        self.exciter.set_drive(config.exciter_drive);
        self.spatializer.set_mode(config.spatial_mode);
        self.spatializer.set_width(effective_width);
        self.spatializer.set_depth(config.spatial_depth);
        self.spatializer.set_crossfeed(effective_cross);
        self.transient_shaper.set_attack(effective_trans);
        self.compressor.set_intensity(effective_comp);
        self.compressor.set_dynamic_loudness(effective_loud);
        self.ai_analyzer.set_ai_enhancement_amount(if config.ai_boost_enabled { config.ai_intensity } else { 0.0 });
        self.update_combined_eq();
    }

    #[inline(always)]
    pub fn process_stereo_sample(&mut self, raw_in_l: f32, raw_in_r: f32) -> (f32, f32) {
        // Strip inaudible sub-sonic DC offset / flutter rumble
        let in_l = self.dc_blocker_l.process(raw_in_l);
        let in_r = self.dc_blocker_r.process(raw_in_r);

        // Maintain time-aligned dry delay line to match limiter lookahead
        let dry_l = self.dry_delay_l[self.dry_delay_idx];
        let dry_r = self.dry_delay_r[self.dry_delay_idx];
        self.dry_delay_l[self.dry_delay_idx] = in_l;
        self.dry_delay_r[self.dry_delay_idx] = in_r;
        self.dry_delay_idx = (self.dry_delay_idx + 1) % self.dry_delay_len;

        // Smooth de-clicking crossfade ramp for [Space] bypass toggle
        let target_fade = if self.config.enabled { 1.0 } else { 0.0 };
        if self.current_fade < target_fade {
            self.current_fade = (self.current_fade + self.fade_step).min(1.0);
        } else if self.current_fade > target_fade {
            self.current_fade = (self.current_fade - self.fade_step).max(0.0);
        }

        let fade = self.current_fade;

        // Apply Master Input / Output Preamp
        let master_gain = 10.0_f32.powf(self.config.master_gain_db / 20.0);
        let s_l = in_l * master_gain;
        let s_r = in_r * master_gain;

        // 1. Multi-band Equalizer (User + Earphone Calibration + Environmental Anti-Masking)
        let (eq_l, eq_r) = self.eq.process(s_l, s_r);

        // 2. Psychoacoustic Sub-Bass Enhancement (Missing Fundamental)
        let (e_bass, _, _, _, _) = self.config.earphone_type.dsp_modifiers();
        let (env_bass, _, _, _, _) = self.config.environment_mode.dsp_modifiers();
        let effective_bass_intensity = if self.config.ai_boost_enabled {
            self.config.bass_boost_intensity * e_bass * env_bass * self.ai_analyzer.adaptive_params.dynamic_bass_intensity_mod
        } else {
            self.config.bass_boost_intensity * e_bass * env_bass
        };
        self.psycho_bass.set_intensity(effective_bass_intensity);
        let (bass_l, bass_r) = self.psycho_bass.process(eq_l, eq_r);

        // 3. Transient Shaper (Punch & Attack)
        let (trans_l, trans_r) = self.transient_shaper.process(bass_l, bass_r);

        // 4. Harmonic Exciter & High-Frequency Air (B&O Sheen & Earphone Air Injection)
        let (_, e_air, _, _, _) = self.config.earphone_type.dsp_modifiers();
        let (_, env_air, _, _, _) = self.config.environment_mode.dsp_modifiers();
        let effective_air = if self.config.ai_boost_enabled {
            self.config.exciter_air_mix * e_air * env_air * self.ai_analyzer.adaptive_params.dynamic_exciter_air_mod
        } else {
            self.config.exciter_air_mix * e_air * env_air
        };
        self.exciter.set_air_mix(effective_air);
        let (exc_l, exc_r) = self.exciter.process(trans_l, trans_r);

        // 5. 3D Binaural Spatializer & Soundstage Widener
        let (_, _, e_width, _, _) = self.config.earphone_type.dsp_modifiers();
        let (_, _, env_width, _, _) = self.config.environment_mode.dsp_modifiers();
        let effective_width = if self.config.ai_boost_enabled {
            self.config.spatial_width * e_width * env_width * self.ai_analyzer.adaptive_params.dynamic_spatial_width_mod
        } else {
            self.config.spatial_width * e_width * env_width
        };
        self.spatializer.set_width(effective_width);
        let (spat_l, spat_r) = self.spatializer.process(exc_l, exc_r);

        // 6. Multiband Compressor & Fletcher-Munson Loudness
        let (comp_l, comp_r) = self.compressor.process(spat_l, spat_r);

        // 7. True-Peak Lookahead Brickwall Limiter & Soft Clipper
        let (dsp_out_l, dsp_out_r) = self.limiter.process(comp_l, comp_r);

        // 8. Equal-Power Time-Aligned Clickless Crossfade between dry input and processed DSP output
        let out_l = dry_l * (1.0 - fade) + dsp_out_l * fade;
        let out_r = dry_r * (1.0 - fade) + dsp_out_r * fade;

        // 9. Push sample into AI analyzer for real-time spectral metrics & visualizer
        self.ai_analyzer.push_sample(out_l, out_r);

        (out_l, out_r)
    }

    pub fn reset(&mut self) {
        self.eq.reset();
        self.psycho_bass.reset();
        self.transient_shaper.reset();
        self.exciter.reset();
        self.spatializer.reset();
        self.compressor.reset();
        self.limiter.reset();
        self.dc_blocker_l.reset();
        self.dc_blocker_r.reset();
        self.dry_delay_l.fill(0.0);
        self.dry_delay_r.fill(0.0);
    }
}

pub type SharedPipeline = Arc<Mutex<AudioPipeline>>;
