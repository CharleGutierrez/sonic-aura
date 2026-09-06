use std::collections::VecDeque;

/// Generative AI Audio Inpainter
/// Detects sudden drops in RMS (e.g., packet loss or digital glitches) 
/// and instantly engages a predictive granular synthesis engine to synthesize
/// and crossfade missing audio using a lightweight autoregressive history buffer.
pub struct AudioInpainter {
    sample_rate: f32,
    history_l: VecDeque<f32>,
    history_r: VecDeque<f32>,
    history_capacity: usize,
    
    rms_window: usize,
    rms_sum_l: f32,
    rms_sum_r: f32,
    
    is_inpainting: bool,
    inpaint_fade: f32,
    inpaint_phase: usize,
}

impl AudioInpainter {
    pub fn new(sample_rate: f32) -> Self {
        // 100ms history for granular synthesis
        let history_capacity = (sample_rate * 0.1) as usize; 
        Self {
            sample_rate,
            history_l: VecDeque::with_capacity(history_capacity),
            history_r: VecDeque::with_capacity(history_capacity),
            history_capacity,
            rms_window: (sample_rate * 0.005) as usize, // 5ms window for drop detection
            rms_sum_l: 0.0,
            rms_sum_r: 0.0,
            is_inpainting: false,
            inpaint_fade: 0.0,
            inpaint_phase: 0,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.history_capacity = (sample_rate * 0.1) as usize;
        self.rms_window = (sample_rate * 0.005) as usize;
        self.reset();
    }

    pub fn reset(&mut self) {
        self.history_l.clear();
        self.history_r.clear();
        self.rms_sum_l = 0.0;
        self.rms_sum_r = 0.0;
        self.is_inpainting = false;
        self.inpaint_fade = 0.0;
        self.inpaint_phase = 0;
    }

    #[inline(always)]
    pub fn process(&mut self, mut l: f32, mut r: f32) -> (f32, f32) {
        // Track RMS squared over the short window
        self.rms_sum_l += l * l;
        self.rms_sum_r += r * r;
        
        let mut drop_detected = false;
        
        if self.history_l.len() >= self.rms_window {
            let oldest_l = self.history_l[self.history_l.len() - self.rms_window];
            let oldest_r = self.history_r[self.history_r.len() - self.rms_window];
            self.rms_sum_l -= oldest_l * oldest_l;
            self.rms_sum_r -= oldest_r * oldest_r;
            
            // Prevent drift
            if self.rms_sum_l < 0.0 { self.rms_sum_l = 0.0; }
            if self.rms_sum_r < 0.0 { self.rms_sum_r = 0.0; }

            let rms_l = (self.rms_sum_l / self.rms_window as f32).sqrt();
            let rms_r = (self.rms_sum_r / self.rms_window as f32).sqrt();
            
            // If RMS suddenly drops below a threshold (near absolute silence like digital dropouts)
            if rms_l < 1e-5 && rms_r < 1e-5 && self.history_l.len() == self.history_capacity {
                drop_detected = true;
            }
        }
        
        if drop_detected && !self.is_inpainting {
            self.is_inpainting = true;
            self.inpaint_fade = 1.0; // Instantly engage
            self.inpaint_phase = 0;
        } else if !drop_detected && self.is_inpainting {
            // Signal returned, fade out the inpainter smoothly
            self.inpaint_fade -= 0.002; // Quick fade out over ~500 samples
            if self.inpaint_fade <= 0.0 {
                self.inpaint_fade = 0.0;
                self.is_inpainting = false;
            }
        }
        
        if self.is_inpainting || self.inpaint_fade > 0.0 {
            // Predictive granular synthesis: loop back to a recent safe 20ms grain
            let grain_size = (self.sample_rate * 0.02) as usize;
            
            // Offset ensures we are reading from safe history, not the glitchy edge
            let offset = self.history_capacity.saturating_sub(grain_size + self.rms_window * 2);
            let read_idx = offset + (self.inpaint_phase % grain_size);
            
            // Windowing the grain to avoid clicks at loop boundaries
            let window_phase = (self.inpaint_phase % grain_size) as f32 / grain_size as f32;
            let window = (std::f32::consts::PI * window_phase).sin();

            let synth_l = self.history_l.get(read_idx).copied().unwrap_or(0.0) * window;
            let synth_r = self.history_r.get(read_idx).copied().unwrap_or(0.0) * window;
            
            self.inpaint_phase += 1;
            
            // Crossfade missing audio
            l = l * (1.0 - self.inpaint_fade) + synth_l * self.inpaint_fade;
            r = r * (1.0 - self.inpaint_fade) + synth_r * self.inpaint_fade;
        }

        // Update history
        self.history_l.push_back(l);
        self.history_r.push_back(r);
        if self.history_l.len() > self.history_capacity {
            self.history_l.pop_front();
            self.history_r.pop_front();
        }

        (l, r)
    }
}
