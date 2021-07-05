use std::fmt;

use serde::{Serialize, Deserialize};

use crate::constants::*;
use crate::types::*;
use crate::movement::*;
use crate::messaging::*;
use crate::utils::*;
use crate::config::Config;
use crate::map::Wall;


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

impl fmt::Display for Behavior {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Behavior::Idle => write!(f, "idle"),
            Behavior::Investigating(pos) => write!(f, "investigating {} {}", pos.x, pos.y),
            Behavior::Attacking(entity_id) => write!(f, "attacking {}", entity_id),
        }
    }
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
                    msg_log: &mut MsgLog) {
    if data.entities.status[&monster_id].alive {
        match data.entities.ai.get(&monster_id) {
            Some(Ai::Basic) => {
                basic_ai_take_turn(monster_id, data, msg_log, config);
            }

            None => {
                panic!("AI didn't have an ai entry!");
            }
        }
    }
}

pub fn basic_ai_take_turn(monster_id: EntityId,
                          data: &mut GameData,
                          msg_log: &mut MsgLog,
                          config: &Config) {
    let monster_pos = data.entities.pos[&monster_id];

    if data.map.is_within_bounds(monster_pos) {
        if data.entities.status[&monster_id].frozen == 0 {
            match data.entities.behavior[&monster_id] {
                Behavior::Idle => {
                    ai_idle(monster_id, data, msg_log, config);
                }

                Behavior::Investigating(target_pos) => {
                    ai_investigate(target_pos, monster_id, data, msg_log, config);
                }

                Behavior::Attacking(object_id) => {
                    ai_attack(monster_id, object_id, data, msg_log, config);
                }
            }
        }
    }
}

pub fn ai_attack(monster_id: EntityId,
                 target_id: EntityId,
                 data: &mut GameData,
                 msg_log: &mut MsgLog,
                 _config: &Config) {
    let target_pos = data.entities.pos[&target_id];

    // we need to turn towards the target first, so the
    // rest of the processing is done in the AIAttack message
    msg_log.log(Msg::FaceTowards(monster_id, target_pos));
    msg_log.log(Msg::AiAttack(monster_id));
}

pub fn ai_idle(monster_id: EntityId,
               data: &mut GameData,
               msg_log: &mut MsgLog,
               config: &Config) {
    let player_id = data.find_by_name(EntityName::Player).unwrap();

    if config.sound_golem_idle_radius > 0 {
        let should_animate = true;
        let monster_pos = data.entities.pos[&monster_id];
        msg_log.log(Msg::Sound(monster_id, monster_pos, config.sound_golem_idle_radius, should_animate));
    }

    if ai_is_in_fov(monster_id, player_id, data, config) {
        let player_pos = data.entities.pos[&player_id];
        msg_log.log(Msg::FaceTowards(monster_id, player_pos));

        if data.entities.attack.get(&monster_id).is_some() {
            msg_log.log(Msg::StateChange(monster_id, Behavior::Attacking(player_id)));
        } else {
            msg_log.log(Msg::StateChange(monster_id, Behavior::Investigating(player_pos)));
        }
    } else if let Some(Message::Attack(entity_id)) = data.entities.was_attacked(monster_id) {
    let entity_pos = data.entities.pos[&entity_id];
        msg_log.log(Msg::FaceTowards(monster_id, entity_pos));

        if data.entities.attack.get(&monster_id).is_some() {
            msg_log.log(Msg::StateChange(monster_id, Behavior::Attacking(entity_id)));
        } else {
            msg_log.log(Msg::StateChange(monster_id, Behavior::Investigating(entity_pos)));
        }
    } else if let Some(Message::Sound(entity_id, sound_pos)) = data.entities.heard_sound(monster_id) {
        let is_player = entity_id == player_id;

        let needs_investigation = is_player;

        if needs_investigation {
            msg_log.log(Msg::FaceTowards(monster_id, sound_pos));
            msg_log.log(Msg::StateChange(monster_id, Behavior::Investigating(sound_pos)));
        }
    }
}

pub fn ai_investigate(target_pos: Pos, 
                      monster_id: EntityId,
                      data: &mut GameData,
                      msg_log: &mut MsgLog,
                      config: &Config) {
    let player_id = data.find_by_name(EntityName::Player).unwrap();

    let monster_pos = data.entities.pos[&monster_id];

    let player_in_fov = ai_is_in_fov(monster_id, player_id, data, config);

    if player_in_fov {
        //let fov_path_clear = data.map.path_blocked_fov(monster_pos, player_pos).is_none();
        let player_pos = data.entities.pos[&player_id];
        msg_log.log(Msg::FaceTowards(monster_id, player_pos));

        if data.entities.attack.get(&monster_id).is_some() {
            msg_log.log(Msg::StateChange(monster_id, Behavior::Attacking(player_id)));
        } else {
            // if the monster cannot attack, just keep walking towards the target.
            ai_move_towards_target(player_pos, monster_id, data, msg_log);

            data.entities.took_turn[&monster_id] = true;
            msg_log.log(Msg::StateChange(monster_id, Behavior::Investigating(player_pos)));
        }
    } else { // the monster can't see the player
        if let Some(Message::Sound(_entity_id, pos)) = data.entities.heard_sound(monster_id) {
            msg_log.log(Msg::StateChange(monster_id, Behavior::Investigating(pos)));
        } else {
            if target_pos == monster_pos { 
                // monster reached their target position
                data.entities.took_turn[&monster_id] = true;
                msg_log.log(Msg::StateChange(monster_id, Behavior::Idle));
            } else {
                ai_move_towards_target(target_pos, monster_id, data, msg_log);
            }
        }
    }
}

fn ai_move_towards_target(target_pos: Pos, monster_id: EntityId, data: &mut GameData, msg_log: &mut MsgLog) {
    let monster_pos = data.entities.pos[&monster_id];

    // if the monster has not reached its target, move towards the target.
    let must_reach = false;
    let pos_offset = ai_take_astar_step(monster_id, target_pos, must_reach, &data);
    let move_pos = add_pos(monster_pos, pos_offset);

    if let Some(direction) = Direction::from_positions(monster_pos, move_pos) {
        msg_log.log(Msg::TryMove(monster_id, direction, 1, MoveMode::Walk));
    }
}

pub fn ai_pos_that_hit_target(monster_id: EntityId,
                              target_id: EntityId,
                              data: &mut GameData,
                              config: &Config) -> Vec<Pos> {
    let mut potential_move_targets = Vec::new();

    let target_pos = data.entities.pos[&target_id];
    let monster_pos = data.entities.pos[&monster_id];

    // check all movement options in case one lets us hit the target
    let attack = data.entities.attack[&monster_id];
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

    return potential_move_targets;
}

pub fn ai_fov_cost(monster_id: EntityId,
                   check_pos: Pos,
                   target_pos: Pos,
                   data: &mut GameData,
                   config: &Config) -> usize {
    let monster_pos = data.entities.pos[&monster_id];

    // the fov_cost is added in if the next move would leave the target's FOV
    data.entities.set_pos(monster_id, check_pos);
    let cur_dir = data.entities.direction[&monster_id];
    data.entities.face(monster_id, target_pos);
    let cost =
        if data.pos_in_fov(monster_id, target_pos, config) {
             NOT_IN_FOV_COST
        } else {
            0
        };
    data.entities.direction[&monster_id] = cur_dir;
    data.entities.set_pos(monster_id, monster_pos);

    return cost;
}

pub fn ai_target_pos_cost(monster_id: EntityId,
                          target_id: EntityId,
                          check_pos: Pos,
                          lowest_cost: usize,
                          data: &mut GameData,
                          config: &Config) -> Option<(usize, Pos)> {
    let monster_pos = data.entities.pos[&monster_id];
    let target_pos = data.entities.pos[&target_id];
    let movement = data.entities.movement[&monster_id];

    let mut cost: usize = 0;

    cost += ai_fov_cost(monster_id, check_pos, target_pos, data, config);

    // if the current cost is already higher then the lowest cost found so far,
    // there is no reason to consider this path
    if cost > lowest_cost {
        return None;
    }
    // if the current cost (FOV cost), plus distance (the shortest possible path)
    // if *already* more then the best path so far, this cannot possibly be the best
    // path to take, so skip it
    if cost + distance(monster_pos, check_pos) as usize > lowest_cost {
        return None;
    }

    let must_reach = true;
    let traps_block = true;
    let path = data.path_between(monster_pos, check_pos, movement, must_reach, traps_block, None);

    // paths contain the starting square, so less than 2 is no path at all
    if path.len() < 2 {
        return None;
    }

    cost += path.len();

    let next_pos = path[1];

    return Some((cost, next_pos));
}

pub fn ai_attempt_step(monster_id: EntityId, new_pos: Pos, data: &GameData) -> Option<Pos> {
    let monster_pos = data.entities.pos[&monster_id];

    let pos_offset = ai_take_astar_step(monster_id, new_pos, true, &data);

    let step_pos;
    if pos_mag(pos_offset) > 0 {
        step_pos = Some(add_pos(monster_pos, pos_offset));
    } else {
        step_pos = None;
    }

    return step_pos;
}


pub fn ai_can_hit_target(data: &mut GameData,
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

    // we don't use ai_is_in_fov here because the other checks already
    // cover blocked movement.
    let within_fov = data.pos_in_fov(monster_id, target_pos, config);

    let traps_block = false;

    // both clear_path_up_to and path_blocked_move are used here because
    // clear_path_up_to checks for entities, not including the target pos
    // which contains the player, while path_blocked_move only checks the map
    // up to and including the player pos.
    let clear_path = data.clear_path_up_to(monster_pos, target_pos, traps_block);
    let clear_map = data.entities.attack_type[&monster_id] == AttackType::Ranged ||
                    data.map.path_blocked_move(monster_pos, target_pos).is_none();

    if within_fov && clear_path && clear_map {
        // get all locations they can hit
        let positions: Vec<Pos> = reach.reachables(monster_pos);

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

pub fn ai_move_to_attack_pos(monster_id: EntityId,
                             target_id: EntityId,
                             data: &mut GameData,
                             config: &Config) -> Option<Pos> {
    let monster_pos = data.entities.pos[&monster_id];

    let old_dir = data.entities.direction[&monster_id];

    let mut new_pos = monster_pos;

    let mut potential_move_targets = ai_pos_that_hit_target(monster_id, target_id, data, config);

    // sort by distance to monster to we consider closer positions first, allowing us to
    // skip far away paths we won't take anyway.
    sort_by_distance_to(monster_pos, &mut potential_move_targets);
    let potential_move_targets = potential_move_targets;

    // path_solutions contains the path length, the amount of turning (absolute value), and the
    // next position to go to for this solution.
    let mut path_solutions: Vec<((usize, i32), Pos)> = Vec::new();

    // look through all potential positions for the shortest path
    let mut lowest_cost = std::usize::MAX;
    for target in potential_move_targets {
        let maybe_cost = ai_target_pos_cost(monster_id, target_id, target, lowest_cost, data, config);

        if let Some((cost, next_pos)) = maybe_cost {
            let turn_dir = data.entities.face_to(monster_id, next_pos);
            let turn_amount = old_dir.turn_amount(turn_dir);

            path_solutions.push(((cost, turn_amount.abs()), next_pos));

            lowest_cost = std::cmp::min(lowest_cost, cost);
        }
    }

    // if there is a solution, get the best one and use it
    if let Some(best_sol) = path_solutions.iter().min_by(|a, b| a.0.partial_cmp(&b.0).unwrap()) {
        new_pos = best_sol.1;
    }

    // step towards the closest location that lets us hit the target
    let maybe_pos = ai_attempt_step(monster_id, new_pos, &data);
    return maybe_pos;
}

// NOTE perhaps this should be merged into is_in_fov?
pub fn ai_is_in_fov(monster_id: EntityId, target_id: EntityId, data: &mut GameData, config: &Config) -> bool {
    let monster_pos = data.entities.pos[&monster_id];
    let target_pos = data.entities.pos[&target_id];

    let within_fov = data.pos_in_fov(monster_id, target_pos, config);
    let move_blocked = data.map.path_blocked_move(monster_pos, target_pos);

    if within_fov && move_blocked.is_some() {
        let move_blocked = move_blocked.unwrap();
        let blocked_by_short_wall = move_blocked.wall_type == Wall::ShortWall;
        let target_stance = data.entities.stance[&target_id];

        return blocked_by_short_wall && target_stance != Stance::Crouching;
    } else {
        return within_fov;
    }
}

fn ai_astar_cost(_start: Pos, _prev: Pos, next: Pos, data: &GameData) -> Option<i32> {
    let mut cost = Some(1);

    // check for an armed trap in the list of entities on this tile
    for entity_id in data.has_entities(next) {
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

