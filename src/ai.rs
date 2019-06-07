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

pub fn smart_ai_take_turn(monster_id: usize,
                      map: &Map,
                      objects: &mut [Object],
                      fov_map: &FovMap,
                      messages: &mut Messages) {
    let (monster_x, monster_y) = objects[monster_id].pos();
    let (player_x, player_y) = objects[PLAYER].pos();
    let player_pos = Position::new(player_x, player_y);

    match &objects[monster_id].behavior {
        Some(Behavior::Idle) => {
            if fov_map.is_in_fov(monster_x, monster_y) {
                objects[monster_id].behavior =
                    Some(Behavior::Seeking(Position::new(player_x, player_y)));
            }
        }

        Some(Behavior::Seeking(target_pos)) => {

            ai_seek_take_turn(*target_pos,
                              monster_id,
                              map,
                              objects,
                              fov_map,
                              messages);

            if !fov_map.is_in_fov(monster_x, monster_y) {
                let mut awareness_map = AwarenessMap::new(MAP_WIDTH as usize, MAP_HEIGHT as usize);
                awareness_map.expected_position(player_pos);
                objects[monster_id].behavior =
                    Some(Behavior::SmartSeeking(awareness_map));

            }
        }

        Some(Behavior::SmartSeeking(awareness_map)) => {
            if fov_map.is_in_fov(monster_x, monster_y) {
                objects[monster_id].behavior = Some(Behavior::Seeking(player_pos));
            } else {
                let mut awareness_map = awareness_map.clone();
                //fov_map.compute_fov(monster_x, monster_y, TORCH_RADIOUS, FOV_LIGHT_WALLS, FOV_ALGO);

                for y in 0..MAP_HEIGHT {
                    for x in 0..MAP_WIDTH {
                        if fov_map.is_in_fov(x, y) {
                            awareness_map.visible(Position::new(x, y));
                        }
                    }
                }

                awareness_map.disperse();

                // recompute player fov, in case it is used by other monsters. likely there is a
                // better way to do this.
                //fov_map.compute_fov(player_x, player_y, TORCH_RADIOUS, FOV_LIGHT_WALLS, FOV_ALGO);

                // NOTE must update with new map here.
            }
        }
        
        ref behavior => {
            panic!("Ai behavior {:?} unexpected!", behavior);
        }
    }
}

pub fn ai_seek_take_turn(target_pos_orig: Position, 
                     monster_id: usize,
                     map: &Map,
                     objects: &mut [Object],
                     fov_map: &FovMap,
                     _messages: &mut Messages) {
    let mut target_pos = target_pos_orig;
    let (player_x, player_y) = objects[PLAYER].pos();
    let player_pos = Position::new(player_x, player_y);

    let (monster_x, monster_y) = objects[monster_id].pos();

    if fov_map.is_in_fov(monster_x, monster_y) {
        objects[monster_id].behavior = Some(Behavior::Seeking(player_pos));
        target_pos = player_pos;
    }

    let map_copy = map.make_tcod_map();
    let mut astar = AStar::new_from_map(map_copy, 1.5);
    astar.find((monster_x, monster_y), target_pos.pair());

    if let Some((dx, dy)) = astar.walk_one_step(true) {
        move_towards(monster_id, dx, dy, map, objects);

        if objects[monster_id].pos() == target_pos.pair() {
            objects[monster_id].behavior = Some(Behavior::Idle);
        }
    }
}

fn basic_ai_take_turn(monster_id: usize,
                      map: &Map,
                      objects: &mut [Object],
                      fov_map: &FovMap,
                      messages: &mut Messages) {
    let (monster_x, monster_y) = objects[monster_id].pos();
    let (player_x, player_y) = objects[PLAYER].pos();
    let player_pos = Position::new(player_x, player_y);

    match objects[monster_id].behavior {
        Some(Behavior::Idle) => {
            if fov_map.is_in_fov(monster_x, monster_y) {
                objects[monster_id].behavior = Some(Behavior::Seeking(player_pos));
            }
        }

        Some(Behavior::Seeking(target_pos)) => {
            ai_seek_take_turn(target_pos,
                              monster_id,
                              map,
                              objects,
                              fov_map,
                              messages);
        }
        
        ref behavior => {
            panic!("Ai behavior {:?} unexpected!", behavior);
        }
    }
}

pub fn ai_take_turn(monster_id: usize, map: &Map, objects: &mut [Object], fov_map: &FovMap, messages: &mut Messages) {
    match objects[monster_id].ai {
        Some(Ai::Basic) => {
            basic_ai_take_turn(monster_id, map, objects, fov_map, messages);
        }

        Some(Ai::Smart) => {
            smart_ai_take_turn(monster_id, map, objects, fov_map, messages);
        }

        Some(Ai::Patrol) => {
        }

        Some(Ai::Guard) => {
        }

        Some(Ai::Passive) => {
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

