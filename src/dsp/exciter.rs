//! Crystalline Harmonic Exciter & High-Frequency Air Engine (B&O / Aphex Aural Exciter Style)
//! Generates subtle euphonic harmonics and high-frequency "air" (>10kHz)
//! to restore lost transient sparkle, vocal breath, and acoustic clarity.

use crate::dsp::biquad::{Biquad, FilterType};

#[derive(Debug, Clone)]
pub struct HarmonicExciter {
    sample_rate: f32,
    cutoff_hz: f32,       // Exciter crossover frequency (typically 4.5kHz - 8kHz)
    drive: f32,           // Non-linear harmonic generation drive
    air_mix: f32,         // Wet mix level of the generated air/sparkle
    warmth: f32,          // Ratio of 2nd (warm) vs 3rd (crisp) harmonics

    // Highpass filters to isolate high frequencies for excitation
    hpf_in_l: Biquad,
    hpf_in_r: Biquad,

    // Highpass filters to clean up harmonics and extract the ultra-high "Air" band
    hpf_air_l: Biquad,
    hpf_air_r: Biquad,

    // High shelf filter for air shimmer boost
    shelf_l: Biquad,
    shelf_r: Biquad,

    // Dynamic high-frequency envelope tracking
    hf_envelope_l: f32,
    hf_envelope_r: f32,
}

impl HarmonicExciter {
    pub fn new(sample_rate: f32) -> Self {
        let cutoff_hz = 6000.0;
        let air_cutoff = 10500.0;

        let mut instance = Self {
            sample_rate,
            cutoff_hz,
            drive: 0.6,
            air_mix: 0.45,
            warmth: 0.35,

            hpf_in_l: Biquad::new(FilterType::HighPass, cutoff_hz, 0.707, 0.0, sample_rate),
            hpf_in_r: Biquad::new(FilterType::HighPass, cutoff_hz, 0.707, 0.0, sample_rate),

            hpf_air_l: Biquad::new(FilterType::HighPass, air_cutoff, 0.707, 0.0, sample_rate),
            hpf_air_r: Biquad::new(FilterType::HighPass, air_cutoff, 0.707, 0.0, sample_rate),

            shelf_l: Biquad::new(FilterType::HighShelf, 12000.0, 0.707, 2.5, sample_rate),
            shelf_r: Biquad::new(FilterType::HighShelf, 12000.0, 0.707, 2.5, sample_rate),

            hf_envelope_l: 0.0,
            hf_envelope_r: 0.0,
        };
        instance.update_filters();
        instance
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.update_filters();
    }

    pub fn set_drive(&mut self, drive: f32) {
        self.drive = drive.clamp(0.0, 2.0);
    }

    pub fn set_air_mix(&mut self, mix: f32) {
        self.air_mix = mix.clamp(0.0, 1.5);
    }

    pub fn set_cutoff(&mut self, cutoff_hz: f32) {
        self.cutoff_hz = cutoff_hz.clamp(3000.0, 12000.0);
        self.update_filters();
    }

    pub fn set_warmth(&mut self, warmth: f32) {
        self.warmth = warmth.clamp(0.0, 1.0);
    }

    fn update_filters(&mut self) {
        let cutoff = self.cutoff_hz;
        let air_cutoff = (cutoff * 1.6).min(self.sample_rate * 0.45);

        self.hpf_in_l = Biquad::new(FilterType::HighPass, cutoff, 0.707, 0.0, self.sample_rate);
        self.hpf_in_r = Biquad::new(FilterType::HighPass, cutoff, 0.707, 0.0, self.sample_rate);

        self.hpf_air_l = Biquad::new(FilterType::HighPass, air_cutoff, 0.707, 0.0, self.sample_rate);
        self.hpf_air_r = Biquad::new(FilterType::HighPass, air_cutoff, 0.707, 0.0, self.sample_rate);

        self.shelf_l = Biquad::new(FilterType::HighShelf, 12000.0, 0.707, 2.5, self.sample_rate);
        self.shelf_r = Biquad::new(FilterType::HighShelf, 12000.0, 0.707, 2.5, self.sample_rate);
    }

    #[inline(always)]
    pub fn process(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        if self.air_mix <= 0.001 && self.drive <= 0.001 {
            return (in_l, in_r);
        }

        // 1. Isolate high-frequency band
        let hf_l = self.hpf_in_l.process(in_l);
        let hf_r = self.hpf_in_r.process(in_r);

        // 2. Track fast HF envelope
        let env_attack = 0.15;
        let env_release = 0.005;
        self.hf_envelope_l += (hf_l.abs() - self.hf_envelope_l) * (if hf_l.abs() > self.hf_envelope_l { env_attack } else { env_release });
        self.hf_envelope_r += (hf_r.abs() - self.hf_envelope_r) * (if hf_r.abs() > self.hf_envelope_r { env_attack } else { env_release });

        // 3. Generate non-linear euphonic harmonics
        // Soft polynomial saturation: mix of even (warmth) and odd (crisp bite) harmonics
        let gain = 1.0 + self.drive * 3.5;
        let scaled_l = (hf_l * gain).clamp(-3.0, 3.0);
        let scaled_r = (hf_r * gain).clamp(-3.0, 3.0);

        // Even harmonic: x^2 (asymmetric)
        let even_l = self.warmth * (scaled_l.abs() * scaled_l * 0.4);
        let even_r = self.warmth * (scaled_r.abs() * scaled_r * 0.4);

        // Odd harmonic: x - x^3/3 (symmetric soft clipping)
        let odd_l = (1.0 - self.warmth * 0.5) * (scaled_l - 0.28 * scaled_l.powi(3));
        let odd_r = (1.0 - self.warmth * 0.5) * (scaled_r - 0.28 * scaled_r.powi(3));

        let sat_l = (even_l + odd_l).tanh();
        let sat_r = (even_r + odd_r).tanh();

        // 4. Extract ultra-high sparkle and add high-shelf shimmer
        let air_l = self.shelf_l.process(self.hpf_air_l.process(sat_l));
        let air_r = self.shelf_r.process(self.hpf_air_r.process(sat_r));

        let out_l = in_l + air_l * self.air_mix;
        let out_r = in_r + air_r * self.air_mix;

        (out_l, out_r)
    }

    pub fn reset(&mut self) {
        self.hpf_in_l.reset();
        self.hpf_in_r.reset();
        self.hpf_air_l.reset();
        self.hpf_air_r.reset();
        self.shelf_l.reset();
        self.shelf_r.reset();
        self.hf_envelope_l = 0.0;
        self.hf_envelope_r = 0.0;
    }
}
