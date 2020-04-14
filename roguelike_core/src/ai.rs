use crate::types::*;
use crate::movement::*;
use crate::messaging::*;
use crate::utils::*;
use crate::config::Config;


#[derive(Clone, Copy, Debug, PartialEq)]
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
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();

    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;

    return Pos::new(dx, dy);
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
        // otherwise attempt to move towards their target
        // check positions that can hit target, filter by FOV, and get the closest.
        // then move to this closest position.
        let mut pos_offset = Pos::new(0, 0);
        if let (Some(attack), Some(movement)) =
            (data.objects[monster_id].attack, data.objects[monster_id].movement) {
            // get all locations they can hit
            let move_positions =
                Direction::move_actions().iter()
                                         .map(|move_action| movement.move_with_reach(move_action))
                                         .filter_map(|mov| mov)
                                         .map(|pos| add_pos(pos, monster_pos))
                                         .filter(|pos| data.map.is_within_bounds(*pos))
                                         .filter(|pos| data.is_blocked_tile(*pos).is_none())
                                         .collect::<Vec<Pos>>();

            // filter locations that are blocked or out of sight
            // NOTE hack to get positions to work out...
            let positions: Vec<Pos> =
                move_positions
                .iter()
                .filter(|new_pos| {
                    data.objects[monster_id].set_pos(**new_pos);
                    let can_hit = ai_can_hit_target(data, monster_id, target_pos, &attack, config).is_some();
                    data.objects[monster_id].set_pos(monster_pos);
                    return can_hit;
                })
                .map(|pair| *pair)
                .collect();

            // if there are any options to move to that will allow the monster to
            // attack, move to the one closest to their current position.
            if positions.len() > 0 {
                target_pos = positions.iter()
                                      .min_by_key(|pos| distance(target_pos, Pos::from(**pos)))
                                      .map(|pair| Pos::from(*pair))
                                      .unwrap();
            }

            pos_offset = ai_take_astar_step(monster_pos, target_pos, &data);
        }

        turn = Action::Move(Movement::move_to(add_pos(monster_pos, pos_offset), MoveType::Move));
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

    let turn: Action;

               
    if game_data.objects[monster_id].is_in_fov(&mut game_data.map, player_pos, config) {
        game_data.objects[monster_id].face(player_pos);
        // TODO this causes a turn delay between seeing the player and attacking them
        turn = Action::StateChange(Behavior::Attacking(player_id));
    } else { // the monster can't see the player
        // TODO AI reached here - heard sound, but needs to face position so they can see the
        // player
        // SOUND POSITIONS SET IN LIB SHOULD USE ORIGIN, NOT CURRENT POSITION!
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

    return turn;
}

fn ai_can_hit_target(data: &mut GameData,
                     monster_id: ObjectId,
                     target_pos: Pos,
                     reach: &Reach,
                     config: &Config) -> Option<Pos> {
    let mut hit_pos = None;
    let monster_pos = data.objects[monster_id].pos();

    let within_fov =
        data.objects[monster_id].is_in_fov(&mut data.map, target_pos, config);

    if within_fov {
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
                      game_data: &GameData) -> Pos {
    let astar_iter = game_data.map.astar(monster_pos, target_pos);

    if astar_iter.len() > 1 && !game_data.is_blocked_tile(astar_iter[1]).is_some() {
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

                    msg_log.log(Msg::Moved(monster_id, movement, movement.pos));
                }

                Some(Attack::Attack(target_id)) => {
                    let pos_diff = sub_pos(movement.pos, pos);

                    // ensure that attacking changes the orientation of an entity
                    game_data.objects[monster_id].direction =
                        Direction::from_dxy(pos_diff.x, pos_diff.y);

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

