#[allow(unused_imports)]use std::cmp;

use num::clamp;

#[allow(unused_imports)]use tcod::map::{Map as FovMap};
#[allow(unused_imports)]use tcod::pathfinding::*;
use tcod::line::*;

use crate::constants::*;
use crate::types::*;
use crate::map::*;


pub fn make_orc(config: &Config, x: i32, y :i32) -> Object {
    let mut orc = Object::new(x, y, '\u{98}', "orc", config.color_light_orange.color(), true);
    orc.fighter = Some( Fighter { max_hp: 10, hp: 10, defense: 0, power: 5, on_death: DeathCallback::Monster } );
    orc.ai = Some(Ai::Basic);
    orc.behavior = Some(Behavior::Idle);
    orc.color = config.color_light_orange.color();
    orc.movement = Some(Reach::Single);
    orc.attack = Some(Reach::Diag);
    orc.alive = true;
    orc
} 

pub fn make_troll(config: &Config, x: i32, y :i32) -> Object {
    let mut troll = Object::new(x, y, '\u{15}', "troll", config.color_mint_green.color(), true);
    troll.fighter = Some( Fighter { max_hp: 16, hp: 16, defense: 1, power: 10, on_death: DeathCallback::Monster } );
    troll.ai = Some(Ai::Basic);
    troll.behavior = Some(Behavior::Idle);
    troll.color = config.color_mint_green.color();
    troll.movement = Some(Reach::Single);
    troll.attack = Some(Reach::Diag);
    troll.alive = true;
    troll
}

pub fn make_kobold(config: &Config, x: i32, y :i32) -> Object {
    let mut kobold = Object::new(x, y, '\u{A5}', "kobold", config.color_ice_blue.color(), true);
    kobold.fighter = Some( Fighter { max_hp: 16, hp: 16, defense: 1, power: 5, on_death: DeathCallback::Monster } );
    kobold.ai = Some(Ai::Basic);
    kobold.behavior = Some(Behavior::Idle);
    kobold.color = config.color_ice_blue.color();
    kobold.movement = Some(Reach::Horiz);
    kobold.attack = Some(Reach::Horiz);
    kobold.alive = true;
    kobold
}

pub fn can_see_player(fov_map: &FovMap, monster_pos: Position, player_pos: Position) -> bool {
    let within_fov = fov_map.is_in_fov(monster_pos.0, monster_pos.1);
    let within_sight_range = player_pos.distance(&monster_pos) <= MONSTER_VIEW_DIST;

    return within_fov && within_sight_range;
}

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

pub fn move_towards(start_pos: (i32, i32), target_pos: (i32, i32)) -> (i32, i32) {
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

    if let Some(hit_pos) = ai_can_hit_target(monster_id, target_id, objects) {
        turn.add(AiAction::Attack(hit_pos));
    } else {
        // check positions that can hit target, filter by FOV, and get the closest.
        // then move to this closest position.
        let mut pos_offset = (0, 0);
        if let Some(reach) = objects[monster_id].attack {
            // get all locations they can hit
            let positions: Vec<(i32, i32)> =
                reach.offsets()
                .iter()
                .map(|pos| (pos.0 + target_x, pos.1 + target_y))
                .filter(|(x, y)| fov_map.is_in_fov(*x, *y))
                .filter(|(x, y)| !map.is_blocked(*x, *y, objects))
                .collect();
            if positions.len() > 0 {
                target_pos = positions.iter()
                                      .min_by_key(|pos| monster_pos.distance(&Position::from_pair(pos)))
                                      .map(|pair| Position::from_pair(pair))
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

               
    if can_see_player(fov_map, monster_pos, player_pos) {
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

fn ai_can_hit_target(monster_id: ObjectId, target_id: ObjectId, objects: &[Object]) -> Option<(i32, i32)> {
    let (target_x, target_y) = objects[target_id].pos();
    let (monster_x, monster_y) = objects[monster_id].pos();
    let mut hit_pos = None;

    if let Some(reach) = objects[monster_id].attack {
        // get all locations they can hit
        let positions: Vec<(i32, i32)> =
            reach.offsets()
            .iter()
            .map(|pos| (pos.0 + monster_x, pos.1 + monster_y))
            .collect();

        // look through attack positions, in case one hits the target
        for pos in positions {
            if target_x == pos.0 && target_y == pos.1 {
                hit_pos = Some(pos)
            }
        }
    }

    return hit_pos;
}

fn ai_take_astar_step(monster_pos: (i32, i32), target_pos: (i32, i32), map: &Map) -> (i32, i32) {
    let map_copy = map.make_tcod_map();
    let mut astar = AStar::new_from_map(map_copy, 1.5);
    astar.find(monster_pos, target_pos);

    match astar.walk_one_step(true) {
        Some(target_pos) => {
            return move_towards(monster_pos, target_pos);
        }

        None => {
            return (0, 0);
        }
    }
}

fn basic_ai_take_turn(monster_id: usize,
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

            if can_see_player(fov_map, monster_pos, player_pos) {
                // TODO will cause a turn between seeing the player and attacking
                turn.add(AiAction::StateChange(Behavior::Attacking(PLAYER)));
            } else if let Some(sound_pos) = map[(monster_x, monster_y)].sound {
                let sound_position = Position::from_pair(&sound_pos);
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
                    animations: &mut Vec<Animation>) {
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
                     animations);
}

pub fn ai_apply_actions(monster_id: usize,
                        turn: AiTurn,
                        map: &Map,
                        objects: &mut Vec<Object>,
                        fov_map: &FovMap,
                        animations: &mut Vec<Animation>) {
    for action in turn.actions().iter() {
        match action {
            AiAction::Move(pos) => {
                move_by(monster_id, pos.0, pos.1, map, objects);
            },

            AiAction::Attack(pos) => {
                let (player_x, player_y) = objects[PLAYER].pos();
                let (monster_x, monster_y) = objects[monster_id].pos();
                let (player, monster) = mut_two(PLAYER, monster_id, objects);

                // apply attack 
                monster.attack(player);

                // add animation
                let mut thrown_obj =
                    Object::new(monster_x, monster_y, '.', "thrown", tcod::colors::BLACK, false);
                let obj_id = objects.len();
                thrown_obj.x = monster_x;
                thrown_obj.y = monster_y;
                objects.push(thrown_obj);
                let animation =
                    Animation::Thrown(obj_id,
                                      Line::new((monster_x, monster_y),
                                      (player_x, player_y)));
                animations.push(animation);
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
