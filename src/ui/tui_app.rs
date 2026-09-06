//! Interactive Terminal User Interface (TUI) Dashboard

use crate::config::{AppConfig, ThemeMode};
use crate::dsp::ai_analyzer::{AiAdaptiveParameters, AudioFeatures, NUM_SPECTRUM_BINS};
use crate::dsp::audiogram::{TinnitusEar, TinnitusTherapyMode};
use crate::dsp::earphone_profiler::EarphoneType;
use crate::dsp::environment_adapter::EnvironmentMode;
use crate::dsp::pipeline::{PipelineConfig, SharedPipeline};
use crate::dsp::spatializer::SpatialMode;
use crate::presets::PresetManager;
use crate::ui::spectrum::{SpectrumVisualizer, VuMeter};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Gauge, Paragraph};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActivePanel {
    Presets,
    Equalizer,
    Enhancers,
    Tinnitus,
}

#[derive(Clone, Copy)]
pub struct ThemePalette {
    pub mode: ThemeMode,
    pub bg: Option<Color>,
    pub border_normal: Color,
    pub border_focused: Color,
    pub title: Color,
    pub title_focused: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub text_accent: Color,
    pub accent_good: Color,
    pub accent_warn: Color,
    pub accent_alert: Color,
    pub gauge_fill: Color,
    pub gauge_bg: Color,
    pub badge_active_bg: Color,
    pub badge_active_fg: Color,
    pub badge_bypass_bg: Color,
    pub badge_bypass_fg: Color,
}

impl ThemePalette {
    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self {
                mode,
                bg: None,
                border_normal: Color::Rgb(70, 90, 120),
                border_focused: Color::Yellow,
                title: Color::Cyan,
                title_focused: Color::Yellow,
                text_primary: Color::White,
                text_secondary: Color::LightCyan,
                text_muted: Color::Rgb(140, 155, 175),
                text_accent: Color::Yellow,
                accent_good: Color::Green,
                accent_warn: Color::Yellow,
                accent_alert: Color::LightRed,
                gauge_fill: Color::Cyan,
                gauge_bg: Color::Rgb(40, 50, 65),
                badge_active_bg: Color::Green,
                badge_active_fg: Color::Black,
                badge_bypass_bg: Color::Rgb(70, 80, 95),
                badge_bypass_fg: Color::White,
            },
            ThemeMode::Light => Self {
                mode,
                bg: Some(Color::Rgb(245, 246, 250)),
                border_normal: Color::Rgb(140, 150, 165),
                border_focused: Color::Blue,
                title: Color::Blue,
                title_focused: Color::Rgb(180, 83, 9),
                text_primary: Color::Black,
                text_secondary: Color::Rgb(60, 70, 85),
                text_muted: Color::Rgb(130, 140, 155),
                text_accent: Color::Blue,
                accent_good: Color::Rgb(16, 130, 50),
                accent_warn: Color::Rgb(190, 110, 0),
                accent_alert: Color::Rgb(190, 20, 20),
                gauge_fill: Color::Blue,
                gauge_bg: Color::Rgb(215, 220, 230),
                badge_active_bg: Color::Rgb(22, 163, 74),
                badge_active_fg: Color::White,
                badge_bypass_bg: Color::Rgb(148, 163, 184),
                badge_bypass_fg: Color::White,
            },
        }
    }
}

#[derive(Clone)]
struct UiSnapshot {
    visualizer_bins: [f32; NUM_SPECTRUM_BINS],
    peak_hold_bins: [f32; NUM_SPECTRUM_BINS],
    features: AudioFeatures,
    adaptive_params: AiAdaptiveParameters,
    eq_gains: [f32; 10],
    config: PipelineConfig,
    synth_active: bool,
    tinnitus_mode: TinnitusTherapyMode,
    tinnitus_ear: TinnitusEar,
    tinnitus_freq: f32,
    tinnitus_q: f32,
    tinnitus_mask_level_db: f32,
    tinnitus_test_tone: bool,
}

pub struct TuiApp {
    pipeline: SharedPipeline,
    presets: PresetManager,
    config: AppConfig,
    synth_enabled: Arc<AtomicBool>,
    active_sink_name: Arc<Mutex<String>>,
    active_panel: ActivePanel,
    selected_eq_band: usize,
    selected_enhancer: usize,
    selected_tinnitus: usize,
    theme: ThemePalette,
    status_message: String,
    _status_time: Instant,
}

impl TuiApp {
    pub fn new(
        pipeline: SharedPipeline,
        config: AppConfig,
        synth_enabled: Arc<AtomicBool>,
        active_sink_name: Arc<Mutex<String>>,
    ) -> Self {
        let mut presets = PresetManager::new();
        if let Some(idx) = presets.find_by_name(&config.active_preset) {
            presets.select(idx);
        }

        // Apply preset, config, and tinnitus therapy settings
        {
            let mut pl = pipeline.lock().unwrap();
            let p = presets.current();
            let mut cfg = p.to_pipeline_config();
            cfg.earphone_type = config.earphone_type;
            cfg.environment_mode = config.environment_mode;
            pl.apply_config(&cfg);
            pl.set_all_user_eq_gains(&p.eq_gains_10);
            pl.eq.set_preamp(p.master_gain_db);

            let t_mode = TinnitusTherapyMode::from_str(&config.tinnitus_mode);
            let t_ear = TinnitusEar::from_str(&config.tinnitus_ear);
            pl.set_tinnitus_mode(t_mode);
            pl.set_tinnitus_ear(t_ear);
            pl.set_tinnitus_freq(config.tinnitus_freq);
            pl.set_tinnitus_q(config.tinnitus_q);
            pl.set_tinnitus_mask_level_db(config.tinnitus_mask_level_db);
        }

        let theme = ThemePalette::for_mode(config.theme_mode);

        Self {
            pipeline,
            presets,
            config,
            synth_enabled,
            active_sink_name,
            active_panel: ActivePanel::Presets,
            selected_eq_band: 0,
            selected_enhancer: 0,
            selected_tinnitus: 0,
            theme,
            status_message: "Ready. Real-Time Audio DSP & Tinnitus Therapy Active!".to_string(),
            _status_time: Instant::now(),
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let tick_rate = Duration::from_millis(33); // ~30 FPS UI refresh
        let mut last_tick = Instant::now();

        loop {
            // Snapshot UI data with a sub-microsecond lock
            let snapshot = self.take_ui_snapshot();

            terminal.draw(|f| self.render_ui(f, &snapshot))?;

            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.code == KeyCode::Char('q')
                        || (key.code == KeyCode::Char('c')
                            && key.modifiers.contains(KeyModifiers::CONTROL))
                    {
                        break;
                    }
                    self.handle_input(key.code, key.modifiers);
                }
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }

        // Restore terminal
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        Ok(())
    }

    fn take_ui_snapshot(&self) -> UiSnapshot {
        let pl = self.pipeline.lock().unwrap();
        let mut eq_gains = [0.0; 10];
        for i in 0..10 {
            eq_gains[i] = pl.eq.get_band_gain(i);
        }
        let synth_active = self.synth_enabled.load(Ordering::Relaxed);
        let tinnitus_mode = pl.audiogram.mode;
        let tinnitus_ear = pl.audiogram.ear;
        let tinnitus_freq = pl.audiogram.freq;
        let tinnitus_q = pl.audiogram.q;
        let tinnitus_mask_level_db = pl.audiogram.mask_level_db;
        let tinnitus_test_tone = pl.audiogram.test_tone_active;

        UiSnapshot {
            visualizer_bins: pl.ai_analyzer.visualizer_bins,
            peak_hold_bins: pl.ai_analyzer.peak_hold_bins,
            features: pl.ai_analyzer.features.clone(),
            adaptive_params: pl.ai_analyzer.adaptive_params.clone(),
            eq_gains,
            config: pl.config.clone(),
            synth_active,
            tinnitus_mode,
            tinnitus_ear,
            tinnitus_freq,
            tinnitus_q,
            tinnitus_mask_level_db,
            tinnitus_test_tone,
        }
    }

    fn set_status(&mut self, msg: &str) {
        self.status_message = msg.to_string();
        self._status_time = Instant::now();
    }

    fn handle_input(&mut self, code: KeyCode, _modifiers: KeyModifiers) {
        match code {
            KeyCode::Tab => {
                self.active_panel = match self.active_panel {
                    ActivePanel::Presets => ActivePanel::Equalizer,
                    ActivePanel::Equalizer => ActivePanel::Enhancers,
                    ActivePanel::Enhancers => ActivePanel::Tinnitus,
                    ActivePanel::Tinnitus => ActivePanel::Presets,
                };
            }
            KeyCode::Char('x') | KeyCode::Char('X') => {
                if self.active_panel == ActivePanel::Tinnitus {
                    self.active_panel = ActivePanel::Enhancers;
                    self.set_status("🎚️ Switched to Audio Enhancers Panel");
                } else {
                    self.active_panel = ActivePanel::Tinnitus;
                    self.set_status("🩺 Switched to Tinnitus Relief & Acoustic Therapy Studio");
                }
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                self.config.theme_mode = match self.config.theme_mode {
                    ThemeMode::Dark => ThemeMode::Light,
                    ThemeMode::Light => ThemeMode::Dark,
                };
                self.theme = ThemePalette::for_mode(self.config.theme_mode);
                let _ = self.config.save();
                self.set_status(&format!(
                    "🎨 Switched to {} Theme (Saved to config.toml)",
                    self.config.theme_mode.name()
                ));
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                if self.active_panel == ActivePanel::Tinnitus {
                    let active = {
                        let mut pl = self.pipeline.lock().unwrap();
                        pl.toggle_tinnitus_test_tone()
                    };
                    if active {
                        self.set_status("🔊 Pitch-Matching Sine Tone: ACTIVE (-30 dBFS)");
                    } else {
                        self.set_status("🔇 Pitch-Matching Sine Tone: MUTED");
                    }
                } else {
                    let prev = self.synth_enabled.fetch_xor(true, Ordering::Relaxed);
                    let now_active = !prev;
                    if now_active {
                        self.set_status(
                            "🎵 Audio Generator ENGAGED! Driving Real-Time 32-Band FFT & Dynamics",
                        );
                    } else {
                        self.set_status("📥 Switched to System Loopback Audio (SonicAura Sink)");
                    }
                }
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                self.presets.next();
                let p = self.presets.current().clone();
                {
                    let mut pl = self.pipeline.lock().unwrap();
                    let mut cfg = p.to_pipeline_config();
                    cfg.earphone_type = pl.config.earphone_type;
                    cfg.environment_mode = pl.config.environment_mode;
                    pl.apply_config(&cfg);
                    pl.set_all_user_eq_gains(&p.eq_gains_10);
                    pl.eq.set_preamp(p.master_gain_db);
                }
                self.set_status(&format!("Loaded Preset: {}", p.name));
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                let (_next_earphone, name, desc) = {
                    let mut pl = self.pipeline.lock().unwrap();
                    let all = EarphoneType::ALL;
                    let current_idx = all
                        .iter()
                        .position(|&e| e == pl.config.earphone_type)
                        .unwrap_or(0);
                    let next_e = all[(current_idx + 1) % all.len()];
                    pl.set_earphone_type(next_e);
                    self.config.earphone_type = next_e;
                    (next_e, next_e.name(), next_e.description())
                };
                self.set_status(&format!("Earphone: {} - {}", name, desc));
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                let (_next_env, name, desc) = {
                    let mut pl = self.pipeline.lock().unwrap();
                    let all = EnvironmentMode::ALL;
                    let current_idx = all
                        .iter()
                        .position(|&env| env == pl.config.environment_mode)
                        .unwrap_or(0);
                    let next_env = all[(current_idx + 1) % all.len()];
                    pl.set_environment_mode(next_env);
                    self.config.environment_mode = next_env;
                    (next_env, next_env.name(), next_env.description())
                };
                self.set_status(&format!("Environment: {} - {}", name, desc));
            }
            KeyCode::Char(' ') => {
                if self.active_panel == ActivePanel::Tinnitus {
                    if self.selected_tinnitus == 0 {
                        let mode = {
                            let mut pl = self.pipeline.lock().unwrap();
                            let next = match pl.audiogram.mode {
                                TinnitusTherapyMode::Off => TinnitusTherapyMode::Notch,
                                TinnitusTherapyMode::Notch => TinnitusTherapyMode::Masking,
                                TinnitusTherapyMode::Masking => TinnitusTherapyMode::Combined,
                                TinnitusTherapyMode::Combined => TinnitusTherapyMode::Off,
                            };
                            pl.set_tinnitus_mode(next);
                            next
                        };
                        self.set_status(&format!("Tinnitus Therapy: {}", mode.description()));
                    } else {
                        let active = {
                            let mut pl = self.pipeline.lock().unwrap();
                            pl.toggle_tinnitus_test_tone()
                        };
                        if active {
                            self.set_status("🔊 Pitch-Matching Sine Tone: ACTIVE (-30 dBFS)");
                        } else {
                            self.set_status("🔇 Pitch-Matching Sine Tone: MUTED");
                        }
                    }
                } else {
                    let state_str = {
                        let mut pl = self.pipeline.lock().unwrap();
                        pl.config.enabled = !pl.config.enabled;
                        if pl.config.enabled {
                            "ENABLED (Active DSP)"
                        } else {
                            "BYPASSED (Clean Audio)"
                        }
                    };
                    self.set_status(&format!("SonicAura Engine is now {}", state_str));
                }
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                let mode_str = {
                    let mut pl = self.pipeline.lock().unwrap();
                    pl.config.spatial_mode = match pl.config.spatial_mode {
                        SpatialMode::HeadphonesBinaural => SpatialMode::LaptopSpeakers,
                        SpatialMode::LaptopSpeakers => SpatialMode::StudioNearfield,
                        SpatialMode::StudioNearfield => SpatialMode::HeadphonesBinaural,
                    };
                    let mode = pl.config.spatial_mode;
                    pl.spatializer.set_mode(mode);
                    match mode {
                        SpatialMode::HeadphonesBinaural => "Headphones 3D (Dolby Atmos Binaural)",
                        SpatialMode::LaptopSpeakers => "Laptop Speakers (Acoustic Lens Widener)",
                        SpatialMode::StudioNearfield => "Studio Nearfield (Direct Reference)",
                    }
                };
                self.set_status(&format!("Output Mode: {}", mode_str));
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.config.active_preset = self.presets.current().name.clone();
                {
                    let pl = self.pipeline.lock().unwrap();
                    self.config.tinnitus_mode = pl.audiogram.mode.to_str().to_string();
                    self.config.tinnitus_ear = pl.audiogram.ear.to_str().to_string();
                    self.config.tinnitus_freq = pl.audiogram.freq;
                    self.config.tinnitus_q = pl.audiogram.q;
                    self.config.tinnitus_mask_level_db = pl.audiogram.mask_level_db;
                }
                if self.config.save().is_ok() {
                    self.set_status(
                        "Configuration saved successfully to ~/.config/sonic_aura/config.toml",
                    );
                } else {
                    self.set_status("Failed to save config.");
                }
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                let status = {
                    let mut pl = self.pipeline.lock().unwrap();
                    pl.config.ai_boost_enabled = !pl.config.ai_boost_enabled;
                    if pl.config.ai_boost_enabled {
                        "ON"
                    } else {
                        "OFF"
                    }
                };
                self.set_status(&format!("AI Adaptive Intelligence Boost: {}", status));
            }
            KeyCode::Up => self.handle_nav_up(),
            KeyCode::Down => self.handle_nav_down(),
            KeyCode::Left => self.handle_adjust_left(),
            KeyCode::Right => self.handle_adjust_right(),
            KeyCode::Char('0') => {
                if self.active_panel == ActivePanel::Equalizer {
                    {
                        let mut pl = self.pipeline.lock().unwrap();
                        pl.set_user_eq_gain(self.selected_eq_band, 0.0);
                    }
                    self.set_status("Reset EQ band to 0.0 dB");
                } else if self.active_panel == ActivePanel::Tinnitus {
                    {
                        let mut pl = self.pipeline.lock().unwrap();
                        pl.set_tinnitus_freq(6000.0);
                        pl.set_tinnitus_q(4.0);
                        pl.set_tinnitus_mask_level_db(-36.0);
                    }
                    self.set_status("Reset Tinnitus settings to default (6,000 Hz, Q=4.0, -36 dB)");
                }
            }
            _ => {}
        }
    }

    fn handle_nav_up(&mut self) {
        match self.active_panel {
            ActivePanel::Presets => {
                self.presets.prev();
                let p = self.presets.current().clone();
                {
                    let mut pl = self.pipeline.lock().unwrap();
                    let mut cfg = p.to_pipeline_config();
                    cfg.earphone_type = pl.config.earphone_type;
                    cfg.environment_mode = pl.config.environment_mode;
                    pl.apply_config(&cfg);
                    pl.set_all_user_eq_gains(&p.eq_gains_10);
                    pl.eq.set_preamp(p.master_gain_db);
                }
                self.set_status(&format!("Selected Preset: {}", p.name));
            }
            ActivePanel::Equalizer => {
                let mut pl = self.pipeline.lock().unwrap();
                let current = pl.user_eq_gains[self.selected_eq_band];
                pl.set_user_eq_gain(self.selected_eq_band, current + 0.5);
            }
            ActivePanel::Enhancers => {
                if self.selected_enhancer == 0 {
                    self.selected_enhancer = 7;
                } else {
                    self.selected_enhancer -= 1;
                }
            }
            ActivePanel::Tinnitus => {
                if self.selected_tinnitus == 0 {
                    self.selected_tinnitus = 5;
                } else {
                    self.selected_tinnitus -= 1;
                }
            }
        }
    }

    fn handle_nav_down(&mut self) {
        match self.active_panel {
            ActivePanel::Presets => {
                self.presets.next();
                let p = self.presets.current().clone();
                {
                    let mut pl = self.pipeline.lock().unwrap();
                    let mut cfg = p.to_pipeline_config();
                    cfg.earphone_type = pl.config.earphone_type;
                    cfg.environment_mode = pl.config.environment_mode;
                    pl.apply_config(&cfg);
                    pl.set_all_user_eq_gains(&p.eq_gains_10);
                    pl.eq.set_preamp(p.master_gain_db);
                }
                self.set_status(&format!("Selected Preset: {}", p.name));
            }
            ActivePanel::Equalizer => {
                let mut pl = self.pipeline.lock().unwrap();
                let current = pl.user_eq_gains[self.selected_eq_band];
                pl.set_user_eq_gain(self.selected_eq_band, current - 0.5);
            }
            ActivePanel::Enhancers => {
                self.selected_enhancer = (self.selected_enhancer + 1) % 8;
            }
            ActivePanel::Tinnitus => {
                self.selected_tinnitus = (self.selected_tinnitus + 1) % 6;
            }
        }
    }

    fn handle_adjust_left(&mut self) {
        match self.active_panel {
            ActivePanel::Equalizer => {
                if self.selected_eq_band == 0 {
                    self.selected_eq_band = 9;
                } else {
                    self.selected_eq_band -= 1;
                }
            }
            ActivePanel::Enhancers => {
                let mut pl = self.pipeline.lock().unwrap();
                match self.selected_enhancer {
                    0 => {
                        pl.config.ai_intensity = (pl.config.ai_intensity - 0.05).max(0.0);
                    }
                    1 => {
                        pl.config.bass_boost_intensity =
                            (pl.config.bass_boost_intensity - 0.05).max(0.0);
                    }
                    2 => {
                        pl.config.exciter_air_mix = (pl.config.exciter_air_mix - 0.05).max(0.0);
                    }
                    3 => {
                        pl.config.spatial_width = (pl.config.spatial_width - 0.05).max(0.0);
                    }
                    4 => {
                        pl.config.spatial_depth = (pl.config.spatial_depth - 0.05).max(0.0);
                    }
                    5 => {
                        pl.config.transient_attack = (pl.config.transient_attack - 0.05).max(-1.0);
                    }
                    6 => {
                        pl.config.compressor_intensity =
                            (pl.config.compressor_intensity - 0.05).max(0.0);
                    }
                    7 => {
                        pl.config.dynamic_loudness = (pl.config.dynamic_loudness - 0.05).max(0.0);
                    }
                    _ => {}
                }
                let cfg = pl.config.clone();
                pl.apply_config(&cfg);
            }
            ActivePanel::Tinnitus => {
                let msg = {
                    let mut pl = self.pipeline.lock().unwrap();
                    match self.selected_tinnitus {
                        0 => {
                            let next = match pl.audiogram.mode {
                                TinnitusTherapyMode::Off => TinnitusTherapyMode::Combined,
                                TinnitusTherapyMode::Notch => TinnitusTherapyMode::Off,
                                TinnitusTherapyMode::Masking => TinnitusTherapyMode::Notch,
                                TinnitusTherapyMode::Combined => TinnitusTherapyMode::Masking,
                            };
                            pl.set_tinnitus_mode(next);
                            format!("Tinnitus Mode: {}", next.description())
                        }
                        1 => {
                            let new_f = (pl.audiogram.freq - 100.0).max(500.0);
                            pl.set_tinnitus_freq(new_f);
                            format!("Tinnitus Pitch: {:.0} Hz", new_f)
                        }
                        2 => {
                            let cur = pl.audiogram.q;
                            let next = if cur <= 2.5 {
                                8.0
                            } else if cur <= 5.0 {
                                2.0
                            } else {
                                4.0
                            };
                            pl.set_tinnitus_q(next);
                            format!("Notch Bandwidth Q: {:.1}", next)
                        }
                        3 => {
                            let new_l = (pl.audiogram.mask_level_db - 1.0).max(-70.0);
                            pl.set_tinnitus_mask_level_db(new_l);
                            format!("Masking Volume: {:.1} dB", new_l)
                        }
                        4 => {
                            let active = pl.toggle_tinnitus_test_tone();
                            format!(
                                "Test Tone: {}",
                                if active { "ACTIVE" } else { "OFF" }
                            )
                        }
                        5 => {
                            let cur = pl.audiogram.ear;
                            let next = match cur {
                                TinnitusEar::Both => TinnitusEar::RightOnly,
                                TinnitusEar::LeftOnly => TinnitusEar::Both,
                                TinnitusEar::RightOnly => TinnitusEar::LeftOnly,
                            };
                            pl.set_tinnitus_ear(next);
                            format!("Target Ear: {}", next.name())
                        }
                        _ => String::new(),
                    }
                };
                if !msg.is_empty() {
                    self.set_status(&msg);
                }
            }
            _ => {}
        }
    }

    fn handle_adjust_right(&mut self) {
        match self.active_panel {
            ActivePanel::Equalizer => {
                self.selected_eq_band = (self.selected_eq_band + 1) % 10;
            }
            ActivePanel::Enhancers => {
                let mut pl = self.pipeline.lock().unwrap();
                match self.selected_enhancer {
                    0 => {
                        pl.config.ai_intensity = (pl.config.ai_intensity + 0.05).min(1.5);
                    }
                    1 => {
                        pl.config.bass_boost_intensity =
                            (pl.config.bass_boost_intensity + 0.05).min(2.0);
                    }
                    2 => {
                        pl.config.exciter_air_mix = (pl.config.exciter_air_mix + 0.05).min(1.5);
                    }
                    3 => {
                        pl.config.spatial_width = (pl.config.spatial_width + 0.05).min(2.2);
                    }
                    4 => {
                        pl.config.spatial_depth = (pl.config.spatial_depth + 0.05).min(1.0);
                    }
                    5 => {
                        pl.config.transient_attack = (pl.config.transient_attack + 0.05).min(2.0);
                    }
                    6 => {
                        pl.config.compressor_intensity =
                            (pl.config.compressor_intensity + 0.05).min(1.0);
                    }
                    7 => {
                        pl.config.dynamic_loudness = (pl.config.dynamic_loudness + 0.05).min(1.5);
                    }
                    _ => {}
                }
                let cfg = pl.config.clone();
                pl.apply_config(&cfg);
            }
            ActivePanel::Tinnitus => {
                let msg = {
                    let mut pl = self.pipeline.lock().unwrap();
                    match self.selected_tinnitus {
                        0 => {
                            let next = match pl.audiogram.mode {
                                TinnitusTherapyMode::Off => TinnitusTherapyMode::Notch,
                                TinnitusTherapyMode::Notch => TinnitusTherapyMode::Masking,
                                TinnitusTherapyMode::Masking => TinnitusTherapyMode::Combined,
                                TinnitusTherapyMode::Combined => TinnitusTherapyMode::Off,
                            };
                            pl.set_tinnitus_mode(next);
                            format!("Tinnitus Mode: {}", next.description())
                        }
                        1 => {
                            let new_f = (pl.audiogram.freq + 100.0).min(18000.0);
                            pl.set_tinnitus_freq(new_f);
                            format!("Tinnitus Pitch: {:.0} Hz", new_f)
                        }
                        2 => {
                            let cur = pl.audiogram.q;
                            let next = if cur <= 2.5 {
                                4.0
                            } else if cur <= 5.0 {
                                8.0
                            } else {
                                2.0
                            };
                            pl.set_tinnitus_q(next);
                            format!("Notch Bandwidth Q: {:.1}", next)
                        }
                        3 => {
                            let new_l = (pl.audiogram.mask_level_db + 1.0).min(-12.0);
                            pl.set_tinnitus_mask_level_db(new_l);
                            format!("Masking Volume: {:.1} dB", new_l)
                        }
                        4 => {
                            let active = pl.toggle_tinnitus_test_tone();
                            format!(
                                "Test Tone: {}",
                                if active { "ACTIVE" } else { "OFF" }
                            )
                        }
                        5 => {
                            let cur = pl.audiogram.ear;
                            let next = match cur {
                                TinnitusEar::Both => TinnitusEar::LeftOnly,
                                TinnitusEar::LeftOnly => TinnitusEar::RightOnly,
                                TinnitusEar::RightOnly => TinnitusEar::Both,
                            };
                            pl.set_tinnitus_ear(next);
                            format!("Target Ear: {}", next.name())
                        }
                        _ => String::new(),
                    }
                };
                if !msg.is_empty() {
                    self.set_status(&msg);
                }
            }
            _ => {}
        }
    }

    fn render_ui(&self, f: &mut ratatui::Frame, snap: &UiSnapshot) {
        let size = f.area();

        if let Some(bg) = self.theme.bg {
            let bg_block = Block::default().style(Style::default().bg(bg));
            f.render_widget(bg_block, size);
        }

        // Main layout: Header, Body (Top & Bottom), Status Footer
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(16),   // Body
                Constraint::Length(3), // Status bar
            ])
            .split(size);

        self.render_header(f, main_chunks[0], snap);
        self.render_body(f, main_chunks[1], snap);
        self.render_footer(f, main_chunks[2]);
    }

    fn render_header(&self, f: &mut ratatui::Frame, area: Rect, snap: &UiSnapshot) {
        let status_badge = if snap.config.enabled {
            Span::styled(
                " ● ACTIVE ",
                Style::default()
                    .fg(self.theme.badge_active_fg)
                    .bg(self.theme.badge_active_bg)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                " ○ BYPASS ",
                Style::default()
                    .fg(self.theme.badge_bypass_fg)
                    .bg(self.theme.badge_bypass_bg),
            )
        };

        let synth_badge = if snap.synth_active {
            Span::styled(
                " [AUDIO GEN: ENGAGED] ",
                Style::default()
                    .fg(self.theme.badge_active_fg)
                    .bg(self.theme.accent_warn)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                " [AUDIO: SINK LOOPBACK] ",
                Style::default().fg(self.theme.title),
            )
        };

        let earphone_str = format!("🎧 {}", snap.config.earphone_type.name());
        let env_str = snap.config.environment_mode.name();
        let theme_badge = format!(" [🎨 {}] ", self.theme.mode.name());

        let header_text = Line::from(vec![
            Span::styled(
                " ⚡ SONIC AURA AI ",
                Style::default()
                    .fg(self.theme.title_focused)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("│ ", Style::default().fg(self.theme.text_muted)),
            Span::styled(earphone_str, Style::default().fg(self.theme.title)),
            Span::styled(" │ ", Style::default().fg(self.theme.text_muted)),
            Span::styled(env_str, Style::default().fg(self.theme.accent_good)),
            Span::styled(" │ ", Style::default().fg(self.theme.text_muted)),
            Span::styled(theme_badge, Style::default().fg(self.theme.text_accent)),
            Span::styled("│ ", Style::default().fg(self.theme.text_muted)),
            synth_badge,
            Span::raw(" "),
            status_badge,
        ]);

        let header_widget = Paragraph::new(header_text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(self.theme.border_normal)),
        );

        f.render_widget(header_widget, area);
    }

    fn render_body(&self, f: &mut ratatui::Frame, area: Rect, snap: &UiSnapshot) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(45), // Top half: Visualizer & AI Telemetry
                Constraint::Percentage(55), // Bottom half: EQ & Enhancers/Tinnitus
            ])
            .split(area);

        // Top Split: Spectrum Visualizer (55%) + AI & Acoustic Telemetry (45%)
        let top_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(rows[0]);

        self.render_visualizer(f, top_cols[0], snap);
        self.render_ai_telemetry(f, top_cols[1], snap);

        // Bottom Split:
        // On wide terminals (>=135 columns), display 3 panels side-by-side!
        // On standard terminals (<135 columns), split 2 columns (EQ on left, and Enhancers or Tinnitus Studio on right)
        if area.width >= 135 {
            let bot_cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(34),
                    Constraint::Percentage(33),
                    Constraint::Percentage(33),
                ])
                .split(rows[1]);

            self.render_eq_panel(f, bot_cols[0], snap);
            self.render_enhancers_panel(f, bot_cols[1], snap);
            self.render_tinnitus_panel(f, bot_cols[2], snap);
        } else {
            let bot_cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
                .split(rows[1]);

            self.render_eq_panel(f, bot_cols[0], snap);
            if self.active_panel == ActivePanel::Tinnitus {
                self.render_tinnitus_panel(f, bot_cols[1], snap);
            } else {
                self.render_enhancers_panel(f, bot_cols[1], snap);
            }
        }
    }

    fn render_visualizer(&self, f: &mut ratatui::Frame, area: Rect, snap: &UiSnapshot) {
        let block = Block::default()
            .title(Line::from(Span::styled(
                " 📊 Real-Time 32-Band FFT Spectrum & Dynamics ",
                Style::default()
                    .fg(self.theme.accent_good)
                    .add_modifier(Modifier::BOLD),
            )))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(self.theme.accent_good));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(4), Constraint::Length(2)])
            .split(inner);

        // Render FFT bars
        let spec_widget = SpectrumVisualizer::new(&snap.visualizer_bins, &snap.peak_hold_bins, "");
        f.render_widget(spec_widget, split[0]);

        // Render VU meter
        let vu = VuMeter {
            peak_db_l: snap.features.peak_db_l,
            peak_db_r: snap.features.peak_db_r,
            rms_db_l: snap.features.rms_db,
            rms_db_r: snap.features.rms_db,
        };
        vu.render_meter(split[1], f.buffer_mut());
    }

    fn render_ai_telemetry(&self, f: &mut ratatui::Frame, area: Rect, snap: &UiSnapshot) {
        let feat = &snap.features;
        let adapt = &snap.adaptive_params;

        let active_sink = self
            .active_sink_name
            .lock()
            .map(|s| s.clone())
            .unwrap_or_else(|_| "Auto".to_string());

        let block = Block::default()
            .title(Line::from(Span::styled(
                " 🧠 AI & Real-Time Output Sound Telemetry ",
                Style::default()
                    .fg(self.theme.title)
                    .add_modifier(Modifier::BOLD),
            )))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(self.theme.title));

        let text = vec![
            Line::from(vec![
                Span::styled("Tracked Audio Output: ", Style::default().fg(self.theme.text_accent)),
                Span::styled(
                    active_sink,
                    Style::default()
                        .fg(self.theme.text_primary)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Earphone Target: ", Style::default().fg(self.theme.text_secondary)),
                Span::styled(
                    snap.config.earphone_type.name(),
                    Style::default().fg(self.theme.title),
                ),
                Span::styled(" │ ", Style::default().fg(self.theme.text_muted)),
                Span::styled("Context: ", Style::default().fg(self.theme.text_secondary)),
                Span::styled(
                    snap.config.environment_mode.name(),
                    Style::default().fg(self.theme.accent_good),
                ),
            ]),
            Line::from(vec![
                Span::styled("Dialogue Index: ", Style::default().fg(self.theme.text_secondary)),
                Span::styled(
                    format!("{:.0}%", feat.voice_probability * 100.0),
                    Style::default().fg(if feat.voice_probability > 0.5 {
                        self.theme.accent_good
                    } else {
                        self.theme.text_primary
                    }),
                ),
                Span::styled(" │ ", Style::default().fg(self.theme.text_muted)),
                Span::styled("Centroid: ", Style::default().fg(self.theme.text_secondary)),
                Span::styled(
                    format!("{:.0} Hz", feat.spectral_centroid),
                    Style::default().fg(self.theme.text_accent),
                ),
            ]),
            Line::from(vec![
                Span::styled("Loudness: ", Style::default().fg(self.theme.text_secondary)),
                Span::styled(
                    format!("{:.1} LUFS", feat.perceived_loudness_lufs),
                    Style::default().fg(self.theme.title),
                ),
                Span::styled(" │ ", Style::default().fg(self.theme.text_muted)),
                Span::styled("AI Vocal Lift: ", Style::default().fg(self.theme.text_secondary)),
                Span::styled(
                    format!("+{:.1} dB", adapt.dynamic_eq_vocal_boost_db),
                    Style::default()
                        .fg(self.theme.accent_good)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
        ];

        let p = Paragraph::new(text).block(block);
        f.render_widget(p, area);
    }

    fn render_eq_panel(&self, f: &mut ratatui::Frame, area: Rect, snap: &UiSnapshot) {
        let is_focused = self.active_panel == ActivePanel::Equalizer;
        let border_color = if is_focused {
            self.theme.border_focused
        } else {
            self.theme.border_normal
        };

        let title = if is_focused {
            " 🎚️ 10-Band Precision Equalizer [FOCUSED] "
        } else {
            " 🎚️ 10-Band Precision Equalizer "
        };
        let title_style = if is_focused {
            Style::default()
                .fg(self.theme.title_focused)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(self.theme.title)
                .add_modifier(Modifier::BOLD)
        };

        let block = Block::default()
            .title(Line::from(Span::styled(title, title_style)))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let freqs = [
            "31", "63", "125", "250", "500", "1k", "2k", "4k", "8k", "16k",
        ];
        let band_width = (inner.width as usize / 10).max(1);
        let center_y = inner.top() + (inner.height / 2);

        for (i, &freq) in freqs.iter().enumerate() {
            let x = inner.left() + (i * band_width) as u16 + (band_width as u16 / 2);
            let gain = snap.eq_gains[i];
            let is_sel = is_focused && self.selected_eq_band == i;

            // Draw Frequency label
            let f_color = if is_sel {
                self.theme.text_accent
            } else {
                self.theme.text_secondary
            };
            f.buffer_mut().set_string(
                x.saturating_sub(1),
                inner.bottom().saturating_sub(1),
                freq,
                Style::default().fg(f_color),
            );

            // Draw Gain value label
            let g_str = format!("{:+.1}", gain);
            let g_color = if gain > 0.0 {
                self.theme.accent_good
            } else if gain < 0.0 {
                self.theme.accent_alert
            } else {
                self.theme.text_primary
            };
            f.buffer_mut().set_string(
                x.saturating_sub(2),
                inner.top(),
                &g_str,
                Style::default().fg(g_color),
            );

            // Draw slider line & notch
            let max_travel = (inner.height.saturating_sub(3) / 2) as i32;
            let offset = ((gain / 12.0) * max_travel as f32).round() as i32;
            let notch_y = (center_y as i32 - offset)
                .clamp(inner.top() as i32 + 1, inner.bottom() as i32 - 2)
                as u16;

            for y in (inner.top() + 1)..(inner.bottom() - 1) {
                let ch = if y == notch_y {
                    if is_sel { '◆' } else { '■' }
                } else if y == center_y {
                    '┼'
                } else {
                    '│'
                };
                let color = if y == notch_y {
                    if is_sel {
                        self.theme.text_accent
                    } else {
                        self.theme.title
                    }
                } else {
                    self.theme.border_normal
                };
                f.buffer_mut()[(x, y)]
                    .set_char(ch)
                    .set_style(Style::default().fg(color));
            }
        }
    }

    fn render_enhancers_panel(&self, f: &mut ratatui::Frame, area: Rect, snap: &UiSnapshot) {
        let is_focused = self.active_panel == ActivePanel::Enhancers;
        let border_color = if is_focused {
            self.theme.border_focused
        } else {
            self.theme.border_normal
        };

        let title = if is_focused {
            " ✨ AI & Psychoacoustic Boost [FOCUSED - Tab/X for Tinnitus] "
        } else {
            " ✨ AI & Psychoacoustic Boost Parameters "
        };
        let title_style = if is_focused {
            Style::default()
                .fg(self.theme.title_focused)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(self.theme.title)
                .add_modifier(Modifier::BOLD)
        };

        let block = Block::default()
            .title(Line::from(Span::styled(title, title_style)))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let cfg = &snap.config;

        let items = [
            (
                "AI Adaptive Boost",
                format!("{:.0}%", cfg.ai_intensity * 100.0),
                (cfg.ai_intensity / 1.5).clamp(0.0, 1.0),
            ),
            (
                "Psycho Sub-Bass (Missing Fund.)",
                format!("{:.0}%", cfg.bass_boost_intensity * 100.0),
                (cfg.bass_boost_intensity / 2.0).clamp(0.0, 1.0),
            ),
            (
                "B&O Crystal Air & Sparkle",
                format!("{:.0}%", cfg.exciter_air_mix * 100.0),
                (cfg.exciter_air_mix / 1.5).clamp(0.0, 1.0),
            ),
            (
                "Dolby 3D Spatial Width",
                format!("{:.2}x", cfg.spatial_width),
                (cfg.spatial_width / 2.2).clamp(0.0, 1.0),
            ),
            (
                "Virtual Soundstage Depth",
                format!("{:.0}%", cfg.spatial_depth * 100.0),
                cfg.spatial_depth.clamp(0.0, 1.0),
            ),
            (
                "Dynamic Transient Attack",
                format!("{:+.0}%", cfg.transient_attack * 100.0),
                ((cfg.transient_attack + 1.0) / 3.0).clamp(0.0, 1.0),
            ),
            (
                "Multiband Dynamics / Punch",
                format!("{:.0}%", cfg.compressor_intensity * 100.0),
                cfg.compressor_intensity.clamp(0.0, 1.0),
            ),
            (
                "Fletcher-Munson Loudness",
                format!("{:.0}%", cfg.dynamic_loudness * 100.0),
                (cfg.dynamic_loudness / 1.5).clamp(0.0, 1.0),
            ),
        ];

        let num_items = items.len();
        let item_height = (inner.height as usize / num_items).max(1);

        for (i, (name, val_str, ratio)) in items.iter().enumerate() {
            let y = inner.top() + (i * item_height) as u16;
            if y >= inner.bottom() {
                break;
            }

            let is_sel = is_focused && self.selected_enhancer == i;
            let prefix = if is_sel { "▶ " } else { "  " };
            let title_style = if is_sel {
                Style::default()
                    .fg(self.theme.text_accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(self.theme.text_secondary)
            };

            let line_area = Rect {
                x: inner.left(),
                y,
                width: inner.width,
                height: 1,
            };
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(48),
                    Constraint::Percentage(36),
                    Constraint::Percentage(16),
                ])
                .split(line_area);

            // Name
            f.render_widget(
                Paragraph::new(format!("{}{}", prefix, name)).style(title_style),
                cols[0],
            );

            // Mini Gauge / Bar
            let gauge_color = if is_sel {
                self.theme.text_accent
            } else {
                self.theme.gauge_fill
            };
            let gauge = Gauge::default()
                .ratio(*ratio as f64)
                .label("")
                .gauge_style(
                    Style::default()
                        .fg(gauge_color)
                        .bg(self.theme.gauge_bg),
                )
                .use_unicode(true);
            f.render_widget(gauge, cols[1]);

            // Value text
            let val_color = if is_sel {
                self.theme.text_accent
            } else {
                self.theme.title
            };
            f.render_widget(
                Paragraph::new(val_str.as_str())
                    .style(
                        Style::default()
                            .fg(val_color)
                            .add_modifier(Modifier::BOLD),
                    )
                    .alignment(Alignment::Right),
                cols[2],
            );
        }
    }

    fn render_tinnitus_panel(&self, f: &mut ratatui::Frame, area: Rect, snap: &UiSnapshot) {
        let is_focused = self.active_panel == ActivePanel::Tinnitus;
        let border_color = if is_focused {
            self.theme.border_focused
        } else {
            self.theme.border_normal
        };

        let title = if is_focused {
            " 🩺 Tinnitus Relief Studio [FOCUSED - Tab/X to switch] "
        } else {
            " 🩺 Tinnitus Relief Studio "
        };
        let title_style = if is_focused {
            Style::default()
                .fg(self.theme.title_focused)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(self.theme.accent_good)
                .add_modifier(Modifier::BOLD)
        };

        let block = Block::default()
            .title(Line::from(Span::styled(title, title_style)))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));

        let inner = block.inner(area);
        f.render_widget(block, area);

        if inner.height < 6 || inner.width < 25 {
            return;
        }

        let q_str = match snap.tinnitus_q {
            q if q >= 6.0 => "Narrow (Q=8.0 - Sharp Notch)",
            q if q <= 2.5 => "Wide (Q=2.0 - Broad Relief)",
            _ => "Standard (Q=4.0 - Optimal)",
        };

        let mask_pct = ((snap.tinnitus_mask_level_db + 70.0) / 58.0).clamp(0.0, 1.0);

        let items = [
            (
                "Therapy Mode",
                snap.tinnitus_mode.name().to_string(),
                match snap.tinnitus_mode {
                    TinnitusTherapyMode::Off => 0.0,
                    TinnitusTherapyMode::Notch => 0.33,
                    TinnitusTherapyMode::Masking => 0.66,
                    TinnitusTherapyMode::Combined => 1.0,
                },
            ),
            (
                "Pitch Match Freq",
                format!("{:.0} Hz", snap.tinnitus_freq),
                ((snap.tinnitus_freq - 500.0) / 17500.0).clamp(0.0, 1.0),
            ),
            (
                "Notch Bandwidth",
                q_str.to_string(),
                ((snap.tinnitus_q - 2.0) / 6.0).clamp(0.0, 1.0),
            ),
            (
                "Masking Noise Vol",
                format!("{:.1} dB", snap.tinnitus_mask_level_db),
                mask_pct,
            ),
            (
                "Sine Test Tone [T]",
                if snap.tinnitus_test_tone {
                    "● ACTIVE (-30 dBFS)".to_string()
                } else {
                    "○ OFF".to_string()
                },
                if snap.tinnitus_test_tone { 1.0 } else { 0.0 },
            ),
            (
                "Target Ear",
                snap.tinnitus_ear.name().to_string(),
                match snap.tinnitus_ear {
                    TinnitusEar::Both => 0.5,
                    TinnitusEar::LeftOnly => 0.0,
                    TinnitusEar::RightOnly => 1.0,
                },
            ),
        ];

        let num_items = items.len();
        let available_height = inner.height.saturating_sub(1); // reserve 1 line for clinical note
        let item_height = (available_height as usize / num_items).max(1);

        for (i, (name, val_str, ratio)) in items.iter().enumerate() {
            let y = inner.top() + (i * item_height) as u16;
            if y >= inner.bottom().saturating_sub(1) {
                break;
            }

            let is_sel = is_focused && self.selected_tinnitus == i;
            let prefix = if is_sel { "▶ " } else { "  " };
            let title_style = if is_sel {
                Style::default()
                    .fg(self.theme.text_accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(self.theme.text_secondary)
            };

            let line_area = Rect {
                x: inner.left(),
                y,
                width: inner.width,
                height: 1,
            };
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(38),
                    Constraint::Percentage(30),
                    Constraint::Percentage(32),
                ])
                .split(line_area);

            // Item Name
            f.render_widget(
                Paragraph::new(format!("{}{}", prefix, name)).style(title_style),
                cols[0],
            );

            // Ratio Gauge
            let gauge_color = if is_sel {
                self.theme.text_accent
            } else {
                self.theme.gauge_fill
            };
            let gauge = Gauge::default()
                .ratio(*ratio as f64)
                .label("")
                .gauge_style(
                    Style::default()
                        .fg(gauge_color)
                        .bg(self.theme.gauge_bg),
                )
                .use_unicode(true);
            f.render_widget(gauge, cols[1]);

            // Value text
            let val_color = if i == 4 && snap.tinnitus_test_tone {
                self.theme.accent_warn
            } else if is_sel {
                self.theme.text_accent
            } else {
                self.theme.title
            };
            f.render_widget(
                Paragraph::new(val_str.as_str())
                    .style(
                        Style::default()
                            .fg(val_color)
                            .add_modifier(Modifier::BOLD),
                    )
                    .alignment(Alignment::Right),
                cols[2],
            );
        }

        // Clinical explanation / guidance line at bottom
        let note_y = inner.bottom().saturating_sub(1);
        let note_text = match snap.tinnitus_mode {
            TinnitusTherapyMode::Notch => {
                "💡 TMNST Active: Notches out ringing frequency to retrain cortical neurons via lateral inhibition"
            }
            TinnitusTherapyMode::Masking => {
                "💡 Acoustic Masker Active: Gentle pink noise blends out your tinnitus ringing frequency"
            }
            TinnitusTherapyMode::Combined => {
                "💡 Full Therapy: Simultaneous TMNST notched music + ambient shaped noise pillow"
            }
            TinnitusTherapyMode::Off => {
                "💡 Press [Left]/[Right] on Therapy Mode or [T] to calibrate ringing pitch"
            }
        };

        f.render_widget(
            Paragraph::new(note_text).style(Style::default().fg(self.theme.text_secondary)),
            Rect {
                x: inner.left(),
                y: note_y,
                width: inner.width,
                height: 1,
            },
        );
    }

    fn render_footer(&self, f: &mut ratatui::Frame, area: Rect) {
        let preset_name = self.presets.current().name.as_str();

        let help_text = Line::from(vec![
            Span::styled(
                " [T] ",
                Style::default()
                    .fg(self.theme.badge_active_fg)
                    .bg(self.theme.text_accent),
            ),
            Span::styled(" Output/Tone │ ", Style::default().fg(self.theme.text_accent)),
            Span::styled(
                " [P] ",
                Style::default()
                    .fg(self.theme.badge_active_fg)
                    .bg(self.theme.text_primary),
            ),
            Span::styled(
                format!(" Preset ({}) │ ", preset_name),
                Style::default().fg(self.theme.title),
            ),
            Span::styled(
                " [X] ",
                Style::default()
                    .fg(self.theme.badge_active_fg)
                    .bg(self.theme.text_primary),
            ),
            Span::styled(" Tinnitus Studio │ ", Style::default().fg(self.theme.accent_good)),
            Span::styled(
                " [L] ",
                Style::default()
                    .fg(self.theme.badge_active_fg)
                    .bg(self.theme.text_primary),
            ),
            Span::styled(
                format!(" Theme ({}) │ ", self.theme.mode.name()),
                Style::default().fg(self.theme.title),
            ),
            Span::styled(
                " [Space] ",
                Style::default()
                    .fg(self.theme.badge_active_fg)
                    .bg(self.theme.text_primary),
            ),
            Span::styled(" Bypass │ ", Style::default().fg(self.theme.text_secondary)),
            Span::styled(
                " [Q] ",
                Style::default()
                    .fg(self.theme.badge_active_fg)
                    .bg(self.theme.text_primary),
            ),
            Span::styled(" Quit", Style::default().fg(self.theme.text_secondary)),
        ]);

        let status_line = Line::from(vec![
            Span::styled(" Status: ", Style::default().fg(self.theme.text_secondary)),
            Span::styled(&self.status_message, Style::default().fg(self.theme.text_accent)),
        ]);

        let footer_widget = Paragraph::new(vec![help_text, status_line]).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(self.theme.border_normal)),
        );

        f.render_widget(footer_widget, area);
    }
}
