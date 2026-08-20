//! AI Adaptive Spectral Analyzer & Intelligent DSP Controller
//! Performs real-time psychoacoustic feature extraction (Spectral Centroid, Flux, Voice Activity Index, Band Energies)
//! and dynamically modulates DSP parameters for optimal crispness, punch, and spatial immersion.

use std::sync::Arc;
use realfft::{RealFftPlanner, RealToComplex};

pub const FFT_SIZE: usize = 512;
pub const NUM_SPECTRUM_BINS: usize = 32;

#[derive(Debug, Clone, Default)]
pub struct AudioFeatures {
    pub rms_db: f32,
    pub peak_db: f32,
    pub spectral_centroid: f32,    // Hz
    pub spectral_flux: f32,
    pub voice_probability: f32,    // 0.0 to 1.0
    pub bass_energy: f32,          // 0.0 to 1.0
    pub mid_energy: f32,           // 0.0 to 1.0
    pub treble_energy: f32,        // 0.0 to 1.0
    pub perceived_loudness_lufs: f32,
}

#[derive(Debug, Clone, Default)]
pub struct AiAdaptiveParameters {
    pub dynamic_eq_vocal_boost_db: f32,
    pub dynamic_eq_bass_tighten_db: f32,
    pub dynamic_exciter_air_mod: f32,
    pub dynamic_spatial_width_mod: f32,
    pub dynamic_bass_intensity_mod: f32,
}

pub struct AiSpectralAnalyzer {
    sample_rate: f32,
    fft_planner: RealFftPlanner<f32>,
    fft: Arc<dyn RealToComplex<f32>>,
    input_window: Vec<f32>,
    window_idx: usize,
    hann_window: Vec<f32>,

    // FFT scratch buffers
    fft_in: Vec<f32>,
    fft_out: Vec<num_complex::Complex<f32>>,
    prev_magnitudes: Vec<f32>,

    // Spectrum visualizer bins (smoothed dB levels for UI)
    pub visualizer_bins: [f32; NUM_SPECTRUM_BINS],
    peak_hold_bins: [f32; NUM_SPECTRUM_BINS],

    // Latest analyzed features & AI adaptive parameters
    pub features: AudioFeatures,
    pub adaptive_params: AiAdaptiveParameters,
    ai_enhancement_amount: f32, // 0.0 (disabled) to 1.0 (full AI boost)
}

impl AiSpectralAnalyzer {
    pub fn new(sample_rate: f32) -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);

        let mut hann_window = vec![0.0; FFT_SIZE];
        for (i, w) in hann_window.iter_mut().enumerate() {
            *w = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (FFT_SIZE as f32 - 1.0)).cos());
        }

        Self {
            sample_rate,
            fft_planner: planner,
            fft,
            input_window: vec![0.0; FFT_SIZE],
            window_idx: 0,
            hann_window,
            fft_in: vec![0.0; FFT_SIZE],
            fft_out: vec![num_complex::Complex::default(); FFT_SIZE / 2 + 1],
            prev_magnitudes: vec![0.0; FFT_SIZE / 2 + 1],
            visualizer_bins: [0.0; NUM_SPECTRUM_BINS],
            peak_hold_bins: [0.0; NUM_SPECTRUM_BINS],
            features: AudioFeatures::default(),
            adaptive_params: AiAdaptiveParameters::default(),
            ai_enhancement_amount: 0.8,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    pub fn set_ai_enhancement_amount(&mut self, amount: f32) {
        self.ai_enhancement_amount = amount.clamp(0.0, 1.5);
    }

    pub fn get_ai_enhancement_amount(&self) -> f32 {
        self.ai_enhancement_amount
    }

    /// Feed stereo audio sample to the AI analyzer
    #[inline]
    pub fn push_sample(&mut self, l: f32, r: f32) {
        let mono = (l + r) * 0.5;
        self.input_window[self.window_idx] = mono;
        self.window_idx += 1;

        if self.window_idx >= FFT_SIZE {
            self.window_idx = 0;
            self.analyze_block();
        }
    }

    fn analyze_block(&mut self) {
        // 1. Calculate RMS & Peak
        let mut sum_sq = 0.0;
        let mut peak = 0.0_f32;
        for &s in &self.input_window {
            let abs_s = s.abs();
            if abs_s > peak {
                peak = abs_s;
            }
            sum_sq += s * s;
        }
        let rms = (sum_sq / FFT_SIZE as f32).sqrt();
        self.features.rms_db = (20.0 * (rms + 1e-5).log10()).clamp(-80.0, 6.0);
        self.features.peak_db = (20.0 * (peak + 1e-5).log10()).clamp(-80.0, 6.0);
        self.features.perceived_loudness_lufs = self.features.rms_db - 3.0;

        // 2. Apply Hann window & compute Forward Real FFT
        for i in 0..FFT_SIZE {
            self.fft_in[i] = self.input_window[i] * self.hann_window[i];
        }

        let _ = self.fft.process(&mut self.fft_in, &mut self.fft_out);

        let num_bins = FFT_SIZE / 2 + 1;
        let bin_hz = (self.sample_rate * 0.5) / (num_bins as f32);

        let mut total_energy = 0.0;
        let mut weighted_freq_sum = 0.0;
        let mut flux_sum = 0.0;

        let mut bass_sum = 0.0;
        let mut mid_sum = 0.0;
        let mut vocal_sum = 0.0;
        let mut treble_sum = 0.0;

        // 3. Extract Spectral Features
        for i in 0..num_bins {
            let mag = self.fft_out[i].norm() / (FFT_SIZE as f32 * 0.5);
            let freq = i as f32 * bin_hz;

            total_energy += mag;
            weighted_freq_sum += freq * mag;

            let prev_mag = self.prev_magnitudes[i];
            let diff = mag - prev_mag;
            if diff > 0.0 {
                flux_sum += diff;
            }
            self.prev_magnitudes[i] = mag;

            if freq < 250.0 {
                bass_sum += mag;
            } else if freq < 1000.0 {
                mid_sum += mag;
            } else if freq < 4500.0 {
                vocal_sum += mag;
                mid_sum += mag;
            } else {
                treble_sum += mag;
            }
        }

        let centroid = if total_energy > 1e-4 {
            weighted_freq_sum / total_energy
        } else {
            1000.0
        };

        self.features.spectral_centroid = centroid;
        self.features.spectral_flux = flux_sum;

        let inv_tot = 1.0 / (total_energy + 1e-4);
        self.features.bass_energy = (bass_sum * inv_tot).clamp(0.0, 1.0);
        self.features.mid_energy = (mid_sum * inv_tot).clamp(0.0, 1.0);
        self.features.treble_energy = (treble_sum * inv_tot).clamp(0.0, 1.0);

        // Vocal Presence Index (formant band ratio + harmonic consistency)
        let vocal_ratio = (vocal_sum * inv_tot) * 2.2;
        let is_voice_range = (centroid > 800.0 && centroid < 3200.0) as i32 as f32;
        self.features.voice_probability = (vocal_ratio * 0.6 + is_voice_range * 0.4).clamp(0.0, 1.0);

        // 4. AI Adaptive Parameter Modulation
        let ai = self.ai_enhancement_amount;

        // Dynamic Dialogue / Vocal Clarity boost
        if self.features.voice_probability > 0.45 {
            self.adaptive_params.dynamic_eq_vocal_boost_db = (self.features.voice_probability * 2.8 * ai).clamp(0.0, 4.0);
        } else {
            self.adaptive_params.dynamic_eq_vocal_boost_db = 0.0;
        }

        // Bass transient tightening (if bass is high energy, tighten bass to keep clarity)
        if self.features.bass_energy > 0.35 {
            self.adaptive_params.dynamic_eq_bass_tighten_db = (self.features.bass_energy * 2.0 * ai).clamp(0.0, 3.0);
            self.adaptive_params.dynamic_bass_intensity_mod = 1.0 + (self.features.bass_energy * 0.4 * ai);
        } else {
            self.adaptive_params.dynamic_eq_bass_tighten_db = 0.0;
            self.adaptive_params.dynamic_bass_intensity_mod = 1.0;
        }

        // Dynamic Air Shimmer modulation
        if self.features.treble_energy < 0.20 && self.features.rms_db > -45.0 {
            // Brighten dark recordings automatically
            self.adaptive_params.dynamic_exciter_air_mod = 1.0 + (0.45 * ai);
        } else {
            self.adaptive_params.dynamic_exciter_air_mod = 1.0;
        }

        // Spatial immersion expansion when expansive sound is detected
        if centroid > 1500.0 && self.features.spectral_flux > 0.1 {
            self.adaptive_params.dynamic_spatial_width_mod = 1.0 + (0.25 * ai);
        } else {
            self.adaptive_params.dynamic_spatial_width_mod = 1.0;
        }

        // 5. Update Visualizer Bins (Logarithmic frequency mapping to 32 visualizer bars)
        self.update_visualizer_bins();
    }

    fn update_visualizer_bins(&mut self) {
        let num_bins = FFT_SIZE / 2 + 1;
        let nyquist = self.sample_rate * 0.5;

        // Logarithmic frequency bands from 20Hz to 20kHz
        let min_freq = 20.0_f32;
        let max_freq = nyquist.min(20000.0);
        let log_min = min_freq.ln();
        let log_max = max_freq.ln();

        for b in 0..NUM_SPECTRUM_BINS {
            let f_low = (log_min + (b as f32 / NUM_SPECTRUM_BINS as f32) * (log_max - log_min)).exp();
            let f_high = (log_min + ((b + 1) as f32 / NUM_SPECTRUM_BINS as f32) * (log_max - log_min)).exp();

            let idx_low = ((f_low / nyquist) * num_bins as f32) as usize;
            let idx_high = (((f_high / nyquist) * num_bins as f32) as usize).max(idx_low + 1).min(num_bins);

            let mut band_energy = 0.0;
            let count = (idx_high - idx_low).max(1);
            for i in idx_low..idx_high {
                band_energy += self.prev_magnitudes[i];
            }
            let avg_energy = band_energy / count as f32;

            // Convert to visualizer normalized scale (0.0 to 1.0)
            let db = 20.0 * (avg_energy + 1e-5).log10();
            let norm = ((db + 65.0) / 65.0).clamp(0.0, 1.0);

            // Smooth decay
            if norm > self.visualizer_bins[b] {
                self.visualizer_bins[b] = norm; // Fast attack
            } else {
                self.visualizer_bins[b] = self.visualizer_bins[b] * 0.78 + norm * 0.22; // Smooth release
            }

            if self.visualizer_bins[b] > self.peak_hold_bins[b] {
                self.peak_hold_bins[b] = self.visualizer_bins[b];
            } else {
                self.peak_hold_bins[b] *= 0.94;
            }
        }
    }
}
