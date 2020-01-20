use std::iter::Iterator;

use tcod::line::*;

use euclid::*;

use crate::types::*;
use crate::ai::*;


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Movement {
    Move(Pos),
    Attack(Pos, ObjectId),
    Collide(Pos),
    WallKick(Pos, i32, i32), // x, y, dir_x, dir_y
    JumpWall(Pos),
}

impl Movement {
    pub fn xy(&self) -> Pos {
        match self {
            Movement::Move(pos) => *pos,
            Movement::Attack(pos, _) => *pos,
            Movement::Collide(pos) => *pos,
            Movement::WallKick(pos, _, _) => *pos,
            Movement::JumpWall(pos) => *pos,
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
    pub fn move_with_reach(&self, move_action: &MoveAction) -> Option<Pos> {
        match self {
            Reach::Single(dist) => {
                let dist = (*dist) as i32;
                let neg_dist = dist * -1;
                match move_action {
                    MoveAction::Left => Some(Pos::new(neg_dist, 0)),
                    MoveAction::Right => Some(Pos::new(dist, 0)),
                    MoveAction::Up => Some(Pos::new(0, neg_dist)),
                    MoveAction::Down => Some(Pos::new(0, dist)),
                    MoveAction::DownLeft => Some(Pos::new(neg_dist, dist)),
                    MoveAction::DownRight => Some(Pos::new(dist, dist)),
                    MoveAction::UpLeft => Some(Pos::new(neg_dist, neg_dist)),
                    MoveAction::UpRight => Some(Pos::new(dist, neg_dist)),
                    MoveAction::Center => Some(Pos::new(0, 0)),
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
                    MoveAction::DownLeft => Some(Pos::new(neg_dist, dist)),
                    MoveAction::DownRight => Some(Pos::new(dist, dist)),
                    MoveAction::UpLeft => Some(Pos::new(neg_dist, neg_dist)),
                    MoveAction::UpRight => Some(Pos::new(dist, neg_dist)),
                    MoveAction::Center => Some(Pos::new(0, 0)),
                }
            }

            Reach::Horiz(dist) => {
                let dist = (*dist) as i32;
                let neg_dist = dist * -1;
                match move_action {
                    MoveAction::Left => Some(Pos::new(neg_dist, 0)),
                    MoveAction::Right => Some(Pos::new(dist, 0)),
                    MoveAction::Up => Some(Pos::new(0, neg_dist)),
                    MoveAction::Down => Some(Pos::new(0, dist)),
                    MoveAction::DownLeft => None,
                    MoveAction::DownRight => None,
                    MoveAction::UpLeft => None,
                    MoveAction::UpRight => None,
                    MoveAction::Center => None,
                }
            }
        }
    }

    pub fn offsets(&self) -> Vec<Pos> {
        match self {
            Reach::Single(dist) => {
                let dist = (*dist) as i32;
                let offsets =
                    vec!( (0, dist),      (-dist, dist), (-dist,  0),
                          (-dist, -dist), (0,  -dist),   (dist, -dist),
                          (dist,  0), (dist, dist));
                offsets.iter().map(|pair| Pos::from(*pair)).collect()
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
                offsets.iter().map(|pair| Pos::from(*pair)).collect()
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
                offsets.iter().map(|pair| Pos::from(*pair)).collect()
            },
        }
    }
}


#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Collision {
    NoCollision(Pos),
    BlockedTile(Pos, Pos),
    Wall(Pos, Pos),
    Entity(ObjectId, Pos),
}

impl Collision {
    pub fn no_collsion(&self) -> bool {
        match self {
            Collision::NoCollision(_) => true,
            _ => false,
        }
    }

    pub fn move_location(&self) -> Pos {
        let last_pos;

        match *self {
            Collision::NoCollision(pos) => {
                last_pos = pos
            }

            Collision::BlockedTile(_, pos) => {
                last_pos = pos;
            }

            Collision::Wall(_, pos) => {
                last_pos = pos;
            }

            Collision::Entity(_, pos) => {
                last_pos = pos;
            }
        }

        return last_pos;
    }
}

pub fn player_move_or_attack(move_action: MoveAction, data: &mut GameData) -> Action {
    use Action::*;

    let player_action: Action;

    let player_handle = data.find_player().unwrap();

    let movement = calculate_move(move_action,
                                  data.objects[player_handle].movement.unwrap(),
                                  player_handle,
                                  data);

    match movement {
        Some(Movement::Attack(new_pos, target_handle)) => {
            attack(player_handle, target_handle, &mut data.objects);

            // if we attack without moving, we lost all our momentum
            if new_pos == data.objects[player_handle].pos() {
                data.objects[player_handle].momentum.as_mut().map(|momentum| momentum.clear());
            }

            data.objects[player_handle].set_pos(new_pos);

            player_action = Move(movement.unwrap());
        }

        Some(Movement::Collide(pos)) => {
            data.objects[player_handle].set_pos(pos);
            data.objects[player_handle].momentum.unwrap().clear();
            player_action = Move(movement.unwrap());
        }

        Some(Movement::Move(pos)) | Some(Movement::JumpWall(pos)) => {
            let (x, y) = pos.to_tuple();
            let (dx, dy) = (x - data.objects[player_handle].x, y - data.objects[player_handle].y);

            // Update position and momentum
            data.objects[player_handle].set_pos(pos);
            let momentum = data.objects[player_handle].momentum.unwrap();

            data.objects[player_handle].momentum.as_mut().map(|momentum| momentum.moved(dx, dy));

            // Resolve half-turn mechanic
            if momentum.magnitude() > 1 && !momentum.took_half_turn {
                player_action = Move(movement.unwrap());
            } else if dx == 0 && dy == 0 {
                player_action = Action::none();
            } else {
                player_action = Move(movement.unwrap());
            }

            // update half turn flag
            if player_action != Action::NoAction {
                data.objects[player_handle]
                    .momentum
                    .as_mut()
                    .map(|momentum| momentum.took_half_turn =
                         player_action == Move(movement.unwrap()));

                // Set up sound for movement
                let momentum_amount = data.objects[player_handle].momentum.unwrap();
                let mut sound = Object::new(x, y, ' ' as char, Color::white(), "sound", false);
                sound.animation = Some(Animation::Sound(0, momentum_amount.magnitude() as usize));
                data.objects.insert(sound);
            }
        }

        Some(Movement::WallKick(pos, dir_x, dir_y)) => {
            let mut momentum = data.objects[player_handle].momentum.unwrap();
            data.objects[player_handle].set_pos(pos);
            momentum.set_momentum(dir_x, dir_y);

            /*
            let sprite_handle =
                display_state.lookup_sprite("player_wall_kick".to_string())
                             .expect("Could not find sprite 'player_wall_kick'");
            d,ta.objects[player_handle].animation = Some(sprite_handle);
            */

            // TODO could check for enemy and attack
            player_action = Move(movement.unwrap());
        }

        None => {
            player_action = Action::none();
        }
    }

    return player_action;
}

// TODO consider moving to GameData
/// Moves the given object with a given offset, returning the square that it collides with, or None
/// indicating no collision.
pub fn check_collision(pos: Pos,
                       dx: i32,
                       dy: i32,
                       data: &GameData) -> Collision {
    let mut last_pos = pos;
    let mut result: Collision = Collision::NoCollision(pos + Vector2D::new(dx, dy));

    if !data.map.is_within_bounds(Pos::new(pos.x + dx, pos.y + dy)) {
        result = Collision::Wall(pos, pos);
    } else if data.map.is_blocked_by_wall(pos, dx, dy) {
        // TODO this returns the final position, not the position of the wal
        // mayye need a block_by_wall function which returns this instead of a bool
        result = Collision::Wall(pos + Vector2D::new(dx, dy), pos);
    } else {
        let move_line = Line::new(pos.to_tuple(), (pos.x + dx, pos.y + dy));

        for pos in move_line.into_iter() {
            let pos = Pos::from(pos);
            if is_blocked(pos, data) {
                if data.map[pos].blocked {
                    result = Collision::BlockedTile(pos, last_pos);
                } else {
                    for (key, object) in data.objects.iter() {
                        if object.pos() == pos {
                            result = Collision::Entity(key, last_pos);
                            break;
                        }
                    }
                }
                break;
            }

            last_pos = pos;
        }
    }

    return result;
}

pub fn calculate_move(action: MoveAction,
                      reach: Reach,
                      object_id: ObjectId,
                      data: &GameData) -> Option<Movement> {
    let movement: Option<Movement>;

    let pos = data.objects[object_id].pos();

    if let Some(delta_pos) = reach.move_with_reach(&action) {
        let (dx, dy) = delta_pos.to_tuple();
        // check if movement collides with a blocked location or an entity
        match check_collision(pos, dx, dy, data) {
            Collision::NoCollision(new_pos) => {
                // no collision- just move to location
                movement = Some(Movement::Move(new_pos));
            }

            Collision::BlockedTile(tile_pos, new_pos) => {
                movement = Some(Movement::Move(new_pos));
            }

            Collision::Wall(tile_pos, new_pos) => {
                match data.objects[object_id].momentum {
                    Some(momentum) => {
                        // if max momentum, the momentum is in the same direction as the movement,
                        // and there is space beyond the wall, than jump over the wall.
                        if momentum.at_maximum() &&
                           momentum.along(dx, dy) && 
                           !is_blocked(tile_pos, data) {
                                movement = Some(Movement::JumpWall(tile_pos));
                        } else { // otherwise move normally, stopping just before the blocking tile
                            movement = Some(Movement::Move(new_pos));
                        }
                    },

                    None => {
                        // with no momentum, the movement will end just before the blocked location
                        movement = Some(Movement::Move(Pos::new(pos.x + dx, pos.y + dy)));
                    },
                }
            }

            Collision::Entity(other_object_id, new_pos) => {
                // record that an attack would occur. if this is not desired, the
                // calling code will handle this.
                movement = Some(Movement::Attack(new_pos, other_object_id));
            }
        }
    } else {
        movement = None;

    }

    return movement;
}

pub fn is_blocked(pos: Pos, data: &GameData) -> bool {
    if !data.map.is_within_bounds(pos) {
        return true;
    }

    if data.map[pos].blocked {
        return true;
    }

    let mut is_blocked = false;
    for object in data.objects.values() {
        if object.blocks && object.pos() == pos {
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

