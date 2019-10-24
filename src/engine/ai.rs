#[allow(unused_imports)]use std::cmp;

#[allow(unused_imports)]use tcod::map::{Map as FovMap};
#[allow(unused_imports)]use tcod::pathfinding::*;

use crate::constants::*;
use crate::engine::types::*;
use crate::engine::map::*;



pub fn location_within_fov(fov_map: &FovMap, monster_pos: Position, player_pos: Position) -> bool {
    let within_fov = fov_map.is_in_fov(monster_pos.0, monster_pos.1);
    let within_sight_range = player_pos.distance(&monster_pos) <= MONSTER_VIEW_DIST;

    return within_fov && within_sight_range;
}

pub fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut [Object]) {
    let (x, y) = objects[id].pos();

    if !map.is_blocked(x + dx, y + dy, objects){
        objects[id].set_pos(x + dx, y + dy);
    }
}

pub fn step_towards(start_pos: (i32, i32), target_pos: (i32, i32)) -> (i32, i32) {
    let dx = target_pos.0 - start_pos.0;
    let dy = target_pos.1 - start_pos.1;
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();

    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;

    return (dx, dy);
}

pub fn ai_attack(monster_id: usize,
                 target_id: usize,
                 map: &Map,
                 objects: &Vec<Object>,
                 fov_map: &FovMap) -> AiTurn {
    let (target_x, target_y) = objects[target_id].pos();
    let mut target_pos = Position::new(target_x, target_y);

    let (monster_x, monster_y) = objects[monster_id].pos();
    let monster_pos = Position::new(monster_x, monster_y);

    let mut turn: AiTurn = AiTurn::new();

    if let Some(hit_pos) = ai_can_hit_target(objects[monster_id].pos(), objects[target_id].pos(), &objects[monster_id].attack.unwrap()) {
        turn.add(AiAction::Attack(target_id, hit_pos));
    } else {
        // check positions that can hit target, filter by FOV, and get the closest.
        // then move to this closest position.
        let mut pos_offset = (0, 0);
        if let (Some(attack), Some(movement)) = (objects[monster_id].attack, objects[monster_id].movement) {
            // get all locations they can hit
            let move_positions =
                MoveAction::move_actions().iter()
                                          .map(|move_action| movement.move_with_reach(move_action))
                                          .filter_map(|mov| mov)
                                          .map(|mov| mov.add(monster_pos).into_pair())
                                          .collect::<Vec<(i32, i32)>>();

            // filter locations that are blocked or out of sight
            let positions: Vec<(i32, i32)> =
                move_positions
                .iter()
                .filter(|(x, y)| fov_map.is_in_fov(*x, *y))
                .filter(|(x, y)| !map.is_blocked(*x, *y, objects))
                .filter(|new_pos| ai_can_hit_target(**new_pos, (target_x, target_y), &attack).is_some())
                .map(|pair| *pair)
                .collect();

            // if there are any options to move to that will allow the monster to
            // attack, move to the one closest to their current position.
            if positions.len() > 0 {
                target_pos = positions.iter()
                                      .min_by_key(|pos| target_pos.distance(&Position::from_pair(**pos)))
                                      .map(|pair| Position::from_pair(*pair))
                                      .unwrap();
            }

            pos_offset = ai_take_astar_step((monster_x, monster_y), target_pos.pair(), map);
        }

        turn.add(AiAction::Move(pos_offset));
    }

    return turn;
}

pub fn ai_investigate(target_pos_orig: Position, 
                      monster_id: usize,
                      map: &Map,
                      objects: &Vec<Object>,
                      fov_map: &FovMap) -> AiTurn {
    let target_pos = target_pos_orig;
    let (player_x, player_y) = objects[PLAYER].pos();
    let player_pos = Position::new(player_x, player_y);

    let (monster_x, monster_y) = objects[monster_id].pos();
    let monster_pos = Position::new(monster_x, monster_y);
    let mut turn: AiTurn = AiTurn::new();

               
    if location_within_fov(fov_map, monster_pos, player_pos) {
        // TODO this causes a turn delay between seeing the player and attacking them
        turn.add(AiAction::StateChange(Behavior::Attacking(PLAYER)));
    } else { // the monster can't see the player
        if target_pos == monster_pos { 
            // if the monster reached its target then go back to being idle
            turn.add(AiAction::StateChange(Behavior::Idle));
        } else {
            // if the monster has not reached its target, move towards the target.
            let pos_offset = ai_take_astar_step((monster_x, monster_y), target_pos.pair(), map);

            turn.add(AiAction::Move(pos_offset));
        }
    }

    return turn;
}

fn ai_can_hit_target(monster_pos: (i32, i32), target_pos: (i32, i32), reach: &Reach) -> Option<(i32, i32)> {
    let (monster_x, monster_y) = monster_pos;
    let (target_x, target_y) = target_pos;
    let mut hit_pos = None;

    // get all locations they can hit
    let positions: Vec<(i32, i32)> =
        reach.offsets()
        .iter()
        .map(|pos| (pos.0 + monster_x, pos.1 + monster_y))
        .collect();

    // look through attack positions, in case one hits the target
    for pos in positions {
        if target_x == pos.0 && target_y == pos.1 {
            hit_pos = Some(pos);
            break;
        }
    }

    return hit_pos;
}

fn ai_take_astar_step(monster_pos: (i32, i32),
                      target_pos: (i32, i32),
                      map: &Map) -> (i32, i32) {
    let map_copy = map.make_tcod_map();
    let mut astar = AStar::new_from_map(map_copy, 1.5);
    astar.find(monster_pos, target_pos);

    let recalculate_when_needed = true;
    match astar.walk_one_step(recalculate_when_needed) {
        Some(target_pos) => {
            return step_towards(monster_pos, target_pos);
        }

        None => {
            return (0, 0);
        }
    }
}

pub fn basic_ai_take_turn(monster_id: usize,
                      map: &Map,
                      objects: &Vec<Object>,
                      fov_map: &FovMap) -> AiTurn {
    let (monster_x, monster_y) = objects[monster_id].pos();
    let (player_x, player_y) = objects[PLAYER].pos();
    let player_pos = Position::new(player_x, player_y);
    let monster_pos = Position::new(monster_x, monster_y);

    match objects[monster_id].behavior {
        Some(Behavior::Idle) => {
            let mut turn = AiTurn::new();

            if location_within_fov(fov_map, monster_pos, player_pos) {
                // TODO will cause a turn between seeing the player and attacking
                turn.add(AiAction::StateChange(Behavior::Attacking(PLAYER)));
            } else if let Some(sound_pos) = map[(monster_x, monster_y)].sound {
                let sound_position = Position::from_pair(sound_pos);
                turn.add(AiAction::StateChange(Behavior::Investigating(sound_position)));
            }

            return turn;
        }

        Some(Behavior::Investigating(target_pos)) => {
            ai_investigate(target_pos,
                           monster_id,
                           map,
                           objects,
                           fov_map)
        }

        Some(Behavior::Attacking(object_id)) => {
            ai_attack(monster_id,
                      object_id,
                      map,
                      objects,
                      fov_map)
        }

        ref behavior => {
            panic!("Ai behavior {:?} unexpected!", behavior);
        }
    }
}

pub fn ai_take_turn(monster_id: usize,
                    map: &Map,
                    objects: &mut Vec<Object>,
                    fov_map: &FovMap,
                    config: &Config) {
    let turn: AiTurn;

    match objects[monster_id].ai {
        Some(Ai::Basic) => {
            turn = basic_ai_take_turn(monster_id, map, objects, fov_map);
        }

        None => {
            turn = AiTurn::new();
        }
    }

    ai_apply_actions(monster_id,
                     turn,
                     map,
                     objects,
                     fov_map,
                     config);
}

pub fn ai_apply_actions(monster_id: usize,
                        turn: AiTurn,
                        map: &Map,
                        objects: &mut Vec<Object>,
                        _fov_map: &FovMap,
                        config: &Config) {
    for action in turn.actions().iter() {
        match action {
            AiAction::Move(pos) => {
                move_by(monster_id, pos.0, pos.1, map, objects);
            },

            AiAction::Attack(target_id, pos) => {
                let (target_x, target_y) = *pos;
                let (monster_x, monster_y) = objects[monster_id].pos();
                let (target, monster) = mut_two(*target_id, monster_id, objects);

                // apply attack 
                monster.attack(target, config);
            },

            AiAction::StateChange(behavior) => {
                objects[monster_id].behavior = Some(*behavior);
            },
        }
    }
}

pub fn mut_two<T>(first_index: usize, second_index: usize, items: &mut [T]) -> (&mut T, &mut T) {
    assert!(first_index != second_index);

    let split_at_index = cmp::max(first_index, second_index);
    let (first_slice, second_slice) = items.split_at_mut(split_at_index);
    if first_index < second_index {
        (&mut first_slice[first_index], &mut second_slice[0])
    } else {
        (&mut second_slice[0], &mut first_slice[second_index])
    }
}

