use std::convert::Into;
use std::collections::VecDeque;

use serde_derive::*;

use tcod::line::*;

use slotmap::dense::*;
use slotmap::DefaultKey;

use symbol::Symbol;

use euclid::Point2D;

use crate::map::*;
use crate::utils::*;
use crate::constants::*;
use crate::movement::*;
use crate::animation::AnimKey;


pub type Name = Symbol;

pub type ObjectId = DefaultKey;

pub type ObjMap = DenseSlotMap<ObjectId, Object>;

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
            if let Some(radius) = object.sound {
                if distance(pos, Pos::new(object.x, object.y)) < radius as i32 {
                    return Some(object_id);
                }
            }
        }

        return None;
    }

    pub fn clear_path(&self, start: Pos, end: Pos) -> bool {
        let line = Line::new((start.x, start.y), (end.x, end.y));
    
        let path_blocked =
            line.into_iter().any(|point| self.is_blocked_tile(Pos::from(point)));
    
        let (dx, dy) = (end.x - start.x, end.y - start.y);

        return !path_blocked &&
               self.map.is_blocked_by_wall(start, dx, dy).is_none();
    }

    pub fn is_blocked_tile(&self, pos: Pos) -> bool {
        if !self.map.is_within_bounds(pos) {
            return true;
        }

        let mut is_blocked = false;
        for object in self.objects.values() {
            if object.blocks && object.pos() == pos {
                is_blocked = true;
                break;
            }
        }

        return is_blocked;
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


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Item {
    Stone,
    Goal,
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

pub type Hp = i32;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fighter {
    pub max_hp: Hp,
    pub hp: Hp,
    pub defense: i32,
    pub power: i32,
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
    pub movement: Option<Reach>,
    pub attack: Option<Reach>,
    pub animation: VecDeque<AnimKey>,
    pub inventory: Vec<ObjectId>,
    pub trap: Option<Trap>,
    pub sound: Option<usize>,
    pub count_down: Option<usize>,
    pub move_mode: Option<MoveMode>,
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
            movement: None,
            attack: None,
            animation: VecDeque::new(),
            inventory: Vec::new(),
            trap: None,
            sound: None,
            count_down: None,
            move_mode: None,
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

