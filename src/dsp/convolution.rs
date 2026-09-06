use num_complex::Complex;
use realfft::RealFftPlanner;
use std::sync::Arc;

pub struct ConvolutionReverb {
    enabled: bool,
    planner: RealFftPlanner<f32>,
    fft: Arc<dyn realfft::RealToComplex<f32>>,
    ifft: Arc<dyn realfft::ComplexToReal<f32>>,
    ir_freq: Vec<Complex<f32>>,
    in_buffer: Vec<f32>,
    out_buffer: Vec<f32>,
    block_size: usize,
    pos: usize,
}

impl ConvolutionReverb {
    pub fn new(block_size: usize) -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(block_size * 2);
        let ifft = planner.plan_fft_inverse(block_size * 2);

        Self {
            enabled: false,
            planner,
            fft,
            ifft,
            ir_freq: vec![Complex::new(0.0, 0.0); block_size + 1], // Default empty IR
            in_buffer: vec![0.0; block_size],
            out_buffer: vec![0.0; block_size * 2],
            block_size,
            pos: 0,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn load_ir(&mut self, ir: &[f32]) {
        let mut padded_ir = vec![0.0; self.block_size * 2];
        let copy_len = ir.len().min(self.block_size);
        padded_ir[..copy_len].copy_from_slice(&ir[..copy_len]);

        let mut ir_freq = vec![Complex::new(0.0, 0.0); self.block_size + 1];
        let _ = self.fft.process(&mut padded_ir, &mut ir_freq);
        self.ir_freq = ir_freq;
    }

    pub fn process_mono(&mut self, input: f32) -> f32 {
        if !self.enabled {
            return input;
        }

        self.in_buffer[self.pos] = input;

        let out_sample = self.out_buffer[self.pos];
        self.out_buffer[self.pos] = 0.0; // Clear after reading

        self.pos += 1;

        if self.pos >= self.block_size {
            let mut padded_in = vec![0.0; self.block_size * 2];
            padded_in[..self.block_size].copy_from_slice(&self.in_buffer);

            let mut in_freq = vec![Complex::new(0.0, 0.0); self.block_size + 1];
            let _ = self.fft.process(&mut padded_in, &mut in_freq);

            // Multiply in frequency domain
            let mut out_freq = vec![Complex::new(0.0, 0.0); self.block_size + 1];
            for i in 0..in_freq.len() {
                out_freq[i] = in_freq[i] * self.ir_freq[i];
            }

            let mut block_out = vec![0.0; self.block_size * 2];
            let _ = self.ifft.process(&mut out_freq, &mut block_out);

            // Overlap-add
            let norm = 1.0 / (self.block_size * 2) as f32;
            for i in 0..(self.block_size * 2) {
                if i < self.block_size {
                    self.out_buffer[i] += block_out[i] * norm;
                } else {
                    self.out_buffer[i] = block_out[i] * norm; // this resets the tail for next block
                }
            }

            self.pos = 0;
            self.in_buffer.fill(0.0);
        }

        out_sample
    }

    pub fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        if !self.enabled {
            return (l, r);
        }
        // Very basic mono-based convolution for demonstration
        let mixed = (l + r) * 0.5;
        let conv = self.process_mono(mixed);
        (l * 0.5 + conv * 0.5, r * 0.5 + conv * 0.5)
    }
}
