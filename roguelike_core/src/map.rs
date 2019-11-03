use std::ops::{Index, IndexMut};
use std::collections::HashSet;
use std::boxed::Box;

use rand::prelude::*;

use tcod::map::{Map as FovMap};
use tcod::pathfinding::*;
use tcod::line::*;

use crate::types::*;


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
}

impl Map {
    pub fn with_vec(tiles: Vec<Vec<Tile>>) -> Map {
        let width = tiles.len();
        let height = tiles[0].len();
        let mut map =
            Map {
                tiles,
                fov: FovMap::new(width as i32, height as i32),
            };

        map.compute_fov();

        return map;
    }

    pub fn from_dims(width: usize, height: usize) -> Map {
        let tiles = vec!(vec!(Tile::empty(); height); width);
        let mut map =
            Map {
                tiles,
                fov: FovMap::new(width as i32, height as i32),
            };

        map.compute_fov();

        return map;
    }

    pub fn empty() -> Map {
        let map =
            Map {
                tiles: Vec::new(),
                fov: FovMap::new(1, 1),
            };

        return map;
    }

    // TODO this does not check all squares between start and end- only
    // the first and last square are used. this could result in skipping
    // over walls for long movements.
    pub fn is_blocked_by_wall(&self, x: i32, y: i32, dx: i32, dy: i32) -> bool {
        let mut blocked = false;

        if dx >= 1 {
            blocked = self[(x + dx, y)].left_wall != Wall::Empty;
        } else if dx <= -1 {
            blocked = self[(x, y)].left_wall != Wall::Empty;
        } 
        
        if dy >= 1 {
            blocked |= self[(x, y)].bottom_wall != Wall::Empty;
        } else if dy <= -1 {
            blocked |= self[(x, y + dy)].bottom_wall != Wall::Empty;
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

    pub fn is_in_fov(&self, width: i32, height: i32) -> bool {
        return self.fov.is_in_fov(width, height);
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

    pub fn astar(&self, start: (i32, i32), end: (i32, i32)) -> Vec<(i32, i32)> {
        let map_copy = make_tcod_map(&self.tiles);
        let mut astar = AStar::new_from_map(map_copy, 1.5);

        astar.find(start, end);

        // NOTE potentially inefficient
        return astar.iter().into_iter().collect::<Vec<_>>();
    }

    pub fn set_cell(&mut self, x: i32, y: i32, transparent: bool, walkable: bool) {
        self.fov.set(x, y, transparent, walkable);
    }

    pub fn compute_fov(&mut self) {
        let dims = self.fov.size();

        for y in 0..dims.1 {
            for x in 0..dims.0 {
                self.fov.set(x,
                             y,
                             !self.tiles[x as usize][y as usize].block_sight,
                             !self.tiles[x as usize][y as usize].blocked);
            }
        }
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

        if map[neighbor_position].tile_type == tile_type {
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
        map[pos] = tile;
        positions.push(pos);
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
fn distance(start: (i32, i32), end: (i32, i32)) -> i32 {
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

