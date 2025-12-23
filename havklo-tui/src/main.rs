//! Havklo TUI - Stunning terminal interface for Kraken market data
//!
//! Run with: cargo run -p havklo-tui

mod app;
mod data;
mod ui;
mod widgets;

use anyhow::Result;
use app::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io::stdout;
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the app
    let result = run_app(&mut terminal).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {e}");
    }

    Ok(())
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>) -> Result<()> {
    let mut app = App::new();

    // Show splash screen first
    app.show_splash = true;
    let splash_start = Instant::now();

    // Start connecting in background
    let _connect_handle = tokio::spawn(async {
        // Small delay to show splash
        tokio::time::sleep(Duration::from_millis(500)).await;
    });

    let tick_rate = Duration::from_millis(16); // ~60 FPS
    let mut last_tick = Instant::now();

    loop {
        // Render
        terminal.draw(|f| ui::render(f, &mut app))?;

        // Handle input
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            if !app.show_splash {
                                return Ok(());
                            }
                        }
                        KeyCode::Char('1') => app.current_tab = app::Tab::Orderbook,
                        KeyCode::Char('2') => app.current_tab = app::Tab::Dashboard,
                        KeyCode::Char('3') => app.current_tab = app::Tab::Imbalance,
                        KeyCode::Char('4') => app.current_tab = app::Tab::Futures,
                        KeyCode::Char('5') => app.current_tab = app::Tab::Alerts,
                        KeyCode::Tab => app.next_tab(),
                        KeyCode::BackTab => app.prev_tab(),
                        KeyCode::Left => app.prev_symbol(),
                        KeyCode::Right => app.next_symbol(),
                        KeyCode::Char(' ') => app.toggle_pause(),
                        KeyCode::Char('r') | KeyCode::Char('R') => app.reconnect(),
                        _ => {}
                    }
                }
            }
        }

        // Update tick
        if last_tick.elapsed() >= tick_rate {
            app.tick();
            last_tick = Instant::now();
        }

        // Transition from splash after delay
        if app.show_splash && splash_start.elapsed() > Duration::from_secs(2) {
            app.show_splash = false;
            // Start actual connection
            app.start_connection().await?;
        }
    }
}
