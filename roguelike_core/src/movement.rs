use std::iter::Iterator;
use std::fmt;

use tcod::line::*;

use euclid::*;

use crate::constants::*;
use crate::types::*;
use crate::utils::*;
use crate::map::{Wall, Blocked};
use crate::messaging::{MsgLog, Msg};
use crate::ai::Behavior;


pub type Loudness = usize;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Move(Movement),
    StateChange(Behavior),
    Pickup(ObjectId),
    ThrowItem(Pos, usize), // end position, inventory index
    Pass,
    Yell,
    NoAction,
}

impl Action {
    pub fn none() -> Action {
        return Action::NoAction;
    }

    pub fn is_none(&self) -> bool {
        return *self == Action::NoAction;
    }
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
pub enum Attack {
    Attack(ObjectId), // target_id
    Push(ObjectId, Pos), //target_id, delta_pos
    Stab(ObjectId), // target_id
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoveType {
    Move,
    Pass,
    JumpWall,
    WallKick(i32, i32),
    Collide,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Movement {
    pub pos: Pos,
    pub typ: MoveType,
    pub attack: Option<Attack>,
}

impl Movement {
    pub fn move_to(pos: Pos, typ: MoveType) -> Movement {
        return Movement {
            pos,
            typ,
            attack: None,
        };
    }

    pub fn attack(pos: Pos, typ: MoveType, attack: Attack) -> Movement {
        return Movement {
            pos,
            typ,
            attack: Some(attack),
        };
    }
}


#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd)]
pub enum Cardinal {
    Up,
    Down,
    Left,
    Right
}

impl Cardinal {
    pub fn from_dxy(last: Option<Cardinal>, dx: i32, dy: i32) -> Option<Cardinal> {
        if dx == 0 && dy == 0 {
            None
        } else if dx == 0 && dy < 0 {
            Some(Cardinal::Up)
        } else if dx == 0 && dy > 0 {
            Some(Cardinal::Down)
        } else if dx > 0 && dy == 0 {
            Some(Cardinal::Right)
        } else if dx < 0 && dy == 0 {
            Some(Cardinal::Left)
        } else {
            // NOTE this makes diagonal moves always create a certain facing.
            // could use previous position as well.
            if let Some(_dir) = last {
                if dx > 0 && dy > 0 {
                    Some(Cardinal::Right)
                } else if dx > 0 && dy < 0 {
                    Some(Cardinal::Right)
                } else if dx < 0 && dy > 0 {
                    Some(Cardinal::Left)
                } else if dx < 0 && dy < 0 {
                    Some(Cardinal::Left)
                } else {
                    None
                }
            } else {
                None
            }
        }
    }

    pub fn x_signum(&self) -> i32 {
        match self {
            Cardinal::Up => 0,
            Cardinal::Down => 0,
            Cardinal::Left => -1,
            Cardinal::Right => 1,
        }
    }

    pub fn y_signum(&self) -> i32 {
        match self {
            Cardinal::Up => -1,
            Cardinal::Down => 1,
            Cardinal::Left => 0,
            Cardinal::Right => 0,
        }
    }
}


#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
    DownLeft,
    DownRight,
    UpLeft,
    UpRight,
}

impl Direction {
    pub fn from_dxy(dx: i32, dy: i32) -> Option<Direction> {
        if dx == 0 && dy == 0 {
            None
        } else if dx == 0 && dy < 0 {
            Some(Direction::Up)
        } else if dx == 0 && dy > 0 {
            Some(Direction::Down)
        } else if dx > 0 && dy == 0 {
            Some(Direction::Right)
        } else if dx < 0 && dy == 0 {
            Some(Direction::Left)
        } else if dx > 0 && dy > 0 {
            Some(Direction::DownRight)
        } else if dx > 0 && dy < 0 {
            Some(Direction::UpRight)
        } else if dx < 0 && dy > 0 {
            Some(Direction::DownLeft)
        } else if dx < 0 && dy < 0 {
            Some(Direction::UpLeft)
        } else {
            panic!(format!("Direction should not exist {:?}", (dx, dy)));
        }
    }

    pub fn into_move(self) -> (i32, i32) {
        match self {
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
            Direction::Up => (0, -1),
            Direction::Down => (0, 1),
            Direction::DownLeft => (-1, 1),
            Direction::DownRight => (1, 1),
            Direction::UpLeft => (-1, -1),
            Direction::UpRight => (1, -1),
        }
    }

    pub fn move_actions() -> Vec<Direction> {
        return vec!(Direction::Left,
                    Direction::Right,
                    Direction::Up,
                    Direction::Down,
                    Direction::DownLeft,
                    Direction::DownRight,
                    Direction::UpLeft,
                    Direction::UpRight);
    }

    pub fn from_f32(flt: f32) -> Direction {
        let index = (flt * 8.0) as usize;
        let dirs = Direction::move_actions();
        return dirs[index];
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reach {
    Single(usize),
    Diag(usize),
    Horiz(usize),
}

impl Reach {
    pub fn move_with_reach(&self, move_action: &Direction) -> Option<Pos> {
        match self {
            Reach::Single(dist) => {
                let dist = (*dist) as i32;
                let neg_dist = dist * -1;
                match move_action {
                    Direction::Left => Some(Pos::new(neg_dist, 0)),
                    Direction::Right => Some(Pos::new(dist, 0)),
                    Direction::Up => Some(Pos::new(0, neg_dist)),
                    Direction::Down => Some(Pos::new(0, dist)),
                    Direction::DownLeft => Some(Pos::new(neg_dist, dist)),
                    Direction::DownRight => Some(Pos::new(dist, dist)),
                    Direction::UpLeft => Some(Pos::new(neg_dist, neg_dist)),
                    Direction::UpRight => Some(Pos::new(dist, neg_dist)),
                }
            }

            Reach::Diag(dist) => {
                let dist = (*dist) as i32;
                let neg_dist = dist * -1;
                match move_action {
                    Direction::Left => None,
                    Direction::Right => None,
                    Direction::Up => None,
                    Direction::Down => None,
                    Direction::DownLeft => Some(Pos::new(neg_dist, dist)),
                    Direction::DownRight => Some(Pos::new(dist, dist)),
                    Direction::UpLeft => Some(Pos::new(neg_dist, neg_dist)),
                    Direction::UpRight => Some(Pos::new(dist, neg_dist)),
                }
            }

            Reach::Horiz(dist) => {
                let dist = (*dist) as i32;
                let neg_dist = dist * -1;
                match move_action {
                    Direction::Left => Some(Pos::new(neg_dist, 0)),
                    Direction::Right => Some(Pos::new(dist, 0)),
                    Direction::Up => Some(Pos::new(0, neg_dist)),
                    Direction::Down => Some(Pos::new(0, dist)),
                    Direction::DownLeft => None,
                    Direction::DownRight => None,
                    Direction::UpLeft => None,
                    Direction::UpRight => None,
                }
            }
        }
    }

    pub fn offsets(&self) -> Vec<Pos> {
        let end_points: Vec<Pos>;

        match self {
            Reach::Single(dist) => {
                let dist = (*dist) as i32;
                let offsets =
                    vec!( (0, dist),      (-dist, dist), (-dist,  0),
                          (-dist, -dist), (0,  -dist),   (dist, -dist),
                          (dist,  0), (dist, dist));
                end_points = offsets.iter().map(|pair| Pos::from(*pair)).collect();
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
                end_points = offsets.iter().map(|pair| Pos::from(*pair)).collect();
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
                end_points = offsets.iter().map(|pair| Pos::from(*pair)).collect();
            },
        }

        let mut offsets = Vec::new();
        for end in end_points {
            for pos in Line::new((0, 0), end.to_tuple()) {
                offsets.push(Pos::from(pos));
            }
        }

        return offsets;
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


#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MoveResult {
    entity: Option<ObjectId>,
    blocked: Option<Blocked>,
    move_pos: Pos,
}

impl MoveResult {
    pub fn with_pos(pos: Pos) -> MoveResult {
        return MoveResult {
            entity: None,
            blocked: None,
            move_pos: pos
        };
    }

    pub fn no_collsion(&self) -> bool {
        return self.blocked.is_none() && self.entity.is_none();
    }
}

pub fn player_move_or_attack(movement: Movement,
                             data: &mut GameData,
                             msg_log: &mut MsgLog) -> Action {
    use Action::*;

    let player_action: Action;

    let player_id = data.find_player().unwrap();

    match movement.attack {
        None => {
            match movement.typ {
                MoveType::Collide => {
                    data.objects[player_id].move_to(movement.pos);
                    player_action = Move(movement);

                    msg_log.log(Msg::Collided(player_id, movement.pos));
                }

                MoveType::Pass => {
                    player_action = Action::none();
                    msg_log.log(Msg::Moved(player_id, movement, movement.pos));
                }

                MoveType::Move | MoveType::JumpWall => {
                    let player_pos = data.objects[player_id].pos();

                    if player_pos != movement.pos {
                        data.objects[player_id].move_to(movement.pos);

                        player_action = Move(movement);

                        if movement.typ == MoveType::Move {
                            msg_log.log(Msg::Moved(player_id, movement, movement.pos));
                        } else {
                            msg_log.log(Msg::JumpWall(player_id, player_pos, movement.pos));
                        }
                    } else {
                        player_action = NoAction;
                    }
                }

                MoveType::WallKick(_dir_x, _dir_y) => {
                    data.objects[player_id].move_to(movement.pos);

                    // TODO could check for enemy and attack
                    player_action = Move(movement);

                    msg_log.log(Msg::WallKick(player_id, movement.pos));
                }
            }
        }

        Some(Attack::Push(target_id, delta_pos)) => {
            if data.objects[target_id].typ == ObjType::Column {
                let pos = data.objects[player_id].pos();
                    let next_pos = next_pos(pos, sub_pos(movement.pos, pos));

                // if there is a path to the next tile, move it.
                let diff = sub_pos(movement.pos, pos);
                let blocked =
                    data.map.is_blocked_by_wall(movement.pos, diff.x, diff.y); 

                if blocked == None {
                    data.objects[player_id].move_to(movement.pos);

                    data.objects.remove(target_id);

                    if let Some(hit_entity) = data.is_blocked_tile(next_pos) {
                        crush(target_id, hit_entity, &mut data.objects, msg_log);
                    }

                    player_action = Move(movement);

                    msg_log.log(Msg::Crushed(player_id, next_pos, ObjType::Column));
                } else {
                    player_action = NoAction;
                }
            } else if data.objects[target_id].alive {
                push_attack(player_id, target_id, delta_pos, data, msg_log);
                player_action = Move(movement);
            } else {
                dbg!(data.objects[target_id].typ);
                //player_action = NoAction;
                panic!("What did you push?");
            }
        }

        Some(Attack::Attack(target_id)) => {
            attack(player_id, target_id, data, msg_log);

            let target_pos = data.objects[target_id].pos();
            data.objects[player_id].move_next_to(target_pos);

            player_action = Move(movement);
        }

        Some(Attack::Stab(target_id)) => {
            // if enemy is aware of the enemy, just push instead
            if data.objects[target_id].behavior.map_or(false, |beh| beh.is_aware()) {
                panic!("This shouldn't actually be possible- stabbing a aware enemy");
            } else {
                // otherwise enemy is not aware, so stab them
                stab(player_id, target_id, &mut data.objects, msg_log);
            }

            // dagger is one use only- remove it from inventory
            let dagger_ix =
                data.objects[player_id]
                    .inventory
                    .iter()
                    .position(|item| data.objects[*item].item == Some(Item::Dagger))
                    .expect("Stabbed without a dagger!");
            data.objects[player_id].inventory.remove(dagger_ix);

            data.objects[player_id].move_to(movement.pos);

            player_action = Move(movement);
        }
    }

    return player_action;
}

/// Moves the given object with a given offset, returning the square that it collides with, or None
/// indicating no collision.
/// NOTE if the movement carries multiple tiles, then the resulting MoveResult can report that
/// there is a blocking wall, and on a different location a blocking entity. These are checked
/// separately.
pub fn check_collision(pos: Pos,
                       dx: i32,
                       dy: i32,
                       data: &GameData) -> MoveResult {
    let mut last_pos = pos;
    let mut result: MoveResult =
        MoveResult::with_pos(pos + Vector2D::new(dx, dy));

    // if no movement occurs, no need to check walls and entities.
    if !(dx == 0 && dy == 0) {
        if let Some(blocked) = data.map.is_blocked_by_wall(pos, dx, dy) {
            result.blocked = Some(blocked);
            result.move_pos = blocked.start_pos;
        } 

        // check for collision with an enitity
        let move_line = Line::new(pos.to_tuple(), (pos.x + dx, pos.y + dy));

        for line_tuple in move_line {
            let line_pos = Pos::from(line_tuple);

            if let Some(key) = data.is_blocked_tile(line_pos) {
                result.move_pos = last_pos;
                result.entity = Some(key);
                break;
            }

            // if we are blocked by a wall, and the current position is at that blocked
            // position, we don't need to continue the search
            if let Some(blocked) = result.blocked {
                if line_pos == blocked.start_pos {
                    break;
                }
            }

            last_pos = pos;
        }
    }

    return result;
}

pub fn calculate_move(action: Direction,
                      reach: Reach,
                      object_id: ObjectId,
                      data: &GameData) -> Option<Movement> {
    let mut movement: Option<Movement>;

    let pos = data.objects[object_id].pos();

    // get the location we would move to given the input action
    if let Some(delta_pos) = reach.move_with_reach(&action) {
        let (dx, dy) = delta_pos.to_tuple();

        // check if movement collides with a blocked location or an entity
        let move_result = check_collision(pos, dx, dy, data);

        match (move_result.blocked, move_result.entity) {
            // both blocked by wall and by entity
            (Some(blocked), Some(entity)) => {
                let entity_pos = data.objects[entity].pos();

                let entity_dist = distance(pos, entity_pos);
                let wall_dist = distance(pos, blocked.start_pos);

                if entity_dist < wall_dist {
                    let attack =
                        if can_stab(data, object_id, entity) {
                            Attack::Stab(entity)
                        } else {
                            Attack::Push(entity, delta_pos)
                        };
                    let move_pos = move_next_to(pos, entity_pos);
                    movement = Some(Movement::attack(move_pos, MoveType::Move, attack));
                } else if entity_dist > wall_dist {
                    // wall is first
                    let mut jumped_wall = false;
                    if data.objects[object_id].move_mode.unwrap() == MoveMode::Run {
                        if !blocked.blocked_tile && blocked.wall_type == Wall::ShortWall {
                            jumped_wall = true;
                        } 
                    }

                    if jumped_wall {
                        let attack =
                            if can_stab(data, object_id, entity) {
                                Attack::Stab(entity)
                            } else {
                                Attack::Push(entity, delta_pos)
                            };
                        let move_pos = move_next_to(pos, entity_pos);
                        movement = Some(Movement::attack(move_pos, MoveType::Move, attack));
                    } else {
                        // can't jump the wall- just move up to it.
                        movement = Some(Movement::move_to(blocked.start_pos, MoveType::Move));
                    }
                } else {
                    // entity and wall are together
                    // move up to the wall- we can't jump it or attack through it
                    movement = Some(Movement::move_to(blocked.start_pos, MoveType::Move));
                }
            }

            // blocked by entity only
            (None, Some(entity)) => {
                if can_stab(data, object_id, entity) {
                    let attack = Attack::Stab(entity);
                    movement = Some(Movement::attack(move_result.move_pos, MoveType::Move, attack));
                    dbg!();
                } else if data.objects[entity].blocks {
                    let attack = Attack::Push(entity, delta_pos);
                    movement = Some(Movement::attack(add_pos(pos, delta_pos), MoveType::Move, attack));
                    dbg!();
                } else {
                    movement = Some(Movement::move_to(move_result.move_pos, MoveType::Move));
                    dbg!();
                }
            }

            // blocked by wall only
            (Some(blocked), None) => {
                let mut jumped_wall = false;

                if data.objects[object_id].move_mode.unwrap() == MoveMode::Run {
                    if !blocked.blocked_tile && blocked.wall_type == Wall::ShortWall {
                        jumped_wall = true;
                    } 
                }

                if jumped_wall {
                    // if we jump the wall, we have to recheck for collisions for the
                    // remaining move distance.
                    let (dx, dy) = dxy(blocked.end_pos, add_pos(pos, delta_pos));
                    let next_move_result = check_collision(blocked.end_pos, dx, dy, data);
                    let new_pos = next_move_result.move_pos;

                    movement = Some(Movement::move_to(new_pos, MoveType::JumpWall));

                    let next_pos = next_pos(pos, delta_pos);
                    if let Some(other_id) = data.is_blocked_tile(next_pos) {
                        if can_stab(data, object_id, other_id) {
                           let attack = Attack::Stab(other_id);
                           movement = Some(Movement::attack(new_pos, MoveType::JumpWall, attack));
                       }
                    }
                } else {
                    // else move up to the wall (start_pos is just before the colliding tile)
                    movement = Some(Movement::move_to(blocked.start_pos, MoveType::Move));
                }
            }

            // not blocked at all
            (None, None) => {
                let next_pos = next_pos(pos, delta_pos);
                if let Some(other_id) = data.is_blocked_tile(next_pos) {
                    if can_stab(data, object_id, other_id) {
                       let attack = Attack::Stab(other_id);
                       movement = Some(Movement::attack(move_result.move_pos, MoveType::Move, attack));
                   } else {
                      movement = Some(Movement::move_to(move_result.move_pos, MoveType::Move));
                   }
                } else {
                  movement = Some(Movement::move_to(move_result.move_pos, MoveType::Move));
                }
            }
        }
    } else {
        // movement is not valid given the mover's reach- reject movement by return None
        movement = None;
    }

    if let Some(moved) = movement {
        if moved.attack == None && moved.pos == pos {
            movement = None;
        }
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

