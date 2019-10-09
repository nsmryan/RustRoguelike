use std::ops::{Index, IndexMut};
use std::collections::HashSet;

use rand::prelude::*;

use tcod::line::*;

use crate::engine::types::*;
use crate::constants::*;


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
}

impl Map {
    pub fn with_vec(tiles: Vec<Vec<Tile>>) -> Map {
        let map =
            Map {
                tiles: tiles,
            };

        return map;
    }

    pub fn from_dims(width: usize, height: usize) -> Map {
        let map =
            Map {
              tiles: vec!(vec!(Tile::empty(); height); width),
            };

        return map;
    }

    pub fn empty() -> Map {
        let map =
            Map {
                tiles: Vec::new(),
            };

        return map;
    }

    pub fn is_blocked_by_wall(&self, x: i32, y: i32, dx: i32, dy: i32) -> bool {
        let mut blocked = false;

        if dx >= 1 {
            blocked = self[(x + dx, y)].left_wall != Wall::Empty;
        } else if dx <= -1 {
            blocked = self[(x, y)].left_wall != Wall::Empty;
        } 
        
        if dy >= 1 {
            blocked |= self[(x, y + dy)].bottom_wall != Wall::Empty;
        } else if dy <= -1 {
            blocked |= self[(x, y)].bottom_wall != Wall::Empty;
        }

        return blocked;
    }

    pub fn is_blocked(&self, x: i32, y: i32, objects: &[Object]) -> bool {
        if self[(x, y)].blocked {
            return true;
        }

        let mut is_blocked = false;
        for object in objects.iter() {
            if object.blocks && object.pos() == (x, y) {
                is_blocked = true;
                break;
            }
        }

        return is_blocked;
    }

    pub fn is_empty(&self, x: i32, y: i32, _objects: &[Object]) -> bool {
        return self[(x, y)].tile_type == TileType::Empty;
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

    pub fn make_tcod_map(&self) -> tcod::map::Map {
        let (map_width, map_height) = self.size();
        let mut map_copy = tcod::map::Map::new(map_width, map_height);
        for x in 0..map_width {
            for y in 0..map_height {
                let transparent = !self.tiles[x as usize][y as usize].block_sight;
                let walkable = !self.tiles[x as usize][y as usize].blocked;
                map_copy.set(x, y, transparent, walkable);
            }
        }

        map_copy
    }

    pub fn clear_path(&self, start: (i32, i32), end: (i32, i32), objects: &[Object]) -> bool {
        let line = Line::new((start.0, start.1), (end.0, end.1));

        let path_blocked =
            line.into_iter().any(|point| self.is_blocked(point.0, point.1, objects));

        return !path_blocked;
    }

    // this function is like clear_path, but only looks for terrain, not objects like monsters
    pub fn clear_path_obstacles(&self, start: (i32, i32), end: (i32, i32), _objects: &[Object]) -> bool {
        let line = Line::new((start.0, start.1), (end.0, end.1));

        let path_blocked =
            line.into_iter().any(|point| self[point].blocked);

        return !path_blocked;
    }

    pub fn pos_in_radius(&self, start: (i32, i32), radius: i32) -> Vec<(i32, i32)> {
        let mut circle_positions = HashSet::new();
        let start_pos = Position::from_pair(&start);

        // for each position on the edges of a square around the point, with the
        // radius as the distance in x/y, add to a set.
        // duplicates will be removed, leaving only points within the radius.
        for x in (start.0 - radius)..(start.0 + radius) {
            for y in (start.1 - radius)..(start.1 + radius) {
                let line = Line::new((start.0, start.1), (x, y));

                // get points to the edge of square, filtering for points within the given radius
                for point in line.into_iter() {
                    if start_pos.distance(&Position::from_pair(&point)) < radius {
                        circle_positions.insert(point);
                    }
                }
            }
        }

        return circle_positions.iter().map(|pos| *pos).collect();
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

impl Index<Position> for Map {
    type Output = Tile;

    fn index(&self, position: Position) -> &Tile {
        &self.tiles[position.0 as usize][position.1 as usize]
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

impl IndexMut<Position> for Map {
    fn index_mut(&mut self, position: Position) -> &mut Tile {
        &mut self.tiles[position.0 as usize][position.1 as usize]
    }
}

pub fn near_tile_type(map: &Map, position: Position, tile_type: TileType) -> bool {
    let neighbor_offsets: Vec<(i32, i32)>
        = vec!((1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1));

    let mut near_given_tile = false;

    for offset in neighbor_offsets {
        let neighbor_position = position.add(Position(offset.0, offset.1));

        if map[neighbor_position].tile_type == tile_type {
            near_given_tile = true;
            break;
        }
    }

    return near_given_tile;
}

pub fn random_offset(rng: &mut SmallRng) -> Position {
    let position = 
        Position(rng.gen_range(-ISLAND_RADIUS, ISLAND_RADIUS),
                 rng.gen_range(-ISLAND_RADIUS, ISLAND_RADIUS));

    return position;
}

pub fn random_position(rng: &mut SmallRng) -> Position {
    Position(rng.gen_range(0, MAP_WIDTH), rng.gen_range(0, MAP_HEIGHT))
}

pub fn pos_in_radius(pos: Position, radius: i32, rng: &mut SmallRng) -> Position {
    return Position(pos.0 + rng.gen_range(-radius, radius),
    pos.1 + rng.gen_range(-radius, radius));
}

pub fn place_block(map: &mut Map, start: &Position, width: i32, tile: Tile) -> Vec<Position> {
    let mut positions = Vec::new();

    for x in 0..width {
        for y in 0..width {
            let pos = (start.0 + x, start.1 + y);
            map[pos] = tile;
            positions.push(Position::new(pos.0, pos.1));
        }
    }

    positions
}

pub fn place_line(map: &mut Map, start: &Position, end: &Position, tile: Tile) -> Vec<Position> {
    let mut positions = Vec::new();
    let mut line = Line::new(start.pair(), end.pair());

    while let Some(pos) = line.step() {
        map[pos] = tile;
        positions.push(Position::new(pos.0, pos.1));
    }

    positions
}

pub fn add_obstacle(map: &mut Map, pos: &Position, obstacle: Obstacle, rng: &mut SmallRng) {
    match obstacle {
        Obstacle::Block => {
            map.tiles[pos.0 as usize][pos.1 as usize] = Tile::wall();
        }

        Obstacle::Wall => {
            let end_pos = if rng.gen_bool(0.5) {
                pos.move_x(3)
            } else {
                pos.move_y(3)
            };
            place_line(map, pos, &end_pos, Tile::wall());
        }

        Obstacle::ShortWall => {
            let end_pos = if rng.gen_bool(0.5) {
                pos.move_x(3)
            } else {
                pos.move_y(3)
            };
            place_line(map, pos, &end_pos, Tile::short_wall());
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
            positions.append(&mut place_line(map, &pos.move_by(-size, size), &pos.move_by(size, size), Tile::wall()));
            positions.append(&mut place_line(map, &pos.move_by(-size, size), &pos.move_by(-size, -size), Tile::wall()));
            positions.append(&mut place_line(map, &pos.move_by(-size, -size), &pos.move_by(size, -size), Tile::wall()));
            positions.append(&mut place_line(map, &pos.move_by(size, -size), &pos.move_by(size, size), Tile::wall()));

            for _ in 0..rng.gen_range(0, 10) {
                positions.swap_remove(rng.gen_range(0, positions.len()));
            }
        }
    }
}

