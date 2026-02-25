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
    Food, // dropped by player
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
        }
    }
}

pub struct World {
    pub tiles: Vec<Vec<Terrain>>,
    pub campfire_pos: (usize, usize),
}

impl World {
    pub fn generate(rng: &mut impl Rng) -> Self {
        let mut tiles = vec![vec![Terrain::Grass; MAP_WIDTH]; MAP_HEIGHT];

        // Place campfire near center
        let cx = MAP_WIDTH / 2;
        let cy = MAP_HEIGHT / 2;
        tiles[cy][cx] = Terrain::Campfire;

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

        // Place several ponds scattered across the map
        let num_ponds = rng.gen_range(6..12);
        for _ in 0..num_ponds {
            let wx = rng.gen_range(5..MAP_WIDTH - 10);
            let wy = rng.gen_range(5..MAP_HEIGHT - 8);
            let pw = rng.gen_range(3..8);
            let ph = rng.gen_range(2..5);
            for dy in 0..ph {
                for dx in 0..pw {
                    let y = wy + dy;
                    let x = wx + dx;
                    if y < MAP_HEIGHT && x < MAP_WIDTH && tiles[y][x] != Terrain::Campfire {
                        tiles[y][x] = Terrain::Water;
                    }
                }
            }
        }

        World {
            tiles,
            campfire_pos: (cx, cy),
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

    /// Find the nearest tile of a given type from position
    pub fn find_nearest(&self, from_x: usize, from_y: usize, terrain: Terrain) -> Option<(usize, usize)> {
        let mut best: Option<(usize, usize, usize)> = None; // (x, y, dist)
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
}
