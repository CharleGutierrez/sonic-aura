//! Precision Bi-quad Digital IIR Filter Implementation
//! Based on Robert Bristow-Johnson's Audio EQ Cookbook formulas.

use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterType {
    LowPass,
    HighPass,
    BandPass,
    Notch,
    Peaking,
    LowShelf,
    HighShelf,
    AllPass,
}

#[derive(Debug, Clone)]
pub struct BiquadCoefficients {
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub a1: f32,
    pub a2: f32,
}

impl BiquadCoefficients {
    pub fn identity() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }

    /// Calculate filter coefficients for a given sample rate, filter type, frequency, Q, and gain in dB.
    pub fn calculate(
        filter_type: FilterType,
        sample_rate: f32,
        frequency: f32,
        q: f32,
        gain_db: f32,
    ) -> Self {
        let nyquist = sample_rate * 0.499;
        let freq = frequency.clamp(10.0, nyquist);
        let q = q.max(0.01);
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * freq / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);

        let (b0, b1, b2, a0, a1, a2) = match filter_type {
            FilterType::LowPass => {
                let b0 = (1.0 - cos_w0) * 0.5;
                let b1 = 1.0 - cos_w0;
                let b2 = (1.0 - cos_w0) * 0.5;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighPass => {
                let b0 = (1.0 + cos_w0) * 0.5;
                let b1 = -(1.0 + cos_w0);
                let b2 = (1.0 + cos_w0) * 0.5;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::BandPass => {
                let b0 = alpha;
                let b1 = 0.0;
                let b2 = -alpha;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::Notch => {
                let b0 = 1.0;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::Peaking => {
                let b0 = 1.0 + alpha * a;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0 - alpha * a;
                let a0 = 1.0 + alpha / a;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha / a;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::LowShelf => {
                let sqrt_a = a.sqrt();
                let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha);
                let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
                let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha);
                let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha;
                let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
                let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighShelf => {
                let sqrt_a = a.sqrt();
                let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha);
                let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
                let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * air_fix(alpha, sqrt_a));
                let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha;
                let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
                let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::AllPass => {
                let b0 = 1.0 - alpha;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0 + alpha;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
        };

        let inv_a0 = 1.0 / a0;
        Self {
            b0: b0 * inv_a0,
            b1: b1 * inv_a0,
            b2: b2 * inv_a0,
            a1: a1 * inv_a0,
            a2: a2 * inv_a0,
        }
    }
}

#[inline(always)]
fn air_fix(alpha: f32, _sqrt_a: f32) -> f32 {
    alpha
}

/// State variable Direct Form II Transposed Biquad Filter
#[derive(Debug, Clone)]
pub struct Biquad {
    filter_type: FilterType,
    freq: f32,
    q: f32,
    gain_db: f32,
    sample_rate: f32,
    coeffs: BiquadCoefficients,
    // Filter state for direct-form II transposed
    s1: f32,
    s2: f32,
}

impl Biquad {
    pub fn new(filter_type: FilterType, freq: f32, q: f32, gain_db: f32, sample_rate: f32) -> Self {
        let coeffs = BiquadCoefficients::calculate(filter_type, sample_rate, freq, q, gain_db);
        Self {
            filter_type,
            freq,
            q,
            gain_db,
            sample_rate,
            coeffs,
            s1: 0.0,
            s2: 0.0,
        }
    }

    pub fn set_params(&mut self, freq: f32, q: f32, gain_db: f32) {
        self.freq = freq;
        self.q = q;
        self.gain_db = gain_db;
        self.coeffs = BiquadCoefficients::calculate(
            self.filter_type,
            self.sample_rate,
            self.freq,
            self.q,
            self.gain_db,
        );
    }

    pub fn set_gain(&mut self, gain_db: f32) {
        if (self.gain_db - gain_db).abs() > 0.001 {
            self.gain_db = gain_db;
            self.coeffs = BiquadCoefficients::calculate(
                self.filter_type,
                self.sample_rate,
                self.freq,
                self.q,
                self.gain_db,
            );
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.coeffs = BiquadCoefficients::calculate(
            self.filter_type,
            self.sample_rate,
            self.freq,
            self.q,
            self.gain_db,
        );
        self.reset();
    }

    pub fn reset(&mut self) {
        self.s1 = 0.0;
        self.s2 = 0.0;
    }

    #[inline(always)]
    pub fn process(&mut self, input: f32) -> f32 {
        // Direct Form II Transposed implementation (superior numerical stability)
        let out = self.coeffs.b0 * input + self.s1;
        self.s1 = self.coeffs.b1 * input - self.coeffs.a1 * out + self.s2;
        self.s2 = self.coeffs.b2 * input - self.coeffs.a2 * out;

        // Underflow protection / flush denormals
        if self.s1.abs() < 1e-15 {
            self.s1 = 0.0;
        }
        if self.s2.abs() < 1e-15 {
            self.s2 = 0.0;
        }

        out
    }
}
