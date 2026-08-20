//! Spectrum Analyzer & Level Meter Widgets for TUI

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

pub struct SpectrumVisualizer<'a> {
    bins: &'a [f32],
    _title: &'a str,
}

impl<'a> SpectrumVisualizer<'a> {
    pub fn new(bins: &'a [f32], title: &'a str) -> Self {
        Self { bins, _title: title }
    }
}

const BAR_CHARS: [char; 8] = [' ', ' ', '▂', '▃', '▄', '▅', '▆', '▇'];

impl<'a> Widget for SpectrumVisualizer<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 3 {
            return;
        }

        let num_bars = (area.width as usize).min(self.bins.len());
        let height = area.height.saturating_sub(1) as usize;

        for (col, &bin_val) in self.bins.iter().take(num_bars).enumerate() {
            let x = area.left() + col as u16;
            let val = bin_val.clamp(0.0, 1.0);
            let total_steps = height * 8;
            let fill_steps = (val * total_steps as f32) as usize;

            for row in 0..height {
                let y = area.bottom().saturating_sub(2).saturating_sub(row as u16);
                let row_step_start = row * 8;

                let char_to_draw = if fill_steps >= row_step_start + 8 {
                    '█'
                } else if fill_steps > row_step_start {
                    BAR_CHARS[fill_steps - row_step_start]
                } else {
                    ' '
                };

                let color = if row > height * 3 / 4 {
                    Color::LightRed
                } else if row > height / 2 {
                    Color::Yellow
                } else if row > height / 4 {
                    Color::Cyan
                } else {
                    Color::Green
                };

                buf[(x, y)]
                    .set_char(char_to_draw)
                    .set_style(Style::default().fg(color));
            }
        }

        // Bottom frequency labels
        let label_y = area.bottom().saturating_sub(1);
        let labels = ["30Hz", "125Hz", "500Hz", "2kHz", "8kHz", "16k"];
        let step = (area.width as usize) / (labels.len());
        for (i, label) in labels.iter().enumerate() {
            let lx = area.left() + (i * step) as u16;
            if lx + label.len() as u16 <= area.right() {
                for (ch_idx, ch) in label.chars().enumerate() {
                    buf[(lx + ch_idx as u16, label_y)]
                        .set_char(ch)
                        .set_style(Style::default().fg(Color::DarkGray));
                }
            }
        }
    }
}

pub struct VuMeter {
    pub peak_db_l: f32,
    pub peak_db_r: f32,
    pub _rms_db_l: f32,
    pub _rms_db_r: f32,
}

impl VuMeter {
    pub fn render_meter(&self, area: Rect, buf: &mut Buffer) {
        if area.height < 2 || area.width < 10 {
            return;
        }

        let width = (area.width - 6) as usize;
        let left_norm = ((self.peak_db_l + 60.0) / 60.0).clamp(0.0, 1.0);
        let right_norm = ((self.peak_db_r + 60.0) / 60.0).clamp(0.0, 1.0);

        let left_fill = (left_norm * width as f32) as usize;
        let right_fill = (right_norm * width as f32) as usize;

        // Render L channel
        buf[(area.left(), area.top())].set_symbol("L: ").set_style(Style::default().fg(Color::Cyan));
        for i in 0..width {
            let x = area.left() + 3 + i as u16;
            let ch = if i < left_fill { '█' } else { '░' };
            let color = if i > width * 9 / 10 {
                Color::Red
            } else if i > width * 7 / 10 {
                Color::Yellow
            } else {
                Color::Green
            };
            buf[(x, area.top())].set_char(ch).set_style(Style::default().fg(color));
        }

        // Render R channel
        if area.height > 1 {
            buf[(area.left(), area.top() + 1)].set_symbol("R: ").set_style(Style::default().fg(Color::Cyan));
            for i in 0..width {
                let x = area.left() + 3 + i as u16;
                let ch = if i < right_fill { '█' } else { '░' };
                let color = if i > width * 9 / 10 {
                    Color::Red
                } else if i > width * 7 / 10 {
                    Color::Yellow
                } else {
                    Color::Green
                };
                buf[(x, area.top() + 1)].set_char(ch).set_style(Style::default().fg(color));
            }
        }
    }
}
