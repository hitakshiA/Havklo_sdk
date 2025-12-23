//! Orderbook view with depth chart visualization

use crate::app::{App, Theme};
use ratatui::prelude::*;
use ratatui::widgets::*;
use rust_decimal::Decimal;

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    // Split into main content and sidebar
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(65),  // Orderbook
            Constraint::Percentage(35),  // Stats sidebar
        ])
        .split(area);

    render_orderbook(frame, app, layout[0]);
    render_sidebar(frame, app, layout[1]);
}

fn render_orderbook(frame: &mut Frame, app: &App, area: Rect) {
    let symbol = app.selected_symbol();
    let ob_data = app.orderbooks.get(symbol);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2),  // Title
            Constraint::Min(5),     // Depth chart
            Constraint::Length(2),  // Symbol selector
        ])
        .split(area);

    // Block border
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER))
        .style(Style::default().bg(Theme::BG));
    frame.render_widget(block, area);

    // Title with sync indicator
    let synced = app.symbol_data.get(symbol).map(|d| d.synced).unwrap_or(false);
    let sync_icon = if synced { "●" } else { "○" };
    let sync_color = if synced { Theme::SUCCESS } else { Theme::MUTED };

    let title = Paragraph::new(Line::from(vec![
        Span::styled(symbol, Style::default().fg(Theme::FG).bold()),
        Span::raw("  "),
        Span::styled(sync_icon, Style::default().fg(sync_color)),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(title, inner[0]);

    // Depth chart
    if let Some(data) = ob_data {
        render_depth_chart(frame, data, inner[1]);
    } else {
        let loading = Paragraph::new("Waiting for data...")
            .style(Style::default().fg(Theme::MUTED))
            .alignment(Alignment::Center);
        frame.render_widget(loading, inner[1]);
    }

    // Symbol selector
    let symbols: Vec<Span> = app.symbols.iter().enumerate().map(|(i, s)| {
        let short = s.split('/').next().unwrap_or(s);
        if i == app.selected_symbol_idx {
            Span::styled(format!(" {} ", short), Style::default().fg(Theme::ACCENT).bold())
        } else {
            Span::styled(format!(" {} ", short), Style::default().fg(Theme::MUTED))
        }
    }).collect();

    let mut selector_spans = vec![Span::styled("◀ ", Style::default().fg(Theme::MUTED))];
    selector_spans.extend(symbols);
    selector_spans.push(Span::styled(" ▶", Style::default().fg(Theme::MUTED)));

    let selector = Paragraph::new(Line::from(selector_spans))
        .alignment(Alignment::Center);
    frame.render_widget(selector, inner[2]);
}

fn render_depth_chart(frame: &mut Frame, data: &crate::app::OrderbookData, area: Rect) {
    let levels_to_show = ((area.height - 2) / 2) as usize;
    let bar_width = area.width.saturating_sub(25) as usize;

    // Find max quantity for scaling
    let max_qty = data.bids.iter()
        .chain(data.asks.iter())
        .take(levels_to_show)
        .map(|(_, q)| *q)
        .max()
        .unwrap_or(Decimal::ONE);

    let mut lines = Vec::new();

    // ASKS header
    lines.push(Line::from(Span::styled("   ASKS", Style::default().fg(Theme::ASK).bold())));

    // Asks (reversed - show from spread outward)
    let asks: Vec<_> = data.asks.iter().take(levels_to_show).collect();
    for (price, qty) in asks.iter().rev() {
        let bar_len = if !max_qty.is_zero() {
            ((*qty / max_qty) * Decimal::from(bar_width as u32))
                .to_string()
                .parse::<usize>()
                .unwrap_or(0)
                .min(bar_width)
        } else { 0 };

        let bar = format!("{:>width$}", "▓".repeat(bar_len), width = bar_width);
        let line = Line::from(vec![
            Span::styled(bar, Style::default().fg(Theme::ASK)),
            Span::raw("  "),
            Span::styled(format!("{:.4}", qty), Style::default().fg(Theme::FG)),
            Span::raw("  "),
            Span::styled(format!("${:.2}", price), Style::default().fg(Theme::ASK)),
        ]);
        lines.push(line);
    }

    // Spread line
    let spread_str = data.spread
        .map(|s| format!("━━━━━━━━━━ SPREAD ${:.2} ━━━━━━━━━━", s))
        .unwrap_or_else(|| "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string());
    lines.push(Line::from(Span::styled(spread_str, Style::default().fg(Theme::HIGHLIGHT))));

    // Bids
    for (price, qty) in data.bids.iter().take(levels_to_show) {
        let bar_len = if !max_qty.is_zero() {
            ((*qty / max_qty) * Decimal::from(bar_width as u32))
                .to_string()
                .parse::<usize>()
                .unwrap_or(0)
                .min(bar_width)
        } else { 0 };

        let bar = format!("{:<width$}", "▓".repeat(bar_len), width = bar_width);
        let line = Line::from(vec![
            Span::styled(bar, Style::default().fg(Theme::BID)),
            Span::raw("  "),
            Span::styled(format!("{:.4}", qty), Style::default().fg(Theme::FG)),
            Span::raw("  "),
            Span::styled(format!("${:.2}", price), Style::default().fg(Theme::BID)),
        ]);
        lines.push(line);
    }

    // BIDS footer
    lines.push(Line::from(Span::styled("   BIDS", Style::default().fg(Theme::BID).bold())));

    let depth = Paragraph::new(lines);
    frame.render_widget(depth, area);
}

fn render_sidebar(frame: &mut Frame, app: &App, area: Rect) {
    let symbol = app.selected_symbol();
    let ob_data = app.orderbooks.get(symbol);
    let sym_data = app.symbol_data.get(symbol);

    let block = Block::default()
        .title(Span::styled(" MARKET METRICS ", Style::default().fg(Theme::FG).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // Spread
            Constraint::Length(3),  // Mid price
            Constraint::Length(3),  // Imbalance
            Constraint::Length(1),  // Separator
            Constraint::Length(4),  // VWAP
            Constraint::Length(1),  // Separator
            Constraint::Min(3),     // Stats
        ])
        .split(inner);

    // Spread
    let spread = ob_data.and_then(|d| d.spread);
    let spread_text = spread.map(|s| format!("${:.2}", s)).unwrap_or("-".to_string());
    let spread_bps = spread.and_then(|s| {
        ob_data.and_then(|d| d.mid_price).map(|m| {
            if !m.is_zero() { (s / m * Decimal::from(10000)).to_string() } else { "-".to_string() }
        })
    }).unwrap_or("-".to_string());

    let spread_widget = Paragraph::new(vec![
        Line::from(Span::styled("Spread", Style::default().fg(Theme::MUTED))),
        Line::from(vec![
            Span::styled(&spread_text, Style::default().fg(Theme::HIGHLIGHT).bold()),
            Span::styled(format!("  {} bps", spread_bps), Style::default().fg(Theme::MUTED)),
        ]),
    ]);
    frame.render_widget(spread_widget, layout[0]);

    // Mid price with sparkline
    let mid = ob_data.and_then(|d| d.mid_price);
    let mid_text = mid.map(|m| format!("${:.2}", m)).unwrap_or("-".to_string());
    let change = sym_data.map(|d| d.change_pct).unwrap_or(0.0);
    let change_color = if change > 0.0 { Theme::BID } else if change < 0.0 { Theme::ASK } else { Theme::FG };

    // Simple sparkline from price history
    let sparkline = sym_data.map(|d| {
        if d.price_history.is_empty() { return "".to_string(); }
        let min = d.price_history.iter().min().cloned().unwrap_or(Decimal::ZERO);
        let max = d.price_history.iter().max().cloned().unwrap_or(Decimal::ONE);
        let range = max - min;
        if range.is_zero() { return "▄".repeat(d.price_history.len().min(15)); }

        d.price_history.iter().rev().take(15).rev().map(|p| {
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

    let mid_widget = Paragraph::new(vec![
        Line::from(Span::styled("Mid Price", Style::default().fg(Theme::MUTED))),
        Line::from(vec![
            Span::styled(&mid_text, Style::default().fg(Theme::FG).bold()),
            Span::raw("  "),
            Span::styled(&sparkline, Style::default().fg(Theme::ACCENT)),
            Span::styled(format!(" {:+.2}%", change), Style::default().fg(change_color)),
        ]),
    ]);
    frame.render_widget(mid_widget, layout[1]);

    // Imbalance gauge
    let imbalance = app.imbalance;
    let imbalance_visual = render_imbalance_mini(imbalance);
    let pressure = if imbalance > 0.2 { "BUY" } else if imbalance < -0.2 { "SELL" } else { "NEUTRAL" };
    let pressure_color = if imbalance > 0.2 { Theme::BID } else if imbalance < -0.2 { Theme::ASK } else { Theme::FG };

    let imbalance_widget = Paragraph::new(vec![
        Line::from(Span::styled("Imbalance", Style::default().fg(Theme::MUTED))),
        Line::from(vec![
            Span::styled(&imbalance_visual, Style::default().fg(Theme::ACCENT)),
            Span::styled(format!(" {:+.2}", imbalance), Style::default().fg(Theme::FG)),
        ]),
        Line::from(Span::styled(pressure, Style::default().fg(pressure_color).bold())),
    ]);
    frame.render_widget(imbalance_widget, layout[2]);

    // Separator
    let sep_text = "─".repeat(layout[3].width as usize);
    let sep = Paragraph::new(sep_text.clone())
        .style(Style::default().fg(Theme::BORDER));
    frame.render_widget(sep, layout[3]);

    // VWAP impact (simulated)
    let vwap_widget = Paragraph::new(vec![
        Line::from(Span::styled("VWAP IMPACT (1.0)", Style::default().fg(Theme::MUTED))),
        Line::from(vec![
            Span::styled("Buy  ", Style::default().fg(Theme::MUTED)),
            Span::styled(mid.map(|m| format!("${:.2}", m + Decimal::from(2))).unwrap_or("-".to_string()), Style::default().fg(Theme::FG)),
            Span::styled("  ▲$2.00", Style::default().fg(Theme::BID)),
        ]),
        Line::from(vec![
            Span::styled("Sell ", Style::default().fg(Theme::MUTED)),
            Span::styled(mid.map(|m| format!("${:.2}", m - Decimal::from(2))).unwrap_or("-".to_string()), Style::default().fg(Theme::FG)),
            Span::styled("  ▼$2.00", Style::default().fg(Theme::ASK)),
        ]),
    ]);
    frame.render_widget(vwap_widget, layout[4]);

    // Separator
    let sep2 = Paragraph::new(sep_text.clone())
        .style(Style::default().fg(Theme::BORDER));
    frame.render_widget(sep2, layout[5]);

    // Stats
    let update_count = ob_data.map(|d| d.update_count).unwrap_or(0);
    let checksum_ok = ob_data.map(|d| d.checksum_valid).unwrap_or(false);
    let checksum_icon = if checksum_ok { "✓" } else { "✗" };
    let checksum_color = if checksum_ok { Theme::SUCCESS } else { Theme::ASK };

    let stats_widget = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(checksum_icon, Style::default().fg(checksum_color)),
            Span::styled(" Checksum Valid", Style::default().fg(Theme::MUTED)),
        ]),
        Line::from(vec![
            Span::styled("✓", Style::default().fg(Theme::SUCCESS)),
            Span::styled(format!(" {} updates", update_count), Style::default().fg(Theme::MUTED)),
        ]),
    ]);
    frame.render_widget(stats_widget, layout[6]);
}

fn render_imbalance_mini(imbalance: f64) -> String {
    let filled = ((imbalance + 1.0) / 2.0 * 5.0) as usize;
    let empty = 5 - filled.min(5);
    format!("{}{}", "▰".repeat(filled.min(5)), "▱".repeat(empty))
}
