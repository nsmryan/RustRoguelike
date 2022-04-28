use std::ops::{Index, IndexMut};
use std::collections::{HashSet, HashMap};
use std::cell::RefCell;

use symmetric_shadowcasting::Pos as SymPos;
use symmetric_shadowcasting::compute_fov;

use smallvec::SmallVec;

use itertools::Itertools;

use euclid::*;

use serde_derive::*;

use roguelike_utils::line::*;

use crate::pathing::*;
use crate::utils::*;
use crate::types::*;


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Map {

    pub tiles: Vec<Vec<Tile>>,
    pub fov_cache: RefCell<HashMap<Pos, Vec<Pos>>>,
}

impl Map {
    pub fn with_vec(tiles: Vec<Vec<Tile>>) -> Map {
        let map =
            Map {
                tiles,
                fov_cache: RefCell::new(HashMap::new()),
            };

        return map;
    }

    pub fn from_dims(width: u32, height: u32) -> Map {
        let tiles = vec!(vec!(Tile::empty(); height as usize); width as usize);
        let map =
            Map {
                tiles,
                fov_cache: RefCell::new(HashMap::new()),
            };

        return map;
    }

    pub fn empty() -> Map {
        let map =
            Map {
                tiles: Vec::new(),
                fov_cache: RefCell::new(HashMap::new()),
            };

        return map;
    }

    pub fn is_empty(&self, pos: Pos) -> bool {
        return self[pos].tile_type == TileType::Empty;
    }

    pub fn is_within_bounds(&self, pos: Pos) -> bool {
        let x_bounds = pos.x >= 0 && pos.x < self.width();
        let y_bounds = pos.y >= 0 && pos.y < self.height();

        return x_bounds && y_bounds;
    }

    pub fn size(&self) -> (i32, i32) {
        return (self.tiles.len() as i32, self.tiles[0].len() as i32);
    }

    pub fn width(&self) -> i32 {
        return self.tiles.len() as i32;
    }

    pub fn height(&self) -> i32 {
        return self.tiles[0].len() as i32;
    }

    pub fn is_in_fov_edge(&self, start_pos: Pos, end_pos: Pos, radius: i32, low: bool) -> FovResult {
        if self.is_in_fov(start_pos, end_pos, radius + 1, low) {
            if distance_maximum(start_pos, end_pos) == radius + 1 {
                return FovResult::Edge;
            } else {
                return FovResult::Inside;
            }
        } else {
            return FovResult::Outside;
        }
    }

    pub fn is_in_fov(&self, start_pos: Pos, end_pos: Pos, radius: i32, low: bool) -> bool {
        let mut in_fov = false;

        // check that the position is within the max view distance.
        if distance_maximum(start_pos, end_pos) <= radius {
            if self.is_in_fov_shadowcast(start_pos, end_pos) {
                // so far, the position is in fov
                in_fov = true;

                // make sure there is a clear path, but allow the player to
                // see walls (blocking position is the end_pos tile)
                let path_fov =
                    if low {
                        self.path_blocked_fov_low(start_pos, end_pos)
                    } else {
                        self.path_blocked_fov(start_pos, end_pos)
                    };

                if let Some(blocked) = path_fov {
                    // If we get here, the position is in FOV but blocked.
                    // The only blocked positions that are visible are at the end of the
                    // path and also block tiles (like a wall).
                    // TODO to hide in tall grass
                    //in_fov = end_pos == blocked.end_pos && blocked.blocked_tile && self[end_pos].surface != Surface::Grass;
                    in_fov = end_pos == blocked.end_pos && blocked.blocked_tile;
                } 
            }
        }

        return in_fov;
    }

    pub fn is_in_fov_shadowcast(&self, start_pos: Pos, end_pos: Pos) -> bool {
        if let Some(visible) = self.fov_cache.borrow_mut().get(&start_pos) {
            return visible.contains(&end_pos);
        }

        // NOTE(perf) this should be correct- shadowcasting is symmetrical, so 
        // we either need a precomputed start-to-end or end-to-start
        // calculation, but not both.
        if let Some(visible) = self.fov_cache.borrow_mut().get(&end_pos) {
            return visible.contains(&start_pos);
        }

        // NOTE(perf) this pre-allocation speeds up FOV significantly
        let mut visible_positions = Vec::with_capacity(120);

        let mut mark_fov = |sym_pos: SymPos| {
            let pos = Pos::new(sym_pos.0 as i32, sym_pos.1 as i32);
            visible_positions.push(pos);
        };

        let mut is_blocking = |sym_pos: SymPos| {
            let pos = Pos::new(sym_pos.0 as i32, sym_pos.1 as i32);

            if !self.is_within_bounds(pos) {
                return true;
            }

            let blocked_sight = self[pos].block_sight;

            return blocked_sight;
        };

        compute_fov((start_pos.x as isize, start_pos.y as isize), &mut is_blocking, &mut mark_fov);

        let in_fov = visible_positions.contains(&end_pos);
        self.fov_cache.borrow_mut().insert(start_pos, visible_positions);

        return in_fov;
    }

    pub fn is_in_fov_direction(&self, start_pos: Pos, end_pos: Pos, radius: i32, dir: Direction, low: bool) -> bool {
        if start_pos == end_pos {
            return true;
        } else if self.is_in_fov(start_pos, end_pos, radius, low) {
            return visible_in_direction(start_pos, end_pos, dir);
        } else {
            return false;
        }
    }

    pub fn path_clear_of_obstacles(&self, start: Pos, end: Pos) -> bool {
        let line = line(start, end);

        let path_blocked =
            line.into_iter().any(|point| self[Pos::from(point)].block_move);

        return !path_blocked;
    }

    pub fn pos_in_radius(&self, start: Pos, radius: i32) -> Vec<Pos> {
        let mut circle_positions = HashSet::new();

        // for each position on the edges of a square around the point, with the
        // radius as the distance in x/y, add to a set.
        // duplicates will be removed, leaving only points within the radius.
        for x in (start.x - radius)..(start.x + radius) {
            for y in (start.y - radius)..(start.y + radius) {
                let line = line(start, Pos::new(x, y));

                // get points to the edge of square, filtering for points within the given radius
                for point in line.into_iter() {
                    let point = Pos::from(point);
                    if distance(start, point) < radius {
                        circle_positions.insert(Pos::from(point));
                    }
                }
            }
        }

        return circle_positions.iter().map(|pos| *pos).collect();
    }

    pub fn neighbors(&self, pos: Pos) -> SmallVec<[Pos; 8]> {
        let neighbors = [(1, 0),  (1, 1),  (0, 1), 
                         (-1, 1), (-1, 0), (-1, -1),
                         (0, -1), (1, -1)];

        let mut result = SmallVec::new();
        for delta in neighbors.iter() {
            let new_pos = pos + Vector2D::new(delta.0, delta.1);
            if self.is_within_bounds(new_pos) {
                result.push(new_pos);
            }
        }

        return result;
    }

    pub fn cardinal_neighbors(&self, pos: Pos) -> SmallVec<[Pos; 4]> {
        let neighbors = [(1, 0), (0, 1), (-1, 0), (0, -1),];

        let mut result = SmallVec::new();
        for delta in neighbors.iter() {
            let new_pos = pos + Vector2D::new(delta.0, delta.1);
            if self.is_within_bounds(new_pos) {
                result.push(new_pos);
            }
        }

        return result;
    }

    pub fn reachable_neighbors(&self, pos: Pos) -> SmallVec<[Pos; 8]> {
        let neighbors = [(1, 0),  (1, 1),  (0, 1), 
                         (-1, 1), (-1, 0), (-1, -1),
                         (0, -1), (1, -1)];

        let mut result = SmallVec::new();

        for delta in neighbors.iter() {
            let end_pos = Pos::new(pos.x + delta.0, pos.y + delta.1);
            if self.path_blocked_move(pos, end_pos).is_none() {
                result.push(pos + Vector2D::new(delta.0, delta.1));
            }
        }

        return result;
    }

    pub fn get_all_pos(&self) -> Vec<Pos> {
        let (width, height) = self.size();
        return (0..width).cartesian_product(0..height)
                         .map(|pair| Pos::from(pair))
                         .collect::<Vec<Pos>>();
    }

    pub fn get_empty_pos(&self) -> Vec<Pos> {
        let (width, height) = self.size();
        return (0..width).cartesian_product(0..height)
                         .map(|pair| Pos::from(pair))
                         .filter(|pos| self[*pos].tile_type != TileType::Wall)
                         .filter(|pos| self[*pos].tile_type != TileType::Water)
                         .collect::<Vec<Pos>>();
    }

    pub fn get_wall_pos(&self) -> Vec<Pos> {
        let (width, height) = self.size();
        return (0..width).cartesian_product(0..height)
                         .map(|pair| Pos::from(pair))
                         .filter(|pos| self[*pos].tile_type == TileType::Wall)
                         .collect::<Vec<Pos>>();
    }

    pub fn clamp(&self, pos: Pos) -> Pos {
        let (width, height) = self.size();
        let new_x = std::cmp::min(width - 1, std::cmp::max(0, pos.x));
        let new_y = std::cmp::min(height - 1, std::cmp::max(0, pos.y));
        return Pos::new(new_x, new_y);
    }

    pub fn place_intertile_wall(&mut self, pos: Pos, material: Surface, direction: Direction) {
        match direction {
            Direction::Left => {
                let left_pos = move_x(pos, -1);
                let tile_supports_wall = self.is_within_bounds(left_pos) && 
                                         !self[left_pos].does_tile_block(BlockedType::Move) && 
                                         self[pos].tile_type != TileType::Water;

                if tile_supports_wall && self[pos].left_wall == Wall::Empty {
                    self[pos].left_wall = Wall::ShortWall;
                    self[pos].left_material = material;
                }
            }

            Direction::Right => {
                let pos = move_x(pos, 1);
                let tile_supports_wall = self.is_within_bounds(pos) && 
                                         !self[pos].does_tile_block(BlockedType::Move) && 
                                         self[pos].tile_type != TileType::Water;

                if tile_supports_wall && self[pos].left_wall == Wall::Empty {
                    self[pos].left_wall = Wall::ShortWall;
                    self[pos].left_material = material;
                }
            }

            Direction::Up => {
                let pos = move_y(pos, -1);
                let tile_supports_wall = self.is_within_bounds(pos) && 
                                         !self[pos].does_tile_block(BlockedType::Move) && 
                                         self[pos].tile_type != TileType::Water;

                if tile_supports_wall && self[pos].bottom_wall == Wall::Empty {
                    self[pos].bottom_wall = Wall::ShortWall;
                    self[pos].bottom_material = material;
                }
            }

            Direction::Down => {
                let down_pos = move_y(pos, 1);
                let tile_supports_wall = self.is_within_bounds(down_pos) &&
                                         !self[down_pos].does_tile_block(BlockedType::Move) &&
                                         self[down_pos].tile_type != TileType::Water;

                if tile_supports_wall && self[pos].bottom_wall == Wall::Empty {
                    self[pos].bottom_wall = Wall::ShortWall;
                    self[pos].bottom_material = material;
                }
            }

            Direction::DownLeft | Direction::DownRight | Direction::UpLeft | Direction::UpRight => {
                panic!("Placing an intertile wall on a diagonal makes no sense!");
            }
        }
    }

    pub fn chrs(&self) -> Vec<char> {
        let mut chrs = Vec::new();
        for y in 0..self.height() {
            for x in 0..self.width() {
                for chr in self[(x, y)].chrs().iter() {
                    chrs.push(*chr);
                }
            }
        }

        return chrs;
    }

    pub fn tile_summary(&self) -> Vec<Tile> {
        let mut tile_set = HashSet::new();

        for y in 0..self.height() {
            for x in 0..self.width() {
                let tile = self[(x, y)];
                tile_set.insert(tile);
            }
        }

        return tile_set.iter().map(|t| *t).collect::<Vec<Tile>>();
    }

    pub fn compact_chrs(&self) -> String {
        let mut chrs = Vec::new();

        let summary = self.tile_summary();
        if summary.len() > 26 {
            panic!("Not enough letters!");
        }

        for chr in format!("{}!{}!", self.width(), self.height()).chars() {
            chrs.push(chr);
        }

        // push summary tiles
        for common_tile in summary.iter() {
            for chr in common_tile.chrs().iter() {
                chrs.push(*chr);
            }
        }
        // separate by !.
        chrs.push('!');

        // emit map tiles
        for y in 0..self.height() {
            for x in 0..self.width() {
                let tile = self[(x, y)];

                if let Some(tile_index) = summary.iter().position(|t| *t == tile) {
                    // emit tile index character
                    let chr;
                    if tile.explored {
                        chr = char::from('A' as u8 + tile_index as u8);
                    } else {
                        chr = char::from('a' as u8 + tile_index as u8);
                    }
                    chrs.push(chr);
                }
            }
        }

        let mut read_index = 0;
        // skip to start of map tiles
        while chrs[read_index] != '!' {
            read_index += 1;
        }
        // skip '!' character
        read_index += 1;

        let mut write_index = read_index;
        while read_index < chrs.len() {
            let cur_char = chrs[read_index];
            let mut index_run_end = read_index;
            for offset in 1..11 {
                if offset + read_index >= chrs.len() {
                    break;
                }

                if chrs[offset + read_index] != chrs[read_index] {
                    break;
                }
                index_run_end += 1;
            }

            let run_length = index_run_end - read_index + 1;
            if run_length > 2 {
                let count_chr;
                if run_length == 11 {
                    count_chr = '1';
                } else if run_length == 10 {
                    count_chr = '0';
                } else {
                    count_chr = ('0' as u8 + run_length as u8) as char;
                }
                chrs[write_index] = count_chr;
                write_index += 1;
                chrs[write_index] = cur_char;
                write_index += 1;

                read_index = index_run_end + 1;
            } else {
                chrs[write_index] = chrs[read_index];
                write_index += 1;
                read_index += 1;
            }
        }
        chrs.truncate(write_index);

        return chrs.iter().collect::<String>();
    }
}

impl Index<(i32, i32)> for Map {
    type Output = Tile;

    fn index(&self, index: (i32, i32)) -> &Tile {
        &self.tiles[index.0 as usize][index.1 as usize]
    }
}

impl IndexMut<(i32, i32)> for Map {
    fn index_mut(&mut self, index: (i32, i32)) -> &mut Tile {
        self.fov_cache.borrow_mut().clear();
        &mut self.tiles[index.0 as usize][index.1 as usize]
    }
}

impl Index<Pos> for Map {
    type Output = Tile;

    fn index(&self, index: Pos) -> &Tile {
        &self.tiles[index.x as usize][index.y as usize]
    }
}

impl IndexMut<Pos> for Map {
    fn index_mut(&mut self, index: Pos) -> &mut Tile {
        self.fov_cache.borrow_mut().clear();
        &mut self.tiles[index.x as usize][index.y as usize]
    }
}


