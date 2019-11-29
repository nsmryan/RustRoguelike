use crate::map::*;
use crate::types::*;
use crate::constants::*;
use crate::movement::*;


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AiAction {
    Move((i32, i32)),
    Attack(ObjectId, (i32, i32)),
    StateChange(Behavior),
    NoAction,
}


#[derive(Clone, Debug, PartialEq)]
pub struct AiTurn(AiAction);

impl AiTurn {
    pub fn new(action: AiAction) -> AiTurn {
        return AiTurn(action);
    }

    pub fn none() -> AiTurn {
        return AiTurn(AiAction::NoAction);
    }

    pub fn action(self) -> AiAction {
        return self.0;
    }
}


pub fn location_within_fov(map: &mut Map, monster_pos: Position, player_pos: Position) -> bool {
    let within_fov = map.is_in_fov(monster_pos.0, monster_pos.1, player_pos.0, player_pos.1, MONSTER_VIEW_DIST);
    let within_sight_range = player_pos.distance(&monster_pos) <= MONSTER_VIEW_DIST;
    let blocked_by_wall = map.is_blocked_by_wall(monster_pos.0,
                                                 monster_pos.1,
                                                 player_pos.0 - monster_pos.0,
                                                 player_pos.1 - monster_pos.1);

    return within_fov && within_sight_range && !blocked_by_wall;
}

// TODO consider moving to GameData
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
                 data: &mut GameData) -> AiTurn {
    let (target_x, target_y) = data.objects[target_handle].pos();
    let mut target_pos = Position::new(target_x, target_y);

    let (monster_x, monster_y) = data.objects[monster_handle].pos();
    let monster_pos = Position::new(monster_x, monster_y);

    let mut turn: AiTurn = AiTurn::none();

    // if AI can hit their target
    if let Some(hit_pos) =
        ai_can_hit_target(&mut data.map, 
                          data.objects[monster_handle].pos(),
                          data.objects[target_handle].pos(),
                          &data.objects[monster_handle].attack.unwrap()) {
        turn = AiTurn::new(AiAction::Attack(target_handle, hit_pos));
    } else if data.map.is_blocked_by_wall(monster_x, monster_y, target_x - monster_x, target_y - monster_y) {
        turn = AiTurn::new( AiAction::StateChange(Behavior::Investigating((target_pos))));
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
                                      .min_by_key(|pos| target_pos.distance(&Position::from_pair(**pos)))
                                      .map(|pair| Position::from_pair(*pair))
                                      .unwrap();
            }

            pos_offset = ai_take_astar_step((monster_x, monster_y), target_pos.pair(), &data.map);
        }

        turn = AiTurn::new(AiAction::Move(pos_offset));
    }

    return turn;
}

pub fn ai_investigate(target_pos_orig: Position, 
                      monster_handle: ObjectId,
                      data: &mut GameData) -> AiTurn {
    let player_handle = data.find_player().unwrap();
    let target_pos = target_pos_orig;
    let (player_x, player_y) = data.objects[player_handle].pos();
    let player_pos = Position::new(player_x, player_y);

    let (monster_x, monster_y) = data.objects[monster_handle].pos();
    let monster_pos = Position::new(monster_x, monster_y);
    let mut turn: AiTurn;

               
    if location_within_fov(&mut data.map, monster_pos, player_pos) {
        // TODO this causes a turn delay between seeing the player and attacking them
        turn = AiTurn::new(AiAction::StateChange(Behavior::Attacking(player_handle)));
    } else { // the monster can't see the player
        if target_pos == monster_pos { 
            // if the monster reached its target then go back to being idle
            turn = AiTurn::new(AiAction::StateChange(Behavior::Idle));
        } else {
            // if the monster has not reached its target, move towards the target.
            let pos_offset = ai_take_astar_step((monster_x, monster_y), target_pos.pair(), &data.map);

            turn = AiTurn::new(AiAction::Move(pos_offset));
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

    if location_within_fov(map, Position::from_pair(monster_pos), Position::from_pair(target_pos)) {

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
                          data: &mut GameData) -> AiTurn {
    let player_handle = data.find_player().unwrap();
    let (monster_x, monster_y) = data.objects[monster_handle].pos();
    let (player_x, player_y) = data.objects[player_handle].pos();
    let player_pos = Position::new(player_x, player_y);
    let monster_pos = Position::new(monster_x, monster_y);

    if data.map.is_within_bounds(monster_pos.0, monster_pos.1) {
        match data.objects[monster_handle].behavior {
            Some(Behavior::Idle) => {
                let mut turn = AiTurn::none();

                if location_within_fov(&mut data.map, monster_pos, player_pos) {
                    // TODO will cause a turn between seeing the player and attacking
                    turn = AiTurn::new(AiAction::StateChange(Behavior::Attacking(player_handle)));
                } else if let Some(sound_pos) = data.map[(monster_x, monster_y)].sound {
                    let sound_position = Position::from_pair(sound_pos);
                    turn = AiTurn::new(AiAction::StateChange(Behavior::Investigating(sound_position)));
                }

                return turn;
            }

            Some(Behavior::Investigating(target_pos)) => {
                return ai_investigate(target_pos, monster_handle, data);
            }

            Some(Behavior::Attacking(object_handle)) => {
                return ai_attack(monster_handle, object_handle, data);
            }

            ref behavior => {
                panic!("Ai behavior {:?} unexpected!", behavior);
            }
        }
    } else {
        // position outside of map- return empty turn
        return AiTurn::none();
    }
}

pub fn ai_take_turn(monster_handle: ObjectId, data: &mut GameData) {
    let turn: AiTurn;

    match data.objects[monster_handle].ai {
        Some(Ai::Basic) => {
            turn = basic_ai_take_turn(monster_handle, data);
        }

        None => {
            turn = AiTurn::none();
        }
    }

    ai_apply_actions(monster_handle,
                     turn,
                     data);
}

pub fn ai_apply_actions(monster_handle: ObjectId,
                        turn: AiTurn,
                        data: &mut GameData) {
    match turn.action() {
        AiAction::Move(pos) => {
            move_by(monster_handle, pos.0, pos.1, data);
        },

        AiAction::Attack(target_handle, _pos) => {
            //let (target, monster) = mut_two(*target_handle, monster_handle, &mut data.objects);

            // apply attack 
            attack(monster_handle, target_handle, &mut data.objects);
        },

        AiAction::StateChange(behavior) => {
            data.objects.get_mut(monster_handle).unwrap().behavior = Some(behavior);
        },

        AiAction::NoAction => {
        }
    }
}

pub fn attack(handle: ObjectId, other_handle: ObjectId, objects: &mut ObjMap) {
    let damage = objects[handle].fighter.map_or(0, |f| f.power) -
                 objects[other_handle].fighter.map_or(0, |f| f.defense);

    if damage > 0 {
        //messages.message(format!("{} attacks {} for {} hit points.", self.name, target.name, damage), WHITE);
        objects[other_handle].take_damage(damage);
    } else {
        //messages.message(format!("{} attacks {} but it has no effect!", self.name, target.name), WHITE);
    }
}

