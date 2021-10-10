use std::ops::{Index, IndexMut};
use std::collections::{HashSet, HashMap};
use std::iter;
use std::cell::RefCell;
use std::str::FromStr;
use std::fmt;

use oorandom::Rand32;

use logging_timer::timer;

use pathfinding::directed::astar::astar;

use symmetric_shadowcasting::Pos as SymPos;
use symmetric_shadowcasting::compute_fov;

use smallvec::SmallVec;

use itertools::Itertools;

use euclid::*;

use serde_derive::*;

use crate::types::*;
use crate::constants::*;
use crate::utils::*;
use crate::movement::Direction;
use crate::line::*;


// multiplier used to scale costs up in astar, allowing small
// adjustments of costs even though they are integers.
pub const ASTAR_COST_MULTIPLIER: i32 = 100;


#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum TileType {
    Empty,
    ShortWall,
    Wall,
    Water,
    Exit,
}

impl fmt::Display for TileType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TileType::Empty => write!(f, "empty"),
            TileType::ShortWall => write!(f, "shortwall"),
            TileType::Wall => write!(f, "wall"),
            TileType::Water => write!(f, "water"),
            TileType::Exit => write!(f, "exit"),
        }
    }
}

impl FromStr for TileType {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();
        if s == "empty" {
            return Ok(TileType::Empty);
        } else if s == "shortwall" {
            return Ok(TileType::ShortWall);
        } else if s == "wall" {
            return Ok(TileType::Wall);
        } else if s == "water" {
            return Ok(TileType::Water);
        } else if s == "exit" {
            return Ok(TileType::Exit);
        }

        return Err(format!("Could not parse '{}' as TileType", s));
    }
}

impl TileType {
    pub fn is_wall(&self) -> bool {
        match self {
            TileType::Empty => false,
            TileType::ShortWall => true,
            TileType::Wall => true,
            TileType::Water => false,
            TileType::Exit => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AoeEffect {
    Sound,
    Freeze,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MapLoadConfig {
    Random,
    TestMap,
    TestWall,
    Empty,
    TestCorner,
    TestPlayer,
    TestVaults,
    TestTraps,
    FromFile(String),
    VaultFile(String),
    ProcGen(String),
}

impl Default for MapLoadConfig {
    fn default() -> MapLoadConfig {
        return MapLoadConfig::Random;
    }
}

impl fmt::Display for MapLoadConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MapLoadConfig::Random => write!(f, "random"),
            MapLoadConfig::TestMap => write!(f, "test_map"),
            MapLoadConfig::TestWall => write!(f, "test_wall"),
            MapLoadConfig::Empty => write!(f, "empty"),
            MapLoadConfig::TestCorner => write!(f, "test_corner"),
            MapLoadConfig::TestPlayer => write!(f, "test_player"),
            MapLoadConfig::TestVaults => write!(f, "test_vaults"),
            MapLoadConfig::TestTraps => write!(f, "test_traps"),
            MapLoadConfig::FromFile(file) => write!(f, "from_file {}", file),
            MapLoadConfig::VaultFile(file) => write!(f, "vault_file {}", file),
            MapLoadConfig::ProcGen(file) => write!(f, "procgen {}", file),
        }
    }
}

impl FromStr for MapLoadConfig {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();
        if s == "random" {
            return Ok(MapLoadConfig::Random);
        } else if s == "test_map" {
            return Ok(MapLoadConfig::TestMap);
        } else if s == "test_wall" {
            return Ok(MapLoadConfig::TestWall);
        } else if s == "empty" {
            return Ok(MapLoadConfig::Empty);
        } else if s == "test_corner" {
            return Ok(MapLoadConfig::TestCorner);
        } else if s == "test_player" {
            return Ok(MapLoadConfig::TestPlayer);
        } else if s == "test_vaults" {
            return Ok(MapLoadConfig::TestVaults);
        } else if s == "test_traps" {
            return Ok(MapLoadConfig::TestTraps);
        } else if s.starts_with("from_file") {
            let args = s.split(" ").collect::<Vec<&str>>();
            return Ok(MapLoadConfig::FromFile(args[1].to_string()));
        } else if s.starts_with("vault_file") {
            let args = s.split(" ").collect::<Vec<&str>>();
            return Ok(MapLoadConfig::VaultFile(args[1].to_string()));
        } else if s.starts_with("procgen") {
            let args = s.split(" ").collect::<Vec<&str>>();
            return Ok(MapLoadConfig::ProcGen(args[1].to_string()));
        }

        return Err(format!("Could not parse '{}' as MapLoadConfig", s));
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Surface {
    Floor,
    Rubble,
    Grass,
}

impl fmt::Display for Surface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Surface::Floor => write!(f, "floor"),
            Surface::Rubble => write!(f, "rubble"),
            Surface::Grass => write!(f, "grass"),
        }
    }
}

impl FromStr for Surface {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();
        if s == "floor" {
            return Ok(Surface::Floor);
        } else if s == "rubble" {
            return Ok(Surface::Rubble);
        } else if s == "grass" {
            return Ok(Surface::Grass);
        }

        return Err(format!("Could not parse '{}' as Surface", s));
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum BlockedType {
    Fov,
    FovLow,
    Move,
}

impl BlockedType {
    pub fn blocking(&self, wall: Wall) -> bool {
        let empty = wall == Wall::Empty;
        let see_over = wall == Wall::ShortWall && *self == BlockedType::Fov;
        return !empty && !see_over;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[repr(C, packed)]
pub struct Tile {
    pub block_move: bool,
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
        Tile {
            block_move: false,
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
        Tile {
            block_move: true,
            block_sight: false,
            explored: false,
            tile_type: TileType::Water,
            bottom_wall: Wall::Empty,
            left_wall: Wall::Empty,
            chr: ' ' as u8,
            surface: Surface::Floor,
        }
    }

    pub fn grass() -> Self {
        let mut tile = Tile::empty();
        tile.surface = Surface::Grass;
        return tile;
    }

    pub fn rubble() -> Self {
        let mut tile = Tile::empty();
        tile.surface = Surface::Rubble;
        return tile;
    }

    pub fn wall() -> Self {
        return Tile::wall_with(MAP_WALL as char);
    }

    pub fn wall_with(chr: char) -> Self {
        Tile { block_move: true,
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
        Tile {
            block_move: false,
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
        Tile {
            block_move: false,
            block_sight: false,
            explored: false,
            tile_type: TileType::Exit,
            bottom_wall: Wall::Empty,
            left_wall: Wall::Empty,
            chr: ' ' as u8,
            surface: Surface::Floor,
        }
    }

    pub fn does_tile_block(self, block_type: BlockedType) -> bool {
        match block_type {
            BlockedType::Fov => {
                return self.block_sight;
            }

            BlockedType::FovLow => {
                return self.block_sight;
            }

            BlockedType::Move => {
                return self.block_move;
            }
        }
    }

    pub fn shorten(&mut self) {
        if self.bottom_wall == Wall::TallWall {
            self.bottom_wall = Wall::ShortWall;
        }

        if self.left_wall == Wall::TallWall {
            self.left_wall = Wall::ShortWall;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum FovResult {
    Outside,
    Edge,
    Inside,
}

impl FovResult {
    pub fn combine(&self, other: FovResult) -> FovResult {
        if *self == FovResult::Inside || other == FovResult::Inside {
            return FovResult::Inside;
        } else if *self == FovResult::Edge || other == FovResult::Edge {
            return FovResult::Edge;
        } else {
            return FovResult::Outside;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Debug)]
pub enum Rotation {
    Degrees0,
    Degrees90,
    Degrees180,
    Degrees270,
}

impl Rotation {
    pub fn rotate(&self, pos: Pos, width: i32, height: i32) -> Pos {
        let mut result = pos;
        match self {
            Rotation::Degrees0 => {
            }
            Rotation::Degrees90 => {
                // 90 degrees: swap x and y, mirror in x
                result = Pos::new(result.y, result.x);
                result = mirror_in_x(result, width);
            }
            Rotation::Degrees180 => {
                // 180 degrees: mirror in x, mirror in y
                result = mirror_in_x(result, width);
                result = mirror_in_y(result, height);
            }
            Rotation::Degrees270 => {
                // 270: swap x and y, mirror in y
                result = Pos::new(result.y, result.x);
                result = mirror_in_y(result, height);
            }
        }

        return result;
    }
}

#[test]
fn test_rotation() {
    let pos = Pos::new(0, 0);
    let width = 10;
    let height = 20;

    assert_eq!(pos, Rotation::Degrees0.rotate(pos, width, height));
    assert_eq!(Pos::new(width - 1, 0), Rotation::Degrees90.rotate(pos, width, height));
    assert_eq!(Pos::new(width - 1, height - 1), Rotation::Degrees180.rotate(pos, width, height));
    assert_eq!(Pos::new(0, height - 1), Rotation::Degrees270.rotate(pos, width, height));
}

pub fn reorient_map(map: &Map, rotation: Rotation, mirror: bool) -> Map {
    let (width, height) = map.size();

    let (mut new_width, mut new_height) = (width, height);
    if rotation == Rotation::Degrees90 || rotation == Rotation::Degrees270 {
        new_width = height;
        new_height = width;
    }
    let mut new_map = Map::from_dims(new_width as u32, new_height as u32);

    let mut left_walls = Vec::new();
    let mut bottom_walls = Vec::new();
    for x in 0..width {
        for y in 0..height {
            let orig_pos = Pos::new(x, y);
            
            let mut pos = Pos::new(x, y);
            if mirror {
                pos = mirror_in_x(pos, width);
            }
            pos = rotation.rotate(pos, new_width, new_height);
            new_map[pos] = map[orig_pos];

            if map[orig_pos].left_wall != Wall::Empty {
                left_walls.push((pos, map[orig_pos].left_wall));
            }

            if map[orig_pos].bottom_wall != Wall::Empty {
                bottom_walls.push((pos, map[orig_pos].bottom_wall));
            }
        }
    }

    for x in 0..new_width {
        for y in 0..new_height {
            let pos = Pos::new(x, y);
            new_map[pos].left_wall = Wall::Empty;
            new_map[pos].bottom_wall = Wall::Empty;
        }
    }

    for (wall_pos, wall_type) in left_walls {
        match rotation {
            Rotation::Degrees0 => {
                new_map[wall_pos].left_wall = wall_type;
            }

            Rotation::Degrees90 => {
                let new_wall_pos = move_y(wall_pos, -1);
                if new_map.is_within_bounds(new_wall_pos) {
                    new_map[new_wall_pos].bottom_wall = wall_type;
                }
            }

            Rotation::Degrees180 => {
                let new_wall_pos = move_x(wall_pos, 1);
                if new_map.is_within_bounds(new_wall_pos) {
                    new_map[new_wall_pos].left_wall = wall_type;
                }
            }

            Rotation::Degrees270 => {
                new_map[wall_pos].bottom_wall = wall_type;
            }
        }
    }

    for (wall_pos, wall_type) in bottom_walls {
        match rotation {
            Rotation::Degrees0 => {
                new_map[wall_pos].bottom_wall = wall_type;
            }

            Rotation::Degrees90 => {
                new_map[wall_pos].left_wall = wall_type;
            }

            Rotation::Degrees180 => {
                let new_wall_pos = move_y(wall_pos, -1);
                if new_map.is_within_bounds(new_wall_pos) {
                    new_map[new_wall_pos].bottom_wall = wall_type;
                }
            }

            Rotation::Degrees270 => {
                let new_wall_pos = move_x(wall_pos, 1);
                if new_map.is_within_bounds(new_wall_pos) {
                    new_map[new_wall_pos].left_wall = wall_type;
                }
            }
        }
    }

    return new_map;
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

impl fmt::Display for Wall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Wall::Empty => write!(f, "empty"),
            Wall::ShortWall => write!(f, "shortwall"),
            Wall::TallWall => write!(f, "tallwall"),
        }
    }
}

impl FromStr for Wall {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();
        if s == "empty" {
            return Ok(Wall::Empty);
        } else if s == "shortwall" {
            return Ok(Wall::ShortWall);
        } else if s == "wall" {
            return Ok(Wall::TallWall);
        } else if s == "water" {
        }

        return Err(format!("Could not parse '{}' as Wall", s));
    }
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

    pub fn blocked_left(&self, pos: Pos, blocked_type: BlockedType) -> bool {
        let offset = Pos::new(pos.x - 1, pos.y);
        if !self.is_within_bounds(offset) || !self.is_within_bounds(pos) {
            return true;
        }

        let blocking_wall = blocked_type.blocking(self[pos].left_wall);
        let blocking_tile = self[offset].does_tile_block(blocked_type);
        return blocking_wall || blocking_tile;
    }

    pub fn blocked_right(&self, pos: Pos, blocked_type: BlockedType) -> bool {
        let offset = Pos::new(pos.x + 1, pos.y);
        if !self.is_within_bounds(pos) || !self.is_within_bounds(pos) { 
            return true;
        }

        let blocking_wall = blocked_type.blocking(self[offset].left_wall);
        let blocking_tile = self[offset].does_tile_block(blocked_type);
        return blocking_wall || blocking_tile;
    }

    pub fn blocked_down(&self, pos: Pos, blocked_type: BlockedType) -> bool {
        let offset = Pos::new(pos.x, pos.y + 1);
        if !self.is_within_bounds(pos) || !self.is_within_bounds(pos) {
            return true;
        }

        let blocking_wall = blocked_type.blocking(self[pos].bottom_wall);
        let blocking_tile = self[offset].does_tile_block(blocked_type);
        return blocking_wall || blocking_tile;
    }

    pub fn blocked_up(&self, pos: Pos, blocked_type: BlockedType) -> bool {
        let offset = Pos::new(pos.x, pos.y - 1);
        if !self.is_within_bounds(pos) || !self.is_within_bounds(pos) {
            return true;
        }

        let blocking_wall = blocked_type.blocking(self[offset].bottom_wall);
        let blocking_tile = self[offset].does_tile_block(blocked_type);
        return blocking_wall || blocking_tile;
    }

    pub fn path_blocked_fov(&self, start_pos: Pos, end_pos: Pos) -> Option<Blocked> {
        return self.path_blocked(start_pos, end_pos, BlockedType::Fov);
    }

    pub fn path_blocked_fov_low(&self, start_pos: Pos, end_pos: Pos) -> Option<Blocked> {
        return self.path_blocked(start_pos, end_pos, BlockedType::FovLow);
    }

    pub fn path_blocked_move(&self, start_pos: Pos, end_pos: Pos) -> Option<Blocked> {
        return self.path_blocked(start_pos, end_pos, BlockedType::Move);
    }

    pub fn path_blocked_all(&self, start_pos: Pos, end_pos: Pos, blocked_type: BlockedType) -> Vec<Blocked> {
        let mut blocked_vec = Vec::new();
        let mut cur_pos = start_pos;
        while let Some(blocked) = self.path_blocked(cur_pos, end_pos, blocked_type) {
            blocked_vec.push(blocked);
            cur_pos = blocked.end_pos;
        }
        return blocked_vec;
    }

    pub fn move_blocked(&self, start_pos: Pos, end_pos: Pos, blocked_type: BlockedType) -> Option<Blocked> {
        let dxy = sub_pos(end_pos, start_pos);
        if dxy.x == 0 && dxy.y == 0 {
            return None;
        }

        let dir = Direction::from_dxy(dxy.x, dxy.y)
                            .expect(&format!("Check for blocking wall with no movement {:?}?", dxy));


        let mut blocked = Blocked::new(start_pos, end_pos, dir, false, Wall::Empty);

        // if the target position is out of bounds, we are blocked
        if !self.is_within_bounds(end_pos) {
            blocked.blocked_tile = true;

            // continuing to check after finding an out-of-bounds
            // position results in a panic, so stop now.
            return Some(blocked);
        }

        let mut found_blocker = false;

        // if moving into a blocked tile, we are blocked
        if self[end_pos].does_tile_block(blocked_type) {
            blocked.blocked_tile = true;
            found_blocker = true;
        }

        let (x, y) = (start_pos.x, start_pos.y);
        let move_dir = end_pos - Vector2D::new(x, y);

        // used for diagonal movement checks
        let x_moved = Pos::new(end_pos.x, y);
        let y_moved = Pos::new(x, end_pos.y);
        
        let direction = Direction::from_dxy(move_dir.x, move_dir.y).unwrap();
        match direction {
            Direction::Right | Direction::Left => {
                let mut left_wall_pos = start_pos;
                // moving right
                if move_dir.x >= 1 {
                    left_wall_pos = Pos::new(x + move_dir.x, y);
                }

                if self.is_within_bounds(left_wall_pos) &&
                   blocked_type.blocking(self[left_wall_pos].left_wall) {
                        blocked.wall_type = self[left_wall_pos].left_wall;
                        found_blocker = true;
                }
            }

            Direction::Up | Direction::Down => {
                let mut bottom_wall_pos = Pos::new(x, y + move_dir.y);
                if move_dir.y >= 1 {
                    bottom_wall_pos = start_pos;
                }

                if self.is_within_bounds(bottom_wall_pos) &&
                   blocked_type.blocking(self[bottom_wall_pos].bottom_wall) {
                        blocked.wall_type = self[bottom_wall_pos].bottom_wall;
                        found_blocker = true;
                }
            }

            Direction::DownRight => {
                if self.blocked_right(start_pos, blocked_type) && self.blocked_down(start_pos, blocked_type) {
                    blocked.wall_type = self[start_pos].bottom_wall;
                    found_blocker = true;
                }

                if self.blocked_right(move_y(start_pos, 1), blocked_type) &&
                   self.blocked_down(move_x(start_pos, 1), blocked_type) {
                    let blocked_pos = add_pos(start_pos, Pos::new(1, 0));
                    if self.is_within_bounds(blocked_pos) {
                        blocked.wall_type = self[blocked_pos].bottom_wall;
                    }
                    found_blocker = true;
                }

                if self.blocked_right(start_pos, blocked_type) &&
                   self.blocked_right(y_moved, blocked_type) {
                    blocked.wall_type = self[move_x(start_pos, 1)].left_wall;
                    found_blocker = true;
                }

                if self.blocked_down(start_pos, blocked_type) &&
                   self.blocked_down(x_moved, blocked_type) {
                    blocked.wall_type = self[start_pos].bottom_wall;
                    found_blocker = true;
                }
            }

            Direction::UpRight => {
                if self.blocked_up(start_pos, blocked_type) && self.blocked_right(start_pos, blocked_type) {
                    blocked.wall_type = self[move_y(start_pos, -1)].bottom_wall;
                    found_blocker = true;
                }

                if self.blocked_up(move_x(start_pos, 1), blocked_type) &&
                   self.blocked_right(move_y(start_pos, -1), blocked_type) {
                    let blocked_pos = add_pos(start_pos, Pos::new(1, -1));
                    if self.is_within_bounds(blocked_pos) {
                        blocked.wall_type = self[blocked_pos].bottom_wall;
                    }
                    found_blocker = true;
                }

                if self.blocked_right(start_pos, blocked_type) && self.blocked_right(y_moved, blocked_type) {
                    blocked.wall_type = self[move_x(start_pos, 1)].left_wall;
                    found_blocker = true;
                }

                if self.blocked_up(start_pos, blocked_type) && self.blocked_up(x_moved, blocked_type) {
                    blocked.wall_type = self[move_y(start_pos, -1)].bottom_wall;
                    found_blocker = true;
                }
            }

            Direction::DownLeft => {
                if self.blocked_left(start_pos, blocked_type) && self.blocked_down(start_pos, blocked_type) {
                    blocked.wall_type = self[start_pos].left_wall;
                    found_blocker = true;
                }

                if self.blocked_left(move_y(start_pos, 1), blocked_type) &&
                   self.blocked_down(move_x(start_pos, -1), blocked_type) {
                    let blocked_pos = add_pos(start_pos, Pos::new(-1, 1));
                    if self.is_within_bounds(blocked_pos) {
                        blocked.wall_type = self[blocked_pos].left_wall;
                    }
                    found_blocker = true;
                }

                if self.blocked_left(start_pos, blocked_type) && self.blocked_left(y_moved, blocked_type) {
                    blocked.wall_type = self[start_pos].left_wall;
                    found_blocker = true;
                }

                if self.blocked_down(start_pos, blocked_type) && self.blocked_down(x_moved, blocked_type) {
                    blocked.wall_type = self[start_pos].bottom_wall;
                    found_blocker = true;
                }
            }

            Direction::UpLeft => {
                if self.blocked_left(move_y(start_pos, -1), blocked_type) &&
                   self.blocked_up(move_x(start_pos, -1), blocked_type) {
                    let blocked_pos = add_pos(start_pos, Pos::new(-1, -1));
                    if self.is_within_bounds(blocked_pos) {
                        blocked.wall_type = self[blocked_pos].left_wall;
                    }
                    found_blocker = true;
                }

                if self.blocked_left(start_pos, blocked_type) && self.blocked_up(start_pos, blocked_type) {
                    blocked.wall_type = self[start_pos].left_wall;
                    found_blocker = true;
                }

                if self.blocked_left(start_pos, blocked_type) && self.blocked_left(y_moved, blocked_type) {
                    blocked.wall_type = self[start_pos].left_wall;
                    found_blocker = true;
                }

                if self.blocked_up(start_pos, blocked_type) && self.blocked_up(x_moved, blocked_type) {
                    let blocked_pos = move_y(start_pos, -1);
                    if self.is_within_bounds(blocked_pos) {
                        blocked.wall_type = self[blocked_pos].bottom_wall;
                    }
                    found_blocker = true;
                }
            }
        }

        if found_blocker {
            return Some(blocked);
        } else {
            return None;
        }
    }

    pub fn path_blocked(&self, start_pos: Pos, end_pos: Pos, blocked_type: BlockedType) -> Option<Blocked> {
        let line = line(start_pos, end_pos);
        let positions = iter::once(start_pos).chain(line.into_iter());
        for (pos, target_pos) in positions.tuple_windows() {
            let blocked = self.move_blocked(pos, target_pos, blocked_type);
            if blocked.is_some() {
                return blocked;
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

        let _fov = timer!("FOV");
        // check that the position is within the max view distance.
        if distance_maximum(start_pos, end_pos) <= radius {
            if self.is_in_fov_shadowcast(start_pos, end_pos) {
                let _fovrest = timer!("FOVREST");
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
                    in_fov = end_pos == blocked.end_pos && blocked.blocked_tile;
                } 
            }
        }

        return in_fov;
    }

    pub fn is_in_fov_shadowcast(&self, start_pos: Pos, end_pos: Pos) -> bool {
        let _fovshad = timer!("FOVSHAD");
        if let Some(visible) = self.fov_cache.borrow_mut().get(&start_pos) {
            return visible.contains(&end_pos);
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

pub fn random_offset(rng: &mut Rand32, radius: i32) -> Pos {
    return Pos::new(rng_range_i32(rng, -radius, radius),
                    rng_range_i32(rng, -radius, radius));
}

pub fn pos_in_radius(pos: Pos, radius: i32, rng: &mut Rand32) -> Pos {
    let offset = Vector2D::new(rng_range_i32(rng, -radius, radius),
                               rng_range_i32(rng, -radius, radius));
    return pos + offset;
}

pub fn path_find_distance(start: Pos, next_pos: Pos, end: Pos) -> i32 {
    let mut dist = distance(next_pos, end) * ASTAR_COST_MULTIPLIER;
    let diff = sub_pos(next_pos, start);

    // penalize diagonal movement just a little bit to avoid zigzagging.
    if diff.x != 0 && diff.y != 0 {
        dist += 1;
    }

    return dist;
}

pub fn astar_path(map: &Map,
                  start: Pos,
                  end: Pos,
                  max_dist: Option<i32>,
                  cost_fn: Option<fn(Pos, Pos, &Map) -> i32>) -> Vec<Pos> {
    let result;

    let maybe_results = 
        astar(&start,
              |&pos| astar_neighbors(map, start, pos, max_dist),
              |&pos| {
                  if let Some(fun) = &cost_fn { 
                      fun(start, pos, map) * ASTAR_COST_MULTIPLIER
                  } else {
                      path_find_distance(start, pos, end) as i32
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

pub fn astar_next_pos(map: &Map,
                      start: Pos,
                      end: Pos,
                      max_dist: Option<i32>,
                      cost_fn: Option<fn(Pos, Pos, &Map) -> i32>) -> Option<Pos> {
    let next_positions = astar_path(map, start, end, max_dist, cost_fn);

    if let Some(next_pos) = next_positions.get(1) {
        return Some(*next_pos);
    } else {
        return None;
    }
}

pub fn astar_neighbors(map: &Map, start: Pos, pos: Pos, max_dist: Option<i32>) -> SmallVec<[(Pos, i32); 8]> {
      if let Some(max_dist) = max_dist {
          if distance(start, pos) > max_dist {
              return SmallVec::new();
          }
      }

      let next_pos = map.reachable_neighbors(pos)
                        .iter()
                        .map(|pos| (*pos, 1))
                        .collect::<SmallVec<[(Pos, i32); 8]>>();

    return next_pos;
}

#[test]
fn test_blocked_by_wall_right() {
    let mut map = Map::from_dims(10, 10);

    let pos = Pos::new(5, 5);
    map[pos].left_wall = Wall::ShortWall;
  
    let left_of_wall = Pos::new(4, 5);
    let blocked = map.path_blocked_move(left_of_wall, add_pos(left_of_wall, Pos::new(1, 0)));
    assert_eq!(blocked.map(|b| b.wall_type == Wall::ShortWall), Some(true));

    assert!(map.path_blocked_move(pos, add_pos(pos, Pos::new(1, 0))).is_none());

    let two_left_of_wall = Pos::new(3, 5);
    assert_eq!(map.path_blocked_move(two_left_of_wall, add_pos(two_left_of_wall, Pos::new(1, 0))), None);
}

#[test]
fn test_blocked_by_wall_up() {
    let mut map = Map::from_dims(10, 10);

    let pos = Pos::new(5, 5);
    map[pos].bottom_wall = Wall::ShortWall;
  
    let blocked = map.path_blocked_move(Pos::new(5, 6), Pos::new(5, 5));
    assert_eq!(blocked.map(|b| b.wall_type), Some(Wall::ShortWall));
    assert!(map.path_blocked_move(Pos::new(5, 5), Pos::new(5, 4)).is_none());
    assert!(map.path_blocked_move(Pos::new(5, 4), Pos::new(5, 3)).is_none());
}

#[test]
fn test_blocked_by_wall_down() {
    let mut map = Map::from_dims(10, 10);

    let pos = Pos::new(5, 5);
    map[pos].bottom_wall = Wall::ShortWall;
  
    let blocked = map.path_blocked_move(Pos::new(5, 5), Pos::new(5, 6));
    assert_eq!(blocked.map(|b| b.wall_type), Some(Wall::ShortWall));
    assert!(map.path_blocked_move(Pos::new(5, 6), Pos::new(5, 7)).is_none());
    assert!(map.path_blocked_move(Pos::new(5, 7), Pos::new(5, 8)).is_none());
}

#[test]
fn test_blocked_by_wall_left() {
    let mut map = Map::from_dims(10, 10);

    let pos = Pos::new(5, 5);
    map[pos].left_wall = Wall::ShortWall;
  
    let blocked = map.path_blocked_move(Pos::new(5, 5), Pos::new(4, 5));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));
    assert!(map.path_blocked_move(Pos::new(4, 5), Pos::new(3, 5)).is_none());
    assert!(map.path_blocked_move(Pos::new(6, 5), Pos::new(5, 5)).is_none());
}

#[test]
fn test_blocked_in_corners() {
    let mut map = Map::from_dims(10, 10);

    // .....
    // ._|_. middle | is (5, 5)
    // ..|..
    // .....
    map[(5, 5)].left_wall = Wall::ShortWall;
    map[(5, 5)].bottom_wall = Wall::ShortWall;
    map[(4, 5)].bottom_wall = Wall::ShortWall;
    map[(5, 6)].left_wall = Wall::ShortWall;
  
    // down left
    let blocked = map.path_blocked_move(Pos::new(5, 5), Pos::new(4, 6));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));

    // down right
    let blocked = map.path_blocked_move(Pos::new(4, 5), Pos::new(5, 6));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));

    // up right
    let blocked = map.path_blocked_move(Pos::new(4, 6), Pos::new(5, 5));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));

    // up left
    let blocked = map.path_blocked_move(Pos::new(6, 6), Pos::new(4, 5));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));
}

#[test]
fn test_blocked_out_corners() {
    let mut map = Map::from_dims(10, 10);

    // ._...
    // |_|.. middle _ is (5, 5)
    // .....
    map[(5, 5)].left_wall = Wall::ShortWall;
    map[(5, 5)].bottom_wall = Wall::ShortWall;
    map[(6, 5)].left_wall = Wall::ShortWall;
    map[(5, 4)].bottom_wall = Wall::ShortWall;
  
    // down left
    let blocked = map.path_blocked_move(Pos::new(6, 4), Pos::new(5, 5));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));

    // down right
    let blocked = map.path_blocked_move(Pos::new(4, 5), Pos::new(5, 5));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));

    // up right
    let blocked = map.path_blocked_move(Pos::new(4, 6), Pos::new(5, 5));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));

    // up left
    let blocked = map.path_blocked_move(Pos::new(6, 6), Pos::new(5, 5));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));
}

#[test]
fn test_blocked_horiz_line() {
    let mut map = Map::from_dims(10, 10);

    // .....
    // ...__ middle . is (5, 5)
    // .....
    map[(6, 5)].bottom_wall = Wall::ShortWall;
    map[(7, 5)].bottom_wall = Wall::ShortWall;
  
    // down left
    let blocked = map.path_blocked_move(Pos::new(7, 5), Pos::new(6, 6));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));

    // down right
    let blocked = map.path_blocked_move(Pos::new(6, 5), Pos::new(7, 6));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));

    // up right
    let blocked = map.path_blocked_move(Pos::new(6, 6), Pos::new(7, 5));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));

    // up left
    let blocked = map.path_blocked_move(Pos::new(7, 6), Pos::new(6, 5));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));
}

#[test]
fn test_blocked_vert_line() {
    let mut map = Map::from_dims(10, 10);

    // .....
    // ...__ middle . is (5, 5)
    // .....
    map[(6, 5)].bottom_wall = Wall::ShortWall;
    map[(7, 5)].bottom_wall = Wall::ShortWall;
  
    // down left
    let blocked = map.path_blocked_move(Pos::new(7, 5), Pos::new(6, 6));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));

    // down right
    let blocked = map.path_blocked_move(Pos::new(6, 5), Pos::new(7, 6));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));

    // up right
    let blocked = map.path_blocked_move(Pos::new(6, 6), Pos::new(7, 5));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));

    // up left
    let blocked = map.path_blocked_move(Pos::new(7, 6), Pos::new(6, 5));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));
}

#[test]
fn test_fov_blocked_by_wall_right() {
    let radius = 10;
    let mut map = Map::from_dims(10, 10);

    for wall_y_pos in 2..8 {
        let pos: Pos = Pos::new(5, wall_y_pos);
        map[pos] = Tile::empty();
        map[pos].block_sight = true;
    }
  
    assert_eq!(false, map.is_in_fov(Pos::new(4, 5), Pos::new(9, 5), radius, false));
}

#[test]
fn test_fov_blocked_by_wall_left() {
    let radius = 10;
    let mut map = Map::from_dims(10, 10);

    for wall_y_pos in 2..8 {
        let pos: Pos = Pos::new(6, wall_y_pos);
        map[pos] = Tile::empty();
        map[pos].block_sight = true;
        map[pos].block_move = true;
    }
  
    assert_eq!(false, map.is_in_fov(Pos::new(9, 5), Pos::new(4, 5), radius, false));
}

#[test]
fn test_fov_blocked_by_wall_up() {
    let radius = 10;
    let mut map = Map::from_dims(10, 10);

    for wall_x_pos in 2..8 {
        let pos: (i32, i32) = (wall_x_pos, 6);
        map[pos] = Tile::empty();
        map[pos].block_sight = true;
        map[pos].block_move = true;
    }
  
    assert_eq!(false, map.is_in_fov(Pos::new(5, 9), Pos::new(5, 5), radius, false));
}

#[test]
fn test_fov_blocked_by_wall_down() {
    let radius = 10;
    let mut map = Map::from_dims(10, 10);

    for wall_x_pos in 2..8 {
        let pos: (i32, i32) = (wall_x_pos, 4);
        map[pos] = Tile::empty();
        map[pos].block_sight = true;
        map[pos].block_move = true;
    }
  
    assert_eq!(false, map.is_in_fov(Pos::new(5, 1), Pos::new(5, 6), radius, false));
}

#[test]
fn test_blocked_by_wall() {
    let mut map = Map::from_dims(10, 10);

    map[(5, 5)] = Tile::water();
  
    assert!(map.path_blocked_move(Pos::new(4, 5), Pos::new(5, 5)).is_some());
    assert!(map.path_blocked_move(Pos::new(4, 5), Pos::new(7, 5)).is_some());
    assert!(map.path_blocked_move(Pos::new(3, 5), Pos::new(6, 5)).is_some());

    assert!(map.path_blocked_move(Pos::new(6, 5), Pos::new(5, 5)).is_some());

    assert!(map.path_blocked_move(Pos::new(5, 6), Pos::new(5, 5)).is_some());
    assert!(map.path_blocked_move(Pos::new(5, 4), Pos::new(5, 5)).is_some());
}

#[test]
fn test_path_blocked_all() {
    let mut map = Map::from_dims(10, 10);
    map[(3, 5)].left_wall = Wall::ShortWall;
    map[(4, 5)].block_move = true;
    map[(5, 5)].block_move = true;
    map[(6, 5)].left_wall = Wall::TallWall;

    let start_pos = Pos::new(0, 5);
    let end_pos = Pos::new(9, 5);

    let blocked_positions = map.path_blocked_all(start_pos, end_pos, BlockedType::Move);

    assert_eq!(4, blocked_positions.len());

    assert_eq!(false, blocked_positions[0].blocked_tile);
    assert_eq!(Wall::ShortWall, blocked_positions[0].wall_type);

    assert_eq!(true, blocked_positions[1].blocked_tile);
    assert_eq!(Wall::Empty, blocked_positions[1].wall_type);

    assert_eq!(true, blocked_positions[2].blocked_tile);
    assert_eq!(Wall::Empty, blocked_positions[2].wall_type);

    assert_eq!(false, blocked_positions[3].blocked_tile);
    assert_eq!(Wall::TallWall, blocked_positions[3].wall_type);
}

