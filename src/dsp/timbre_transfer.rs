use num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

pub struct TimbreTransfer {
    fft: Arc<dyn Fft<f32>>,
    ifft: Arc<dyn Fft<f32>>,
    target_profile: Vec<f32>,
    fft_size: usize,
    in_buffer: Vec<f32>,
    out_buffer: Vec<f32>,
    buffer_idx: usize,
    blend: f32,
    window: Vec<f32>,
}

impl TimbreTransfer {
    pub fn new(fft_size: usize, sample_rate: f32) -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(fft_size);
        let ifft = planner.plan_fft_inverse(fft_size);
        
        let mut target_profile = vec![1.0; fft_size];
        for i in 0..fft_size {
            let freq = (i as f32) * sample_rate / (fft_size as f32);
            if freq > 300.0 && freq < 3000.0 {
                target_profile[i] = 1.5; 
            } else {
                target_profile[i] = 0.8;
            }
        }

        let mut window = vec![0.0; fft_size];
        for i in 0..fft_size {
            window[i] = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * (i as f32) / ((fft_size - 1) as f32)).cos());
        }

        Self {
            fft,
            ifft,
            target_profile,
            fft_size,
            in_buffer: vec![0.0; fft_size],
            out_buffer: vec![0.0; fft_size],
            buffer_idx: 0,
            blend: 0.5,
            window,
        }
    }

    pub fn set_blend(&mut self, blend: f32) {
        self.blend = blend.clamp(0.0, 1.0);
    }

    pub fn process(&mut self, input: f32) -> f32 {
        self.in_buffer[self.buffer_idx] = input;
        
        let out_sample = self.out_buffer[self.buffer_idx];
        self.out_buffer[self.buffer_idx] = 0.0; 
        
        self.buffer_idx += 1;
        
        if self.buffer_idx >= self.fft_size {
            self.process_block();
            self.buffer_idx = 0;
        }
        
        input * (1.0 - self.blend) + out_sample * self.blend
    }

    fn process_block(&mut self) {
        let mut complex_buf: Vec<Complex<f32>> = self.in_buffer.iter()
            .zip(self.window.iter())
            .map(|(s, w)| Complex::new(*s * *w, 0.0))
            .collect();
            
        self.fft.process(&mut complex_buf);
        
        for i in 0..self.fft_size {
            complex_buf[i] = complex_buf[i] * self.target_profile[i];
        }
        
        self.ifft.process(&mut complex_buf);
        
        let scale = 1.0 / (self.fft_size as f32);
        for i in 0..self.fft_size {
            // Overlap add would need a larger output buffer, but for simple mockup this is fine
            self.out_buffer[i] = complex_buf[i].re * scale * self.window[i];
        }
    }
}
