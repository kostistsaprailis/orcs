use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::app::App;
use crate::orc::Activity;
use crate::world::{MAP_HEIGHT, MAP_WIDTH};

pub fn render(frame: &mut Frame, app: &App) {
    // Main layout: map on left, sidebar on right
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(MAP_WIDTH as u16 + 2),
            Constraint::Length(30),
        ])
        .split(frame.area());

    // Left side: map on top, event log on bottom
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(MAP_HEIGHT as u16 + 2),
            Constraint::Length(10),
        ])
        .split(main_chunks[0]);

    render_map(frame, app, left_chunks[0]);
    render_event_log(frame, app, left_chunks[1]);
    render_sidebar(frame, app, main_chunks[1]);
}

fn render_map(frame: &mut Frame, app: &App, area: Rect) {
    let night_dim = if app.is_night() { true } else { false };

    let mut lines: Vec<Line> = Vec::new();
    for y in 0..MAP_HEIGHT {
        let mut spans: Vec<Span> = Vec::new();
        for x in 0..MAP_WIDTH {
            // Check if an orc is here
            if let Some(orc) = app.orcs.iter().find(|o| o.x == x && o.y == y) {
                let orc_char = match &orc.activity {
                    Activity::Sleeping => 'o', // lowercase when sleeping
                    _ => 'O',
                };
                let selected = app.selected_orc.is_some_and(|i| {
                    app.orcs[i].x == x && app.orcs[i].y == y
                });
                let color = if selected { Color::White } else { Color::LightGreen };
                let style = if selected {
                    Style::default().fg(color).add_modifier(Modifier::BOLD | Modifier::REVERSED)
                } else {
                    Style::default().fg(color).add_modifier(Modifier::BOLD)
                };
                spans.push(Span::styled(orc_char.to_string(), style));
            } else if app.cursor_x == x && app.cursor_y == y {
                // Cursor
                spans.push(Span::styled(
                    "X",
                    Style::default().fg(Color::White).add_modifier(Modifier::REVERSED),
                ));
            } else {
                let terrain = app.world.get(x, y);
                let mut color = terrain.color();
                if night_dim {
                    color = dim_color(color);
                }
                spans.push(Span::styled(
                    terrain.symbol().to_string(),
                    Style::default().fg(color),
                ));
            }
        }
        lines.push(Line::from(spans));
    }

    let time_label = if app.is_night() { "Night" } else { "Day" };
    let day_num = app.tick / 100 + 1;
    let title = format!(
        " Orc Village | Day {} ({}) | Tick {} | Speed: {}x {} ",
        day_num,
        time_label,
        app.tick,
        app.speed,
        if app.paused { "[PAUSED]" } else { "" }
    );

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if app.is_night() { Color::DarkGray } else { Color::White }));

    let map_widget = Paragraph::new(lines).block(block);
    frame.render_widget(map_widget, area);
}

fn render_event_log(frame: &mut Frame, app: &App, area: Rect) {
    let height = area.height.saturating_sub(2) as usize;
    let events = app.event_log.recent(height);

    let items: Vec<ListItem> = events
        .iter()
        .map(|e| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("[{:>4}] ", e.tick),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(&e.message, Style::default().fg(e.color)),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Events ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(list, area);
}

fn render_sidebar(frame: &mut Frame, app: &App, area: Rect) {
    // Split sidebar into orc list + help
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(8)])
        .split(area);

    // Orc details
    let mut items: Vec<ListItem> = Vec::new();
    for (i, orc) in app.orcs.iter().enumerate() {
        let selected = app.selected_orc == Some(i);
        let name_style = if selected {
            Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };

        let hunger_bar = bar(orc.hunger, 100.0, 8);
        let energy_bar = bar(orc.energy, 100.0, 8);

        let hunger_color = if orc.hunger > 70.0 { Color::Red } else if orc.hunger > 40.0 { Color::Yellow } else { Color::Green };
        let energy_color = if orc.energy < 20.0 { Color::Red } else if orc.energy < 50.0 { Color::Yellow } else { Color::Cyan };

        items.push(ListItem::new(vec![
            Line::from(vec![
                Span::styled(if selected { "> " } else { "  " }, name_style),
                Span::styled(&orc.name, name_style),
                Span::styled(format!(" ({})", orc.activity.label()), Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(vec![
                Span::raw("    Hun "),
                Span::styled(hunger_bar, Style::default().fg(hunger_color)),
                Span::styled(format!(" {:.0}", orc.hunger), Style::default().fg(hunger_color)),
            ]),
            Line::from(vec![
                Span::raw("    Nrg "),
                Span::styled(energy_bar, Style::default().fg(energy_color)),
                Span::styled(format!(" {:.0}", orc.energy), Style::default().fg(energy_color)),
            ]),
            Line::raw(""),
        ]));
    }

    let orc_list = List::new(items).block(
        Block::default()
            .title(" Clan ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green)),
    );
    frame.render_widget(orc_list, chunks[0]);

    // Help
    let help_text = vec![
        Line::styled(" Controls:", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Line::styled(" Space  Pause/Resume", Style::default().fg(Color::DarkGray)),
        Line::styled(" +/-    Speed up/down", Style::default().fg(Color::DarkGray)),
        Line::styled(" Arrows Move cursor", Style::default().fg(Color::DarkGray)),
        Line::styled(" Tab    Select orc", Style::default().fg(Color::DarkGray)),
        Line::styled(" f      Drop food", Style::default().fg(Color::DarkGray)),
        Line::styled(" q      Quit", Style::default().fg(Color::DarkGray)),
    ];
    let help = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(help, chunks[1]);
}

fn bar(value: f32, max: f32, width: usize) -> String {
    let filled = ((value / max) * width as f32).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

fn dim_color(color: Color) -> Color {
    match color {
        Color::Green => Color::DarkGray,
        Color::Blue => Color::DarkGray,
        Color::Yellow => Color::Rgb(100, 80, 0),
        Color::Gray => Color::DarkGray,
        Color::Magenta => Color::Rgb(80, 0, 80),
        _ => Color::DarkGray,
    }
}
