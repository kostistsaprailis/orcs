use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph};

use crate::app::App;
use crate::orc::Activity;
use crate::world::{MAP_HEIGHT, MAP_WIDTH};

pub fn render(frame: &mut Frame, app: &mut App) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(20),
            Constraint::Length(32),
        ])
        .split(frame.area());

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),
            Constraint::Length(10),
        ])
        .split(main_chunks[0]);

    render_map(frame, app, left_chunks[0]);
    render_event_log(frame, app, left_chunks[1]);
    render_sidebar(frame, app, main_chunks[1]);
}

fn render_map(frame: &mut Frame, app: &mut App, area: Rect) {
    let night_dim = app.is_night();

    let vw = (area.width.saturating_sub(2)) as usize;
    let vh = (area.height.saturating_sub(2)) as usize;

    app.update_camera(vw, vh);

    let cam_x = app.camera_x;
    let cam_y = app.camera_y;

    let mut lines: Vec<Line> = Vec::new();
    for y in cam_y..(cam_y + vh).min(MAP_HEIGHT) {
        let mut spans: Vec<Span> = Vec::new();
        for x in cam_x..(cam_x + vw).min(MAP_WIDTH) {
            // Check if an orc is here
            if let Some((idx, orc)) = app.orcs.iter().enumerate().find(|(_, o)| o.x == x && o.y == y) {
                if !orc.alive {
                    // Dead orc tombstone
                    spans.push(Span::styled("†", Style::default().fg(Color::DarkGray)));
                } else {
                    let orc_char = match &orc.activity {
                        Activity::Sleeping => '◎',
                        Activity::Hunting { .. } => '⚔',
                        Activity::CarryingMeat => '☻',
                        _ => '☻',
                    };
                    let selected = app.selected_orc == Some(idx);
                    let color = if orc.health < 30.0 {
                        Color::Red
                    } else if selected {
                        Color::White
                    } else if orc.carrying_food {
                        Color::Rgb(180, 120, 60)
                    } else {
                        Color::LightGreen
                    };
                    let style = if selected {
                        Style::default().fg(color).add_modifier(Modifier::BOLD | Modifier::REVERSED)
                    } else {
                        Style::default().fg(color).add_modifier(Modifier::BOLD)
                    };
                    spans.push(Span::styled(orc_char.to_string(), style));
                }
            } else if let Some(animal) = app.animals.iter().find(|a| a.alive && a.x == x && a.y == y) {
                // Render animal
                let mut color = animal.kind.color();
                if night_dim {
                    color = dim_color(color);
                }
                spans.push(Span::styled(
                    animal.kind.symbol().to_string(),
                    Style::default().fg(color),
                ));
            } else if app.cursor_x == x && app.cursor_y == y {
                spans.push(Span::styled(
                    "▣",
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
    let alive_count = app.orcs.iter().filter(|o| o.alive).count();
    let title = format!(
        " Orc Village | Day {} ({}) | Pop: {} | Meat: {} | Speed: {}x {} | ({},{}) ",
        day_num,
        time_label,
        alive_count,
        app.world.food_stockpile,
        app.speed,
        if app.paused { "[PAUSED]" } else { "" },
        app.cursor_x,
        app.cursor_y,
    );

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
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
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(list, area);
}

fn render_sidebar(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(9)])
        .split(area);

    // Orc details
    let mut items: Vec<ListItem> = Vec::new();
    for (i, orc) in app.orcs.iter().enumerate() {
        if !orc.alive {
            items.push(ListItem::new(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(&orc.name, Style::default().fg(Color::DarkGray)),
                Span::styled(" (Dead)", Style::default().fg(Color::Red)),
            ])));
            continue;
        }

        let selected = app.selected_orc == Some(i);
        let name_style = if selected {
            Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };

        let health_bar = bar(orc.health, 100.0, 6);
        let hunger_bar = bar(orc.hunger, 100.0, 6);
        let energy_bar = bar(orc.energy, 100.0, 6);
        let thirst_bar = bar(orc.thirst, 100.0, 6);

        let health_color = if orc.health < 30.0 { Color::Red } else if orc.health < 60.0 { Color::Yellow } else { Color::Green };
        let hunger_color = if orc.hunger > 70.0 { Color::Red } else if orc.hunger > 40.0 { Color::Yellow } else { Color::Green };
        let energy_color = if orc.energy < 20.0 { Color::Red } else if orc.energy < 50.0 { Color::Yellow } else { Color::Cyan };
        let thirst_color = if orc.thirst > 70.0 { Color::Red } else if orc.thirst > 40.0 { Color::Yellow } else { Color::Rgb(65, 105, 225) };

        items.push(ListItem::new(vec![
            Line::from(vec![
                Span::styled(if selected { "> " } else { "  " }, name_style),
                Span::styled(&orc.name, name_style),
                Span::styled(format!(" ({})", orc.activity.label()), Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(vec![
                Span::raw("   HP "),
                Span::styled(health_bar, Style::default().fg(health_color)),
                Span::styled(format!(" {:.0}", orc.health), Style::default().fg(health_color)),
            ]),
            Line::from(vec![
                Span::raw("   Hun"),
                Span::styled(hunger_bar, Style::default().fg(hunger_color)),
                Span::styled(format!(" {:.0}", orc.hunger), Style::default().fg(hunger_color)),
            ]),
            Line::from(vec![
                Span::raw("   Nrg"),
                Span::styled(energy_bar, Style::default().fg(energy_color)),
                Span::styled(format!(" {:.0}", orc.energy), Style::default().fg(energy_color)),
            ]),
            Line::from(vec![
                Span::raw("   H2O"),
                Span::styled(thirst_bar, Style::default().fg(thirst_color)),
                Span::styled(format!(" {:.0}", orc.thirst), Style::default().fg(thirst_color)),
            ]),
            Line::raw(""),
        ]));
    }

    let orc_list = List::new(items).block(
        Block::default()
            .title(" Clan ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
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
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(help, chunks[1]);
}

fn bar(value: f32, max: f32, width: usize) -> String {
    let ratio = value / max;
    let filled = (ratio * width as f32).floor() as usize;
    let remainder = (ratio * width as f32) - filled as f32;
    let has_transition = filled < width && remainder > 0.3;
    let empty = width.saturating_sub(filled).saturating_sub(if has_transition { 1 } else { 0 });
    let transition = if has_transition { "▒" } else { "" };
    format!("[{}{}{}]", "▓".repeat(filled), transition, "░".repeat(empty))
}

fn dim_color(color: Color) -> Color {
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(r / 3, g / 3, b / 3),
        Color::Gray => Color::DarkGray,
        _ => Color::DarkGray,
    }
}
