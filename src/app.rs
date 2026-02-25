use rand::rngs::ThreadRng;

use crate::event::EventLog;
use crate::orc::Orc;
use crate::world::{MAP_HEIGHT, MAP_WIDTH, Terrain, World};

pub struct App {
    pub world: World,
    pub orcs: Vec<Orc>,
    pub event_log: EventLog,
    pub tick: u64,
    pub paused: bool,
    pub speed: u32, // 1 = normal, 2 = 2x, etc.
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub selected_orc: Option<usize>,
    pub should_quit: bool,
    rng: ThreadRng,
}

impl App {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let world = World::generate(&mut rng);
        let orcs = Orc::spawn_clan(5, &world, &mut rng);
        let mut event_log = EventLog::new();

        event_log.log(0, "A clan of orcs settles in a new land...".to_string(), ratatui::style::Color::White);
        for orc in &orcs {
            event_log.log(0, format!("{} joins the clan", orc.name), ratatui::style::Color::Green);
        }

        let (cx, cy) = world.campfire_pos;

        App {
            world,
            orcs,
            event_log,
            tick: 0,
            paused: false,
            speed: 1,
            cursor_x: cx,
            cursor_y: cy,
            selected_orc: None,
            should_quit: false,
            rng,
        }
    }

    pub fn is_night(&self) -> bool {
        let time_of_day = self.tick % 100;
        time_of_day >= 60 // last 40% of day is night
    }

    pub fn tick(&mut self) {
        if self.paused {
            return;
        }

        self.tick += 1;

        // Day/night transition messages
        let time_of_day = self.tick % 100;
        if time_of_day == 0 {
            let day = self.tick / 100 + 1;
            self.event_log.log(self.tick, format!("=== Day {} begins ===", day), ratatui::style::Color::White);
        } else if time_of_day == 60 {
            self.event_log.log(self.tick, "Night falls...".to_string(), ratatui::style::Color::Blue);
        }

        let is_night = self.is_night();

        // Update each orc (need to work around borrow checker)
        let num_orcs = self.orcs.len();
        for i in 0..num_orcs {
            let mut orc = std::mem::replace(&mut self.orcs[i], Orc::new(String::new(), 0, 0));
            orc.update(&mut self.world, &mut self.rng, &mut self.event_log, self.tick, is_night);
            self.orcs[i] = orc;
        }
    }

    pub fn move_cursor(&mut self, dx: i32, dy: i32) {
        let nx = (self.cursor_x as i32 + dx).clamp(0, MAP_WIDTH as i32 - 1) as usize;
        let ny = (self.cursor_y as i32 + dy).clamp(0, MAP_HEIGHT as i32 - 1) as usize;
        self.cursor_x = nx;
        self.cursor_y = ny;
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn speed_up(&mut self) {
        if self.speed < 10 {
            self.speed += 1;
        }
    }

    pub fn speed_down(&mut self) {
        if self.speed > 1 {
            self.speed -= 1;
        }
    }

    pub fn cycle_selected_orc(&mut self) {
        self.selected_orc = match self.selected_orc {
            None => Some(0),
            Some(i) => {
                if i + 1 >= self.orcs.len() {
                    None
                } else {
                    Some(i + 1)
                }
            }
        };
    }

    pub fn drop_food(&mut self) {
        let terrain = self.world.get(self.cursor_x, self.cursor_y);
        if terrain == Terrain::Grass {
            self.world.set(self.cursor_x, self.cursor_y, Terrain::Food);
            self.event_log.log(
                self.tick,
                format!("Food dropped at ({}, {})", self.cursor_x, self.cursor_y),
                ratatui::style::Color::Magenta,
            );
        }
    }

    pub fn tick_interval_ms(&self) -> u64 {
        1000 / self.speed as u64
    }
}
