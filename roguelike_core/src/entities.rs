use std::collections::VecDeque;
use std::default::Default;

use serde::{Serialize, Deserialize};

use roguelike_utils::comp::*;
use roguelike_utils::math::*;

use crate::ai::{Ai, Behavior};
use crate::movement::*;
use crate::types::*;

use roguelike_map::*;


#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Entities {
    pub ids: Vec<EntityId>,
    pub next_id: EntityId,
    pub pos: Comp<Pos>,
    pub name: Comp<EntityName>,
    pub hp: Comp<Hp>,
    pub ai: Comp<Ai>,
    pub behavior: Comp<Behavior>,
    pub fov_radius: Comp<i32>,
    pub attack_type: Comp<AttackType>,
    pub item: Comp<Item>,
    pub movement: Comp<Reach>,
    pub attack: Comp<Reach>,
    pub inventory: Comp<VecDeque<EntityId>>,
    pub trap: Comp<Trap>,
    pub armed: Comp<bool>,
    pub energy: Comp<u32>,
    pub stamina: Comp<Stamina>,
    pub count_down: Comp<usize>,
    pub move_mode: Comp<MoveMode>,
    pub direction: Comp<Direction>,
    pub selected_item: Comp<EntityId>,
    pub class: Comp<EntityClass>,
    pub skills: Comp<Vec<Skill>>,
    pub talents: Comp<Vec<Talent>>,
    pub status: Comp<StatusEffect>,
    pub passive: Comp<Passive>,
    pub illuminate: Comp<usize>,
    pub gate_pos: Comp<Pos>,
    pub stance: Comp<Stance>,
    pub took_turn: Comp<bool>,
    pub durability: Comp<usize>,
    pub modifier: Comp<ItemModifier>,
    pub fov_block: Comp<FovBlock>,

    // NOTE not sure about keeping these ones, or packaging into larger ones
    pub sound: Comp<Pos>,
    pub typ: Comp<EntityType>,
    pub blocks: Comp<bool>,
    pub needs_removal: Comp<bool>,

    pub messages: Comp<Vec<Message>>,
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

    pub fn pick_up_item(&mut self, entity_id: EntityId, item_id: EntityId) -> Option<usize> {
        let mut dropped_item = None;

        let item = self.item[&item_id];
        let item_class = item.class();

        match item_class {
            ItemClass::Primary => {
                if let Some(item_index) = self.item_by_class(entity_id, ItemClass::Primary) {
                    // return the last primary item, so it can be dropped
                    dropped_item = Some(item_index);

                    self.inventory[&entity_id].push_back(item_id);
                } else {
                    self.inventory[&entity_id].push_front(item_id);
                }
            }

            ItemClass::Consumable => {
                if let Some(item_index) = self.item_by_class(entity_id, ItemClass::Consumable) {
                    // return the last secondary item, so it can be dropped
                    dropped_item = Some(item_index);

                    self.inventory[&entity_id].push_back(item_id);
                } else {
                    self.inventory[&entity_id].push_front(item_id);
                }
            }

            ItemClass::Misc => {
                self.inventory[&entity_id].push_back(item_id);
            }
        }

        self.set_xy(item_id, -1, -1);

        return dropped_item;
    }

    pub fn item_by_type(&self, entity_id: EntityId, item: Item) -> Option<usize> {
        for ix in 0..self.inventory[&entity_id].len() {
            let item_id = self.inventory[&entity_id][ix];
            if self.item[&item_id] == item {
                return Some(ix);
            }
        }

        return None;
    }

    pub fn item_by_class(&self, entity_id: EntityId, item_class: ItemClass) -> Option<usize> {
        for ix in 0..self.inventory[&entity_id].len() {
            let item_id = self.inventory[&entity_id][ix];
            if self.item[&item_id].class() == item_class {
                return Some(ix);
            }
        }

        return None;
    }

    pub fn summarize_entity(&mut self, id: EntityId) -> String {
        return format!("Entity {:?}: {} at {}", self.name[&id], self.typ[&id], self.pos[&id]);
    }

    pub fn create_entity(&mut self, x: i32, y: i32, typ: EntityType, name: EntityName, blocks: bool) -> EntityId {
        let id = self.next_id;
        self.next_id += 1;
        self.ids.push(id);

        // add fields that all entities share
        self.pos.insert(id, Pos::new(x, y));
        self.typ.insert(id, typ);
        self.name.insert(id, name);
        self.blocks.insert(id, blocks);
        self.direction.insert(id, Direction::Up);
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

    pub fn has_enough_stamina(&mut self, entity_id: EntityId, amount: u32) -> bool {
        if self.status[&entity_id].test_mode || self.stamina.get(&entity_id).is_none() {
            return true;
        }

        return self.stamina[&entity_id].amount >= amount;
    }

    pub fn use_energy(&mut self, entity_id: EntityId) {
        if !self.status[&entity_id].test_mode {
            self.energy[&entity_id] -= 1;
        }
    }

    pub fn take_damage(&mut self, entity_id: EntityId, damage: i32) -> bool {
        let mut was_hit = false;

        if damage > 0 && self.status[&entity_id].alive && self.status[&entity_id].stone == 0 {
            if let Some(hp) = self.hp.get_mut(&entity_id) {
                if !self.status[&entity_id].test_mode {
                    hp.hp -= damage;

                    if hp.hp <= 0 {
                        self.status[&entity_id].alive = false;
                        self.blocks[&entity_id] = false;
                    }

                    was_hit = true;
                }
            }
        }

        return was_hit;
    }

    pub fn triggered_traps(&self, pos: Pos) -> Vec<EntityId> {
        let mut traps: Vec<EntityId> = Vec::new();
        for key in self.ids.iter() {
            if self.trap.get(key).is_some()       && // key is a trap
               self.armed.get(key) == Some(&true) && // trap is armed
               !self.needs_removal[key]           && // not being removed
               self.pos[key] == pos {
                traps.push(*key);
            }
        }

        return traps;
    }

    pub fn target_disappeared(&mut self, entity_id: EntityId) -> Option<Message> {
        if let Some(index) = self.messages[&entity_id].iter().position(|msg| matches!(msg, Message::Disappeared(_))) {
            return Some(self.messages[&entity_id].remove(index));
        } else {
            return None;
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
        if let Some(index) = self.messages[&entity_id].iter().position(|msg| matches!(msg, Message::Sound(_))) {
            return Some(self.messages[&entity_id].remove(index));
        } else {
            return None;
        }
    }

    pub fn was_hit(&mut self, entity_id: EntityId) -> Option<Message> {
        if let Some(index) = self.messages[&entity_id].iter().position(|msg| matches!(msg, Message::Hit(_))) {
            return Some(self.messages[&entity_id].remove(index));
        } else {
            return None;
        }
    }

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
        dbg!(diff);
        panic!("Facing with diff not valid!");
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
        return !self.ids.contains(&entity_id) || self.needs_removal[&entity_id] ||
            matches!(self.hp.get(&entity_id), Some(Hp { hp: 0, .. } ));
    }

    pub fn active_ais(&self) -> Vec<EntityId> {
        let mut ai_ids = Vec::new();
        // get entity ids for any active AI entity
        for key in self.ids.iter() {
            if self.ai.get(key).is_some()    &&
               self.status[key].alive        &&
               !self.needs_removal[key] &&
               self.hp.get(key).is_some() {
               ai_ids.push(*key);
           }
        }
        return ai_ids;
    }

    pub fn get_names_at_pos(&mut self, check_pos: Pos, name: EntityName) -> Vec<EntityId> {
        let mut entity_ids: Vec<EntityId> = Vec::new();

        for key in self.ids.iter() {
            let pos = self.pos[key];
            if pos == check_pos && name == self.name[key] {
                entity_ids.push(*key);
            }
        }

        return entity_ids;
    }

    pub fn add_skill(&mut self, entity_id: EntityId, skill: Skill) {
        if !self.skills[&entity_id].iter().any(|s| *s == skill) {
            self.skills[&entity_id].push(skill);
        }
    }

    pub fn mark_for_removal(&mut self, entity_id: EntityId) {
        // Removing the player is handled specially.
        if !matches!(self.typ.get(&entity_id), Some(EntityType::Player)) {
            self.needs_removal[&entity_id] = true;
        }
    }

    pub fn count_down(&mut self) {
        let mut to_remove = Vec::new();
        for entity_id in self.ids.iter() {
            if let Some(ref mut count) = self.count_down.get_mut(entity_id) {
                if **count == 0 {
                    to_remove.push(*entity_id);
                } else {
                    **count -= 1;
                }
            }
        }

        // remove entities waiting removal
        for key in to_remove {
            self.mark_for_removal(key);
        }
    }

    pub fn remove_from_inventory(&mut self, entity_id: EntityId, item_id: EntityId) {
        let mut index = 0;
        let mut found = false;
        while index < self.inventory[&entity_id].len() {
            if self.inventory[&entity_id][index] == item_id {
                found = true;
                break;
            }
            index += 1;
        }

        if found {
            self.inventory[&entity_id].remove(index);
        }
    }

    pub fn remove_entity(&mut self, id: EntityId) {
        let ix_pos = self.ids.iter().position(|val| *val == id).unwrap();
        self.ids.remove(ix_pos);

        self.pos.remove(&id);
        self.name.remove(&id);
        self.hp.remove(&id);
        self.stance.remove(&id);
        self.ai.remove(&id);
        self.behavior.remove(&id);
        self.fov_radius.remove(&id);
        self.fov_block.remove(&id);
        self.attack_type.remove(&id);
        self.item.remove(&id);
        self.movement.remove(&id);
        self.attack.remove(&id);
        self.inventory.remove(&id);
        self.trap.remove(&id);
        self.armed.remove(&id);
        self.energy.remove(&id);
        self.count_down.remove(&id);
        self.move_mode.remove(&id);
        self.direction.remove(&id);
        self.selected_item.remove(&id);
        self.class.remove(&id);
        self.skills.remove(&id);
        self.sound.remove(&id);
        self.typ.remove(&id);
        self.status.remove(&id);
        self.illuminate.remove(&id);
        self.gate_pos.remove(&id);
        self.took_turn.remove(&id);
        self.durability.remove(&id);
        self.blocks.remove(&id);
        self.needs_removal.remove(&id);
        self.messages.remove(&id);
        self.modifier.remove(&id);
        self.passive.remove(&id);
        self.stamina.remove(&id);
    }
}

