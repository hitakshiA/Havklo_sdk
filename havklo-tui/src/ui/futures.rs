//! Futures funding rates view

use crate::app::{App, Theme};
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render(frame: &mut Frame, _app: &mut App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(" PERPETUAL FUNDING RATES ", Style::default().fg(Theme::FG).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BORDER));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),   // Header
            Constraint::Min(10),     // Table
            Constraint::Length(3),   // Footer
        ])
        .split(inner);

    // Header row
    let header = Line::from(vec![
        Span::styled(format!("{:<14}", "PRODUCT"), Style::default().fg(Theme::MUTED).bold()),
        Span::styled(format!("{:>14}", "MARK PRICE"), Style::default().fg(Theme::MUTED).bold()),
        Span::styled(format!("{:>12}", "FUNDING"), Style::default().fg(Theme::MUTED).bold()),
        Span::styled(format!("{:>12}", "ANNUAL"), Style::default().fg(Theme::MUTED).bold()),
        Span::styled(format!("{:>12}", "PREMIUM"), Style::default().fg(Theme::MUTED).bold()),
        Span::styled(format!("{:>14}", ""), Style::default()),
    ]);
    frame.render_widget(Paragraph::new(header), layout[0]);

    // Separator
    let sep = "─".repeat(layout[0].width as usize);
    frame.render_widget(
        Paragraph::new(sep.clone()).style(Style::default().fg(Theme::BORDER)),
        Rect::new(layout[0].x, layout[0].y + 1, layout[0].width, 1)
    );

    // Table rows
    let rows_area = layout[1];
    let row_height = 3u16;

    // Simulated futures data (would come from real connection)
    let futures_data = [
        ("PI_XBTUSD", 98456.00, 0.0100, 10.95, 0.02, true),
        ("PI_ETHUSD", 3456.00, -0.0050, -5.48, -0.01, false),
        ("PF_SOLUSD", 198.45, 0.0200, 21.90, 0.05, true),
    ];

    for (i, (product, mark, funding, annual, premium, longs_pay)) in futures_data.iter().enumerate() {
        let y = rows_area.y + (i as u16 * row_height);
        if y + row_height > rows_area.y + rows_area.height {
            break;
        }

        let row_area = Rect::new(rows_area.x, y, rows_area.width, row_height);
        render_futures_row(frame, product, *mark, *funding, *annual, *premium, *longs_pay, row_area);
    }

    // Footer with countdown and OI
    let footer = Line::from(vec![
        Span::styled("Next Funding: ", Style::default().fg(Theme::MUTED)),
        Span::styled("02:34:56", Style::default().fg(Theme::HIGHLIGHT).bold()),
        Span::raw("   │   "),
        Span::styled("Total OI: ", Style::default().fg(Theme::MUTED)),
        Span::styled("$2.4B", Style::default().fg(Theme::FG).bold()),
    ]);
    frame.render_widget(Paragraph::new(footer).alignment(Alignment::Center), layout[2]);
}

#[allow(clippy::too_many_arguments)]
fn render_futures_row(
    frame: &mut Frame,
    product: &str,
    mark_price: f64,
    funding: f64,
    annual: f64,
    premium: f64,
    longs_pay: bool,
    area: Rect,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    // Main row
    let funding_color = if funding >= 0.0 { Theme::BID } else { Theme::ASK };
    let annual_color = if annual >= 0.0 { Theme::BID } else { Theme::ASK };
    let premium_color = if premium >= 0.0 { Theme::BID } else { Theme::ASK };

    let pay_text = if longs_pay { "LONGS PAY" } else { "SHORTS PAY" };
    let pay_color = if longs_pay { Theme::ASK } else { Theme::BID };

    let main_row = Line::from(vec![
        Span::styled(format!("{:<14}", product), Style::default().fg(Theme::ACCENT).bold()),
        Span::styled(format!("${:>13.2}", mark_price), Style::default().fg(Theme::FG)),
        Span::styled(format!("{:>+11.4}%", funding), Style::default().fg(funding_color)),
        Span::styled(format!("{:>+11.2}%", annual), Style::default().fg(annual_color)),
        Span::styled(format!("{:>+11.2}%", premium), Style::default().fg(premium_color)),
        Span::raw("  "),
    ]);
    frame.render_widget(Paragraph::new(main_row), layout[0]);

    // Sparkline and pay info row
    let sparkline = "▁▂▃▄▅▆▇█▇▆▅";
    let funding_bar_width = 10;
    let funding_filled = ((funding.abs() * 100.0) as usize).min(funding_bar_width);
    let funding_bar = format!(
        "{}{}",
        "█".repeat(funding_filled),
        "░".repeat(funding_bar_width - funding_filled)
    );

    let detail_row = Line::from(vec![
        Span::raw("              "),
        Span::styled(sparkline, Style::default().fg(Theme::MUTED)),
        Span::raw("  "),
        Span::styled(&funding_bar, Style::default().fg(funding_color)),
        Span::raw("   "),
        Span::styled(pay_text, Style::default().fg(pay_color).bold()),
    ]);
    frame.render_widget(Paragraph::new(detail_row), layout[1]);
}
