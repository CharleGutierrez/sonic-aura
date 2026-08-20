#[cfg(test)]
mod tests {
    use sonic_aura::dsp::ai_analyzer::AiSpectralAnalyzer;
    use std::f32::consts::PI;

    #[test]
    fn test_visualizer_bins_response() {
        let mut analyzer = AiSpectralAnalyzer::new(48000.0);

        for i in 0..48000 {
            let t = i as f32 / 48000.0;
            let s = (2.0 * PI * 440.0 * t).sin() * 0.5;
            analyzer.push_sample(s, s);
        }

        println!("RMS dB: {:.2}", analyzer.features.rms_db);
        println!("Peak dB: {:.2}", analyzer.features.peak_db);
        println!("Peak dB L: {:.2}", analyzer.features.peak_db_l);
        println!("Peak dB R: {:.2}", analyzer.features.peak_db_r);
        println!("Visualizer bins:");
        for (i, b) in analyzer.visualizer_bins.iter().enumerate() {
            print!("b{:02}:{:.2} ", i, b);
            if (i + 1) % 8 == 0 { println!(); }
        }

        assert!(analyzer.features.peak_db > -10.0, "Peak db is too low: {}", analyzer.features.peak_db);
        let max_bin = analyzer.visualizer_bins.iter().cloned().fold(0.0_f32, f32::max);
        assert!(max_bin > 0.3, "Max bin is too low: {}", max_bin);
    }
}
