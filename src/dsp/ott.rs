use crate::dsp::biquad::{Biquad, FilterType};

pub struct MultibandUpwardCompressor {
    sample_rate: f32,
    lp_l: Biquad,
    lp_r: Biquad,
    hp_l: Biquad,
    hp_r: Biquad,
    intensity: f32,
    env_low_l: f32,
    env_low_r: f32,
    env_mid_l: f32,
    env_mid_r: f32,
    env_high_l: f32,
    env_high_r: f32,
}

impl MultibandUpwardCompressor {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            lp_l: Biquad::new(FilterType::LowPass, 150.0, 0.707, 0.0, sample_rate),
            lp_r: Biquad::new(FilterType::LowPass, 150.0, 0.707, 0.0, sample_rate),
            hp_l: Biquad::new(FilterType::HighPass, 2500.0, 0.707, 0.0, sample_rate),
            hp_r: Biquad::new(FilterType::HighPass, 2500.0, 0.707, 0.0, sample_rate),
            intensity: 0.5,
            env_low_l: 0.0,
            env_low_r: 0.0,
            env_mid_l: 0.0,
            env_mid_r: 0.0,
            env_high_l: 0.0,
            env_high_r: 0.0,
        }
    }

    pub fn set_intensity(&mut self, intensity: f32) {
        self.intensity = intensity.clamp(0.0, 1.0);
    }

    fn compress_band(sample: f32, env: &mut f32, target_level: f32, ratio: f32, attack: f32, release: f32) -> f32 {
        let abs_s = sample.abs();
        if abs_s > *env {
            *env += attack * (abs_s - *env);
        } else {
            *env += release * (abs_s - *env);
        }
        
        let env_db = 20.0 * (*env + 1e-7).log10();
        let target_db = 20.0 * (target_level + 1e-7).log10();
        
        let gain_db = if env_db < target_db {
            // Upward compression: pull up quiet signals
            (target_db - env_db) * (1.0 - ratio)
        } else {
            // Downward compression: pull down loud signals
            (target_db - env_db) * (1.0 - 1.0 / ratio)
        };
        
        let gain = 10.0_f32.powf(gain_db / 20.0);
        sample * gain
    }

    pub fn process(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        let low_l = self.lp_l.process(in_l);
        let low_r = self.lp_r.process(in_r);
        
        let high_l = self.hp_l.process(in_l);
        let high_r = self.hp_r.process(in_r);
        
        let mid_l = in_l - low_l - high_l;
        let mid_r = in_r - low_r - high_r;
        
        let attack = 0.01;
        let release = 0.001;
        
        let ratio = 0.5; // ratio < 1.0 for upward compression in our formula
        let target = 0.1;
        
        let c_low_l = Self::compress_band(low_l, &mut self.env_low_l, target, ratio, attack, release);
        let c_low_r = Self::compress_band(low_r, &mut self.env_low_r, target, ratio, attack, release);
        
        let c_mid_l = Self::compress_band(mid_l, &mut self.env_mid_l, target, ratio, attack, release);
        let c_mid_r = Self::compress_band(mid_r, &mut self.env_mid_r, target, ratio, attack, release);
        
        let c_high_l = Self::compress_band(high_l, &mut self.env_high_l, target, ratio, attack, release);
        let c_high_r = Self::compress_band(high_r, &mut self.env_high_r, target, ratio, attack, release);
        
        let out_l = in_l * (1.0 - self.intensity) + (c_low_l + c_mid_l + c_high_l) * self.intensity;
        let out_r = in_r * (1.0 - self.intensity) + (c_low_r + c_mid_r + c_high_r) * self.intensity;
        
        (out_l, out_r)
    }
}
