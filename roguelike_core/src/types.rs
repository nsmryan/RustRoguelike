use std::collections::VecDeque;
use std::default::Default;
use std::fmt;
use std::str::FromStr;

use serde::{Serialize, Deserialize};

use pathfinding::directed::astar::astar;

use euclid::Point2D;

use roguelike_utils::line::*;
use roguelike_utils::comp::*;

use crate::ai::{Ai, Behavior};
use crate::map::*;
use crate::movement::*;
use crate::utils::*;
use crate::config::Config;
use crate::constants::*;


pub type Pos = Point2D<i32, ()>;


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Level {
    pub map: Map,
    pub entities: Entities,
}

impl Level {
    pub fn new(map: Map, entities: Entities) -> Level {
        Level {
            map,
            entities,
        }
    }

    pub fn empty(width: u32, height: u32) -> Level {
        return Level::new(Map::from_dims(width, height), Entities::new());
    }

    pub fn pos_blocked(&self, pos: Pos) -> bool {
        return self.has_blocking_entity(pos).is_some() || self.map[pos].block_move;
    }

    pub fn get_clear_pos(&self) -> Vec<Pos> {
        return self.map.get_empty_pos()
                       .iter()
                       .map(|pos| *pos)
                       .filter(|pos| self.has_blocking_entity(*pos).is_none())
                       .filter(|pos| !self.map[*pos].block_move)
                       .collect::<Vec<Pos>>();
    }

    // The item index is usually determined by the ItemClass, but for Misc it can
    // only be a stone.
    pub fn find_item(&self, item_class: ItemClass) -> Option<usize> {
            let player_id = self.find_by_name(EntityName::Player).unwrap();
            let maybe_index;
            if item_class == ItemClass::Misc {
                maybe_index = self.entities.item_by_type(player_id, Item::Stone);
            } else {
                maybe_index = self.entities.item_by_class(player_id, item_class);
            }
            return maybe_index;
    }

    pub fn path_between(&self,
                        start: Pos,
                        end: Pos,
                        reach: Reach,
                        must_reach: bool,
                        traps_block: bool,
                        cost_fun: Option<fn(Pos, Pos, Pos, &Level) -> Option<i32>>) -> Vec<Pos> {
        let result;

        let maybe_results =
            astar(&start,
                  |&pos| {
                      // NOTE(perf) this allocation could be avoided with an Iterable
                      let mut next_positions = Vec::with_capacity(10);

                      for direction in &Direction::move_actions() {
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

    pub fn fov_radius(&self, entity_id: EntityId) -> i32 {
        if self.entities.fov_radius.get(&entity_id).is_none() {
            dbg!(entity_id, self.entities.name[&entity_id], self.entities.typ[&entity_id]);
        }
        let mut radius: i32 = self.entities.fov_radius[&entity_id];

        if let Some(status) = self.entities.status.get(&entity_id) {
            radius += status.extra_fov as i32;
        }

        return radius;
    }

    pub fn is_in_fov(&self, entity_id: EntityId, other_id: EntityId) -> FovResult {
        let stance = self.entities.stance[&entity_id];
        let other_stance = self.entities.stance.get(&other_id).unwrap_or(&Stance::Standing);
        let crouching = stance == Stance::Crouching || other_stance == &Stance::Crouching;

        let other_pos = self.entities.pos[&other_id];

        if self.entities.needs_removal[&other_id] {
            return FovResult::Outside;
        } else {
            return self.fov_check(entity_id, other_pos, crouching);
        }
    }

    pub fn pos_in_fov_edge(&self, entity_id: EntityId, other_pos: Pos) -> FovResult {
        let stance = self.entities.stance[&entity_id];
        let crouching = stance == Stance::Crouching;

        return self.fov_check(entity_id, other_pos, crouching);
    }

    pub fn pos_in_fov(&self, entity_id: EntityId, other_pos: Pos) -> bool {
        let fov_result = self.pos_in_fov_edge(entity_id, other_pos);
        return fov_result == FovResult::Inside;
    }

    fn fov_check_player(&self, entity_id: EntityId, check_pos: Pos, crouching: bool, view_distance: i32) -> FovResult {
        let entity_pos = self.entities.pos[&entity_id];

        let fov_result = self.map.is_in_fov_edge(entity_pos, check_pos, view_distance, crouching);

        // If we can't see the tile, check for a latern that illuminates it, allowing
        // us to see it anyway. Ignore tiles that are blocked for sight anyway.
        //if fov_result != FovResult::Inside && !self.map[check_pos].block_sight {
            //fov_result = self.check_illumination(entity_id, fov_result, check_pos, ILLUMINATE_FOV_RADIUS, crouching);
        //}

        return fov_result;
    }

    fn fov_magnification(&self, entity_id: EntityId, check_pos: Pos, crouching: bool) -> i32 {
        let entity_pos = self.entities.pos[&entity_id];
        let mut magnification: i32 = 0;

        for (fov_block_id, fov_block) in self.entities.fov_block.iter() {
            for to_pos in line(entity_pos, check_pos) {
                // fov_check_result is an Option to avoid computing this fov_check unless there
                // is actually a magnifier in line with the entity's FoV.
                let mut fov_check_result = None;

                for from_pos in line(check_pos, entity_pos) {
                    // If the lines overlap, check for magnifiers
                    if to_pos == from_pos {
                        if self.entities.pos[&fov_block_id] == to_pos {
                            if let FovBlock::Magnify(amount) = fov_block {
                                if fov_check_result.is_none() {
                                    fov_check_result = Some(self.fov_check(entity_id, to_pos, crouching) == FovResult::Inside);
                                }

                                if let Some(true) = fov_check_result {
                                    magnification += *amount as i32;
                                }
                            }
                        }
                    }
                }
            }
        }

        return magnification;
    }
    
    fn fov_reduction(&self, entity_id: EntityId, check_pos: Pos, view_distance: i32) -> i32 {
        let mut reduction = 0;

        let entity_pos = self.entities.pos[&entity_id];

        // Search along a line from the entity, to the given position,
        // and search back from the given position to the entity, looking
        // for matching positions.
        for to_pos in line(entity_pos, check_pos) {
            for from_pos in line(check_pos, entity_pos) {
                // If the lines overlap, check for FoV modifying entities.
                if to_pos == from_pos {
                    for (entity_id, fov_block) in self.entities.fov_block.iter() {
                        if self.entities.pos[&entity_id] == to_pos {
                            match fov_block {
                                FovBlock::Block => {
                                    // Blocking entities completly block LoS
                                    return 0;
                                }

                                FovBlock::Transparent => {
                                    // Transparent FovBlockers have no effect.
                                }

                                FovBlock::Opaque(amount) => {
                                    // If an entity makes the tile completely
                                    // outside of the FoV, we can just return
                                    // immediately.
                                    if *amount as i32 + reduction > view_distance {
                                        return view_distance;
                                    }
                                    reduction += *amount as i32;
                                }

                                FovBlock::Magnify(_) => {
                                    // magnification is handled before FoV above.
                                }
                            }
                        }
                    }
                }
            }
        }

        return reduction;
    }

    fn fov_check(&self, entity_id: EntityId, check_pos: Pos, crouching: bool) -> FovResult {
        if check_pos.x < 0 || check_pos.y < 0 {
            return FovResult::Outside;
        }

        let entity_pos = self.entities.pos[&entity_id];
        let mut view_distance: i32 = self.fov_radius(entity_id);

        // Add in the result of magnification effects.
        view_distance += self.fov_magnification(entity_id, check_pos, crouching);

        let mut fov_result;

        // The player and the other entities have slightly different FoV checks.
        // The other entities have directional FoV which is layered on the base FoV algorithm.
        if self.entities.typ[&entity_id] == EntityType::Player {
            fov_result = self.fov_check_player(entity_id, check_pos, crouching, view_distance);
        } else {
            if let Some(dir) = self.entities.direction.get(&entity_id) {
                if self.map.is_in_fov_direction(entity_pos, check_pos, view_distance, *dir, crouching) {
                    fov_result = FovResult::Inside;
                } else {
                    fov_result = FovResult::Outside;
                }
            } else {
                panic!("tried to perform is_in_fov on entity without facing");
            }
        }

        let mut fog_reduction = 0;

        // If the position is within Fov then apply modifiers from fog, etc.
        if fov_result != FovResult::Outside {
            fog_reduction = self.fov_reduction(entity_id, check_pos, view_distance);
            // This subtraction is safe due to checks within fov_reduction.
            view_distance -= fog_reduction;

            let pos_dist = distance_maximum(entity_pos, check_pos);
            if pos_dist == view_distance + 1 {
                fov_result = FovResult::Edge;
            } else if pos_dist <= view_distance {
                fov_result = FovResult::Inside;
            } else {
                fov_result = FovResult::Outside;
            }
        }


        if self.entities.typ[&entity_id] == EntityType::Player {
            // If we can't see the tile, check for a latern that illuminates it, allowing
            // us to see it anyway. Ignore tiles that are blocked for sight anyway.
            if fov_result != FovResult::Inside && !self.map[check_pos].block_sight {
                // First, check that there is no FoV blocker between the player
                // and the check position. If there is, we return the current answer.
                // Otherwise we check for illuminators.
                for to_pos in line(entity_pos, check_pos) {
                    for from_pos in line(check_pos, entity_pos) {
                        // If the lines overlap, check for FoV modifying entities.
                        if to_pos == from_pos {
                            for (_entity_id, fov_block) in self.entities.fov_block.iter() {
                                if matches!(fov_block, FovBlock::Opaque(_)) || matches!(fov_block, FovBlock::Block) {

                                    // We just return fov_result here as the remaining modifications
                                    // only apply illumination which does not pierce the fog.
                                    return fov_result;
                                }
                            }
                        }
                    }
                }

                fov_result = self.check_illumination(entity_id, fov_result, check_pos, fog_reduction, crouching);
            }
        }


        return fov_result;
    }

    pub fn check_illumination(&self, entity_id: EntityId, init_fov_result: FovResult, check_pos: Pos, reduction: i32, crouching: bool) -> FovResult {
        let mut fov_result = init_fov_result;
        let entity_pos = self.entities.pos[&entity_id];
        
        if reduction > ILLUMINATE_FOV_RADIUS {
            return fov_result;
        }
        let illuminate_fov_radius = ILLUMINATE_FOV_RADIUS - reduction;

        // check for illumination that might make this tile visible.
        for (id, illuminate_radius) in self.entities.illuminate.iter() {
            let illuminate_radius = *illuminate_radius as i32;

            let illuminator_on_map = self.entities.pos[&id].x >= 0 && self.entities.pos[&id].y >= 0;

            if illuminate_radius != 0 && illuminator_on_map && !self.entities.needs_removal[&id] {
                let illuminate_pos = self.entities.pos[&id];

                let pos_near_illuminator = self.map.is_in_fov(illuminate_pos, check_pos, illuminate_radius, crouching);
                if pos_near_illuminator {
                    // Check that the position is within the radius visible through
                    // illumination. This prevents seeing illuminated tiles that are just
                    // too far for the player to reasonably see.
                    if self.map.is_in_fov(entity_pos, check_pos, illuminate_fov_radius, crouching) {
                        let max_axis_dist = distance_maximum(illuminate_pos, check_pos);
                        if max_axis_dist < illuminate_radius {
                            // The position is fully within the illumination radius.
                            fov_result = fov_result.combine(FovResult::Inside);
                        } else if max_axis_dist == illuminate_radius {
                            // The position is just at the edge of the illumation radius.
                            fov_result = fov_result.combine(FovResult::Edge);
                        }
                        // Otherwise return the original result, Edge or Outside.
                    }
                }
            }
        }

        return fov_result;
    }

    pub fn find_by_name(&self, name: EntityName) -> Option<EntityId> {
        for (key, nam) in self.entities.name.iter() {
            if *nam == name {
                return Some(key);
            }
        }

        return None;
    }

    pub fn get_golem_at_pos(&mut self, check_pos: Pos) -> Option<EntityId> {
        for key in self.entities.ids.iter() {
            let pos = self.entities.pos[key];
            let is_golem = self.entities.typ[key] == EntityType::Enemy;

            if is_golem && check_pos == pos {
                return Some(*key);
            }
        }

        return None;
    }

    pub fn get_entities_at_pos(&mut self, check_pos: Pos) -> Vec<EntityId> {
        let mut entity_ids: Vec<EntityId> = Vec::new();

        for key in self.entities.ids.iter() {
            let pos = self.entities.pos[key];
            let is_mouse = self.entities.name[key] == EntityName::Mouse;

            if !is_mouse && check_pos == pos {
                entity_ids.push(*key);
            }
        }

        return entity_ids;
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
        for entity_id in self.entities.ids.iter() {
            let is_disarmed_trap =
                self.entities.trap.get(entity_id).is_some() &&
                self.entities.armed.get(entity_id) == Some(&false);
            let is_item = self.entities.item.get(entity_id).is_some();

            if self.entities.pos[entity_id] == pos && (is_item || is_disarmed_trap) {
                return Some(*entity_id);
            }
        }

        return None;
    }

    pub fn has_entities(&self, pos: Pos) -> Vec<EntityId> {
        let mut entities = Vec::new();
        for (key, other_pos) in self.entities.pos.iter() {
            if *other_pos == pos {
                entities.push(key);
            }
        }

        return entities;
    }

    pub fn has_entity(&self, pos: Pos) -> Option<EntityId> {
        for (key, other_pos) in self.entities.pos.iter() {
            if *other_pos == pos {
                return Some(key);
            }
        }

        return None;
    }

    pub fn has_blocking_entity(&self, pos: Pos) -> Option<EntityId> {
        for (key, other_pos) in self.entities.pos.iter() {
            if *other_pos == pos {
                if self.entities.blocks[&key] {
                    return Some(key);
                }
            }
        }

        return None;
    }

    pub fn has_trap(&self, pos: Pos) -> Option<EntityId> {
        for (key, other_pos) in self.entities.pos.iter() {
            if *other_pos == pos {
                if self.entities.trap.get(&key).is_some() && self.entities.armed.get(&key).is_some() {
                    return Some(key);
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

    pub fn using(&self, entity_id: EntityId, item: Item) -> Option<EntityId> {
        if let Some(inventory) = self.entities.inventory.get(&entity_id) {
            if let Some(item_id) = inventory.get(0) {
                if self.entities.item[item_id] == item {
                    return Some(*item_id);
                }
            }
        }

        return None;
    }

    pub fn used_up_item(&mut self, entity_id: EntityId, item_id: EntityId) {
        if let Some(inventory) = self.entities.inventory.get_mut(&entity_id) {
            let item_index = inventory.iter()
                                      .position(|id| *id == item_id)
                                      .expect("Item wasn't in inventory!");
            inventory.remove(item_index);
        }

        self.entities.mark_for_removal(item_id);
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
    pub fn could_see(&mut self, entity_id: EntityId, target_pos: Pos, _config: &Config) -> bool {
        let current_facing = self.entities.direction[&entity_id];
        self.entities.face(entity_id, target_pos);

        let visible = self.pos_in_fov(entity_id, target_pos);

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

    pub fn throw_towards(&self, start_pos: Pos, end_pos: Pos) -> Pos {
        let mut hit_pos = start_pos;

        for pos in line(start_pos, end_pos) {
            if !self.map.is_within_bounds(pos) {
                break;
            }

            if let Some(hit_entity) = self.has_blocking_entity(pos) {
                if self.entities.typ[&hit_entity] != EntityType::Column {
                    // hitting an entity puts the stone on their tile, except
                    // for columns
                    hit_pos = pos;
                } 

                break;
            } else if self.map.move_blocked(hit_pos, pos, BlockedType::Move).is_some() {
                break;
            }

            hit_pos = pos;
        }

        return hit_pos;
    }

    pub fn calculate_use_move(&self, entity_id: EntityId, item_index: usize, dir: Direction, move_mode: MoveMode) -> ItemUseResult {
        let pos = self.entities.pos[&entity_id];
        let item_id = self.entities.inventory[&entity_id][item_index];

        let item = self.entities.item[&item_id];

        let mut result = ItemUseResult::new();
        match item {
            Item::Stone | Item::SeedOfStone | Item::GlassEye |
            Item::Lantern | Item::Teleporter | Item::SpikeTrap | 
            Item::SoundTrap | Item::BlinkTrap | Item::FreezeTrap |
            Item::Sling | Item::SeedCache | Item::SmokeBomb |
            Item::LookingGlass | Item::Thumper => {
                let dist = if item == Item::Sling {
                    PLAYER_THROW_DIST
                } else {
                    SLING_THROW_DIST
                };

                result.pos = Some(pos);
                let end_pos = dir.offset_pos(pos, dist as i32);
                let hit_pos = self.throw_towards(pos, end_pos);
                for travel_pos in line(pos, hit_pos) {
                    result.hit_positions.push(travel_pos);
                }
            }

            Item::Dagger => {
                let target_pos = dir.offset_pos(pos, 1);
                let hit_pos = dir.offset_pos(target_pos, 1);

                let is_crouching = self.entities.stance[&entity_id] == Stance::Crouching;
                let is_clear_path = self.clear_path(pos, target_pos, false);

                // if crouching and not blocked, then the dagger can be used.
                if is_crouching && is_clear_path {
                    result.pos = Some(target_pos);
                    result.hit_positions.push(hit_pos);
                }
            }

            Item::Shield => {
                let target_pos = dir.offset_pos(pos, 1);
                let hit_pos = dir.offset_pos(target_pos, 1);

                let in_facing_dir = dir == self.entities.direction[&entity_id];
                let is_clear_path = self.clear_path(pos, target_pos, false);

                // Shield attacks only occur in the entities' facing direction,
                // and if there is a path to the hit position.
                if in_facing_dir && is_clear_path {
                    result.pos = Some(target_pos);
                    result.hit_positions.push(hit_pos);
                }
            }

            Item::Hammer => {
                let hit_pos = dir.offset_pos(pos, 1);
                // hammers can always be used in any direction
                result.pos = Some(pos);
                result.hit_positions.push(hit_pos);
            }

            Item::Spear => {
                // If running, we can also attack an extra tile and move towards the golem.
                if move_mode == MoveMode::Run {
                    // move pos is where the entity will run to.
                    let move_pos = dir.offset_pos(pos, 2);

                    // We can only spear if there is a clear path to the player's
                    // position.
                    if self.clear_path(pos, dir.offset_pos(pos, 2), false) {
                        result.pos = Some(move_pos);

                        // the spear will hit both intervening positions.
                        let far_target_pos = dir.offset_pos(pos, 4);
                        result.hit_positions.push(far_target_pos);

                        let close_target_pos = dir.offset_pos(pos, 3);
                        result.hit_positions.push(close_target_pos);
                    }
                } else {
                    if self.clear_path(pos, dir.offset_pos(pos, 1), false) {
                        result.pos = Some(pos);

                        let target_pos = dir.offset_pos(pos, 2);
                        result.hit_positions.push(target_pos);

                        let target_pos = dir.offset_pos(pos, 3);
                        result.hit_positions.push(target_pos);
                    }
                }
            }

            Item::GreatSword => {
                // TODO add in great sword positions
            }

            Item::Khopesh => {
                let target_pos = dir.offset_pos(pos, 1);
                let move_pos = dir.reverse().offset_pos(pos, 1);
                if self.clear_path(pos, move_pos, false) {
                    result.pos = Some(move_pos);
                    result.hit_positions.push(target_pos);
                }
            }

            Item::Axe => {
                let target_pos = dir.offset_pos(pos, 1);
                if self.clear_path(pos, target_pos, false) {
                    result.pos = Some(pos);

                    result.hit_positions.push(target_pos);

                    let left_pos = dir.clockwise().offset_pos(pos, 1);
                    result.hit_positions.push(left_pos);

                    let right_pos = dir.counterclockwise().offset_pos(pos, 1);
                    result.hit_positions.push(right_pos);
                }
            }

            Item::Sword => {
                let target_pos = dir.offset_pos(pos, 1);
                if self.clear_path(pos, target_pos, false) {
                    result.pos = Some(target_pos);

                    for dir in &Direction::directions() {
                        let dir_pos = dir.offset_pos(pos, 1);

                        let still_adjacent = distance(target_pos, dir_pos) == 1;
                        if still_adjacent {
                            result.hit_positions.push(dir_pos);
                        }
                    }
                }
            }

            _ => {
                dbg!(item);
                panic!("Tried to use item in use-mode!");
            }
        }

        return result;
    }

    // NOTE this is no longer used
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
                self.entities.remove_entity(*id);
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ItemUseResult {
    pub pos: Option<Pos>,
    pub hit_positions: Vec<Pos>,
}

impl ItemUseResult {
    pub fn new() -> ItemUseResult {
        return ItemUseResult {
            pos: None,
            hit_positions: Vec::new(),
        };
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FovBlock {
    Block,
    Transparent,
    Opaque(usize),
    Magnify(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AttackStyle {
    Stealth,
    Normal,
    Strong,
}

impl fmt::Display for AttackStyle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AttackStyle::Stealth => write!(f, "stealth"),
            AttackStyle::Normal => write!(f, "normal"),
            AttackStyle::Strong => write!(f, "strong"),
        }
    }
}

impl FromStr for AttackStyle {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();
        if s == "stealth" {
            return Ok(AttackStyle::Stealth);
        } else if s == "normal" {
            return Ok(AttackStyle::Normal);
        } else if s == "strong" {
            return Ok(AttackStyle::Strong);
        }

        return Err(format!("Could not parse '{}' as AttackStyle", s));
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum WeaponType {
    Blunt,
    Pierce,
    Slash,
}

impl fmt::Display for WeaponType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WeaponType::Blunt => write!(f, "blunt"),
            WeaponType::Pierce => write!(f, "pierce"),
            WeaponType::Slash => write!(f, "slash"),
        }
    }
}

impl FromStr for WeaponType {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();
        if s == "blunt" {
            return Ok(WeaponType::Blunt);
        } else if s == "pierce" {
            return Ok(WeaponType::Pierce);
        } else if s == "slash" {
            return Ok(WeaponType::Slash);
        }

        return Err(format!("Could not parse '{}' as WeaponType", s));
    }
}

impl WeaponType {
    pub fn sharp(&self) -> bool {
        return *self == WeaponType::Slash || *self == WeaponType::Pierce;
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
    GrassWall,
    GrassThrow,
    GrassBlade,
    GrassShoes,
    GrassCover,
    Blink,
    PassWall,
    Rubble,
    StoneThrow,
    Reform,
    Swap,
    Push,
    Illuminate,
    Heal,
    FarSight,
    Sprint,
    Roll,
    Ping,
    StoneSkin,
    PassThrough,
    WhirlWind,
    Swift,
}

impl fmt::Display for Skill {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Skill::GrassWall => write!(f, "grass_wall"),
            Skill::GrassThrow => write!(f, "grass_throw"),
            Skill::GrassBlade => write!(f, "grass_blade"),
            Skill::GrassShoes => write!(f, "grass_shoes"),
            Skill::GrassCover => write!(f, "grass_cover"),
            Skill::Blink => write!(f, "blink"),
            Skill::PassWall => write!(f, "pass_wall"),
            Skill::Rubble => write!(f, "rubble"),
            Skill::StoneThrow => write!(f, "stone_throw"),
            Skill::Reform => write!(f, "reform"),
            Skill::Swap => write!(f, "swap"),
            Skill::Push => write!(f, "push"),
            Skill::Illuminate => write!(f, "illuminate"),
            Skill::Heal => write!(f, "heal"),
            Skill::FarSight => write!(f, "farsight"),
            Skill::Sprint => write!(f, "sprint"),
            Skill::Roll => write!(f, "roll"),
            Skill::Ping => write!(f, "ping"),
            Skill::StoneSkin => write!(f, "stone_skin"),
            Skill::PassThrough => write!(f, "pass_through"),
            Skill::WhirlWind => write!(f, "whirlwind"),
            Skill::Swift => write!(f, "swift"),
        }
    }
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
    Use,
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
            GameState::Use => write!(f, "use"),
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
        return self == GameState::Inventory   ||
               self == GameState::SkillMenu   ||
               self == GameState::ConfirmQuit ||
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
pub struct ItemModifier {
    pub fragile: bool,
    pub modifier: i32,
}

impl ItemModifier {
    pub fn new() -> ItemModifier {
        return ItemModifier { fragile: false, modifier: 0 };
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Item {
    Stone,
    Key,
    Dagger,
    Shield,
    Hammer,
    Spear,
    GreatSword,
    Axe,
    Khopesh,
    Sword,
    Lantern,
    Thumper,
    Sling,
    Teleporter,
    Herb,
    SeedOfStone,
    SeedCache,
    SmokeBomb,
    LookingGlass,
    GlassEye,
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
            Item::Spear => write!(f, "spear"),
            Item::GreatSword => write!(f, "greatsword"),
            Item::Sword => write!(f, "sword"),
            Item::Axe => write!(f, "axe"),
            Item::Khopesh => write!(f, "khopesh"),
            Item::Lantern => write!(f, "lantern"),
            Item::Thumper => write!(f, "thumper"),
            Item::Sling => write!(f, "sling"),
            Item::Teleporter => write!(f, "teleporter"),
            Item::Herb => write!(f, "herb"),
            Item::SeedOfStone => write!(f, "seedofstone"),
            Item::SeedCache => write!(f, "seedcache"),
            Item::SmokeBomb => write!(f, "smokebomb"),
            Item::LookingGlass => write!(f, "lookinglass"),
            Item::GlassEye => write!(f, "glasseye"),
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
        } else if s == "spear" {
            return Ok(Item::Spear);
        } else if s == "lantern" {
            return Ok(Item::Lantern);
        } else if s == "thumper" {
            return Ok(Item::Thumper);
        } else if s == "axe" {
            return Ok(Item::Axe);
        } else if s == "khopesh" {
            return Ok(Item::Khopesh);
        } else if s == "sling" {
            return Ok(Item::Sling);
        } else if s == "greatsword" {
            return Ok(Item::GreatSword);
        } else if s == "sword" {
            return Ok(Item::Sword);
        } else if s == "teleporter" {
            return Ok(Item::Teleporter);
        } else if s == "herb" {
            return Ok(Item::Herb);
        } else if s == "seedofstone" {
            return Ok(Item::SeedOfStone);
        } else if s == "seedcache" {
            return Ok(Item::SeedCache);
        } else if s == "smokebomb" {
            return Ok(Item::SmokeBomb);
        } else if s == "lookingglass" {
            return Ok(Item::LookingGlass);
        } else if s == "glasseye" {
            return Ok(Item::GlassEye);
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
            Item::Stone => ItemClass::Misc,
            Item::Key => ItemClass::Misc,
            Item::Dagger => ItemClass::Primary,
            Item::Shield => ItemClass::Primary,
            Item::Hammer => ItemClass::Primary,
            Item::Spear => ItemClass::Primary,
            Item::GreatSword => ItemClass::Primary,
            Item::Sword => ItemClass::Primary,
            Item::Axe => ItemClass::Primary,
            Item::Khopesh => ItemClass::Primary,
            Item::Sling => ItemClass::Primary,
            Item::Teleporter => ItemClass::Consumable,
            Item::Herb => ItemClass::Consumable,
            Item::SeedOfStone => ItemClass::Consumable,
            Item::SeedCache => ItemClass::Consumable,
            Item::SmokeBomb => ItemClass::Consumable,
            Item::LookingGlass => ItemClass::Consumable,
            Item::GlassEye => ItemClass::Consumable,
            Item::Lantern => ItemClass::Consumable,
            Item::Thumper => ItemClass::Consumable,
            Item::SpikeTrap => ItemClass::Consumable,
            Item::SoundTrap => ItemClass::Consumable,
            Item::BlinkTrap => ItemClass::Consumable,
            Item::FreezeTrap => ItemClass::Consumable,
        }
    }

    pub fn name(&self) -> EntityName {
        match self {
            Item::Stone => EntityName::Stone,
            Item::Key => EntityName::Key,
            Item::Dagger => EntityName::Dagger,
            Item::Shield => EntityName::Shield,
            Item::Hammer => EntityName::Hammer,
            Item::Spear => EntityName::Spear,
            Item::GreatSword => EntityName::GreatSword,
            Item::Sword => EntityName::Sword,
            Item::Axe => EntityName::Axe,
            Item::Khopesh => EntityName::Khopesh,
            Item::Teleporter => EntityName::Teleporter,
            Item::Herb => EntityName::Herb,
            Item::SeedOfStone => EntityName::SeedOfStone,
            Item::SeedCache => EntityName::SeedCache,
            Item::SmokeBomb => EntityName::SmokeBomb,
            Item::LookingGlass => EntityName::LookingGlass,
            Item::GlassEye => EntityName::GlassEye,
            Item::Lantern => EntityName::Lantern,
            Item::Thumper => EntityName::Thumper,
            Item::Sling => EntityName::Sling,
            Item::SpikeTrap => EntityName::SpikeTrap,
            Item::SoundTrap => EntityName::SoundTrap,
            Item::BlinkTrap => EntityName::BlinkTrap,
            Item::FreezeTrap => EntityName::FreezeTrap,
        }
    }

    pub fn weapon_type(&self) -> Option<WeaponType> {
        match self {
            Item::Dagger => Some(WeaponType::Slash),
            Item::Shield => Some(WeaponType::Blunt),
            Item::Hammer => Some(WeaponType::Blunt),
            Item::Spear => Some(WeaponType::Pierce),
            Item::GreatSword => Some(WeaponType::Slash),
            Item::Sword => Some(WeaponType::Slash),
            Item::Sling => Some(WeaponType::Blunt),
            Item::Axe => Some(WeaponType::Slash),
            Item::Khopesh => Some(WeaponType::Slash),

            Item::Teleporter => None,
            Item::SeedOfStone => None,
            Item::SeedCache => None,
            Item::SmokeBomb => None,
            Item::LookingGlass => None,
            Item::GlassEye => None,
            Item::Herb => None,
            Item::Stone => None,
            Item::Key => None,
            Item::Lantern => None,
            Item::Thumper => None,
            Item::SpikeTrap => None,
            Item::SoundTrap => None,
            Item::BlinkTrap => None,
            Item::FreezeTrap => None,
        }
    }

    pub fn throw_stun_turns(&self) -> usize {
        match self {
            Item::Stone => STONE_STUN_TURNS,
            Item::Spear => SPEAR_STUN_TURNS,
            _ => DEFAULT_ITEM_STUN_TURNS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ItemClass {
    Primary,
    Consumable,
    Misc,
}

impl fmt::Display for ItemClass {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ItemClass::Primary => write!(f, "primary"),
            ItemClass::Consumable => write!(f, "consumable"),
            ItemClass::Misc => write!(f, "misc"),
        }
    }
}

impl FromStr for ItemClass {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();
        if s == "primary" {
            return Ok(ItemClass::Primary);
        } else if s == "consumable" {
            return Ok(ItemClass::Consumable);
        } else if s == "misc" {
            return Ok(ItemClass::Misc);
        }

        return Err(format!("Could not parse '{}' as ItemClass", s));
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum UseAction {
    Item(ItemClass),
    Interact,
}

impl fmt::Display for UseAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UseAction::Item(item_class) => item_class.fmt(f),
            UseAction::Interact => write!(f, "interact"),
        }
    }
}

impl FromStr for UseAction {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();
        if s == "interact" {
            return Ok(UseAction::Interact);
        } else {
            return ItemClass::from_str(string).map(|item_class| UseAction::Item(item_class));
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hp {
    pub max_hp: i32,
    pub hp: i32,
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
        dbg!();
        match (self, move_mode) {
            (Stance::Crouching, MoveMode::Run) => Stance::Standing,
            (Stance::Standing, MoveMode::Sneak) => Stance::Crouching,
            (Stance::Running, _) => Stance::Standing,
            _ => *self,
        }
    }
}

impl fmt::Display for Stance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Stance::Crouching => write!(f, "crouching"),
            Stance::Standing => write!(f, "standing"),
            Stance::Running => write!(f, "running"),
        }
    }
}

impl FromStr for Stance {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();
        if s == "crouching" {
            return Ok(Stance::Crouching);
        } else if s == "standing" {
            return Ok(Stance::Standing);
        } else if s == "running" {
            return Ok(Stance::Running);
        }

        return Err(format!("Could not parse '{}' as Stance", s));
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum EntityName {
    Player,
    Gol,
    Pawn,
    Rook,
    Column,
    Key,
    Exit,
    Dagger,
    Hammer,
    Spear,
    GreatSword,
    Sword,
    Shield,
    Lantern,
    Thumper,
    Axe,
    Khopesh,
    Sling,
    SeedOfStone,
    SeedCache,
    SmokeBomb,
    LookingGlass,
    GlassEye,
    Teleporter,
    Spire,
    Armil,
    SpikeTrap,
    BlinkTrap,
    FreezeTrap,
    SoundTrap,
    GateTrigger,
    Stone,
    Mouse,
    Cursor,
    Energy,
    Herb,
    Grass,
    Statue,
    Smoke,
    Magnifier,
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
            EntityName::Rook => write!(f, "rook"),
            EntityName::Column => write!(f, "column"),
            EntityName::Key => write!(f, "key"),
            EntityName::Exit => write!(f, "exit"),
            EntityName::Dagger => write!(f, "dagger"),
            EntityName::Hammer => write!(f, "hammer"),
            EntityName::Spear => write!(f, "spear"),
            EntityName::GreatSword => write!(f, "greatsword"),
            EntityName::Axe => write!(f, "axe"),
            EntityName::Khopesh => write!(f, "khopesh"),
            EntityName::Sword => write!(f, "sword"),
            EntityName::Teleporter => write!(f, "teleporter"),
            EntityName::Lantern => write!(f, "lantern"),
            EntityName::Thumper => write!(f, "thumper"),
            EntityName::Sling => write!(f, "sling"),
            EntityName::SeedOfStone => write!(f, "seedofstone"),
            EntityName::SeedCache => write!(f, "seedcache"),
            EntityName::SmokeBomb => write!(f, "smokebomb"),
            EntityName::LookingGlass => write!(f, "lookingglass"),
            EntityName::GlassEye => write!(f, "glasseye"),
            EntityName::Shield => write!(f, "shield"),
            EntityName::Spire => write!(f, "spire"),
            EntityName::Armil => write!(f, "armil"),
            EntityName::SpikeTrap => write!(f, "spiketrap"),
            EntityName::BlinkTrap => write!(f, "blinktrap"),
            EntityName::FreezeTrap => write!(f, "freezetrap"),
            EntityName::SoundTrap => write!(f, "soundtrap"),
            EntityName::GateTrigger => write!(f, "gatetrigger"),
            EntityName::Stone => write!(f, "stone"),
            EntityName::Mouse => write!(f, "mouse"),
            EntityName::Cursor => write!(f, "cursor"),
            EntityName::Energy => write!(f, "energy"),
            EntityName::Herb => write!(f, "herb"),
            EntityName::Grass => write!(f, "grass"),
            EntityName::Statue => write!(f, "statue"),
            EntityName::Smoke => write!(f, "smoke"),
            EntityName::Magnifier => write!(f, "magnifier"),
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
        } else if s == "rook" {
            return Ok(EntityName::Rook);
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
        } else if s == "teleporter" {
            return Ok(EntityName::Teleporter);
        } else if s == "spear" {
            return Ok(EntityName::Spear);
        } else if s == "greatsword" {
            return Ok(EntityName::GreatSword);
        } else if s == "sword" {
            return Ok(EntityName::Sword);
        } else if s == "axe" {
            return Ok(EntityName::Axe);
        } else if s == "khopesh" {
            return Ok(EntityName::Khopesh);
        } else if s == "lantern" {
            return Ok(EntityName::Lantern);
        } else if s == "thumper" {
            return Ok(EntityName::Thumper);
        } else if s == "sling" {
            return Ok(EntityName::Sling);
        } else if s == "seedofstone" {
            return Ok(EntityName::SeedOfStone);
        } else if s == "seedcache" {
            return Ok(EntityName::SeedCache);
        } else if s == "smokebomb" {
            return Ok(EntityName::SmokeBomb);
        } else if s == "lookingglass" {
            return Ok(EntityName::LookingGlass);
        } else if s == "glasseye" {
            return Ok(EntityName::GlassEye);
        } else if s == "shield" {
            return Ok(EntityName::Shield);
        } else if s == "spire" {
            return Ok(EntityName::Spire);
        } else if s == "armil" {
            return Ok(EntityName::Armil);
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
        } else if s == "herb" {
            return Ok(EntityName::Herb);
        } else if s == "grass" {
            return Ok(EntityName::Grass);
        } else if s == "statue" {
            return Ok(EntityName::Statue);
        } else if s == "smoke" {
            return Ok(EntityName::Smoke);
        } else if s == "magnifier" {
            return Ok(EntityName::Magnifier);
        } else if s == "other" {
            return Ok(EntityName::Other);
        }

        dbg!(s);
        panic!("EntityName not expected!");
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
    Environment,
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
            EntityType::Environment => write!(f, "environment"),
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
        } else if s == "environment" {
            return Ok(EntityType::Environment);
        } else if s == "other" {
            return Ok(EntityType::Other);
        }

        dbg!(s);
        panic!("EntityType not expected!");
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
    Wind,
}

impl fmt::Display for EntityClass {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EntityClass::General => write!(f, "general"),
            EntityClass::Grass => write!(f, "grass"),
            EntityClass::Monolith => write!(f, "monolith"),
            EntityClass::Clockwork => write!(f, "clockword"),
            EntityClass::Hierophant => write!(f, "hierophant"),
            EntityClass::Wind => write!(f, "wind"),
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
        return vec!(General, Grass, Monolith, Clockwork, Hierophant, Wind);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Passive {
    pub stone_thrower: bool,
    pub whet_stone: bool,
    pub soft_shoes: bool,
    pub light_touch: bool,
    pub sure_footed: bool,
    pub quick_reflexes: bool,
}

impl Passive {
    pub fn new() -> Passive {
        return Passive::default();
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct StatusEffect {
    pub frozen: usize, // turns
    pub soft_steps: usize, // turns
    pub extra_fov: usize, // amount
    pub blinked: bool,
    pub active: bool,
    pub alive: bool,
    pub stone: usize,
    pub land_roll: bool,
    pub hammer_raised: Option<(EntityId, Direction, usize)>, // item id, direction to hit, turns to wait
    pub test_mode: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Message {
    Sound(Pos),
    Attack(EntityId),
    Hit(EntityId, Pos),
    Disappeared(EntityId),
}

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
    pub count_down: Comp<usize>,
    pub move_mode: Comp<MoveMode>,
    pub direction: Comp<Direction>,
    pub selected_item: Comp<EntityId>,
    pub class: Comp<EntityClass>,
    pub skills: Comp<Vec<Skill>>,
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
    pub sound: Comp<Pos>, // source position
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

    pub fn use_energy(&mut self, entity_id: EntityId) {
        if !self.status[&entity_id].test_mode {
            self.energy[&entity_id] -= 1;
        }
    }

    pub fn take_damage(&mut self, entity_id: EntityId, damage: i32) {
        if let Some(hp) = self.hp.get_mut(&entity_id) {
            if damage > 0 && !self.status[&entity_id].test_mode {
                hp.hp -= damage;
            }
        }

        if let Some(hp) = self.hp.get(&entity_id) {
            if hp.hp <= 0 {
                self.status[&entity_id].alive = false;
            }
        }
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
        if let Some(index) = self.messages[&entity_id].iter().position(|msg| matches!(msg, Message::Hit(_, _))) {
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
        // removing the player is handled specially
        if self.typ[&entity_id] != EntityType::Player {
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
    }
}

