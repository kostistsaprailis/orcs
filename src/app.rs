use rand::rngs::ThreadRng;
use rand::Rng;

use crate::animal::{self, Animal};
use crate::event::EventLog;
use crate::orc::{self, Orc};
use crate::world::{MAP_HEIGHT, MAP_WIDTH, Terrain, World};

const MAX_CLAN_SIZE: usize = 15;

pub struct App {
    pub world: World,
    pub orcs: Vec<Orc>,
    pub animals: Vec<Animal>,
    pub event_log: EventLog,
    pub tick: u64,
    pub paused: bool,
    pub speed: u32,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub camera_x: usize,
    pub camera_y: usize,
    pub selected_orc: Option<usize>,
    pub should_quit: bool,
    rng: ThreadRng,
}

impl App {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let world = World::generate(&mut rng);
        let orcs = Orc::spawn_clan(5, &world, &mut rng);
        let animals = Animal::spawn_initial(&world, &mut rng);
        let mut event_log = EventLog::new();

        event_log.log(0, "A clan of orcs settles in a new land...".to_string(), ratatui::style::Color::White);
        for orc in &orcs {
            event_log.log(0, format!("{} joins the clan", orc.name), ratatui::style::Color::Green);
        }

        let (cx, cy) = world.campfire_pos;

        App {
            world,
            orcs,
            animals,
            event_log,
            tick: 0,
            paused: false,
            speed: 1,
            cursor_x: cx,
            cursor_y: cy,
            camera_x: 0,
            camera_y: 0,
            selected_orc: None,
            should_quit: false,
            rng,
        }
    }

    pub fn is_night(&self) -> bool {
        let time_of_day = self.tick % 100;
        time_of_day >= 60
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

        // Update animals
        let orc_positions: Vec<(usize, usize)> = self.orcs.iter()
            .filter(|o| o.alive)
            .map(|o| (o.x, o.y))
            .collect();
        for animal in &mut self.animals {
            animal.update(&self.world, &orc_positions, &mut self.rng);
        }

        // Update each orc
        let num_orcs = self.orcs.len();
        for i in 0..num_orcs {
            let mut orc = std::mem::replace(&mut self.orcs[i], Orc::new(String::new(), 0, 0));
            orc.update(&mut self.world, &mut self.animals, &mut self.rng, &mut self.event_log, self.tick, is_night);
            self.orcs[i] = orc;
        }

        // Remove dead orcs after a few ticks (show tombstone briefly)
        self.orcs.retain(|orc| {
            if !orc.alive {
                if let Some(death_tick) = orc.death_tick {
                    return self.tick - death_tick < 20; // keep tombstone for 20 ticks
                }
            }
            true
        });

        // Fix selected_orc index if orcs were removed
        if let Some(idx) = self.selected_orc {
            if idx >= self.orcs.len() {
                self.selected_orc = if self.orcs.is_empty() { None } else { Some(self.orcs.len() - 1) };
            }
        }

        // Remove dead animals
        self.animals.retain(|a| a.alive);

        // Animal respawn
        animal::try_respawn(&mut self.animals, &self.world, &mut self.rng, self.tick);

        // Bush regrowth
        self.world.tick_regrowth(self.tick);

        // Birth system - check every 300 ticks
        if self.tick % 300 == 0 {
            self.check_birth();
        }
    }

    fn check_birth(&mut self) {
        let living: Vec<&Orc> = self.orcs.iter().filter(|o| o.alive).collect();
        let count = living.len();

        if count < 2 || count >= MAX_CLAN_SIZE {
            return;
        }

        let avg_hunger: f32 = living.iter().map(|o| o.hunger).sum::<f32>() / count as f32;
        let avg_energy: f32 = living.iter().map(|o| o.energy).sum::<f32>() / count as f32;

        // Birth conditions: well-fed, rested, have stockpile
        if avg_hunger < 40.0 && avg_energy > 40.0 && self.world.food_stockpile > 0 {
            self.world.food_stockpile -= 1;

            let existing_names: Vec<String> = self.orcs.iter().map(|o| o.name.clone()).collect();
            let name = orc::pick_name(&mut self.rng, &existing_names);

            let (cx, cy) = self.world.campfire_pos;
            let mut x = cx;
            let mut y = cy;
            for _ in 0..20 {
                let nx = (cx as i32 + self.rng.gen_range(-2..=2)).clamp(0, MAP_WIDTH as i32 - 1) as usize;
                let ny = (cy as i32 + self.rng.gen_range(-2..=2)).clamp(0, MAP_HEIGHT as i32 - 1) as usize;
                if self.world.is_walkable(nx, ny) {
                    x = nx;
                    y = ny;
                    break;
                }
            }

            self.event_log.log(
                self.tick,
                format!("{} is born into the clan!", name),
                ratatui::style::Color::LightGreen,
            );
            self.orcs.push(Orc::new(name, x, y));
        }
    }

    pub fn move_cursor(&mut self, dx: i32, dy: i32) {
        let nx = (self.cursor_x as i32 + dx).clamp(0, MAP_WIDTH as i32 - 1) as usize;
        let ny = (self.cursor_y as i32 + dy).clamp(0, MAP_HEIGHT as i32 - 1) as usize;
        self.cursor_x = nx;
        self.cursor_y = ny;
    }

    pub fn update_camera(&mut self, viewport_w: usize, viewport_h: usize) {
        let half_w = viewport_w / 2;
        let half_h = viewport_h / 2;

        self.camera_x = if self.cursor_x < half_w {
            0
        } else if self.cursor_x + half_w >= MAP_WIDTH {
            MAP_WIDTH.saturating_sub(viewport_w)
        } else {
            self.cursor_x - half_w
        };

        self.camera_y = if self.cursor_y < half_h {
            0
        } else if self.cursor_y + half_h >= MAP_HEIGHT {
            MAP_HEIGHT.saturating_sub(viewport_h)
        } else {
            self.cursor_y - half_h
        };
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
        let living: Vec<usize> = self.orcs.iter().enumerate()
            .filter(|(_, o)| o.alive)
            .map(|(i, _)| i)
            .collect();

        if living.is_empty() {
            self.selected_orc = None;
            return;
        }

        self.selected_orc = match self.selected_orc {
            None => Some(living[0]),
            Some(current) => {
                let pos = living.iter().position(|&i| i == current);
                match pos {
                    Some(p) if p + 1 < living.len() => Some(living[p + 1]),
                    _ => None,
                }
            }
        };

        // Snap cursor to selected orc
        if let Some(i) = self.selected_orc {
            self.cursor_x = self.orcs[i].x;
            self.cursor_y = self.orcs[i].y;
        }
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
