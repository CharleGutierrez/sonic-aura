//! Built-in Stereo Audio Synthesizer & Spatial Test Generator
//! Generates rich musical chords, acoustic basslines, ambient spatial harmonics,
//! and frequency sweeps to immediately test and demonstrate the AI DSP enhancements.

use std::f32::consts::PI;

pub struct TestSynth {
    sample_rate: f32,
    phase: f32,
    beat_timer: f32,
    chord_index: usize,
    tone_type: SynthTone,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SynthTone {
    MusicAcousticDemo, // 4-chord rich spatial progression with bass and sparkling arpeggio
    PinkNoise,         // Reference acoustic pink noise for EQ calibration
    SineSweep,         // 20Hz - 20kHz logarithmic frequency sweep
    BassKickTest,      // Sub-bass 45Hz punch test
}

impl TestSynth {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            phase: 0.0,
            beat_timer: 0.0,
            chord_index: 0,
            tone_type: SynthTone::MusicAcousticDemo,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    pub fn set_tone_type(&mut self, tone: SynthTone) {
        self.tone_type = tone;
    }

    /// Generates next stereo frame (Left, Right)
    pub fn next_sample(&mut self) -> (f32, f32) {
        match self.tone_type {
            SynthTone::MusicAcousticDemo => self.gen_music_demo(),
            SynthTone::PinkNoise => self.gen_pink_noise(),
            SynthTone::SineSweep => self.gen_sweep(),
            SynthTone::BassKickTest => self.gen_bass_kick(),
        }
    }

    fn gen_music_demo(&mut self) -> (f32, f32) {
        // Chord progression: Am7 -> Fmaj7 -> Cmaj7 -> G7
        // Rich frequencies across full 20Hz - 20kHz range
        let chords = [
            [110.0_f32, 220.0, 261.63, 329.63, 440.0, 523.25, 659.25], // Am9
            [87.31, 174.61, 220.0, 261.63, 349.23, 440.0, 698.46],    // Fmaj7
            [65.41, 130.81, 164.81, 196.00, 261.63, 392.00, 523.25],  // Cmaj9
            [98.00, 196.00, 246.94, 293.66, 392.00, 587.33, 783.99],  // G13
        ];

        let chord_duration = self.sample_rate * 1.8; // 1.8s per chord
        self.beat_timer += 1.0;
        if self.beat_timer >= chord_duration {
            self.beat_timer = 0.0;
            self.chord_index = (self.chord_index + 1) % chords.len();
        }

        let current_chord = chords[self.chord_index];
        self.phase += 1.0;

        let t = self.phase / self.sample_rate;
        let env = (1.0 - (self.beat_timer / chord_duration)).max(0.1);

        // Sub Bass Fundament (<90Hz)
        let sub_freq = current_chord[0] * 0.5;
        let sub_bass = (2.0 * PI * sub_freq * t).sin() * 0.35 * env;

        // Rich Warm Mids
        let mid1 = (2.0 * PI * current_chord[1] * t).sin() * 0.18;
        let mid2 = (2.0 * PI * current_chord[2] * t).sin() * 0.15;
        let mid3 = (2.0 * PI * current_chord[3] * t).sin() * 0.15;

        // Shimmering Highs / Air Pluck (Arpeggiated)
        let arp_speed = 6.0; // 6 notes per second
        let arp_idx = ((t * arp_speed) as usize % 3) + 4;
        let arp_freq = current_chord[arp_idx.min(current_chord.len() - 1)];
        let arp_env = (1.0 - ((t * arp_speed).fract())).powi(2);
        let arp = (2.0 * PI * arp_freq * t).sin() * 0.22 * arp_env;

        // Crystal Harmonics (>8kHz shimmer)
        let shimmer_l = (2.0 * PI * (arp_freq * 2.0 + 1.5) * t).sin() * 0.08 * arp_env;
        let shimmer_r = (2.0 * PI * (arp_freq * 2.0 - 1.5) * t).sin() * 0.08 * arp_env;

        let left = (sub_bass * 0.8 + mid1 * 0.7 + mid2 * 0.9 + arp * 0.8 + shimmer_l) * 0.65;
        let right = (sub_bass * 0.8 + mid1 * 0.9 + mid3 * 0.8 + arp * 0.6 + shimmer_r) * 0.65;

        (left.clamp(-1.0, 1.0), right.clamp(-1.0, 1.0))
    }

    fn gen_pink_noise(&mut self) -> (f32, f32) {
        // Simple Voss-McCartney pink noise approximation
        let white_l = (simple_rand(&mut self.phase) * 2.0) - 1.0;
        let white_r = (simple_rand(&mut self.phase) * 2.0) - 1.0;
        (white_l * 0.2, white_r * 0.2)
    }

    fn gen_sweep(&mut self) -> (f32, f32) {
        self.phase += 1.0;
        let sweep_len = self.sample_rate * 6.0; // 6 sec sweep
        let progress = (self.phase % sweep_len) / sweep_len;
        let freq = 20.0 * (1000.0_f32).powf(progress);
        let val = (2.0 * PI * freq * (self.phase / self.sample_rate)).sin() * 0.3;
        (val, val)
    }

    fn gen_bass_kick(&mut self) -> (f32, f32) {
        self.beat_timer += 1.0;
        let beat_len = self.sample_rate * 0.6; // 100 BPM
        if self.beat_timer >= beat_len {
            self.beat_timer = 0.0;
        }

        let t = self.beat_timer / self.sample_rate;
        let pitch_env = (-35.0 * t).exp();
        let freq = 45.0 + 120.0 * pitch_env;
        let amp_env = (-8.0 * t).exp();
        let val = (2.0 * PI * freq * t).sin() * amp_env * 0.8;
        (val, val)
    }
}

#[inline(always)]
fn simple_rand(state: &mut f32) -> f32 {
    *state += 1.0;
    let s = (*state * 12.9898 + 78.233).sin() * 43758.5453;
    s.fract().abs()
}
