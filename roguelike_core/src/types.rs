use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::default::Default;
use std::fmt;
use std::str::FromStr;

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
use crate::line::*;


pub type Name = Symbol;

pub type EntityId = u64;

pub type CompStore<T> = IndexMap<EntityId, T>;

pub type Pos = Point2D<i32, ()>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

    pub fn empty(width: u32, height: u32) -> GameData {
        return GameData::new(Map::from_dims(width, height), Entities::new());
    }

    pub fn get_clear_pos(&self) -> Vec<Pos> {
        return self.map.get_empty_pos()
                       .iter()
                       .map(|pos| *pos)
                       .filter(|pos| self.has_blocking_entity(*pos).is_none())
                       .collect::<Vec<Pos>>();
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
                      let mut next_positions = Vec::with_capacity(10);

                      for direction in Direction::move_actions() {
                          for offset in reach.move_with_reach(&direction) {
                              let next_pos = add_pos(pos, offset);

                              let mut can_move = false;
                              let clear = self.clear_path(pos, next_pos, traps_block);
                              can_move |= clear;

                              if !can_move {
                                  if !must_reach && next_pos == end {
                                      let not_blocked = self.map.path_blocked_move(pos, next_pos).is_none();
                                      can_move |= not_blocked;
                                  }
                              }

                              if can_move {
                                 let mut cost = 1;
                                  if let Some(cost_fun) = cost_fun {
                                      // very small amount of time
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

    pub fn is_in_fov(&self, entity_id: EntityId, other_id: EntityId, config: &Config) -> bool {
        let stance = self.entities.stance[&entity_id];
        let other_stance = self.entities.stance.get(&other_id).unwrap_or(&Stance::Standing);
        let crouching = stance == Stance::Crouching || other_stance == &Stance::Crouching;

        let other_pos = self.entities.pos[&other_id];

        return self.fov_check(entity_id, other_pos, crouching, config);
    }

    pub fn pos_in_fov(&self, entity_id: EntityId, other_pos: Pos, config: &Config) -> bool {
        let stance = self.entities.stance[&entity_id];
        let crouching = stance == Stance::Crouching;

        return self.fov_check(entity_id, other_pos, crouching, config);
    }

    fn fov_check(&self, entity_id: EntityId, other_pos: Pos, crouching: bool, config: &Config) -> bool {
        let pos = self.entities.pos[&entity_id];

        let radius: i32 = match self.entities.typ[&entity_id] {
            EntityType::Enemy => config.fov_radius_monster,
            EntityType::Player => config.fov_radius_player,
            _ => return false, // other things have no FOV
        };

        if self.entities.typ[&entity_id] == EntityType::Player {
            let mut can_see = self.map.is_in_fov(pos, other_pos, radius, crouching);

            for id in self.entities.ids.iter() {
                if can_see {
                    break;
                }

                if self.entities.status[id].illuminate != 0 && self.entities.pos[id].x >= 0 {
                    let illuminate_pos = self.entities.pos[id];
                    let illuminate_radius = self.entities.status[id].illuminate as i32;
                    can_see |= self.map.is_in_fov(illuminate_pos, other_pos, illuminate_radius, crouching);
                }
            }

            return can_see;
        } else {
            if let Some(dir) = self.entities.direction.get(&entity_id) {
                return self.map.is_in_fov_direction(pos, other_pos, radius, *dir, crouching);
            } else {
                panic!(format!("tried to perform is_in_fov on entity without facing"));
            }
        }
    }

    pub fn find_by_name(&self, name: EntityName) -> Option<EntityId> {
        for (key, nam) in self.entities.name.iter() {
            if *nam == name {
                return Some(*key);
            }
        }

        return None;
    }

    pub fn get_entities_at_pos(&mut self, check_pos: Pos) -> Vec<EntityId> {
        let mut object_ids: Vec<EntityId> = Vec::new();

        for key in self.entities.ids.iter() {
            let pos = self.entities.pos[key];
            let is_mouse = self.entities.name[key] == EntityName::Mouse;
            let removing = self.entities.needs_removal[key];

            if !removing && !is_mouse && check_pos == pos {
                object_ids.push(*key);
            }
        }

        return object_ids;
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

        return !path_blocked && self.map.path_blocked_move(start, end).is_none();
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
            let is_disarmed_trap =
                self.entities.trap.get(key).is_some() &&
                self.entities.armed.get(key) == Some(&false);
            let is_item = self.entities.item.get(key).is_some();

            if self.entities.pos[key] == pos && (is_item || is_disarmed_trap) {
                return Some(*key);
            }
        }

        return None;
    }

    pub fn count_down(&mut self) {
        let mut to_remove = Vec::new();
        for entity_id in self.entities.ids.iter() {
            if let Some(ref mut count) = self.entities.count_down.get_mut(entity_id) {
                if **count == 0 {
                    to_remove.push(*entity_id);
                } else {
                    **count -= 1;
                }
            }

            //if self.entities.needs_removal[entity_id] &&
            //   self.entities.animation[entity_id].len() == 0 {
            //    to_remove.push(*entity_id);
            //}
        }

        // remove objects waiting removal
        for key in to_remove {
            self.remove_entity(key);
        }
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

    // check whether the entity could see a location if it were facing towards that position.
    pub fn could_see(&mut self, entity_id: EntityId, target_pos: Pos, config: &Config) -> bool {
        let current_facing = self.entities.direction[&entity_id];
        self.entities.face(entity_id, target_pos);

        let visible = self.pos_in_fov(entity_id, target_pos, config);

        self.entities.direction[&entity_id] = current_facing;

        return visible;
    }

    pub fn can_push(&self, entity_id: EntityId, other_id: EntityId) -> bool {
        let entity_type = self.entities.typ[&entity_id];
        let other_type = self.entities.typ[&other_id];

        // the player can't push the enemies
        let player_pushing = entity_type == EntityType::Player && other_type == EntityType::Enemy;
        let enemies_pushing_each_other = entity_type == EntityType::Enemy && other_type == EntityType::Enemy;
        return !(player_pushing || enemies_pushing_each_other);
    }

    // clear all entities, except those in the given vector.
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
        self.entities.stance.remove(&id);
        self.entities.ai.remove(&id);
        self.entities.behavior.remove(&id);
        self.entities.item.remove(&id);
        self.entities.movement.remove(&id);
        self.entities.attack.remove(&id);
        self.entities.inventory.remove(&id);
        self.entities.trap.remove(&id);
        self.entities.armed.remove(&id);
        self.entities.energy.remove(&id);
        self.entities.count_down.remove(&id);
        self.entities.move_mode.remove(&id);
        self.entities.direction.remove(&id);
        self.entities.selected_item.remove(&id);
        self.entities.class.remove(&id);
        self.entities.skills.remove(&id);
        self.entities.limbo.remove(&id);
        self.entities.animation.remove(&id);
        self.entities.sound.remove(&id);
        self.entities.typ.remove(&id);
        self.entities.status.remove(&id);
        self.entities.gate_pos.remove(&id);
        self.entities.took_turn.remove(&id);
        self.entities.color.remove(&id);
        self.entities.blocks.remove(&id);
        self.entities.needs_removal.remove(&id);
        self.entities.messages.remove(&id);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Trap {
    Spikes,
    Sound,
    Blink,
    Freeze,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Skill {
    GrassThrow,
    GrassBlade,
    GrassShoes,
    Blink,
    PassWall,
    Rubble,
    Reform,
    Swap,
    Push,
    Illuminate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize, Default)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum GameState {
    Playing,
    Win,
    Lose,
    Inventory,
    SkillMenu,
    ClassMenu,
    ConfirmQuit,
    Exit,
}

impl fmt::Display for GameState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GameState::Playing => write!(f, "playing"),
            GameState::Win => write!(f, "win"),
            GameState::Lose => write!(f, "lose"),
            GameState::Inventory => write!(f, "inventory"),
            GameState::SkillMenu => write!(f, "skillmenu"),
            GameState::ClassMenu => write!(f, "classmenu"),
            GameState::ConfirmQuit => write!(f, "confirmquit"),
            GameState::Exit => write!(f, "exit"),
        }
    }
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


#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ActionMode {
    Primary,
    Alternate,
}

impl fmt::Display for ActionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionMode::Primary => write!(f, "primary"),
            ActionMode::Alternate => write!(f, "alternate"),
        }
    }
}

impl FromStr for ActionMode {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();
        if s == "primary" {
            return Ok(ActionMode::Primary);
        } else if s == "alternate" {
            return Ok(ActionMode::Alternate);
        }

        return Err(format!("Could not parse '{}' as ActionMode", s));
    }
}


#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Item {
    Stone,
    Key,
    Dagger,
    Shield,
    Hammer,
    Sword,
    Lantern,
    SpikeTrap,
    SoundTrap,
    BlinkTrap,
    FreezeTrap,
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Item::Stone => write!(f, "stone"),
            Item::Key => write!(f, "key"),
            Item::Dagger => write!(f, "dagger"),
            Item::Shield => write!(f, "shield"),
            Item::Hammer => write!(f, "hammer"),
            Item::Sword => write!(f, "sword"),
            Item::Lantern => write!(f, "lantern"),
            Item::SpikeTrap => write!(f, "spiketrap"),
            Item::SoundTrap => write!(f, "soundtrap"),
            Item::BlinkTrap => write!(f, "blinktrap"),
            Item::FreezeTrap => write!(f, "freezetrap"),
        }
    }
}

impl FromStr for Item {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();
        if s == "stone" {
            return Ok(Item::Stone);
        } else if s == "key" {
            return Ok(Item::Key);
        } else if s == "dagger" {
            return Ok(Item::Dagger);
        } else if s == "shield" {
            return Ok(Item::Shield);
        } else if s == "hammer" {
            return Ok(Item::Hammer);
        } else if s == "sword" {
            return Ok(Item::Sword);
        } else if s == "spiketrap" {
            return Ok(Item::SpikeTrap);
        } else if s == "soundtrap" {
            return Ok(Item::SoundTrap);
        } else if s == "blinktrap" {
            return Ok(Item::BlinkTrap);
        } else if s == "freezetrap" {
            return Ok(Item::FreezeTrap);
        }

        return Err(format!("Could not parse '{}' as Item", s));
    }
}

impl Item {
    pub fn class(&self) -> ItemClass {
        match self {
            Item::Stone => ItemClass::Secondary,
            Item::Key => ItemClass::Secondary,
            Item::Dagger => ItemClass::Primary,
            Item::Shield => ItemClass::Primary,
            Item::Hammer => ItemClass::Primary,
            Item::Sword => ItemClass::Primary,
            Item::Lantern => ItemClass::Secondary,
            Item::SpikeTrap => ItemClass::Secondary,
            Item::SoundTrap => ItemClass::Secondary,
            Item::BlinkTrap => ItemClass::Secondary,
            Item::FreezeTrap => ItemClass::Secondary,
        }
    }

    pub fn name(&self) -> EntityName {
        match self {
            Item::Stone => EntityName::Stone,
            Item::Key => EntityName::Key,
            Item::Dagger => EntityName::Dagger,
            Item::Shield => EntityName::Shield,
            Item::Hammer => EntityName::Hammer,
            Item::Sword => EntityName::Sword,
            Item::Lantern => EntityName::Lantern,
            Item::SpikeTrap => EntityName::SpikeTrap,
            Item::SoundTrap => EntityName::SoundTrap,
            Item::BlinkTrap => EntityName::BlinkTrap,
            Item::FreezeTrap => EntityName::FreezeTrap,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum InventoryAction {
    Use,
    Drop,
}

impl Default for InventoryAction {
    fn default() -> InventoryAction {
        return InventoryAction::Drop;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum Stance {
    Crouching,
    Standing,
    Running,
}

impl Stance {
    pub fn waited(&self, move_mode: MoveMode) -> Stance {
        match (self, move_mode) {
            (Stance::Crouching, MoveMode::Run) => Stance::Standing,
            (Stance::Standing, MoveMode::Sneak) => Stance::Crouching,
            (Stance::Running, _) => Stance::Standing,
            _ => *self,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum EntityName {
    Player,
    Gol,
    Pawn,
    Column,
    Key,
    Exit,
    Dagger,
    Hammer,
    Sword,
    Shield,
    Lantern,
    Spire,
    SpikeTrap,
    BlinkTrap,
    FreezeTrap,
    SoundTrap,
    GateTrigger,
    Stone,
    Mouse,
    Cursor,
    Energy,
    Other,
}

impl Default for EntityName {
    fn default() -> EntityName {
        return EntityName::Other;
    }
}

impl fmt::Display for EntityName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntityName::Player => write!(f, "player"),
            EntityName::Gol => write!(f, "gol"),
            EntityName::Pawn => write!(f, "pawn"),
            EntityName::Column => write!(f, "column"),
            EntityName::Key => write!(f, "key"),
            EntityName::Exit => write!(f, "exit"),
            EntityName::Dagger => write!(f, "dagger"),
            EntityName::Hammer => write!(f, "hammer"),
            EntityName::Sword => write!(f, "sword"),
            EntityName::Lantern => write!(f, "lantern"),
            EntityName::Shield => write!(f, "shield"),
            EntityName::Spire => write!(f, "spire"),
            EntityName::SpikeTrap => write!(f, "spiketrap"),
            EntityName::BlinkTrap => write!(f, "blinktrap"),
            EntityName::FreezeTrap => write!(f, "freezetrap"),
            EntityName::SoundTrap => write!(f, "soundtrap"),
            EntityName::GateTrigger => write!(f, "gatetrigger"),
            EntityName::Stone => write!(f, "stone"),
            EntityName::Mouse => write!(f, "mouse"),
            EntityName::Cursor => write!(f, "cursor"),
            EntityName::Energy => write!(f, "energy"),
            EntityName::Other => write!(f, "other"),
        }
    }
}

impl FromStr for EntityName {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();

        if s == "player" {
            return Ok(EntityName::Player);
        } else if s == "gol" {
            return Ok(EntityName::Gol);
        } else if s == "pawn" {
            return Ok(EntityName::Pawn);
        } else if s == "column" {
            return Ok(EntityName::Column);
        } else if s == "key" {
            return Ok(EntityName::Key);
        } else if s == "exit" {
            return Ok(EntityName::Exit);
        } else if s == "dagger" {
            return Ok(EntityName::Dagger);
        } else if s == "hammer" {
            return Ok(EntityName::Hammer);
        } else if s == "sword" {
            return Ok(EntityName::Sword);
        } else if s == "lantern" {
            return Ok(EntityName::Lantern);
        } else if s == "shield" {
            return Ok(EntityName::Shield);
        } else if s == "spire" {
            return Ok(EntityName::Spire);
        } else if s == "spiketrap" {
            return Ok(EntityName::SpikeTrap);
        } else if s == "blinktrap" {
            return Ok(EntityName::BlinkTrap);
        } else if s == "freezetrap" {
            return Ok(EntityName::FreezeTrap);
        } else if s == "soundtrap" {
            return Ok(EntityName::SoundTrap);
        } else if s == "gatetrigger" {
            return Ok(EntityName::GateTrigger);
        } else if s == "stone" {
            return Ok(EntityName::Stone);
        } else if s == "mouse" {
            return Ok(EntityName::Mouse);
        } else if s == "cursor" {
            return Ok(EntityName::Cursor);
        } else if s == "energy" {
            return Ok(EntityName::Energy);
        } else if s == "other" {
            return Ok(EntityName::Other);
        }

        panic!(format!("EntityName {} not expected!", s));
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum EntityType {
    Player,
    Enemy,
    Item,
    Column,
    Energy,
    Trigger,
    Other,
}

impl fmt::Display for EntityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntityType::Player => write!(f, "player"),
            EntityType::Enemy => write!(f, "enemy"),
            EntityType::Item => write!(f, "item"),
            EntityType::Column => write!(f, "column"),
            EntityType::Energy => write!(f, "energy"),
            EntityType::Trigger => write!(f, "trigger"),
            EntityType::Other => write!(f, "other"),
        }
    }
}

impl FromStr for EntityType {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();

        if s == "player" {
            return Ok(EntityType::Player);
        } else if s == "enemy" {
            return Ok(EntityType::Enemy);
        } else if s == "item" {
            return Ok(EntityType::Item);
        } else if s == "column" {
            return Ok(EntityType::Column);
        } else if s == "energy" {
            return Ok(EntityType::Energy);
        } else if s == "trigger" {
            return Ok(EntityType::Trigger);
        } else if s == "other" {
            return Ok(EntityType::Other);
        }

        panic!(format!("EntityType {} not expected!", s));
    }
}

impl Default for EntityType {
    fn default() -> EntityType {
        return EntityType::Other;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum EntityClass {
    General,
    Grass,
    Monolith,
    Clockwork,
    Hierophant,
}

impl fmt::Display for EntityClass {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EntityClass::General => write!(f, "general"),
            EntityClass::Grass => write!(f, "grass"),
            EntityClass::Monolith => write!(f, "monolith"),
            EntityClass::Clockwork => write!(f, "clockword"),
            EntityClass::Hierophant => write!(f, "hierophant"),
        }
    }
}

impl Default for EntityClass {
    fn default() -> EntityClass {
        return EntityClass::General;
    }
}

impl EntityClass {
    pub fn classes() -> Vec<EntityClass> {
        use EntityClass::*;
        return vec!(General, Grass, Monolith, Clockwork, Hierophant);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct StatusEffect {
    pub frozen: usize, // turns
    pub soft_steps: usize, // turns
    pub illuminate: usize, // radius
    pub active: bool,
    pub alive: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Message {
    Sound(EntityId, Pos),
    Attack(EntityId),
}

// ensure that each entity has a unique ID, up to 2^64 entities
static OBJECT_ID_COUNT: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
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
    pub class: CompStore<EntityClass>,
    pub skills: CompStore<Vec<Skill>>,
    pub limbo: CompStore<()>,
    pub status: CompStore<StatusEffect>,
    pub gate_pos: CompStore<Option<Pos>>,
    pub stance: CompStore<Stance>,
    pub took_turn: CompStore<bool>,

    // TODO should end up in animation system instead
    pub animation: CompStore<VecDeque<AnimKey>>,

    // NOTE not sure about keeping these ones, or packaging into larger ones
    pub sound: CompStore<Pos>, // source position
    pub typ: CompStore<EntityType>,
    pub color: CompStore<Color>,
    pub blocks: CompStore<bool>,
    pub needs_removal: CompStore<bool>,

    // NOTE should be able to remove this
    pub messages: CompStore<Vec<Message>>,
}

impl Entities {
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

    pub fn pick_up_item(&mut self, entity_id: EntityId, item_id: EntityId) {
        let item = self.item[&item_id];
        let item_class = item.class();

        match item_class {
            ItemClass::Primary => {
                if item_primary_at(entity_id, self, 0) &&
                   item_primary_at(entity_id, self, 1) {
                    self.inventory[&entity_id][0] = item_id;

                    let obj_pos = self.pos[&entity_id];
                    self.set_pos(entity_id, obj_pos);
                } else {
                    self.inventory[&entity_id].push_front(item_id);
                }
            }

            ItemClass::Secondary => {
                self.inventory[&entity_id].push_back(item_id);
            }
        }

        self.set_xy(item_id, -1, -1);
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
        self.direction.insert(id, Direction::Up);
        self.animation.insert(id,  VecDeque::new());
        self.messages.insert(id,  Vec::new());
        self.needs_removal.insert(id,  false);
        self.status.insert(id,  StatusEffect::default());
        self.took_turn.insert(id,  false);

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
                self.status[&entity].alive = false;
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

    pub fn active_ais(&self) -> Vec<EntityId> {
        let mut ai_ids = Vec::new();
        // get entity ids for any active AI entity
        for key in self.ids.iter() {
            if self.ai.get(key).is_some()    &&
               self.status[key].alive        &&
               self.limbo.get(key).is_none() &&
               self.fighter.get(key).is_some() {
               ai_ids.push(*key);
           }
        }
        return ai_ids;
    }

    pub fn add_skill(&mut self, entity_id: EntityId, skill: Skill) {
        if !self.skills[&entity_id].iter().any(|s| *s == skill) {
            self.skills[&entity_id].push(skill);
        }
    }

    // NOTE cloning entities may not remap all entity ids that an entity tracks!
    // this could cause subtle problems, so this is really only for level generation.
    pub fn clone_entity(&mut self, other: &Entities, entity_id: EntityId) {
        let new_id = OBJECT_ID_COUNT.fetch_add(1, Ordering::SeqCst);

        self.ids.push(new_id);

        // this macro simply clones a component and inserts into the self
        // entity map with the new id.
        macro_rules! move_component {
            ($comp_name:ident) => {
                if let Some($comp_name) = other.$comp_name.get(&entity_id) {
                    self.$comp_name.insert(new_id, $comp_name.clone());
                }
            }
        }

        move_component!(pos);
        move_component!(chr);
        move_component!(name);
        move_component!(fighter);
        move_component!(stance);
        move_component!(ai);
        move_component!(behavior);
        move_component!(item);
        move_component!(movement);
        move_component!(attack);
        move_component!(trap);
        move_component!(energy);
        move_component!(count_down);
        move_component!(move_mode);
        move_component!(direction);
        move_component!(selected_item);
        move_component!(class);
        move_component!(skills);
        move_component!(limbo);
        move_component!(animation);
        move_component!(sound);
        move_component!(typ);
        move_component!(status);
        move_component!(gate_pos);
        move_component!(took_turn);
        move_component!(color);
        move_component!(blocks);
        move_component!(needs_removal);
        move_component!(messages);

        // NOTE this might not work if entity IDs are left!
        if let Some(inventory) = other.inventory.get(&entity_id) { 
            assert_eq!(0, inventory.len());
        }
    }

    pub fn merge(&mut self, other: &Entities) {
        for id in other.ids.iter() {
            self.clone_entity(other, *id);
        }
    }
}

