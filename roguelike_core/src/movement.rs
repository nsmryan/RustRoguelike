use std::iter::Iterator;
use std::fmt;

use tcod::line::*;

use euclid::*;

use crate::constants::*;
use crate::types::*;
use crate::utils::*;
use crate::messaging::{MsgLog, Msg};


pub type Loudness = usize;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Move(Movement),
    StateChange(Behavior),
    Pickup(ObjectId),
    ThrowStone(Pos, ObjectId),
    Yell,
    NoAction,
}

impl Action {
    pub fn none() -> Action {
        return Action::NoAction; }
}



#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoveMode {
    Sneak,
    Walk,
    Run,
}

impl MoveMode {
    pub fn increase(&self) -> MoveMode {
        match self {
            MoveMode::Sneak => MoveMode::Walk,
            MoveMode::Walk => MoveMode::Run,
            MoveMode::Run => MoveMode::Run,
        }
    }

    pub fn decrease(&self) -> MoveMode {
        match self {
            MoveMode::Sneak => MoveMode::Sneak,
            MoveMode::Walk => MoveMode::Sneak,
            MoveMode::Run => MoveMode::Walk,
        }
    }
}

impl fmt::Display for MoveMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MoveMode::Sneak => write!(f, "sneaking"),
            MoveMode::Walk => write!(f, "walking"),
            MoveMode::Run => write!(f, "running"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Movement {
    Move(Pos),
    Pass(Pos),
    Attack(Pos, ObjectId),
    Collide(Pos),
    WallKick(Pos, i32, i32), // (x, y), dir_x, dir_y
    JumpWall(Pos),
}

impl Movement {
    pub fn xy(&self) -> Pos {
        match self {
            Movement::Move(pos) => *pos,
            Movement::Pass(pos) => *pos,
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


#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Momentum {
    pub mx: i32,
    pub my: i32,
    pub max: i32,
}

impl Default for Momentum {
    fn default() -> Momentum {
        Momentum {
            mx: 0,
            my: 0,
            max: MAX_MOMENTUM,
        }
    }
}

impl Momentum {
    pub fn running(&mut self) -> bool {
        return self.magnitude() != 0;
    }

    pub fn at_maximum(&self) -> bool {
        return self.magnitude() == MAX_MOMENTUM;
    }
        
    pub fn magnitude(&self) -> i32 {
        if self.mx.abs() > self.my.abs() {
            return self.mx.abs();
        } else {
            return self.my.abs();
        }
    }

    pub fn diagonal(&self) -> bool {
        return self.mx.abs() != 0 && self.my.abs() != 0;
    }

    pub fn moved(&mut self, dx: i32, dy: i32) {
        // if the movement is in the opposite direction, and we have some momentum
        // currently, lose our momentum.

        if self.mx != 0 && dx.signum() != self.mx.signum() {
            self.mx = 0;
        } else {
            self.mx = clamp(self.mx + dx.signum(), -self.max, self.max);
        }

        if self.my != 0 && dy.signum() != self.my.signum() {
            self.my = 0;
        } else {
            self.my = clamp(self.my + dy.signum(), -self.max, self.max);
        }
    }

    pub fn set_momentum(&mut self, mx: i32, my: i32) {
        self.mx = mx;
        self.my = my;
    }

    pub fn along(&self, dx: i32, dy: i32) -> bool {
        return (self.mx * dx + self.my * dy) > 0;
    }

    pub fn clear(&mut self) {
        self.mx = 0;
        self.my = 0;
    }
}


#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Collision {
    NoCollision(Pos),
    BlockedTile(Pos, Pos), // current position, last position
    Wall(Pos),
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

            Collision::Wall(pos) => {
                last_pos = pos;
            }

            Collision::Entity(_, pos) => {
                last_pos = pos;
            }
        }

        return last_pos;
    }
}

pub fn player_move_or_attack(movement: Movement,
                             data: &mut GameData,
                             msg_log: &mut MsgLog) -> Action {
    use Action::*;

    let player_action: Action;

    let player_handle = data.find_player().unwrap();

    match movement {
        Movement::Attack(new_pos, target_handle) => {
            attack(player_handle, target_handle, &mut data.objects, msg_log);

            data.objects[player_handle].set_pos(new_pos);

            player_action = Move(movement);
        }

        Movement::Collide(pos) => {
            data.objects[player_handle].set_pos(pos);
            player_action = Move(movement);

            msg_log.log(Msg::Collided(player_handle, pos));
        }

        Movement::Pass(pos) => {
            player_action = Action::none();
            msg_log.log(Msg::Moved(player_handle, movement, pos));
        }

        Movement::Move(pos) | Movement::JumpWall(pos) => {
            let (x, y) = pos.to_tuple();
            let (dx, dy) = (x - data.objects[player_handle].x, y - data.objects[player_handle].y);

            // Update position and momentum
            data.objects[player_handle].set_pos(pos);

            player_action = Move(movement);

            if movement == Movement::Move(pos) {
                msg_log.log(Msg::Moved(player_handle, movement, pos));
            } else {
                msg_log.log(Msg::JumpWall(player_handle, pos));
            }
        }

        Movement::WallKick(pos, dir_x, dir_y) => {
            data.objects[player_handle].set_pos(pos);

            // TODO could check for enemy and attack
            player_action = Move(movement);

            msg_log.log(Msg::WallKick(player_handle, pos));
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

    if let Some(blocked) = data.map.is_blocked_by_wall(pos, dx, dy) {
        let block_pos = blocked.blocked_pos();
        result = Collision::Wall(block_pos);
    } else {
        let move_line = Line::new(pos.to_tuple(), (pos.x + dx, pos.y + dy));

        for pos in move_line.into_iter() {
            let pos = Pos::from(pos);
            if data.is_blocked_tile(pos) {
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

                // if didn't move, pass turn
                if dx == 0 && dy == 0 {
                    movement = Some(Movement::Pass(pos));
                } else {
                    movement = Some(Movement::Move(new_pos));
                }
            }

            Collision::BlockedTile(_tile_pos, new_pos) => {
                movement = Some(Movement::Move(new_pos));
            }

            Collision::Wall(new_pos) => {
                if data.objects[object_id].move_mode.unwrap() == MoveMode::Run {
                    if !data.is_blocked_tile(new_pos) {
                        // TODO likely this does not work anymore
                        movement = Some(Movement::JumpWall(new_pos));
                    } else { // otherwise move normally, stopping just before the blocking tile
                        movement = Some(Movement::Move(new_pos));
                    }
                } else {
                    movement = Some(Movement::Move(new_pos));
                }
            }

            Collision::Entity(other_object_id, new_pos) => {
                if data.objects[other_object_id].alive {
                    // record that an attack would occur. if this is not desired, the
                    // calling code will handle this.
                    movement = Some(Movement::Attack(new_pos, other_object_id));
                } else {
                    movement = None;
                }
            }
        }
    } else {
        movement = None;
    }

    return movement;
}

pub fn direction(value: i32) -> i32 {
    if value == 0 {
        return 0;
    } else {
        return value.signum();
    }
}

