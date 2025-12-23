//! Header component with connection status and stats

use crate::app::{App, ConnectionState, Theme};
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20),  // Logo
            Constraint::Min(10),     // Spacer
            Constraint::Length(40),  // Stats
        ])
        .split(area);

    // Logo
    let logo = Paragraph::new(Line::from(vec![
        Span::styled("██ ", Style::default().fg(Theme::ACCENT)),
        Span::styled("HAVKLO", Style::default().fg(Theme::FG).bold()),
    ]))
    .block(Block::default().padding(Padding::new(1, 0, 1, 0)));
    frame.render_widget(logo, layout[0]);

    // Connection status and stats
    let uptime = app.uptime();
    let uptime_str = format!(
        "{:02}:{:02}:{:02}",
        uptime.as_secs() / 3600,
        (uptime.as_secs() % 3600) / 60,
        uptime.as_secs() % 60
    );

    let (status_icon, status_color) = match app.connection_state {
        ConnectionState::Connected => ("●", Theme::SUCCESS),
        ConnectionState::Connecting => ("◐", Theme::HIGHLIGHT),
        ConnectionState::Reconnecting => ("◑", Theme::WARNING),
        ConnectionState::Disconnected => ("○", Theme::MUTED),
        ConnectionState::Error => ("●", Theme::ASK),
    };

    let status_text = match app.connection_state {
        ConnectionState::Connected => "LIVE",
        ConnectionState::Connecting => "CONNECTING",
        ConnectionState::Reconnecting => "RECONNECTING",
        ConnectionState::Disconnected => "OFFLINE",
        ConnectionState::Error => "ERROR",
    };

    let stats = Paragraph::new(Line::from(vec![
        Span::styled(status_icon, Style::default().fg(status_color)),
        Span::raw(" "),
        Span::styled(status_text, Style::default().fg(status_color).bold()),
        Span::raw("   "),
        Span::styled("⏱ ", Style::default().fg(Theme::MUTED)),
        Span::styled(&uptime_str, Style::default().fg(Theme::FG)),
        Span::raw("   "),
        Span::styled("▲", Style::default().fg(Theme::BID)),
        Span::styled(format!("{}/s", app.update_count / uptime.as_secs().max(1)), Style::default().fg(Theme::FG)),
        Span::raw("   "),
        Span::styled(format!("{:.0}fps", app.fps), Style::default().fg(Theme::MUTED)),
    ]))
    .alignment(Alignment::Right)
    .block(Block::default().padding(Padding::new(0, 1, 1, 0)));
    frame.render_widget(stats, layout[2]);

    // Border at bottom
    let border = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Theme::ACCENT));
    frame.render_widget(border, area);
}
