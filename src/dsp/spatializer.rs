//! Dolby Atmos & DTS-grade 3D Spatializer and Soundstage Widener
//! Features:
//! - Frequency-dependent Mid/Side Stereophonic Expander
//! - Binaural HRTF Crossfeed with Interaural Time Delay (ITD) & Head-Shadow Filter
//! - Multi-Tap Early Reflection Ambience Decorrelator for 3D Depth and Immersion
//! - Speaker Cross-Talk Cancellation (Stereo Expansion for Laptop Speakers)

use crate::dsp::biquad::{Biquad, FilterType};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpatialMode {
    HeadphonesBinaural, // Dolby Atmos / Apple Spatial HRTF crossfeed + 3D room
    LaptopSpeakers,     // Acoustic lens stereo widener with cross-talk expansion
    StudioNearfield,    // Clean mastering-grade M/S soundstage
}

#[derive(Debug, Clone)]
pub struct Spatializer {
    sample_rate: f32,
    mode: SpatialMode,
    width: f32,         // Stereo width (0.0 = Mono, 1.0 = Normal, 1.8 = Super-wide)
    depth: f32,         // 3D Depth / early reflection ambience mix (0.0 to 1.0)
    crossfeed: f32,     // HRTF crossfeed amount (0.0 = off, 1.0 = full natural acoustic)

    // Mid/Side frequency filters: keep sub-bass mono, widen mids/highs
    side_hpf: Biquad,
    side_shelf: Biquad,

    // HRTF Crossfeed filters (Head shadow simulation)
    cross_lpf_l: Biquad,
    cross_lpf_r: Biquad,

    // Delay lines for HRTF ITD (Interaural Time Difference ~0.28ms)
    delay_cross_l: Vec<f32>,
    delay_cross_r: Vec<f32>,
    cross_idx: usize,

    // 4-Tap Ambience Decorrelator delay buffers for 3D Room soundstage
    ambience_buf_l: Vec<f32>,
    ambience_buf_r: Vec<f32>,
    ambience_idx_l: usize,
    ambience_idx_r: usize,

    // Allpass filters for phase diffusion
    allpass_l: Biquad,
    allpass_r: Biquad,
}

impl Spatializer {
    pub fn new(sample_rate: f32) -> Self {
        let max_cross_delay = (sample_rate * 0.001) as usize + 8; // 1ms max
        let max_amb_delay = (sample_rate * 0.060) as usize + 8;  // 60ms max

        let mut instance = Self {
            sample_rate,
            mode: SpatialMode::HeadphonesBinaural,
            width: 1.25,
            depth: 0.35,
            crossfeed: 0.45,

            side_hpf: Biquad::new(FilterType::HighPass, 140.0, 0.707, 0.0, sample_rate),
            side_shelf: Biquad::new(FilterType::HighShelf, 2500.0, 0.707, 2.0, sample_rate),

            cross_lpf_l: Biquad::new(FilterType::LowShelf, 700.0, 0.707, -4.5, sample_rate),
            cross_lpf_r: Biquad::new(FilterType::LowShelf, 700.0, 0.707, -4.5, sample_rate),

            delay_cross_l: vec![0.0; max_cross_delay],
            delay_cross_r: vec![0.0; max_cross_delay],
            cross_idx: 0,

            ambience_buf_l: vec![0.0; max_amb_delay],
            ambience_buf_r: vec![0.0; max_amb_delay],
            ambience_idx_l: 0,
            ambience_idx_r: 0,

            allpass_l: Biquad::new(FilterType::AllPass, 1200.0, 1.4, 0.0, sample_rate),
            allpass_r: Biquad::new(FilterType::AllPass, 1800.0, 1.4, 0.0, sample_rate),
        };
        instance.update_filters();
        instance
    }

    pub fn set_mode(&mut self, mode: SpatialMode) {
        self.mode = mode;
    }

    pub fn get_mode(&self) -> SpatialMode {
        self.mode
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        let max_cross_delay = (sample_rate * 0.001) as usize + 8;
        let max_amb_delay = (sample_rate * 0.060) as usize + 8;
        self.delay_cross_l.resize(max_cross_delay, 0.0);
        self.delay_cross_r.resize(max_cross_delay, 0.0);
        self.ambience_buf_l.resize(max_amb_delay, 0.0);
        self.ambience_buf_r.resize(max_amb_delay, 0.0);
        self.update_filters();
    }

    pub fn set_width(&mut self, width: f32) {
        self.width = width.clamp(0.0, 2.5);
    }

    pub fn set_depth(&mut self, depth: f32) {
        self.depth = depth.clamp(0.0, 1.0);
    }

    pub fn set_crossfeed(&mut self, crossfeed: f32) {
        self.crossfeed = crossfeed.clamp(0.0, 1.0);
    }

    fn update_filters(&mut self) {
        self.side_hpf = Biquad::new(FilterType::HighPass, 140.0, 0.707, 0.0, self.sample_rate);
        self.side_shelf = Biquad::new(FilterType::HighShelf, 2500.0, 0.707, 2.0, self.sample_rate);
        self.cross_lpf_l = Biquad::new(FilterType::LowShelf, 700.0, 0.707, -4.5, self.sample_rate);
        self.cross_lpf_r = Biquad::new(FilterType::LowShelf, 700.0, 0.707, -4.5, self.sample_rate);
        self.allpass_l = Biquad::new(FilterType::AllPass, 1200.0, 1.4, 0.0, self.sample_rate);
        self.allpass_r = Biquad::new(FilterType::AllPass, 1800.0, 1.4, 0.0, self.sample_rate);
    }

    #[inline(always)]
    pub fn process(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        // 1. Mid / Side Encoding
        let mid = (in_l + in_r) * 0.70710678;
        let side = (in_l - in_r) * 0.70710678;

        // Apply frequency-dependent stereo widening (tight bass mono center, wide stereo highs)
        let filtered_side = self.side_shelf.process(self.side_hpf.process(side));
        let wide_side = (side * 0.35 + filtered_side * 0.65) * self.width;

        // Mid / Side Decoding
        let mut out_l = (mid + wide_side) * 0.70710678;
        let mut out_r = (mid - wide_side) * 0.70710678;

        match self.mode {
            SpatialMode::HeadphonesBinaural => {
                // Binaural HRTF Crossfeed simulation
                if self.crossfeed > 0.01 {
                    let delay_samples = (self.sample_rate * 0.00028) as usize; // ~0.28ms ITD

                    // Write current samples to crossfeed buffers
                    self.delay_cross_l[self.cross_idx] = out_l;
                    self.delay_cross_r[self.cross_idx] = out_r;

                    let read_idx = if self.cross_idx >= delay_samples {
                        self.cross_idx - delay_samples
                    } else {
                        self.delay_cross_l.len() + self.cross_idx - delay_samples
                    };

                    let delayed_l = self.delay_cross_l[read_idx];
                    let delayed_r = self.delay_cross_r[read_idx];

                    self.cross_idx = (self.cross_idx + 1) % self.delay_cross_l.len();

                    // Filter delayed opposite channels (Head-shadow attenuation)
                    let shadow_l = self.cross_lpf_l.process(delayed_r) * (self.crossfeed * 0.35);
                    let shadow_r = self.cross_lpf_r.process(delayed_l) * (self.crossfeed * 0.35);

                    out_l += shadow_l;
                    out_r += shadow_r;
                }
            }
            SpatialMode::LaptopSpeakers => {
                // Speaker Cross-talk cancellation: inverted delayed cross-bleed expands sound beyond the physical laptop body
                let delay_spk = (self.sample_rate * 0.00022) as usize; // 220 microseconds
                self.delay_cross_l[self.cross_idx] = out_l;
                self.delay_cross_r[self.cross_idx] = out_r;

                let read_idx = if self.cross_idx >= delay_spk {
                    self.cross_idx - delay_spk
                } else {
                    self.delay_cross_l.len() + self.cross_idx - delay_spk
                };

                let del_l = self.delay_cross_l[read_idx];
                let del_r = self.delay_cross_r[read_idx];
                self.cross_idx = (self.cross_idx + 1) % self.delay_cross_l.len();

                let spk_expansion = 0.28 * self.width;
                out_l -= del_r * spk_expansion;
                out_r -= del_l * spk_expansion;
            }
            SpatialMode::StudioNearfield => {
                // Pure clean stereo imaging
            }
        }

        // 2. Virtual 3D Depth & Early Reflection Ambience (Cinema / Studio soundstage)
        if self.depth > 0.01 {
            let buf_len = self.ambience_buf_l.len();
            self.ambience_buf_l[self.ambience_idx_l] = out_l;
            self.ambience_buf_r[self.ambience_idx_r] = out_r;

            // Tap 1: 7ms, Tap 2: 13ms, Tap 3: 19ms, Tap 4: 29ms (prime delays)
            let tap1 = ((self.sample_rate * 0.007) as usize).min(buf_len - 1);
            let tap2 = ((self.sample_rate * 0.013) as usize).min(buf_len - 1);
            let tap3 = ((self.sample_rate * 0.019) as usize).min(buf_len - 1);
            let tap4 = ((self.sample_rate * 0.029) as usize).min(buf_len - 1);

            let r1_l = self.read_amb_l(tap1);
            let r2_r = self.read_amb_r(tap2);
            let r3_l = self.read_amb_l(tap3);
            let r4_r = self.read_amb_r(tap4);

            let early_l = self.allpass_l.process(r1_l * 0.4 + r3_l * 0.25 - r2_r * 0.15);
            let early_r = self.allpass_r.process(r2_r * 0.4 + r4_r * 0.25 - r1_l * 0.15);

            self.ambience_idx_l = (self.ambience_idx_l + 1) % buf_len;
            self.ambience_idx_r = (self.ambience_idx_r + 1) % buf_len;

            out_l += early_l * (self.depth * 0.32);
            out_r += early_r * (self.depth * 0.32);
        }

        (out_l, out_r)
    }

    #[inline(always)]
    fn read_amb_l(&self, delay: usize) -> f32 {
        let len = self.ambience_buf_l.len();
        let idx = if self.ambience_idx_l >= delay {
            self.ambience_idx_l - delay
        } else {
            len + self.ambience_idx_l - delay
        };
        self.ambience_buf_l[idx]
    }

    #[inline(always)]
    fn read_amb_r(&self, delay: usize) -> f32 {
        let len = self.ambience_buf_r.len();
        let idx = if self.ambience_idx_r >= delay {
            self.ambience_idx_r - delay
        } else {
            len + self.ambience_idx_r - delay
        };
        self.ambience_buf_r[idx]
    }

    pub fn reset(&mut self) {
        self.side_hpf.reset();
        self.side_shelf.reset();
        self.cross_lpf_l.reset();
        self.cross_lpf_r.reset();
        self.allpass_l.reset();
        self.allpass_r.reset();
        self.delay_cross_l.fill(0.0);
        self.delay_cross_r.fill(0.0);
        self.ambience_buf_l.fill(0.0);
        self.ambience_buf_r.fill(0.0);
    }
}
