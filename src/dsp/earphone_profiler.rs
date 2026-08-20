//! Universal Earphone & Headphone Acoustic Calibration Engine
//! Calibrates frequency response, driver limitations, resonance peaks, and spatial characteristics
//! for any type of earphone in the market: from ultra-cheap $5 earbuds to high-end planar IEMs and open-back studio headphones.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EarphoneType {
    /// Low-End / $5-$20 Budget Earbuds: Fixes tinny sound, removes 3kHz harshness, generates missing sub-bass, adds crystal air
    BudgetEarbudsFix,
    /// Apple AirPods / TWS Earbuds: Harman In-Ear target, wide 3D Atmos soundstage, open-fit low-end compensation
    AirPodsAndTws,
    /// Bass-Heavy Commercial Buds (Beats, Skullcandy, Sony Extra Bass): De-bloats muddy 150-300Hz, lifts vocal presence & transient punch
    BassHeavyCommercial,
    /// Audiophile / In-Ear Monitors (IEMs - Moondrop, Chi-Fi, Sennheiser IE): Harman 2019 target, anti-fatigue HRTF crossfeed, sibilance tamer
    IemAudiophile,
    /// Open-Back Studio Reference (Sennheiser HD600/650, DT990, Hifiman Planar): Sub-bass roll-off extension, treble de-spike, concert depth
    StudioOpenBack,
    /// Closed-Back Studio Monitoring (Audio-Technica ATH-M50x, Sony MDR-7506, DT770): Cup resonance de-masking, 3D stereo expander
    StudioClosedBack,
    /// Flat Reference / Uncalibrated
    UniversalNeutral,
}

impl EarphoneType {
    pub const ALL: [EarphoneType; 7] = [
        EarphoneType::BudgetEarbudsFix,
        EarphoneType::AirPodsAndTws,
        EarphoneType::BassHeavyCommercial,
        EarphoneType::IemAudiophile,
        EarphoneType::StudioOpenBack,
        EarphoneType::StudioClosedBack,
        EarphoneType::UniversalNeutral,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            EarphoneType::BudgetEarbudsFix => "Budget Earbuds (Tinny Fix & Sub-Bass)",
            EarphoneType::AirPodsAndTws => "Apple AirPods / TWS (Harman 3D)",
            EarphoneType::BassHeavyCommercial => "Bass-Heavy Buds (Vocal De-Bloat)",
            EarphoneType::IemAudiophile => "Audiophile IEMs (Harman Target & HRTF)",
            EarphoneType::StudioOpenBack => "Studio Open-Back (Sub Extension & Air)",
            EarphoneType::StudioClosedBack => "Studio Closed-Back (Cup De-Resonance)",
            EarphoneType::UniversalNeutral => "Universal Reference (Flat Neutral)",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            EarphoneType::BudgetEarbudsFix => "Rescues cheap earbuds: injects psychoacoustic missing fundamental bass, notches 3kHz harshness, adds >10kHz air.",
            EarphoneType::AirPodsAndTws => "Calibrated for AirPods/TWS: Harman In-Ear target curve, 3D Atmos soundstage, pristine vocal articulation.",
            EarphoneType::BassHeavyCommercial => "Cleans up bloated mid-bass, restores buried vocal formants, and sharpens transient snare/drum punch.",
            EarphoneType::IemAudiophile => "Harman 2019 Reference In-Ear Target with Meier binaural crossfeed to eliminate listening fatigue.",
            EarphoneType::StudioOpenBack => "Extends rolling-off sub-bass (<50Hz), smooths 8.5kHz treble spikes, and creates deep concert hall imaging.",
            EarphoneType::StudioClosedBack => "Eliminates enclosed earcup resonance (220Hz), expands narrow soundstage, and delivers mastering clarity.",
            EarphoneType::UniversalNeutral => "Zero hardware coloration, completely transparent studio calibration.",
        }
    }

    /// Returns the 10-Band EQ target compensation offsets [31Hz, 63Hz, 125Hz, 250Hz, 500Hz, 1kHz, 2kHz, 4kHz, 8kHz, 16kHz]
    pub fn eq_offsets(&self) -> [f32; 10] {
        match self {
            EarphoneType::BudgetEarbudsFix => [
                6.5,   // 31Hz: Sub-bass excitation boost
                5.0,   // 63Hz: Lows
                2.5,   // 125Hz
                -3.5,  // 250Hz: De-mud boxiness
                -2.0,  // 500Hz
                1.5,   // 1kHz: Vocal presence
                -2.5,  // 2kHz: Tames piercing cheap driver resonance
                2.0,   // 4kHz: Clarity
                4.5,   // 8kHz: Shimmer
                6.0,   // 16kHz: Missing Air restore
            ],
            EarphoneType::AirPodsAndTws => [
                3.5, 2.5, 1.0, -0.5, 0.0, 1.2, 2.2, 2.8, 3.5, 4.0,
            ],
            EarphoneType::BassHeavyCommercial => [
                1.0, 0.0, -3.5, -4.0, -1.5, 1.5, 3.0, 3.5, 4.0, 3.5,
            ],
            EarphoneType::IemAudiophile => [
                2.5, 2.0, 0.8, -0.2, 0.0, 0.8, 1.5, 1.2, 1.0, 2.5,
            ],
            EarphoneType::StudioOpenBack => [
                5.0, 3.5, 1.2, 0.0, 0.0, 0.5, 1.0, 0.5, -1.8, 2.5,
            ],
            EarphoneType::StudioClosedBack => [
                2.0, 1.5, -2.5, -2.0, 0.5, 1.2, 2.0, 2.5, 3.0, 3.5,
            ],
            EarphoneType::UniversalNeutral => [
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
        }
    }

    /// Tuning modifiers: (bass_intensity_mult, air_mix_mult, spatial_width_mult, crossfeed_mult, transient_mult)
    pub fn dsp_modifiers(&self) -> (f32, f32, f32, f32, f32) {
        match self {
            EarphoneType::BudgetEarbudsFix => (1.5, 1.4, 1.35, 0.3, 1.3),
            EarphoneType::AirPodsAndTws => (1.1, 1.1, 1.35, 0.45, 1.1),
            EarphoneType::BassHeavyCommercial => (0.7, 1.3, 1.25, 0.4, 1.5),
            EarphoneType::IemAudiophile => (0.9, 1.0, 1.2, 0.65, 1.0),
            EarphoneType::StudioOpenBack => (1.3, 1.0, 1.15, 0.35, 1.0),
            EarphoneType::StudioClosedBack => (1.0, 1.15, 1.4, 0.45, 1.1),
            EarphoneType::UniversalNeutral => (1.0, 1.0, 1.0, 0.0, 1.0),
        }
    }
}
