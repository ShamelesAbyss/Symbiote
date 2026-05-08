mod app;
mod automata;
mod cluster;
mod density;
mod ecology;
mod field;
mod life;
mod memory;
mod particle;
mod pattern;
mod render;
mod sim;
mod species;
mod tree;

use anyhow::Result;
use app::App;
use crossterm::{
    event::{self, Event, KeyCode, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io,
    time::{Duration, Instant},
};

fn main() -> Result<()> {
    let _ = pattern::bootstrap_pattern_layer(0);

    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| render::draw(f, &app))?;

        if event::poll(Duration::from_millis(10))? {
            match event::read()? {
                Event::Key(key) => match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char(' ') => app.paused = !app.paused,
                    KeyCode::Char('r') => app.reset_particles(),
                    KeyCode::Char('n') => app.randomize_world(),
                    KeyCode::Char('+') | KeyCode::Char('=') => app.speed_up(),
                    KeyCode::Char('-') => app.slow_down(),
                    KeyCode::Left => app.pan_left(),
                    KeyCode::Right => app.pan_right(),
                    KeyCode::Up => app.pan_up(),
                    KeyCode::Down => app.pan_down(),
                    KeyCode::Char('0') => app.reset_camera(),
                    _ => {}
                },
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollUp => app.zoom_in(),
                    MouseEventKind::ScrollDown => app.zoom_out(),
                    _ => {}
                },
                _ => {}
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
    execute!(
        terminal.backend_mut(),
        crossterm::event::DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    Ok(())
}
