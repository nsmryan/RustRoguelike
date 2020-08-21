use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::default::Default;

use serde::{Serialize, Deserialize};

use pathfinding::directed::astar::astar;

use indexmap::map::IndexMap;

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

pub type Pos = Point2D<i32, ()>;

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

    pub fn path_between(&self,
                        start: Pos,
                        end: Pos,
                        reach: Reach,
                        must_reach: bool,
                        traps_block: bool,
                        cost_fun: Option<fn(Pos, Pos, Pos, &GameData) -> Option<i32>>) -> Vec<Pos> {
        let result;

        let maybe_results =
            astar(&start,
                  |&pos| {
                      // NOTE(perf) this allocation could be avoided with an Iterable
                      let mut next_positions = Vec::new();

                      for direction in Direction::move_actions() {
                          for offset in reach.move_with_reach(&direction) {
                              let next_pos = add_pos(pos, offset);
                              let (dx, dy) = (next_pos.x - pos.x, next_pos.y - pos.y);

                              let clear = self.clear_path(pos, next_pos, traps_block);
                              if  clear ||
                                  (!must_reach && next_pos == end && self.map.is_blocked_by_wall(pos, dx, dy).is_none()) {

                                 let mut cost = 1;
                                  if let Some(cost_fun) = cost_fun {
                                      if let Some(cur_cost) = cost_fun(start, pos, next_pos, self) {
                                          cost = cur_cost;
                                      } else {
                                          continue;
                                      }
                                  }
                                  next_positions.push((next_pos, cost));
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

    pub fn is_in_fov(&mut self, entity_id: EntityId, other_pos: Pos, config: &Config) -> bool {
        let pos = self.entities.pos[&entity_id];

        let radius: i32 = match self.entities.typ[&entity_id] {
            EntityType::Enemy => config.fov_radius_monster,
            EntityType::Player => config.fov_radius_player,
            typ => panic!(format!("Tried to see with object of type {:?}", typ)),
        };
        
        if self.entities.typ[&entity_id] == EntityType::Player {
            return self.map.is_in_fov(pos, other_pos, radius);
        } else {
            if let Some(dir) = self.entities.direction.get(&entity_id) {
                return self.map.is_in_fov_direction(pos, other_pos, radius, *dir);
            } else {
                panic!(format!("tried to perform is_in_fov on entity without facing"));
            }
        }

        return false;
    }

    pub fn find_player(&self) -> Option<EntityId> {
        for (key, typ) in self.entities.typ.iter() {
            if *typ == EntityType::Player {
                return Some(*key);
            }
        }

        return None;
    }

    pub fn find_exit(&self) -> Option<EntityId> {
        for (key, name) in self.entities.name.iter() {
            if *name == EntityName::Exit {
                return Some(*key);
            }
        }

        return None;
    }

    pub fn find_mouse(&self) -> Option<EntityId> {
        for (key, name) in self.entities.name.iter() {
            if *name == EntityName::Mouse {
                return Some(*key);
            }
        }

        return None;
    }

    pub fn clear_path_up_to(&self, start: Pos, end: Pos, traps_block: bool) -> bool {
        let up_to = move_next_to(start, end);

        return self.clear_path(start, up_to, traps_block);
    }

    pub fn clear_path(&self, start: Pos, end: Pos, traps_block: bool) -> bool {
        let line = line_inclusive(start, end);

        let path_blocked =
            line.into_iter().any(|point| {
                let pos = Pos::from(point);
                return self.has_blocking_entity(pos).is_some() || (traps_block && self.has_trap(pos).is_some());
            });

        let (dx, dy) = (end.x - start.x, end.y - start.y);

        let blocked_by_wall = self.map.is_blocked_by_wall(start, dx, dy).is_some();
        return !path_blocked && !blocked_by_wall;
    }

    pub fn has_item_in_inventory(&self, entity_id: EntityId, item: Item) -> Option<EntityId> {
        for item_id in self.entities.inventory[&entity_id].iter() {
            if Some(&item) == self.entities.item.get(item_id) {
                return Some(*item_id);
            }
        }

        return None;
    }

    pub fn item_at_pos(&self, pos: Pos) -> Option<EntityId> {
        for key in self.entities.ids.iter() {
            if self.entities.pos[key] == pos && self.entities.item.get(key).is_some() {
                return Some(*key);
            }
        }

        return None;
    }

    pub fn has_entities(&self, pos: Pos) -> Vec<EntityId> {
        let mut entities = Vec::new();
        for (key, other_pos) in self.entities.pos.iter() {
            if *other_pos == pos {
                entities.push(*key);
            }
        }

        return entities;
    }

    pub fn has_entity(&self, pos: Pos) -> Option<EntityId> {
        for (key, other_pos) in self.entities.pos.iter() {
            if *other_pos == pos {
                return Some(*key);
            }
        }

        return None;
    }

    pub fn has_blocking_entity(&self, pos: Pos) -> Option<EntityId> {
        for (key, other_pos) in self.entities.pos.iter() {
            if *other_pos == pos {
                if self.entities.blocks[key] {
                    return Some(*key);
                }
            }
        }

        return None;
    }

    pub fn has_trap(&self, pos: Pos) -> Option<EntityId> {
        for (key, other_pos) in self.entities.pos.iter() {
            if *other_pos == pos {
                if self.entities.trap.get(key).is_some() && self.entities.armed.get(key).is_some() {
                    return Some(*key);
                }
            }
        }

        return None;
    }

    pub fn is_in_inventory(&self, entity_id: EntityId, item: Item) -> Option<EntityId> {
        for item_key in self.entities.inventory[&entity_id].iter() {
            if self.entities.item[item_key] == item {
                return Some(*item_key);
            }
        }

        return None;
    }

    pub fn using(&self, entity_id: EntityId, item: Item) -> bool {
        if let Some(inventory) = self.entities.inventory.get(&entity_id) {
            if let Some(item_id) = inventory.get(0) {
                return self.entities.item[item_id] == item;
            }
        }

        return false;
    }

    pub fn used_up_item(&mut self, entity_id: EntityId) {
        let mut option_item_id = None;
        if let Some(inventory) = self.entities.inventory.get_mut(&entity_id) {
            if let Some(item_id) = inventory.get(0) {
                option_item_id = Some(*item_id);
            }
        }

        if let Some(item_id) = option_item_id {
            self.remove_entity(item_id);
            self.entities.inventory[&entity_id].remove(0);
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

    pub fn could_see(&mut self, entity_id: EntityId, target_pos: Pos, config: &Config) -> bool {
        let current_facing = self.entities.direction[&entity_id];
        self.entities.face(entity_id, target_pos);

        let visible = self.is_in_fov(entity_id, target_pos, config);

        self.entities.direction[&entity_id] = current_facing;

        return visible;
    }

    pub fn all_within_fov(&mut self, entity_id: EntityId, config: &Config) -> Vec<EntityId> {
        // NOTE(perf) unnecessary allocation
        let mut in_fov: Vec<EntityId> = Vec::new();

        let ids = self.entities.ids.iter().map(|id| *id).collect::<Vec<EntityId>>();
        for other_id in ids {
            if entity_id != other_id {
                let pos = self.entities.pos[&other_id];
                if self.is_in_fov(entity_id, pos, config) {
                    in_fov.push(other_id);
                }
            }
        }

        return in_fov;
    }

    pub fn can_push(&self, entity_id: EntityId, other_id: EntityId) -> bool {
        let entity_type = self.entities.typ[&entity_id];
        let other_type = self.entities.typ[&other_id];

        // the player can't push the enemies
        return !(entity_type == EntityType::Player && other_type == EntityType::Enemy);
    }

    pub fn clear_except(&mut self, exceptions: Vec<EntityId>) {
        let mut dont_clear: Vec<EntityId> = Vec::new();

        for id in exceptions.iter() {
            dont_clear.push(*id);

            if let Some(inventory) = self.entities.inventory.get(id) {
                dont_clear.extend(inventory);
            }
        }

        for id in self.entities.ids.clone().iter() {
            if !dont_clear.contains(&id) {
                self.remove_entity(*id);
            }
        }
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
        self.entities.energy.remove(&id);
        self.entities.armed.remove(&id);
        self.entities.count_down.remove(&id);
        self.entities.move_mode.remove(&id);
        self.entities.direction.remove(&id);
        self.entities.selected_item.remove(&id);
        self.entities.action.remove(&id);
        self.entities.class.remove(&id);
        self.entities.skills.remove(&id);
        self.entities.limbo.remove(&id);
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Skill {
    GrassThrow,
    Blink,
    PassWall,
    Rubble,
    Reform,
    Swap,
    Push,
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
    Selection,
    SkillMenu,
    ClassMenu,
    ConfirmQuit,
}

impl Default for GameState {
    fn default() -> GameState {
        return GameState::Playing;
    }
}

impl GameState {
    pub fn is_menu(self) -> bool {
        return self == GameState::Inventory ||
               self == GameState::SkillMenu ||
               self == GameState::ClassMenu;
    }
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
    Hammer,
    Sword,
    SpikeTrap,
    SoundTrap,
}

impl Item {
    pub fn class(&self) -> ItemClass {
        match self {
            Item::Stone => ItemClass::Secondary,
            Item::Goal => ItemClass::Secondary,
            Item::Dagger => ItemClass::Primary,
            Item::Shield => ItemClass::Primary,
            Item::Hammer => ItemClass::Primary,
            Item::Sword => ItemClass::Primary,
            Item::SpikeTrap => ItemClass::Secondary,
            Item::SoundTrap => ItemClass::Secondary,
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
pub enum EntityName {
    Player,
    Gol,
    Pawn,
    Column,
    Key,
    Sound,
    Exit,
    Dagger,
    Hammer,
    Sword,
    Shield,
    Spire,
    Spike,
    Stone,
    Mouse,
    Other,
}

impl Default for EntityName {
    fn default() -> EntityName {
        return EntityName::Other;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum EntityType {
    Player,
    Enemy,
    Item,
    Column,
    Other,
}

impl Default for EntityType {
    fn default() -> EntityType {
        return EntityType::Other;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum EntityClass {
    General,
    Grass,
    Monolith,
    Clockwork,
}

impl Default for EntityClass {
    fn default() -> EntityClass {
        return EntityClass::General;
    }
}

impl EntityClass {
    pub fn classes() -> Vec<EntityClass> {
        use EntityClass::*;
        return vec!(General, Grass, Monolith, Clockwork);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Message {
    Sound(EntityId, Pos),
    Attack(EntityId),
}

// ensure that each entity has a unique ID, up to 2^64 entities
static OBJECT_ID_COUNT: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Entities {
    pub ids: Vec<EntityId>,
    pub pos: CompStore<Pos>,
    pub chr: CompStore<char>,
    pub name: CompStore<EntityName>,
    pub fighter: CompStore<Fighter>,
    pub ai: CompStore<Ai>,
    pub behavior: CompStore<Behavior>,
    pub item: CompStore<Item>,
    pub movement: CompStore<Reach>,
    pub attack: CompStore<Reach>,
    pub inventory: CompStore<VecDeque<EntityId>>,
    pub trap: CompStore<Trap>,
    pub armed: CompStore<bool>,
    pub energy: CompStore<u32>,
    pub count_down: CompStore<usize>,
    pub move_mode: CompStore<MoveMode>,
    pub direction: CompStore<Direction>,
    pub selected_item: CompStore<EntityId>,
    pub action: CompStore<Action>,
    pub class: CompStore<EntityClass>,
    pub skills: CompStore<Vec<Skill>>,
    pub limbo: CompStore<()>,

    // TODO should end up in animation system instead
    pub animation: CompStore<VecDeque<AnimKey>>,

    // NOTE not sure about keeping these ones, or packaging into larger ones
    pub alive: CompStore<bool>,
    pub sound: CompStore<Pos>, // source position
    pub typ: CompStore<EntityType>,
    pub color: CompStore<Color>,
    pub blocks: CompStore<bool>,
    pub needs_removal: CompStore<bool>,

    // NOTE should be able to remove this
    pub messages: CompStore<Vec<Message>>,
}

impl Entities {
    /* TODO can delete?
    pub fn remove_second(&mut self, entity_id: &EntityId) {
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
        self.energy.remove(entity_id);
        self.armed.remove(entity_id);
        self.count_down.remove(entity_id);
        self.move_mode.remove(entity_id);
        self.direction.remove(entity_id);
        self.selected_item.remove(entity_id);
        self.action.remove(entity_id);
        self.class.remove(entity_id);
        self.skills.remove(entity_id);
        self.animation.remove(entity_id);
        self.alive.remove(entity_id);
        self.sound.remove(entity_id);
        self.typ.remove(entity_id);
        self.color.remove(entity_id);
        self.blocks.remove(entity_id);
        self.needs_removal.remove(entity_id);
        self.messages.remove(entity_id);
    }
    */

    pub fn new() -> Entities {
        return Default::default();
    }

    pub fn clear(&mut self) {
        *self = Default::default();
    }

    pub fn remove_item(&mut self, entity_id: EntityId, item_id: EntityId) {
        let index = self.inventory[&entity_id].iter().position(|id| *id == item_id).unwrap();
        self.inventory[&entity_id].remove(index);
    }

    pub fn create_entity(&mut self, x: i32, y: i32, typ: EntityType, chr: char, color: Color, name: EntityName, blocks: bool) -> EntityId {
        let id = OBJECT_ID_COUNT.fetch_add(1, Ordering::SeqCst);
        self.ids.push(id);

        // add fields that all entities share
        self.pos.insert(id, Pos::new(x, y));
        self.typ.insert(id, typ);
        self.chr.insert(id, chr);
        self.color.insert(id, color);
        self.name.insert(id, name);
        self.blocks.insert(id, blocks);
        self.alive.insert(id, false);
        self.direction.insert(id, Direction::Up);
        self.animation.insert(id,  VecDeque::new());
        self.action.insert(id,  Action::NoAction);
        self.messages.insert(id,  Vec::new());
        self.needs_removal.insert(id,  false);

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

    // NOTE(duplication) could merge with face_to
    pub fn face(&mut self, entity_id: EntityId, face_pos: Pos) {
        let diff = sub_pos(face_pos, self.pos[&entity_id]);
        if let Some(dir) = Direction::from_dxy(diff.x, diff.y) {
            self.direction[&entity_id] = dir;
        }
    }

    pub fn face_to(&mut self, entity_id: EntityId, face_pos: Pos) -> Direction {
        let diff = sub_pos(face_pos, self.pos[&entity_id]);
        if let Some(dir) = Direction::from_dxy(diff.x, diff.y) {
            return dir;
        }
        panic!(format!("Facing with diff ({}, {}) not valid!", diff.x, diff.y));
    }

    pub fn target(&self, entity_id: EntityId) -> Option<Pos> {
        if let Some(Behavior::Investigating(target_pos)) = self.behavior.get(&entity_id) {
            return Some(*target_pos);
        }

        if let Some(Behavior::Attacking(target_id)) = self.behavior.get(&entity_id) {
            let target_pos = self.pos[target_id];
            return Some(target_pos);
        }

        return None;
    }

    pub fn is_dead(&self, entity_id: EntityId) -> bool {
        return !self.ids.contains(&entity_id) || self.limbo.get(&entity_id).is_some();
    }

    /// Set the entity's animation, removing any old animations in play
    pub fn set_animation(&mut self, entity_id: EntityId, key: AnimKey) {
        self.animation[&entity_id].clear();
        self.animation[&entity_id].push_back(key);
    }
}

