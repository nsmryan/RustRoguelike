use std::iter::Iterator;

use tcod::line::*;

use slotmap::dense::*;

use crate::map::*;
use crate::types::*;
use crate::ai::*;


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Movement {
    Move(i32, i32),
    Attack(i32, i32, ObjectId),
    Collide(i32, i32),
    WallKick(i32, i32, i32, i32), // x, y, dir_x, dir_y
    JumpWall(i32, i32),
}

impl Movement {
    pub fn xy(&self) -> (i32, i32) {
        match self {
            Movement::Move(x, y) => (*x, *y),
            Movement::Attack(x, y, _) => (*x, *y),
            Movement::Collide(x, y) => (*x, *y),
            Movement::WallKick(x, y, _, _) => (*x, *y),
            Movement::JumpWall(x, y) => (*x, *y),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoveAction {
    Left,
    Right,
    Up,
    Down,
    DownLeft,
    DownRight,
    UpLeft,
    UpRight,
    Center,
}

impl MoveAction {
    pub fn into_move(self) -> (i32, i32) {
        match self {
            MoveAction::Left => (-1, 0),
            MoveAction::Right => (1, 0),
            MoveAction::Up => (0, -1),
            MoveAction::Down => (0, 1),
            MoveAction::DownLeft => (-1, 1),
            MoveAction::DownRight => (1, 1),
            MoveAction::UpLeft => (-1, -1),
            MoveAction::UpRight => (1, -1),
            MoveAction::Center => (0, 0),
        }
    }

    pub fn move_actions() -> Vec<MoveAction> {
        return vec!(MoveAction::Left,
                    MoveAction::Right,
                    MoveAction::Up,
                    MoveAction::Down,
                    MoveAction::DownLeft,
                    MoveAction::DownRight,
                    MoveAction::UpLeft,
                    MoveAction::UpRight,
                    MoveAction::Center);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reach {
    Single(usize),
    Diag(usize),
    Horiz(usize),
}

impl Reach {
    pub fn move_with_reach(&self, move_action: &MoveAction) -> Option<Position> {
        match self {
            Reach::Single(dist) => {
                let dist = (*dist) as i32;
                let neg_dist = dist * -1;
                match move_action {
                    MoveAction::Left => Some(Position::from_pair((neg_dist, 0))),
                    MoveAction::Right => Some(Position::from_pair((dist, 0))),
                    MoveAction::Up => Some(Position::from_pair((0, neg_dist))),
                    MoveAction::Down => Some(Position::from_pair((0, dist))),
                    MoveAction::DownLeft => Some(Position::from_pair((neg_dist, dist))),
                    MoveAction::DownRight => Some(Position::from_pair((dist, dist))),
                    MoveAction::UpLeft => Some(Position::from_pair((neg_dist, neg_dist))),
                    MoveAction::UpRight => Some(Position::from_pair((dist, neg_dist))),
                    MoveAction::Center => Some(Position::from_pair((0, 0))),
                }
            }

            Reach::Diag(dist) => {
                let dist = (*dist) as i32;
                let neg_dist = dist * -1;
                match move_action {
                    MoveAction::Left => None,
                    MoveAction::Right => None,
                    MoveAction::Up => None,
                    MoveAction::Down => None,
                    MoveAction::DownLeft => Some(Position::from_pair((neg_dist, dist))),
                    MoveAction::DownRight => Some(Position::from_pair((dist, dist))),
                    MoveAction::UpLeft => Some(Position::from_pair((neg_dist, neg_dist))),
                    MoveAction::UpRight => Some(Position::from_pair((dist, neg_dist))),
                    MoveAction::Center => Some(Position::from_pair((0, 0))),
                }
            }

            Reach::Horiz(dist) => {
                let dist = (*dist) as i32;
                let neg_dist = dist * -1;
                match move_action {
                    MoveAction::Left => Some(Position::from_pair((neg_dist, 0))),
                    MoveAction::Right => Some(Position::from_pair((dist, 0))),
                    MoveAction::Up => Some(Position::from_pair((0, neg_dist))),
                    MoveAction::Down => Some(Position::from_pair((0, dist))),
                    MoveAction::DownLeft => None,
                    MoveAction::DownRight => None,
                    MoveAction::UpLeft => None,
                    MoveAction::UpRight => None,
                    MoveAction::Center => None,
                }
            }
        }
    }

    pub fn offsets(&self) -> Vec<Position> {
        match self {
            Reach::Single(dist) => {
                let dist = (*dist) as i32;
                let offsets =
                    vec!( (0, dist),      (-dist, dist), (-dist,  0),
                          (-dist, -dist), (0,  -dist),   (dist, -dist),
                          (dist,  0), (dist, dist));
                offsets.iter().map(|pair| Position::from_pair(*pair)).collect()
            },

            Reach::Horiz(dist) => {
                let dist = (*dist) as i32;
                let mut offsets = vec!();
                for dist in 1..dist {
                    offsets.push((dist, 0));
                    offsets.push((0, dist));
                    offsets.push((-1 * dist, 0));
                    offsets.push((0, -1 * dist));
                }
                offsets.iter().map(|pair| Position::from_pair(*pair)).collect()
            },


            Reach::Diag(dist) => {
                let mut offsets = vec!();
                let dist = (*dist) as i32;
                for dist in 1..dist {
                    offsets.push((dist, dist));
                    offsets.push((-1 * dist, dist));
                    offsets.push((dist, -1 * dist));
                    offsets.push((-1 * dist, -1 * dist));
                }
                offsets.iter().map(|pair| Position::from_pair(*pair)).collect()
            },
        }
    }
}


#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Collision {
    NoCollision(i32, i32),
    BlockedTile((i32, i32), (i32, i32)),
    Wall((i32, i32), (i32, i32)),
    Entity(ObjectId, (i32, i32)),
}

impl Collision {
    pub fn no_collsion(&self) -> bool {
        match self {
            Collision::NoCollision(_, _) => true,
            _ => false,
        }
    }

    pub fn move_location(&self) -> (i32, i32) {
        let last_pos;

        match *self {
            Collision::NoCollision(x, y) => {
                last_pos = (x, y);
            }

            Collision::BlockedTile(_, (x, y)) => {
                last_pos = (x, y);
            }

            Collision::Wall(_, (x, y)) => {
                last_pos = (x, y);
            }

            Collision::Entity(_, (x, y)) => {
                last_pos = (x, y);
            }
        }

        return last_pos;
    }
}

// TODO consider moving to GameData
/// Moves the given object with a given offset, returning the square that it collides with, or None
/// indicating no collision.
pub fn check_collision(x: i32,
                       y: i32,
                       dx: i32,
                       dy: i32,
                       data: &GameData) -> Collision {
    let mut last_pos = (x, y);
    let mut result: Collision = Collision::NoCollision(x + dx, y + dy);

    if !data.map.is_within_bounds(x + dx, y + dy) {
        result = Collision::Wall((x, y), (x, y));
    } else if data.map.is_blocked_by_wall(x, y, dx, dy) {
        // TODO this returns the final position, not the position of the wal
        // mayye need a block_by_wall function which returns this instead of a bool
        result = Collision::Wall((x + dx, y + dy), (x, y));
    } else {
        let move_line = Line::new((x, y), (x + dx, y + dy));

        for (x_pos, y_pos) in move_line.into_iter() {
            if is_blocked(x_pos, y_pos, data) {
                if data.map[(x_pos, y_pos)].blocked {
                    result = Collision::BlockedTile((x_pos, y_pos), last_pos);
                } else {
                    for (key, object) in data.objects.iter() {
                        if object.pos() == (x_pos, y_pos) {
                            result = Collision::Entity(key, last_pos);
                            break;
                        }
                    }
                }
                break;
            }

            last_pos = (x_pos, y_pos);
        }
    }

    return result;
}

pub fn player_move_or_attack(move_action: MoveAction, data: &mut GameData) -> PlayerTurn {
    use PlayerAction::*;

    let player_action: PlayerTurn;

    let player_handle = data.find_player().unwrap();

    let movement = calculate_move(move_action,
                                  data.objects[player_handle].movement.unwrap(),
                                  player_handle,
                                  data);

    match movement {
        Some(Movement::Attack(new_x, new_y, target_handle)) => {
            attack(player_handle, target_handle, &mut data.objects);

            // if we attack without moving, we lost all our momentum
            if (new_x, new_y) == (data.objects[player_handle].x, data.objects[player_handle].y)
            {
                data.objects[player_handle].momentum.as_mut().map(|momentum| momentum.clear());
            }

            data.objects[player_handle].set_pos(new_x, new_y);

            player_action = PlayerTurn::TookTurn(Move(movement.unwrap()));
        }

        Some(Movement::Collide(x, y)) => {
            data.objects[player_handle].set_pos(x, y);
            data.objects[player_handle].momentum.unwrap().clear();
            player_action = PlayerTurn::TookTurn(Move(movement.unwrap()));
        }

        Some(Movement::Move(x, y)) | Some(Movement::JumpWall(x, y)) => {
            let (dx, dy) = (x - data.objects[player_handle].x, y - data.objects[player_handle].y);

            data.objects[player_handle].set_pos(x, y);
            let momentum = data.objects[player_handle].momentum.unwrap();

            data.objects[player_handle].momentum.as_mut().map(|momentum| momentum.moved(dx, dy));

            if momentum.magnitude() > 1 && !momentum.took_half_turn {
                player_action = PlayerTurn::TookHalfTurn(Move(movement.unwrap()));
            } else if dx == 0 && dy == 0 {
                player_action = PlayerTurn::DidntTakeTurn;
            } else {
                player_action = PlayerTurn::TookTurn(Move(movement.unwrap()));
            }

            if player_action != PlayerTurn::DidntTakeTurn {
                data.objects[player_handle]
                    .momentum
                    .as_mut()
                    .map(|momentum| momentum.took_half_turn =
                         player_action == PlayerTurn::TookHalfTurn(Move(movement.unwrap())));
            }
        }

        Some(Movement::WallKick(x, y, dir_x, dir_y)) => {
            let mut momentum = data.objects[player_handle].momentum.unwrap();
            data.objects[player_handle].set_pos(x, y);
            momentum.set_momentum(dir_x, dir_y);

            /*
            let sprite_handle =
                display_state.lookup_sprite("player_wall_kick".to_string())
                             .expect("Could not find sprite 'player_wall_kick'");
            d,ta.objects[player_handle].animation = Some(sprite_handle);
            */

            // TODO could check for enemy and attack
            player_action = PlayerTurn::TookTurn(Move(movement.unwrap()));
        }

        None => {
            player_action = PlayerTurn::DidntTakeTurn;
        }
    }

    return player_action;
}

// TODO consider moving to GameData
pub fn calculate_move(action: MoveAction,
                      reach: Reach,
                      object_id: ObjectId,
                      data: &GameData) -> Option<Movement> {
    let movement: Option<Movement>;

    let (x, y) = data.objects[object_id].pos();
    if let Some(delta_pos) = reach.move_with_reach(&action) {
        let (dx, dy) = delta_pos.into_pair();
        // check if movement collides with a blocked location or an entity
        match check_collision(x, y, dx, dy, data) {
            Collision::NoCollision(new_x, new_y) => {
                // no collision- just move to location
                movement = Some(Movement::Move(new_x, new_y));
            }

            Collision::BlockedTile((_tile_x, _tile_y), (new_x, new_y)) => {
                movement = Some(Movement::Move(new_x, new_y));
            }

            Collision::Wall((tile_x, tile_y), (new_x, new_y)) => {
                match data.objects[object_id].momentum {
                    Some(momentum) => {
                        // if max momentum, the momentum is in the same direction as the movement,
                        // and there is space beyond the wall, than jump over the wall.
                        if momentum.at_maximum() &&
                           momentum.along(dx, dy) && 
                           !is_blocked(tile_x, tile_y, data) {
                                movement = Some(Movement::JumpWall(tile_x, tile_y));
                        } else { // otherwise move normally, stopping just before the blocking tile
                            movement = Some(Movement::Move(new_x, new_y));
                        }
                    },

                    None => {
                        // with no momentum, the movement will end just before the blocked location
                        movement = Some(Movement::Move(x + dx, y + dy));
                    },
                }
            }

            Collision::Entity(other_object_id, (new_x, new_y)) => {
                // record that an attack would occur. if this is not desired, the
                // calling code will handle this.
                movement = Some(Movement::Attack(new_x, new_y, other_object_id));
            }
        }
    } else {
        movement = None;

    }

    return movement;
}

// TODO consider moving to GameData
pub fn is_blocked(x: i32, y: i32, data: &GameData) -> bool {
    if !data.map.is_within_bounds(x, y) {
        return true;
    }

    if data.map[(x, y)].blocked {
        return true;
    }

    let mut is_blocked = false;
    for object in data.objects.values() {
        if object.blocks && object.pos() == (x, y) {
            is_blocked = true;
            break;
        }
    }

    return is_blocked;
}

pub fn direction(value: i32) -> i32 {
    if value == 0 {
        return 0;
    } else {
        return value.signum();
    }
}

