//! Alerts view with active alerts and history

use crate::app::{App, Theme};
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(" PRICE ALERT SYSTEM ", Style::default().fg(Theme::FG).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(app.alerts.len() as u16 + 4),  // Active alerts
            Constraint::Min(5),                               // History
            Constraint::Length(3),                            // Controls
        ])
        .split(inner);

    render_active_alerts(frame, app, layout[0]);
    render_history(frame, app, layout[1]);
    render_controls(frame, layout[2]);
}

fn render_active_alerts(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(" ACTIVE ALERTS ", Style::default().fg(Theme::HIGHLIGHT)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = Vec::new();
    for alert in &app.alerts {
        let status_icon = if alert.triggered { "●" } else { "◉" };
        let status_color = if alert.triggered { Theme::SUCCESS } else { Theme::HIGHLIGHT };
        let status_text = if alert.triggered { "TRIGGERED" } else { "WATCHING" };

        let line = Line::from(vec![
            Span::styled(format!("  {} ", status_icon), Style::default().fg(status_color)),
            Span::styled(&alert.symbol, Style::default().fg(Theme::ACCENT).bold()),
            Span::styled(format!(" {} ", alert.condition), Style::default().fg(Theme::FG)),
            Span::raw("          "),
            Span::styled(format!("Status: {}", status_text), Style::default().fg(Theme::MUTED)),
        ]);
        lines.push(line);
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No active alerts. Press [A] to add one.",
            Style::default().fg(Theme::MUTED)
        )));
    }

    let alerts_widget = Paragraph::new(lines);
    frame.render_widget(alerts_widget, inner);
}

fn render_history(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(" ALERT HISTORY ", Style::default().fg(Theme::MUTED)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = Vec::new();

    for event in app.alert_history.iter().take(inner.height as usize) {
        let timestamp = event.timestamp.format("%H:%M:%S").to_string();

        let line = if event.message.contains("Session started") {
            Line::from(vec![
                Span::styled(format!("  {} ", timestamp), Style::default().fg(Theme::MUTED)),
                Span::styled("── ", Style::default().fg(Theme::BORDER)),
                Span::styled(&event.message, Style::default().fg(Theme::MUTED)),
                Span::styled(" ──", Style::default().fg(Theme::BORDER)),
            ])
        } else {
            Line::from(vec![
                Span::styled(format!("  {} ", timestamp), Style::default().fg(Theme::MUTED)),
                Span::styled("🔔 ", Style::default()),
                Span::styled(&event.message, Style::default().fg(Theme::HIGHLIGHT)),
            ])
        };
        lines.push(line);
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No alerts triggered yet",
            Style::default().fg(Theme::MUTED)
        )));
    }

    let history_widget = Paragraph::new(lines);
    frame.render_widget(history_widget, inner);
}

fn render_controls(frame: &mut Frame, area: Rect) {
    let controls = Line::from(vec![
        Span::styled("[A]", Style::default().fg(Theme::ACCENT).bold()),
        Span::styled(" Add Alert  ", Style::default().fg(Theme::MUTED)),
        Span::styled("[D]", Style::default().fg(Theme::ACCENT).bold()),
        Span::styled(" Delete  ", Style::default().fg(Theme::MUTED)),
        Span::styled("[C]", Style::default().fg(Theme::ACCENT).bold()),
        Span::styled(" Clear History", Style::default().fg(Theme::MUTED)),
    ]);

    let controls_widget = Paragraph::new(controls)
        .alignment(Alignment::Center)
        .block(Block::default().padding(Padding::new(0, 0, 1, 0)));
    frame.render_widget(controls_widget, area);
}
