//! Clinical Audiogram Compensation & Interactive Tinnitus Therapy Suite
//!
//! Features:
//! 1. Multi-band Audiogram Loss Compensation
//! 2. Tailor-Made Notched Music Training (TMNST) for auditory cortex lateral inhibition
//! 3. Soothing Narrowband Pink Noise Masker (Bandpass-shaped ambient therapy)
//! 4. Pitch-Matching Sine Calibrator (Safe -30 dBFS test tone)
//! 5. Selective Ear Targeting (Both Ears, Left Only, Right Only)

use crate::dsp::biquad::{Biquad, FilterType};
use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TinnitusTherapyMode {
    Off,
    Notch,    // Tailor-Made Notched Music Training (TMNST)
    Masking,  // Narrowband Ambient Masking Noise
    Combined, // Notch + Masking
}

impl TinnitusTherapyMode {
    pub const ALL: [Self; 4] = [
        Self::Off,
        Self::Notch,
        Self::Masking,
        Self::Combined,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Notch => "Notch (TMNST)",
            Self::Masking => "Masking Noise",
            Self::Combined => "Notch + Mask",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Off => "Tinnitus relief disabled",
            Self::Notch => "Tailor-Made Notched Music (Auditory Lateral Inhibition)",
            Self::Masking => "Soothing Narrowband Ambient Acoustic Masking",
            Self::Combined => "TMNST Notch Filter + Soft Ambient Noise Masker",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "notch" | "tmnst" => Self::Notch,
            "mask" | "masking" => Self::Masking,
            "both" | "combined" => Self::Combined,
            _ => Self::Off,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Notch => "notch",
            Self::Masking => "mask",
            Self::Combined => "combined",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TinnitusEar {
    Both,
    LeftOnly,
    RightOnly,
}

impl TinnitusEar {
    pub const ALL: [Self; 3] = [Self::Both, Self::LeftOnly, Self::RightOnly];

    pub fn name(&self) -> &'static str {
        match self {
            Self::Both => "Both Ears",
            Self::LeftOnly => "Left Ear Only",
            Self::RightOnly => "Right Ear Only",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "left" | "leftonly" => Self::LeftOnly,
            "right" | "rightonly" => Self::RightOnly,
            _ => Self::Both,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            Self::Both => "both",
            Self::LeftOnly => "left",
            Self::RightOnly => "right",
        }
    }
}

pub struct AudiogramMasker {
    sample_rate: f32,

    // Clinical Audiogram compensation filters
    filters_l: Vec<Biquad>,
    filters_r: Vec<Biquad>,

    // Tinnitus Therapy parameters
    pub mode: TinnitusTherapyMode,
    pub ear: TinnitusEar,
    pub freq: f32,
    pub q: f32,
    pub mask_level_db: f32,
    mask_linear_gain: f32,

    // Notch filters for TMNST
    notch_filter_l: Biquad,
    notch_filter_r: Biquad,

    // Narrowband masker filters
    masker_filter_l: Biquad,
    masker_filter_r: Biquad,

    // Noise generation state (Paul Kellet 3-pole pink noise filter)
    noise_phase: f32,
    pink_b0: f32,
    pink_b1: f32,
    pink_b2: f32,

    // Pitch matching test tone generator (Soft sine wave, max -30 dBFS)
    pub test_tone_active: bool,
    test_tone_phase: f32,
    test_tone_envelope: f32,
}

impl AudiogramMasker {
    pub fn new(sample_rate: f32) -> Self {
        let freq = 6000.0;
        let q = 4.0;
        let mask_level_db = -36.0;

        let notch_l = Biquad::new(FilterType::Notch, freq, q, 0.0, sample_rate);
        let notch_r = Biquad::new(FilterType::Notch, freq, q, 0.0, sample_rate);
        let mask_l = Biquad::new(FilterType::BandPass, freq, q, 0.0, sample_rate);
        let mask_r = Biquad::new(FilterType::BandPass, freq, q, 0.0, sample_rate);

        Self {
            sample_rate,
            filters_l: Vec::new(),
            filters_r: Vec::new(),
            mode: TinnitusTherapyMode::Off,
            ear: TinnitusEar::Both,
            freq,
            q,
            mask_level_db,
            mask_linear_gain: 10.0_f32.powf(mask_level_db / 20.0),
            notch_filter_l: notch_l,
            notch_filter_r: notch_r,
            masker_filter_l: mask_l,
            masker_filter_r: mask_r,
            noise_phase: 0.0,
            pink_b0: 0.0,
            pink_b1: 0.0,
            pink_b2: 0.0,
            test_tone_active: false,
            test_tone_phase: 0.0,
            test_tone_envelope: 0.0,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.update_filters();
        for f in self.filters_l.iter_mut() {
            f.set_sample_rate(sample_rate);
        }
        for f in self.filters_r.iter_mut() {
            f.set_sample_rate(sample_rate);
        }
    }

    pub fn update_filters(&mut self) {
        self.notch_filter_l =
            Biquad::new(FilterType::Notch, self.freq, self.q, 0.0, self.sample_rate);
        self.notch_filter_r =
            Biquad::new(FilterType::Notch, self.freq, self.q, 0.0, self.sample_rate);
        self.masker_filter_l =
            Biquad::new(FilterType::BandPass, self.freq, self.q, 0.0, self.sample_rate);
        self.masker_filter_r =
            Biquad::new(FilterType::BandPass, self.freq, self.q, 0.0, self.sample_rate);
    }

    pub fn set_mode(&mut self, mode: TinnitusTherapyMode) {
        self.mode = mode;
    }

    pub fn set_ear(&mut self, ear: TinnitusEar) {
        self.ear = ear;
    }

    pub fn set_freq(&mut self, freq: f32) {
        self.freq = freq.clamp(500.0, 18000.0);
        self.update_filters();
    }

    pub fn set_q(&mut self, q: f32) {
        self.q = q.clamp(1.0, 16.0);
        self.update_filters();
    }

    pub fn set_mask_level_db(&mut self, level_db: f32) {
        self.mask_level_db = level_db.clamp(-70.0, -12.0);
        self.mask_linear_gain = 10.0_f32.powf(self.mask_level_db / 20.0);
    }

    pub fn set_test_tone(&mut self, active: bool) {
        self.test_tone_active = active;
    }

    pub fn toggle_test_tone(&mut self) -> bool {
        self.test_tone_active = !self.test_tone_active;
        self.test_tone_active
    }

    /// Sets clinical audiogram thresholds (frequencies in Hz, hearing loss in dB)
    pub fn set_audiogram(&mut self, frequencies: &[f32], hearing_loss_db: &[f32]) {
        self.filters_l.clear();
        self.filters_r.clear();
        for (&f, &loss) in frequencies.iter().zip(hearing_loss_db.iter()) {
            let gain = loss * 0.5;
            self.filters_l.push(Biquad::new(
                FilterType::Peaking,
                f,
                1.0,
                gain,
                self.sample_rate,
            ));
            self.filters_r.push(Biquad::new(
                FilterType::Peaking,
                f,
                1.0,
                gain,
                self.sample_rate,
            ));
        }
    }

    pub fn reset(&mut self) {
        for f in self.filters_l.iter_mut() {
            f.reset();
        }
        for f in self.filters_r.iter_mut() {
            f.reset();
        }
        self.notch_filter_l.reset();
        self.notch_filter_r.reset();
        self.masker_filter_l.reset();
        self.masker_filter_r.reset();
        self.pink_b0 = 0.0;
        self.pink_b1 = 0.0;
        self.pink_b2 = 0.0;
        self.test_tone_phase = 0.0;
        self.test_tone_envelope = 0.0;
    }

    #[inline(always)]
    pub fn process(&mut self, mut in_l: f32, mut in_r: f32) -> (f32, f32) {
        // 1. Clinical Audiogram Loss Compensation
        for f in self.filters_l.iter_mut() {
            in_l = f.process(in_l);
        }
        for f in self.filters_r.iter_mut() {
            in_r = f.process(in_r);
        }

        // 2. Tailor-Made Notched Music Training (TMNST)
        let applies_notch =
            self.mode == TinnitusTherapyMode::Notch || self.mode == TinnitusTherapyMode::Combined;
        if applies_notch {
            if self.ear == TinnitusEar::Both || self.ear == TinnitusEar::LeftOnly {
                in_l = self.notch_filter_l.process(in_l);
            }
            if self.ear == TinnitusEar::Both || self.ear == TinnitusEar::RightOnly {
                in_r = self.notch_filter_r.process(in_r);
            }
        }

        // 3. Narrowband Soothing Ambient Pink Noise Masker
        let applies_mask =
            self.mode == TinnitusTherapyMode::Masking || self.mode == TinnitusTherapyMode::Combined;
        if applies_mask {
            // Generate soothing pink noise
            self.noise_phase += 1.0;
            if self.noise_phase > 100000.0 {
                self.noise_phase = 0.0;
            }
            let white = ((self.noise_phase * 12.9898 + 78.233).sin() * 43758.5453).fract() * 2.0
                - 1.0;

            self.pink_b0 = 0.99886 * self.pink_b0 + white * 0.0555179;
            self.pink_b1 = 0.99332 * self.pink_b1 + white * 0.0750759;
            self.pink_b2 = 0.96900 * self.pink_b2 + white * 0.1538520;
            let pink = (self.pink_b0 + self.pink_b1 + self.pink_b2 + white * 0.5362) * 0.15;

            let masked_l = self.masker_filter_l.process(pink) * self.mask_linear_gain;
            let masked_r = self.masker_filter_r.process(pink) * self.mask_linear_gain;

            if self.ear == TinnitusEar::Both || self.ear == TinnitusEar::LeftOnly {
                in_l += masked_l;
            }
            if self.ear == TinnitusEar::Both || self.ear == TinnitusEar::RightOnly {
                in_r += masked_r;
            }
        }

        // 4. Safe Pitch-Matching Test Tone Generator (-30 dBFS peak)
        let target_env = if self.test_tone_active { 1.0 } else { 0.0 };
        self.test_tone_envelope += (target_env - self.test_tone_envelope) * 0.005;

        if self.test_tone_envelope > 0.0001 {
            let phase_step = 2.0 * PI * self.freq / self.sample_rate;
            self.test_tone_phase += phase_step;
            if self.test_tone_phase > 2.0 * PI {
                self.test_tone_phase -= 2.0 * PI;
            }

            // Soft pure sine at -30 dBFS (amplitude 0.03162)
            let tone = self.test_tone_phase.sin() * 0.03162 * self.test_tone_envelope;

            if self.ear == TinnitusEar::Both || self.ear == TinnitusEar::LeftOnly {
                in_l += tone;
            }
            if self.ear == TinnitusEar::Both || self.ear == TinnitusEar::RightOnly {
                in_r += tone;
            }
        }

        (in_l, in_r)
    }
}

