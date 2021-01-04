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

    pub fn is_in_fov(&self, entity_id: EntityId, other_pos: Pos, config: &Config) -> bool {
        let pos = self.entities.pos[&entity_id];

        let radius: i32 = match self.entities.typ[&entity_id] {
            EntityType::Enemy => config.fov_radius_monster,
            EntityType::Player => config.fov_radius_player,
            typ => return false, // other things have no FOV
        };

        let stance = self.entities.stance[&entity_id];
        let crouching = stance == Stance::Crouching;

        if self.entities.typ[&entity_id] == EntityType::Player {
            return self.map.is_in_fov(pos, other_pos, radius, crouching);
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
        let player_pushing = entity_type == EntityType::Player && other_type == EntityType::Enemy;
        let enemies_pushing_each_other = entity_type == EntityType::Enemy && other_type == EntityType::Enemy;
        return !(player_pushing || enemies_pushing_each_other);
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
        self.entities.action.remove(&id);
        self.entities.class.remove(&id);
        self.entities.skills.remove(&id);
        self.entities.limbo.remove(&id);
        self.entities.animation.remove(&id);
        self.entities.sound.remove(&id);
        self.entities.typ.remove(&id);
        self.entities.status.remove(&id);
        self.entities.gate_pos.remove(&id);
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
    Blink,
    PassWall,
    Rubble,
    Reform,
    Swap,
    Push,
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
    Selection,
    SkillMenu,
    ClassMenu,
    ConfirmQuit,
    Exit,
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
    Goal,
    Dagger,
    Shield,
    Hammer,
    Sword,
    SpikeTrap,
    SoundTrap,
    BlinkTrap,
    FreezeTrap,
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
            Item::BlinkTrap => ItemClass::Secondary,
            Item::FreezeTrap => ItemClass::Secondary,
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct StatusEffect {
    pub frozen: usize, // turns
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
    pub action: CompStore<Action>,
    pub class: CompStore<EntityClass>,
    pub skills: CompStore<Vec<Skill>>,
    pub limbo: CompStore<()>,
    pub status: CompStore<StatusEffect>,
    pub gate_pos: CompStore<Option<Pos>>,
    pub stance: CompStore<Stance>,

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
        self.action.insert(id,  Action::NoAction);
        self.messages.insert(id,  Vec::new());
        self.needs_removal.insert(id,  false);
        self.status.insert(id,  StatusEffect::default());

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

    pub fn add_skill(&mut self, entity_id: EntityId, skill: Skill) {
        if !self.skills[&entity_id].iter().any(|s| *s == skill) {
            self.skills[&entity_id].push(skill);
        }
    }

    pub fn merge(&mut self, other: &Entities) {
        self.ids.extend(other.ids.iter());
        self.pos.extend(other.pos.iter());
        self.chr.extend(other.chr.iter());
        self.name.extend(other.name.iter());
        self.fighter.extend(other.fighter.iter());
        self.stance.extend(other.stance.iter());
        self.ai.extend(other.ai.iter());
        self.behavior.extend(other.behavior.iter());
        self.item.extend(other.item.iter());
        self.movement.extend(other.movement.iter());
        self.attack.extend(other.attack.iter());
        self.inventory.extend(other.inventory.iter().map(|(k, v)| (*k, v.clone())));
        self.trap.extend(other.trap.iter());
        self.armed.extend(other.armed.iter());
        self.energy.extend(other.energy.iter());
        self.count_down.extend(other.count_down.iter());
        self.move_mode.extend(other.move_mode.iter());
        self.direction.extend(other.direction.iter());
        self.selected_item.extend(other.selected_item.iter());
        self.action.extend(other.action.iter());
        self.class.extend(other.class.iter());
        self.skills.extend(other.skills.iter().map(|(k, v)| (*k, v.clone())));
        self.limbo.extend(other.limbo.iter());
        self.animation.extend(other.animation.iter().map(|(k, v)| (*k, v.clone())));
        self.sound.extend(other.sound.iter());
        self.typ.extend(other.typ.iter());
        self.status.extend(other.status.iter());
        self.gate_pos.extend(other.gate_pos.iter());
        self.color.extend(other.color.iter());
        self.blocks.extend(other.blocks.iter());
        self.needs_removal.extend(other.needs_removal.iter());
        self.messages.extend(other.messages.iter().map(|(k, v)| (*k, v.clone())));
    }
}

