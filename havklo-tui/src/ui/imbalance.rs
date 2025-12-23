//! Imbalance analyzer view with large gauge

use crate::app::{App, Theme};
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(" MARKET IMBALANCE ANALYZER ", Style::default().fg(Theme::FG).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),   // Title
            Constraint::Length(7),   // Gauge
            Constraint::Length(2),   // Spacer
            Constraint::Length(5),   // History
            Constraint::Min(5),      // Info
        ])
        .split(inner);

    // Title
    let title = Paragraph::new("MARKET IMBALANCE GAUGE")
        .style(Style::default().fg(Theme::FG).bold())
        .alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    // Large gauge
    render_large_gauge(frame, app.imbalance, layout[1]);

    // History sparkline
    render_history(frame, &app.imbalance_history, layout[3]);

    // Info section
    render_info(frame, app, layout[4]);
}

fn render_large_gauge(frame: &mut Frame, imbalance: f64, area: Rect) {
    let gauge_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER));

    let inner = gauge_block.inner(area);
    frame.render_widget(gauge_block, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),  // Labels
            Constraint::Length(1),  // Gauge bar
            Constraint::Length(1),  // Value
        ])
        .split(inner);

    // Labels: SELL - BUY
    let labels = Line::from(vec![
        Span::styled("SELL", Style::default().fg(Theme::ASK).bold()),
        Span::styled(" ◀", Style::default().fg(Theme::ASK)),
        Span::raw("━".repeat((layout[0].width / 2 - 8) as usize)),
        Span::styled("●", Style::default().fg(Theme::HIGHLIGHT)),
        Span::raw("━".repeat((layout[0].width / 2 - 8) as usize)),
        Span::styled("▶ ", Style::default().fg(Theme::BID)),
        Span::styled("BUY", Style::default().fg(Theme::BID).bold()),
    ]);
    frame.render_widget(Paragraph::new(labels).alignment(Alignment::Center), layout[0]);

    // Gauge bar
    let gauge_width = layout[1].width as usize - 4;
    let center = gauge_width / 2;
    let position = ((imbalance + 1.0) / 2.0 * gauge_width as f64) as usize;

    let mut bar = String::new();
    for i in 0..gauge_width {
        if i == position {
            bar.push('●');
        } else if i < center && i >= position {
            bar.push('▓');
        } else if i > center && i <= position {
            bar.push('▓');
        } else {
            bar.push('░');
        }
    }

    let bar_color = if imbalance > 0.2 {
        Theme::BID
    } else if imbalance < -0.2 {
        Theme::ASK
    } else {
        Theme::MUTED
    };

    let gauge_line = Line::from(Span::styled(bar, Style::default().fg(bar_color)));
    frame.render_widget(Paragraph::new(gauge_line).alignment(Alignment::Center), layout[1]);

    // Value and label
    let pressure = if imbalance > 0.2 {
        ("BUY PRESSURE", Theme::BID)
    } else if imbalance < -0.2 {
        ("SELL PRESSURE", Theme::ASK)
    } else {
        ("NEUTRAL", Theme::FG)
    };

    let value_line = Line::from(vec![
        Span::styled("-1.0", Style::default().fg(Theme::MUTED)),
        Span::raw("        "),
        Span::styled(format!("{:+.2}", imbalance), Style::default().fg(Theme::HIGHLIGHT).bold()),
        Span::raw("  "),
        Span::styled(pressure.0, Style::default().fg(pressure.1).bold()),
        Span::raw("        "),
        Span::styled("+1.0", Style::default().fg(Theme::MUTED)),
    ]);
    frame.render_widget(Paragraph::new(value_line).alignment(Alignment::Center), layout[2]);
}

fn render_history(frame: &mut Frame, history: &std::collections::VecDeque<f64>, area: Rect) {
    let block = Block::default()
        .title(Span::styled(" IMBALANCE HISTORY (30s) ", Style::default().fg(Theme::MUTED)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Generate sparkline from history
    let sparkline: String = history.iter().rev().take(inner.width as usize).rev().map(|&v| {
        let normalized = (v + 1.0) / 2.0;
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
    }).collect();

    let spark = Paragraph::new(sparkline)
        .style(Style::default().fg(Theme::ACCENT))
        .alignment(Alignment::Center);
    frame.render_widget(spark, inner);
}

fn render_info(frame: &mut Frame, app: &App, area: Rect) {
    let symbol = app.selected_symbol();

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Queue position simulation
    let queue_block = Block::default()
        .title(Span::styled(" QUEUE SIMULATION ", Style::default().fg(Theme::MUTED)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER));

    let queue_inner = queue_block.inner(layout[0]);
    frame.render_widget(queue_block, layout[0]);

    let queue_info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Position: ", Style::default().fg(Theme::MUTED)),
            Span::styled("#3", Style::default().fg(Theme::FG).bold()),
        ]),
        Line::from(vec![
            Span::styled("Ahead: ", Style::default().fg(Theme::MUTED)),
            Span::styled("2.45 BTC", Style::default().fg(Theme::FG)),
        ]),
        Line::from(vec![
            Span::styled("Fill Prob: ", Style::default().fg(Theme::MUTED)),
            Span::styled("67%", Style::default().fg(Theme::BID).bold()),
        ]),
    ]);
    frame.render_widget(queue_info, queue_inner);

    // Market info
    let info_block = Block::default()
        .title(Span::styled(" MARKET INFO ", Style::default().fg(Theme::MUTED)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER));

    let info_inner = info_block.inner(layout[1]);
    frame.render_widget(info_block, layout[1]);

    let ob_data = app.orderbooks.get(symbol);
    let bid_vol: f64 = ob_data.map(|d| {
        d.bids.iter().take(5).map(|(_, q)| q.to_string().parse::<f64>().unwrap_or(0.0)).sum()
    }).unwrap_or(0.0);
    let ask_vol: f64 = ob_data.map(|d| {
        d.asks.iter().take(5).map(|(_, q)| q.to_string().parse::<f64>().unwrap_or(0.0)).sum()
    }).unwrap_or(0.0);

    let info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Bid Vol (5): ", Style::default().fg(Theme::MUTED)),
            Span::styled(format!("{:.2}", bid_vol), Style::default().fg(Theme::BID)),
        ]),
        Line::from(vec![
            Span::styled("Ask Vol (5): ", Style::default().fg(Theme::MUTED)),
            Span::styled(format!("{:.2}", ask_vol), Style::default().fg(Theme::ASK)),
        ]),
        Line::from(vec![
            Span::styled("Symbol: ", Style::default().fg(Theme::MUTED)),
            Span::styled(symbol, Style::default().fg(Theme::ACCENT)),
        ]),
    ]);
    frame.render_widget(info, info_inner);
}
