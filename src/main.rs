mod app;
mod cluster;
mod memory;
mod particle;
mod render;
mod sim;

use anyhow::Result;
use app::App;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io,
    time::{Duration, Instant},
};

fn main() -> Result<()> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| render::draw(f, &app))?;

        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char(' ') => app.paused = !app.paused,
                    KeyCode::Char('r') => app.reset_particles(),
                    KeyCode::Char('m') => app.force_mutation(),
                    KeyCode::Char('n') => app.randomize_world(),
                    KeyCode::Char('e') => app.toggle_evolution(),
                    KeyCode::Char('+') | KeyCode::Char('=') => app.speed_up(),
                    KeyCode::Char('-') => app.slow_down(),
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= Duration::from_millis(app.tick_ms) {
            if !app.paused {
                app.step();
            }

            last_tick = Instant::now();
        }
    }

    app.save_all();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
