//! Imbalance gauge widget

#![allow(dead_code)]

use ratatui::prelude::*;
use ratatui::widgets::Widget;

pub struct ImbalanceGauge {
    value: f64,  // -1.0 to 1.0
}

impl ImbalanceGauge {
    pub fn new(value: f64) -> Self {
        Self {
            value: value.clamp(-1.0, 1.0),
        }
    }
}

impl Widget for ImbalanceGauge {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let width = area.width as usize;
        let center = width / 2;
        let position = ((self.value + 1.0) / 2.0 * width as f64) as usize;

        let bar_color = if self.value > 0.2 {
            Color::Rgb(0, 255, 136)  // Green (buy pressure)
        } else if self.value < -0.2 {
            Color::Rgb(255, 68, 68)  // Red (sell pressure)
        } else {
            Color::Rgb(74, 74, 74)   // Gray (neutral)
        };

        for x in 0..width {
            let char = if x == position {
                "●"
            } else if (x < center && x >= position) || (x > center && x <= position) {
                "▓"
            } else {
                "░"
            };

            let style = if x == position {
                Style::default().fg(Color::Rgb(255, 215, 0))  // Gold for pointer
            } else {
                Style::default().fg(bar_color)
            };

            buf.set_string(area.x + x as u16, area.y, char, style);
        }
    }
}
