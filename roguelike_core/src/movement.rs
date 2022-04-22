use std::iter::Iterator;
use std::fmt;
use std::str::FromStr;

use euclid::*;

use serde::{Serialize, Deserialize};

use roguelike_utils::line::*;
use roguelike_utils::comp::*;

use crate::types::*;
use crate::utils::*;
use crate::map::{Wall, Blocked, TileType, Pos, Direction};
use crate::level::*;


#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum MoveMode {
    Sneak,
    Walk,
    Run,
}

impl fmt::Display for MoveMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MoveMode::Sneak => write!(f, "sneak"),
            MoveMode::Walk => write!(f, "walk"),
            MoveMode::Run => write!(f, "run"),
        }
    }
}

impl FromStr for MoveMode {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();
        if s == "sneak" {
            return Ok(MoveMode::Sneak);
        } else if s == "walk" {
            return Ok(MoveMode::Walk);
        } else if s == "run" {
            return Ok(MoveMode::Run);
        }

        return Err(format!("Could not parse '{}' as MoveMode", s));
    }
}

impl Default for MoveMode {
    fn default() -> MoveMode {
        return MoveMode::Sneak;
    }
}

impl MoveMode {
    pub fn increase(&self) -> MoveMode {
        match self {
            MoveMode::Sneak => MoveMode::Walk,
            MoveMode::Run => MoveMode::Run,
            MoveMode::Walk => MoveMode::Run,
        }
    }

    pub fn decrease(&self) -> MoveMode {
        match self {
            MoveMode::Sneak => MoveMode::Sneak,
            MoveMode::Run => MoveMode::Walk,
            MoveMode::Walk => MoveMode::Sneak,
        }
    }

    /*
    pub fn move_amount(&self) -> usize {
        match self {
            MoveMode::Sneak => 1,
            MoveMode::Walk => 1,
            MoveMode::Run => 2,
        }
    }
    */
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Attack {
    Attack(EntityId), // target_id
    Push(EntityId, Direction, usize), //target_id, direction, amount
    Stab(EntityId, bool), // target_id, move into space
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum AttackType {
    Melee,
    Ranged,
    Push,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum MoveType {
    Move,
    Pass,
    JumpWall,
    WallKick,
    Collide,
    Blink,
    Misc,
}

impl fmt::Display for MoveType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MoveType::Move => write!(f, "move"),
            MoveType::Pass => write!(f, "pass"),
            MoveType::JumpWall => write!(f, "jumpwall"),
            MoveType::WallKick => write!(f, "wallkick"),
            MoveType::Collide => write!(f, "collide"),
            MoveType::Blink => write!(f, "blink"),
            MoveType::Misc => write!(f, "misc"),
        }
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub struct Movement {
    pub pos: Pos,
    pub typ: MoveType,
    pub attack: Option<Attack>,
    pub wall: Option<Pos>,
}

impl Default for Movement {
    fn default() -> Movement {
        return Movement {
            pos: Pos::new(0, 0),
            typ: MoveType::Pass,
            attack: None,
            wall: None,
        };
    }
}

impl Movement {
    pub fn step_to(pos: Pos) -> Movement {
        return Movement {
            pos: pos,
            typ: MoveType::Move,
            attack: None,
            wall: None,
        };
    }

    pub fn new(pos: Pos, typ: MoveType, attack: Option<Attack>) -> Movement {
        return Movement {
            pos,
            typ,
            attack,
            wall: None,
        };
    }

    pub fn pass(pos: Pos) -> Movement {
        return Movement::new(pos, MoveType::Pass, None);
    }

    pub fn move_to(pos: Pos, typ: MoveType) -> Movement {
        return Movement {
            pos,
            typ,
            attack: None,
            wall: None,
        };
    }

    pub fn attack(pos: Pos, typ: MoveType, attack: Attack) -> Movement {
        return Movement {
            pos,
            typ,
            attack: Some(attack),
            wall: None,
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


#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Reach {
    Single(usize),
    Diag(usize),
    Horiz(usize),
}

impl Reach {
    pub fn new() -> Reach {
        return Reach::Single(1);
    }

    pub fn single(dist: usize) -> Reach {
        return Reach::Single(dist);
    }

    pub fn diag(dist: usize) -> Reach {
        return Reach::Diag(dist);
    }

    pub fn horiz(dist: usize) -> Reach {
        return Reach::Horiz(dist);
    }

    pub fn dist(&self) -> usize {
        match self {
            Reach::Single(dist) => *dist,
            Reach::Diag(dist) => *dist,
            Reach::Horiz(dist) => *dist,
        }
    }

    pub fn with_dist(&self, dist: usize) -> Reach {
        match self {
            Reach::Single(_) => Reach::Single(dist),
            Reach::Diag(_) => Reach::Diag(dist),
            Reach::Horiz(_) => Reach::Horiz(dist),
        }
    }

    pub fn furthest_in_direction(&self, pos: Pos, dir: Direction) -> Option<Pos> {
        let valid = 
            match self {
                Reach::Diag(_) => dir.diag(),
                Reach::Horiz(_) => dir.horiz(),
                Reach::Single(_) => true,
            };

        if valid {
            return Some(dir.offset_pos(pos, self.dist() as i32));
        } else {
            return None;
        }
    }

    pub fn closest_to(&self, pos: Pos, other: Pos) -> Pos {
        let offsets = self.offsets();

        let mut closest: Pos = *offsets.get(0).expect(&format!("Reach had 0 options {:?}?", self));

        for offset in offsets {
            let other_pos = add_pos(pos, offset);
            if distance(other, other_pos) < distance(other, closest) {
                closest = other_pos;
            }
        }

        return closest;
    }

    pub fn attacks_with_reach(&self, move_action: &Direction) -> Vec<Pos> {
        let mut positions = Vec::new();

        if let Some(pos) = self.move_with_reach(move_action) {
            for pos in line_inclusive(Pos::new(0, 0), pos) {
                positions.push(Pos::from(pos));
            }
        }

        return positions;
    }

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

    pub fn reachables(&self, start: Pos) -> Vec<Pos> {
        let offsets = self.offsets();
        return offsets.iter()
                      .map(|off| add_pos(start, *off))
                      .collect::<Vec<Pos>>();
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
                for dist in 1..=dist {
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
                for dist in 1..=dist {
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
            for pos in line_inclusive(Pos::new(0, 0), end) {
                if !offsets.contains(&pos) {
                    offsets.push(Pos::from(pos));
                }
            }
        }

        return offsets;
    }
}

#[test]
pub fn test_reach_offsets_horiz() {
    let horiz = Reach::Horiz(1);
    let offsets = horiz.offsets();

    let expected_pos =
        vec!((1, 0), (-1, 0), (0, 1), (0, -1)).iter()
                                              .map(|p| Pos::from(*p))
                                              .collect::<Vec<Pos>>();
    assert!(offsets.iter().all(|p| expected_pos.iter().any(|other| other == p)));
}

#[test]
pub fn test_reach_offsets_diag() {
    let horiz = Reach::Diag(1);
    let offsets = horiz.offsets();

    let expected_pos =
        vec!((1, 1), (-1, 1), (1, -1), (-1, -1)).iter()
                                              .map(|p| Pos::from(*p))
                                              .collect::<Vec<Pos>>();
    assert!(offsets.iter().all(|p| expected_pos.iter().any(|other| other == p)));
}

#[test]
pub fn test_reach_offsets_single() {
    let horiz = Reach::Single(1);
    let offsets = horiz.offsets();

    let expected_pos_vec =
        vec!((1, 0), (0, 1), (-1, 0), (0, -1), (1, 1), (-1, 1), (1, -1), (-1, -1));

    let expected_pos = expected_pos_vec.iter()
                                       .map(|p| Pos::from(*p))
                                       .collect::<Vec<Pos>>();

    assert!(offsets.iter().all(|p| expected_pos.iter().any(|other| other == p)));
}

#[test]
pub fn test_reach_reachables() {
    let single = Reach::Single(1);
    let offsets = single.offsets();
    assert_eq!(8, offsets.len());

    let positions = single.reachables(Pos::new(5, 5));
    assert_eq!(8, positions.len());
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MoveResult {
    entity: Option<EntityId>,
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

    pub fn no_collision(&self) -> bool {
        return self.blocked.is_none() && self.entity.is_none();
    }
}

/// Moves the given object with a given offset, returning the square that it collides with, or None
/// indicating no collision.
/// NOTE if the movement carries multiple tiles, then the resulting MoveResult can report that
/// there is a blocking wall, and on a different location a blocking entity. These are checked
/// separately.
pub fn check_collision(pos: Pos,
                       dx: i32,
                       dy: i32,
                       level: &Level) -> MoveResult {
    let mut last_pos = pos;
    let mut result: MoveResult =
        MoveResult::with_pos(pos + Vector2D::new(dx, dy));

    // if no movement occurs, no need to check walls and entities.
    if !(dx == 0 && dy == 0) {
        if let Some(blocked) = level.map.path_blocked_move(pos, Pos::new(pos.x + dx, pos.y + dy)) {
            result.blocked = Some(blocked);
            result.move_pos = blocked.start_pos;
        } 

        // check for collision with an enitity
        let move_line = line_inclusive(pos, Pos::new(pos.x + dx, pos.y + dy));

        for line_tuple in move_line {
            let line_pos = Pos::from(line_tuple);

            if let Some(key) = level.has_blocking_entity(line_pos) {
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

pub fn entity_move_not_blocked(entity_id: EntityId, move_pos: Pos, delta_pos: Pos, level: &Level) -> Option<Movement> {
    let movement: Option<Movement>;

    let pos = level.entities.pos[&entity_id];

    let next_pos = next_pos(pos, delta_pos);
    if let Some(_other_id) = level.has_blocking_entity(next_pos) {
        movement = Some(Movement::move_to(move_pos, MoveType::Move));

        // NOTE removing dagger use by movement- only occurs in use mode
        //if can_stab(level, entity_id, other_id) {
        //   let attack = Attack::Stab(other_id, true);
        //   movement = Some(Movement::attack(move_pos, MoveType::Move, attack));
        //} else {
        //   movement = Some(Movement::move_to(move_pos, MoveType::Move));
        //}
    } else {
        movement = Some(Movement::move_to(move_pos, MoveType::Move));
    }

    return movement;
}

pub fn entity_move_blocked_by_wall(entity_id: EntityId, delta_pos: Pos, blocked: &Blocked, level: &Level) -> Option<Movement> {
    let movement: Option<Movement>;

    let pos = level.entities.pos[&entity_id];
    let mut jumped_wall = false;

    if level.entities.move_mode[&entity_id] == MoveMode::Run &&
       level.entities.stance[&entity_id] != Stance::Crouching {
        if !blocked.blocked_tile && blocked.wall_type == Wall::ShortWall {
            jumped_wall = true;
        } 
    }

    if jumped_wall {
        let mut new_pos = blocked.end_pos;
        if let Some(status) = level.entities.status.get(&entity_id) {
            if status.land_roll {
                new_pos = next_pos(pos, delta_pos);
            }
        }

        let mut jump_move = Movement::move_to(new_pos, MoveType::JumpWall);

        let next_pos = next_pos(pos, delta_pos);
        if let Some(other_id) = level.has_blocking_entity(next_pos) {
            if can_stab(level, entity_id, other_id) {
               let attack = Attack::Stab(other_id, true);
               jump_move = Movement::attack(new_pos, MoveType::JumpWall, attack);
           }
        }

        // Save the position at which the movement was blocked by a wall, which
        // is the start position of the Blocked structure.
        jump_move.wall = Some(blocked.start_pos);

        movement = Some(jump_move);
    } else {
        // else move up to the wall (start_pos is just before the colliding tile)
        movement = Some(Movement::move_to(blocked.start_pos, MoveType::Move));
    }

    return movement;
}

pub fn entity_move_blocked_by_entity(entity_id: EntityId,
                                     other_id: EntityId,
                                     move_pos: Pos,
                                     delta_pos: Pos,
                                     level: &Level) -> Option<Movement> {
    let movement: Option<Movement>;

    let pos = level.entities.pos[&entity_id];
    if can_stab(level, entity_id, other_id) {
        let attack = Attack::Stab(other_id, true);
        movement = Some(Movement::attack(move_pos, MoveType::Move, attack));
    } else if level.entities.blocks[&other_id] {
        let other_pos = level.entities.pos[&other_id];
        let next = next_pos(pos, delta_pos);
        if !level.map.is_within_bounds(next) {
            return None;
        }

        let next_tile_water = level.map[next].tile_type == TileType::Water;
        let push_is_blocked = level.map.path_blocked_move(other_pos, next).is_some();
        let is_column = level.entities.typ[&other_id] == EntityType::Column;

        if level.can_push(entity_id, other_id) && !next_tile_water && !(push_is_blocked && is_column) {
            let direction = Direction::from_dxy(delta_pos.x, delta_pos.y).unwrap();
            let push_amount = 1;
            // TODO issue 150 this is where pushing comes from. 
            let attack = Attack::Push(other_id, direction, push_amount);
            movement = Some(Movement::attack(add_pos(pos, delta_pos), MoveType::Move, attack));
        } else {
            movement = None;
        }
    } else {
        movement = Some(Movement::move_to(move_pos, MoveType::Move));
    }

    return movement;
}

pub fn entity_move_blocked_by_entity_and_wall(entity_id: EntityId, other_id: EntityId, blocked: &Blocked, delta_pos: Pos, level: &Level) -> Option<Movement> {
    let movement: Option<Movement>;

    let entity_pos = level.entities.pos[&other_id];
    let pos = level.entities.pos[&entity_id];

    let entity_dist = distance(pos, entity_pos);
    let wall_dist = distance(pos, blocked.end_pos);

    // We reach entity first, wall second
    if entity_dist < wall_dist {
        let dxy = sub_pos(entity_pos, pos);
        let attack: Option<Attack>;
        if can_stab(level, entity_id, other_id) {
            attack = Some(Attack::Stab(other_id, true));
        } else if level.map[next_pos(pos, dxy)].tile_type != TileType::Water {
            let direction = Direction::from_dxy(delta_pos.x, delta_pos.y).unwrap();
            let push_amount = 1;
            attack = Some(Attack::Push(other_id, direction, push_amount));
        } else {
            // water after push, so supress attack
            attack = None;
        }

        if let Some(attack) = attack {
            let move_pos = move_next_to(pos, entity_pos);
            movement = Some(Movement::attack(move_pos, MoveType::Move, attack));
        } else {
            movement = None;
        }
    } else if entity_dist > wall_dist {
        // we reach wall first, entity second
        let mut jumped_wall = false;
        if level.entities.move_mode[&entity_id] == MoveMode::Run {
            if !blocked.blocked_tile && blocked.wall_type == Wall::ShortWall {
                jumped_wall = true;
            } 
        }

        if jumped_wall {
            let attack =
                if can_stab(level, entity_id, other_id) {
                    Attack::Stab(other_id, true)
                } else {
                    let direction = Direction::from_dxy(delta_pos.x, delta_pos.y).unwrap();
                    let push_amount = 1;
                    Attack::Push(other_id, direction, push_amount)
                };
            let move_pos = move_next_to(pos, entity_pos);
            movement = Some(Movement::attack(move_pos, MoveType::Move, attack));
        } else {
            // can't jump the wall- just move up to it.
            movement = Some(Movement::move_to(blocked.start_pos, MoveType::Move));
        }
    } else {
        // entity and wall are together- between-tile wall in front of entity
        // move up to the wall- we can't jump it or attack through it
        movement = Some(Movement::move_to(blocked.start_pos, MoveType::Move));

        // TODO issue #248 when can attack/push/move?
        //let move_pos = move_next_to(pos, entity_pos);
        //let direction = Direction::from_dxy(delta_pos.x, delta_pos.y).unwrap();
        //let push_amount = 1;
        //let attack = Attack::Push(other_id, direction, push_amount);
        //movement = Some(Movement::attack(move_pos, MoveType::Move, attack));
    }

    return movement;
}

pub fn calculate_move(dir: Direction,
                      reach: Reach,
                      entity_id: EntityId,
                      level: &Level) -> Option<Movement> {
    let mut movement: Option<Movement>;

    let pos = level.entities.pos[&entity_id];

    // get the location we would move to given the input direction
    if let Some(delta_pos) = reach.move_with_reach(&dir) {
        let (dx, dy) = delta_pos.to_tuple();

        // check if movement collides with a blocked location or an entity
        let move_result = check_collision(pos, dx, dy, level);

        match (move_result.blocked, move_result.entity) {
            // both blocked by wall and by entity
            (Some(blocked), Some(other_id)) => {
                movement = entity_move_blocked_by_entity_and_wall(entity_id, other_id, &blocked, delta_pos, level);
            }

            // blocked by entity only
            (None, Some(other_id)) => {
                movement = entity_move_blocked_by_entity(entity_id, other_id, move_result.move_pos, delta_pos, level);
            }

            // blocked by wall only
            (Some(blocked), None) => {
                movement = entity_move_blocked_by_wall(entity_id, delta_pos, &blocked, level);
            }

            // not blocked at all
            (None, None) => {
                movement = entity_move_not_blocked(entity_id, move_result.move_pos, delta_pos, level);
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

