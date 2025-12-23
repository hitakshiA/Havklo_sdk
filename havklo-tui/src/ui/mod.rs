//! UI rendering components

mod header;
mod footer;
mod splash;
mod orderbook;
mod dashboard;
mod imbalance;
mod futures;
mod alerts;

use crate::app::{App, Tab, Theme};
use ratatui::prelude::*;
use ratatui::widgets::*;

/// Main render function - routes to splash or main UI
pub fn render(frame: &mut Frame, app: &mut App) {
    if app.show_splash {
        splash::render(frame, app);
    } else {
        render_main(frame, app);
    }
}

fn render_main(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Main layout: header, tabs, content, footer
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(3),  // Tabs
            Constraint::Min(10),    // Content
            Constraint::Length(3),  // Footer
        ])
        .split(area);

    // Render components
    header::render(frame, app, layout[0]);
    render_tabs(frame, app, layout[1]);
    render_content(frame, app, layout[2]);
    footer::render(frame, app, layout[3]);
}

fn render_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let tabs: Vec<Line> = Tab::all()
        .iter()
        .enumerate()
        .map(|(i, tab)| {
            let num = format!("[{}] ", i + 1);
            let title = tab.title();

            if *tab == app.current_tab {
                Line::from(vec![
                    Span::styled(num, Style::default().fg(Theme::MUTED)),
                    Span::styled(title, Style::default().fg(Theme::ACCENT).bold()),
                ])
            } else {
                Line::from(vec![
                    Span::styled(num, Style::default().fg(Theme::MUTED)),
                    Span::styled(title, Style::default().fg(Theme::FG)),
                ])
            }
        })
        .collect();

    let tabs_line = tabs.into_iter()
        .flat_map(|line| {
            let mut spans: Vec<Span> = line.spans;
            spans.push(Span::raw("   "));
            spans
        })
        .collect::<Vec<_>>();

    let tabs_widget = Paragraph::new(Line::from(tabs_line))
        .style(Style::default().bg(Theme::BG))
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Theme::BORDER))
        );

    frame.render_widget(tabs_widget, area);
}

fn render_content(frame: &mut Frame, app: &mut App, area: Rect) {
    match app.current_tab {
        Tab::Orderbook => orderbook::render(frame, app, area),
        Tab::Dashboard => dashboard::render(frame, app, area),
        Tab::Imbalance => imbalance::render(frame, app, area),
        Tab::Futures => futures::render(frame, app, area),
        Tab::Alerts => alerts::render(frame, app, area),
    }
}
