pub struct StemMixer {
    pub stems: [(f32, f32); 4], // 4 stereo stems
    pub gains: [f32; 4],
}

impl StemMixer {
    pub fn new() -> Self {
        Self {
            stems: [(0.0, 0.0); 4],
            gains: [1.0, 1.0, 1.0, 1.0], // Vocals, Drums, Bass, Other
        }
    }

    pub fn set_stem(&mut self, index: usize, left: f32, right: f32) {
        if index < 4 {
            self.stems[index] = (left, right);
        }
    }

    pub fn set_gain(&mut self, index: usize, gain: f32) {
        if index < 4 {
            self.gains[index] = gain;
        }
    }

    pub fn mix(&self) -> (f32, f32) {
        let mut out_l = 0.0;
        let mut out_r = 0.0;
        for i in 0..4 {
            out_l += self.stems[i].0 * self.gains[i];
            out_r += self.stems[i].1 * self.gains[i];
        }
        (out_l, out_r)
    }
}
