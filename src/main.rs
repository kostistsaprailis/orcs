mod animal;
mod app;
mod event;
mod orc;
mod pathfinding;
mod render;
mod world;

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self as ct_event, Event as CtEvent, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::App;

fn main() -> io::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut app = App::new();
    let mut last_tick = Instant::now();

    loop {
        // Render
        terminal.draw(|frame| render::render(frame, &mut app))?;

        // Handle input with timeout
        let tick_rate = Duration::from_millis(app.tick_interval_ms());
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::ZERO);

        if ct_event::poll(timeout)? {
            if let CtEvent::Key(key) = ct_event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => {
                            app.should_quit = true;
                        }
                        KeyCode::Char(' ') => app.toggle_pause(),
                        KeyCode::Char('+') | KeyCode::Char('=') => app.speed_up(),
                        KeyCode::Char('-') => app.speed_down(),
                        KeyCode::Up => app.move_cursor(0, -1),
                        KeyCode::Down => app.move_cursor(0, 1),
                        KeyCode::Left => app.move_cursor(-1, 0),
                        KeyCode::Right => app.move_cursor(1, 0),
                        KeyCode::Tab => app.cycle_selected_orc(),
                        KeyCode::Char('f') => app.drop_food(),
                        _ => {}
                    }
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }

        // Tick simulation
        if last_tick.elapsed() >= tick_rate {
            app.tick();
            last_tick = Instant::now();
        }
    }
}
