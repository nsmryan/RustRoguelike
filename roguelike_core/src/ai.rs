use crate::map::*;
use crate::types::*;
use crate::constants::*;
use crate::movement::*;


pub fn move_by(handle: ObjectId, dx: i32, dy: i32, data: &mut GameData) {
    let (x, y) = data.objects[handle].pos();

    if !is_blocked(x + dx, y + dy, data) {
        data.objects[handle].set_pos(x + dx, y + dy);
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

pub fn ai_attack(monster_handle: ObjectId,
                 target_handle: ObjectId,
                 data: &mut GameData) -> Action {
    let (target_x, target_y) = data.objects[target_handle].pos();
    let mut target_pos = Pos::new(target_x, target_y);

    let (monster_x, monster_y) = data.objects[monster_handle].pos();
    let monster_pos = Pos::new(monster_x, monster_y);

    let turn: Action;

    // if AI can hit their target
    if let Some(hit_pos) =
        ai_can_hit_target(&mut data.map, 
                          data.objects[monster_handle].pos(),
                          data.objects[target_handle].pos(),
                          &data.objects[monster_handle].attack.unwrap()) {
        turn = Action::Attack(target_handle, hit_pos);
    } else if data.map.is_blocked_by_wall(monster_x, monster_y, target_x - monster_x, target_y - monster_y) {
        turn = Action::StateChange(Behavior::Investigating(target_pos));
    } else { // otherwise attempt to move towards their target
        // check positions that can hit target, filter by FOV, and get the closest.
        // then move to this closest position.
        let mut pos_offset = (0, 0);
        if let (Some(attack), Some(movement)) =
            (data.objects[monster_handle].attack, data.objects[monster_handle].movement) {
            // get all locations they can hit
            let move_positions =
                MoveAction::move_actions().iter()
                                          .map(|move_action| movement.move_with_reach(move_action))
                                          .filter_map(|mov| mov)
                                          .map(|mov| mov.add(monster_pos).into_pair())
                                          .filter(|mov| data.map.is_within_bounds(mov.0, mov.1))
                                          .collect::<Vec<(i32, i32)>>();

            // filter locations that are blocked or out of sight
            let positions: Vec<(i32, i32)> =
                move_positions
                .iter()
                .filter(|new_pos| ai_can_hit_target(&mut data.map, **new_pos, (target_x, target_y), &attack).is_some())
                .map(|pair| *pair)
                .collect();

            // if there are any options to move to that will allow the monster to
            // attack, move to the one closest to their current position.
            if positions.len() > 0 {
                target_pos = positions.iter()
                                      .min_by_key(|pos| target_pos.distance(&Pos::from_pair(**pos)))
                                      .map(|pair| Pos::from_pair(*pair))
                                      .unwrap();
            }

            pos_offset = ai_take_astar_step((monster_x, monster_y), target_pos.pair(), &data.map);
        }

        turn = Action::Move(Movement::Move(pos_offset.0, pos_offset.1));
    }

    return turn;
}

pub fn ai_investigate(target_pos_orig: Pos, 
                      monster_handle: ObjectId,
                      data: &mut GameData) -> Action {
    let player_handle = data.find_player().unwrap();
    let mut target_pos = target_pos_orig;
    let (player_x, player_y) = data.objects[player_handle].pos();

    let (monster_x, monster_y) = data.objects[monster_handle].pos();
    let monster_pos = Pos::new(monster_x, monster_y);
    let turn: Action;

               
    if data.map.is_in_fov(monster_x, monster_y, player_x, player_y, MONSTER_VIEW_DIST) {
        // TODO this causes a turn delay between seeing the player and attacking them
        turn = Action::StateChange(Behavior::Attacking(player_handle));
    } else { // the monster can't see the player
        if let Some(sound_id) = data.sound_within_earshot(monster_x, monster_y) {
            target_pos = Pos::new(data.objects[sound_id].x, data.objects[sound_id].y);
            data.objects[monster_handle].behavior =
                Some(Behavior::Investigating(target_pos));
        }

        if target_pos == monster_pos { 
            // if the monster reached its target then go back to being idle
            turn = Action::StateChange(Behavior::Idle);
        } else {
            // if the monster has not reached its target, move towards the target.
            let pos_offset = ai_take_astar_step((monster_x, monster_y), target_pos.pair(), &data.map);

            turn = Action::Move(Movement::Move(pos_offset.0, pos_offset.1));
        }
    }

    return turn;
}

fn ai_can_hit_target(map: &mut Map,
                     monster_pos: (i32, i32),
                     target_pos: (i32, i32),
                     reach: &Reach) -> Option<(i32, i32)> {
    let (monster_x, monster_y) = monster_pos;
    let (target_x, target_y) = target_pos;
    let mut hit_pos = None;

    let within_fov =
        map.is_in_fov(monster_x, monster_y,
                      target_x, target_y,
                      MONSTER_VIEW_DIST);

    if within_fov {
            // get all locations they can hit
            let positions: Vec<(i32, i32)> =
                reach.offsets()
                .iter()
                .map(|pos| (pos.0 + monster_x, pos.1 + monster_y))
                .collect();

        // look through attack positions, in case one hits the target
        for pos in positions {
            if (target_x, target_y) == pos {
                hit_pos = Some(pos);
                break;
            }
        }
    }

    return hit_pos;
}

fn ai_take_astar_step(monster_pos: (i32, i32),
                      target_pos: (i32, i32),
                      map: &Map) -> (i32, i32) {
    let astar_iter = map.astar(monster_pos, target_pos);

    if astar_iter.len() > 0 {
        return step_towards(monster_pos, astar_iter[1]);
    } else {
        return (0, 0);
    }
}

pub fn basic_ai_take_turn(monster_handle: ObjectId,
                          data: &mut GameData) -> Action {
    let player_handle = data.find_player().unwrap();
    let (monster_x, monster_y) = data.objects[monster_handle].pos();
    let (player_x, player_y) = data.objects[player_handle].pos();
    let monster_pos = Pos::new(monster_x, monster_y);

    if data.map.is_within_bounds(monster_pos.0, monster_pos.1) {
        match data.objects[monster_handle].behavior {
            Some(Behavior::Idle) => {
                let mut turn = Action::none();

                if data.map.is_in_fov(monster_x, monster_y, player_x, player_y, MONSTER_VIEW_DIST) {
                    // NOTE will cause a turn between seeing the player and attacking
                    turn = Action::StateChange(Behavior::Attacking(player_handle));
                } else if let Some(sound_id) = data.sound_within_earshot(monster_x, monster_y) {
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
                Movement::Move(x, y) => {
                move_by(monster_handle, x, y, data);
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

