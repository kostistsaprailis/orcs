use rand::Rng;

pub const MAP_WIDTH: usize = 300;
pub const MAP_HEIGHT: usize = 150;

#[derive(Clone, Copy, PartialEq)]
pub enum Terrain {
    Grass,
    Tree,
    Rock,
    Water,
    Campfire,
    Food,
    Bush,
    DepletedBush,
    MeatRack,
}

impl Terrain {
    pub fn symbol(&self) -> char {
        match self {
            Terrain::Grass => '·',
            Terrain::Tree => '♣',
            Terrain::Rock => '◆',
            Terrain::Water => '≈',
            Terrain::Campfire => '♨',
            Terrain::Food => '⚘',
            Terrain::Bush => '✿',
            Terrain::DepletedBush => '✿',
            Terrain::MeatRack => '⌸',
        }
    }

    pub fn walkable(&self) -> bool {
        match self {
            Terrain::Rock | Terrain::Water => false,
            _ => true,
        }
    }

    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            Terrain::Grass => Color::DarkGray,
            Terrain::Tree => Color::Rgb(34, 139, 34),
            Terrain::Rock => Color::Gray,
            Terrain::Water => Color::Rgb(65, 105, 225),
            Terrain::Campfire => Color::Rgb(255, 140, 0),
            Terrain::Food => Color::Rgb(255, 100, 180),
            Terrain::Bush => Color::Rgb(220, 50, 80),
            Terrain::DepletedBush => Color::Rgb(80, 60, 60),
            Terrain::MeatRack => Color::Rgb(180, 120, 60),
        }
    }
}

pub struct World {
    pub tiles: Vec<Vec<Terrain>>,
    pub campfire_pos: (usize, usize),
    pub food_stockpile: u32,
    pub regrowth_timers: Vec<(usize, usize, u64)>, // (x, y, regrow_at_tick)
}

impl World {
    pub fn generate(rng: &mut impl Rng) -> Self {
        let mut tiles = vec![vec![Terrain::Grass; MAP_WIDTH]; MAP_HEIGHT];

        // Place campfire near center
        let cx = MAP_WIDTH / 2;
        let cy = MAP_HEIGHT / 2;
        tiles[cy][cx] = Terrain::Campfire;

        // Place meat rack near campfire
        tiles[cy + 2][cx + 2] = Terrain::MeatRack;

        // Scatter trees and rocks
        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                if tiles[y][x] != Terrain::Grass {
                    continue;
                }
                // Keep area around campfire clear
                let dx = (x as i32 - cx as i32).unsigned_abs() as usize;
                let dy = (y as i32 - cy as i32).unsigned_abs() as usize;
                if dx <= 3 && dy <= 3 {
                    continue;
                }
                if rng.gen_ratio(12, 100) {
                    tiles[y][x] = Terrain::Tree;
                } else if rng.gen_ratio(3, 100) {
                    tiles[y][x] = Terrain::Rock;
                }
            }
        }

        // Place berry bushes near trees
        let mut bush_positions = Vec::new();
        for y in 1..MAP_HEIGHT - 1 {
            for x in 1..MAP_WIDTH - 1 {
                if tiles[y][x] == Terrain::Grass {
                    // Check if adjacent to a tree
                    let near_tree = [(0, 1), (0, -1), (1, 0), (-1, 0)]
                        .iter()
                        .any(|&(dx, dy)| {
                            let nx = (x as i32 + dx) as usize;
                            let ny = (y as i32 + dy) as usize;
                            tiles[ny][nx] == Terrain::Tree
                        });
                    if near_tree && rng.gen_ratio(5, 100) {
                        bush_positions.push((x, y));
                    }
                }
            }
        }
        for (x, y) in bush_positions {
            tiles[y][x] = Terrain::Bush;
        }

        // Place several ponds scattered across the map
        let num_ponds = rng.gen_range(8..15);
        for _ in 0..num_ponds {
            let wx = rng.gen_range(5..MAP_WIDTH - 10);
            let wy = rng.gen_range(5..MAP_HEIGHT - 8);
            let pw = rng.gen_range(3..8);
            let ph = rng.gen_range(2..5);
            for dy in 0..ph {
                for dx in 0..pw {
                    let y = wy + dy;
                    let x = wx + dx;
                    if y < MAP_HEIGHT && x < MAP_WIDTH && tiles[y][x] != Terrain::Campfire && tiles[y][x] != Terrain::MeatRack {
                        tiles[y][x] = Terrain::Water;
                    }
                }
            }
        }

        // Ensure there's a pond near the campfire (within 15 tiles)
        let pond_near = (cx.saturating_sub(6), cy.saturating_sub(8));
        for dy in 0..3 {
            for dx in 0..4 {
                let y = pond_near.1 + dy;
                let x = pond_near.0 + dx;
                if y < MAP_HEIGHT && x < MAP_WIDTH && tiles[y][x] == Terrain::Grass {
                    tiles[y][x] = Terrain::Water;
                }
            }
        }

        World {
            tiles,
            campfire_pos: (cx, cy),
            food_stockpile: 3, // start with a small stockpile
            regrowth_timers: Vec::new(),
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Terrain {
        self.tiles[y][x]
    }

    pub fn set(&mut self, x: usize, y: usize, terrain: Terrain) {
        self.tiles[y][x] = terrain;
    }

    pub fn is_walkable(&self, x: usize, y: usize) -> bool {
        if x >= MAP_WIDTH || y >= MAP_HEIGHT {
            return false;
        }
        self.tiles[y][x].walkable()
    }

    pub fn deplete_bush(&mut self, x: usize, y: usize, current_tick: u64) {
        if self.tiles[y][x] == Terrain::Bush {
            self.tiles[y][x] = Terrain::DepletedBush;
            self.regrowth_timers.push((x, y, current_tick + 80));
        }
    }

    pub fn tick_regrowth(&mut self, current_tick: u64) {
        let mut regrown = Vec::new();
        self.regrowth_timers.retain(|&(x, y, regrow_at)| {
            if current_tick >= regrow_at {
                regrown.push((x, y));
                false
            } else {
                true
            }
        });
        for (x, y) in regrown {
            if self.tiles[y][x] == Terrain::DepletedBush {
                self.tiles[y][x] = Terrain::Bush;
            }
        }
    }

    /// Find the nearest tile of a given type from position
    pub fn find_nearest(&self, from_x: usize, from_y: usize, terrain: Terrain) -> Option<(usize, usize)> {
        let mut best: Option<(usize, usize, usize)> = None;
        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                if self.tiles[y][x] == terrain {
                    let dist = from_x.abs_diff(x) + from_y.abs_diff(y);
                    if best.is_none() || dist < best.unwrap().2 {
                        best = Some((x, y, dist));
                    }
                }
            }
        }
        best.map(|(x, y, _)| (x, y))
    }

    /// Find a walkable tile adjacent to the nearest water
    pub fn find_water_adjacent(&self, from_x: usize, from_y: usize) -> Option<(usize, usize)> {
        // Find nearest water tile, then return a walkable neighbor
        if let Some((wx, wy)) = self.find_nearest(from_x, from_y, Terrain::Water) {
            let neighbors = [(0i32, 1i32), (0, -1), (1, 0), (-1, 0)];
            let mut best: Option<(usize, usize, usize)> = None;
            for &(dx, dy) in &neighbors {
                let nx = (wx as i32 + dx).clamp(0, MAP_WIDTH as i32 - 1) as usize;
                let ny = (wy as i32 + dy).clamp(0, MAP_HEIGHT as i32 - 1) as usize;
                if self.is_walkable(nx, ny) {
                    let dist = from_x.abs_diff(nx) + from_y.abs_diff(ny);
                    if best.is_none() || dist < best.unwrap().2 {
                        best = Some((nx, ny, dist));
                    }
                }
            }
            return best.map(|(x, y, _)| (x, y));
        }
        None
    }

    pub fn meat_rack_pos(&self) -> Option<(usize, usize)> {
        let (cx, cy) = self.campfire_pos;
        let x = cx + 2;
        let y = cy + 2;
        if x < MAP_WIDTH && y < MAP_HEIGHT && self.tiles[y][x] == Terrain::MeatRack {
            Some((x, y))
        } else {
            None
        }
    }
}
