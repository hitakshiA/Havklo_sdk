//! Multi-symbol dashboard view

use crate::app::{App, Theme};
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(" MULTI-SYMBOL DASHBOARD ", Style::default().fg(Theme::FG).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Create 2x3 grid for 6 symbols
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    let top_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(1, 3), Constraint::Ratio(1, 3)])
        .split(rows[0]);

    let bottom_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(1, 3), Constraint::Ratio(1, 3)])
        .split(rows[1]);

    let cells = [
        top_row[0], top_row[1], top_row[2],
        bottom_row[0], bottom_row[1], bottom_row[2],
    ];

    for (i, &cell) in cells.iter().enumerate() {
        if i < app.symbols.len() {
            render_symbol_card(frame, app, &app.symbols[i].clone(), cell, i == app.selected_symbol_idx);
        }
    }
}

fn render_symbol_card(frame: &mut Frame, app: &App, symbol: &str, area: Rect, selected: bool) {
    let data = app.symbol_data.get(symbol);
    let synced = data.map(|d| d.synced).unwrap_or(false);

    let border_color = if selected { Theme::ACCENT } else { Theme::BORDER };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .padding(Padding::new(1, 1, 0, 0));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Symbol + status
            Constraint::Length(1),  // Price
            Constraint::Length(1),  // Change
            Constraint::Length(1),  // Sparkline
            Constraint::Length(1),  // Spread
        ])
        .split(inner);

    // Symbol + sync status
    let sync_icon = if synced { "●" } else { "○" };
    let sync_color = if synced { Theme::SUCCESS } else { Theme::MUTED };
    let symbol_line = Line::from(vec![
        Span::styled(symbol, Style::default().fg(Theme::FG).bold()),
        Span::raw("  "),
        Span::styled(sync_icon, Style::default().fg(sync_color)),
    ]);
    frame.render_widget(Paragraph::new(symbol_line), layout[0]);

    // Price
    let price = data.and_then(|d| d.price)
        .map(|p| format!("${:.2}", p))
        .unwrap_or("-".to_string());
    let price_line = Line::from(Span::styled(&price, Style::default().fg(Theme::FG).bold()));
    frame.render_widget(Paragraph::new(price_line), layout[1]);

    // Change percentage
    let change = data.map(|d| d.change_pct).unwrap_or(0.0);
    let change_color = if change > 0.0 { Theme::BID } else if change < 0.0 { Theme::ASK } else { Theme::FG };
    let change_icon = if change > 0.0 { "▲" } else if change < 0.0 { "▼" } else { "─" };
    let change_line = Line::from(vec![
        Span::styled(change_icon, Style::default().fg(change_color)),
        Span::styled(format!(" {:+.2}%", change), Style::default().fg(change_color)),
    ]);
    frame.render_widget(Paragraph::new(change_line), layout[2]);

    // Sparkline
    let sparkline = data.map(|d| {
        if d.price_history.is_empty() { return "".to_string(); }
        let min = d.price_history.iter().min().cloned().unwrap_or(rust_decimal::Decimal::ZERO);
        let max = d.price_history.iter().max().cloned().unwrap_or(rust_decimal::Decimal::ONE);
        let range = max - min;
        if range.is_zero() { return "▄".repeat(d.price_history.len().min(12)); }

        d.price_history.iter().rev().take(12).rev().map(|p| {
            let normalized = ((*p - min) / range).to_string().parse::<f64>().unwrap_or(0.5);
            match (normalized * 8.0) as usize {
                0 => '▁',
                1 => '▂',
                2 => '▃',
                3 => '▄',
                4 => '▅',
                5 => '▆',
                6 => '▇',
                _ => '█',
            }
        }).collect()
    }).unwrap_or_default();

    let sparkline_color = if change >= 0.0 { Theme::BID } else { Theme::ASK };
    let spark_line = Line::from(Span::styled(&sparkline, Style::default().fg(sparkline_color)));
    frame.render_widget(Paragraph::new(spark_line), layout[3]);

    // Spread
    let spread = data.and_then(|d| d.spread)
        .map(|s| format!("Spread: ${:.4}", s))
        .unwrap_or("Spread: -".to_string());
    let spread_line = Line::from(Span::styled(&spread, Style::default().fg(Theme::MUTED)));
    frame.render_widget(Paragraph::new(spread_line), layout[4]);
}
