use serde::{Serialize, Deserialize};

use crate::constants::*;
use crate::types::*;
use crate::movement::*;
use crate::messaging::*;
use crate::utils::*;
use crate::config::Config;


#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Ai {
    Basic,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Behavior {
    Idle,
    Investigating(Pos),
    Attacking(EntityId),
}

impl Default for Behavior {
    fn default() -> Behavior {
        return Behavior::Idle;
    }
}

impl Behavior {
    pub fn description(&self) -> String {
        match self {
            Behavior::Idle => "idle".to_string(),
            Behavior::Investigating(_position) => "investigating".to_string(),
            Behavior::Attacking(_obj_id) => "attacking".to_string(),
        }
    }

    pub fn is_aware(&self) -> bool {
        return matches!(self, Behavior::Attacking(_));
    }
}

pub fn ai_take_turn(monster_id: EntityId,
                    data: &mut GameData,
                    config: &Config,
                    _msg_log: &mut MsgLog) -> Action {
    let mut turn: Action = Action::NoAction;

    if data.entities.alive[&monster_id] {
        match data.entities.ai.get(&monster_id) {
            Some(Ai::Basic) => {
                turn = basic_ai_take_turn(monster_id, data, config);
            }

            None => {
                panic!("AI didn't have an ai entry!");
            }
        }
    }

    return turn;
}

pub fn step_towards(start_pos: Pos, target_pos: Pos) -> Pos {
    let dx = target_pos.x - start_pos.x;
    let dy = target_pos.y - start_pos.y;
    let delta_pos = Pos::new(signedness(dx), signedness(dy));
    return delta_pos;
}

pub fn ai_attack(monster_id: EntityId,
                 target_id: EntityId,
                 data: &mut GameData,
                 config: &Config) -> Action {
    let target_pos = data.entities.pos[&target_id];
    let monster_pos = data.entities.pos[&monster_id];

    let turn: Action;

    let attack_reach = data.entities.attack[&monster_id];

    data.entities.face(monster_id, target_pos);

    if !data.entities.alive[&target_id] {
        // if AI target is no longer alive
        turn = Action::StateChange(Behavior::Investigating(target_pos));
    } else if let Some(hit_pos) =
        // if AI can hit their target
        ai_can_hit_target(data, 
                          monster_id,
                          target_pos,
                          &attack_reach,
                          config) {
        let attack = Attack::Attack(target_id);
        turn = Action::Move(Movement::attack(hit_pos, MoveType::Move, attack));
    } else if !data.is_in_fov(monster_id, target_pos, config) {
        // path to target is blocked by a wall- investigate the last known position
        turn = Action::StateChange(Behavior::Investigating(target_pos));
    } else {
        // can see target, but can't hit them. try to move to a position where we can hit them
        let mut new_pos = monster_pos;

        let pos_offset;
        let (attack, movement) =
            (data.entities.attack[&monster_id], data.entities.movement[&monster_id]);

        let mut potential_move_targets = Vec::new();

        // check all movement options in case one lets us hit the target
        let direction = data.entities.direction[&monster_id];
        for move_action in Direction::move_actions() {
            for attack_offset in attack.attacks_with_reach(&move_action) {
                let attackable_pos = add_pos(target_pos, attack_offset);

                if attackable_pos == monster_pos ||
                   !data.map.is_within_bounds(attackable_pos) {
                    continue;
                }

                data.entities.set_pos(monster_id, attackable_pos);
                data.entities.face(monster_id, target_pos);
                let can_hit = ai_can_hit_target(data, monster_id, target_pos, &attack, config).is_some();

                if can_hit {
                    potential_move_targets.push(attackable_pos);
                }
            }
        }
        data.entities.set_pos(monster_id, monster_pos);
        data.entities.direction[&monster_id] = direction;

        // look through all potential positions for the shortest path
        let mut targets = potential_move_targets.iter();
        if let Some(first_target) = targets.next() {
            let must_reach = true;
            let traps_block = true;

            let mut best_target = first_target;

            let path = data.path_between(monster_pos, *best_target, movement, must_reach, traps_block, None);
            let mut best_dist = path.len();

            let large_dist = (MAP_WIDTH + MAP_HEIGHT) as usize;
            if best_dist == 0 {
                best_dist = large_dist;
            }

            for move_target in targets {
                let path = data.path_between(monster_pos, *move_target, movement, must_reach, traps_block, None);
                let path_length = path.len();
                    
                if path_length > 0 && (path_length < best_dist || best_dist == large_dist) {
                    best_dist = path_length;
                    best_target = move_target;
                }
            }

            if best_dist > 0 && best_dist != large_dist {
                new_pos = *best_target;
            }
        }
        // step towards the closest location that lets us hit the target
        pos_offset = ai_take_astar_step(monster_id, new_pos, true, &data);
        if pos_mag(pos_offset) > 0 {
            turn = Action::Move(Movement::move_to(add_pos(monster_pos, pos_offset), MoveType::Move));
        } else {
            turn = Action::NoAction;
        }
    }

    return turn;
}

pub fn ai_idle(monster_id: EntityId,
               data: &mut GameData,
               config: &Config) -> Action {
    let player_id = data.find_player().unwrap();
    let player_pos = data.entities.pos[&player_id];

    let mut turn = Action::none();

    if data.is_in_fov(monster_id, player_pos, config) {
        data.entities.face(monster_id, player_pos);
        turn = Action::StateChange(Behavior::Attacking(player_id));
    } else if let Some(Message::Attack(entity_id)) = data.entities.was_attacked(monster_id) {
        data.entities.face(monster_id, player_pos);
        turn = Action::StateChange(Behavior::Attacking(entity_id));
    } else if let Some(Message::Sound(entity_id, sound_pos)) = data.entities.heard_sound(monster_id) {
        let in_fov = data.is_in_fov(monster_id, sound_pos, config);
        let is_player = entity_id == player_id;

        let needs_investigation = is_player;

        // only investigate if the monster can't see the tile, or if they can but it contains the
        // player.
        if needs_investigation {
            data.entities.face(monster_id, sound_pos);
            turn = Action::StateChange(Behavior::Investigating(sound_pos));
        }
    }

    return turn;
}

pub fn ai_investigate(target_pos: Pos, 
                      monster_id: EntityId,
                      data: &mut GameData,
                      config: &Config) -> Action {
    let player_id = data.find_player().unwrap();

    let player_pos = data.entities.pos[&player_id];
    let monster_pos = data.entities.pos[&monster_id];

    let mut turn: Action;

    let (dx, dy) = sub_pos(player_pos,  monster_pos).to_tuple();
    if data.map.is_blocked_by_wall(monster_pos, dx, dy).is_none() {
        data.entities.face(monster_id, player_pos);
    }
               
    if data.is_in_fov(monster_id, player_pos, config) {
        data.entities.face(monster_id, player_pos);
        turn = Action::StateChange(Behavior::Attacking(player_id));
    } else { // the monster can't see the player
        if let Some(Message::Sound(_entity_id, pos)) = data.entities.heard_sound(monster_id) {
            turn = Action::StateChange(Behavior::Investigating(pos));
        } else {
            if target_pos == monster_pos { 
                // if the monster reached its target then go back to being idle
                turn = Action::StateChange(Behavior::Idle);
            } else {
                // if the monster has not reached its target, move towards the target.
                let must_reach = false;
                let pos_offset = ai_take_astar_step(monster_id, target_pos, must_reach, &data);

                let movement = Movement::move_to(add_pos(monster_pos, pos_offset), MoveType::Move);
                turn = Action::Move(movement);
            }
        }
    }

    // if the monster moved, but didn't go anywhere, they stop investigating
    if let Action::Move(movement) = turn {
        if movement.pos == monster_pos {
            // NOTE this causes monster to give up whenever they can't reach their goal.
            // the problem is that this might happen in a long corridor, for example, where
            // you might want them to keep trying for a while in case there is a monster
            // in front of them.
            turn = Action::StateChange(Behavior::Idle);
        }
    }

    return turn;
}

fn ai_can_hit_target(data: &mut GameData,
                     monster_id: EntityId,
                     target_pos: Pos,
                     reach: &Reach,
                     config: &Config) -> Option<Pos> {
    let mut hit_pos = None;
    let monster_pos = data.entities.pos[&monster_id];

    // don't allow hitting from the same tile...
    if target_pos == monster_pos {
        return None;
    }

    let within_fov =
        data.is_in_fov(monster_id, target_pos, config);

    let clear_path = data.clear_path_up_to(monster_pos, target_pos, false);

    if within_fov && clear_path {
        // get all locations they can hit
        let positions: Vec<Pos> =
            reach.offsets()
            .iter()
            .map(|pos| Pos::new(pos.x + monster_pos.x, pos.y + monster_pos.y))
            .collect();

        // look through attack positions, in case one hits the target
        for pos in positions {
            if target_pos == pos {
                hit_pos = Some(pos);
                break;
            }
        }
    }

    return hit_pos;
}

fn ai_astar_cost(_start: Pos, _prev: Pos, next: Pos, data: &GameData) -> Option<i32> {
    let mut cost = Some(1);
    if let Some(entity_id) = data.has_entity(next) {
        if data.entities.trap.get(&entity_id).is_some() &&
           data.entities.armed.get(&entity_id) == Some(&true) {
               // NOTE determined randomly. could be infinite, or smaller?
               cost = None;
        }
    }

    return cost;
}

fn ai_astar_step(monster_id: EntityId,
                 target_pos: Pos,
                 must_reach: bool,
                 data: &GameData) -> Vec<Pos> {
    let reach = data.entities.movement[&monster_id];
    let monster_pos = data.entities.pos[&monster_id];

    let traps_block = true;

    let path = data.path_between(monster_pos, target_pos, reach, must_reach, traps_block, Some(ai_astar_cost));

    return path;
}

fn ai_take_astar_step(monster_id: EntityId,
                      target_pos: Pos,
                      must_reach: bool,
                      data: &GameData) -> Pos {
    let path = ai_astar_step(monster_id, target_pos, must_reach, data);

    if path.len() > 1 {
        let monster_pos = data.entities.pos[&monster_id];
        return step_towards(monster_pos, path[1]);
    } else {
        return Pos::new(0, 0);
    }
}

// NOTE this function takes a mutable GameData because FOV requires
// mutation under the hood. It does not otherwise modify the game
pub fn basic_ai_take_turn(monster_id: EntityId,
                          data: &mut GameData,
                          config: &Config) -> Action {
    let monster_pos = data.entities.pos[&monster_id];

    if data.map.is_within_bounds(monster_pos) {
        match data.entities.behavior[&monster_id] {
            Behavior::Idle => {
                return ai_idle(monster_id, data, config);
            }

            Behavior::Investigating(target_pos) => {
                return ai_investigate(target_pos, monster_id, data, config);
            }

            Behavior::Attacking(object_id) => {
                return ai_attack(monster_id, object_id, data, config);
            }
        }
    } else {
        // position outside of map- return empty turn
        return Action::none();
    }
}

//pub fn ai_finalize_turn(monster_id: EntityId,
//                        data: &mut GameData,
//                        config: &Config,
//                        _msg_log: &mut MsgLog) -> Option<Action> {
//    let behavior = data.entities.behavior[&monster_id];
//    match behavior {
//        Behavior::Investigating(target_pos)) => {
//        }
//
//        Behavior::Attacking(target_id) => {
//        }
//
//        _ => {
//        }
//    }
//}
