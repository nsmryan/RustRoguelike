use std::ops::{Index, IndexMut};
use std::collections::{HashSet, HashMap};
use std::cell::RefCell;
use std::str::FromStr;
use std::fmt;
use std::hash::Hash;

use symmetric_shadowcasting::Pos as SymPos;
use symmetric_shadowcasting::compute_fov;

use smallvec::SmallVec;

use itertools::Itertools;

use euclid::*;

use serde_derive::*;

use roguelike_utils::line::*;

use crate::pathing::*;
use crate::utils::*;


pub type Pos = Point2D<i32, ()>;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
    DownLeft,
    DownRight,
    UpLeft,
    UpRight,
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Direction::Left => write!(f, "left"),
            Direction::Right => write!(f, "right"),
            Direction::Up => write!(f, "up"),
            Direction::Down => write!(f, "down"),
            Direction::DownLeft => write!(f, "downleft"),
            Direction::DownRight => write!(f, "downright"),
            Direction::UpLeft => write!(f, "upleft"),
            Direction::UpRight => write!(f, "upright"),
        }
    }
}

impl FromStr for Direction {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();
        if s == "left" {
            return Ok(Direction::Left);
        } else if s == "right" {
            return Ok(Direction::Right);
        } else if s == "up" {
            return Ok(Direction::Up);
        } else if s == "down" {
            return Ok(Direction::Down);
        } else if s == "upright" {
            return Ok(Direction::UpRight);
        } else if s == "upleft" {
            return Ok(Direction::UpLeft);
        } else if s == "downright" {
            return Ok(Direction::DownRight);
        } else if s == "downleft" {
            return Ok(Direction::DownLeft);
        }

        return Err(format!("Could not parse '{}' as Direction", s));
    }
}

impl Direction {
    pub fn from_dxy(dx: i32, dy: i32) -> Option<Direction> {
        if dx == 0 && dy == 0 {
            None
        } else if dx == 0 && dy < 0 {
            Some(Direction::Up)
        } else if dx == 0 && dy > 0 {
            Some(Direction::Down)
        } else if dx > 0 && dy == 0 {
            Some(Direction::Right)
        } else if dx < 0 && dy == 0 {
            Some(Direction::Left)
        } else if dx > 0 && dy > 0 {
            Some(Direction::DownRight)
        } else if dx > 0 && dy < 0 {
            Some(Direction::UpRight)
        } else if dx < 0 && dy > 0 {
            Some(Direction::DownLeft)
        } else if dx < 0 && dy < 0 {
            Some(Direction::UpLeft)
        } else {
            dbg!(dx, dy);
            panic!("Direction should not exist");
        }
    }

    pub fn from_positions(start: Pos, end: Pos) -> Option<Direction> {
        let dxy = sub_pos(end, start);
        return Direction::from_dxy(dxy.x, dxy.y);
    }

    pub fn reverse(&self) -> Direction {
        match self {
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::DownLeft => Direction::UpRight,
            Direction::DownRight => Direction::UpLeft,
            Direction::UpLeft => Direction::DownRight,
            Direction::UpRight => Direction::DownLeft,
        }
    }

    pub fn horiz(self) -> bool {
        match self {
            Direction::Left | Direction::Right |
            Direction::Up | Direction::Down => true,
            _ => false,
        }
    }

    pub fn diag(self) -> bool {
        match self {
            Direction::DownLeft | Direction::DownRight |
            Direction::UpLeft   | Direction::UpRight => true,
            _ => false,
        }
    }

    pub fn into_move(&self) -> Pos {
        match self {
            Direction::Left => Pos::new(-1, 0),
            Direction::Right => Pos::new(1, 0),
            Direction::Up => Pos::new(0, -1),
            Direction::Down => Pos::new(0, 1),
            Direction::DownLeft => Pos::new(-1, 1),
            Direction::DownRight => Pos::new(1, 1),
            Direction::UpLeft => Pos::new(-1, -1),
            Direction::UpRight => Pos::new(1, -1),
        }
    }

    pub fn move_actions() -> [Direction; 8] {
        return [Direction::Left,
                Direction::Right,
                Direction::Up,
                Direction::Down,
                Direction::DownLeft,
                Direction::DownRight,
                Direction::UpLeft,
                Direction::UpRight];
    }

    pub fn from_f32(flt: f32) -> Direction {
        let index = (flt * 8.0) as usize;
        let dirs = Direction::move_actions();
        return dirs[index];
    }

    pub fn offset_pos(&self, pos: Pos, amount: i32) -> Pos {
        let mov = self.into_move();
        return add_pos(pos, scale_pos(mov, amount));
    }

    pub fn turn_amount(&self, dir: Direction) -> i32 {
        let dirs = Direction::directions();
        let count = dirs.len() as i32;

        let start_ix = dirs.iter().position(|d| *d == *self).unwrap() as i32;
        let end_ix = dirs.iter().position(|d| *d == dir).unwrap() as i32;

        if (end_ix - start_ix).abs() < 4 {
            return end_ix - start_ix;
        } else if end_ix > start_ix {
            return (count - end_ix) + start_ix;
        } else {
            return (count - start_ix) + end_ix;
        }
    }

    pub fn directions() -> [Direction; 8] {
        use Direction::*;
        let dirs = [DownLeft, Left, UpLeft, Up, UpRight, Right, DownRight, Down];
        return dirs;
    }

    pub fn clockwise(&self) -> Direction {
        match self {
            Direction::Left => Direction::UpLeft,
            Direction::Right => Direction::DownRight,
            Direction::Up => Direction::UpRight,
            Direction::Down => Direction::DownLeft,
            Direction::DownLeft => Direction::Left,
            Direction::DownRight => Direction::Down,
            Direction::UpLeft => Direction::Up,
            Direction::UpRight => Direction::Right,
        }
    }

    pub fn counterclockwise(&self) -> Direction {
        match self {
            Direction::Left => Direction::DownLeft,
            Direction::Right => Direction::UpRight,
            Direction::Up => Direction::UpLeft,
            Direction::Down => Direction::DownRight,
            Direction::DownLeft => Direction::Down,
            Direction::DownRight => Direction::Right,
            Direction::UpLeft => Direction::Left,
            Direction::UpRight => Direction::Up,
        }
    }
}

#[test]
pub fn test_direction_turn_amount() {
    assert_eq!(-1, Direction::Up.turn_amount(Direction::UpLeft));
    assert_eq!(1, Direction::Up.turn_amount(Direction::UpRight));

    for move_action in &Direction::move_actions() {
        assert_eq!(0, move_action.turn_amount(*move_action));
    }

    assert_eq!(1, Direction::Down.turn_amount(Direction::DownLeft));
    assert_eq!(-1, Direction::Down.turn_amount(Direction::DownRight));

    assert_eq!(1, Direction::Left.turn_amount(Direction::UpLeft));
    assert_eq!(-1, Direction::Left.turn_amount(Direction::DownLeft));
}

#[test]
pub fn test_direction_clockwise() {
    let dir = Direction::Right;

    for _ in 0..8 {
        let new_dir = dir.clockwise();
        assert_eq!(1, dir.turn_amount(new_dir));
    }
    assert_eq!(Direction::Right, dir);
}

#[test]
pub fn test_direction_counterclockwise() {
    let dir = Direction::Right;

    for _ in 0..8 {
        let new_dir = dir.counterclockwise();
        assert_eq!(-1, dir.turn_amount(new_dir));
    }
    assert_eq!(Direction::Right, dir);
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialOrd, Ord, PartialEq, Serialize, Deserialize)]
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

    pub fn chr(&self) -> char {
        match self {
            TileType::Empty => 'e',
            TileType::ShortWall => 's',
            TileType::Wall => 'w',
            TileType::Water => 'a',
            TileType::Exit => 'x',
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AoeEffect {
    Sound,
    Freeze,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MapLoadConfig {
    Random,
    TestMap,
    TestWall,
    TestColumns,
    Empty,
    TestSmoke,
    TestCorner,
    TestPlayer,
    TestArmil,
    TestVaults,
    TestTraps,
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
            MapLoadConfig::TestColumns => write!(f, "test_columns"),
            MapLoadConfig::Empty => write!(f, "empty"),
            MapLoadConfig::TestSmoke => write!(f, "test_smoke"),
            MapLoadConfig::TestCorner => write!(f, "test_corner"),
            MapLoadConfig::TestPlayer => write!(f, "test_player"),
            MapLoadConfig::TestArmil => write!(f, "test_armil"),
            MapLoadConfig::TestVaults => write!(f, "test_vaults"),
            MapLoadConfig::TestTraps => write!(f, "test_traps"),
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
        } else if s == "test_columns" {
            return Ok(MapLoadConfig::TestColumns);
        } else if s == "test_wall" {
            return Ok(MapLoadConfig::TestWall);
        } else if s == "empty" {
            return Ok(MapLoadConfig::Empty);
        } else if s == "test_corner" {
            return Ok(MapLoadConfig::TestCorner);
        } else if s == "test_smoke" {
            return Ok(MapLoadConfig::TestSmoke);
        } else if s == "test_player" {
            return Ok(MapLoadConfig::TestPlayer);
        } else if s == "test_armil" {
            return Ok(MapLoadConfig::TestArmil);
        } else if s == "test_vaults" {
            return Ok(MapLoadConfig::TestVaults);
        } else if s == "test_traps" {
            return Ok(MapLoadConfig::TestTraps);
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


#[derive(Clone, Copy, Debug, Hash, Eq, PartialOrd, PartialEq, Serialize, Deserialize)]
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

impl Surface {
    pub fn chr(&self) -> char {
        match self {
            Surface::Floor => 'f',
            Surface::Rubble => 'r',
            Surface::Grass => 'g',
        }
    }
}


#[derive(Clone, Copy, Hash, Eq, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tile {
    pub block_move: bool,
    pub block_sight: bool,
    pub explored: bool,
    pub tile_type: TileType,
    pub bottom_wall: Wall,
    pub bottom_material: Surface,
    pub left_wall: Wall,
    pub left_material: Surface,
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
            bottom_material: Surface::Floor,
            left_material: Surface::Floor,
            surface: Surface::Floor,
        }
    }

    pub fn short_bottom_wall() -> Self {
        let mut tile = Tile::empty();
        tile.bottom_wall = Wall::ShortWall;
        return tile;
    }

    pub fn short_left_wall() -> Self {
        let mut tile = Tile::empty();
        tile.left_wall = Wall::ShortWall;
        return tile;
    }

    pub fn short_left_and_bottom_wall() -> Self {
        let mut tile = Tile::empty();
        tile.left_wall = Wall::ShortWall;
        tile.bottom_wall = Wall::ShortWall;
        return tile;
    }

    pub fn water() -> Self {
        Tile {
            block_move: true,
            block_sight: false,
            explored: false,
            tile_type: TileType::Water,
            bottom_wall: Wall::Empty,
            left_wall: Wall::Empty,
            bottom_material: Surface::Floor,
            left_material: Surface::Floor,
            surface: Surface::Floor,
        }
    }

    pub fn grass() -> Self {
        let mut tile = Tile::empty();
        tile.surface = Surface::Grass;
        return tile;
    }

    pub fn tall_grass() -> Self {
        let mut tile = Tile::empty();
        tile.block_sight = true;
        tile.surface = Surface::Grass;
        return tile;
    }

    pub fn rubble() -> Self {
        let mut tile = Tile::empty();
        tile.surface = Surface::Rubble;
        return tile;
    }

    pub fn wall() -> Self {
        Tile { block_move: true,
               block_sight: true,
               explored: false,
               tile_type: TileType::Wall,
               bottom_wall: Wall::Empty,
               left_wall: Wall::Empty,
               bottom_material: Surface::Floor,
               left_material: Surface::Floor,
               surface: Surface::Floor,
        }
    }

    pub fn short_wall() -> Self {
        Tile {
            block_move: false,
            block_sight: false,
            explored: false,
            tile_type: TileType::ShortWall,
            bottom_wall: Wall::Empty,
            left_wall: Wall::Empty,
            bottom_material: Surface::Floor,
            left_material: Surface::Floor,
            surface: Surface::Floor,
        }
    }

    pub fn clear_walls(&mut self) {
        self.block_move = false;
        self.block_sight = false;
        self.bottom_wall = Wall::Empty;
        self.bottom_material = Surface::Floor;
        self.left_wall = Wall::Empty;
        self.left_material = Surface::Floor;
        self.tile_type = TileType::Empty;
    }

    pub fn exit() -> Self {
        Tile {
            block_move: false,
            block_sight: false,
            explored: false,
            tile_type: TileType::Exit,
            bottom_wall: Wall::Empty,
            left_wall: Wall::Empty,
            bottom_material: Surface::Floor,
            left_material: Surface::Floor,
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

    pub fn does_left_block(&self) -> bool {
        return self.left_wall != Wall::Empty && self.left_material != Surface::Grass;
    }

    pub fn does_down_block(&self) -> bool {
        return self.bottom_wall != Wall::Empty && self.bottom_material != Surface::Grass;
    }

    pub fn shorten(&mut self) {
        if self.bottom_wall == Wall::TallWall {
            self.bottom_wall = Wall::ShortWall;
        }

        if self.left_wall == Wall::TallWall {
            self.left_wall = Wall::ShortWall;
        }
    }

    pub fn chrs(&self) -> [char; 8] {
        let mut chrs: [char; 8] = ['0'; 8];

        let mut index = 0;
        if self.block_move {
            chrs[index] = '1';
        } else {
            chrs[index] = '0';
        }
        index += 1;

        if self.block_sight {
            chrs[index] = '1';
        } else {
            chrs[index] = '0';
        }
        index += 1;

        chrs[index] = self.tile_type.chr();
        index += 1;

        chrs[index] = self.bottom_wall.chr();
        index += 1;

        chrs[index] = self.bottom_material.chr();
        index += 1;

        chrs[index] = self.left_wall.chr();
        index += 1;

        chrs[index] = self.left_material.chr();
        index += 1;

        chrs[index] = self.surface.chr();

        return chrs;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum FovResult {
    Outside,
    Edge,
    Inside,
}

impl fmt::Display for FovResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FovResult::Outside => write!(f, "outside"),
            FovResult::Edge => write!(f, "edge"),
            FovResult::Inside => write!(f, "inside"),
        }
    }
}

impl FromStr for FovResult {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();
        if s == "empty" {
            return Ok(FovResult::Outside);
        } else if s == "outside" {
            return Ok(FovResult::Edge);
        } else if s == "Edge" {
            return Ok(FovResult::Inside);
        }

        return Err(format!("Could not parse '{}' as Wall", s));
    }
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
                left_walls.push((pos, map[orig_pos].left_wall, map[orig_pos].left_material));
            }

            if map[orig_pos].bottom_wall != Wall::Empty {
                bottom_walls.push((pos, map[orig_pos].bottom_wall, map[orig_pos].bottom_material));
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

    for (wall_pos, wall_type, material) in left_walls {
        match rotation {
            Rotation::Degrees0 => {
                new_map[wall_pos].left_wall = wall_type;
                new_map[wall_pos].left_material = material;
            }

            Rotation::Degrees90 => {
                let new_wall_pos = move_y(wall_pos, -1);
                if new_map.is_within_bounds(new_wall_pos) {
                    new_map[new_wall_pos].bottom_wall = wall_type;
                    new_map[wall_pos].bottom_material = material;
                }
            }

            Rotation::Degrees180 => {
                let new_wall_pos = move_x(wall_pos, 1);
                if new_map.is_within_bounds(new_wall_pos) {
                    new_map[new_wall_pos].left_wall = wall_type;
                    new_map[wall_pos].left_material = material;
                }
            }

            Rotation::Degrees270 => {
                new_map[wall_pos].bottom_wall = wall_type;
                new_map[wall_pos].bottom_material = material;
            }
        }
    }

    for (wall_pos, wall_type, material) in bottom_walls {
        match rotation {
            Rotation::Degrees0 => {
                new_map[wall_pos].bottom_wall = wall_type;
                new_map[wall_pos].bottom_material = material;
            }

            Rotation::Degrees90 => {
                new_map[wall_pos].left_wall = wall_type;
                new_map[wall_pos].left_material = material;
            }

            Rotation::Degrees180 => {
                let new_wall_pos = move_y(wall_pos, -1);
                if new_map.is_within_bounds(new_wall_pos) {
                    new_map[new_wall_pos].bottom_wall = wall_type;
                    new_map[wall_pos].bottom_material = material;
                }
            }

            Rotation::Degrees270 => {
                let new_wall_pos = move_x(wall_pos, 1);
                if new_map.is_within_bounds(new_wall_pos) {
                    new_map[new_wall_pos].left_wall = wall_type;
                    new_map[wall_pos].left_material = material;
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

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
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

    pub fn chr(&self) -> char {
        match self {
            Wall::Empty => 'e',
            Wall::ShortWall => 's',
            Wall::TallWall => 't',
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


