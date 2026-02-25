use rand::Rng;

use crate::animal::Animal;
use crate::event::EventLog;
use crate::pathfinding;
use crate::world::{MAP_HEIGHT, MAP_WIDTH, Terrain, World};

const ORC_NAMES: &[&str] = &[
    "Grok", "Thrak", "Murg", "Zug", "Brak", "Gor", "Krag", "Drog", "Narg", "Skul",
    "Gash", "Rok", "Brug", "Thar", "Grub", "Vak", "Snak", "Blud", "Kurz", "Mogz",
    "Thog", "Grim", "Uzk", "Ragz", "Lurk", "Bonk", "Drak", "Gurn", "Tusk", "Mok",
];

#[derive(Clone, Debug, PartialEq)]
pub enum Activity {
    Idle,
    GoingTo { x: usize, y: usize, reason: String },
    Eating,
    Sleeping,
    Drinking,
    Hunting { target_idx: usize },
    CarryingMeat,
}

impl Activity {
    pub fn label(&self) -> &str {
        match self {
            Activity::Idle => "Idling",
            Activity::GoingTo { reason, .. } => reason.as_str(),
            Activity::Eating => "Eating",
            Activity::Sleeping => "Sleeping",
            Activity::Drinking => "Drinking",
            Activity::Hunting { .. } => "Hunting",
            Activity::CarryingMeat => "Carrying meat",
        }
    }
}

pub struct Orc {
    pub name: String,
    pub x: usize,
    pub y: usize,
    pub hunger: f32,
    pub energy: f32,
    pub thirst: f32,
    pub health: f32,
    pub alive: bool,
    pub death_tick: Option<u64>,
    pub activity: Activity,
    idle_ticks: u32,
    pub carrying_food: bool,
    path: Vec<(usize, usize)>, // A* computed waypoints
    path_step: usize,
}

impl Orc {
    pub fn new(name: String, x: usize, y: usize) -> Self {
        Orc {
            name,
            x,
            y,
            hunger: 20.0,
            energy: 80.0,
            thirst: 10.0,
            health: 100.0,
            alive: true,
            death_tick: None,
            activity: Activity::Idle,
            idle_ticks: 0,
            carrying_food: false,
            path: Vec::new(),
            path_step: 0,
        }
    }

    pub fn spawn_clan(count: usize, world: &World, rng: &mut impl Rng) -> Vec<Orc> {
        let mut used_names: Vec<String> = Vec::new();
        let mut orcs = Vec::new();

        for _ in 0..count {
            let name = pick_name(rng, &used_names);
            used_names.push(name.clone());

            let (cx, cy) = world.campfire_pos;
            loop {
                let x = cx.saturating_sub(3) + rng.gen_range(0..7);
                let y = cy.saturating_sub(3) + rng.gen_range(0..7);
                if x < MAP_WIDTH && y < MAP_HEIGHT && world.is_walkable(x, y) {
                    if !orcs.iter().any(|o: &Orc| o.x == x && o.y == y) {
                        orcs.push(Orc::new(name, x, y));
                        break;
                    }
                }
            }
        }

        orcs
    }

    /// Compute and store an A* path to the target
    fn plan_path(&mut self, tx: usize, ty: usize, world: &World, allow_tree: bool) {
        if let Some(p) = pathfinding::find_path(world, self.x, self.y, tx, ty, allow_tree) {
            self.path = p;
            self.path_step = 0;
        } else {
            // No path found — clear and rely on fallback
            self.path.clear();
            self.path_step = 0;
        }
    }

    /// Follow the stored A* path. Returns true if moved, false if path exhausted.
    fn follow_path(&mut self) -> bool {
        if self.path_step < self.path.len() {
            let (nx, ny) = self.path[self.path_step];
            self.x = nx;
            self.y = ny;
            self.path_step += 1;
            true
        } else {
            false
        }
    }

    /// Set a GoingTo activity and compute the path
    fn go_to(&mut self, x: usize, y: usize, reason: String, world: &World) {
        let allow_tree = matches!(world.get(x, y), Terrain::Tree | Terrain::Bush);
        self.plan_path(x, y, world, allow_tree);
        self.activity = Activity::GoingTo { x, y, reason };
    }

    pub fn update(
        &mut self,
        world: &mut World,
        animals: &mut Vec<Animal>,
        rng: &mut impl Rng,
        log: &mut EventLog,
        tick: u64,
        is_night: bool,
    ) {
        if !self.alive {
            return;
        }

        // Update needs
        let hunger_rate = if is_night { 0.3 } else { 0.5 };
        let energy_drain = if is_night { 0.8 } else { 0.4 };
        let thirst_rate = 0.6;

        self.hunger = (self.hunger + hunger_rate).clamp(0.0, 100.0);
        self.thirst = (self.thirst + thirst_rate).clamp(0.0, 100.0);

        match &self.activity {
            Activity::Sleeping => {
                self.energy = (self.energy + 3.0).clamp(0.0, 100.0);
            }
            _ => {
                self.energy = (self.energy - energy_drain).clamp(0.0, 100.0);
            }
        }

        // Health system
        let mut health_delta = 0.0f32;
        if self.hunger >= 95.0 {
            health_delta -= 2.0;
        }
        if self.thirst >= 95.0 {
            health_delta -= 3.0;
        }
        if self.energy <= 5.0 {
            health_delta -= 1.0;
        }
        if self.hunger < 50.0 && self.thirst < 50.0 && self.energy > 30.0 {
            health_delta += 0.5;
        }
        self.health = (self.health + health_delta).clamp(0.0, 100.0);

        // Death check
        if self.health <= 0.0 {
            self.alive = false;
            self.death_tick = Some(tick);
            log.log(tick, format!("{} has died!", self.name), ratatui::style::Color::Red);
            return;
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
            Activity::Drinking => {
                self.thirst = (self.thirst - 20.0).clamp(0.0, 100.0);
                if self.thirst <= 5.0 {
                    log.log(tick, format!("{} finished drinking", self.name), ratatui::style::Color::Cyan);
                    self.activity = Activity::Idle;
                }
            }
            Activity::Hunting { target_idx } => {
                let idx = *target_idx;
                if idx < animals.len() && animals[idx].alive {
                    let (ax, ay) = (animals[idx].x, animals[idx].y);
                    let dist = self.x.abs_diff(ax) + self.y.abs_diff(ay);
                    if dist <= 1 {
                        animals[idx].kill(world, log, tick);
                        log.log(tick, format!("{} caught a {}!", self.name, animals[idx].kind.name()), ratatui::style::Color::Green);
                        if self.hunger > 50.0 {
                            self.activity = Activity::Eating;
                        } else {
                            self.carrying_food = true;
                            self.activity = Activity::CarryingMeat;
                            if world.get(ax, ay) == Terrain::Food {
                                world.set(ax, ay, Terrain::Grass);
                            }
                            // Plan path to meat rack
                            if let Some((mx, my)) = world.meat_rack_pos() {
                                self.plan_path(mx, my, world, false);
                            }
                        }
                    } else {
                        // Recompute path to moving target every few steps
                        if self.path.is_empty() || self.path_step >= self.path.len() {
                            self.plan_path(ax, ay, world, false);
                        }
                        if !self.follow_path() {
                            // Fallback: greedy move
                            self.move_toward_greedy(ax, ay, world, rng);
                        }
                    }
                } else {
                    self.activity = Activity::Idle;
                }
            }
            Activity::CarryingMeat => {
                if let Some((mx, my)) = world.meat_rack_pos() {
                    let dist = self.x.abs_diff(mx) + self.y.abs_diff(my);
                    if dist <= 1 {
                        world.food_stockpile += 1;
                        self.carrying_food = false;
                        log.log(tick, format!("{} stored meat (stockpile: {})", self.name, world.food_stockpile), ratatui::style::Color::Rgb(180, 120, 60));
                        self.activity = Activity::Idle;
                    } else if !self.follow_path() {
                        self.move_toward_greedy(mx, my, world, rng);
                    }
                } else {
                    self.carrying_food = false;
                    self.activity = Activity::Idle;
                }
            }
            Activity::GoingTo { x, y, .. } => {
                let (tx, ty) = (*x, *y);
                if self.x == tx && self.y == ty {
                    self.arrive_at_destination(world, log, tick);
                } else if !self.follow_path() {
                    // Path exhausted or failed — fallback to greedy
                    self.move_toward_greedy(tx, ty, world, rng);
                }
            }
            Activity::Idle => {
                self.decide_action(world, animals, rng, log, tick, is_night);
            }
        }
    }

    fn arrive_at_destination(&mut self, world: &mut World, log: &mut EventLog, tick: u64) {
        let terrain = world.get(self.x, self.y);

        if terrain == Terrain::Bush {
            log.log(tick, format!("{} found berries and starts eating", self.name), ratatui::style::Color::Green);
            world.deplete_bush(self.x, self.y, tick);
            self.activity = Activity::Eating;
        } else if terrain == Terrain::Food {
            log.log(tick, format!("{} found food and starts eating", self.name), ratatui::style::Color::Green);
            world.set(self.x, self.y, Terrain::Grass);
            self.activity = Activity::Eating;
        } else if terrain == Terrain::Tree {
            log.log(tick, format!("{} forages from a tree", self.name), ratatui::style::Color::Green);
            self.activity = Activity::Eating;
        } else if terrain == Terrain::MeatRack && world.food_stockpile > 0 {
            world.food_stockpile -= 1;
            log.log(tick, format!("{} takes food from stockpile (left: {})", self.name, world.food_stockpile), ratatui::style::Color::Rgb(180, 120, 60));
            self.activity = Activity::Eating;
        } else if self.is_adjacent_to_water(world) {
            log.log(tick, format!("{} drinks water", self.name), ratatui::style::Color::Rgb(65, 105, 225));
            self.activity = Activity::Drinking;
        } else {
            log.log(tick, format!("{} lies down to sleep by the fire", self.name), ratatui::style::Color::Blue);
            self.activity = Activity::Sleeping;
        }
    }

    fn decide_action(
        &mut self,
        world: &mut World,
        animals: &[Animal],
        rng: &mut impl Rng,
        log: &mut EventLog,
        tick: u64,
        _is_night: bool,
    ) {
        let (cx, cy) = world.campfire_pos;

        // Priority 1: Health critical
        if self.health < 20.0 {
            if self.thirst > self.hunger && self.thirst > (100.0 - self.energy) {
                if let Some((wx, wy)) = world.find_water_adjacent(self.x, self.y) {
                    log.log(tick, format!("{} desperately needs water!", self.name), ratatui::style::Color::Red);
                    self.go_to(wx, wy, "Desperate for water".to_string(), world);
                    return;
                }
            } else if self.hunger > (100.0 - self.energy) {
                if let Some(target) = self.find_food_target(world, animals) {
                    log.log(tick, format!("{} desperately needs food!", self.name), ratatui::style::Color::Red);
                    self.set_activity_with_path(target, world);
                    return;
                }
            } else {
                let (sx, sy) = self.find_spot_near(cx, cy, world, rng);
                log.log(tick, format!("{} desperately needs rest!", self.name), ratatui::style::Color::Red);
                self.go_to(sx, sy, "Desperate for sleep".to_string(), world);
                return;
            }
        }

        // Priority 2: Thirst
        if self.thirst > 60.0 {
            if let Some((wx, wy)) = world.find_water_adjacent(self.x, self.y) {
                log.log(tick, format!("{} is thirsty, heading to water", self.name), ratatui::style::Color::Yellow);
                self.go_to(wx, wy, "Going to drink".to_string(), world);
                return;
            }
        }

        // Priority 3: Hunger
        if self.hunger > 70.0 {
            if let Some(target) = self.find_food_target(world, animals) {
                log.log(tick, format!("{} is hungry, looking for food", self.name), ratatui::style::Color::Yellow);
                self.set_activity_with_path(target, world);
                return;
            }
        }

        // Priority 4: Sleep
        if self.energy < 20.0 {
            let (sx, sy) = self.find_spot_near(cx, cy, world, rng);
            log.log(tick, format!("{} is exhausted, heading to campfire", self.name), ratatui::style::Color::Yellow);
            self.go_to(sx, sy, "Going to sleep".to_string(), world);
            return;
        }

        // Priority 5: Carrying meat
        if self.carrying_food {
            self.activity = Activity::CarryingMeat;
            if let Some((mx, my)) = world.meat_rack_pos() {
                self.plan_path(mx, my, world, false);
            }
            return;
        }

        // Priority 6: Wander
        self.idle_ticks += 1;
        if self.idle_ticks > 3 {
            self.idle_ticks = 0;
            let max_dist: i32 = 30;
            let nx = (self.x as i32 + rng.gen_range(-4..=4))
                .clamp(cx as i32 - max_dist, cx as i32 + max_dist)
                .clamp(0, MAP_WIDTH as i32 - 1) as usize;
            let ny = (self.y as i32 + rng.gen_range(-4..=4))
                .clamp(cy as i32 - max_dist, cy as i32 + max_dist)
                .clamp(0, MAP_HEIGHT as i32 - 1) as usize;
            if world.is_walkable(nx, ny) {
                self.go_to(nx, ny, "Wandering".to_string(), world);
            }
        }
    }

    /// Set an activity that may be GoingTo or Hunting, computing path if needed
    fn set_activity_with_path(&mut self, activity: Activity, world: &World) {
        match &activity {
            Activity::GoingTo { x, y, .. } => {
                let (tx, ty) = (*x, *y);
                let allow_tree = matches!(world.get(tx, ty), Terrain::Tree | Terrain::Bush);
                self.plan_path(tx, ty, world, allow_tree);
            }
            Activity::Hunting { .. } => {
                // Hunting paths are recomputed dynamically since the target moves
                self.path.clear();
                self.path_step = 0;
            }
            _ => {}
        }
        self.activity = activity;
    }

    fn find_food_target(&self, world: &World, animals: &[Animal]) -> Option<Activity> {
        if world.food_stockpile > 0 {
            if let Some((mx, my)) = world.meat_rack_pos() {
                return Some(Activity::GoingTo {
                    x: mx, y: my,
                    reason: "Going to stockpile".to_string(),
                });
            }
        }

        let bush = world.find_nearest(self.x, self.y, Terrain::Bush);
        let food = world.find_nearest(self.x, self.y, Terrain::Food);
        let tree = world.find_nearest(self.x, self.y, Terrain::Tree);

        let mut best: Option<(usize, usize, usize)> = None;
        for target in [bush, food, tree].iter().flatten() {
            let dist = self.x.abs_diff(target.0) + self.y.abs_diff(target.1);
            if best.is_none() || dist < best.unwrap().2 {
                best = Some((target.0, target.1, dist));
            }
        }

        let nearest_animal = animals.iter().enumerate()
            .filter(|(_, a)| a.alive)
            .min_by_key(|(_, a)| self.x.abs_diff(a.x) + self.y.abs_diff(a.y));

        if let Some((idx, animal)) = nearest_animal {
            let animal_dist = self.x.abs_diff(animal.x) + self.y.abs_diff(animal.y);
            if best.is_none() || animal_dist < 15 {
                return Some(Activity::Hunting { target_idx: idx });
            }
        }

        best.map(|(x, y, _)| Activity::GoingTo {
            x, y,
            reason: "Looking for food".to_string(),
        })
    }

    fn is_adjacent_to_water(&self, world: &World) -> bool {
        let neighbors = [(0i32, 1i32), (0, -1), (1, 0), (-1, 0)];
        neighbors.iter().any(|&(dx, dy)| {
            let nx = (self.x as i32 + dx).clamp(0, MAP_WIDTH as i32 - 1) as usize;
            let ny = (self.y as i32 + dy).clamp(0, MAP_HEIGHT as i32 - 1) as usize;
            world.get(nx, ny) == Terrain::Water
        })
    }

    /// Greedy fallback when A* path is unavailable or exhausted
    fn move_toward_greedy(&mut self, tx: usize, ty: usize, world: &World, rng: &mut impl Rng) {
        let dx = (tx as i32 - self.x as i32).signum();
        let dy = (ty as i32 - self.y as i32).signum();

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
        (cx, cy)
    }
}

pub fn pick_name(rng: &mut impl Rng, existing: &[String]) -> String {
    let available: Vec<&&str> = ORC_NAMES.iter().filter(|n| !existing.iter().any(|e| e == **n)).collect();
    if available.is_empty() {
        let prefix = ["Gr", "Th", "Kr", "Br", "Dr", "Sk", "Zn", "Gl"];
        let suffix = ["ok", "ag", "ug", "ak", "im", "oz", "ur", "ash"];
        format!(
            "{}{}",
            prefix[rng.gen_range(0..prefix.len())],
            suffix[rng.gen_range(0..suffix.len())]
        )
    } else {
        available[rng.gen_range(0..available.len())].to_string()
    }
}
