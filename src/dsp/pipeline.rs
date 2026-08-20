//! Master Audio Processing Engine Pipeline
//! Unifies EQ, AI Analysis, Psychoacoustic Bass, Harmonic Excitation, 3D Spatializer,
//! Multiband Compressor, and True-Peak Limiter into a high-performance, real-time audio pipeline.

use crate::dsp::ai_analyzer::AiSpectralAnalyzer;
use crate::dsp::compressor::MultibandCompressor;
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
            ai_intensity: 0.8,
            bass_boost_intensity: 0.75,
            bass_speaker_protect: false,
            exciter_air_mix: 0.5,
            exciter_drive: 0.6,
            spatial_width: 1.25,
            spatial_depth: 0.35,
            spatial_crossfeed: 0.45,
            spatial_mode: SpatialMode::HeadphonesBinaural,
            transient_attack: 0.4,
            compressor_intensity: 0.6,
            dynamic_loudness: 0.4,
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
}

impl AudioPipeline {
    pub fn new(sample_rate: f32) -> Self {
        let config = PipelineConfig::default();
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
    }

    pub fn apply_config(&mut self, config: &PipelineConfig) {
        self.config = config.clone();
        self.psycho_bass.set_intensity(config.bass_boost_intensity);
        self.psycho_bass.set_speaker_protection(config.bass_speaker_protect);
        self.exciter.set_air_mix(config.exciter_air_mix);
        self.exciter.set_drive(config.exciter_drive);
        self.spatializer.set_mode(config.spatial_mode);
        self.spatializer.set_width(config.spatial_width);
        self.spatializer.set_depth(config.spatial_depth);
        self.spatializer.set_crossfeed(config.spatial_crossfeed);
        self.transient_shaper.set_attack(config.transient_attack);
        self.compressor.set_intensity(config.compressor_intensity);
        self.compressor.set_dynamic_loudness(config.dynamic_loudness);
        self.ai_analyzer.set_ai_enhancement_amount(if config.ai_boost_enabled { config.ai_intensity } else { 0.0 });
    }

    #[inline(always)]
    pub fn process_stereo_sample(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        if !self.config.enabled {
            self.ai_analyzer.push_sample(in_l, in_r);
            return (in_l, in_r);
        }

        // Apply Master Input / Output Preamp
        let master_gain = 10.0_f32.powf(self.config.master_gain_db / 20.0);
        let s_l = in_l * master_gain;
        let s_r = in_r * master_gain;

        // 1. Multi-band Equalizer
        let (eq_l, eq_r) = self.eq.process(s_l, s_r);

        // 2. Psychoacoustic Sub-Bass Enhancement (Missing Fundamental)
        let effective_bass_intensity = if self.config.ai_boost_enabled {
            self.config.bass_boost_intensity * self.ai_analyzer.adaptive_params.dynamic_bass_intensity_mod
        } else {
            self.config.bass_boost_intensity
        };
        self.psycho_bass.set_intensity(effective_bass_intensity);
        let (bass_l, bass_r) = self.psycho_bass.process(eq_l, eq_r);

        // 3. Transient Shaper (Punch & Attack)
        let (trans_l, trans_r) = self.transient_shaper.process(bass_l, bass_r);

        // 4. Harmonic Exciter & High-Frequency Air (B&O Sheen)
        let effective_air = if self.config.ai_boost_enabled {
            self.config.exciter_air_mix * self.ai_analyzer.adaptive_params.dynamic_exciter_air_mod
        } else {
            self.config.exciter_air_mix
        };
        self.exciter.set_air_mix(effective_air);
        let (exc_l, exc_r) = self.exciter.process(trans_l, trans_r);

        // 5. 3D Binaural Spatializer & Soundstage Widener
        let effective_width = if self.config.ai_boost_enabled {
            self.config.spatial_width * self.ai_analyzer.adaptive_params.dynamic_spatial_width_mod
        } else {
            self.config.spatial_width
        };
        self.spatializer.set_width(effective_width);
        let (spat_l, spat_r) = self.spatializer.process(exc_l, exc_r);

        // 6. Multiband Compressor & Fletcher-Munson Loudness
        let (comp_l, comp_r) = self.compressor.process(spat_l, spat_r);

        // 7. True-Peak Lookahead Brickwall Limiter & Soft Clipper
        let (out_l, out_r) = self.limiter.process(comp_l, comp_r);

        // 8. Push sample into AI analyzer for real-time spectral metrics & visualizer
        self.ai_analyzer.push_sample(out_l, out_r);

        (out_l, out_r)
    }

    /// Process an interleaved stereo buffer in-place: [L0, R0, L1, R1, ...]
    pub fn process_interleaved_stereo(&mut self, buffer: &mut [f32]) {
        for chunk in buffer.chunks_exact_mut(2) {
            let (l, r) = self.process_stereo_sample(chunk[0], chunk[1]);
            chunk[0] = l;
            chunk[1] = r;
        }
    }

    pub fn reset(&mut self) {
        self.eq.reset();
        self.psycho_bass.reset();
        self.transient_shaper.reset();
        self.exciter.reset();
        self.spatializer.reset();
        self.compressor.reset();
        self.limiter.reset();
    }
}

pub type SharedPipeline = Arc<Mutex<AudioPipeline>>;
