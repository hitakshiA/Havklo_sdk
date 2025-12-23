//! Horizontal depth bar widget

#![allow(dead_code)]

use ratatui::prelude::*;
use ratatui::widgets::Widget;
use rust_decimal::Decimal;

pub struct DepthBars<'a> {
    levels: &'a [(Decimal, Decimal)],
    max_qty: Decimal,
    is_bids: bool,
}

impl<'a> DepthBars<'a> {
    pub fn new(levels: &'a [(Decimal, Decimal)], is_bids: bool) -> Self {
        let max_qty = levels.iter()
            .map(|(_, q)| *q)
            .max()
            .unwrap_or(Decimal::ONE);

        Self {
            levels,
            max_qty,
            is_bids,
        }
    }
}

impl Widget for DepthBars<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let bar_color = if self.is_bids {
            Color::Rgb(0, 255, 136)  // Green
        } else {
            Color::Rgb(255, 68, 68)  // Red
        };

        for (i, (_price, qty)) in self.levels.iter().enumerate() {
            if i as u16 >= area.height {
                break;
            }

            let y = area.y + i as u16;
            let bar_width = if !self.max_qty.is_zero() {
                ((*qty / self.max_qty) * Decimal::from(area.width as u32))
                    .to_string()
                    .parse::<u16>()
                    .unwrap_or(0)
                    .min(area.width)
            } else {
                0
            };

            // Draw bar
            for x in 0..bar_width {
                buf.set_string(
                    area.x + if self.is_bids { x } else { area.width - 1 - x },
                    y,
                    "▓",
                    Style::default().fg(bar_color),
                );
            }
        }
    }
}
