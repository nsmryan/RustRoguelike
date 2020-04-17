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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Behavior {
    Idle,
    Investigating(Pos),
    Attacking(ObjectId),
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

pub fn ai_take_turn(monster_id: ObjectId,
                    data: &mut GameData,
                    config: &Config,
                    msg_log: &mut MsgLog) {
    let mut turn: Action;

    match data.objects[monster_id].ai {
        Some(Ai::Basic) => {
            turn = basic_ai_take_turn(monster_id, data, config);

            // if the AI changes state, allow it to take an action as well
            if matches!(turn, Action::StateChange(_)) {
                turn = basic_ai_take_turn(monster_id, data, config);
            }
        }

        None => {
            turn = Action::none();
        }
    }

    ai_apply_actions(monster_id,
                     turn,
                     data,
                     msg_log);
}

pub fn step_towards(start_pos: Pos, target_pos: Pos) -> Pos {
    let dx = target_pos.x - start_pos.x;
    let dy = target_pos.y - start_pos.y;
    let delta_pos = Pos::new(signedness(dx), signedness(dy));
    return delta_pos;
}

pub fn ai_attack(monster_id: ObjectId,
                 target_id: ObjectId,
                 data: &mut GameData,
                 config: &Config) -> Action {
    let mut target_pos = data.objects[target_id].pos();
    let monster_pos = data.objects[monster_id].pos();

    let turn: Action;

    if !data.objects[target_id].alive {
        turn = Action::StateChange(Behavior::Investigating(target_pos));
    } else if let Some(hit_pos) =
        // if AI can hit their target
        ai_can_hit_target(data, 
                          monster_id,
                          target_pos,
                          &data.objects[monster_id].attack.unwrap(),
                          config) {
        let attack = Attack::Attack(target_id);
        turn = Action::Move(Movement::attack(hit_pos, MoveType::Move, attack));
    } else if data.map.is_blocked_by_wall(monster_pos, target_pos.x - monster_pos.x, target_pos.y - monster_pos.y).is_some() {
        turn = Action::StateChange(Behavior::Investigating(target_pos));
    } else {
        let mut new_pos = monster_pos;

        let mut pos_offset = Pos::new(0, 0);
        if let (Some(attack), Some(movement)) =
            (data.objects[monster_id].attack, data.objects[monster_id].movement) {

            let mut potential_move_targets = Vec::new();

            let direction = data.objects[monster_id].direction;
            for move_action in Direction::move_actions() {
                for attack_offset in attack.attacks_with_reach(&move_action) {
                    let attackable_pos = add_pos(target_pos, attack_offset);

                    if attackable_pos == monster_pos {
                        continue;
                    }

                    data.objects[monster_id].set_pos(attackable_pos);
                    data.objects[monster_id].face(target_pos);
                    let can_hit = ai_can_hit_target(data, monster_id, target_pos, &attack, config).is_some();

                    dbg!(attackable_pos, can_hit);
                    if can_hit {
                        potential_move_targets.push(attackable_pos);
                    }
                }
            }
            data.objects[monster_id].set_pos(monster_pos);
            data.objects[monster_id].direction = direction;

            let mut targets = potential_move_targets.iter();
            if let Some(first_target) = targets.next() {
                let mut best_target = first_target;

                let path = data.path_between(monster_pos, *best_target, movement);
                let mut best_dist = path.len();

                let large_dist = (MAP_WIDTH + MAP_HEIGHT) as usize;
                if best_dist == 0 {
                    best_dist = large_dist;
                }

                for move_target in targets {
                    let path = data.path_between(monster_pos, *move_target, movement);
                    let path_length = path.len();
                    dbg!(move_target, path_length);
                        
                    if path_length > 0 && (path_length < best_dist || best_dist == large_dist) {
                        best_dist = path_length;
                        best_target = move_target;
                    }
                }

                if best_dist > 0 && best_dist != large_dist {
                    new_pos = *best_target;
                }

                dbg!(best_target, best_dist);
            }

            dbg!(new_pos);

            pos_offset = ai_take_astar_step(monster_pos, new_pos, &data);
        }

        dbg!(pos_offset);

        if pos_mag(pos_offset) > 0 {
            turn = Action::Move(Movement::move_to(add_pos(monster_pos, pos_offset), MoveType::Move));
        } else {
            turn = Action::NoAction;
        }
    }

    return turn;
}

pub fn ai_investigate(target_pos_orig: Pos, 
                      monster_id: ObjectId,
                      game_data: &mut GameData,
                      config: &Config) -> Action {
    let player_id = game_data.find_player().unwrap();

    let target_pos = target_pos_orig;
    let player_pos = game_data.objects[player_id].pos();
    let monster_pos = game_data.objects[monster_id].pos();

    let mut turn: Action;
               
    if game_data.objects[monster_id].is_in_fov(&mut game_data.map, player_pos, config) {
        game_data.objects[monster_id].face(player_pos);
        turn = Action::StateChange(Behavior::Attacking(player_id));
    } else { // the monster can't see the player
        if let Some(Message::Sound(_entity_id, pos)) = game_data.objects[monster_id].heard_sound() {
            game_data.objects[monster_id].behavior =
                Some(Behavior::Investigating(pos));
        }

        if target_pos == monster_pos { 
            // if the monster reached its target then go back to being idle
            turn = Action::StateChange(Behavior::Idle);
        } else {
            // if the monster has not reached its target, move towards the target.
            let pos_offset = ai_take_astar_step(monster_pos, target_pos, &game_data);

            let movement = Movement::move_to(add_pos(monster_pos, pos_offset), MoveType::Move);
            turn = Action::Move(movement);
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
                     monster_id: ObjectId,
                     target_pos: Pos,
                     reach: &Reach,
                     config: &Config) -> Option<Pos> {
    let mut hit_pos = None;
    let monster_pos = data.objects[monster_id].pos();

    // don't allow hitting from the same tile...
    if target_pos == monster_pos {
        return None;
    }

    let within_fov =
        data.objects[monster_id].is_in_fov(&mut data.map, target_pos, config);

    // check that there is a clear path right up to the target position
    let next_to_tile = move_next_to(monster_pos, target_pos);
    let clear_path = data.clear_path(monster_pos, next_to_tile);

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

fn ai_take_astar_step(monster_pos: Pos,
                      target_pos: Pos,
                      data: &GameData,
                      reach: Reach) -> Pos {
    let path = data.path_between(monster_pos, *best_target, reach);

    if astar_iter.len() > 1 {
        return step_towards(monster_pos, astar_iter[1]);
    } else {
        return Pos::new(0, 0);
    }
}

// NOTE this function takes a mutable GameData because FOV requires
// mutation under the hood. It does not otherwise modify the game
pub fn basic_ai_take_turn(monster_id: ObjectId,
                          game_data: &mut GameData,
                          config: &Config) -> Action {
    let player_id = game_data.find_player().unwrap();
    let monster_pos = game_data.objects[monster_id].pos();
    let player_pos = game_data.objects[player_id].pos();

    if game_data.map.is_within_bounds(monster_pos) {
        match game_data.objects[monster_id].behavior {
            Some(Behavior::Idle) => {
                let mut turn = Action::none();

                if game_data.objects[monster_id].is_in_fov(&mut game_data.map, player_pos, config) {
                    game_data.objects[monster_id].face(player_pos);
                    // NOTE will cause a turn between seeing the player and attacking
                    turn = Action::StateChange(Behavior::Attacking(player_id));
                } else if let Some(Message::Sound(_entity_id, pos)) = game_data.objects[monster_id].heard_sound() {
                    game_data.objects[monster_id].face(player_pos);
                    turn = Action::StateChange(Behavior::Investigating(pos));
                } else if let Some(Message::Attack(entity_id)) = game_data.objects[monster_id].was_attacked() {
                    game_data.objects[monster_id].face(player_pos);
                    turn = Action::StateChange(Behavior::Attacking(entity_id));
                }

                return turn;
            }

            Some(Behavior::Investigating(target_pos)) => {
                return ai_investigate(target_pos, monster_id, game_data, config);
            }

            Some(Behavior::Attacking(object_id)) => {
                return ai_attack(monster_id, object_id, game_data, config);
            }

            behavior => {
                panic!("Ai behavior {:?} unexpected!", behavior);
            }
        }
    } else {
        // position outside of map- return empty turn
        return Action::none();
    }
}

pub fn ai_apply_actions(monster_id: ObjectId,
                        turn: Action,
                        game_data: &mut GameData,
                        msg_log: &mut MsgLog) {
    let pos = game_data.objects[monster_id].pos();

    match turn {
        Action::Move(movement) => {
            match movement.attack {
                None => {
                    game_data.objects[monster_id].move_to(movement.pos);

                    if let Some(Behavior::Attacking(target_id)) = game_data.objects[monster_id].behavior {
                        let target_pos = game_data.objects[target_id].pos();
                        game_data.objects[monster_id].face(target_pos);
                    }

                    msg_log.log(Msg::Moved(monster_id, movement, movement.pos));
                }

                Some(Attack::Attack(target_id)) => {
                    let pos_diff = sub_pos(movement.pos, pos);

                    attack(monster_id, target_id, game_data, msg_log);
                },

                _ => panic!("Unexpected movement!"),
            }
        },

        Action::StateChange(behavior) => {
            game_data.objects.get_mut(monster_id).unwrap().behavior = Some(behavior);

            msg_log.log(Msg::StateChange(monster_id, behavior));
        },

        _ => {
        }
    }
}

