use rand::Rng;

use crate::event::EventLog;
use crate::world::{MAP_HEIGHT, MAP_WIDTH, Terrain, World};

const ORC_NAMES: &[&str] = &[
    "Grok", "Thrak", "Murg", "Zug", "Brak", "Gor", "Krag", "Drog", "Narg", "Skul",
    "Gash", "Rok", "Brug", "Thar", "Grub", "Vak", "Snak", "Blud", "Kurz", "Mogz",
];

#[derive(Clone, Debug, PartialEq)]
pub enum Activity {
    Idle,
    GoingTo { x: usize, y: usize, reason: String },
    Eating,
    Sleeping,
}

impl Activity {
    pub fn label(&self) -> &str {
        match self {
            Activity::Idle => "Idling",
            Activity::GoingTo { reason, .. } => reason.as_str(),
            Activity::Eating => "Eating",
            Activity::Sleeping => "Sleeping",
        }
    }
}

pub struct Orc {
    pub name: String,
    pub x: usize,
    pub y: usize,
    pub hunger: f32,   // 0 = full, 100 = starving
    pub energy: f32,   // 0 = exhausted, 100 = fully rested
    pub activity: Activity,
    idle_ticks: u32,
}

impl Orc {
    pub fn new(name: String, x: usize, y: usize) -> Self {
        Orc {
            name,
            x,
            y,
            hunger: 30.0,
            energy: 80.0,
            activity: Activity::Idle,
            idle_ticks: 0,
        }
    }

    pub fn spawn_clan(count: usize, world: &World, rng: &mut impl Rng) -> Vec<Orc> {
        let mut names: Vec<&str> = ORC_NAMES.to_vec();
        let mut orcs = Vec::new();

        for _ in 0..count {
            let name_idx = rng.gen_range(0..names.len());
            let name = names.remove(name_idx).to_string();

            // Spawn near campfire
            let (cx, cy) = world.campfire_pos;
            loop {
                let x = cx.saturating_sub(3) + rng.gen_range(0..7);
                let y = cy.saturating_sub(3) + rng.gen_range(0..7);
                if x < MAP_WIDTH && y < MAP_HEIGHT && world.is_walkable(x, y) {
                    // Check no other orc is here
                    if !orcs.iter().any(|o: &Orc| o.x == x && o.y == y) {
                        orcs.push(Orc::new(name, x, y));
                        break;
                    }
                }
            }
        }

        orcs
    }

    pub fn update(&mut self, world: &mut World, rng: &mut impl Rng, log: &mut EventLog, tick: u64, is_night: bool) {
        // Update needs
        let hunger_rate = if is_night { 0.3 } else { 0.5 };
        let energy_rate = if is_night { -0.8 } else { -0.4 }; // drains faster at night
        self.hunger = (self.hunger + hunger_rate).clamp(0.0, 100.0);

        match &self.activity {
            Activity::Sleeping => {
                self.energy = (self.energy + 3.0).clamp(0.0, 100.0);
            }
            _ => {
                self.energy = (self.energy + energy_rate).clamp(0.0, 100.0);
            }
        }

        // AI decision-making
        match &self.activity {
            Activity::Sleeping => {
                if self.energy >= 90.0 {
                    log.log(tick, format!("{} woke up, feeling rested", self.name), ratatui::style::Color::Cyan);
                    self.activity = Activity::Idle;
                }
            }
            Activity::Eating => {
                self.hunger = (self.hunger - 15.0).clamp(0.0, 100.0);
                if self.hunger <= 10.0 {
                    log.log(tick, format!("{} finished eating", self.name), ratatui::style::Color::Cyan);
                    self.activity = Activity::Idle;
                }
            }
            Activity::GoingTo { x, y, .. } => {
                let (tx, ty) = (*x, *y);
                if self.x == tx && self.y == ty {
                    // Arrived at destination
                    let terrain = world.get(tx, ty);
                    if terrain == Terrain::Tree || terrain == Terrain::Food {
                        log.log(tick, format!("{} found food and starts eating", self.name), ratatui::style::Color::Green);
                        if terrain == Terrain::Food {
                            world.set(tx, ty, Terrain::Grass);
                        }
                        self.activity = Activity::Eating;
                    } else {
                        // Near campfire to sleep
                        log.log(tick, format!("{} lies down to sleep by the fire", self.name), ratatui::style::Color::Blue);
                        self.activity = Activity::Sleeping;
                    }
                } else {
                    self.move_toward(tx, ty, world, rng);
                }
            }
            Activity::Idle => {
                // Check critical needs
                if self.energy < 20.0 {
                    // Need sleep - go to campfire
                    let (cx, cy) = world.campfire_pos;
                    // Find a walkable spot near campfire
                    let (sx, sy) = self.find_spot_near(cx, cy, world, rng);
                    log.log(tick, format!("{} is exhausted, heading to campfire", self.name), ratatui::style::Color::Yellow);
                    self.activity = Activity::GoingTo {
                        x: sx,
                        y: sy,
                        reason: "Going to sleep".to_string(),
                    };
                } else if self.hunger > 70.0 {
                    // Need food - find nearest tree or food
                    let target = world.find_nearest(self.x, self.y, Terrain::Food)
                        .or_else(|| world.find_nearest(self.x, self.y, Terrain::Tree));
                    if let Some((fx, fy)) = target {
                        log.log(tick, format!("{} is hungry, looking for food", self.name), ratatui::style::Color::Yellow);
                        self.activity = Activity::GoingTo {
                            x: fx,
                            y: fy,
                            reason: "Looking for food".to_string(),
                        };
                    }
                } else {
                    // Wander or idle
                    self.idle_ticks += 1;
                    if self.idle_ticks > 3 {
                        self.idle_ticks = 0;
                        // Pick a random nearby spot
                        let nx = (self.x as i32 + rng.gen_range(-3..=3)).clamp(0, MAP_WIDTH as i32 - 1) as usize;
                        let ny = (self.y as i32 + rng.gen_range(-3..=3)).clamp(0, MAP_HEIGHT as i32 - 1) as usize;
                        if world.is_walkable(nx, ny) {
                            self.activity = Activity::GoingTo {
                                x: nx,
                                y: ny,
                                reason: "Wandering".to_string(),
                            };
                        }
                    }
                }
            }
        }
    }

    fn move_toward(&mut self, tx: usize, ty: usize, world: &World, rng: &mut impl Rng) {
        let dx = (tx as i32 - self.x as i32).signum();
        let dy = (ty as i32 - self.y as i32).signum();

        // Try to move in the primary direction, with some randomness
        let candidates = if rng.gen_bool(0.7) {
            vec![(dx, dy), (dx, 0), (0, dy)]
        } else {
            vec![(dx, 0), (0, dy), (dx, dy)]
        };

        for (cdx, cdy) in candidates {
            if cdx == 0 && cdy == 0 {
                continue;
            }
            let nx = (self.x as i32 + cdx).clamp(0, MAP_WIDTH as i32 - 1) as usize;
            let ny = (self.y as i32 + cdy).clamp(0, MAP_HEIGHT as i32 - 1) as usize;
            if world.is_walkable(nx, ny) || world.get(nx, ny) == Terrain::Tree {
                self.x = nx;
                self.y = ny;
                return;
            }
        }
    }

    fn find_spot_near(&self, cx: usize, cy: usize, world: &World, rng: &mut impl Rng) -> (usize, usize) {
        for _ in 0..20 {
            let x = (cx as i32 + rng.gen_range(-2..=2)).clamp(0, MAP_WIDTH as i32 - 1) as usize;
            let y = (cy as i32 + rng.gen_range(-2..=2)).clamp(0, MAP_HEIGHT as i32 - 1) as usize;
            if world.is_walkable(x, y) {
                return (x, y);
            }
        }
        (cx, cy) // fallback
    }
}
