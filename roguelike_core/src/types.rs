use std::convert::Into;

use serde_derive::*;

use tcod::line::*;

use num::clamp;

use slotmap::dense::*;
use slotmap::DefaultKey;

use euclid::Point2D;

use crate::map::*;
use crate::constants::*;
use crate::movement::*;


// TODO consider using custom key types to distinguish
// muliple maps
pub type ObjectId = DefaultKey;

pub type ObjMap = DenseSlotMap<ObjectId, Object>;

pub type SpriteKey = DefaultKey;

pub type SpriteIndex = f32;

pub struct GameData {
    pub map: Map,
    pub objects: ObjMap,
}

impl GameData {
    pub fn new(map: Map, objects: ObjMap) -> GameData {
        GameData {
            map,
            objects,
        }
    }

    pub fn find_player(&self) -> Option<ObjectId> {
        for (key, object) in self.objects.iter() {
            if object.name == "player" {
                return Some(key);
            }
        }

        return None;
    }

    pub fn find_mouse(&self) -> Option<ObjectId> {
        for (key, object) in self.objects.iter() {
            if object.name == "mouse" {
                return Some(key);
            }
        }

        return None;
    }

    pub fn sound_within_earshot(&self, pos: Pos) -> Option<ObjectId> {
        for (object_id, object) in self.objects.iter() {
            if object.name == "sound" && distance(pos, Pos::new(object.x, object.y)) < SOUND_RADIUS as i32 {
                return Some(object_id);
            }
        }

        return None;
    }

    pub fn clear_path(&self, start: Pos, end: Pos) -> bool {
        let line = Line::new((start.x, start.y), (end.x, end.y));
    
        let path_blocked =
            line.into_iter().any(|point| is_blocked(Pos::from(point), self));
    
        let (dx, dy) = (end.x - start.x, end.y - start.y);

        return !path_blocked &&
               !self.map.is_blocked_by_wall(start, dx, dy);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Trap {
    Spikes,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn white() -> Color {
        return Color::new(255, 255, 255, 255);
    }

    pub fn black() -> Color {
        return Color::new(0, 0, 0, 255);
    }

    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Color {
        return Color {r,
                      g,
                      b,
                      a,
        };
    }
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum GameState {
    Playing,
    Win,
    Lose,
}


#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub left_pressed: bool,
    pub middle_pressed: bool,
    pub right_pressed: bool,
    pub wheel: f32,
}


#[derive(Clone, Debug, PartialEq)]
pub enum Animation {
    Idle(SpriteKey, SpriteIndex),
    WallKick(SpriteKey, SpriteIndex, Pos, Pos),
    StoneThrow(Pos, Pos),
    Sound(usize, usize), // current radius, max radius
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Item {
    Stone,
    Goal,
}


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Move(Movement),
    Attack(ObjectId, Pos),
    StateChange(Behavior),
    ThrowStone,
    NoAction,
}

impl Action {
    pub fn none() -> Action {
        return Action::NoAction; }
}


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ai {
    Basic,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Behavior {
    Idle,
    Investigating(Pos),
    Attacking(ObjectId),
}

impl Behavior {
    pub fn description(&self) -> String {
        match self {
            Behavior::Idle => "idle".to_string(),
            Behavior::Investigating(_position) => "investigating".to_string(),
            Behavior::Attacking(_obj_id) => "attacking".to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fighter {
    pub max_hp: i32,
    pub hp: i32,
    pub defense: i32,
    pub power: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Momentum {
    pub mx: i32,
    pub my: i32,
    pub took_half_turn: bool,
    pub max: i32,
}

impl Default for Momentum {
    fn default() -> Momentum {
        Momentum {
            mx: 0,
            my: 0,
            took_half_turn: false,
            max: 2, // TODO make this configurable
        }
    }
}

impl Momentum {
    pub fn running(&mut self) -> bool {
        return self.magnitude() != 0;
    }

    pub fn at_maximum(&self) -> bool {
        return self.magnitude() == MAX_MOMENTUM;
    }
        
    pub fn magnitude(&self) -> i32 {
        if self.mx.abs() > self.my.abs() {
            return self.mx.abs();
        } else {
            return self.my.abs();
        }
    }

    pub fn diagonal(&self) -> bool {
        return self.mx.abs() != 0 && self.my.abs() != 0;
    }

    pub fn moved(&mut self, dx: i32, dy: i32) {
        // if the movement is in the opposite direction, and we have some momentum
        // currently, lose our momentum.

        if self.mx != 0 && dx.signum() != self.mx.signum() {
            self.mx = 0;
        } else {
            self.mx = clamp(self.mx + dx.signum(), -self.max, self.max);
        }

        if self.my != 0 && dy.signum() != self.my.signum() {
            self.my = 0;
        } else {
            self.my = clamp(self.my + dy.signum(), -self.max, self.max);
        }
    }

    pub fn set_momentum(&mut self, mx: i32, my: i32) {
        self.mx = mx;
        self.my = my;
    }

    pub fn along(&self, dx: i32, dy: i32) -> bool {
        return (self.mx * dx + self.my * dy) > 0;
    }

    pub fn clear(&mut self) {
        self.mx = 0;
        self.my = 0;
    }
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


pub type Pos = Point2D<i32, ()>;


#[derive(Clone, Debug, PartialEq)]
pub struct Object {
    pub x: i32,
    pub y: i32,
    pub chr: char,
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
    pub animation: Option<Animation>,
    pub inventory: Vec<ObjectId>,
    pub trap: Option<Trap>,
}

impl Object {
    pub fn new(x: i32, y: i32, chr: char, color: Color, name: &str, blocks: bool) -> Self {
        Object {
            x,
            y,
            chr,
            color,
            name: name.into(),
            blocks,
            alive: false,
            fighter: None,
            ai: None,
            behavior: None,
            item: None,        
            momentum: None,
            movement: None,
            attack: None,
            animation: None,
            inventory: Vec::new(),
            trap: None,
        }
    }

    pub fn pos(&self) -> Pos {
        return Pos::new(self.x, self.y);
    }

    pub fn set_xy(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    pub fn set_pos(&mut self, pos: Pos) {
        self.x = pos.x;
        self.y = pos.y;
    }

    pub fn distance_to(&self, other: &Object) -> f32 {
        return self.distance(&Pos::new(other.x, other.y));
    }

    pub fn distance(&self, other: &Pos) -> f32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
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
            }
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
}

pub fn distance(pos1: Pos, pos2: Pos) -> i32 {
    return (((pos1.x - pos2.x).pow(2) + (pos1.y - pos2.y).pow(2)) as f32).sqrt() as i32;
}

