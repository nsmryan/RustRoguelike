use std::convert::Into;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};

use serde::{Serialize, Deserialize};

use pathfinding::directed::astar::astar;

use tcod::line::*;

use slotmap::dense::*;
use slotmap::DefaultKey;

use symbol::Symbol;

use euclid::Point2D;

use crate::ai::{Ai, Behavior};
use crate::map::*;
use crate::movement::*;
use crate::animation::AnimKey;
use crate::utils::*;
use crate::config::Config;


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

    pub fn path_between(&self, start: Pos, end: Pos, reach: Reach) -> Vec<Pos> {
        let result;

        let maybe_results = 
            astar(&start,
                  |&pos| {
                      // NOTE this allocation could be avoided with an Iterable
                      let mut next_positions = Vec::new();

                      for direction in Direction::move_actions() {
                          for offset in reach.move_with_reach(&direction) {
                              let next_pos = add_pos(pos, offset);
                              if self.clear_path(pos, next_pos) {
                                  next_positions.push((next_pos, 1));
                              }
                          }
                      }

                      return next_positions;
                  },
                  |&pos| distance(pos, end) as i32,
                  |&pos| pos == end);

        if let Some((results, _cost)) = maybe_results {
            result = results;
        } else {
            result = Vec::new();
        }

        return result;
    }

    pub fn find_player(&self) -> Option<ObjectId> {
        for (key, object) in self.objects.iter() {
            if object.typ == ObjType::Player {
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

    pub fn clear_path(&self, start: Pos, end: Pos) -> bool {
        let line = Line::new((start.x, start.y), (end.x, end.y));
    
        let path_blocked =
            line.into_iter().any(|point|
                                 self.has_blocking_entity(Pos::from(point)).is_some());
    
        let (dx, dy) = (end.x - start.x, end.y - start.y);

        let blocked_by_wall = self.map.is_blocked_by_wall(start, dx, dy).is_some();
        return !path_blocked && !blocked_by_wall;
               
    }

    pub fn has_blocking_entity(&self, pos: Pos) -> Option<ObjectId> {
        for (key, object) in self.objects.iter() {
            if object.blocks && object.pos() == pos {
                return Some(key);
            }
        }

        return None;
    }

    pub fn walked_into(&self, start_pos: Pos, end_pos: Pos, look_ahead: i32) -> Option<ObjectId> {
        let pos_diff = sub_pos(end_pos, start_pos);
        let x_dir = if pos_diff.x == 0 {
            0
        } else {
            pos_diff.x.signum()
        };

        let y_dir = if pos_diff.y == 0 {
            0
        } else {
            pos_diff.y.signum()
        };

        if self.map.is_blocked_by_wall(end_pos, x_dir * look_ahead, y_dir * look_ahead).is_some() {
            return None;
        } else {
            let move_pos = (end_pos.x + x_dir * look_ahead,
                            end_pos.y + y_dir * look_ahead);
            let line = Line::new((end_pos.x, end_pos.y), move_pos);

            for pos in line {
                let maybe_id = self.has_blocking_entity(Pos::new(pos.0, pos.1));
                if maybe_id.is_some() {
                    return maybe_id;
                }
            }

            return None;
        }
    }

    pub fn using(&self, obj_id: ObjectId, item: Item) -> bool {
        if let Some(item_id) = self.objects[obj_id].inventory.get(0) {
            return self.objects[*item_id].item == Some(item);
        } else {
            return false;
        }
    }

    pub fn sound_at(&mut self, cause_id: ObjectId, source_pos: Pos, radius: usize) -> Aoe {
        let sound_aoe =
            self.map.aoe_fill(AoeEffect::Sound, source_pos, radius);

        for sound_pos in sound_aoe.positions() {
            for (obj_id, obj) in self.objects.iter_mut() {
                if obj.pos() == sound_pos && obj_id != cause_id {
                    obj.messages.push(Message::Sound(cause_id, source_pos));
                }
            }
        }

        return sound_aoe;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Trap {
    Spikes,
    Sound,
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

#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub enum GameState {
    Playing,
    Win,
    Lose,
    Inventory,
    Throwing,
    Console,
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


#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Item {
    Stone,
    Goal,
    Dagger,
    Shield,
}

impl Item {
    pub fn class(&self) -> ItemClass {
        match self {
            Item::Stone => ItemClass::Secondary,
            Item::Goal => ItemClass::Secondary,
            Item::Dagger => ItemClass::Primary,
            Item::Shield => ItemClass::Primary,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ItemClass {
    Primary,
    Secondary,
}


pub type Hp = i32;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ObjType {
    Player,
    Enemy,
    Item,
    Column,
    Other,
}


pub type Pos = Point2D<i32, ()>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Message {
    Sound(ObjectId, Pos),
    Attack(ObjectId),
}

static OBJECT_ID_COUNT: AtomicU32 = AtomicU32::new(0);

// TODO consider replacing this with BTreeMaps
// game.data.objects[handle].x would become
// game.data.objects.x.get(handle) or
// game.data.objects.x(handle) or
// game.data.objects.get_x(handle) with an Option<T> and T variant
#[derive(Clone, Debug, PartialEq)]
pub struct Object {
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub chr: char,
    pub typ: ObjType,
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
    pub inventory: VecDeque<ObjectId>,
    pub trap: Option<Trap>,
    pub sound: Option<Pos>, // source position
    pub count_down: Option<usize>,
    pub move_mode: Option<MoveMode>,
    pub needs_removal: bool,
    pub direction: Option<Direction>,
    pub messages: Vec<Message>,
    pub selected_item: Option<ObjectId>,
}

impl Object {
    pub fn new(x: i32, y: i32, typ: ObjType, chr: char, color: Color, name: &str, blocks: bool) -> Self {
        let id = OBJECT_ID_COUNT.fetch_add(1, Ordering::SeqCst);

        Object {
            id,
            x,
            y,
            typ,
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
            inventory: VecDeque::new(),
            trap: None,
            sound: None,
            count_down: None,
            move_mode: None,
            needs_removal: false,
            direction: Some(Direction::Up),
            messages: Vec::new(),
            selected_item: None,
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

    pub fn move_to(&mut self, pos: Pos) {
        let self_pos = self.pos();
        let diff_x = pos.x - self_pos.x;
        let diff_y = pos.y - self_pos.y;

        if self_pos != pos {
            self.direction = Direction::from_dxy(diff_x, diff_y);
        }

        self.set_pos(pos);
    }

    pub fn move_next_to(&mut self, pos: Pos) {
        let self_pos = self.pos();
        let mut diff_x = pos.x - self_pos.x;
        let mut diff_y = pos.y - self_pos.y;

        if diff_x != 0 {
            diff_x -= diff_x.signum();
        }

        if diff_y != 0 {
            diff_y -= diff_y.signum();
        }

        self.direction = Direction::from_dxy(diff_x, diff_y);
        self.set_pos(Pos::new(self.x + diff_x, self.y + diff_y));
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

    pub fn is_in_fov(&self, map: &mut Map, other_pos: Pos, config: &Config) -> bool {
        let pos = self.pos();

        let radius: i32 = match self.typ {
            ObjType::Enemy => config.fov_radius_monster,
            ObjType::Player => config.fov_radius_player,
            _ => panic!(format!("Tried to see with object of type {:?}", self.typ)),
        };

        if let Some(dir) = self.direction {
            if map.is_in_fov_direction(pos, other_pos, radius, dir) {
                return true;
            }
        }

        return false;
    }

    pub fn was_attacked(&mut self) -> Option<Message> {
        if let Some(index) = self.messages.iter().position(|msg| matches!(msg, Message::Attack(_))) {
            return Some(self.messages.remove(index));
        } else {
            return None;
        }
    }

    pub fn heard_sound(&mut self) -> Option<Message> {
        if let Some(index) = self.messages.iter().position(|msg| matches!(msg, Message::Sound(_, _))) {
            return Some(self.messages.remove(index));
        } else {
            return None;
        }
    }

    pub fn face(&mut self, face_pos: Pos) {
        let diff_pos = sub_pos(face_pos, self.pos());
        self.direction = Direction::from_dxy(diff_pos.x, diff_pos.y);
    }
}

