//! Multi-Band Intelligent Dynamic Compressor & Fletcher-Munson Loudness Engine
//! Keeps audio punchy, full-bodied, and consistent across varying volume levels.

use crate::dsp::biquad::{Biquad, FilterType};

#[derive(Debug, Clone)]
struct CompressorBand {
    threshold_db: f32,
    ratio: f32,
    attack_coeff: f32,
    release_coeff: f32,
    makeup_db: f32,
    makeup_linear: f32,
    envelope: f32,
}

impl CompressorBand {
    fn new(threshold_db: f32, ratio: f32, attack_ms: f32, release_ms: f32, makeup_db: f32, sample_rate: f32) -> Self {
        let attack_coeff = (-1.0 / (attack_ms * 0.001 * sample_rate)).exp();
        let release_coeff = (-1.0 / (release_ms * 0.001 * sample_rate)).exp();
        let makeup_linear = 10.0_f32.powf(makeup_db / 20.0);

        Self {
            threshold_db,
            ratio,
            attack_coeff,
            release_coeff,
            makeup_db,
            makeup_linear,
            envelope: 0.0,
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32, attack_ms: f32, release_ms: f32) {
        self.attack_coeff = (-1.0 / (attack_ms * 0.001 * sample_rate)).exp();
        self.release_coeff = (-1.0 / (release_ms * 0.001 * sample_rate)).exp();
    }

    fn set_makeup(&mut self, makeup_db: f32) {
        self.makeup_db = makeup_db;
        self.makeup_linear = 10.0_f32.powf(makeup_db / 20.0);
    }

    #[inline(always)]
    fn process(&mut self, in_val: f32) -> f32 {
        let abs_val = in_val.abs();
        let coeff = if abs_val > self.envelope {
            self.attack_coeff
        } else {
            self.release_coeff
        };
        self.envelope = (1.0 - coeff) * abs_val + coeff * self.envelope;

        let env_db = 20.0 * (self.envelope + 1e-6).log10();
        let gain_db = if env_db > self.threshold_db {
            let over_db = env_db - self.threshold_db;
            -over_db * (1.0 - 1.0 / self.ratio)
        } else {
            0.0
        };

        let gain_linear = 10.0_f32.powf(gain_db / 20.0);
        in_val * gain_linear * self.makeup_linear
    }

    fn reset(&mut self) {
        self.envelope = 0.0;
    }
}

#[derive(Debug, Clone)]
pub struct MultibandCompressor {
    sample_rate: f32,
    enabled: bool,
    intensity: f32,       // 0.0 (off) to 1.0 (full studio compression)
    dynamic_loudness: f32,// Fletcher-Munson loudness compensation (0.0 to 1.0)

    // Crossover filters (Linkwitz-Riley 24dB/oct simulation via 2-pole cascaded biquads)
    low_lpf_l: Biquad,
    low_lpf_r: Biquad,
    low_hpf_l: Biquad,
    low_hpf_r: Biquad,

    high_lpf_l: Biquad,
    high_lpf_r: Biquad,
    high_hpf_l: Biquad,
    high_hpf_r: Biquad,

    // 3 Dynamic Bands: Low (<200Hz), Mid (200-4500Hz), High (>4500Hz)
    comp_low_l: CompressorBand,
    comp_low_r: CompressorBand,
    comp_mid_l: CompressorBand,
    comp_mid_r: CompressorBand,
    comp_high_l: CompressorBand,
    comp_high_r: CompressorBand,

    // Loudness curve shaping filters
    loudness_bass_l: Biquad,
    loudness_bass_r: Biquad,
    loudness_treble_l: Biquad,
    loudness_treble_r: Biquad,
}

impl MultibandCompressor {
    pub fn new(sample_rate: f32) -> Self {
        let low_cross = 220.0;
        let high_cross = 4500.0;

        let comp_low_l = CompressorBand::new(-16.0, 3.2, 15.0, 120.0, 2.0, sample_rate);
        let comp_low_r = CompressorBand::new(-16.0, 3.2, 15.0, 120.0, 2.0, sample_rate);
        let comp_mid_l = CompressorBand::new(-18.0, 2.5, 8.0, 80.0, 1.5, sample_rate);
        let comp_mid_r = CompressorBand::new(-18.0, 2.5, 8.0, 80.0, 1.5, sample_rate);
        let comp_high_l = CompressorBand::new(-15.0, 2.8, 3.0, 60.0, 1.8, sample_rate);
        let comp_high_r = CompressorBand::new(-15.0, 2.8, 3.0, 60.0, 1.8, sample_rate);

        let mut instance = Self {
            sample_rate,
            enabled: true,
            intensity: 0.65,
            dynamic_loudness: 0.4,

            low_lpf_l: Biquad::new(FilterType::LowPass, low_cross, 0.707, 0.0, sample_rate),
            low_lpf_r: Biquad::new(FilterType::LowPass, low_cross, 0.707, 0.0, sample_rate),
            low_hpf_l: Biquad::new(FilterType::HighPass, low_cross, 0.707, 0.0, sample_rate),
            low_hpf_r: Biquad::new(FilterType::HighPass, low_cross, 0.707, 0.0, sample_rate),

            high_lpf_l: Biquad::new(FilterType::LowPass, high_cross, 0.707, 0.0, sample_rate),
            high_lpf_r: Biquad::new(FilterType::LowPass, high_cross, 0.707, 0.0, sample_rate),
            high_hpf_l: Biquad::new(FilterType::HighPass, high_cross, 0.707, 0.0, sample_rate),
            high_hpf_r: Biquad::new(FilterType::HighPass, high_cross, 0.707, 0.0, sample_rate),

            comp_low_l,
            comp_low_r,
            comp_mid_l,
            comp_mid_r,
            comp_high_l,
            comp_high_r,

            loudness_bass_l: Biquad::new(FilterType::LowShelf, 90.0, 0.707, 3.0, sample_rate),
            loudness_bass_r: Biquad::new(FilterType::LowShelf, 90.0, 0.707, 3.0, sample_rate),
            loudness_treble_l: Biquad::new(FilterType::HighShelf, 9000.0, 0.707, 2.5, sample_rate),
            loudness_treble_r: Biquad::new(FilterType::HighShelf, 9000.0, 0.707, 2.5, sample_rate),
        };
        instance.update_filters();
        instance
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn set_intensity(&mut self, intensity: f32) {
        self.intensity = intensity.clamp(0.0, 1.5);
    }

    pub fn set_dynamic_loudness(&mut self, loudness: f32) {
        self.dynamic_loudness = loudness.clamp(0.0, 1.5);
        let bass_gain = self.dynamic_loudness * 4.5;
        let treble_gain = self.dynamic_loudness * 3.5;
        self.loudness_bass_l.set_gain(bass_gain);
        self.loudness_bass_r.set_gain(bass_gain);
        self.loudness_treble_l.set_gain(treble_gain);
        self.loudness_treble_r.set_gain(treble_gain);
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.comp_low_l.set_sample_rate(sample_rate, 15.0, 120.0);
        self.comp_low_r.set_sample_rate(sample_rate, 15.0, 120.0);
        self.comp_mid_l.set_sample_rate(sample_rate, 8.0, 80.0);
        self.comp_mid_r.set_sample_rate(sample_rate, 8.0, 80.0);
        self.comp_high_l.set_sample_rate(sample_rate, 3.0, 60.0);
        self.comp_high_r.set_sample_rate(sample_rate, 3.0, 60.0);
        self.update_filters();
    }

    fn update_filters(&mut self) {
        let low_cross = 220.0;
        let high_cross = 4500.0;

        self.low_lpf_l = Biquad::new(FilterType::LowPass, low_cross, 0.707, 0.0, self.sample_rate);
        self.low_lpf_r = Biquad::new(FilterType::LowPass, low_cross, 0.707, 0.0, self.sample_rate);
        self.low_hpf_l = Biquad::new(FilterType::HighPass, low_cross, 0.707, 0.0, self.sample_rate);
        self.low_hpf_r = Biquad::new(FilterType::HighPass, low_cross, 0.707, 0.0, self.sample_rate);

        self.high_lpf_l = Biquad::new(FilterType::LowPass, high_cross, 0.707, 0.0, self.sample_rate);
        self.high_lpf_r = Biquad::new(FilterType::LowPass, high_cross, 0.707, 0.0, self.sample_rate);
        self.high_hpf_l = Biquad::new(FilterType::HighPass, high_cross, 0.707, 0.0, self.sample_rate);
        self.high_hpf_r = Biquad::new(FilterType::HighPass, high_cross, 0.707, 0.0, self.sample_rate);

        let bass_gain = self.dynamic_loudness * 4.5;
        let treble_gain = self.dynamic_loudness * 3.5;
        self.loudness_bass_l = Biquad::new(FilterType::LowShelf, 90.0, 0.707, bass_gain, self.sample_rate);
        self.loudness_bass_r = Biquad::new(FilterType::LowShelf, 90.0, 0.707, bass_gain, self.sample_rate);
        self.loudness_treble_l = Biquad::new(FilterType::HighShelf, 9000.0, 0.707, treble_gain, self.sample_rate);
        self.loudness_treble_r = Biquad::new(FilterType::HighShelf, 9000.0, 0.707, treble_gain, self.sample_rate);
    }

    #[inline(always)]
    pub fn process(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        if !self.enabled || self.intensity <= 0.001 {
            return (in_l, in_r);
        }

        // 1. Crossover 3-band split
        let low_l = self.low_lpf_l.process(in_l);
        let low_r = self.low_lpf_r.process(in_r);

        let rem_l = self.low_hpf_l.process(in_l);
        let rem_r = self.low_hpf_r.process(in_r);

        let mid_l = self.high_lpf_l.process(rem_l);
        let mid_r = self.high_lpf_r.process(rem_r);

        let high_l = self.high_hpf_l.process(rem_l);
        let high_r = self.high_hpf_r.process(rem_r);

        // 2. Compress each band
        let proc_low_l = self.comp_low_l.process(low_l);
        let proc_low_r = self.comp_low_r.process(low_r);

        let proc_mid_l = self.comp_mid_l.process(mid_l);
        let proc_mid_r = self.comp_mid_r.process(mid_r);

        let proc_high_l = self.comp_high_l.process(high_l);
        let proc_high_r = self.comp_high_r.process(high_r);

        // 3. Sum bands with wet/dry blend
        let comp_out_l = proc_low_l + proc_mid_l + proc_high_l;
        let comp_out_r = proc_low_r + proc_mid_r + proc_high_r;

        let blend_l = in_l * (1.0 - self.intensity) + comp_out_l * self.intensity;
        let blend_r = in_r * (1.0 - self.intensity) + comp_out_r * self.intensity;

        // 4. Dynamic Loudness Curve (Fletcher-Munson)
        let out_l = self.loudness_treble_l.process(self.loudness_bass_l.process(blend_l));
        let out_r = self.loudness_treble_r.process(self.loudness_bass_r.process(blend_r));

        (out_l, out_r)
    }

    pub fn reset(&mut self) {
        self.comp_low_l.reset();
        self.comp_low_r.reset();
        self.comp_mid_l.reset();
        self.comp_mid_r.reset();
        self.comp_high_l.reset();
        self.comp_high_r.reset();
        self.low_lpf_l.reset();
        self.low_lpf_r.reset();
        self.low_hpf_l.reset();
        self.low_hpf_r.reset();
        self.high_lpf_l.reset();
        self.high_hpf_r.reset();
        self.high_hpf_l.reset();
        self.high_hpf_r.reset();
        self.loudness_bass_l.reset();
        self.loudness_bass_r.reset();
        self.loudness_treble_l.reset();
        self.loudness_treble_r.reset();
    }
}
