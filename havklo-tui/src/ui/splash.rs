//! Splash screen with ASCII art logo

use crate::app::{App, ConnectionState, Theme};
use ratatui::prelude::*;
use ratatui::widgets::*;

const LOGO: &str = r#"
    ██╗  ██╗ █████╗ ██╗   ██╗██╗  ██╗██╗      ██████╗
    ██║  ██║██╔══██╗██║   ██║██║ ██╔╝██║     ██╔═══██╗
    ███████║███████║██║   ██║█████╔╝ ██║     ██║   ██║
    ██╔══██║██╔══██║╚██╗ ██╔╝██╔═██╗ ██║     ██║   ██║
    ██║  ██║██║  ██║ ╚████╔╝ ██║  ██╗███████╗╚██████╔╝
    ╚═╝  ╚═╝╚═╝  ╚═╝  ╚═══╝  ╚═╝  ╚═╝╚══════╝ ╚═════╝
"#;

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Clear background
    frame.render_widget(
        Block::default().style(Style::default().bg(Theme::BG)),
        area,
    );

    // Center the content
    let content_height = 15;
    let content_width = 60;

    if area.height < content_height || area.width < content_width {
        // Fallback for small terminals
        let text = Paragraph::new("HAVKLO")
            .style(Style::default().fg(Theme::ACCENT).bold())
            .alignment(Alignment::Center);
        frame.render_widget(text, area);
        return;
    }

    let v_margin = (area.height.saturating_sub(content_height)) / 2;
    let h_margin = (area.width.saturating_sub(content_width)) / 2;

    let centered = Rect::new(
        area.x + h_margin,
        area.y + v_margin,
        content_width.min(area.width),
        content_height.min(area.height),
    );

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),  // Logo
            Constraint::Length(1),  // Spacer
            Constraint::Length(1),  // Tagline
            Constraint::Length(2),  // Spacer
            Constraint::Length(1),  // Status
            Constraint::Length(1),  // Progress bar
        ])
        .split(centered);

    // Logo
    let logo = Paragraph::new(LOGO)
        .style(Style::default().fg(Theme::ACCENT))
        .alignment(Alignment::Center);
    frame.render_widget(logo, layout[0]);

    // Tagline
    let tagline = Paragraph::new("━━━━━ Real-time Kraken Market Data Terminal ━━━━━")
        .style(Style::default().fg(Theme::MUTED))
        .alignment(Alignment::Center);
    frame.render_widget(tagline, layout[2]);

    // Connection status
    let status_text = match app.connection_state {
        ConnectionState::Disconnected => "◉ Initializing...",
        ConnectionState::Connecting => "◉ Connecting to Kraken...",
        ConnectionState::Connected => "◉ Connected!",
        ConnectionState::Reconnecting => "◉ Reconnecting...",
        ConnectionState::Error => "◉ Connection Error",
    };

    let status_color = match app.connection_state {
        ConnectionState::Connected => Theme::SUCCESS,
        ConnectionState::Error => Theme::ASK,
        _ => Theme::HIGHLIGHT,
    };

    let status = Paragraph::new(status_text)
        .style(Style::default().fg(status_color))
        .alignment(Alignment::Center);
    frame.render_widget(status, layout[4]);

    // Progress bar
    let progress_width = (layout[5].width as f64 * 0.6) as u16;
    let filled = (progress_width as f64 * app.splash_progress) as usize;
    let empty = progress_width as usize - filled;

    let progress_bar = format!(
        "{}{}  {:.0}%",
        "▓".repeat(filled),
        "░".repeat(empty),
        app.splash_progress * 100.0
    );

    let progress = Paragraph::new(progress_bar)
        .style(Style::default().fg(Theme::ACCENT))
        .alignment(Alignment::Center);
    frame.render_widget(progress, layout[5]);
}
