use std::ops::Add;

use crate::map::*;
use crate::types::*;
use crate::constants::*;
use crate::movement::*;


pub fn move_by(handle: ObjectId, dx: i32, dy: i32, data: &mut GameData) {
    let pos = data.objects[handle].pos();

    if !is_blocked(Pos::new(pos.x + dx, pos.y + dy), data) {
        data.objects[handle].set_pos(pos.x + dx, pos.y + dy);
    }
}

// TOOD move to a utils module
pub fn add_pos(pos1: Pos, pos2: Pos) -> Pos {
    return Pos::new(pos1.x + pos2.x, pos1.y + pos2.y);
}

pub fn step_towards(start_pos: Pos, target_pos: Pos) -> Pos {
    let dx = target_pos.x - start_pos.x;
    let dy = target_pos.y - start_pos.y;
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();

    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;

    return Pos::new(dx, dy);
}

pub fn ai_attack(monster_handle: ObjectId,
                 target_handle: ObjectId,
                 data: &mut GameData) -> Action {
    let target_pos = data.objects[target_handle].pos();
    let monster_pos = data.objects[monster_handle].pos();

    let turn: Action;

    // if AI can hit their target
    if let Some(hit_pos) =
        ai_can_hit_target(&mut data.map, 
                          data.objects[monster_handle].pos(),
                          data.objects[target_handle].pos(),
                          &data.objects[monster_handle].attack.unwrap()) {
        turn = Action::Attack(target_handle, hit_pos);
    } else if data.map.is_blocked_by_wall(monster_pos, target_pos.x - monster_pos.x, target_pos.y - monster_pos.y) {
        turn = Action::StateChange(Behavior::Investigating(target_pos));
    } else { // otherwise attempt to move towards their target
        // check positions that can hit target, filter by FOV, and get the closest.
        // then move to this closest position.
        let mut pos_offset = Pos::new(0, 0);
        if let (Some(attack), Some(movement)) =
            (data.objects[monster_handle].attack, data.objects[monster_handle].movement) {
            // get all locations they can hit
            let move_positions =
                MoveAction::move_actions().iter()
                                          .map(|move_action| movement.move_with_reach(move_action))
                                          .filter_map(|mov| mov)
                                          .map(|mov| add_pos(mov, monster_pos))
                                          .filter(|mov| data.map.is_within_bounds(*mov))
                                          .collect::<Vec<Pos>>();

            // filter locations that are blocked or out of sight
            let positions: Vec<Pos> =
                move_positions
                .iter()
                .filter(|new_pos| ai_can_hit_target(&mut data.map, **new_pos, target_pos, &attack).is_some())
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

            pos_offset = ai_take_astar_step(monster_pos, target_pos, &data.map);
        }

        turn = Action::Move(Movement::Move(pos_offset));
    }

    return turn;
}

pub fn ai_investigate(target_pos_orig: Pos, 
                      monster_handle: ObjectId,
                      data: &mut GameData) -> Action {
    let player_handle = data.find_player().unwrap();

    let mut target_pos = target_pos_orig;
    let player_pos = data.objects[player_handle].pos();
    let monster_pos = data.objects[monster_handle].pos();

    let turn: Action;

               
    if data.map.is_in_fov(monster_pos, player_pos, MONSTER_VIEW_DIST) {
        // TODO this causes a turn delay between seeing the player and attacking them
        turn = Action::StateChange(Behavior::Attacking(player_handle));
    } else { // the monster can't see the player
        if let Some(sound_id) = data.sound_within_earshot(monster_pos) {
            target_pos = Pos::new(data.objects[sound_id].x, data.objects[sound_id].y);
            data.objects[monster_handle].behavior =
                Some(Behavior::Investigating(target_pos));
        }

        if target_pos == monster_pos { 
            // if the monster reached its target then go back to being idle
            turn = Action::StateChange(Behavior::Idle);
        } else {
            // if the monster has not reached its target, move towards the target.
            let pos_offset = ai_take_astar_step(monster_pos, target_pos, &data.map);

            turn = Action::Move(Movement::Move(pos_offset));
        }
    }

    return turn;
}

fn ai_can_hit_target(map: &mut Map,
                     monster_pos: Pos,
                     target_pos: Pos,
                     reach: &Reach) -> Option<Pos> {
    let mut hit_pos = None;

    let within_fov =
        map.is_in_fov(monster_pos,
                      target_pos,
                      MONSTER_VIEW_DIST);

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
                      map: &Map) -> Pos {
    let astar_iter = map.astar(monster_pos, target_pos);

    if astar_iter.len() > 0 {
        return step_towards(monster_pos, astar_iter[1]);
    } else {
        return Pos::new(0, 0);
    }
}

pub fn basic_ai_take_turn(monster_handle: ObjectId,
                          data: &mut GameData) -> Action {
    let player_handle = data.find_player().unwrap();
    let monster_pos = data.objects[monster_handle].pos();
    let player_pos = data.objects[player_handle].pos();

    if data.map.is_within_bounds(monster_pos) {
        match data.objects[monster_handle].behavior {
            Some(Behavior::Idle) => {
                let mut turn = Action::none();

                if data.map.is_in_fov(monster_pos, player_pos, MONSTER_VIEW_DIST) {
                    // NOTE will cause a turn between seeing the player and attacking
                    turn = Action::StateChange(Behavior::Attacking(player_handle));
                } else if let Some(sound_id) = data.sound_within_earshot(monster_pos) {
                    let sound_position = Pos::new(data.objects[sound_id].x, data.objects[sound_id].y);
                    turn = Action::StateChange(Behavior::Investigating(sound_position));
                }

                return turn;
            }

            Some(Behavior::Investigating(target_pos)) => {
                return ai_investigate(target_pos, monster_handle, data);
            }

            Some(Behavior::Attacking(object_handle)) => {
                return ai_attack(monster_handle, object_handle, data);
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

pub fn ai_take_turn(monster_handle: ObjectId, data: &mut GameData) {
    let turn: Action;

    match data.objects[monster_handle].ai {
        Some(Ai::Basic) => {
            turn = basic_ai_take_turn(monster_handle, data);
        }

        None => {
            turn = Action::none();
        }
    }

    ai_apply_actions(monster_handle,
                     turn,
                     data);
}

pub fn ai_apply_actions(monster_handle: ObjectId,
                        turn: Action,
                        data: &mut GameData) {
    match turn {
        Action::Move(movement) => {
            match movement {
                Movement::Move(pos) => {
                    move_by(monster_handle, pos.x, pos.y, data);
                }

                _ => panic!("Unexpected movement!"),
            }
        },

        Action::Attack(target_handle, _pos) => {
            //let (target, monster) = mut_two(*target_handle, monster_handle, &mut data.objects);

            // apply attack 
            attack(monster_handle, target_handle, &mut data.objects);
        },

        Action::StateChange(behavior) => {
            data.objects.get_mut(monster_handle).unwrap().behavior = Some(behavior);
        },

        _ => {
        }
    }
}

pub fn attack(handle: ObjectId, other_handle: ObjectId, objects: &mut ObjMap) {
    let damage = objects[handle].fighter.map_or(0, |f| f.power) -
                 objects[other_handle].fighter.map_or(0, |f| f.defense);
    if damage > 0 {
        objects[other_handle].take_damage(damage);
    }
}

