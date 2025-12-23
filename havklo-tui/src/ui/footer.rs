//! Footer component with keybindings

use crate::app::{App, Theme};
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render(frame: &mut Frame, _app: &App, area: Rect) {
    let keybindings = vec![
        ("Q", "Quit"),
        ("←→", "Symbol"),
        ("1-5", "View"),
        ("Tab", "Next"),
        ("Space", "Pause"),
        ("R", "Reconnect"),
        ("?", "Help"),
    ];

    let spans: Vec<Span> = keybindings
        .into_iter()
        .flat_map(|(key, action)| {
            vec![
                Span::styled(key, Style::default().fg(Theme::ACCENT).bold()),
                Span::styled(format!(" {}  ", action), Style::default().fg(Theme::MUTED)),
            ]
        })
        .collect();

    let help = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Theme::BORDER))
                .padding(Padding::new(0, 0, 1, 0))
        );

    frame.render_widget(help, area);
}
