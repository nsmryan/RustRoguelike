use std::iter;

use smallvec::SmallVec;

use itertools::Itertools;

use pathfinding::directed::astar::astar;

use serde_derive::*;

use roguelike_utils::line::*;
use roguelike_utils::math::Pos;

use crate::utils::*;
use crate::map::*;
use crate::types::*;


// multiplier used to scale costs up in astar, allowing small
// adjustments of costs even though they are integers.
pub const ASTAR_COST_MULTIPLIER: i32 = 100;


/// This structure describes a movement between two
/// tiles that was blocked due to a wall or blocked tile.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Blocked {
    pub start_pos: Pos,
    pub end_pos: Pos,
    pub direction: Direction,
    pub blocked_tile: bool,
    pub wall_type: Wall,
}

impl Blocked {
    pub fn new(start_pos: Pos,
               end_pos: Pos,
               direction: Direction,
               blocked_tile: bool,
               wall_type: Wall) -> Blocked {
        return Blocked { start_pos,
                         end_pos,
                         direction,
                         blocked_tile,
                         wall_type,
        };
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum BlockedType {
    Fov,
    FovLow,
    Move,
}

impl BlockedType {
    pub fn blocking(&self, wall: Wall, material: Surface) -> bool {
        let walk_into = *self == BlockedType::Move && (wall == Wall::Empty || material == Surface::Grass);
        let see_over = (*self == BlockedType::Fov && wall != Wall::TallWall) || (*self == BlockedType::FovLow && wall == Wall::Empty);
        return !walk_into && !see_over;
    }
}

#[test]
fn test_blocked_type() {
    assert_eq!(false, BlockedType::Fov.blocking(Wall::ShortWall, Surface::Floor));
    assert_eq!(false, BlockedType::Fov.blocking(Wall::ShortWall, Surface::Grass));
    assert_eq!(true, BlockedType::Fov.blocking(Wall::TallWall, Surface::Floor));
    assert_eq!(false, BlockedType::Fov.blocking(Wall::Empty, Surface::Floor));
    assert_eq!(false, BlockedType::Fov.blocking(Wall::Empty, Surface::Grass));

    assert_eq!(true, BlockedType::FovLow.blocking(Wall::ShortWall, Surface::Floor));
    assert_eq!(true, BlockedType::FovLow.blocking(Wall::TallWall, Surface::Floor));
    assert_eq!(true, BlockedType::FovLow.blocking(Wall::ShortWall, Surface::Grass));
    assert_eq!(true, BlockedType::FovLow.blocking(Wall::TallWall, Surface::Grass));
    assert_eq!(false, BlockedType::FovLow.blocking(Wall::Empty, Surface::Floor));
    assert_eq!(false, BlockedType::FovLow.blocking(Wall::Empty, Surface::Grass));

    assert_eq!(false, BlockedType::Move.blocking(Wall::Empty, Surface::Floor));
    assert_eq!(true, BlockedType::Move.blocking(Wall::ShortWall, Surface::Floor));
    assert_eq!(false, BlockedType::Move.blocking(Wall::TallWall, Surface::Grass));
    assert_eq!(false, BlockedType::Move.blocking(Wall::ShortWall, Surface::Grass));
    assert_eq!(true, BlockedType::Move.blocking(Wall::TallWall, Surface::Floor));
}

impl Map {
    pub fn blocked_left(&self, pos: Pos, blocked_type: BlockedType) -> bool {
        let offset = Pos::new(pos.x - 1, pos.y);
        if !self.is_within_bounds(pos) || !self.is_within_bounds(offset) {
            return true;
        }

        let blocking_wall = blocked_type.blocking(self[pos].left_wall, self[pos].left_material);
        let blocking_tile = self[offset].does_tile_block(blocked_type);
        return blocking_wall || blocking_tile;
    }

    pub fn blocked_right(&self, pos: Pos, blocked_type: BlockedType) -> bool {
        let offset = Pos::new(pos.x + 1, pos.y);
        if !self.is_within_bounds(pos) || !self.is_within_bounds(offset) { 
            return true;
        }

        let blocking_wall = blocked_type.blocking(self[offset].left_wall, self[offset].left_material);
        let blocking_tile = self[offset].does_tile_block(blocked_type);
        return blocking_wall || blocking_tile;
    }

    pub fn blocked_down(&self, pos: Pos, blocked_type: BlockedType) -> bool {
        let offset = Pos::new(pos.x, pos.y + 1);
        if !self.is_within_bounds(pos) || !self.is_within_bounds(offset) {
            return true;
        }

        let blocking_wall = blocked_type.blocking(self[pos].bottom_wall, self[pos].bottom_material);
        let blocking_tile = self[offset].does_tile_block(blocked_type);
        return blocking_wall || blocking_tile;
    }

    pub fn blocked_up(&self, pos: Pos, blocked_type: BlockedType) -> bool {
        let offset = Pos::new(pos.x, pos.y - 1);
        if !self.is_within_bounds(pos) || !self.is_within_bounds(offset) {
            return true;
        }

        let blocking_wall = blocked_type.blocking(self[offset].bottom_wall, self[offset].bottom_material);
        let blocking_tile = self[offset].does_tile_block(blocked_type);
        return blocking_wall || blocking_tile;
    }

    pub fn path_blocked_fov(&self, start_pos: Pos, end_pos: Pos) -> Option<Blocked> {
        return self.path_blocked(start_pos, end_pos, BlockedType::Fov);
    }

    pub fn path_blocked_fov_low(&self, start_pos: Pos, end_pos: Pos) -> Option<Blocked> {
        return self.path_blocked(start_pos, end_pos, BlockedType::FovLow);
    }

    pub fn path_blocked_move(&self, start_pos: Pos, end_pos: Pos) -> Option<Blocked> {
        return self.path_blocked(start_pos, end_pos, BlockedType::Move);
    }

    pub fn path_blocked_all(&self, start_pos: Pos, end_pos: Pos, blocked_type: BlockedType) -> Vec<Blocked> {
        let mut blocked_vec = Vec::new();
        let mut cur_pos = start_pos;
        while let Some(blocked) = self.path_blocked(cur_pos, end_pos, blocked_type) {
            blocked_vec.push(blocked);
            cur_pos = blocked.end_pos;
        }
        return blocked_vec;
    }

    pub fn move_blocked(&self, start_pos: Pos, end_pos: Pos, blocked_type: BlockedType) -> Option<Blocked> {
        let dxy = sub_pos(end_pos, start_pos);
        if dxy.x == 0 && dxy.y == 0 {
            return None;
        }

        let dir = Direction::from_dxy(dxy.x, dxy.y)
                            .expect(&format!("Check for blocking wall with no movement {:?}?", dxy));


        let mut blocked = Blocked::new(start_pos, end_pos, dir, false, Wall::Empty);

        // if the target position is out of bounds, we are blocked
        if !self.is_within_bounds(end_pos) {
            blocked.blocked_tile = true;

            // continuing to check after finding an out-of-bounds
            // position results in a panic, so stop now.
            return Some(blocked);
        }

        let mut found_blocker = false;

        // if moving into a blocked tile, we are blocked
        if self[end_pos].does_tile_block(blocked_type) {
            blocked.blocked_tile = true;
            found_blocker = true;
        }

        let (x, y) = (start_pos.x, start_pos.y);
        let move_dir = sub_pos(end_pos, Pos::new(x, y));

        // used for diagonal movement checks
        let x_moved = Pos::new(end_pos.x, y);
        let y_moved = Pos::new(x, end_pos.y);
        
        let direction = Direction::from_dxy(move_dir.x, move_dir.y).unwrap();
        match direction {
            Direction::Right | Direction::Left => {
                let mut left_wall_pos = start_pos;
                if move_dir.x >= 1 {
                    left_wall_pos = Pos::new(x + move_dir.x, y);
                }

                if self.is_within_bounds(left_wall_pos) &&
                   blocked_type.blocking(self[left_wall_pos].left_wall, self[left_wall_pos].left_material) {
                        blocked.wall_type = self[left_wall_pos].left_wall;
                        found_blocker = true;
                }
            }

            Direction::Up | Direction::Down => {
                let mut bottom_wall_pos = Pos::new(x, y + move_dir.y);
                if move_dir.y >= 1 {
                    bottom_wall_pos = start_pos;
                }

                if self.is_within_bounds(bottom_wall_pos) &&
                   blocked_type.blocking(self[bottom_wall_pos].bottom_wall, self[bottom_wall_pos].bottom_material) {
                        blocked.wall_type = self[bottom_wall_pos].bottom_wall;
                        found_blocker = true;
                }
            }

            Direction::DownRight => {
                if self.blocked_right(start_pos, blocked_type) && self.blocked_down(start_pos, blocked_type) {
                    blocked.wall_type = self[start_pos].bottom_wall;
                    found_blocker = true;
                }

                if self.blocked_right(move_y(start_pos, 1), blocked_type) &&
                   self.blocked_down(move_x(start_pos, 1), blocked_type) {
                    let blocked_pos = add_pos(start_pos, Pos::new(1, 0));
                    if self.is_within_bounds(blocked_pos) {
                        blocked.wall_type = self[blocked_pos].bottom_wall;
                    }
                    found_blocker = true;
                }

                if self.blocked_right(start_pos, blocked_type) &&
                   self.blocked_right(y_moved, blocked_type) {
                    let blocked_pos = move_x(start_pos, 1);
                    if self.is_within_bounds(blocked_pos) {
                        blocked.wall_type = self[blocked_pos].left_wall;
                    }
                    found_blocker = true;
                }

                if self.blocked_down(start_pos, blocked_type) &&
                   self.blocked_down(x_moved, blocked_type) {
                    blocked.wall_type = self[start_pos].bottom_wall;
                    found_blocker = true;
                }
            }

            Direction::UpRight => {
                if self.blocked_up(start_pos, blocked_type) && self.blocked_right(start_pos, blocked_type) {
                    let blocked_pos = move_y(start_pos, -1);
                    if self.is_within_bounds(blocked_pos) {
                        blocked.wall_type = self[blocked_pos].bottom_wall;
                    }
                    found_blocker = true;
                }

                if self.blocked_up(move_x(start_pos, 1), blocked_type) &&
                   self.blocked_right(move_y(start_pos, -1), blocked_type) {
                    let blocked_pos = add_pos(start_pos, Pos::new(1, -1));
                    if self.is_within_bounds(blocked_pos) {
                        blocked.wall_type = self[blocked_pos].bottom_wall;
                    }
                    found_blocker = true;
                }

                if self.blocked_right(start_pos, blocked_type) && self.blocked_right(y_moved, blocked_type) {
                    let blocked_pos = move_x(start_pos, 1);
                    if self.is_within_bounds(blocked_pos) {
                        blocked.wall_type = self[blocked_pos].left_wall;
                    }
                    found_blocker = true;
                }

                if self.blocked_up(start_pos, blocked_type) && self.blocked_up(x_moved, blocked_type) {
                    let blocked_pos = move_y(start_pos, -1);
                    if self.is_within_bounds(blocked_pos) {
                        blocked.wall_type = self[blocked_pos].bottom_wall;
                    }
                    found_blocker = true;
                }
            }

            Direction::DownLeft => {
                if self.blocked_left(start_pos, blocked_type) && self.blocked_down(start_pos, blocked_type) {
                    blocked.wall_type = self[start_pos].left_wall;
                    found_blocker = true;
                }

                if self.blocked_left(move_y(start_pos, 1), blocked_type) &&
                   self.blocked_down(move_x(start_pos, -1), blocked_type) {
                    let blocked_pos = add_pos(start_pos, Pos::new(-1, 1));
                    if self.is_within_bounds(blocked_pos) {
                        blocked.wall_type = self[blocked_pos].left_wall;
                    }
                    found_blocker = true;
                }

                if self.blocked_left(start_pos, blocked_type) && self.blocked_left(y_moved, blocked_type) {
                    blocked.wall_type = self[start_pos].left_wall;
                    found_blocker = true;
                }

                if self.blocked_down(start_pos, blocked_type) && self.blocked_down(x_moved, blocked_type) {
                    blocked.wall_type = self[start_pos].bottom_wall;
                    found_blocker = true;
                }
            }

            Direction::UpLeft => {
                if self.blocked_left(move_y(start_pos, -1), blocked_type) &&
                   self.blocked_up(move_x(start_pos, -1), blocked_type) {
                    let blocked_pos = add_pos(start_pos, Pos::new(-1, -1));
                    if self.is_within_bounds(blocked_pos) {
                        blocked.wall_type = self[blocked_pos].left_wall;
                    }
                    found_blocker = true;
                }

                if self.blocked_left(start_pos, blocked_type) && self.blocked_up(start_pos, blocked_type) {
                    blocked.wall_type = self[start_pos].left_wall;
                    found_blocker = true;
                }

                if self.blocked_left(start_pos, blocked_type) && self.blocked_left(y_moved, blocked_type) {
                    blocked.wall_type = self[start_pos].left_wall;
                    found_blocker = true;
                }

                if self.blocked_up(start_pos, blocked_type) && self.blocked_up(x_moved, blocked_type) {
                    let blocked_pos = move_y(start_pos, -1);
                    if self.is_within_bounds(blocked_pos) {
                        blocked.wall_type = self[blocked_pos].bottom_wall;
                    }
                    found_blocker = true;
                }
            }
        }

        if found_blocker {
            return Some(blocked);
        } else {
            return None;
        }
    }

    pub fn path_blocked(&self, start_pos: Pos, end_pos: Pos, blocked_type: BlockedType) -> Option<Blocked> {
        let line = line(start_pos, end_pos);
        let positions = iter::once(start_pos).chain(line.into_iter());
        for (pos, target_pos) in positions.tuple_windows() {
            let blocked = self.move_blocked(pos, target_pos, blocked_type);
            if blocked.is_some() {
                return blocked;
            }
        }

        return None;
    }
}

pub fn path_find_distance(start: Pos, next_pos: Pos, end: Pos) -> i32 {
    let mut dist = distance(next_pos, end) * ASTAR_COST_MULTIPLIER;
    let diff = sub_pos(next_pos, start);

    // penalize diagonal movement just a little bit to avoid zigzagging.
    if diff.x != 0 && diff.y != 0 {
        dist += 1;
    }

    return dist;
}

pub fn astar_path(map: &Map,
                  start: Pos,
                  end: Pos,
                  max_dist: Option<i32>,
                  cost_fn: Option<fn(Pos, Pos, &Map) -> i32>) -> Vec<Pos> {
    let result;

    let maybe_results = 
        astar(&start,
              |&pos| astar_neighbors(map, start, pos, max_dist),
              |&pos| {
                  if let Some(fun) = &cost_fn { 
                      fun(start, pos, map) * ASTAR_COST_MULTIPLIER
                  } else {
                      path_find_distance(start, pos, end) as i32
                  }
              },
              |&pos| pos == end);

    if let Some((results, _cost)) = maybe_results {
        result = results.iter().map(|p| *p).collect::<Vec<Pos>>();
    } else {
        result = Vec::new();
    }

    return result;
}

pub fn astar_next_pos(map: &Map,
                      start: Pos,
                      end: Pos,
                      max_dist: Option<i32>,
                      cost_fn: Option<fn(Pos, Pos, &Map) -> i32>) -> Option<Pos> {
    let next_positions = astar_path(map, start, end, max_dist, cost_fn);

    if let Some(next_pos) = next_positions.get(1) {
        return Some(*next_pos);
    } else {
        return None;
    }
}

pub fn astar_neighbors(map: &Map, start: Pos, pos: Pos, max_dist: Option<i32>) -> SmallVec<[(Pos, i32); 8]> {
      if let Some(max_dist) = max_dist {
          if distance(start, pos) > max_dist {
              return SmallVec::new();
          }
      }

      let next_pos = map.reachable_neighbors(pos)
                        .iter()
                        .map(|pos| (*pos, 1))
                        .collect::<SmallVec<[(Pos, i32); 8]>>();

    return next_pos;
}

#[test]
fn test_blocked_by_wall_right() {
    let mut map = Map::from_dims(10, 10);

    let pos = Pos::new(5, 5);
    map[pos].left_wall = Wall::ShortWall;
  
    let left_of_wall = Pos::new(4, 5);
    let blocked = map.path_blocked_move(left_of_wall, add_pos(left_of_wall, Pos::new(1, 0)));
    assert_eq!(blocked.map(|b| b.wall_type == Wall::ShortWall), Some(true));

    assert!(map.path_blocked_move(pos, add_pos(pos, Pos::new(1, 0))).is_none());

    let two_left_of_wall = Pos::new(3, 5);
    assert_eq!(map.path_blocked_move(two_left_of_wall, add_pos(two_left_of_wall, Pos::new(1, 0))), None);
}

#[test]
fn test_blocked_by_wall_up() {
    let mut map = Map::from_dims(10, 10);

    let pos = Pos::new(5, 5);
    map[pos].bottom_wall = Wall::ShortWall;
  
    let blocked = map.path_blocked_move(Pos::new(5, 6), Pos::new(5, 5));
    assert_eq!(blocked.map(|b| b.wall_type), Some(Wall::ShortWall));
    assert!(map.path_blocked_move(Pos::new(5, 5), Pos::new(5, 4)).is_none());
    assert!(map.path_blocked_move(Pos::new(5, 4), Pos::new(5, 3)).is_none());
}

#[test]
fn test_blocked_by_wall_down() {
    let mut map = Map::from_dims(10, 10);

    let pos = Pos::new(5, 5);
    map[pos].bottom_wall = Wall::ShortWall;
  
    let blocked = map.path_blocked_move(Pos::new(5, 5), Pos::new(5, 6));
    assert_eq!(blocked.map(|b| b.wall_type), Some(Wall::ShortWall));
    assert!(map.path_blocked_move(Pos::new(5, 6), Pos::new(5, 7)).is_none());
    assert!(map.path_blocked_move(Pos::new(5, 7), Pos::new(5, 8)).is_none());
}

#[test]
fn test_blocked_by_wall_left() {
    let mut map = Map::from_dims(10, 10);

    let pos = Pos::new(5, 5);
    map[pos].left_wall = Wall::ShortWall;
  
    let blocked = map.path_blocked_move(Pos::new(5, 5), Pos::new(4, 5));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));
    assert!(map.path_blocked_move(Pos::new(4, 5), Pos::new(3, 5)).is_none());
    assert!(map.path_blocked_move(Pos::new(6, 5), Pos::new(5, 5)).is_none());
}

#[test]
fn test_blocked_in_corners() {
    let mut map = Map::from_dims(10, 10);

    // .....
    // ._|_. middle | is (5, 5)
    // ..|..
    // .....
    map[(5, 5)].left_wall = Wall::ShortWall;
    map[(5, 5)].bottom_wall = Wall::ShortWall;
    map[(4, 5)].bottom_wall = Wall::ShortWall;
    map[(5, 6)].left_wall = Wall::ShortWall;
  
    // down left
    let blocked = map.path_blocked_move(Pos::new(5, 5), Pos::new(4, 6));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));

    // down right
    let blocked = map.path_blocked_move(Pos::new(4, 5), Pos::new(5, 6));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));

    // up right
    let blocked = map.path_blocked_move(Pos::new(4, 6), Pos::new(5, 5));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));

    // up left
    let blocked = map.path_blocked_move(Pos::new(6, 6), Pos::new(4, 5));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));
}

#[test]
fn test_blocked_out_corners() {
    let mut map = Map::from_dims(10, 10);

    // ._...
    // |_|.. middle _ is (5, 5)
    // .....
    map[(5, 5)].left_wall = Wall::ShortWall;
    map[(5, 5)].bottom_wall = Wall::ShortWall;
    map[(6, 5)].left_wall = Wall::ShortWall;
    map[(5, 4)].bottom_wall = Wall::ShortWall;
  
    // down left
    let blocked = map.path_blocked_move(Pos::new(6, 4), Pos::new(5, 5));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));

    // down right
    let blocked = map.path_blocked_move(Pos::new(4, 5), Pos::new(5, 5));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));

    // up right
    let blocked = map.path_blocked_move(Pos::new(4, 6), Pos::new(5, 5));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));

    // up left
    let blocked = map.path_blocked_move(Pos::new(6, 6), Pos::new(5, 5));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));
}

#[test]
fn test_blocked_horiz_line() {
    let mut map = Map::from_dims(10, 10);

    // .....
    // ...__ middle . is (5, 5)
    // .....
    map[(6, 5)].bottom_wall = Wall::ShortWall;
    map[(7, 5)].bottom_wall = Wall::ShortWall;
  
    // down left
    let blocked = map.path_blocked_move(Pos::new(7, 5), Pos::new(6, 6));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));

    // down right
    let blocked = map.path_blocked_move(Pos::new(6, 5), Pos::new(7, 6));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));

    // up right
    let blocked = map.path_blocked_move(Pos::new(6, 6), Pos::new(7, 5));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));

    // up left
    let blocked = map.path_blocked_move(Pos::new(7, 6), Pos::new(6, 5));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));
}

#[test]
fn test_blocked_vert_line() {
    let mut map = Map::from_dims(10, 10);

    // .....
    // ...__ middle . is (5, 5)
    // .....
    map[(6, 5)].bottom_wall = Wall::ShortWall;
    map[(7, 5)].bottom_wall = Wall::ShortWall;
  
    // down left
    let blocked = map.path_blocked_move(Pos::new(7, 5), Pos::new(6, 6));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));

    // down right
    let blocked = map.path_blocked_move(Pos::new(6, 5), Pos::new(7, 6));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));

    // up right
    let blocked = map.path_blocked_move(Pos::new(6, 6), Pos::new(7, 5));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));

    // up left
    let blocked = map.path_blocked_move(Pos::new(7, 6), Pos::new(6, 5));
    assert_eq!(blocked.map(|blocked| blocked.wall_type), Some(Wall::ShortWall));
}

#[test]
fn test_fov_blocked_by_wall_right() {
    let radius = 10;
    let mut map = Map::from_dims(10, 10);

    for wall_y_pos in 2..8 {
        let pos: Pos = Pos::new(5, wall_y_pos);
        map[pos] = Tile::empty();
        map[pos].block_sight = true;
    }
  
    assert_eq!(false, map.is_in_fov(Pos::new(4, 5), Pos::new(9, 5), radius, false));
}

#[test]
fn test_fov_blocked_by_wall_left() {
    let radius = 10;
    let mut map = Map::from_dims(10, 10);

    for wall_y_pos in 2..8 {
        let pos: Pos = Pos::new(6, wall_y_pos);
        map[pos] = Tile::empty();
        map[pos].block_sight = true;
        map[pos].block_move = true;
    }
  
    assert_eq!(false, map.is_in_fov(Pos::new(9, 5), Pos::new(4, 5), radius, false));
}

#[test]
fn test_fov_blocked_by_wall_up() {
    let radius = 10;
    let mut map = Map::from_dims(10, 10);

    for wall_x_pos in 2..8 {
        let pos: (i32, i32) = (wall_x_pos, 6);
        map[pos] = Tile::empty();
        map[pos].block_sight = true;
        map[pos].block_move = true;
    }
  
    assert_eq!(false, map.is_in_fov(Pos::new(5, 9), Pos::new(5, 5), radius, false));
}

#[test]
fn test_fov_blocked_by_wall_down() {
    let radius = 10;
    let mut map = Map::from_dims(10, 10);

    for wall_x_pos in 2..8 {
        let pos: (i32, i32) = (wall_x_pos, 4);
        map[pos] = Tile::empty();
        map[pos].block_sight = true;
        map[pos].block_move = true;
    }
  
    assert_eq!(false, map.is_in_fov(Pos::new(5, 1), Pos::new(5, 6), radius, false));
}

#[test]
fn test_fov_blocked_by_tall_grass() {
    let mut map = Map::from_dims(10, 10);

    let pos = Pos::new(5, 5);
    map[pos] = Tile::tall_grass();
    assert!(map.path_blocked_fov(Pos::new(4, 5), Pos::new(5, 5)).is_some());

    // TODO its not clear whether tall grass should block LoS this way.
    //let radius = 10;
    //assert_eq!(false, map.is_in_fov(Pos::new(4, 5), Pos::new(5, 5), radius, false));
}

#[test]
fn test_blocked_by_wall() {
    let mut map = Map::from_dims(10, 10);

    map[(5, 5)] = Tile::water();
  
    assert!(map.path_blocked_move(Pos::new(4, 5), Pos::new(5, 5)).is_some());
    assert!(map.path_blocked_move(Pos::new(4, 5), Pos::new(7, 5)).is_some());
    assert!(map.path_blocked_move(Pos::new(3, 5), Pos::new(6, 5)).is_some());

    assert!(map.path_blocked_move(Pos::new(6, 5), Pos::new(5, 5)).is_some());

    assert!(map.path_blocked_move(Pos::new(5, 6), Pos::new(5, 5)).is_some());
    assert!(map.path_blocked_move(Pos::new(5, 4), Pos::new(5, 5)).is_some());
}

#[test]
fn test_path_blocked_all() {
    let mut map = Map::from_dims(10, 10);
    map[(3, 5)].left_wall = Wall::ShortWall;
    map[(4, 5)].block_move = true;
    map[(5, 5)].block_move = true;
    map[(6, 5)].left_wall = Wall::TallWall;

    let start_pos = Pos::new(0, 5);
    let end_pos = Pos::new(9, 5);

    let blocked_positions = map.path_blocked_all(start_pos, end_pos, BlockedType::Move);

    assert_eq!(4, blocked_positions.len());

    assert_eq!(false, blocked_positions[0].blocked_tile);
    assert_eq!(Wall::ShortWall, blocked_positions[0].wall_type);

    assert_eq!(true, blocked_positions[1].blocked_tile);
    assert_eq!(Wall::Empty, blocked_positions[1].wall_type);

    assert_eq!(true, blocked_positions[2].blocked_tile);
    assert_eq!(Wall::Empty, blocked_positions[2].wall_type);

    assert_eq!(false, blocked_positions[3].blocked_tile);
    assert_eq!(Wall::TallWall, blocked_positions[3].wall_type);
}
