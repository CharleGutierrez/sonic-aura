#[cfg(test)]
mod tests {
    use sonic_aura::dsp::ai_analyzer::AiSpectralAnalyzer;
    use sonic_aura::dsp::biquad::{Biquad, FilterType};
    use sonic_aura::dsp::earphone_profiler::EarphoneType;
    use sonic_aura::dsp::environment_adapter::EnvironmentMode;
    use sonic_aura::dsp::equalizer::Equalizer;
    use sonic_aura::dsp::exciter::HarmonicExciter;
    use sonic_aura::dsp::limiter::Limiter;
    use sonic_aura::dsp::pipeline::AudioPipeline;
    use sonic_aura::dsp::psychoacoustic_bass::PsychoacousticBass;
    use sonic_aura::dsp::spatializer::{SpatialMode, Spatializer};
    use sonic_aura::presets::PresetManager;

    #[test]
    fn test_biquad_stability() {
        let mut biquad = Biquad::new(FilterType::Peaking, 1000.0, 1.414, 6.0, 48000.0);
        let mut max_val: f32 = 0.0;
        for i in 0..1000 {
            let s = (i as f32 * 0.1).sin();
            let out = biquad.process(s);
            assert!(out.is_finite());
            if out.abs() > max_val {
                max_val = out.abs();
            }
        }
        assert!(max_val > 0.0 && max_val < 10.0);
    }

    #[test]
    fn test_declicked_smooth_bypass_crossfade() {
        let mut pipeline = AudioPipeline::new(48000.0);

        // Feed continuous 440Hz tone
        let mut max_step_delta = 0.0_f32;
        let mut prev_sample = 0.0_f32;

        for i in 0..4800 {
            let t = i as f32 / 48000.0;
            let s = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5;

            // Toggle bypass at sample 1000 and sample 2500
            if i == 1000 {
                pipeline.config.enabled = false;
            } else if i == 2500 {
                pipeline.config.enabled = true;
            }

            let (out_l, _) = pipeline.process_stereo_sample(s, s);
            let delta = (out_l - prev_sample).abs();
            if i > 10 && delta > max_step_delta {
                max_step_delta = delta;
            }
            prev_sample = out_l;
        }

        // Verify there is no sudden violent step discontinuity (Dirac click)
        assert!(
            max_step_delta < 0.15,
            "Bypass produced a sudden pop/click step discontinuity: {}",
            max_step_delta
        );
    }

    #[test]
    fn test_psychoacoustic_bass_generation() {
        let mut bass = PsychoacousticBass::new(48000.0);
        bass.set_intensity(1.0);

        let mut harmonic_energy = 0.0;
        for i in 0..2000 {
            let t = i as f32 / 48000.0;
            let s = (2.0 * std::f32::consts::PI * 50.0 * t).sin() * 0.5;
            let (out_l, out_r) = bass.process(s, s);
            assert!(out_l.is_finite());
            assert!(out_r.is_finite());
            harmonic_energy += out_l.abs();
        }
        assert!(harmonic_energy > 10.0);
    }

    #[test]
    fn test_harmonic_exciter_air() {
        let mut exciter = HarmonicExciter::new(48000.0);
        exciter.set_air_mix(0.8);
        exciter.set_drive(0.8);

        let mut out_energy = 0.0;
        for i in 0..1000 {
            let t = i as f32 / 48000.0;
            let s = (2.0 * std::f32::consts::PI * 8000.0 * t).sin() * 0.4;
            let (out_l, out_r) = exciter.process(s, s);
            assert!(out_l.is_finite());
            assert!(out_r.is_finite());
            out_energy += out_l.abs();
        }
        assert!(out_energy > 5.0);
    }

    #[test]
    fn test_spatializer_modes() {
        let mut spat = Spatializer::new(48000.0);
        spat.set_mode(SpatialMode::HeadphonesBinaural);
        spat.set_width(1.5);
        spat.set_depth(0.4);

        let (out_l, out_r) = spat.process(0.5, -0.5);
        assert!(out_l.is_finite() && out_r.is_finite());
        assert_ne!(out_l, 0.0);
    }

    #[test]
    fn test_limiter_brickwall_protection() {
        let mut limiter = Limiter::new(48000.0, 1.5, -0.1, 50.0);
        let ceiling = 10.0_f32.powf(-0.1 / 20.0);

        for i in 0..5000 {
            let t = i as f32 / 48000.0;
            let hot_signal = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 4.0;
            let (out_l, out_r) = limiter.process(hot_signal, hot_signal);
            assert!(
                out_l.abs() <= ceiling + 1e-3,
                "Sample exceeded limiter ceiling: {}",
                out_l
            );
            assert!(
                out_r.abs() <= ceiling + 1e-3,
                "Sample exceeded limiter ceiling: {}",
                out_r
            );
        }
    }

    #[test]
    fn test_ai_analyzer_extraction() {
        let mut analyzer = AiSpectralAnalyzer::new(48000.0);
        analyzer.set_ai_enhancement_amount(1.0);

        for i in 0..1024 {
            let t = i as f32 / 48000.0;
            let s = (2.0 * std::f32::consts::PI * 1000.0 * t).sin() * 0.3;
            analyzer.push_sample(s, s);
        }

        assert!(analyzer.features.spectral_centroid > 0.0);
        assert!(analyzer.features.rms_db > -60.0);
    }

    #[test]
    fn test_equalizer_bands() {
        let mut eq = Equalizer::new_10_band(48000.0);
        eq.set_band_gain(0, 6.0);
        eq.set_band_gain(5, -3.0);

        let (out_l, out_r) = eq.process(0.5, 0.5);
        assert!(out_l.is_finite() && out_r.is_finite());
    }

    #[test]
    fn test_earphone_profiles() {
        for profile in &EarphoneType::ALL {
            assert!(!profile.name().is_empty());
            assert!(!profile.description().is_empty());
            let offsets = profile.eq_offsets();
            assert_eq!(offsets.len(), 10);
            for &g in &offsets {
                assert!(g >= -15.0 && g <= 15.0);
            }
        }
    }

    #[test]
    fn test_environment_modes() {
        for env in &EnvironmentMode::ALL {
            assert!(!env.name().is_empty());
            assert!(!env.description().is_empty());
            let offsets = env.eq_offsets();
            assert_eq!(offsets.len(), 10);
            for &g in &offsets {
                assert!(g >= -15.0 && g <= 15.0);
            }
        }
    }

    #[test]
    fn test_preset_manager() {
        let pm = PresetManager::new();
        assert!(pm.presets.len() >= 10);
        assert!(pm.find_by_name("Dolby Atmos").is_some());
        assert!(pm.find_by_name("Bang & Olufsen").is_some());
        assert!(pm.find_by_name("Laptop Speaker").is_some());
    }

    #[test]
    fn test_full_pipeline_with_earphones_and_environments() {
        let mut pipeline = AudioPipeline::new(48000.0);

        for earphone in &EarphoneType::ALL {
            pipeline.set_earphone_type(*earphone);
            for env in &EnvironmentMode::ALL {
                pipeline.set_environment_mode(*env);

                for i in 0..100 {
                    let t = i as f32 / 48000.0;
                    let s_l = (2.0 * std::f32::consts::PI * 220.0 * t).sin() * 0.3;
                    let s_r = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.3;
                    let (out_l, out_r) = pipeline.process_stereo_sample(s_l, s_r);
                    assert!(out_l.is_finite());
                    assert!(out_r.is_finite());
                    assert!(out_l.abs() <= 1.05);
                    assert!(out_r.abs() <= 1.05);
                }
            }
        }
    }

    #[test]
    fn test_tinnitus_notch_attenuation() {
        use sonic_aura::dsp::audiogram::{AudiogramMasker, TinnitusEar, TinnitusTherapyMode};
        let mut masker = AudiogramMasker::new(48000.0);
        masker.set_mode(TinnitusTherapyMode::Notch);
        masker.set_ear(TinnitusEar::Both);
        masker.set_freq(6000.0);
        masker.set_q(4.0);

        let sample_rate = 48000.0;
        let freq = 6000.0;
        let mut max_in = 0.0_f32;
        let mut max_out = 0.0_f32;

        // Run for 2000 samples to let filter settle
        for i in 0..2000 {
            let t = i as f32 / sample_rate;
            let s = (2.0 * std::f32::consts::PI * freq * t).sin();
            let (out_l, out_r) = masker.process(s, s);

            if i > 500 {
                max_in = max_in.max(s.abs());
                max_out = max_out.max(out_l.abs().max(out_r.abs()));
            }
        }

        // Notch filter at target pitch should attenuate amplitude by at least 85% (>16 dB)
        assert!(max_out < max_in * 0.15, "Notch did not sufficiently attenuate: max_out={}, max_in={}", max_out, max_in);
    }

    #[test]
    fn test_tinnitus_ear_targeting() {
        use sonic_aura::dsp::audiogram::{AudiogramMasker, TinnitusEar, TinnitusTherapyMode};
        let mut masker = AudiogramMasker::new(48000.0);
        masker.set_mode(TinnitusTherapyMode::Notch);
        masker.set_ear(TinnitusEar::LeftOnly); // Only Left ear notched!
        masker.set_freq(6000.0);
        masker.set_q(4.0);

        let sample_rate = 48000.0;
        let freq = 6000.0;
        let mut max_out_l = 0.0_f32;
        let mut max_out_r = 0.0_f32;

        for i in 0..2000 {
            let t = i as f32 / sample_rate;
            let s = (2.0 * std::f32::consts::PI * freq * t).sin();
            let (out_l, out_r) = masker.process(s, s);

            if i > 500 {
                max_out_l = max_out_l.max(out_l.abs());
                max_out_r = max_out_r.max(out_r.abs());
            }
        }

        // Left ear should be notched heavily
        assert!(max_out_l < 0.2, "Left ear should be notched: {}", max_out_l);
        // Right ear should be unattenuated (near 1.0)
        assert!(max_out_r > 0.9, "Right ear should NOT be notched: {}", max_out_r);
    }

    #[test]
    fn test_tinnitus_masking_noise_and_tone() {
        use sonic_aura::dsp::audiogram::{AudiogramMasker, TinnitusEar, TinnitusTherapyMode};
        let mut masker = AudiogramMasker::new(48000.0);
        masker.set_mode(TinnitusTherapyMode::Masking);
        masker.set_ear(TinnitusEar::Both);
        masker.set_freq(8000.0);
        masker.set_mask_level_db(-30.0);

        // Process silence in to verify noise is injected safely
        let mut max_mask_l = 0.0_f32;
        for _ in 0..1000 {
            let (out_l, out_r) = masker.process(0.0, 0.0);
            assert!(out_l.is_finite());
            assert!(out_r.is_finite());
            max_mask_l = max_mask_l.max(out_l.abs());
        }

        assert!(max_mask_l > 0.0, "Masking noise must be generated");
        assert!(max_mask_l < 0.2, "Masking noise must remain within safe soft limits");

        // Test Pitch-Matching Tone
        masker.set_mode(TinnitusTherapyMode::Off);
        masker.set_test_tone(true);
        let mut max_tone = 0.0_f32;
        for _ in 0..2000 {
            let (out_l, _) = masker.process(0.0, 0.0);
            max_tone = max_tone.max(out_l.abs());
        }
        assert!(max_tone > 0.01, "Test tone should be audible");
        assert!(max_tone < 0.05, "Test tone must stay soft (-30 dBFS peak ~0.0316)");
    }

    #[test]
    fn test_theme_and_tinnitus_config() {
        use sonic_aura::config::{AppConfig, ThemeMode};
        let mut cfg = AppConfig::default();
        cfg.theme_mode = ThemeMode::Light;
        cfg.tinnitus_mode = "combined".to_string();
        cfg.tinnitus_freq = 7500.0;
        cfg.tinnitus_q = 8.0;
        cfg.tinnitus_mask_level_db = -42.0;
        cfg.tinnitus_ear = "left".to_string();

        let toml_str = toml::to_string(&cfg).expect("Failed to serialize AppConfig");
        let deserialized: AppConfig = toml::from_str(&toml_str).expect("Failed to deserialize AppConfig");

        assert_eq!(deserialized.theme_mode, ThemeMode::Light);
        assert_eq!(deserialized.tinnitus_mode, "combined");
        assert_eq!(deserialized.tinnitus_freq, 7500.0);
        assert_eq!(deserialized.tinnitus_q, 8.0);
        assert_eq!(deserialized.tinnitus_mask_level_db, -42.0);
        assert_eq!(deserialized.tinnitus_ear, "left");
    }
}
