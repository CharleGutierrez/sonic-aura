//! Multi-Band Precision Parametric & Graphic Equalizer
//! Supports standard 10-band and 15-band ISO frequency bands, plus custom parametric nodes.

use crate::dsp::biquad::{Biquad, FilterType};
use serde::{Deserialize, Serialize};

pub const DEFAULT_EQ_FREQS_10: [f32; 10] = [
    31.0, 63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

pub const DEFAULT_EQ_FREQS_15: [f32; 15] = [
    25.0, 40.0, 63.0, 100.0, 160.0, 250.0, 400.0, 630.0, 1000.0, 1600.0, 2500.0, 4000.0, 6300.0, 10000.0, 16000.0,
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqBandConfig {
    pub freq: f32,
    pub q: f32,
    pub gain_db: f32,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct EqBand {
    pub config: EqBandConfig,
    filter_l: Biquad,
    filter_r: Biquad,
}

impl EqBand {
    pub fn new(freq: f32, q: f32, gain_db: f32, sample_rate: f32) -> Self {
        Self {
            config: EqBandConfig {
                freq,
                q,
                gain_db,
                enabled: true,
            },
            filter_l: Biquad::new(FilterType::Peaking, freq, q, gain_db, sample_rate),
            filter_r: Biquad::new(FilterType::Peaking, freq, q, gain_db, sample_rate),
        }
    }

    pub fn set_gain(&mut self, gain_db: f32) {
        self.config.gain_db = gain_db.clamp(-24.0, 24.0);
        self.filter_l.set_gain(self.config.gain_db);
        self.filter_r.set_gain(self.config.gain_db);
    }

    pub fn set_freq(&mut self, freq: f32) {
        self.config.freq = freq;
        self.filter_l.set_params(self.config.freq, self.config.q, self.config.gain_db);
        self.filter_r.set_params(self.config.freq, self.config.q, self.config.gain_db);
    }

    pub fn set_q(&mut self, q: f32) {
        self.config.q = q.clamp(0.1, 10.0);
        self.filter_l.set_params(self.config.freq, self.config.q, self.config.gain_db);
        self.filter_r.set_params(self.config.freq, self.config.q, self.config.gain_db);
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.filter_l.set_sample_rate(sample_rate);
        self.filter_r.set_sample_rate(sample_rate);
    }

    #[inline(always)]
    pub fn process(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        if !self.config.enabled || self.config.gain_db.abs() < 0.01 {
            (in_l, in_r)
        } else {
            (self.filter_l.process(in_l), self.filter_r.process(in_r))
        }
    }

    pub fn reset(&mut self) {
        self.filter_l.reset();
        self.filter_r.reset();
    }
}

#[derive(Debug, Clone)]
pub struct Equalizer {
    sample_rate: f32,
    pub bands: Vec<EqBand>,
    preamp_db: f32,
    preamp_linear: f32,
    enabled: bool,
}

impl Equalizer {
    pub fn new_10_band(sample_rate: f32) -> Self {
        let bands = DEFAULT_EQ_FREQS_10
            .iter()
            .map(|&freq| EqBand::new(freq, 1.414, 0.0, sample_rate))
            .collect();

        Self {
            sample_rate,
            bands,
            preamp_db: 0.0,
            preamp_linear: 1.0,
            enabled: true,
        }
    }

    pub fn new_15_band(sample_rate: f32) -> Self {
        let bands = DEFAULT_EQ_FREQS_15
            .iter()
            .map(|&freq| EqBand::new(freq, 1.8, 0.0, sample_rate))
            .collect();

        Self {
            sample_rate,
            bands,
            preamp_db: 0.0,
            preamp_linear: 1.0,
            enabled: true,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        for band in &mut self.bands {
            band.set_sample_rate(sample_rate);
        }
    }

    pub fn set_band_gain(&mut self, band_idx: usize, gain_db: f32) {
        if let Some(band) = self.bands.get_mut(band_idx) {
            band.set_gain(gain_db);
        }
    }

    pub fn get_band_gain(&self, band_idx: usize) -> f32 {
        self.bands.get(band_idx).map(|b| b.config.gain_db).unwrap_or(0.0)
    }

    pub fn set_preamp(&mut self, gain_db: f32) {
        self.preamp_db = gain_db.clamp(-24.0, 24.0);
        self.preamp_linear = 10.0_f32.powf(self.preamp_db / 20.0);
    }

    pub fn get_preamp(&self) -> f32 {
        self.preamp_db
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_all_gains(&mut self, gains: &[f32]) {
        for (i, &gain) in gains.iter().enumerate() {
            if i < self.bands.len() {
                self.bands[i].set_gain(gain);
            }
        }
    }

    pub fn reset_all_bands(&mut self) {
        for band in &mut self.bands {
            band.set_gain(0.0);
        }
        self.set_preamp(0.0);
    }

    #[inline(always)]
    pub fn process(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        if !self.enabled {
            return (in_l, in_r);
        }

        let mut l = in_l * self.preamp_linear;
        let mut r = in_r * self.preamp_linear;

        for band in &mut self.bands {
            let (next_l, next_r) = band.process(l, r);
            l = next_l;
            r = next_r;
        }

        (l, r)
    }

    pub fn reset(&mut self) {
        for band in &mut self.bands {
            band.reset();
        }
    }
}
