use std::convert::Into;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::default::Default;

use serde::{Serialize, Deserialize};

use pathfinding::directed::astar::astar;

use indexmap::map::IndexMap;

use tcod::line::*;

use symbol::Symbol;

use euclid::Point2D;

use crate::ai::{Ai, Behavior};
use crate::map::*;
use crate::movement::*;
use crate::animation::AnimKey;
use crate::utils::*;
use crate::config::Config;

// TODO consider renaming this file- it really just contains
// GameData and Entities. likely split into separate files.

// TODO consider adding a symbol store to GameData and removing all Strings
pub type Name = Symbol;

pub type EntityId = u64;

pub type CompStore<T> = IndexMap<EntityId, T>;

pub struct GameData {
    pub map: Map,
    pub entities: Entities,
}

impl GameData {
    pub fn new(map: Map, entities: Entities) -> GameData {
        GameData {
            map,
            entities,
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

    pub fn find_player(&self) -> Option<EntityId> {
        for (key, typ) in self.entities.typ.iter() {
            if *typ == ObjType::Player {
                return Some(*key);
            }
        }

        return None;
    }

    pub fn find_mouse(&self) -> Option<EntityId> {
        for (key, name) in self.entities.name.iter() {
            if name == "mouse" {
                return Some(*key);
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

    pub fn has_blocking_entity(&self, pos: Pos) -> Option<EntityId> {
        for key in self.entities.ids.iter() {
            if self.entities.blocks[key] && self.entities.pos[key] == pos {
                return Some(*key);
            }
        }

        return None;
    }

    pub fn walked_into(&self, start_pos: Pos, end_pos: Pos, look_ahead: i32) -> Option<EntityId> {
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

    pub fn using(&self, entity_id: EntityId, item: Item) -> bool {
        if let Some(item_id) = self.entities.inventory[&entity_id].get(0) {
            return self.entities.item[item_id] == item;
        } else {
            return false;
        }
    }

    pub fn within_aoe(&mut self, aoe: &Aoe) -> Vec<EntityId> {
        let mut within = Vec::new();

        for pos in aoe.positions() {
            for entity_id in self.entities.ids.iter() {
                if self.entities.pos[entity_id] == pos {
                    within.push(*entity_id);
                }
            }
        }

        return within;
    }

    pub fn remove_entity(&mut self, id: EntityId) {
        let ix_pos = self.entities.ids.iter().position(|val| *val == id).unwrap();
        self.entities.ids.remove(ix_pos);

        self.entities.pos.remove(&id);
        self.entities.chr.remove(&id);
        self.entities.name.remove(&id);
        self.entities.fighter.remove(&id);
        self.entities.ai.remove(&id);
        self.entities.behavior.remove(&id);
        self.entities.item.remove(&id);
        self.entities.movement.remove(&id);
        self.entities.attack.remove(&id);
        self.entities.inventory.remove(&id);
        self.entities.trap.remove(&id);
        self.entities.count_down.remove(&id);
        self.entities.move_mode.remove(&id);
        self.entities.direction.remove(&id);
        self.entities.selected_item.remove(&id);
        self.entities.action.remove(&id);
        self.entities.animation.remove(&id);
        self.entities.alive.remove(&id);
        self.entities.sound.remove(&id);
        self.entities.typ.remove(&id);
        self.entities.color.remove(&id);
        self.entities.blocks.remove(&id);
        self.entities.needs_removal.remove(&id);
        self.entities.messages.remove(&id);
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
    Sound(EntityId, Pos),
    Attack(EntityId),
}

static OBJECT_ID_COUNT: AtomicU64 = AtomicU64::new(0);

// TODO consider replacing this with BTreeMaps
// game.data.objects[handle].x would become
// game.data.objects.x.get(handle) or
// game.data.objects.x(handle) or
// game.data.objects.x[handle] or
// game.data.objects.get_x(handle) with an Option<T> and T variant
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Entities {
    pub ids: Vec<EntityId>,
    pub pos: CompStore<Pos>,
    pub chr: CompStore<char>,
    pub name: CompStore<String>,
    pub fighter: CompStore<Fighter>,
    pub ai: CompStore<Ai>,
    pub behavior: CompStore<Behavior>,
    pub item: CompStore<Item>,
    pub movement: CompStore<Reach>,
    pub attack: CompStore<Reach>,
    pub inventory: CompStore<VecDeque<EntityId>>,
    pub trap: CompStore<Trap>,
    pub count_down: CompStore<usize>,
    pub move_mode: CompStore<MoveMode>,
    pub direction: CompStore<Direction>,
    pub selected_item: CompStore<EntityId>,
    pub action: CompStore<Action>,

    // TODO should end up in animation system instead
    pub animation: CompStore<VecDeque<AnimKey>>,

    // TODO not sure about keeping these ones, or packaging into larger ones
    pub alive: CompStore<bool>,
    pub sound: CompStore<Pos>, // source position
    pub typ: CompStore<ObjType>,
    pub color: CompStore<Color>,
    pub blocks: CompStore<bool>,
    pub needs_removal: CompStore<bool>,

    // TODO should be able to remove this
    pub messages: CompStore<Vec<Message>>,
}

impl Entities {
    pub fn remove(&mut self, entity_id: &EntityId) {
        let id_ix = self.ids.iter().position(|id| id == entity_id).unwrap();
        self.ids.swap_remove(id_ix);

        self.pos.remove(entity_id);
        self.chr.remove(entity_id);
        self.name.remove(entity_id);
        self.fighter.remove(entity_id);
        self.ai.remove(entity_id);
        self.behavior.remove(entity_id);
        self.item.remove(entity_id);
        self.movement.remove(entity_id);
        self.attack.remove(entity_id);
        self.inventory.remove(entity_id);
        self.trap.remove(entity_id);
        self.count_down.remove(entity_id);
        self.move_mode.remove(entity_id);
        self.direction.remove(entity_id);
        self.selected_item.remove(entity_id);
        self.action.remove(entity_id);
        self.animation.remove(entity_id);
        self.alive.remove(entity_id);
        self.sound.remove(entity_id);
        self.typ.remove(entity_id);
        self.color.remove(entity_id);
        self.blocks.remove(entity_id);
        self.needs_removal.remove(entity_id);
        self.messages.remove(entity_id);
    }
}

impl Entities {
    pub fn new() -> Entities {
        return Default::default();
        // TODO should be able to remove this
        //return Entities {
        //    id: Vec::new(),
        //    pos: BTreeMap::new(),
        //    typ: BTreeMap::new(),
        //    chr: BTreeMap::new(),
        //    color: BTreeMap::new(),
        //    name: BTreeMap::new(),
        //    blocks: BTreeMap::new(),
        //    alive: BTreeMap::new(),
        //    fighter: BTreeMap::new(),
        //    ai: BTreeMap::new(),
        //    behavior: BTreeMap::new(),
        //    item: BTreeMap::new(),
        //    movement: BTreeMap::new(),
        //    attack: BTreeMap::new(),
        //    animation: BTreeMap::new(),
        //    inventory: BTreeMap::new(),
        //    trap: BTreeMap::new(),
        //    sound: BTreeMap::new(),
        //    count_down: BTreeMap::new(),
        //    move_mode: BTreeMap::new(),
        //    needs_removal: BTreeMap::new(),
        //    direction: BTreeMap::new(),
        //    messages: BTreeMap::new(),
        //    selected_item: BTreeMap::new(),
        //    action: BTreeMap::new(),
        //}
    }

    pub fn clear(&mut self) {
        *self = Default::default();
    }

    // TODO consider simplifying this and allowing these fields to be set
    pub fn create_entity(&mut self, x: i32, y: i32, typ: ObjType, chr: char, color: Color, name: &str, blocks: bool) -> EntityId {
        let id = OBJECT_ID_COUNT.fetch_add(1, Ordering::SeqCst);
        self.ids.push(id);

        self.pos.insert(id, Pos::new(x, y));
        self.typ.insert(id, typ);
        self.chr.insert(id, chr);
        self.color.insert(id, color);
        self.name.insert(id, name.into());
        self.blocks.insert(id, blocks);
        self.alive.insert(id, false);
        self.direction.insert(id, Direction::Up);

        return id;
    }

    pub fn set_xy(&mut self, entity_id: EntityId, x: i32, y: i32) {
        self.pos[&entity_id] = Pos::new(x, y);
    }

    pub fn set_pos(&mut self, entity_id: EntityId, pos: Pos) {
        self.pos[&entity_id] = pos;
    }

    pub fn move_to(&mut self, entity_id: EntityId, pos: Pos) {
        let self_pos = self.pos[&entity_id];
        let diff_x = pos.x - self_pos.x;
        let diff_y = pos.y - self_pos.y;

        if self_pos != pos {
            if let Some(dir) = Direction::from_dxy(diff_x, diff_y) {
                self.direction[&entity_id] = dir;
            }
        }

        self.set_pos(entity_id, pos);
    }

    pub fn move_next_to(&mut self, entity_id: EntityId, pos: Pos) {
        let self_pos = self.pos[&entity_id];
        let mut diff_x = pos.x - self_pos.x;
        let mut diff_y = pos.y - self_pos.y;

        if diff_x != 0 {
            diff_x -= diff_x.signum();
        }

        if diff_y != 0 {
            diff_y -= diff_y.signum();
        }

        if let Some(dir) = Direction::from_dxy(diff_x, diff_y) {
            self.direction[&entity_id] = dir;
        }

        self.set_pos(entity_id, Pos::new(self_pos.x + diff_x, self_pos.y + diff_y));
    }

    pub fn distance_to(&self, entity_id: EntityId, other: EntityId) -> f32 {
        let other_pos = self.pos[&other];
        return self.distance(entity_id, &other_pos);
    }

    pub fn distance(&self, entity_id: EntityId, other: &Pos) -> f32 {
        let pos = self.pos[&entity_id];
        let dx = other.x - pos.x;
        let dy = other.y - pos.y;
        return ((dx.pow(2) + dy.pow(2)) as f32).sqrt();
    }

    pub fn take_damage(&mut self, entity: EntityId, damage: i32) {
        if let Some(fighter) = self.fighter.get_mut(&entity) {
            if damage > 0 {
                fighter.hp -= damage;
            }
        }

        if let Some(fighter) = self.fighter.get(&entity) {
            if fighter.hp <= 0 {
                self.alive[&entity] = false;
            }
        }
    }

    pub fn is_in_fov(&self, entity_id: EntityId, map: &mut Map, other_pos: Pos, config: &Config) -> bool {
        let pos = self.pos[&entity_id];

        let radius: i32 = match self.typ[&entity_id] {
            ObjType::Enemy => config.fov_radius_monster,
            ObjType::Player => config.fov_radius_player,
            _ => panic!(format!("Tried to see with object of type {:?}", self.typ)),
        };

        if let Some(dir) = self.direction.get(&entity_id) {
            if map.is_in_fov_direction(pos, other_pos, radius, *dir) {
                return true;
            }
        }

        return false;
    }

    pub fn was_attacked(&mut self, entity_id: EntityId) -> Option<Message> {
        if let Some(index) = self.messages[&entity_id].iter().position(|msg| matches!(msg, Message::Attack(_))) {
            return Some(self.messages[&entity_id].remove(index));
        } else {
            return None;
        }
    }

    pub fn heard_sound(&mut self, entity_id: EntityId) -> Option<Message> {
        if let Some(index) = self.messages[&entity_id].iter().position(|msg| matches!(msg, Message::Sound(_, _))) {
            return Some(self.messages[&entity_id].remove(index));
        } else {
            return None;
        }
    }

    pub fn face(&mut self, entity_id: EntityId, face_pos: Pos) {
        let diff_pos = sub_pos(face_pos, self.pos[&entity_id]);
        if let Some(dir) = Direction::from_dxy(diff_pos.x, diff_pos.y) {
            self.direction[&entity_id] = dir;
        }
    }
}

