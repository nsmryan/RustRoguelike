use std::str::FromStr;
use std::fmt;
use std::hash::Hash;

use euclid::*;

use serde_derive::*;

use crate::pathing::*;
use crate::utils::*;
use crate::map::*;


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

