use rand::Rng;

use crate::event::EventLog;
use crate::world::{MAP_HEIGHT, MAP_WIDTH, Terrain, World};

#[derive(Clone, Copy, PartialEq)]
pub enum AnimalKind {
    Deer,
    Boar,
}

impl AnimalKind {
    pub fn symbol(&self) -> char {
        match self {
            AnimalKind::Deer => 'δ',
            AnimalKind::Boar => 'β',
        }
    }

    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            AnimalKind::Deer => Color::Rgb(180, 140, 80),
            AnimalKind::Boar => Color::Rgb(140, 100, 60),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            AnimalKind::Deer => "Deer",
            AnimalKind::Boar => "Boar",
        }
    }
}

pub struct Animal {
    pub kind: AnimalKind,
    pub x: usize,
    pub y: usize,
    pub alive: bool,
}

impl Animal {
    pub fn new(kind: AnimalKind, x: usize, y: usize) -> Self {
        Animal {
            kind,
            x,
            y,
            alive: true,
        }
    }

    pub fn spawn_initial(world: &World, rng: &mut impl Rng) -> Vec<Animal> {
        let mut animals = Vec::new();
        let count = rng.gen_range(8..13);
        let (cx, cy) = world.campfire_pos;

        for _ in 0..count {
            let kind = if rng.gen_bool(0.6) {
                AnimalKind::Deer
            } else {
                AnimalKind::Boar
            };

            // Spawn away from campfire (at least 15 tiles)
            for _ in 0..100 {
                let x = rng.gen_range(5..MAP_WIDTH - 5);
                let y = rng.gen_range(5..MAP_HEIGHT - 5);
                let dist = cx.abs_diff(x) + cy.abs_diff(y);
                if dist > 15 && world.is_walkable(x, y) {
                    animals.push(Animal::new(kind, x, y));
                    break;
                }
            }
        }

        animals
    }

    pub fn update(&mut self, world: &World, orcs: &[(usize, usize)], rng: &mut impl Rng) {
        if !self.alive {
            return;
        }

        // Deer flee from nearby orcs
        if self.kind == AnimalKind::Deer {
            if let Some((ox, oy)) = orcs.iter().find(|&&(ox, oy)| {
                self.x.abs_diff(ox) + self.y.abs_diff(oy) <= 5
            }) {
                // Flee away from orc
                let dx = (self.x as i32 - *ox as i32).signum();
                let dy = (self.y as i32 - *oy as i32).signum();
                let nx = (self.x as i32 + dx * 2).clamp(0, MAP_WIDTH as i32 - 1) as usize;
                let ny = (self.y as i32 + dy * 2).clamp(0, MAP_HEIGHT as i32 - 1) as usize;
                if world.is_walkable(nx, ny) {
                    self.x = nx;
                    self.y = ny;
                }
                return;
            }
        }

        // Random wander (boars move less often)
        let move_chance = match self.kind {
            AnimalKind::Deer => 0.4,
            AnimalKind::Boar => 0.2,
        };

        if rng.gen_bool(move_chance) {
            let dx = rng.gen_range(-1..=1i32);
            let dy = rng.gen_range(-1..=1i32);
            let nx = (self.x as i32 + dx).clamp(0, MAP_WIDTH as i32 - 1) as usize;
            let ny = (self.y as i32 + dy).clamp(0, MAP_HEIGHT as i32 - 1) as usize;
            if world.is_walkable(nx, ny) {
                self.x = nx;
                self.y = ny;
            }
        }
    }

    pub fn kill(&mut self, world: &mut World, log: &mut EventLog, tick: u64) {
        self.alive = false;
        // Drop food (meat) at the animal's position
        if world.get(self.x, self.y) == Terrain::Grass {
            world.set(self.x, self.y, Terrain::Food);
        }
        log.log(
            tick,
            format!("A {} was hunted!", self.kind.name()),
            ratatui::style::Color::Rgb(180, 140, 80),
        );
    }
}

pub fn try_respawn(animals: &mut Vec<Animal>, world: &World, rng: &mut impl Rng, tick: u64) {
    // Respawn every ~200 ticks if population is low
    if tick % 200 != 0 {
        return;
    }

    let alive_count = animals.iter().filter(|a| a.alive).count();
    if alive_count >= 12 {
        return;
    }

    let (cx, cy) = world.campfire_pos;
    let spawn_count = rng.gen_range(1..=3);
    for _ in 0..spawn_count {
        let kind = if rng.gen_bool(0.6) {
            AnimalKind::Deer
        } else {
            AnimalKind::Boar
        };
        for _ in 0..50 {
            let x = rng.gen_range(5..MAP_WIDTH - 5);
            let y = rng.gen_range(5..MAP_HEIGHT - 5);
            let dist = cx.abs_diff(x) + cy.abs_diff(y);
            if dist > 20 && world.is_walkable(x, y) {
                animals.push(Animal::new(kind, x, y));
                break;
            }
        }
    }
}
