use ebur128::{EbuR128, Mode};

pub struct LufsNormalizer {
    analyzer: EbuR128,
    target_lufs: f32,
    current_gain: f32,
    target_gain: f32,
}

impl LufsNormalizer {
    pub fn new(sample_rate: f32, target_lufs: f32) -> Self {
        let analyzer = EbuR128::new(2, sample_rate as u32, Mode::M | Mode::S).unwrap();
        Self {
            analyzer,
            target_lufs,
            current_gain: 1.0,
            target_gain: 1.0,
        }
    }

    pub fn process_block(&mut self, data_l: &mut [f32], data_r: &mut [f32]) {
        let len = data_l.len();
        let mut interleaved = Vec::with_capacity(len * 2);
        for i in 0..len {
            interleaved.push(data_l[i]);
            interleaved.push(data_r[i]);
        }
        
        self.analyzer.add_frames_f32(&interleaved).unwrap();
        
        let lufs = self.analyzer.loudness_shortterm().unwrap_or(-70.0) as f32;
        let mut diff = self.target_lufs - lufs;
        
        // Don't boost silence
        if lufs < -60.0 {
            diff = 0.0;
        }
        
        self.target_gain = 10.0_f32.powf(diff / 20.0).clamp(0.1, 10.0);
        
        for i in 0..len {
            self.current_gain += (self.target_gain - self.current_gain) * 0.001;
            data_l[i] *= self.current_gain;
            data_r[i] *= self.current_gain;
        }
    }

    pub fn process_sample(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        let interleaved = [in_l, in_r];
        self.analyzer.add_frames_f32(&interleaved).unwrap_or(());
        
        // Only update target every once in a while to save CPU, but for sample by sample we approximate
        // Or we just use a smoothed envelope
        let lufs = self.analyzer.loudness_momentary().unwrap_or(-70.0) as f32;
        
        let mut diff = self.target_lufs - lufs;
        if lufs < -60.0 {
            diff = 0.0;
        }
        
        let new_target = 10.0_f32.powf(diff / 20.0).clamp(0.1, 10.0);
        self.target_gain = new_target;
        self.current_gain += (self.target_gain - self.current_gain) * 0.0001;
        
        (in_l * self.current_gain, in_r * self.current_gain)
    }
}
