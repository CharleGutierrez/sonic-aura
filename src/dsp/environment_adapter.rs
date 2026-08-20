//! Adaptive Environmental Noise & Acoustic Context Engine
//! Dynamically adapts psychoacoustic DSP to overcome external masking noise
//! whether in a noisy metropolitan city, busy airplane/train, crowded coffee shop,
//! or a dead-silent remote mountain/cabin.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnvironmentMode {
    /// Busy City & Traffic: High ambient bass rumble (engines, buses, tires) and street noise
    CityTraffic,
    /// Airplane / Subway / Train: High continuous low-frequency drone & cabin noise
    CommuteTransit,
    /// Cafe / Office / Crowd: Multi-speaker babble (500Hz-2kHz) and background clatter
    CafeOffice,
    /// Quiet Remote / Silent Room: Zero noise floor (<20dB), audiophile pure dynamic range & micro-details
    QuietRemote,
    /// Late Night Whisper Mode: Ultra-low listening volume with aggressive equal-loudness compensation
    LateNightWhisper,
    /// Standard Room / Balanced Neutral
    NeutralStudio,
}

impl EnvironmentMode {
    pub const ALL: [EnvironmentMode; 6] = [
        EnvironmentMode::CityTraffic,
        EnvironmentMode::CommuteTransit,
        EnvironmentMode::CafeOffice,
        EnvironmentMode::QuietRemote,
        EnvironmentMode::LateNightWhisper,
        EnvironmentMode::NeutralStudio,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            EnvironmentMode::CityTraffic => "🏙️ Busy City & Street Traffic",
            EnvironmentMode::CommuteTransit => "✈️ Airplane / Subway / Train",
            EnvironmentMode::CafeOffice => "☕ Cafe / Office / Coworking",
            EnvironmentMode::QuietRemote => "🍃 Remote Nature / Silent Room",
            EnvironmentMode::LateNightWhisper => "🌙 Late-Night Whisper Mode",
            EnvironmentMode::NeutralStudio => "🏠 Balanced Studio / Indoor",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            EnvironmentMode::CityTraffic => "Overcomes urban traffic & engine rumble: boosts anti-masking sub-bass, lifts speech clarity (+3.5dB), and expands dynamic punch.",
            EnvironmentMode::CommuteTransit => "Cabin drone immunity: suppresses engine rumble masking and applies dialogue-priority compression for clear travel listening.",
            EnvironmentMode::CafeOffice => "Separates audio from background chatter: applies vocal formant focus and 3D soundstage widening to isolate music from crowd noise.",
            EnvironmentMode::QuietRemote => "Maximizes audiophile dynamic range, micro-detail resolution, and natural 3D depth with gentle transparent processing.",
            EnvironmentMode::LateNightWhisper => "Full-bodied listening at whisper-quiet volumes: aggressive Fletcher-Munson equal-loudness curve ensures deep bass & clear vocals at 15% volume.",
            EnvironmentMode::NeutralStudio => "Standard balanced acoustic environment without external noise compensation.",
        }
    }

    /// Environmental EQ offsets [31Hz, 63Hz, 125Hz, 250Hz, 500Hz, 1kHz, 2kHz, 4kHz, 8kHz, 16kHz]
    pub fn eq_offsets(&self) -> [f32; 10] {
        match self {
            EnvironmentMode::CityTraffic => [
                5.0,  // 31Hz: Sub-bass anti-masking
                4.2,  // 63Hz: Traffic de-masking
                2.0,  // 125Hz
                -1.0, // 250Hz
                0.0,  // 500Hz
                1.5,  // 1kHz
                3.5,  // 2kHz: Vocal presence through noise
                3.0,  // 4kHz
                2.0,  // 8kHz
                1.5,  // 16kHz
            ],
            EnvironmentMode::CommuteTransit => [
                6.0, 5.0, 2.5, -0.5, 0.0, 2.0, 3.5, 3.0, 2.5, 2.0,
            ],
            EnvironmentMode::CafeOffice => [
                2.0, 1.5, 0.5, -2.0, -1.0, 2.5, 3.5, 3.0, 2.5, 3.0,
            ],
            EnvironmentMode::QuietRemote => [
                1.5, 1.0, 0.5, 0.0, 0.0, 0.5, 1.0, 1.5, 2.5, 3.5,
            ],
            EnvironmentMode::LateNightWhisper => [
                7.0, 6.0, 3.5, 1.0, 1.5, 2.5, 4.0, 4.5, 5.5, 6.5,
            ],
            EnvironmentMode::NeutralStudio => [
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
        }
    }

    /// Returns modifiers: (bass_boost_mult, air_mix_mult, spatial_width_mult, comp_intensity_mult, dynamic_loudness_mult)
    pub fn dsp_modifiers(&self) -> (f32, f32, f32, f32, f32) {
        match self {
            EnvironmentMode::CityTraffic => (1.4, 1.2, 1.3, 1.4, 1.4),
            EnvironmentMode::CommuteTransit => (1.5, 1.1, 1.2, 1.5, 1.5),
            EnvironmentMode::CafeOffice => (1.1, 1.3, 1.5, 1.2, 1.1),
            EnvironmentMode::QuietRemote => (0.9, 1.0, 1.3, 0.6, 0.8),
            EnvironmentMode::LateNightWhisper => (1.6, 1.5, 1.2, 1.5, 1.8),
            EnvironmentMode::NeutralStudio => (1.0, 1.0, 1.0, 1.0, 1.0),
        }
    }
}
