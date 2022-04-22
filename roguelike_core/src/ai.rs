use std::fmt;

use serde::{Serialize, Deserialize};

use roguelike_utils::comp::*;

use crate::constants::*;
use crate::types::*;
use crate::movement::*;
use crate::messaging::*;
use crate::utils::*;
use crate::config::Config;
use crate::map::{FovResult, Pos, Direction};
use crate::level::*;


#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Ai {
    Basic,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Behavior {
    Idle,
    Alert(Pos),
    Investigating(Pos),
    Attacking(EntityId),
}

impl fmt::Display for Behavior {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Behavior::Idle => write!(f, "idle"),
            Behavior::Alert(pos) => write!(f, "alert {} {}", pos.x, pos.y),
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
    pub fn description(&self) -> &str {
        match self {
            Behavior::Idle => "idle",
            Behavior::Alert(_pos) => "alert",
            Behavior::Investigating(_position) => "investigating",
            Behavior::Attacking(_entity_id) => "attacking",
        }
    }

    pub fn is_aware(&self) -> bool {
        return matches!(self, Behavior::Attacking(_));
    }
}

pub fn ai_take_turn(monster_id: EntityId,
                    level: &mut Level,
                    config: &Config,
                    msg_log: &mut MsgLog) {
    if level.entities.status[&monster_id].alive {
        match level.entities.ai.get(&monster_id) {
            Some(Ai::Basic) => {
                basic_ai_take_turn(monster_id, level, msg_log, config);
            }

            None => {
                panic!("AI didn't have an ai entry!");
            }
        }
    }
}

pub fn basic_ai_take_turn(monster_id: EntityId,
                          level: &mut Level,
                          msg_log: &mut MsgLog,
                          config: &Config) {
    let monster_pos = level.entities.pos[&monster_id];

    if level.map.is_within_bounds(monster_pos) {
        if level.entities.status[&monster_id].frozen == 0 {
            match level.entities.behavior[&monster_id] {
                Behavior::Idle => {
                    ai_idle(monster_id, level, msg_log, config);
                }

                Behavior::Alert(pos) => {
                    ai_alert(monster_id, pos, level, msg_log, config);
                }

                Behavior::Investigating(target_pos) => {
                    ai_investigate(target_pos, monster_id, level, msg_log, config);
                }

                Behavior::Attacking(entity_id) => {
                    ai_attack(monster_id, entity_id, level, msg_log, config);
                }
            }
        }
    }
}

pub fn ai_alert(monster_id: EntityId,
                _target_pos: Pos,
                level: &mut Level,
                msg_log: &mut MsgLog,
                _config: &Config) {
    let player_id = level.find_by_name(EntityName::Player).unwrap();
    let can_see_target = level.is_in_fov(monster_id, player_id) == FovResult::Inside;

    if can_see_target {
        // Can see target- attack
        msg_log.log(Msg::StateChange(monster_id, Behavior::Attacking(player_id)));
    } else {
        // Can't see target- investigate their last position.
        let player_pos = level.entities.pos[&player_id];
        msg_log.log(Msg::StateChange(monster_id, Behavior::Investigating(player_pos)));
    }
}

pub fn ai_attack(monster_id: EntityId,
                 target_id: EntityId,
                 level: &mut Level,
                 msg_log: &mut MsgLog,
                 _config: &Config) {
    let target_pos = level.entities.pos[&target_id];

    // we need to turn towards the target first, so the
    // rest of the processing is done in the AIAttack message
    msg_log.log(Msg::FaceTowards(monster_id, target_pos));
    msg_log.log(Msg::AiAttack(monster_id));
}

pub fn ai_idle(monster_id: EntityId,
               level: &mut Level,
               msg_log: &mut MsgLog,
               config: &Config) {
    let player_id = level.find_by_name(EntityName::Player).unwrap();

    if config.sound_golem_idle_radius > 0 {
        let monster_pos = level.entities.pos[&monster_id];
        msg_log.log(Msg::Sound(monster_id, monster_pos, config.sound_golem_idle_radius));
    }

    if level.is_in_fov(monster_id, player_id) == FovResult::Inside {
        let player_pos = level.entities.pos[&player_id];
        msg_log.log(Msg::FaceTowards(monster_id, player_pos));

        if level.entities.attack.get(&monster_id).is_some() {
            msg_log.log(Msg::StateChange(monster_id, Behavior::Alert(player_pos)));
            level.entities.took_turn[&monster_id] = true;
        } else {
            msg_log.log(Msg::StateChange(monster_id, Behavior::Investigating(player_pos)));
        }
    } else if let Some(Message::Attack(entity_id)) = level.entities.was_attacked(monster_id) {
        let entity_pos = level.entities.pos[&entity_id];
        msg_log.log(Msg::FaceTowards(monster_id, entity_pos));

        if level.entities.attack.get(&monster_id).is_some() {
            msg_log.log(Msg::StateChange(monster_id, Behavior::Attacking(entity_id)));
        } else {
            msg_log.log(Msg::StateChange(monster_id, Behavior::Investigating(entity_pos)));
        }
    } else if let Some(Message::Hit(origin_pos)) = level.entities.was_hit(monster_id) {
        msg_log.log(Msg::FaceTowards(monster_id, origin_pos));
        msg_log.log(Msg::StateChange(monster_id, Behavior::Alert(origin_pos)));
    } else if let Some(Message::Sound(sound_pos)) = level.entities.heard_sound(monster_id) {
        let can_see = level.pos_in_fov(monster_id, sound_pos);

        let caused_by_golem = level.get_golem_at_pos(sound_pos).is_some();
        let needs_investigation = !(can_see && caused_by_golem);

        // Don't investigate a sound caused by another golem that you can see.
        if needs_investigation {
            msg_log.log(Msg::FaceTowards(monster_id, sound_pos));
            msg_log.log(Msg::StateChange(monster_id, Behavior::Investigating(sound_pos)));
        }
    }
}

pub fn ai_investigate(target_pos: Pos, 
                      monster_id: EntityId,
                      level: &mut Level,
                      msg_log: &mut MsgLog,
                      _config: &Config) {
    let player_id = level.find_by_name(EntityName::Player).unwrap();

    let monster_pos = level.entities.pos[&monster_id];

    let player_in_fov = level.is_in_fov(monster_id, player_id) == FovResult::Inside;

    if player_in_fov {
        //let fov_path_clear = level.map.path_blocked_fov(monster_pos, player_pos).is_none();
        let player_pos = level.entities.pos[&player_id];
        msg_log.log(Msg::FaceTowards(monster_id, player_pos));

        if level.entities.attack.get(&monster_id).is_some() {
            msg_log.log(Msg::StateChange(monster_id, Behavior::Attacking(player_id)));
        } else {
            // if the monster cannot attack, just keep walking towards the target.
            ai_move_towards_target(player_pos, monster_id, level, msg_log);

            level.entities.took_turn[&monster_id] = true;
            msg_log.log(Msg::StateChange(monster_id, Behavior::Investigating(player_pos)));
        }
    } else { // the monster can't see the player
        if let Some(Message::Attack(entity_id)) = level.entities.was_attacked(monster_id) {
            let entity_pos = level.entities.pos[&entity_id];
            // Just face towards the attacker. We can act on this on the next turn.
            msg_log.log(Msg::FaceTowards(monster_id, entity_pos));

            // NOTE Removed so we only face towards the attacker.
            //if level.entities.attack.get(&monster_id).is_some() {
            //    msg_log.log(Msg::StateChange(monster_id, Behavior::Attacking(entity_id)));
            //} else {
            //    msg_log.log(Msg::StateChange(monster_id, Behavior::Investigating(entity_pos)));
            //}
        } else if let Some(Message::Hit(origin_pos)) = level.entities.was_hit(monster_id) {
            msg_log.log(Msg::FaceTowards(monster_id, origin_pos));
            msg_log.log(Msg::StateChange(monster_id, Behavior::Investigating(origin_pos)));
        } else if let Some(Message::Sound(sound_pos)) = level.entities.heard_sound(monster_id) {
            let can_see = level.pos_in_fov(monster_id, sound_pos);

            let caused_by_golem = level.get_golem_at_pos(sound_pos).is_some();
            let needs_investigation = !(can_see && caused_by_golem);

            // Only investigate if: we can't see the tile, or we can see it and there is not
            // already a golem there.
            // This prevents golems from following each other around when they should realize
            // that a sound is caued by another golem
            if needs_investigation {
                msg_log.log(Msg::StateChange(monster_id, Behavior::Investigating(sound_pos)));
            }
        } else {
            // If the golem reached the target, they become idle.
            // If they are next to the target, and it is occupied, they also become idle,
            // but face towards the target in case they aren't already.
            // Otherwise they attempt to step towards the target position.
            let nearly_reached_target = distance(target_pos, monster_pos) == 1 && level.pos_blocked(target_pos);
            let reached_target = target_pos == monster_pos;
            if reached_target || nearly_reached_target {
                if nearly_reached_target {
                    msg_log.log(Msg::FaceTowards(monster_id, target_pos));
                }

                // monster reached their target position
                level.entities.took_turn[&monster_id] = true;
                msg_log.log(Msg::StateChange(monster_id, Behavior::Idle));
            } else {
                ai_move_towards_target(target_pos, monster_id, level, msg_log);
            }
        }
    }
}

fn ai_move_towards_target(target_pos: Pos, monster_id: EntityId, level: &mut Level, msg_log: &mut MsgLog) {
    let monster_pos = level.entities.pos[&monster_id];

    // if the monster has not reached its target, move towards the target.
    let must_reach = false;
    let pos_offset = ai_take_astar_step(monster_id, target_pos, must_reach, &level);
    let move_pos = add_pos(monster_pos, pos_offset);

    if let Some(direction) = Direction::from_positions(monster_pos, move_pos) {
        msg_log.log(Msg::TryMove(monster_id, direction, 1, MoveMode::Walk));
    }
}

pub fn ai_pos_that_hit_target(monster_id: EntityId,
                              target_id: EntityId,
                              level: &mut Level,
                              config: &Config) -> Vec<Pos> {
    let mut potential_move_targets = Vec::new();

    let target_pos = level.entities.pos[&target_id];
    let monster_pos = level.entities.pos[&monster_id];

    // check all movement options in case one lets us hit the target
    let attack = level.entities.attack[&monster_id];
    let direction = level.entities.direction[&monster_id];
    for move_action in &Direction::move_actions() {
        for attack_offset in attack.attacks_with_reach(&move_action) {
            let attackable_pos = add_pos(target_pos, attack_offset);

            if attackable_pos == monster_pos ||
               !level.map.is_within_bounds(attackable_pos) {
                continue;
            }

            level.entities.set_pos(monster_id, attackable_pos);
            level.entities.face(monster_id, target_pos);
            let can_hit = ai_can_hit_target(level, monster_id, target_pos, &attack, config).is_some();

            if can_hit {
                potential_move_targets.push(attackable_pos);
            }
        }
    }
    level.entities.set_pos(monster_id, monster_pos);
    level.entities.direction[&monster_id] = direction;

    return potential_move_targets;
}

pub fn ai_fov_cost(monster_id: EntityId,
                   check_pos: Pos,
                   target_pos: Pos,
                   level: &mut Level,
                   _config: &Config) -> usize {
    let monster_pos = level.entities.pos[&monster_id];

    // the fov_cost is added in if the next move would leave the target's FOV
    level.entities.set_pos(monster_id, check_pos);
    let cur_dir = level.entities.direction[&monster_id];
    level.entities.face(monster_id, target_pos);
    let cost =
        if level.pos_in_fov(monster_id, target_pos) {
             NOT_IN_FOV_COST
        } else {
            0
        };
    level.entities.direction[&monster_id] = cur_dir;
    level.entities.set_pos(monster_id, monster_pos);

    return cost;
}

pub fn ai_target_pos_cost(monster_id: EntityId,
                          target_id: EntityId,
                          check_pos: Pos,
                          lowest_cost: usize,
                          level: &mut Level,
                          config: &Config) -> Option<(usize, Pos)> {
    let monster_pos = level.entities.pos[&monster_id];
    let target_pos = level.entities.pos[&target_id];
    let movement = level.entities.movement[&monster_id];

    let mut cost: usize = 0;

    cost += ai_fov_cost(monster_id, check_pos, target_pos, level, config);

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
    let path = level.path_between(monster_pos, check_pos, movement, must_reach, traps_block, None);

    // paths contain the starting square, so less than 2 is no path at all
    if path.len() < 2 {
        return None;
    }

    cost += path.len();

    let next_pos = path[1];

    return Some((cost, next_pos));
}

pub fn ai_attempt_step(monster_id: EntityId, new_pos: Pos, level: &Level) -> Option<Pos> {
    let monster_pos = level.entities.pos[&monster_id];

    let pos_offset = ai_take_astar_step(monster_id, new_pos, true, &level);

    let step_pos;
    if pos_mag(pos_offset) > 0 {
        step_pos = Some(add_pos(monster_pos, pos_offset));
    } else {
        step_pos = None;
    }

    return step_pos;
}


pub fn ai_can_hit_target(level: &mut Level,
                         monster_id: EntityId,
                         target_pos: Pos,
                         reach: &Reach,
                         _config: &Config) -> Option<Pos> {
    let mut hit_pos = None;
    let monster_pos = level.entities.pos[&monster_id];

    // don't allow hitting from the same tile...
    if target_pos == monster_pos {
        return None;
    }

    // we don't use is_in_fov here because the other checks already
    // cover blocked movement.
    let within_fov = level.pos_in_fov(monster_id, target_pos);

    let traps_block = false;

    // both clear_path_up_to and path_blocked_move are used here because
    // clear_path_up_to checks for entities, not including the target pos
    // which contains the player, while path_blocked_move only checks the map
    // up to and including the player pos.
    let clear_path = level.clear_path_up_to(monster_pos, target_pos, traps_block);
    let clear_map = level.entities.attack_type[&monster_id] == AttackType::Ranged ||
                    level.map.path_blocked_move(monster_pos, target_pos).is_none();

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
                             level: &mut Level,
                             config: &Config) -> Option<Pos> {
    let monster_pos = level.entities.pos[&monster_id];

    let old_dir = level.entities.direction[&monster_id];

    let mut new_pos = monster_pos;

    let mut potential_move_targets = ai_pos_that_hit_target(monster_id, target_id, level, config);

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
        let maybe_cost = ai_target_pos_cost(monster_id, target_id, target, lowest_cost, level, config);

        if let Some((cost, next_pos)) = maybe_cost {
            let turn_dir = level.entities.face_to(monster_id, next_pos);
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
    let maybe_pos = ai_attempt_step(monster_id, new_pos, &level);
    return maybe_pos;
}

// NOTE perhaps this should be merged into is_in_fov?
//pub fn ai_is_in_fov(monster_id: EntityId, target_id: EntityId, level: &mut Level, _config: &Config) -> bool {
//    let monster_pos = level.entities.pos[&monster_id];
//    let target_pos = level.entities.pos[&target_id];
//
//    let within_fov = level.pos_in_fov(monster_id, target_pos);
//    let move_blocked = level.map.path_blocked_move(monster_pos, target_pos);
//
//    if within_fov && move_blocked.is_some() {
//        let move_blocked = move_blocked.unwrap();
//        let blocked_by_short_wall = move_blocked.wall_type == Wall::ShortWall;
//        let target_stance = level.entities.stance[&target_id];
//
//        return blocked_by_short_wall && target_stance != Stance::Crouching;
//    } else {
//        return within_fov;
//    }
//}

fn ai_astar_cost(_start: Pos, _prev: Pos, next: Pos, level: &Level) -> Option<i32> {
    let mut cost = Some(1);

    // check for an armed trap in the list of entities on this tile
    for entity_id in level.has_entities(next) {
        if level.entities.trap.get(&entity_id).is_some() &&
           level.entities.armed.get(&entity_id) == Some(&true) {
               // NOTE determined randomly. could be infinite, or smaller?
               cost = None;
        }
    }

    return cost;
}

fn ai_astar_step(monster_id: EntityId,
                 target_pos: Pos,
                 must_reach: bool,
                 level: &Level) -> Vec<Pos> {
    let reach = level.entities.movement[&monster_id];
    let monster_pos = level.entities.pos[&monster_id];

    let traps_block = true;

    let path = level.path_between(monster_pos, target_pos, reach, must_reach, traps_block, Some(ai_astar_cost));

    return path;
}

fn ai_take_astar_step(monster_id: EntityId,
                      target_pos: Pos,
                      must_reach: bool,
                      level: &Level) -> Pos {
    let path = ai_astar_step(monster_id, target_pos, must_reach, level);

    if path.len() > 1 {
        let monster_pos = level.entities.pos[&monster_id];
        return step_towards(monster_pos, path[1]);
    } else {
        return Pos::new(0, 0);
    }
}

