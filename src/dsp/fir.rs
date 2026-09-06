use std::f32::consts::PI;

pub struct LinearPhaseFir {
    kernel: Vec<f32>,
    buffer_l: Vec<f32>,
    buffer_r: Vec<f32>,
    idx: usize,
}

impl LinearPhaseFir {
    pub fn new(taps: usize, cutoff: f32, sample_rate: f32) -> Self {
        let mut kernel = vec![0.0; taps];
        let fc = cutoff / sample_rate;
        let m = (taps - 1) as f32 / 2.0;
        
        let mut sum = 0.0;
        for i in 0..taps {
            let n = i as f32 - m;
            if n == 0.0 {
                kernel[i] = 2.0 * PI * fc;
            } else {
                kernel[i] = (2.0 * PI * fc * n).sin() / n;
            }
            // Hamming window
            kernel[i] *= 0.54 - 0.46 * (2.0 * PI * i as f32 / (taps - 1) as f32).cos();
            sum += kernel[i];
        }
        
        // Normalize
        for i in 0..taps {
            kernel[i] /= sum;
        }
        
        Self {
            kernel,
            buffer_l: vec![0.0; taps],
            buffer_r: vec![0.0; taps],
            idx: 0,
        }
    }

    pub fn process(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        self.buffer_l[self.idx] = in_l;
        self.buffer_r[self.idx] = in_r;
        
        let mut out_l = 0.0;
        let mut out_r = 0.0;
        let taps = self.kernel.len();
        
        for i in 0..taps {
            let buf_idx = (self.idx + taps - i) % taps;
            out_l += self.buffer_l[buf_idx] * self.kernel[i];
            out_r += self.buffer_r[buf_idx] * self.kernel[i];
        }
        
        self.idx = (self.idx + 1) % taps;
        
        (out_l, out_r)
    }
}
