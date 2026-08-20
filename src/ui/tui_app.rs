//! Interactive Terminal User Interface (TUI) Dashboard

use crate::config::AppConfig;
use crate::dsp::ai_analyzer::{AiAdaptiveParameters, AudioFeatures, NUM_SPECTRUM_BINS};
use crate::dsp::earphone_profiler::EarphoneType;
use crate::dsp::environment_adapter::EnvironmentMode;
use crate::dsp::pipeline::{PipelineConfig, SharedPipeline};
use crate::dsp::spatializer::SpatialMode;
use crate::presets::PresetManager;
use crate::ui::spectrum::{SpectrumVisualizer, VuMeter};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Gauge, Paragraph};
use ratatui::Terminal;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActivePanel {
    Presets,
    Equalizer,
    Enhancers,
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

        // Apply preset and config
        {
            let mut pl = pipeline.lock().unwrap();
            let p = presets.current();
            let mut cfg = p.to_pipeline_config();
            cfg.earphone_type = config.earphone_type;
            cfg.environment_mode = config.environment_mode;
            pl.apply_config(&cfg);
            pl.set_all_user_eq_gains(&p.eq_gains_10);
            pl.eq.set_preamp(p.master_gain_db);
        }

        Self {
            pipeline,
            presets,
            config,
            synth_enabled,
            active_sink_name,
            active_panel: ActivePanel::Presets,
            selected_eq_band: 0,
            selected_enhancer: 0,
            status_message: "Ready. Auto-tracking laptop output sound in real time!".to_string(),
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
                    if key.code == KeyCode::Char('q') || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)) {
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
        UiSnapshot {
            visualizer_bins: pl.ai_analyzer.visualizer_bins,
            peak_hold_bins: pl.ai_analyzer.peak_hold_bins,
            features: pl.ai_analyzer.features.clone(),
            adaptive_params: pl.ai_analyzer.adaptive_params.clone(),
            eq_gains,
            config: pl.config.clone(),
            synth_active,
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
                    ActivePanel::Enhancers => ActivePanel::Presets,
                };
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                let prev = self.synth_enabled.fetch_xor(true, Ordering::Relaxed);
                let now_active = !prev;
                if now_active {
                    self.set_status("🎵 Audio Generator ENGAGED! Driving Real-Time 32-Band FFT & Dynamics");
                } else {
                    self.set_status("📥 Switched to System Loopback Audio (SonicAura Sink)");
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
                    let current_idx = all.iter().position(|&e| e == pl.config.earphone_type).unwrap_or(0);
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
                    let current_idx = all.iter().position(|&env| env == pl.config.environment_mode).unwrap_or(0);
                    let next_env = all[(current_idx + 1) % all.len()];
                    pl.set_environment_mode(next_env);
                    self.config.environment_mode = next_env;
                    (next_env, next_env.name(), next_env.description())
                };
                self.set_status(&format!("Environment: {} - {}", name, desc));
            }
            KeyCode::Char(' ') => {
                let state_str = {
                    let mut pl = self.pipeline.lock().unwrap();
                    pl.config.enabled = !pl.config.enabled;
                    if pl.config.enabled { "ENABLED (Active DSP)" } else { "BYPASSED (Clean Audio)" }
                };
                self.set_status(&format!("SonicAura Engine is now {}", state_str));
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
                if self.config.save().is_ok() {
                    self.set_status("Configuration saved successfully to ~/.config/sonic_aura/config.toml");
                } else {
                    self.set_status("Failed to save config.");
                }
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                let status = {
                    let mut pl = self.pipeline.lock().unwrap();
                    pl.config.ai_boost_enabled = !pl.config.ai_boost_enabled;
                    if pl.config.ai_boost_enabled { "ON" } else { "OFF" }
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
                    0 => { pl.config.ai_intensity = (pl.config.ai_intensity - 0.05).max(0.0); }
                    1 => { pl.config.bass_boost_intensity = (pl.config.bass_boost_intensity - 0.05).max(0.0); }
                    2 => { pl.config.exciter_air_mix = (pl.config.exciter_air_mix - 0.05).max(0.0); }
                    3 => { pl.config.spatial_width = (pl.config.spatial_width - 0.05).max(0.0); }
                    4 => { pl.config.spatial_depth = (pl.config.spatial_depth - 0.05).max(0.0); }
                    5 => { pl.config.transient_attack = (pl.config.transient_attack - 0.05).max(-1.0); }
                    6 => { pl.config.compressor_intensity = (pl.config.compressor_intensity - 0.05).max(0.0); }
                    7 => { pl.config.dynamic_loudness = (pl.config.dynamic_loudness - 0.05).max(0.0); }
                    _ => {}
                }
                let cfg = pl.config.clone();
                pl.apply_config(&cfg);
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
                    0 => { pl.config.ai_intensity = (pl.config.ai_intensity + 0.05).min(1.5); }
                    1 => { pl.config.bass_boost_intensity = (pl.config.bass_boost_intensity + 0.05).min(2.0); }
                    2 => { pl.config.exciter_air_mix = (pl.config.exciter_air_mix + 0.05).min(1.5); }
                    3 => { pl.config.spatial_width = (pl.config.spatial_width + 0.05).min(2.2); }
                    4 => { pl.config.spatial_depth = (pl.config.spatial_depth + 0.05).min(1.0); }
                    5 => { pl.config.transient_attack = (pl.config.transient_attack + 0.05).min(2.0); }
                    6 => { pl.config.compressor_intensity = (pl.config.compressor_intensity + 0.05).min(1.0); }
                    7 => { pl.config.dynamic_loudness = (pl.config.dynamic_loudness + 0.05).min(1.5); }
                    _ => {}
                }
                let cfg = pl.config.clone();
                pl.apply_config(&cfg);
            }
            _ => {}
        }
    }

    fn render_ui(&self, f: &mut ratatui::Frame, snap: &UiSnapshot) {
        let size = f.area();

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
            Span::styled(" ● ACTIVE ", Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(" ○ BYPASS ", Style::default().fg(Color::White).bg(Color::DarkGray))
        };

        let synth_badge = if snap.synth_active {
            Span::styled(" [AUDIO GEN: ENGAGED] ", Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(" [AUDIO: SINK LOOPBACK] ", Style::default().fg(Color::Cyan))
        };

        let earphone_str = format!("🎧 {}", snap.config.earphone_type.name());
        let env_str = snap.config.environment_mode.name();

        let header_text = Line::from(vec![
            Span::styled(" ⚡ SONIC AURA AI ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled("│ ", Style::default().fg(Color::DarkGray)),
            Span::styled(earphone_str, Style::default().fg(Color::LightCyan)),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::styled(env_str, Style::default().fg(Color::LightGreen)),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            synth_badge,
            Span::raw(" "),
            status_badge,
        ]);

        let header_widget = Paragraph::new(header_text)
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Color::Cyan)));

        f.render_widget(header_widget, area);
    }

    fn render_body(&self, f: &mut ratatui::Frame, area: Rect, snap: &UiSnapshot) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(45), // Top half: Visualizer & AI Telemetry
                Constraint::Percentage(55), // Bottom half: EQ & Enhancers
            ])
            .split(area);

        // Top Split: Spectrum Visualizer (55%) + AI & Acoustic Telemetry (45%)
        let top_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(55),
                Constraint::Percentage(45),
            ])
            .split(rows[0]);

        self.render_visualizer(f, top_cols[0], snap);
        self.render_ai_telemetry(f, top_cols[1], snap);

        // Bottom Split: Presets & EQ (52%) + DSP Controls (48%)
        let bot_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(52),
                Constraint::Percentage(48),
            ])
            .split(rows[1]);

        self.render_eq_panel(f, bot_cols[0], snap);
        self.render_enhancers_panel(f, bot_cols[1], snap);
    }

    fn render_visualizer(&self, f: &mut ratatui::Frame, area: Rect, snap: &UiSnapshot) {
        let block = Block::default()
            .title(" 📊 Real-Time 32-Band FFT Spectrum & Dynamics ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Green));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(4),
                Constraint::Length(2),
            ])
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

        let active_sink = self.active_sink_name.lock().map(|s| s.clone()).unwrap_or_else(|_| "Auto".to_string());

        let block = Block::default()
            .title(" 🧠 AI & Real-Time Output Sound Telemetry ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan));

        let text = vec![
            Line::from(vec![
                Span::styled("Tracked Audio Output: ", Style::default().fg(Color::Yellow)),
                Span::styled(active_sink, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("Earphone Target: ", Style::default().fg(Color::Gray)),
                Span::styled(snap.config.earphone_type.name(), Style::default().fg(Color::LightCyan)),
                Span::raw(" │ Context: "),
                Span::styled(snap.config.environment_mode.name(), Style::default().fg(Color::LightGreen)),
            ]),
            Line::from(vec![
                Span::styled("Dialogue Index: ", Style::default().fg(Color::Gray)),
                Span::styled(format!("{:.0}%", feat.voice_probability * 100.0), Style::default().fg(if feat.voice_probability > 0.5 { Color::Green } else { Color::White })),
                Span::raw(" │ Centroid: "),
                Span::styled(format!("{:.0} Hz", feat.spectral_centroid), Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::styled("Loudness: ", Style::default().fg(Color::Gray)),
                Span::styled(format!("{:.1} LUFS", feat.perceived_loudness_lufs), Style::default().fg(Color::LightBlue)),
                Span::raw(" │ AI Vocal Lift: "),
                Span::styled(format!("+{:.1} dB", adapt.dynamic_eq_vocal_boost_db), Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)),
            ]),
        ];

        let p = Paragraph::new(text).block(block);
        f.render_widget(p, area);
    }

    fn render_eq_panel(&self, f: &mut ratatui::Frame, area: Rect, snap: &UiSnapshot) {
        let is_focused = self.active_panel == ActivePanel::Equalizer;
        let border_color = if is_focused { Color::Yellow } else { Color::DarkGray };

        let block = Block::default()
            .title(" 🎚️ 10-Band Precision Equalizer (ISO + Calibrated) ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let freqs = ["31", "63", "125", "250", "500", "1k", "2k", "4k", "8k", "16k"];
        let band_width = (inner.width as usize / 10).max(1);
        let center_y = inner.top() + (inner.height / 2);

        for (i, &freq) in freqs.iter().enumerate() {
            let x = inner.left() + (i * band_width) as u16 + (band_width as u16 / 2);
            let gain = snap.eq_gains[i];
            let is_sel = is_focused && self.selected_eq_band == i;

            // Draw Frequency label
            let f_color = if is_sel { Color::Yellow } else { Color::DarkGray };
            f.buffer_mut().set_string(x.saturating_sub(1), inner.bottom().saturating_sub(1), freq, Style::default().fg(f_color));

            // Draw Gain value label
            let g_str = format!("{:+.1}", gain);
            let g_color = if gain > 0.0 { Color::Green } else if gain < 0.0 { Color::LightRed } else { Color::White };
            f.buffer_mut().set_string(x.saturating_sub(2), inner.top(), &g_str, Style::default().fg(g_color));

            // Draw slider line & notch
            let max_travel = (inner.height.saturating_sub(3) / 2) as i32;
            let offset = ((gain / 12.0) * max_travel as f32).round() as i32;
            let notch_y = (center_y as i32 - offset).clamp(inner.top() as i32 + 1, inner.bottom() as i32 - 2) as u16;

            for y in (inner.top() + 1)..(inner.bottom() - 1) {
                let ch = if y == notch_y {
                    if is_sel { '◆' } else { '■' }
                } else if y == center_y {
                    '┼'
                } else {
                    '│'
                };
                let color = if y == notch_y {
                    if is_sel { Color::Yellow } else { Color::Cyan }
                } else {
                    Color::DarkGray
                };
                f.buffer_mut()[(x, y)].set_char(ch).set_style(Style::default().fg(color));
            }
        }
    }

    fn render_enhancers_panel(&self, f: &mut ratatui::Frame, area: Rect, snap: &UiSnapshot) {
        let is_focused = self.active_panel == ActivePanel::Enhancers;
        let border_color = if is_focused { Color::Yellow } else { Color::DarkGray };

        let block = Block::default()
            .title(" ✨ AI & Psychoacoustic Boost Parameters ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let cfg = &snap.config;

        let items = [
            ("AI Adaptive Boost", format!("{:.0}%", cfg.ai_intensity * 100.0), (cfg.ai_intensity / 1.5).clamp(0.0, 1.0)),
            ("Psycho Sub-Bass (Missing Fund.)", format!("{:.0}%", cfg.bass_boost_intensity * 100.0), (cfg.bass_boost_intensity / 2.0).clamp(0.0, 1.0)),
            ("B&O Crystal Air & Sparkle", format!("{:.0}%", cfg.exciter_air_mix * 100.0), (cfg.exciter_air_mix / 1.5).clamp(0.0, 1.0)),
            ("Dolby 3D Spatial Width", format!("{:.2}x", cfg.spatial_width), (cfg.spatial_width / 2.2).clamp(0.0, 1.0)),
            ("Virtual Soundstage Depth", format!("{:.0}%", cfg.spatial_depth * 100.0), cfg.spatial_depth.clamp(0.0, 1.0)),
            ("Dynamic Transient Attack", format!("{:+.0}%", cfg.transient_attack * 100.0), ((cfg.transient_attack + 1.0) / 3.0).clamp(0.0, 1.0)),
            ("Multiband Dynamics / Punch", format!("{:.0}%", cfg.compressor_intensity * 100.0), cfg.compressor_intensity.clamp(0.0, 1.0)),
            ("Fletcher-Munson Loudness", format!("{:.0}%", cfg.dynamic_loudness * 100.0), (cfg.dynamic_loudness / 1.5).clamp(0.0, 1.0)),
        ];

        let num_items = items.len();
        let item_height = (inner.height as usize / num_items).max(1);

        for (i, (name, val_str, ratio)) in items.iter().enumerate() {
            let y = inner.top() + (i * item_height) as u16;
            if y >= inner.bottom() { break; }

            let is_sel = is_focused && self.selected_enhancer == i;
            let prefix = if is_sel { "▶ " } else { "  " };
            let title_style = if is_sel {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let line_area = Rect { x: inner.left(), y, width: inner.width, height: 1 };
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(48),
                    Constraint::Percentage(36),
                    Constraint::Percentage(16),
                ])
                .split(line_area);

            // Name
            f.render_widget(Paragraph::new(format!("{}{}", prefix, name)).style(title_style), cols[0]);

            // Mini Gauge / Bar
            let gauge_color = if is_sel { Color::Yellow } else { Color::Cyan };
            let gauge = Gauge::default()
                .ratio(*ratio as f64)
                .gauge_style(Style::default().fg(gauge_color).bg(Color::DarkGray))
                .use_unicode(true);
            f.render_widget(gauge, cols[1]);

            // Value text
            f.render_widget(
                Paragraph::new(val_str.as_str())
                    .style(Style::default().fg(Color::White))
                    .alignment(Alignment::Right),
                cols[2],
            );
        }
    }

    fn render_footer(&self, f: &mut ratatui::Frame, area: Rect) {
        let preset_name = self.presets.current().name.as_str();

        let help_text = Line::from(vec![
            Span::styled(" [T] ", Style::default().fg(Color::Black).bg(Color::Yellow)),
            Span::styled(" Sound Output/Synth │ ", Style::default().fg(Color::Yellow)),
            Span::styled(" [P] ", Style::default().fg(Color::Black).bg(Color::White)),
            Span::styled(format!(" Preset ({}) │ ", preset_name), Style::default().fg(Color::Magenta)),
            Span::styled(" [E] ", Style::default().fg(Color::Black).bg(Color::White)),
            Span::styled(" Earphone │ ", Style::default().fg(Color::LightCyan)),
            Span::styled(" [N] ", Style::default().fg(Color::Black).bg(Color::White)),
            Span::styled(" Environment │ ", Style::default().fg(Color::LightGreen)),
            Span::styled(" [Space] ", Style::default().fg(Color::Black).bg(Color::White)),
            Span::raw(" Bypass │ "),
            Span::styled(" [Q] ", Style::default().fg(Color::Black).bg(Color::White)),
            Span::raw(" Quit"),
        ]);

        let status_line = Line::from(vec![
            Span::styled(" Status: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&self.status_message, Style::default().fg(Color::Yellow)),
        ]);

        let footer_widget = Paragraph::new(vec![help_text, status_line])
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Color::Cyan)));

        f.render_widget(footer_widget, area);
    }
}
