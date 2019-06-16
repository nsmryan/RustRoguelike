#[allow(unused_imports)]use std::cmp;

use num::clamp;

#[allow(unused_imports)]use tcod::map::{Map as FovMap};
#[allow(unused_imports)]use tcod::pathfinding::*;

use crate::constants::*;
use crate::types::*;
use crate::map::*;


pub fn move_player_by(objects: &mut [Object], map: &Map, dx: i32, dy: i32) {
    let (x, y) = objects[PLAYER].pos();

    let (mut mx, mut my) = objects[PLAYER].momentum.unwrap();

    let has_momentum = mx.abs() > 1 || my.abs() > 1;
    let momentum_diagonal = mx.abs() != 0 && my.abs() != 0;
    let side_move = dx.abs() != 0 && dy.abs() != 0;
    let same_direction = mx.signum() == dx.signum() && my.signum() == dy.signum();

    let momentum_change: MomentumChange;

    // if the space is not blocked, move
    if !map.is_blocked(x + dx, y + dy, objects) {
        objects[PLAYER].set_pos(x + dx, y + dy);
        momentum_change = MomentumChange::CurrentDirection;
    } else if has_momentum &&
              side_move &&
              !momentum_diagonal &&
              !map.is_blocked(x + mx.signum(), y + my.signum(), objects) && // free next to wall
              !map.is_blocked(x + 2*mx.signum(), y + 2*my.signum(), objects) && // free space to move to
              map[(x + dx, y + dy)].tile_type == TileType::Wall {
                // jump off wall
                objects[PLAYER].set_pos(x + 2*mx.signum(), y + 2*my.signum());
                momentum_change = MomentumChange::PreviousDirection;
    } else if has_momentum &&
              same_direction &&
              map[(x + dx, y + dy)].tile_type == TileType::ShortWall &&
              !map.is_blocked(x + 2*dx, y + 2*dy, objects) {
                // if the location is blocked, and the next location in the
                // line is not, and we have momentum, then jump over obstacle
                objects[PLAYER].set_pos(x + 2*dx, y + 2*dy);
                momentum_change = MomentumChange::CurrentDirection;
    } else {
        // otherwise we hit a wall and lose our momentum
        momentum_change = MomentumChange::Lost;
    }

    match momentum_change {
        MomentumChange::Lost => {
            mx = 0;
            my = 0;
        }

        MomentumChange::PreviousDirection => {
            mx = clamp(mx + mx.signum(), -MAX_MOMENTUM, MAX_MOMENTUM);
            my = clamp(my + my.signum(), -MAX_MOMENTUM, MAX_MOMENTUM);
        }

        MomentumChange::CurrentDirection => {
            if same_direction {
                mx = clamp(mx + dx, -MAX_MOMENTUM, MAX_MOMENTUM);
                my = clamp(my + dy, -MAX_MOMENTUM, MAX_MOMENTUM);
            } else {
                mx = dx;
                my = dy;
            }
        }
    }

    objects[PLAYER].momentum = Some((mx, my));
}

pub fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut [Object]) {
    let (x, y) = objects[id].pos();

    if !map.is_blocked(x + dx, y + dy, objects){
        objects[id].set_pos(x + dx, y + dy);
    }
}

pub fn move_towards(id: usize, target_x: i32, target_y: i32, map: &Map, objects: &mut [Object]) {
    let dx = target_x - objects[id].x;
    let dy = target_y - objects[id].y;
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();

    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;
    move_by(id, dx, dy, map, objects);
}

pub fn ai_attack(monster_id: usize,
                 map: &Map,
                 objects: &mut [Object],
                 fov_map: &FovMap,
                 _messages: &mut Messages) -> AiAction {
    let (player_x, player_y) = objects[PLAYER].pos();
    let player_pos = Position::new(player_x, player_y);
    let took_turn: AiAction;

    if let Some(hit_pos) = ai_can_hit_player(monster_id, objects) {
        let (player, monster) = mut_two(PLAYER, monster_id, objects);
        monster.attack(player, _messages);
        took_turn = AiAction::TookTurn;
    } else {
        // can't hit- seek to current player position instead
        objects[monster_id].behavior = Some(Behavior::Seeking(player_pos));
        took_turn = AiAction::DidntTakeTurn;
    }

    return took_turn;
}

pub fn ai_seek_take_turn(target_pos_orig: Position, 
                         monster_id: usize,
                         map: &Map,
                         objects: &mut [Object],
                         fov_map: &FovMap,
                         _messages: &mut Messages) -> AiAction {
    let mut target_pos = target_pos_orig;
    let (player_x, player_y) = objects[PLAYER].pos();
    let player_pos = Position::new(player_x, player_y);

    let (monster_x, monster_y) = objects[monster_id].pos();
    let took_turn: AiAction;

    // if the player is in view, update our target location to seek towards
    if fov_map.is_in_fov(monster_x, monster_y) {
        target_pos = player_pos;
    }

    if let Some(hit_pos) = ai_can_hit_player(monster_id, objects) {
        // TODO we should return something or call something
        // to indicate that we also want to perform an attack.
        // This way the monster attacks when you get into its reach
        objects[monster_id].behavior = Some(Behavior::Attacking);

        took_turn = AiAction::DidntTakeTurn;
    } else {
        // if the monster cannot hit the player, just make a move
        // towards the player
        // TODO this should move not toward the player, but to a square
        // in which we can attack the player
        ai_take_astar_step(monster_id, (monster_x, monster_y), target_pos.pair(), map, objects);
        took_turn = AiAction::TookTurn;
    }

    return took_turn;
}

fn ai_can_hit_player(monster_id: ObjectId, objects: &[Object]) -> Option<(i32, i32)> {
    let (player_x, player_y) = objects[PLAYER].pos();
    let (monster_x, monster_y) = objects[monster_id].pos();
    let mut hit_pos = None;
 
    if let Some(reach) = objects[monster_id].attack {
        // get all locations they can hit
        let positions: Vec<(i32, i32)> =
            reach.offsets()
                 .iter()
                 .map(|pos| (pos.0 + monster_x, pos.1 + monster_y))
                 .collect();

        // look through attack positions, in case one hits the player
        for pos in positions {
            if player_x == pos.0 && player_y == pos.1 {
                hit_pos = Some(pos)
            }
        }
    }

    return hit_pos;
}

fn ai_take_astar_step(monster_id: ObjectId, monster_pos: (i32, i32), target_pos: (i32, i32), map: &Map, objects: &mut [Object]) {
    let map_copy = map.make_tcod_map();
    let mut astar = AStar::new_from_map(map_copy, 1.5);
    astar.find(monster_pos, target_pos);

    if let Some((dx, dy)) = astar.walk_one_step(true) {
        move_towards(monster_id, dx, dy, map, objects);
    }
}

fn basic_ai_take_turn(monster_id: usize,
                      map: &Map,
                      objects: &mut [Object],
                      fov_map: &FovMap,
                      messages: &mut Messages) -> AiAction {
    let (monster_x, monster_y) = objects[monster_id].pos();
    let (player_x, player_y) = objects[PLAYER].pos();
    let player_pos = Position::new(player_x, player_y);

    match objects[monster_id].behavior {
        Some(Behavior::Idle) => {
            if fov_map.is_in_fov(monster_x, monster_y) {
                objects[monster_id].behavior = Some(Behavior::Seeking(player_pos));
            }
            AiAction::TookTurn
        }

        Some(Behavior::Seeking(target_pos)) => {
            ai_seek_take_turn(target_pos,
                              monster_id,
                              map,
                              objects,
                              fov_map,
                              messages)
        }
        
        Some(Behavior::Attacking) => {
            ai_attack(monster_id,
                      map,
                      objects,
                      fov_map,
                      messages)
        }

        ref behavior => {
            panic!("Ai behavior {:?} unexpected!", behavior);
        }
    }
}

pub fn ai_take_turn(monster_id: usize, map: &Map, objects: &mut [Object], fov_map: &FovMap, messages: &mut Messages) {
    match objects[monster_id].ai {
        Some(Ai::Basic) => {
            let took_turn = basic_ai_take_turn(monster_id, map, objects, fov_map, messages);

            // allow an extra iteration if the AI didn't take a turn.
            // note that this is not in a loop- only one extra iteration is allowed
            if took_turn == AiAction::DidntTakeTurn {
                basic_ai_take_turn(monster_id, map, objects, fov_map, messages);
            }
        }

        None => {
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

