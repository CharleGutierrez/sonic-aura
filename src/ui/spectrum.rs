//! Real-Time 32-Band Spectrum Analyzer & Stereo Level Meter Widgets

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

pub struct SpectrumVisualizer<'a> {
    bins: &'a [f32],
    peak_hold: &'a [f32],
    _title: &'a str,
}

impl<'a> SpectrumVisualizer<'a> {
    pub fn new(bins: &'a [f32], peak_hold: &'a [f32], title: &'a str) -> Self {
        Self {
            bins,
            peak_hold,
            _title: title,
        }
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
            let fill_steps = (val * total_steps as f32).round() as usize;

            let peak_val = if col < self.peak_hold.len() {
                self.peak_hold[col].clamp(0.0, 1.0)
            } else {
                val
            };
            let peak_row = (peak_val * height as f32).round() as usize;

            for row in 0..height {
                let y = area.bottom().saturating_sub(2).saturating_sub(row as u16);
                let row_step_start = row * 8;

                let char_to_draw = if fill_steps >= row_step_start + 8 {
                    '█'
                } else if fill_steps > row_step_start {
                    BAR_CHARS[fill_steps - row_step_start]
                } else if row + 1 == peak_row && peak_val > 0.05 {
                    '▔' // Peak-hold floating ceiling indicator
                } else {
                    ' '
                };

                let color = if row > height * 4 / 5 {
                    Color::LightRed
                } else if row > height * 3 / 5 {
                    Color::Yellow
                } else if row > height * 2 / 5 {
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
        let labels = ["31Hz", "125Hz", "500Hz", "2kHz", "8kHz", "16k"];
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
    pub rms_db_l: f32,
    pub rms_db_r: f32,
}

impl VuMeter {
    pub fn render_meter(&self, area: Rect, buf: &mut Buffer) {
        if area.height < 2 || area.width < 14 {
            return;
        }

        let meter_width = (area.width.saturating_sub(14)) as usize;
        let left_norm = ((self.peak_db_l + 60.0) / 60.0).clamp(0.0, 1.0);
        let right_norm = ((self.peak_db_r + 60.0) / 60.0).clamp(0.0, 1.0);

        let left_fill = (left_norm * meter_width as f32).round() as usize;
        let right_fill = (right_norm * meter_width as f32).round() as usize;

        // Render L channel
        buf[(area.left(), area.top())].set_symbol("L: ").set_style(Style::default().fg(Color::Cyan));
        for i in 0..meter_width {
            let x = area.left() + 3 + i as u16;
            let ch = if i < left_fill { '█' } else { '░' };
            let color = if i > meter_width * 9 / 10 {
                Color::Red
            } else if i > meter_width * 7 / 10 {
                Color::Yellow
            } else {
                Color::Green
            };
            buf[(x, area.top())].set_char(ch).set_style(Style::default().fg(color));
        }
        let l_db_str = format!("{:>5.1}dB", self.peak_db_l);
        buf.set_string(area.left() + 4 + meter_width as u16, area.top(), l_db_str, Style::default().fg(Color::White));

        // Render R channel
        if area.height > 1 {
            buf[(area.left(), area.top() + 1)].set_symbol("R: ").set_style(Style::default().fg(Color::Cyan));
            for i in 0..meter_width {
                let x = area.left() + 3 + i as u16;
                let ch = if i < right_fill { '█' } else { '░' };
                let color = if i > meter_width * 9 / 10 {
                    Color::Red
                } else if i > meter_width * 7 / 10 {
                    Color::Yellow
                } else {
                    Color::Green
                };
                buf[(x, area.top() + 1)].set_char(ch).set_style(Style::default().fg(color));
            }
            let r_db_str = format!("{:>5.1}dB", self.peak_db_r);
            buf.set_string(area.left() + 4 + meter_width as u16, area.top() + 1, r_db_str, Style::default().fg(Color::White));
        }
    }
}
