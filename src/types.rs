use std::convert::Into;

use tcod::console::*;
use tcod::map::{Map as FovMap};
use tcod::input::Mouse;
use tcod::colors::*;
use tcod::line::*;

use crate::constants::*;


pub type ObjectId = usize;

pub struct Messages(pub Vec<(String, Color)>);

impl Messages {
    pub fn new() -> Messages {
        Messages(Vec::new())
    }

    pub fn message<T: Into<String>>(&mut self, message: T, color: Color) {
        if self.0.len() == MSG_HEIGHT {
            self.0.remove(0);
        }

        self.0.push((message.into(), color));
    }
}


pub struct Game {
    pub root: Root,
    pub console: Offscreen,
    pub fov: FovMap,
    pub mouse: Mouse,
    pub panel: Offscreen,
    pub turn_count: usize,
    pub display_overlays: bool,
    pub animations: Vec<Animation>,
    pub needs_clear: Vec<(i32, i32)>,
}

impl Game {
    pub fn with_root(root: Root) -> Game {
        Game {
            root: root,
            console: Offscreen::new(SCREEN_WIDTH, SCREEN_HEIGHT),
            fov: FovMap::new(MAP_WIDTH, MAP_HEIGHT),
            mouse: Default::default(),
            panel: Offscreen::new(SCREEN_WIDTH, PANEL_HEIGHT),
            turn_count: 0,
            display_overlays: false,
            animations: Vec::new(),
            needs_clear: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Reach {
    Single,
    Diag,
    Horiz,
}

impl Reach {
    pub fn offsets(&self) -> Vec<Position> {
        match self {
            Reach::Single => {
                let offsets = vec!((0,   1), (-1, 1), (-1,  0), (-1, -1), 
                                   (0,  -1), (1, -1), (1,  0),  (1, 1));
                offsets.iter().map(|pair| Position::from_pair(&pair)).collect()
            },

            Reach::Horiz => {
                let mut offsets = vec!();
                for dist in 1..5 {
                    offsets.push((dist, 0));
                    offsets.push((0, dist));
                    offsets.push((-1 * dist, 0));
                    offsets.push((0, -1 * dist));
                }
                offsets.iter().map(|pair| Position::from_pair(pair)).collect()
            },


            Reach::Diag => {
                let mut offsets = vec!();
                for dist in 1..5 {
                    offsets.push((dist, dist));
                    offsets.push((-1 * dist, dist));
                    offsets.push((dist, -1 * dist));
                    offsets.push((-1 * dist, -1 * dist));
                }
                offsets.iter().map(|pair| Position::from_pair(pair)).collect()
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct Object {
    pub x: i32,
    pub y: i32,
    pub char: char,
    pub color: Color,
    pub name: String,
    pub blocks: bool,
    pub alive: bool,
    pub fighter: Option<Fighter>,
    pub ai: Option<Ai>,
    pub behavior: Option<Behavior>,
    pub item: Option<Item>,
    pub momentum: Option<Momentum>,
    pub movement: Option<Reach>,
    pub attack: Option<Reach>,
}

impl Object {
    pub fn new(x: i32, y: i32, chr: char, name: &str, color: Color, blocks: bool) -> Self {
        Object {
            x: x,
            y: y,
            char: chr,
            color: color,
            name: name.into(),
            blocks: blocks,
            alive: false,
            fighter: None,
            ai: None,
            behavior: None,
            item: None,        
            momentum: None,
            movement: None,
            attack: None,
        }
    }

    pub fn draw(&self, console: &mut Console) {
        console.set_default_foreground(self.color);
        console.put_char(self.x, self.y, self.char, BackgroundFlag::None);
        console.set_default_foreground(WHITE);
    }

    pub fn clear(&self, console: &mut Console) {
        console.put_char(self.x, self.y, ' ', BackgroundFlag::None);
    }

    pub fn pos(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    pub fn distance_to(&self, other: &Object) -> f32 {
        return self.distance(&Position::new(other.x, other.y));
    }

    pub fn distance(&self, other: &Position) -> f32 {
        let dx = other.0 - self.x;
        let dy = other.1 - self.y;
        return ((dx.pow(2) + dy.pow(2)) as f32).sqrt();
    }

    pub fn take_damage(&mut self, damage: i32) {
        if let Some(fighter) = self.fighter.as_mut() {
            if damage > 0 {
                fighter.hp -= damage;
            }
        }

        if let Some(fighter) = self.fighter {
            if fighter.hp <= 0 {
                self.alive = false;
                fighter.on_death.callback(self);
            }
        }
    }

    pub fn attack(&mut self, target: &mut Object) {
        let damage = self.fighter.map_or(0, |f| f.power) - target.fighter.map_or(0, |f| f.defense);

        if damage > 0 {
            //messages.message(format!("{} attacks {} for {} hit points.", self.name, target.name, damage), WHITE);
            target.take_damage(damage);
        } else {
            //messages.message(format!("{} attacks {} but it has no effect!", self.name, target.name), WHITE);
        }
    }

    pub fn heal(&mut self, amount: i32) {
        if let Some(ref mut fighter) = self.fighter {
            fighter.hp += amount;
            if fighter.hp > fighter.max_hp {
                fighter.hp = fighter.max_hp;
            }
        }
    }

    pub fn make_stone(x: i32, y: i32) -> Object {
        Object::new(x, y, 'o', "stone", GREY, false)
    }
}


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ai {
    Basic,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Behavior {
    Idle,
    Seeking(Position),
    Attacking,
    Alert,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PatrolDir {
    Forward,
    Reverse,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AwarenessMap {
    pub weights: Vec<Vec<f32>>,
    pub alt_weights: Vec<Vec<f32>>,
    pub width: usize,
    pub height: usize,
}

impl AwarenessMap {
    pub fn new(width: usize, height: usize) -> AwarenessMap {
        AwarenessMap {
            weights: vec![vec![0.0; width]; height],
            alt_weights: vec![vec![0.0; width]; height],
            width: width,
            height: height,
        }
    }

    pub fn expected_position(&mut self, position: Position) {
        for y in 0..self.height {
            for x in 0..self.width {
                if (x as i32, y as i32) == position.pair() {
                    self.weights[y][x] = 1.0;
                } else {
                    self.weights[y][x] = 0.0;
                }
            }
        }
    }

    pub fn visible(&mut self, position: Position) {
        self.weights[position.1 as usize][position.0 as usize] = 0.0;
    }

    pub fn disperse(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                let potential_positions =
                    vec![(x + 1, y),     (x + 1, y + 1), (x + 1, y - 1),
                    (x,     y + 1), (x,     y - 1), (x - 1, y),
                    (x - 1, y + 1), (x - 1, y - 1)];
                let potential_positions =
                    potential_positions.iter()
                    .filter(|(x, y)| *x < self.width && *y < self.height)
                    .filter(|(x, y)| self.weights[*y as usize][*x as usize] > 0.0);

            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Item {
    Stone,
    Goal,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UseResult {
    UsedUp,
    Cancelled,
    Keep,
}


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AiAction {
    Move((i32, i32)),
    Attack((i32, i32)),
    Skip,
    StateChange(Behavior),
}


#[derive(Clone, Debug, PartialEq)]
pub struct AiTurn(Vec<AiAction>);

impl AiTurn {
    pub fn new() -> AiTurn {
        return AiTurn(Vec::new());
    }

    pub fn add(&mut self, action: AiAction) {
        self.0.push(action);
    }

    pub fn action(&self) -> Vec<AiAction> {
        return self.0;
    }
}


#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fighter {
    pub max_hp: i32,
    pub hp: i32,
    pub defense: i32,
    pub power: i32,
    pub on_death: DeathCallback,
}


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DeathCallback {
    Player,
    Monster,
}

impl DeathCallback {
    fn callback(self, object: &mut Object) {
        use DeathCallback::*;
        let callback: fn(&mut Object) = match self {
            Player => player_death,
            Monster => monster_death,
        };
        callback(object);
    }
}

pub fn player_death(player: &mut Object) {
    player.char = '%';
    player.color = DARK_RED;
}

pub fn monster_death(monster: &mut Object) {
    monster.char = '%';
    monster.color = DARK_RED;
    monster.blocks = false;
    monster.fighter = None;
    monster.ai = None;
    monster.name = format!("remains of {}", monster.name);
}


pub type Momentum = (i32, i32);


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MomentumChange {
    Lost,
    PreviousDirection,
    CurrentDirection,
}


#[derive(Clone, Copy, Debug)]
pub struct Rect  {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect { x1: x, y1: y, x2: x + w, y2: y + h }
    }

    pub fn center(&self) -> (i32, i32) {
        let center_x = (self.x1 + self.x2) / 2;
        let center_y = (self.y1 + self.y2) / 2;
        (center_x, center_y)
    }

    pub fn intersects_with(&self, other: &Rect) -> bool {
        (self.x1 <= other.x2) &&
            (self.x2 >= other.x1) &&
            (self.y1 <= other.y2) &&
            (self.y2 >= other.y1)
    }
}


#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position(pub i32, pub i32);

impl Position {
    pub fn new(x: i32, y: i32) -> Position {
        Position(x, y)
    }

    pub fn from_pair(pair: &(i32, i32)) -> Position {
        Position::new(pair.0, pair.1)
    }

    pub fn distance(&self, other: &Position) -> i32 {
        let dist_i32 = (self.0 - other.0).pow(2) + (self.1 - other.1).pow(2);
        (dist_i32 as f64).sqrt() as i32
    }

    pub fn pair(&self) -> (i32, i32) {
        (self.0, self.1)
    }

    pub fn move_by(&self, dist_x: i32, dist_y: i32) -> Position {
        Position(self.0 + dist_x, self.1 + dist_y)
    }

    pub fn move_x(&self, dist_x: i32) -> Position {
        Position(self.0 + dist_x, self.1)
    }

    pub fn move_y(&self, dist_y: i32) -> Position {
        Position(self.0, self.1 + dist_y)
    }

    pub fn add(&self, other: Position) -> Position{
        Position(self.0 + other.0, self.1 + other.1)
    }
}

impl Into<(i32, i32)> for Position {
    fn into(self) -> (i32, i32) {
        (self.0, self.1)
    }
}

pub enum Animation {
    Thrown(ObjectId, Line),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ColorConfig {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl ColorConfig {
    pub fn color(&self) -> Color {
        Color::new(self.r, self.g, self.b)
    }

    pub fn from_color(color: Color) -> ColorConfig {
        ColorConfig { r: color.r,
        g: color.g,
        b: color.b,
        }
    }
}


#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Config {
    pub color_dark_wall: ColorConfig,
    pub color_light_wall: ColorConfig,
    pub color_dark_ground: ColorConfig,
    pub color_light_ground: ColorConfig,
    pub color_dark_water: ColorConfig,
    pub color_light_water: ColorConfig,
    pub color_dark_exit: ColorConfig,
    pub color_light_exit: ColorConfig,
    pub color_kobold: ColorConfig,    
    pub color_light_ground_low: ColorConfig,
    pub color_light_ground_high: ColorConfig,
    pub color_orc: ColorConfig,
    pub color_troll: ColorConfig,
}

