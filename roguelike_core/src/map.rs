use std::ops::{Index, IndexMut};
use std::collections::HashSet;

use rand::prelude::*;

use pathfinding::directed::astar::astar;

use tcod::map::{Map as FovMap};
use tcod::pathfinding::*;
use tcod::line::*;

use serde_derive::*;

use crate::constants::*;


#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum MapLoadConfig {
    Random,
    TestWall,
    TestCorner,
    FromFile,
}

impl Default for MapLoadConfig {
    fn default() -> MapLoadConfig {
        return MapLoadConfig::Random;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Tile {
    pub blocked: bool,
    pub block_sight: bool,
    pub explored: bool,
    pub tile_type: TileType,
    pub sound: Option<(i32, i32)>,
    pub bottom_wall: Wall,
    pub left_wall: Wall,
    pub chr: Option<char>,
}

impl Tile {
    pub fn empty() -> Self {
        Tile { blocked: false,
               block_sight: false,
               explored: false,
               tile_type: TileType::Empty,
               sound: None,
               bottom_wall: Wall::Empty,
               left_wall: Wall::Empty,
               chr: Some(' '),
        }
    }

    pub fn water() -> Self {
        Tile { blocked: true,
               block_sight: false,
               explored: false,
               tile_type: TileType::Water,
               sound: None,
               bottom_wall: Wall::Empty,
               left_wall: Wall::Empty,
               chr: Some(' '),
        }
    }

    pub fn wall() -> Self {
        return Tile::wall_with(None);
    }

    pub fn wall_with(chr: Option<char>) -> Self {
        Tile { blocked: true,
               block_sight: true,
               explored: false,
               tile_type: TileType::Wall,
               sound: None,
               bottom_wall: Wall::Empty,
               left_wall: Wall::Empty,
               chr: chr,
        }
    }

    pub fn short_wall() -> Self {
        return Tile::short_wall_with(None);
    }

    pub fn short_wall_with(chr: Option<char>) -> Self {
        Tile { blocked: true,
               block_sight: false,
               explored: false,
               tile_type: TileType::ShortWall,
               sound: None,
               bottom_wall: Wall::Empty,
               left_wall: Wall::Empty,
               chr: chr,
        }
    }

    pub fn exit() -> Self {
        Tile { blocked: false,
               block_sight: false,
               explored: false,
               tile_type: TileType::Exit,
               sound: None,
               bottom_wall: Wall::Empty,
               left_wall: Wall::Empty,
               chr: None,
        }
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wall {
    Empty,
    ShortWall,
    TallWall,
}

pub struct Map {
    pub tiles: Vec<Vec<Tile>>,
    fov: FovMap,
    fov_pos: (i32, i32),
    fov_radius: i32,
}

impl Map {
    pub fn with_vec(tiles: Vec<Vec<Tile>>) -> Map {
        let width = tiles.len();
        let height = tiles[0].len();
        let mut map =
            Map {
                tiles,
                fov: FovMap::new(width as i32, height as i32),
                fov_pos: (0, 0),
                fov_radius: FOV_RADIUS,
            };

        map.update_map();

        return map;
    }

    pub fn from_dims(width: usize, height: usize) -> Map {
        let tiles = vec!(vec!(Tile::empty(); height); width);
        let mut map =
            Map {
                tiles,
                fov: FovMap::new(width as i32, height as i32),
                fov_pos: (0, 0),
                fov_radius: FOV_RADIUS,
            };

        map.update_map();

        return map;
    }

    pub fn empty() -> Map {
        let map =
            Map {
                tiles: Vec::new(),
                fov: FovMap::new(1, 1),
                fov_pos: (0, 0),
                fov_radius: FOV_RADIUS,
            };

        return map;
    }

    pub fn blocked_left(&self, x: i32, y: i32) -> bool {
        return self[(x, y)].left_wall != Wall::Empty;
    }

    pub fn blocked_right(&self, x: i32, y: i32) -> bool {
        return self.is_within_bounds(x, y) && self[(x + 1, y)].left_wall != Wall::Empty;
    }

    pub fn blocked_down(&self, x: i32, y: i32) -> bool {
        return self[(x, y)].bottom_wall != Wall::Empty;
    }

    pub fn blocked_up(&self, x: i32, y: i32) -> bool {
        return self.is_within_bounds(x, y) && self[(x, y - 1)].bottom_wall != Wall::Empty;
    }

    pub fn is_blocked_by_wall(&self, start_x: i32, start_y: i32, dx: i32, dy: i32) -> bool {
        let mut blocked = false;

        let (end_x, end_y) = (start_x + dx, start_y + dy);

        let line = Line::new((start_x, start_y), (end_x, end_y));

        let mut move_x;
        let mut move_y;

        let mut positions = Vec::new();
        // ensure that the starting position is looked at
        positions.push((start_x, start_y));
        positions.extend(line.into_iter());

        for pair in positions.windows(2) {
            let (x, y) = pair[0];
            let (target_x, target_y) = pair[1];

            if !self.is_within_bounds(target_x, target_y) {
                blocked = true;
                break;
            }

            move_x = target_x - x;
            move_y = target_y - y;

            // horizontal
            if move_y == 0 && move_x != 0 {
                let mut left_wall_pos = (x, y);
                if move_x >= 1 {
                    left_wall_pos = (x + move_x, y);
                }

                if self.is_within_bounds(left_wall_pos.0, left_wall_pos.1) {
                    blocked = self[left_wall_pos].left_wall != Wall::Empty;
                }
            // vertical 
            } else if move_x == 0 && move_y != 0 {
                let mut bottom_wall_pos = (x, y + move_y);
                if move_y >= 1 {
                    bottom_wall_pos = (x, y);
                }

                if self.is_within_bounds(bottom_wall_pos.0, bottom_wall_pos.1) {
                    blocked = self[bottom_wall_pos].bottom_wall != Wall::Empty;
                }
            } else { // diagonal
                // check for corners
                let current_space_blocked;
                let target_space_blocked;
                let vert_wall;
                let horiz_wall;

                // down right
                if move_x > 0 && move_y > 0 {
                    current_space_blocked = self.blocked_down(x, y) && self.blocked_right(x, y);
                    target_space_blocked = self.blocked_up(target_x, target_y) && self.blocked_left(target_x, target_y);
                    vert_wall = self.blocked_right(x, y) && self.blocked_right(x, target_y);
                    horiz_wall = self.blocked_down(x, y) && self.blocked_down(target_x, y);
                // up right
                } else if move_x > 0 && move_y < 0 {
                    current_space_blocked = self.blocked_up(x, y) && self.blocked_right(x, y);
                    target_space_blocked  = self.blocked_down(target_x, target_y) && self.blocked_left(target_x, target_y);
                    vert_wall = self.blocked_right(x, y) && self.blocked_right(x, target_y);
                    horiz_wall = self.blocked_up(x, y) && self.blocked_up(target_x, y);
                // down left
                } else if move_x < 0 && move_y > 0 {
                    current_space_blocked = self.blocked_down(x, y) && self.blocked_left(x, y);
                    target_space_blocked  = self.blocked_up(target_x, target_y) && self.blocked_right(target_x, target_y);
                    vert_wall = self.blocked_left(x, y) && self.blocked_left(x, target_y);
                    horiz_wall = self.blocked_down(x, y) && self.blocked_down(target_x, y);
                // up left
                } else {
                    current_space_blocked = self.blocked_left(x, y) && self.blocked_up(x, y);
                    target_space_blocked  = self.blocked_down(target_x, target_y) && self.blocked_right(target_x, target_y);
                    vert_wall = self.blocked_left(x, y) && self.blocked_left(x, target_y);
                    horiz_wall = self.blocked_up(x, y) && self.blocked_up(target_x, y);
                }

                blocked = current_space_blocked || target_space_blocked || vert_wall || horiz_wall;
            }

            if blocked {
                break;
            }
        }

        return blocked;
    }

    pub fn is_empty(&self, x: i32, y: i32) -> bool {
        return self[(x, y)].tile_type == TileType::Empty;
    }

    pub fn is_within_bounds(&self, x: i32, y: i32) -> bool {
        let x_bounds = x >= 0 && x < self.width();
        let y_bounds = y >= 0 && y < self.height();

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

    pub fn is_in_fov(&mut self, start_x: i32, start_y: i32, end_x: i32, end_y: i32, radius: i32) -> bool {
        if self.fov_pos != (start_x, start_y) {
            self.compute_fov(start_x, start_y, radius);
        }

        let wall_in_path = self.is_blocked_by_wall(start_x,
                                                   start_y,
                                                   end_x - start_x,
                                                   end_y - start_y);

        return !wall_in_path && self.fov.is_in_fov(end_x, end_y);
    }

    // this function is like clear_path, but only looks for terrain, not objects like monsters
    pub fn clear_path_obstacles(&self, start: (i32, i32), end: (i32, i32)) -> bool {
        let line = Line::new((start.0, start.1), (end.0, end.1));

        let path_blocked =
            line.into_iter().any(|point| self[point].blocked);

        return !path_blocked;
    }

    pub fn pos_in_radius(&self, start: (i32, i32), radius: i32) -> Vec<(i32, i32)> {
        let mut circle_positions = HashSet::new();

        // for each position on the edges of a square around the point, with the
        // radius as the distance in x/y, add to a set.
        // duplicates will be removed, leaving only points within the radius.
        for x in (start.0 - radius)..(start.0 + radius) {
            for y in (start.1 - radius)..(start.1 + radius) {
                let line = Line::new(start, (x, y));

                // get points to the edge of square, filtering for points within the given radius
                for point in line.into_iter() {
                    if distance(start, point) < radius {
                        circle_positions.insert(point);
                    }
                }
            }
        }

        return circle_positions.iter().map(|pos| *pos).collect();
    }

    pub fn reachable_neighbors(&self, x: i32, y: i32) -> Vec<(i32, i32)> {
        let neighbors = [(1, 0),  (1, 1),  (0, 1), 
                         (-1, 1), (-1, 0), (-1, -1),
                         (0, -1), (1, -1)];

        let mut result = Vec::new();

        for delta in neighbors.iter() {
            if !self.is_blocked_by_wall(x, y, delta.0, delta.1) {
                result.push((x + delta.0, y + delta.1));
            }
        }

        return result;
    }

    pub fn astar(&self, start: (i32, i32), end: (i32, i32)) -> Vec<(i32, i32)> {
        let result;

        let maybe_results = 
            astar(&start,
                  |&pos| self.reachable_neighbors(pos.0, pos.1)
                              .iter()
                              .map(|pos| (*pos, 1))
                              .collect::<Vec<((i32, i32), i32)>>()
                  ,
                  |&pos| distance(pos, end) as i32,
                  |&pos| pos == end);

        if let Some((results, _cost)) = maybe_results {
            result = results;
        } else {
            result = Vec::new();
        }

        return result;
    }

    pub fn set_cell(&mut self, x: i32, y: i32, transparent: bool, walkable: bool) {
        self.fov.set(x, y, transparent, walkable);
    }

    pub fn compute_fov(&mut self, x: i32, y: i32, view_radius: i32) {
        self.fov_pos = (x, y);
        self.fov_radius = view_radius;
        self.fov.compute_fov(x, y, view_radius, true, tcod::map::FovAlgorithm::Basic);
    }

    pub fn update_map(&mut self) {
        let dims = self.fov.size();

        for y in 0..dims.1 {
            for x in 0..dims.0 {
                self.fov.set(x,
                             y,
                             !self.tiles[x as usize][y as usize].block_sight,
                             !self.tiles[x as usize][y as usize].blocked);
            }
        }

        self.compute_fov(self.fov_pos.0, self.fov_pos.1, self.fov_radius);
    }
}

impl Index<(i32, i32)> for Map {
    type Output = Tile;

    fn index(&self, index: (i32, i32)) -> &Tile {
        &self.tiles[index.0 as usize][index.1 as usize]
    }
}

impl Index<(usize, usize)> for Map {
    type Output = Tile;

    fn index(&self, index: (usize, usize)) -> &Tile {
        &self.tiles[index.0][index.1]
    }
}

impl IndexMut<(i32, i32)> for Map {
    fn index_mut(&mut self, index: (i32, i32)) -> &mut Tile {
        &mut self.tiles[index.0 as usize][index.1 as usize]
    }
}

impl IndexMut<(usize, usize)> for Map {
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Tile {
        &mut self.tiles[index.0][index.1]
    }
}

pub fn near_tile_type(map: &Map, position: (i32, i32), tile_type: TileType) -> bool {
    let neighbor_offsets: Vec<(i32, i32)>
        = vec!((1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1));

    let mut near_given_tile = false;

    for offset in neighbor_offsets {
        let neighbor_position = move_by(position, offset);

        if map.is_within_bounds(neighbor_position.0, neighbor_position.1) &&
           map[neighbor_position].tile_type == tile_type {
            near_given_tile = true;
            break;
        }
    }

    return near_given_tile;
}

pub fn random_offset(rng: &mut SmallRng, radius: i32) -> (i32, i32) {
    return (rng.gen_range(-radius, radius),
            rng.gen_range(-radius, radius));
}

pub fn pos_in_radius(pos: (i32, i32), radius: i32, rng: &mut SmallRng) -> (i32, i32) {
    return (pos.0 + rng.gen_range(-radius, radius),
            pos.1 + rng.gen_range(-radius, radius));
}

pub fn place_block(map: &mut Map, start: (i32, i32), width: i32, tile: Tile) -> Vec<(i32, i32)> {
    let mut positions = Vec::new();

    for x in 0..width {
        for y in 0..width {
            let pos = (start.0 + x, start.1 + y);
            map[pos] = tile;
            positions.push(pos);
        }
    }

    positions
}

pub fn place_line(map: &mut Map, start: (i32, i32), end: (i32, i32), tile: Tile) -> Vec<(i32, i32)> {
    let mut positions = Vec::new();
    let mut line = Line::new(start, end);

    while let Some(pos) = line.step() {
        if map.is_within_bounds(pos.0, pos.1) {
            map[pos] = tile;
            positions.push(pos);
        }
    }

    positions
}

pub fn add_obstacle(map: &mut Map, pos: (i32, i32), obstacle: Obstacle, rng: &mut SmallRng) {
    match obstacle {
        Obstacle::Block => {
            map.tiles[pos.0 as usize][pos.1 as usize] = Tile::wall();
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
                    map.tiles[pos.0 as usize + x][pos.1 as usize] = Tile::wall();
                }
                map.tiles[pos.0 as usize][(pos.1 + dir) as usize] = Tile::wall();
            } else {
                for y in 0..3 {
                    map.tiles[pos.0 as usize][pos.1 as usize + y] = Tile::wall();
                }
                map.tiles[(pos.0 + dir) as usize][pos.1 as usize] = Tile::wall();
            }
        }

        Obstacle::Building => {
            let size = 2;

            let mut positions = vec!();
            positions.append(&mut place_line(map, move_by(pos, (-size, size)),  move_by(pos, (size,  size)), Tile::wall()));
            positions.append(&mut place_line(map, move_by(pos, (-size, size)),  move_by(pos, (-size, -size)), Tile::wall()));
            positions.append(&mut place_line(map, move_by(pos, (-size, -size)), move_by(pos, (size, -size)), Tile::wall()));
            positions.append(&mut place_line(map, move_by(pos, (size, -size)),  move_by(pos, (size,  size)), Tile::wall()));

            for _ in 0..rng.gen_range(0, 10) {
                positions.swap_remove(rng.gen_range(0, positions.len()));
            }
        }
    }
}

fn move_by(start: (i32, i32), diff: (i32, i32)) -> (i32, i32) {
    return (start.0 + diff.0, start.1 + diff.1);
}

// NOTE does not need to be in map- just a distance function
pub fn distance(start: (i32, i32), end: (i32, i32)) -> i32 {
    let diff = (end.0 - start.0, end.1 - start.1);
    return ((diff.0 * diff.0 + diff.1 * diff.1) as f32).sqrt() as i32;
}

fn move_y(pos: (i32, i32), offset_y: i32) -> (i32, i32) {
    return (pos.0, pos.1 + offset_y);
}

fn move_x(pos: (i32, i32), offset_x: i32) -> (i32, i32) {
    return (pos.0 + offset_x, pos.1);
}

// TODO put in constructor, and in set_fov function
pub fn make_tcod_map(tiles: &Vec<Vec<Tile>>) -> tcod::map::Map {
    let (map_width, map_height) = (tiles.len(), tiles[0].len());
    let mut map_copy = tcod::map::Map::new(map_width as i32, map_height as i32);
    for x in 0..map_width {
        for y in 0..map_height {
            let transparent = !tiles[x as usize][y as usize].block_sight;
            let walkable = !tiles[x as usize][y as usize].blocked;
            map_copy.set(x as i32, y as i32, transparent, walkable);
        }
    }

    map_copy
}


#[test]
fn test_blocked_by_wall_right() {
    let mut map = Map::from_dims(10, 10);

    let pos: (usize, usize) = (5, 5);
    map[pos].left_wall = Wall::ShortWall;
  
    map.update_map();

    assert!(map.is_blocked_by_wall(4, 5, 1, 0) == true);
    assert!(map.is_blocked_by_wall(5, 5, 1, 0) == false);
    assert!(map.is_blocked_by_wall(3, 5, 1, 0) == false);
}

#[test]
fn test_blocked_by_wall_up() {
    let mut map = Map::from_dims(10, 10);

    let pos: (usize, usize) = (5, 5);
    map[pos].bottom_wall = Wall::ShortWall;
  
    map.update_map();

    assert!(map.is_blocked_by_wall(5, 5, 0, 1) == true);
    assert!(map.is_blocked_by_wall(5, 6, 0, 1) == false);
    assert!(map.is_blocked_by_wall(5, 4, 0, 1) == false);
}

#[test]
fn test_blocked_by_wall_down() {
    let mut map = Map::from_dims(10, 10);

    let pos: (usize, usize) = (5, 5);
    map[pos].bottom_wall = Wall::ShortWall;
  
    map.update_map();

    assert!(map.is_blocked_by_wall(5, 6, 0, -1) == true);
    assert!(map.is_blocked_by_wall(5, 5, 0, -1) == false);
    assert!(map.is_blocked_by_wall(5, 7, 0, -1) == false);
}

#[test]
fn test_blocked_by_wall_left() {
    let mut map = Map::from_dims(10, 10);

    let pos: (usize, usize) = (5, 5);
    map[pos].left_wall = Wall::ShortWall;
  
    map.update_map();

    assert!(map.is_blocked_by_wall(5, 5, -1, 0) == true);
    assert!(map.is_blocked_by_wall(6, 5, -1, 0) == false);
    assert!(map.is_blocked_by_wall(4, 5, -1, 0) == false);
}

#[test]
fn test_fov_blocked_by_wall_right() {
    let radius = 10;
    let mut map = Map::from_dims(10, 10);

    for wall_y_pos in 2..8 {
        let pos: (i32, i32) = (5, wall_y_pos);
        map[pos] = Tile::empty();
        map[pos].left_wall = Wall::ShortWall;
    }
  
    map.update_map();

    assert!(map.is_in_fov(4, 5, 9, 5, radius) == false);
}

#[test]
fn test_fov_blocked_by_wall_left() {
    let radius = 10;
    let mut map = Map::from_dims(10, 10);

    for wall_y_pos in 2..8 {
        let pos: (i32, i32) = (5, wall_y_pos);
        map[pos] = Tile::empty();
        map[pos].left_wall = Wall::ShortWall;
    }
  
    map.update_map();

    assert!(map.is_in_fov(9, 5, 4, 5, radius) == false);
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

    assert!(map.is_in_fov(5, 9, 5, 5, radius) == false);
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

    assert!(map.is_in_fov(5, 1, 5, 6, radius) == false);
}
