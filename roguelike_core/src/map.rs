use std::ops::{Index, IndexMut};
use std::collections::HashSet;
use std::iter;

use rand::prelude::*;

use pathfinding::directed::astar::astar;

use smallvec::SmallVec;

use itertools::Itertools;

use log::trace;

use doryen_fov::{MapData, FovAlgorithm, FovRestrictive};

use euclid::*;

use serde_derive::*;

use crate::types::*;
use crate::utils::*;
use crate::movement::Direction;


#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum AoeEffect {
    Sound,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Aoe {
    pub effect: AoeEffect,
    pub positions: Vec<Vec<Pos>>,
}

impl Aoe {
    pub fn new(effect: AoeEffect, positions: Vec<Vec<Pos>>) -> Aoe {
        return Aoe {
            effect, 
            positions,
        };
    }

    pub fn positions(&self) -> Vec<Pos> {
        let mut positions = Vec::new();

        for pos_vec in self.positions.iter() {
            for pos in pos_vec.iter() {
                positions.push(*pos);
            }
        }

        return positions;
    }
}

/// This structure describes a movement between two
/// tiles that was blocked due to a wall or blocked tile.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Blocked {
    pub start_pos: Pos,
    pub end_pos: Pos,
    pub direction: Direction,
    pub blocked_tile: bool,
    pub wall_type: Wall,
}

impl Blocked {
    pub fn new(start_pos: Pos,
               end_pos: Pos,
               direction: Direction,
               blocked_tile: bool,
               wall_type: Wall) -> Blocked {
        return Blocked { start_pos,
        end_pos,
        direction,
        blocked_tile,
        wall_type,
        };
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum MapLoadConfig {
    Random,
    TestMap,
    TestWall,
    Empty,
    TestCorner,
    TestPlayer,
    FromFile,
}

impl Default for MapLoadConfig {
    fn default() -> MapLoadConfig {
        return MapLoadConfig::Random;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Surface {
    Floor,
    Rubble,
    Grass,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[repr(C, packed)]
pub struct Tile {
    pub blocked: bool,
    pub block_sight: bool,
    pub explored: bool,
    pub tile_type: TileType,
    pub bottom_wall: Wall,
    pub left_wall: Wall,
    pub chr: u8,
    pub surface: Surface,
}

impl Tile {
    pub fn empty() -> Self {
        Tile { blocked: false,
        block_sight: false,
        explored: false,
        tile_type: TileType::Empty,
        bottom_wall: Wall::Empty,
        left_wall: Wall::Empty,
        chr: ' ' as u8,
        surface: Surface::Floor,
        }
    }

    pub fn water() -> Self {
        Tile { blocked: true,
        block_sight: false,
        explored: false,
        tile_type: TileType::Water,
        bottom_wall: Wall::Empty,
        left_wall: Wall::Empty,
        chr: ' ' as u8,
        surface: Surface::Floor,
        }
    }

    pub fn wall() -> Self {
        return Tile::wall_with(' ');
    }

    pub fn wall_with(chr: char) -> Self {
        Tile { blocked: true,
        block_sight: true,
        explored: false,
        tile_type: TileType::Wall,
        bottom_wall: Wall::Empty,
        left_wall: Wall::Empty,
        chr: chr as u8,
        surface: Surface::Floor,
        }
    }

    pub fn short_wall() -> Self {
        return Tile::short_wall_with(' ');
    }

    pub fn short_wall_with(chr: char) -> Self {
        Tile { blocked: true,
        block_sight: false,
        explored: false,
        tile_type: TileType::ShortWall,
        bottom_wall: Wall::Empty,
        left_wall: Wall::Empty,
        chr: chr as u8,
        surface: Surface::Floor,
        }
    }

    pub fn exit() -> Self {
        Tile { blocked: false,
        block_sight: false,
        explored: false,
        tile_type: TileType::Exit,
        bottom_wall: Wall::Empty,
        left_wall: Wall::Empty,
        chr: ' ' as u8,
        surface: Surface::Floor,
        }
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum TileType {
    Empty,
    ShortWall,
    Wall,
    Water,
    Exit,
}


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Obstacle {
    Block,
    Wall,
    ShortWall,
    Square,
    LShape,
    Building,
}

impl Obstacle {
    pub fn all_obstacles() -> Vec<Obstacle> {
        vec!(Obstacle::Block,  Obstacle::Wall,   Obstacle::ShortWall,
             Obstacle::Square, Obstacle::LShape, Obstacle::Building)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum Wall {
    Empty,
    ShortWall,
    TallWall,
}

impl Wall {
    pub fn no_wall(&self) -> bool {
        match self {
            Wall::Empty => true,
            Wall::ShortWall => false,
            Wall::TallWall => false,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Map {
    pub tiles: Vec<Vec<Tile>>,
    fov: MapData,
    fov_pos: Pos,
    fov_radius: i32,
}

impl Map {
    pub fn with_vec(tiles: Vec<Vec<Tile>>) -> Map {
        let width = tiles.len();
        let height = tiles[0].len();
        let mut map =
            Map {
                tiles,
                fov: MapData::new(width, height),
                fov_pos: Pos::new(0, 0),
                fov_radius: 1,
            };

        map.update_map();

        return map;
    }

    pub fn from_dims(width: usize, height: usize) -> Map {
        let tiles = vec!(vec!(Tile::empty(); height); width);
        let mut map =
            Map {
                tiles,
                fov: MapData::new(width, height),
                fov_pos: Pos::new(0, 0),
                fov_radius: 1,
            };

        map.update_map();

        return map;
    }

    pub fn empty() -> Map {
        let map =
            Map {
                tiles: Vec::new(),
                fov: MapData::new(1, 1),
                fov_pos: Pos::new(0, 0),
                fov_radius: 1,
            };

        return map;
    }

    pub fn blocked_left(&self, pos: Pos) -> bool {
        let offset = Pos::new(pos.x - 1, pos.y);
        if !self.is_within_bounds(offset) || !self.is_within_bounds(pos) {
            return true;
        }

        let blocking_wall = self[pos].left_wall != Wall::Empty;
        let blocking_tile = self[offset].blocked;
        return blocking_wall || blocking_tile;
    }

    pub fn blocked_right(&self, pos: Pos) -> bool {
        let offset = Pos::new(pos.x + 1, pos.y);
        if !self.is_within_bounds(pos) || !self.is_within_bounds(pos) { 
            return true;
        }

        let blocking_wall = self[offset].left_wall != Wall::Empty;
        let blocking_tile = self[offset].blocked;
        return blocking_wall || blocking_tile;
    }

    pub fn blocked_down(&self, pos: Pos) -> bool {
        let offset = Pos::new(pos.x, pos.y + 1);
        if !self.is_within_bounds(pos) || !self.is_within_bounds(pos) {
            return true;
        }

        let blocking_wall = self[pos].bottom_wall != Wall::Empty;
        let blocking_tile = self[offset].blocked;
        return blocking_wall || blocking_tile;
    }

    pub fn blocked_up(&self, pos: Pos) -> bool {
        let offset = Pos::new(pos.x, pos.y - 1);
        if !self.is_within_bounds(pos) || !self.is_within_bounds(pos) {
            return true;
        }

        let blocking_wall = self[offset].bottom_wall != Wall::Empty;
        let blocking_tile = self[offset].blocked;
        return blocking_wall || blocking_tile;
    }

    pub fn is_blocked_by_wall(&self, start_pos: Pos, dx: i32, dy: i32) -> Option<Blocked> {
        if dx == 0 && dy == 0 {
            return None;
        }

        let (end_x, end_y) = (start_pos.x + dx, start_pos.y + dy);

        let line = line(start_pos, Pos::new(end_x, end_y));

        let dir = Direction::from_dxy(dx, dy).expect("Check for blocking wall with no movement?");

        let positions = iter::once(start_pos).chain(line.into_iter());

        let mut blocked;
        let mut found_blocker = false;
        for (pos, target_pos) in positions.tuple_windows() {
            let (x, y) = (pos.x, pos.y);

            blocked = Blocked::new(pos, target_pos, dir, false, Wall::Empty);

            // if the target position is out of bounds, we are blocked
            if !self.is_within_bounds(target_pos) {
                blocked.blocked_tile = true;

                // continuing to check after finding an out-of-bounds
                // position results in a panic, so stop now.
                return Some(blocked);
            }

            // if moving into a blocked tile, we are blocked
            if self[target_pos].blocked {
                blocked.blocked_tile = true;
                found_blocker = true;
            }

            let move_dir = target_pos - Vector2D::new(x, y);

            // used for diagonal movement checks
            let x_moved = Pos::new(target_pos.x, y);
            let y_moved = Pos::new(x, target_pos.y);
            
            match Direction::from_dxy(move_dir.x, move_dir.y).unwrap() {
                Direction::Right | Direction::Left => {
                    let mut left_wall_pos = pos;
                    // moving right
                    if move_dir.x >= 1 {
                        left_wall_pos = Pos::new(x + move_dir.x, y);
                    }

                    if self.is_within_bounds(left_wall_pos) &&
                        self[left_wall_pos].left_wall != Wall::Empty {
                            blocked.wall_type = self[left_wall_pos].left_wall;
                            found_blocker = true;
                    }
                }

                Direction::Up | Direction::Down => {
                    let mut bottom_wall_pos = Pos::new(x, y + move_dir.y);
                    if move_dir.y >= 1 {
                        bottom_wall_pos = pos;
                    }

                    if self.is_within_bounds(bottom_wall_pos) &&
                        self[bottom_wall_pos].bottom_wall != Wall::Empty {
                            blocked.wall_type = self[bottom_wall_pos].bottom_wall;
                            found_blocker = true;
                    }
                }

                Direction::DownRight => {
                    if self.blocked_right(pos) && self.blocked_down(pos) {
                        blocked.wall_type = self[pos].bottom_wall;
                        found_blocker = true;
                    }

                    if self.blocked_right(move_y(pos, -1)) && self.blocked_down(move_x(pos, 1)) {
                        let blocked_pos = add_pos(pos, Pos::new(-1, 1));
                        if self.is_within_bounds(blocked_pos) {
                            blocked.wall_type = self[blocked_pos].bottom_wall;
                        }
                        found_blocker = true;
                    }

                    if self.blocked_right(pos) && self.blocked_right(y_moved) {
                        blocked.wall_type = self[move_x(pos, 1)].left_wall;
                        found_blocker = true;
                    }

                    if self.blocked_down(pos) && self.blocked_down(x_moved) {
                        blocked.wall_type = self[pos].bottom_wall;
                        found_blocker = true;
                    }
                }

                Direction::UpRight => {
                    if self.blocked_up(pos) && self.blocked_right(pos) {
                        blocked.wall_type = self[move_y(pos, -1)].bottom_wall;
                        found_blocker = true;
                    }

                    if self.blocked_up(move_x(pos, 1)) && self.blocked_right(move_y(pos, -1)) {
                        let blocked_pos = add_pos(pos, Pos::new(1, -1));
                        if self.is_within_bounds(blocked_pos) {
                            blocked.wall_type = self[blocked_pos].bottom_wall;
                        }
                        found_blocker = true;
                    }

                    if self.blocked_right(pos) && self.blocked_right(y_moved) {
                        blocked.wall_type = self[move_x(pos, 1)].left_wall;
                        found_blocker = true;
                    }

                    if self.blocked_up(pos) && self.blocked_up(x_moved) {
                        blocked.wall_type = self[move_y(pos, -1)].bottom_wall;
                        found_blocker = true;
                    }
                }

                Direction::DownLeft => {
                    if self.blocked_left(pos) && self.blocked_down(pos) {
                        blocked.wall_type = self[pos].left_wall;
                        found_blocker = true;
                    }

                    if self.blocked_left(move_y(pos, 1)) && self.blocked_down(move_x(pos, -1)) {
                        let blocked_pos = add_pos(pos, Pos::new(1, -1));
                        if self.is_within_bounds(blocked_pos) {
                            blocked.wall_type = self[blocked_pos].left_wall;
                        }
                        found_blocker = true;
                    }

                    if self.blocked_left(pos) && self.blocked_left(y_moved) {
                        blocked.wall_type = self[pos].left_wall;
                        found_blocker = true;
                    }

                    if self.blocked_down(pos) && self.blocked_down(x_moved) {
                        blocked.wall_type = self[pos].bottom_wall;
                        found_blocker = true;
                    }
                }

                Direction::UpLeft => {
                    if self.blocked_left(move_y(pos, -1)) && self.blocked_up(move_x(pos, -1)) {
                        let blocked_pos = add_pos(pos, Pos::new(-1, -1));
                        if self.is_within_bounds(blocked_pos) {
                            blocked.wall_type = self[blocked_pos].left_wall;
                        }
                        found_blocker = true;
                    }

                    if self.blocked_left(pos) && self.blocked_up(pos) {
                        blocked.wall_type = self[pos].left_wall;
                        found_blocker = true;
                    }

                    if self.blocked_left(pos) && self.blocked_left(y_moved) {
                        blocked.wall_type = self[pos].left_wall;
                        found_blocker = true;
                    }

                    if self.blocked_up(pos) && self.blocked_up(x_moved) {
                        let blocked_pos = move_y(pos, -1);
                        if self.is_within_bounds(blocked_pos) {
                            blocked.wall_type = self[blocked_pos].bottom_wall;
                        }
                        found_blocker = true;
                    }
                }
            }

            if found_blocker {
                return Some(blocked);
            }
        }

        return None;
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

    pub fn is_in_fov_direction(&mut self, start_pos: Pos, end_pos: Pos, radius: i32, dir: Direction) -> bool {
        if start_pos == end_pos {
            return true;
        } else if self.is_in_fov(start_pos, end_pos, radius) {
            let pos_diff = sub_pos(end_pos, start_pos);
            let x_sig = pos_diff.x.signum();
            let y_sig = pos_diff.y.signum();

            match dir {
                Direction::Up => {
                    if y_sig < 1 {
                        return true;
                    }
                }

                Direction::Down => {
                    if y_sig > -1 {
                        return true;
                    }
                }

                Direction::Left => {
                    if x_sig < 1 {
                        return true;
                    }
                }

                Direction::Right => {
                    if x_sig > -1 {
                        return true;
                    }
                }
                Direction::DownLeft => {
                    if pos_diff.x - pos_diff.y < 0 {
                        return true;
                    }
                }

                Direction::DownRight => {
                    if pos_diff.x + pos_diff.y >= 0 {
                        return true;
                    }
                }

                Direction::UpLeft => {
                    if pos_diff.x + pos_diff.y <= 0 {
                        return true;
                    }
                }

                Direction::UpRight => {
                    if pos_diff.x - pos_diff.y > 0 {
                        return true;
                    }
                }
            }
        }
            
        return false;
    }

    pub fn is_in_fov(&mut self, start_pos: Pos, end_pos: Pos, radius: i32) -> bool {
        if start_pos == end_pos {
            return true;
        }

        if !self.is_within_bounds(start_pos) || !self.is_within_bounds(end_pos) {
            return false;
        }

        let within_radius = distance(start_pos, end_pos) < radius;
        if !within_radius {
            return false;
        }

        if self.fov_pos != start_pos {
            self.compute_fov(start_pos, radius);
        }

        let offset = Pos::new(end_pos.x - start_pos.x,
                              end_pos.y - start_pos.y);
        let blocked =
            self.is_blocked_by_wall(start_pos, offset.x, offset.y);

        let mut wall_in_path = false;
        if let Some(blocked) = blocked {
            let at_end = blocked.end_pos == end_pos;
            let no_wall = blocked.wall_type.no_wall();
            let visible_wall = blocked.blocked_tile && no_wall;
            wall_in_path = !(at_end && visible_wall);
        }

        let is_visible =
            self.fov.is_in_fov(end_pos.x as usize, end_pos.y as usize);

        let is_in_fov = !wall_in_path && is_visible;

        return is_in_fov;
    }

    pub fn path_clear_of_obstacles(&self, start: Pos, end: Pos) -> bool {
        let line = line(start, end);

        let path_blocked =
            line.into_iter().any(|point| self[Pos::from(point)].blocked);

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

    pub fn reachable_neighbors(&self, pos: Pos) -> SmallVec<[Pos; 8]> {
        let neighbors = [(1, 0),  (1, 1),  (0, 1), 
                         (-1, 1), (-1, 0), (-1, -1),
                         (0, -1), (1, -1)];

        let mut result = SmallVec::new();

        for delta in neighbors.iter() {
            if self.is_blocked_by_wall(pos, delta.0, delta.1).is_none() {
                result.push(pos + Vector2D::new(delta.0, delta.1));
            }
        }

        return result;
    }

    pub fn set_cell(&mut self, x: i32, y: i32, transparent: bool) {
        self.fov.set_transparent(x as usize, y as usize, transparent);
    }

    pub fn compute_fov(&mut self, pos: Pos, view_radius: i32) {
        self.fov_pos = pos;
        self.fov_radius = view_radius;
        FovRestrictive::new().compute_fov(&mut self.fov,
                                          pos.x as usize,
                                          pos.y as usize,
                                          view_radius as usize,
                                          true);
    }

    pub fn update_map(&mut self) {
        let dims = (self.width(), self.height());

        for y in 0..dims.1 {
            for x in 0..dims.0 {
                let transparent = !self.tiles[x as usize][y as usize].block_sight;
                self.fov.set_transparent(x as usize, y as usize, transparent);
            }
        }

        self.compute_fov(self.fov_pos, self.fov_radius);
    }

    pub fn aoe_fill(&self, aoe_effect: AoeEffect, start: Pos, radius: usize) -> Aoe {
        let flood = self.floodfill(start, radius);

        let mut aoe_dists = vec![Vec::new(); radius + 1];

        let blocked_radius = if radius > 2 {
            radius as i32 - 2
        } else {
            0
        };

        for pos in flood.iter() {
            let dist = distance(start, *pos);

            let dt = sub_pos(*pos, start);
            let is_blocked = self.is_blocked_by_wall(start, dt.x, dt.y).is_some();
            if !is_blocked || (is_blocked && dist <= blocked_radius) {
                if dist as usize == aoe_dists.len() {
                    dbg!(dist, radius, pos);
                }
                aoe_dists[dist as usize].push(*pos);
            }
        }
        let aoe = Aoe::new(aoe_effect, aoe_dists);

        return aoe;
    }

    pub fn floodfill(&self, start: Pos, radius: usize) -> Vec<Pos> {
        let mut flood: Vec<Pos> = Vec::new();

        let mut seen: Vec<Pos> = Vec::new();
        let mut current: Vec<Pos> = Vec::new();
        current.push(start);
        seen.push(start);
        flood.push(start);

        for _index in 0..radius {
            let last = current.clone();
            current.clear();
            for pos in last.iter() {
                let adj = astar_neighbors(self, start, *pos, Some(radius as i32));
                for (next_pos, _cost) in adj {
                    if !seen.contains(&next_pos) {
                        // record having seen this position.
                        seen.push(next_pos);
                        current.push(next_pos);
                        flood.push(next_pos);
                    }
                }
            }
        }

        return flood;
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
        &mut self.tiles[index.x as usize][index.y as usize]
    }
}


pub fn near_tile_type(map: &Map, position: Pos, tile_type: TileType) -> bool {
    let neighbor_offsets: Vec<(i32, i32)>
        = vec!((1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1));

    let mut near_given_tile = false;

    for offset in neighbor_offsets {
        let offset = Pos::from(offset);
        let neighbor_position = move_by(position, offset);

        if map.is_within_bounds(neighbor_position) &&
           map[neighbor_position].tile_type == tile_type {
            near_given_tile = true;
            break;
        }
    }

    return near_given_tile;
}

pub fn random_offset(rng: &mut SmallRng, radius: i32) -> Pos {
    return Pos::new(rng.gen_range(-radius, radius),
                    rng.gen_range(-radius, radius));
}

pub fn pos_in_radius(pos: Pos, radius: i32, rng: &mut SmallRng) -> Pos {
    let offset = Vector2D::new(rng.gen_range(-radius, radius),
                               rng.gen_range(-radius, radius));
    return pos + offset;
}

pub fn place_block(map: &mut Map, start: Pos, width: i32, tile: Tile) -> Vec<Pos> {
    let mut positions = Vec::new();

    for x in 0..width {
        for y in 0..width {
            let pos = start + Vector2D::new(x, y);
            map[pos] = tile;
            positions.push(pos);
        }
    }

    return positions;
}

pub fn place_line(map: &mut Map, start: Pos, end: Pos, tile: Tile) -> Vec<Pos> {
    let mut positions = Vec::new();
    let line = line(start, end);

    for pos in line {
        if map.is_within_bounds(pos) {
            map[pos] = tile;
            positions.push(pos);
        }
    }

    positions
}

pub fn add_obstacle(map: &mut Map, pos: Pos, obstacle: Obstacle, rng: &mut SmallRng) {
    match obstacle {
        Obstacle::Block => {
            map.tiles[pos.x as usize][pos.y as usize] = Tile::wall();
        }

        Obstacle::Wall => {
            let end_pos = if rng.gen_bool(0.5) {
                move_x(pos, 3)
            } else {
                move_y(pos, 3)
            };
            place_line(map, pos, end_pos, Tile::wall());
        }

        Obstacle::ShortWall => {
            let end_pos = if rng.gen_bool(0.5) {
                move_x(pos, 3)
            } else {
                move_y(pos, 3)
            };
            place_line(map, pos, end_pos, Tile::short_wall());
        }

        Obstacle::Square => {
            place_block(map, pos, 2, Tile::wall());
        }

        Obstacle::LShape => {
            let mut dir = 1;
            if rng.gen_bool(0.5) {
                dir = -1;
            }

            if rng.gen_bool(0.5) {
                for x in 0..3 {
                    map.tiles[pos.x as usize + x][pos.y as usize] = Tile::wall();
                }
                map.tiles[pos.x as usize][(pos.y + dir) as usize] = Tile::wall();
            } else {
                for y in 0..3 {
                    map.tiles[pos.x as usize][pos.y as usize + y] = Tile::wall();
                }
                map.tiles[(pos.x + dir) as usize][pos.y as usize] = Tile::wall();
            }
        }

        Obstacle::Building => {
            let size = 2;

            let mut positions = vec!();
            positions.append(&mut place_line(map, move_by(pos, Pos::new(-size, size)),  move_by(pos, Pos::new(size,  size)), Tile::wall()));
            positions.append(&mut place_line(map, move_by(pos, Pos::new(-size, size)),  move_by(pos, Pos::new(-size, -size)), Tile::wall()));
            positions.append(&mut place_line(map, move_by(pos, Pos::new(-size, -size)), move_by(pos, Pos::new(size, -size)), Tile::wall()));
            positions.append(&mut place_line(map, move_by(pos, Pos::new(size, -size)),  move_by(pos, Pos::new(size,  size)), Tile::wall()));

            for _ in 0..rng.gen_range(0, 10) {
                positions.swap_remove(rng.gen_range(0, positions.len()));
            }
        }
    }
}

pub fn astar_path(map: &Map,
                  start: Pos,
                  end: Pos,
                  max_dist: Option<i32>,
                  cost_fn: Option<fn(Pos, Pos, &Map) -> i32>) -> Vec<Pos> {
    let result;

    trace!("astar_path {} {}", start, end);

    let maybe_results = 
        astar(&start,
              |&pos| astar_neighbors(map, start, pos, max_dist),
              |&pos| {
                  if let Some(fun) = &cost_fn { 
                      fun(start, pos, map)
                  } else {
                      distance(pos, end) as i32
                  }
              },
              |&pos| pos == end);

    if let Some((results, _cost)) = maybe_results {
        result = results.iter().map(|p| *p).collect::<Vec<Pos>>();
    } else {
        result = Vec::new();
    }

    return result;
}

fn astar_neighbors(map: &Map, start: Pos, pos: Pos, max_dist: Option<i32>) -> SmallVec<[(Pos, i32); 8]> {
      if let Some(max_dist) = max_dist {
          if distance(start, pos) > max_dist {
              return SmallVec::new();
          }
      }

      map.reachable_neighbors(pos)
         .iter()
         .map(|pos| (*pos, 1))
         .collect::<SmallVec<[(Pos, i32); 8]>>()
}

#[test]
fn test_blocked_by_wall_right() {
    let mut map = Map::from_dims(10, 10);

    let pos = Pos::new(5, 5);
    map[pos].left_wall = Wall::ShortWall;
  
    map.update_map();

    let left_of_wall = Pos::new(4, 5);
    let blocked = map.is_blocked_by_wall(left_of_wall, 1, 0);
    assert_eq!(blocked.map(|b| b.wall_type == Wall::ShortWall), Some(true));

    assert!(map.is_blocked_by_wall(pos, 1, 0).is_none());

    let two_left_of_wall = Pos::new(3, 5);
    assert_eq!(map.is_blocked_by_wall(two_left_of_wall, 1, 0), None);
}

#[test]
fn test_blocked_by_wall_up() {
    let mut map = Map::from_dims(10, 10);

    let pos = Pos::new(5, 5);
    map[pos].bottom_wall = Wall::ShortWall;
  
    map.update_map();

    let blocked = map.is_blocked_by_wall(Pos::new(5, 6), 0, -1);
    assert_eq!(blocked.map(|b| b.wall_type), Some(Wall::ShortWall));
    assert!(map.is_blocked_by_wall(Pos::new(5, 5), 0, -1).is_none());
    assert!(map.is_blocked_by_wall(Pos::new(5, 4), 0, -1).is_none());
}

#[test]
fn test_blocked_by_wall_down() {
    let mut map = Map::from_dims(10, 10);

    let pos = Pos::new(5, 5);
    map[pos].bottom_wall = Wall::ShortWall;
  
    map.update_map();

    let blocked = map.is_blocked_by_wall(Pos::new(5, 5), 0, 1);
    assert_eq!(blocked.map(|b| b.wall_type), Some(Wall::ShortWall));
    assert!(map.is_blocked_by_wall(Pos::new(5, 6), 0, 1).is_none());
    assert!(map.is_blocked_by_wall(Pos::new(5, 7), 0, 1).is_none());
}

#[test]
fn test_blocked_by_wall_left() {
    let mut map = Map::from_dims(10, 10);

    let pos = Pos::new(5, 5);
    map[pos].left_wall = Wall::ShortWall;
  
    map.update_map();

    let blocked = map.is_blocked_by_wall(Pos::new(5, 5), -1, 0);
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));
    assert!(map.is_blocked_by_wall(Pos::new(4, 5), -1, 0).is_none());
    assert!(map.is_blocked_by_wall(Pos::new(6, 5), -1, 0).is_none());
}

#[test]
fn test_fov_blocked_by_wall_right() {
    let radius = 10;
    let mut map = Map::from_dims(10, 10);

    for wall_y_pos in 2..8 {
        let pos: Pos = Pos::new(5, wall_y_pos);
        map[pos] = Tile::empty();
        map[pos].left_wall = Wall::ShortWall;
    }
  
    map.update_map();

    assert!(map.is_in_fov(Pos::new(4, 5), Pos::new(9, 5), radius) == false);
}

#[test]
fn test_fov_blocked_by_wall_left() {
    let radius = 10;
    let mut map = Map::from_dims(10, 10);

    for wall_y_pos in 2..8 {
        let pos: Pos = Pos::new(5, wall_y_pos);
        map[pos] = Tile::empty();
        map[pos].left_wall = Wall::ShortWall;
    }
  
    map.update_map();

    assert!(map.is_in_fov(Pos::new(9, 5), Pos::new(4, 5), radius) == false);
}

#[test]
fn test_fov_blocked_by_wall_up() {
    let radius = 10;
    let mut map = Map::from_dims(10, 10);

    for wall_x_pos in 2..8 {
        let pos: (i32, i32) = (wall_x_pos, 5);
        map[pos] = Tile::empty();
        map[pos].bottom_wall = Wall::ShortWall;
    }
  
    map.update_map();

    assert!(map.is_in_fov(Pos::new(5, 9), Pos::new(5, 5), radius) == false);
}

#[test]
fn test_fov_blocked_by_wall_down() {
    let radius = 10;
    let mut map = Map::from_dims(10, 10);

    for wall_x_pos in 2..8 {
        let pos: (i32, i32) = (wall_x_pos, 5);
        map[pos] = Tile::empty();
        map[pos].bottom_wall = Wall::ShortWall;
    }
  
    map.update_map();

    assert!(map.is_in_fov(Pos::new(5, 1), Pos::new(5, 6), radius) == false);
}

#[test]
fn test_blocked_by_wall() {
    let radius = 10;
    let mut map = Map::from_dims(10, 10);

    map[(5, 5)] = Tile::water();
  
    map.update_map();

    assert!(map.is_blocked_by_wall(Pos::new(4, 5), 1, 0).is_some());
    assert!(map.is_blocked_by_wall(Pos::new(4, 5), 3, 0).is_some());
    assert!(map.is_blocked_by_wall(Pos::new(3, 5), 3, 0).is_some());

    assert!(map.is_blocked_by_wall(Pos::new(6, 5), -1, 0).is_some());

    assert!(map.is_blocked_by_wall(Pos::new(5, 6), 0, -1).is_some());
    assert!(map.is_blocked_by_wall(Pos::new(5, 4), 0, 1).is_some());
}

#[test]
fn test_floodfill() {
    let mut map = Map::from_dims(10, 10);

    let start = Pos::new(5, 5);

    let flood: Vec<Pos> = map.floodfill(start, 0);
    assert_eq!(vec!(start), flood);

    let flood: Vec<Pos> = map.floodfill(start, 1);
    assert_eq!(9, flood.len());

    map[(5, 5)].left_wall = Wall::ShortWall;
    map[(5, 6)].left_wall = Wall::ShortWall;
    map[(5, 4)].left_wall = Wall::ShortWall;
    let flood: Vec<Pos> = map.floodfill(start, 1);
    assert_eq!(6, flood.len());

    map[(6, 3)].left_wall = Wall::ShortWall;
    map[(5, 3)].left_wall = Wall::ShortWall;

    map[(6, 4)].left_wall = Wall::ShortWall;
    map[(5, 4)].left_wall = Wall::ShortWall;

    map[(6, 5)].left_wall = Wall::ShortWall;
    map[(5, 5)].left_wall = Wall::ShortWall;
    map[start].bottom_wall = Wall::ShortWall;
    let flood: Vec<Pos> = map.floodfill(start, 2);
    assert!(flood.contains(&start));
    assert!(flood.contains(&Pos::new(5, 4)));
    assert!(flood.contains(&Pos::new(5, 3)));

    let flood: Vec<Pos> = map.floodfill(start, 3);
    assert_eq!(6, flood.len());
}
