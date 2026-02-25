use std::collections::BinaryHeap;
use std::cmp::Ordering;

use crate::world::{MAP_HEIGHT, MAP_WIDTH, Terrain, World};

#[derive(Clone, Eq, PartialEq)]
struct Node {
    x: usize,
    y: usize,
    cost: usize,    // g: cost from start
    priority: usize, // f: cost + heuristic (lower = better)
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other.priority.cmp(&self.priority) // min-heap
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A* pathfinding from (sx, sy) to (gx, gy).
/// Returns a list of (x, y) waypoints excluding the start, including the goal.
/// `allow_tree` lets orcs walk onto tree tiles (for foraging).
/// Max search limit prevents lag on unreachable targets.
pub fn find_path(
    world: &World,
    sx: usize,
    sy: usize,
    gx: usize,
    gy: usize,
    allow_tree: bool,
) -> Option<Vec<(usize, usize)>> {
    if sx == gx && sy == gy {
        return Some(vec![]);
    }

    let max_search = 5000; // limit to prevent lag on huge maps
    let mut visited = vec![vec![false; MAP_WIDTH]; MAP_HEIGHT];
    let mut came_from = vec![vec![(0usize, 0usize); MAP_WIDTH]; MAP_HEIGHT];
    let mut g_cost = vec![vec![usize::MAX; MAP_WIDTH]; MAP_HEIGHT];

    let mut open = BinaryHeap::new();

    g_cost[sy][sx] = 0;
    open.push(Node {
        x: sx,
        y: sy,
        cost: 0,
        priority: heuristic(sx, sy, gx, gy),
    });

    let mut searched = 0;

    while let Some(current) = open.pop() {
        if current.x == gx && current.y == gy {
            return Some(reconstruct_path(&came_from, sx, sy, gx, gy));
        }

        if visited[current.y][current.x] {
            continue;
        }
        visited[current.y][current.x] = true;

        searched += 1;
        if searched > max_search {
            return None;
        }

        // 8-directional neighbors
        for &(dx, dy) in &[
            (-1i32, -1i32), (-1, 0), (-1, 1),
            (0, -1),                 (0, 1),
            (1, -1),  (1, 0),  (1, 1),
        ] {
            let nx = current.x as i32 + dx;
            let ny = current.y as i32 + dy;

            if nx < 0 || ny < 0 || nx >= MAP_WIDTH as i32 || ny >= MAP_HEIGHT as i32 {
                continue;
            }

            let nx = nx as usize;
            let ny = ny as usize;

            if visited[ny][nx] {
                continue;
            }

            // Check walkability (goal tile is always allowed)
            let is_goal = nx == gx && ny == gy;
            if !is_goal {
                let terrain = world.get(nx, ny);
                let passable = world.is_walkable(nx, ny) || (allow_tree && terrain == Terrain::Tree);
                if !passable {
                    continue;
                }
            }

            // Diagonal movement costs more
            let move_cost = if dx != 0 && dy != 0 { 14 } else { 10 };
            let new_cost = current.cost + move_cost;

            if new_cost < g_cost[ny][nx] {
                g_cost[ny][nx] = new_cost;
                came_from[ny][nx] = (current.x, current.y);
                open.push(Node {
                    x: nx,
                    y: ny,
                    cost: new_cost,
                    priority: new_cost + heuristic(nx, ny, gx, gy),
                });
            }
        }
    }

    None // no path found
}

fn heuristic(x: usize, y: usize, gx: usize, gy: usize) -> usize {
    // Chebyshev distance (for 8-directional movement)
    let dx = x.abs_diff(gx);
    let dy = y.abs_diff(gy);
    let diag = dx.min(dy);
    let straight = dx.max(dy) - diag;
    diag * 14 + straight * 10
}

fn reconstruct_path(
    came_from: &[Vec<(usize, usize)>],
    sx: usize,
    sy: usize,
    gx: usize,
    gy: usize,
) -> Vec<(usize, usize)> {
    let mut path = Vec::new();
    let mut cx = gx;
    let mut cy = gy;

    while cx != sx || cy != sy {
        path.push((cx, cy));
        let (px, py) = came_from[cy][cx];
        cx = px;
        cy = py;
    }

    path.reverse();
    path
}
