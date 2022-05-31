use std::str::FromStr;
use std::fmt;
use std::hash::Hash;

use serde_derive::*;

use parse_display::{Display, FromStr};

use roguelike_utils::math::Pos;

use crate::pathing::*;
use crate::utils::*;
use crate::map::*;


#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd, Display, FromStr, Serialize, Deserialize)]
#[display(style = "lowercase")]
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

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd, Display, FromStr, Serialize, Deserialize)]
#[display(style = "lowercase")]
pub enum PlayerDirection {
    Left,
    Right,
    Up,
    Down,
    DownLeft,
    DownRight,
    UpLeft,
    UpRight,
    Center,
}

impl PlayerDirection {
    pub fn from_direction(dir: Direction) -> PlayerDirection {
        match dir {
            Direction::Left => PlayerDirection::Left,
            Direction::Right => PlayerDirection::Right,
            Direction::Up => PlayerDirection::Up,
            Direction::Down => PlayerDirection::Down,
            Direction::DownLeft => PlayerDirection::DownLeft,
            Direction::DownRight => PlayerDirection::DownRight,
            Direction::UpLeft => PlayerDirection::UpLeft,
            Direction::UpRight => PlayerDirection::UpRight,
        }
    }

    pub fn to_direction(&self) -> Option<Direction> {
        match self {
            PlayerDirection::Left => Some(Direction::Left),
            PlayerDirection::Right => Some(Direction::Right),
            PlayerDirection::Up => Some(Direction::Up),
            PlayerDirection::Down => Some(Direction::Down),
            PlayerDirection::DownLeft => Some(Direction::DownLeft),
            PlayerDirection::DownRight => Some(Direction::DownRight),
            PlayerDirection::UpLeft => Some(Direction::UpLeft),
            PlayerDirection::UpRight => Some(Direction::UpRight),
            PlayerDirection::Center => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialOrd, Ord, PartialEq, Display, FromStr, Serialize, Deserialize)]
#[display(style = "lowercase")]
pub enum TileType {
    Empty,
    ShortWall,
    Wall,
    Water,
    Exit,
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

#[derive(Clone, Debug, PartialEq, Display, FromStr, Serialize, Deserialize)]
#[display(style = "snake_case")]
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
    #[display("vault_file {0}")]
    VaultFile(String),
    #[display("procgen {0}")]
    ProcGen(String),
}

impl Default for MapLoadConfig {
    fn default() -> MapLoadConfig {
        return MapLoadConfig::Random;
    }
}

#[derive(Clone, Copy, Debug, Hash, Eq, PartialOrd, PartialEq, Display, FromStr, Serialize, Deserialize)]
#[display(style = "lowercase")]
pub enum Surface {
    Floor,
    Rubble,
    Grass,
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

