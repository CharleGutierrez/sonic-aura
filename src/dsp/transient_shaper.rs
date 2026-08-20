//! Intelligent Transient Shaper & Dynamic Attack Punch Enhancer
//! Separates fast dynamic transients (drum hits, vocal consonants, guitar plucks)
//! from sustained resonance to provide instant punch, punchy snap, and crisp definition.

#[derive(Debug, Clone)]
pub struct TransientShaper {
    sample_rate: f32,
    attack: f32,   // Attack boost/cut (-1.0 to +2.0)
    sustain: f32,  // Sustain boost/cut (-1.0 to +1.0)
    crispness: f32,// HF transient emphasis

    // Envelope followers
    fast_env_l: f32,
    fast_env_r: f32,
    slow_env_l: f32,
    slow_env_r: f32,

    // Coefficients
    fast_coeff: f32,
    slow_coeff: f32,
}

impl TransientShaper {
    pub fn new(sample_rate: f32) -> Self {
        let mut instance = Self {
            sample_rate,
            attack: 0.5,     // default punchy +50%
            sustain: 0.0,
            crispness: 0.3,
            fast_env_l: 0.0,
            fast_env_r: 0.0,
            slow_env_l: 0.0,
            slow_env_r: 0.0,
            fast_coeff: 0.0,
            slow_coeff: 0.0,
        };
        instance.recalc_coeffs();
        instance
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.recalc_coeffs();
    }

    pub fn set_attack(&mut self, attack: f32) {
        self.attack = attack.clamp(-1.0, 2.5);
    }

    pub fn set_sustain(&mut self, sustain: f32) {
        self.sustain = sustain.clamp(-1.0, 1.5);
    }

    pub fn set_crispness(&mut self, crispness: f32) {
        self.crispness = crispness.clamp(0.0, 1.0);
    }

    fn recalc_coeffs(&mut self) {
        // Fast envelope: ~2ms time constant
        let fast_time = 0.002;
        self.fast_coeff = (-1.0 / (fast_time * self.sample_rate)).exp();

        // Slow envelope: ~35ms time constant
        let slow_time = 0.035;
        self.slow_coeff = (-1.0 / (slow_time * self.sample_rate)).exp();
    }

    #[inline(always)]
    pub fn process(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        if self.attack.abs() < 0.001 && self.sustain.abs() < 0.001 {
            return (in_l, in_r);
        }

        let abs_l = in_l.abs();
        let abs_r = in_r.abs();

        // Update fast envelopes
        self.fast_env_l = (1.0 - self.fast_coeff) * abs_l + self.fast_coeff * self.fast_env_l;
        self.fast_env_r = (1.0 - self.fast_coeff) * abs_r + self.fast_coeff * self.fast_env_r;

        // Update slow envelopes
        self.slow_env_l = (1.0 - self.slow_coeff) * abs_l + self.slow_coeff * self.slow_env_l;
        self.slow_env_r = (1.0 - self.slow_coeff) * abs_r + self.slow_coeff * self.slow_env_r;

        // Transient energy calculation (Attack vs Sustain)
        let diff_l = self.fast_env_l - self.slow_env_l;
        let diff_r = self.fast_env_r - self.slow_env_r;

        let base_l = self.slow_env_l + 1e-4;
        let base_r = self.slow_env_r + 1e-4;

        // Transient gain multiplier
        let trans_ratio_l = (diff_l / base_l).max(-0.95);
        let trans_ratio_r = (diff_r / base_r).max(-0.95);

        let attack_gain_l = 1.0 + self.attack * trans_ratio_l.max(0.0) + self.sustain * (1.0 - (trans_ratio_l.max(0.0)).min(1.0));
        let attack_gain_r = 1.0 + self.attack * trans_ratio_r.max(0.0) + self.sustain * (1.0 - (trans_ratio_r.max(0.0)).min(1.0));

        let out_l = in_l * attack_gain_l.clamp(0.1, 3.0);
        let out_r = in_r * attack_gain_r.clamp(0.1, 3.0);

        (out_l, out_r)
    }

    pub fn reset(&mut self) {
        self.fast_env_l = 0.0;
        self.fast_env_r = 0.0;
        self.slow_env_l = 0.0;
        self.slow_env_r = 0.0;
    }
}
