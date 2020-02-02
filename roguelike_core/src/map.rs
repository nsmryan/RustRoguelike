use std::ops::{Index, IndexMut};
use std::collections::HashSet;

use rand::prelude::*;

use pathfinding::directed::astar::astar;

use tcod::map::{Map as FovMap};
use tcod::line::*;

use euclid::*;

use serde_derive::*;

use crate::types::*;
use crate::utils::*;
use crate::constants::*;


#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum MapLoadConfig {
    Random,
    TestWall,
    TestCorner,
    TestPlayer,
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
                fov: FovMap::new(width as i32, height as i32),
                fov_pos: Pos::new(0, 0),
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
                fov_pos: Pos::new(0, 0),
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
                fov_pos: Pos::new(0, 0),
                fov_radius: FOV_RADIUS,
            };

        return map;
    }

    pub fn blocked_left(&self, pos: Pos) -> bool {
        return self[pos].left_wall != Wall::Empty;
    }

    pub fn blocked_right(&self, pos: Pos) -> bool {
        let offset = Pos::new(pos.x + 1, pos.y);
        return self.is_within_bounds(pos) &&
               self[offset].left_wall != Wall::Empty;
    }

    pub fn blocked_down(&self, pos: Pos) -> bool {
        return self[pos].bottom_wall != Wall::Empty;
    }

    pub fn blocked_up(&self, pos: Pos) -> bool {
        return self.is_within_bounds(pos) &&
               self[Pos::new(pos.x, pos.y - 1)].bottom_wall != Wall::Empty;
    }

    pub fn is_blocked_by_wall(&self, start_pos: Pos, dx: i32, dy: i32) -> bool {
        let mut blocked = false;

        let (end_x, end_y) = (start_pos.x + dx, start_pos.y + dy);

        let line = Line::new((start_pos.x, start_pos.y), (end_x, end_y));

        let mut move_pos;

        let mut positions = Vec::new();
        // ensure that the starting position is looked at
        positions.push((start_pos.x, start_pos.y));
        positions.extend(line.into_iter());

        for pair in positions.windows(2) {
            let (x, y) = pair[0];
            let target_pos = Pos::from(pair[1]);

            if !self.is_within_bounds(target_pos) {
                blocked = true;
                break;
            }

            // if the position is blocked (a solid tile), but is not the target tile,
            // then it is blocked. We exclude the final tile as is it not itself blocked.
            if self[target_pos].blocked && target_pos.to_tuple() != (end_x, end_y) {
                blocked = true;
                break;
            }

            move_pos = target_pos - Vector2D::new(x, y);

            // horizontal
            if move_pos.y == 0 && move_pos.x != 0 {
                let mut left_wall_pos = Pos::new(x, y);
                if move_pos.x >= 1 {
                    left_wall_pos = Pos::new(x + move_pos.x, y);
                }

                if self.is_within_bounds(left_wall_pos) {
                    blocked = self[left_wall_pos].left_wall != Wall::Empty;
                }
            // vertical 
            } else if move_pos.x == 0 && move_pos.y != 0 {
                let mut bottom_wall_pos = Pos::new(x, y + move_pos.y);
                if move_pos.y >= 1 {
                    bottom_wall_pos = Pos::new(x, y);
                }

                if self.is_within_bounds(bottom_wall_pos) {
                    blocked = self[bottom_wall_pos].bottom_wall != Wall::Empty;
                }
            } else { // diagonal
                // check for corners
                let current_space_blocked;
                let target_space_blocked;
                let vert_wall;
                let horiz_wall;

                let pos = Pos::new(x, y);
                let x_moved = Pos::new(target_pos.x, y);
                let y_moved = Pos::new(x, target_pos.y);

                // down right
                if move_pos.x > 0 && move_pos.y > 0 {
                    current_space_blocked = self.blocked_down(pos) && self.blocked_right(pos);
                    target_space_blocked = self.blocked_up(target_pos) && self.blocked_left(target_pos);
                    vert_wall = self.blocked_right(pos) && self.blocked_right(y_moved);
                    horiz_wall = self.blocked_down(pos) && self.blocked_down(x_moved);
                // up right
                } else if move_pos.x > 0 && move_pos.y < 0 {
                    current_space_blocked = self.blocked_up(pos) && self.blocked_right(pos);
                    target_space_blocked  = self.blocked_down(target_pos) && self.blocked_left(target_pos);
                    vert_wall = self.blocked_right(pos) && self.blocked_right(y_moved);
                    horiz_wall = self.blocked_up(pos) && self.blocked_up(x_moved);
                // down left
                } else if move_pos.x < 0 && move_pos.y > 0 {
                    current_space_blocked = self.blocked_down(pos) && self.blocked_left(pos);
                    target_space_blocked  = self.blocked_up(target_pos) && self.blocked_right(target_pos);
                    vert_wall = self.blocked_left(pos) && self.blocked_left(y_moved);
                    horiz_wall = self.blocked_down(pos) && self.blocked_down(x_moved);
                // up left
                } else {
                    current_space_blocked = self.blocked_left(pos) && self.blocked_up(pos);
                    target_space_blocked  = self.blocked_down(target_pos) && self.blocked_right(target_pos);
                    vert_wall = self.blocked_left(pos) && self.blocked_left(y_moved);
                    horiz_wall = self.blocked_up(pos) && self.blocked_up(x_moved);
                }

                blocked = current_space_blocked || target_space_blocked || vert_wall || horiz_wall;
            }

            if blocked {
                break;
            }
        }

        return blocked;
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

    pub fn is_in_fov(&mut self, start_pos: Pos, end_pos: Pos, radius: i32) -> bool {
        if self.fov_pos != start_pos {
            self.compute_fov(start_pos, radius);
        }

        let offset = Pos::new(end_pos.x - start_pos.x,
                              end_pos.y - start_pos.y);
        let wall_in_path = self.is_blocked_by_wall(start_pos, offset.x, offset.y);

        return !wall_in_path && self.fov.is_in_fov(end_pos.x, end_pos.y);
    }

    // this function is like clear_path, but only looks for terrain, not objects like monsters
    pub fn clear_path_obstacles(&self, start: Pos, end: Pos) -> bool {
        let line = Line::new(start.to_tuple(), end.to_tuple());

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
                let line = Line::new(start.to_tuple(), (x, y));

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

    pub fn reachable_neighbors(&self, pos: Pos) -> Vec<Pos> {
        let neighbors = [(1, 0),  (1, 1),  (0, 1), 
                         (-1, 1), (-1, 0), (-1, -1),
                         (0, -1), (1, -1)];

        let mut result = Vec::new();

        for delta in neighbors.iter() {
            if !self.is_blocked_by_wall(pos, delta.0, delta.1) {
                result.push(pos + Vector2D::new(delta.0, delta.1));
            }
        }

        return result;
    }

    pub fn astar(&self, start: Pos, end: Pos) -> Vec<Pos> {
        let result;

        let maybe_results = 
            astar(&start,
                  |&pos| self.reachable_neighbors(pos)
                              .iter()
                              .map(|pos| (*pos, 1))
                              .collect::<Vec<(Pos, i32)>>()
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

    pub fn compute_fov(&mut self, pos: Pos, view_radius: i32) {
        self.fov_pos = pos;
        self.fov_radius = view_radius;
        self.fov.compute_fov(pos.x, pos.y, view_radius, true, tcod::map::FovAlgorithm::Basic);
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

        self.compute_fov(self.fov_pos, self.fov_radius);
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
    let mut line = Line::new(start.to_tuple(), end.to_tuple());

    while let Some(pos) = line.step() {
        let pos = Pos::from(pos);
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

fn move_by(start: Pos, diff: Pos) -> Pos {
    return Pos::new(start.x + diff.x, start.y + diff.y);
}

fn move_y(pos: Pos, offset_y: i32) -> Pos {
    return Pos::new(pos.x, pos.y + offset_y);
}

fn move_x(pos: Pos, offset_x: i32) -> Pos {
    return Pos::new(pos.x + offset_x, pos.y);
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
