use std::f32::consts::PI;

pub struct HrtfPanner {
    yaw: f32,
    pitch: f32,
    roll: f32,
    hrir_buffer_l: Vec<f32>,
    hrir_buffer_r: Vec<f32>,
    delay_l: f32,
    delay_r: f32,
}

impl HrtfPanner {
    pub fn new() -> Self {
        let mut panner = Self {
            yaw: 0.0,
            pitch: 0.0,
            roll: 0.0,
            hrir_buffer_l: vec![0.0; 64],
            hrir_buffer_r: vec![0.0; 64],
            delay_l: 0.0,
            delay_r: 0.0,
        };
        panner.update_hrir();
        panner
    }

    pub fn set_rotation(&mut self, yaw: f32, pitch: f32, roll: f32) {
        self.yaw = yaw;
        self.pitch = pitch;
        self.roll = roll;
        self.update_hrir();
    }

    fn update_hrir(&mut self) {
        // Interpolate synthetic HRIR based on head rotation
        // A real implementation would load spherical harmonics or SOFA files
        // Here we simulate the Interaural Time Difference (ITD) and Interaural Level Difference (ILD)
        
        // ITD approx: Woodworth model (a = head radius ~0.0875m, c = 343m/s)
        let a = 0.0875;
        let c = 343.0;
        let theta = self.yaw.clamp(-PI/2.0, PI/2.0);
        
        let delay_left = (a / c) * (theta + theta.sin());
        let delay_right = (a / c) * (-theta + (-theta).sin());
        
        self.delay_l = if delay_left > 0.0 { delay_left } else { 0.0 };
        self.delay_r = if delay_right > 0.0 { delay_right } else { 0.0 };
        
        // ILD approx using a simple shadow filter
        let shadow = (theta.cos() * 0.5 + 0.5).max(0.1);
        let ipsi_gain = 1.0;
        let contra_gain = shadow;
        
        let (gain_l, gain_r) = if theta > 0.0 {
            (contra_gain, ipsi_gain)
        } else {
            (ipsi_gain, contra_gain)
        };
        
        for i in 0..64 {
            // Very naive synthetic HRIR impulse
            if i == 10 {
                self.hrir_buffer_l[i] = gain_l;
                self.hrir_buffer_r[i] = gain_r;
            } else {
                self.hrir_buffer_l[i] = 0.0;
                self.hrir_buffer_r[i] = 0.0;
            }
        }
    }

    pub fn process(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        // For demonstration, simply applying the synthesized gain and a mock delay
        // using the single tap HRIR at index 10.
        let out_l = in_l * self.hrir_buffer_l[10];
        let out_r = in_r * self.hrir_buffer_r[10];
        (out_l, out_r)
    }
}
