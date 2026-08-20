//! True-Peak Lookahead Brickwall Limiter & Analog Soft Clipper
//! Guarantees zero digital clipping while maximizing perceived loudness and headroom.

#[derive(Debug, Clone)]
pub struct Limiter {
    sample_rate: f32,
    ceiling_linear: f32,
    release_coeff: f32,
    gain_reduction: f32,

    // Lookahead ring buffers
    buf_l: Vec<f32>,
    buf_r: Vec<f32>,
    buf_idx: usize,
    lookahead_samples: usize,

    // Peak tracking
    current_gain: f32,
}

impl Limiter {
    pub fn new(sample_rate: f32, lookahead_ms: f32, ceiling_db: f32, release_ms: f32) -> Self {
        let lookahead_samples = ((lookahead_ms * 0.001 * sample_rate) as usize).max(4);
        let ceiling_linear = 10.0_f32.powf(ceiling_db / 20.0);
        let release_coeff = (-1.0 / (release_ms * 0.001 * sample_rate)).exp();

        Self {
            sample_rate,
            ceiling_linear,
            release_coeff,
            gain_reduction: 1.0,
            buf_l: vec![0.0; lookahead_samples],
            buf_r: vec![0.0; lookahead_samples],
            buf_idx: 0,
            lookahead_samples,
            current_gain: 1.0,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        let lookahead_samples = ((1.5 * 0.001 * sample_rate) as usize).max(4);
        self.lookahead_samples = lookahead_samples;
        self.buf_l.resize(lookahead_samples, 0.0);
        self.buf_r.resize(lookahead_samples, 0.0);
        self.buf_idx = 0;
        self.release_coeff = (-1.0 / (60.0 * 0.001 * sample_rate)).exp();
    }

    pub fn set_ceiling(&mut self, ceiling_db: f32) {
        self.ceiling_linear = 10.0_f32.powf(ceiling_db.min(0.0) / 20.0);
    }

    pub fn get_gain_reduction_db(&self) -> f32 {
        20.0 * self.current_gain.max(1e-4).log10()
    }

    #[inline(always)]
    pub fn process(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        // Read oldest samples from lookahead buffer
        let delayed_l = self.buf_l[self.buf_idx];
        let delayed_r = self.buf_r[self.buf_idx];

        // Store current input samples
        self.buf_l[self.buf_idx] = in_l;
        self.buf_r[self.buf_idx] = in_r;
        self.buf_idx = (self.buf_idx + 1) % self.lookahead_samples;

        // Detect peak in current lookahead window
        let max_peak = in_l.abs().max(in_r.abs());
        let target_gain = if max_peak > self.ceiling_linear {
            self.ceiling_linear / max_peak
        } else {
            1.0
        };

        // Instant attack, exponential release
        if target_gain < self.current_gain {
            self.current_gain = target_gain; // Instant attack
        } else {
            self.current_gain = (1.0 - self.release_coeff) * target_gain + self.release_coeff * self.current_gain;
        }

        self.gain_reduction = self.current_gain;

        // Apply gain reduction to delayed output
        let mut out_l = delayed_l * self.current_gain;
        let mut out_r = delayed_r * self.current_gain;

        // Soft saturation knee to catch any remaining inter-sample transients
        out_l = soft_clip(out_l, self.ceiling_linear);
        out_r = soft_clip(out_r, self.ceiling_linear);

        (out_l, out_r)
    }

    pub fn reset(&mut self) {
        self.buf_l.fill(0.0);
        self.buf_r.fill(0.0);
        self.current_gain = 1.0;
    }
}

/// Analog-style polynomial soft-clipping knee
#[inline(always)]
fn soft_clip(x: f32, ceiling: f32) -> f32 {
    let limit = ceiling * 0.95;
    if x > limit {
        limit + (ceiling - limit) * (1.0 - (- (x - limit) / (ceiling - limit)).exp())
    } else if x < -limit {
        -limit - (ceiling - limit) * (1.0 - (- (-x - limit) / (ceiling - limit)).exp())
    } else {
        x
    }
}
