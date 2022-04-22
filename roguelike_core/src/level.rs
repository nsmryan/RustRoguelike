
use serde::{Serialize, Deserialize};

use pathfinding::directed::astar::astar;

use roguelike_utils::line::*;
use roguelike_utils::comp::*;

use crate::map::*;
use crate::movement::*;
use crate::config::Config;
use crate::constants::*;
use crate::entities::*;
use crate::types::*;


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

    pub fn get_no_entity_pos(&self) -> Vec<Pos> {
        return self.map.get_empty_pos()
                       .iter()
                       .map(|pos| *pos)
                       .filter(|pos| self.has_entity(*pos).is_none())
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

    pub fn get_entities_at_pos(&self, check_pos: Pos) -> Vec<EntityId> {
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

        // Its possible that there is a valid move position, but no position to
        // hit. In this case, suppress the move position to indicate that the
        // use-mode movement is invalid.
        if result.hit_positions.len() == 0 {
            result.pos = None;
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

