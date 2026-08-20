//! Psychoacoustic Bass Enhancement Engine
//! Utilizes the Missing Fundamental psychoacoustic phenomenon (MaxxBass / B&O Acoustic Lens style)
//! to generate euphonic 2nd and 3rd harmonics of low frequencies, allowing small laptop speakers
//! and headphones to reproduce perceived deep sub-bass without mechanical speaker distortion.

use crate::dsp::biquad::{Biquad, FilterType};

#[derive(Debug, Clone)]
pub struct PsychoacousticBass {
    sample_rate: f32,
    cutoff_hz: f32,
    intensity: f32,         // 0.0 to 2.0 (amount of harmonic bass added)
    speaker_protect: bool,   // High-pass filter direct sub-bass to protect tiny laptop drivers
    direct_sub_gain: f32,    // Gain for original sub-bass

    // Filters for Left & Right channels
    sub_lpf_l: Biquad,
    sub_lpf_r: Biquad,
    sub_hpf_l: Biquad,
    sub_hpf_r: Biquad,

    // Bandpass filters to shape generated harmonics into the audible/resonant band
    harm_bpf_l: Biquad,
    harm_bpf_r: Biquad,

    // Protection highpass filter on the direct path
    protect_hpf_l: Biquad,
    protect_hpf_r: Biquad,

    // Dynamic envelope follower for low-end energy
    envelope_l: f32,
    envelope_r: f32,
}

impl PsychoacousticBass {
    pub fn new(sample_rate: f32) -> Self {
        let cutoff_hz = 120.0;
        let harm_center = cutoff_hz * 1.8; // ~216 Hz

        let mut instance = Self {
            sample_rate,
            cutoff_hz,
            intensity: 0.8,
            speaker_protect: false,
            direct_sub_gain: 1.0,

            sub_lpf_l: Biquad::new(FilterType::LowPass, cutoff_hz, 0.707, 0.0, sample_rate),
            sub_lpf_r: Biquad::new(FilterType::LowPass, cutoff_hz, 0.707, 0.0, sample_rate),
            sub_hpf_l: Biquad::new(FilterType::HighPass, 30.0, 0.707, 0.0, sample_rate),
            sub_hpf_r: Biquad::new(FilterType::HighPass, 30.0, 0.707, 0.0, sample_rate),

            harm_bpf_l: Biquad::new(FilterType::BandPass, harm_center, 1.2, 0.0, sample_rate),
            harm_bpf_r: Biquad::new(FilterType::BandPass, harm_center, 1.2, 0.0, sample_rate),

            protect_hpf_l: Biquad::new(FilterType::HighPass, cutoff_hz * 0.75, 0.707, 0.0, sample_rate),
            protect_hpf_r: Biquad::new(FilterType::HighPass, cutoff_hz * 0.75, 0.707, 0.0, sample_rate),

            envelope_l: 0.0,
            envelope_r: 0.0,
        };
        instance.update_filters();
        instance
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.update_filters();
    }

    pub fn set_intensity(&mut self, intensity: f32) {
        self.intensity = intensity.clamp(0.0, 2.5);
    }

    pub fn set_cutoff(&mut self, cutoff_hz: f32) {
        self.cutoff_hz = cutoff_hz.clamp(50.0, 250.0);
        self.update_filters();
    }

    pub fn set_speaker_protection(&mut self, protect: bool) {
        self.speaker_protect = protect;
    }

    pub fn set_direct_sub_gain(&mut self, gain: f32) {
        self.direct_sub_gain = gain.clamp(0.0, 2.0);
    }

    fn update_filters(&mut self) {
        let cutoff = self.cutoff_hz;
        let harm_center = cutoff * 1.8;

        self.sub_lpf_l = Biquad::new(FilterType::LowPass, cutoff, 0.707, 0.0, self.sample_rate);
        self.sub_lpf_r = Biquad::new(FilterType::LowPass, cutoff, 0.707, 0.0, self.sample_rate);

        self.sub_hpf_l = Biquad::new(FilterType::HighPass, 30.0, 0.707, 0.0, self.sample_rate);
        self.sub_hpf_r = Biquad::new(FilterType::HighPass, 30.0, 0.707, 0.0, self.sample_rate);

        self.harm_bpf_l = Biquad::new(FilterType::BandPass, harm_center, 1.2, 0.0, self.sample_rate);
        self.harm_bpf_r = Biquad::new(FilterType::BandPass, harm_center, 1.2, 0.0, self.sample_rate);

        self.protect_hpf_l = Biquad::new(FilterType::HighPass, cutoff * 0.75, 0.707, 0.0, self.sample_rate);
        self.protect_hpf_r = Biquad::new(FilterType::HighPass, cutoff * 0.75, 0.707, 0.0, self.sample_rate);
    }

    /// Process a stereo sample frame (left, right) -> (out_left, out_right)
    #[inline(always)]
    pub fn process(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        if self.intensity <= 0.001 {
            return (in_l, in_r);
        }

        // 1. Isolate the sub-bass band
        let sub_l = self.sub_hpf_l.process(self.sub_lpf_l.process(in_l));
        let sub_r = self.sub_hpf_r.process(self.sub_lpf_r.process(in_r));

        // 2. Track envelope for dynamic saturation
        let abs_l = sub_l.abs();
        let abs_r = sub_r.abs();
        let attack = 0.05;
        let release = 0.002;
        self.envelope_l += if abs_l > self.envelope_l { attack * (abs_l - self.envelope_l) } else { release * (abs_l - self.envelope_l) };
        self.envelope_r += if abs_r > self.envelope_r { attack * (abs_r - self.envelope_r) } else { release * (abs_r - self.envelope_r) };

        // 3. Generate 2nd and 3rd harmonics using non-linear wave-shaping
        // y2 = 2*x^2 - 1 (even harmonic for warmth/punch)
        // y3 = 4*x^3 - 3*x (odd harmonic for definition/presence)
        let boost_factor = 2.4;
        let norm_l = (sub_l * boost_factor).clamp(-2.0, 2.0);
        let norm_r = (sub_r * boost_factor).clamp(-2.0, 2.0);

        let h2_l = 0.6 * (norm_l * norm_l.abs());
        let h3_l = 0.4 * (norm_l - 0.33 * norm_l.powi(3));
        let raw_harm_l = (h2_l + h3_l).tanh();

        let h2_r = 0.6 * (norm_r * norm_r.abs());
        let h3_r = 0.4 * (norm_r - 0.33 * norm_r.powi(3));
        let raw_harm_r = (h2_r + h3_r).tanh();

        // 4. Filter harmonics to fit into the audible fundamental resonance band
        let harm_filtered_l = self.harm_bpf_l.process(raw_harm_l) * self.intensity;
        let harm_filtered_r = self.harm_bpf_r.process(raw_harm_r) * self.intensity;

        // 5. Clean up direct signal if speaker protection is enabled
        let (direct_l, direct_r) = if self.speaker_protect {
            (self.protect_hpf_l.process(in_l), self.protect_hpf_r.process(in_r))
        } else {
            (in_l, in_r)
        };

        let out_l = direct_l * self.direct_sub_gain + harm_filtered_l;
        let out_r = direct_r * self.direct_sub_gain + harm_filtered_r;

        (out_l, out_r)
    }

    pub fn reset(&mut self) {
        self.sub_lpf_l.reset();
        self.sub_lpf_r.reset();
        self.sub_hpf_l.reset();
        self.sub_hpf_r.reset();
        self.harm_bpf_l.reset();
        self.harm_bpf_r.reset();
        self.protect_hpf_l.reset();
        self.protect_hpf_r.reset();
        self.envelope_l = 0.0;
        self.envelope_r = 0.0;
    }
}
