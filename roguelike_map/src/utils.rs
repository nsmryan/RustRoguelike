use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

use euclid::*;

use roguelike_utils::line::line;
use roguelike_utils::rng::*;

use crate::map::*;
use crate::types::*;


pub fn distance(pos1: Pos, pos2: Pos) -> i32 {
    let line = line(pos1, pos2);
    return line.iter().count() as i32;
}

pub fn move_by(start: Pos, diff: Pos) -> Pos {
    return Pos::new(start.x + diff.x, start.y + diff.y);
}

pub fn move_y(pos: Pos, offset_y: i32) -> Pos {
    return Pos::new(pos.x, pos.y + offset_y);
}

pub fn move_x(pos: Pos, offset_x: i32) -> Pos {
    return Pos::new(pos.x + offset_x, pos.y);
}

pub fn sub_pos(pos1: Pos, pos2: Pos) -> Pos {
    return Pos::new(pos1.x - pos2.x, pos1.y - pos2.y);
}

pub fn add_pos(pos1: Pos, pos2: Pos) -> Pos {
    return Pos::new(pos1.x + pos2.x, pos1.y + pos2.y);
}

pub fn scale_pos(pos: Pos, scale: i32) -> Pos {
    return Pos::new(pos.x * scale, pos.y * scale);
}

pub fn move_towards(start: Pos, end: Pos, num_blocks: usize) -> Pos {
    let line = line(start, end);

    return Pos::from(line.iter().map(|p| *p).skip(num_blocks).next().unwrap_or(end));
}

pub fn pos_on_map(pos: Pos) -> bool {
    return pos.x != -1 || pos.y != -1;
}

pub fn rand_from_pos(pos: Pos) -> f32 {
    return rand_from_x_y(pos.x, pos.y);
}

pub fn rand_from_x_y(x: i32, y: i32) -> f32 {
    let mut hasher = DefaultHasher::new();

    (x as u32).hash(&mut hasher);
    (y as u32).hash(&mut hasher);
 
    let result: u64 = hasher.finish();

    return ((result & 0xFFFFFFFF) as f32) / 4294967295.0;
}

pub fn distance_tiles(pos1: Pos, pos2: Pos) -> i32 {
    return (pos1.x - pos2.x).abs() + (pos1.y - pos2.y).abs();
}

pub fn distance_maximum(pos1: Pos, pos2: Pos) -> i32 {
    return std::cmp::max((pos1.x - pos2.x).abs(), (pos1.y - pos2.y).abs());
}

pub fn pos_mag(pos: Pos) -> i32 {
    return distance(Pos::new(0, 0), pos);
}

pub fn signedness(value: i32) -> i32 {
    if value == 0 {
        return 0;
    } else {
        return value.signum();
    }
}

pub fn mirror_in_x(pos: Pos, width: i32) -> Pos {
    return Pos::new(width - pos.x - 1, pos.y);
}

pub fn mirror_in_y(pos: Pos, height: i32) -> Pos {
    return Pos::new(pos.x, height - pos.y - 1);
}

pub fn in_direction_of(start: Pos, end: Pos) -> Pos {
    let dpos = sub_pos(end, start);
    let dx = signedness(dpos.x);
    let dy = signedness(dpos.y);

    return add_pos(start, Pos::new(dx, dy));
}

#[test]
pub fn test_in_direction_of() {
    let start = Pos::new(1, 1);

    assert_eq!(in_direction_of(start, Pos::new(0, 0)), Pos::new(0, 0));
    assert_eq!(in_direction_of(start, Pos::new(10, 10)), Pos::new(2, 2));
    assert_eq!(in_direction_of(start, Pos::new(10, 1)), Pos::new(2, 1));
    assert_eq!(in_direction_of(start, Pos::new(1, 10)), Pos::new(1, 2));
    assert_eq!(in_direction_of(start, Pos::new(1, -10)), Pos::new(1, 0));
    assert_eq!(in_direction_of(start, Pos::new(-10, 1)), Pos::new(0, 1));
    assert_eq!(in_direction_of(start, Pos::new(-10, -10)), Pos::new(0, 0));
}

pub fn is_ordinal(delta: Pos) -> bool {
    return (delta.x == 0 && delta.y != 0) || 
           (delta.y == 0 && delta.x != 0);
}

pub fn sort_by_distance_to(pos: Pos, positions: &mut Vec<Pos>) {
    positions.sort_by(|a, b| distance(pos, *a)
             .partial_cmp(&distance(pos, *b)).unwrap());
}


pub fn step_towards(start_pos: Pos, target_pos: Pos) -> Pos {
    let dx = target_pos.x - start_pos.x;
    let dy = target_pos.y - start_pos.y;
    let delta_pos = Pos::new(signedness(dx), signedness(dy));
    return delta_pos;
}

pub fn next_from_to(start: Pos, end: Pos) -> Pos {
    let diff = sub_pos(end, start);
    return next_pos(start, diff);
}

pub fn next_pos(pos: Pos, delta_pos: Pos) -> Pos {
    let mut next_pos = add_pos(pos, delta_pos);

    if delta_pos.x != 0 {
        next_pos.x += delta_pos.x.signum();
    }

    if delta_pos.y != 0 {
        next_pos.y += delta_pos.y.signum();
    }

    return next_pos;
}

pub fn dxy(start_pos: Pos, end_pos: Pos) -> (i32, i32) {
    return (end_pos.x - start_pos.x, end_pos.y - start_pos.y);
}

pub fn move_next_to(start_pos: Pos, end_pos: Pos) -> Pos {
    if distance(start_pos, end_pos) <= 1 {
        return start_pos;
    }

    let line = line(start_pos, end_pos);

    let mut second_to_last = *line.iter().next().unwrap();

    for pos in line {
        if pos != end_pos {
            second_to_last = pos;
        }
    }

    return second_to_last;
}

#[test]
pub fn test_move_next_to() {
    assert_eq!(move_next_to(Pos::new(0, 0), Pos::new(5, 5)), Pos::new(4, 4));
    assert_eq!(move_next_to(Pos::new(0, 0), Pos::new(1, 1)), Pos::new(0, 0));
    assert_eq!(move_next_to(Pos::new(0, 0), Pos::new(-5, -5)), Pos::new(-4, -4));
    assert_eq!(move_next_to(Pos::new(0, 0), Pos::new(0, 5)), Pos::new(0, 4));
    assert_eq!(move_next_to(Pos::new(0, 0), Pos::new(5, 0)), Pos::new(4, 0));
}

pub fn visible_in_direction(start_pos: Pos, end_pos: Pos, dir: Direction) -> bool {
    let pos_diff = sub_pos(end_pos, start_pos);
    let x_sig = pos_diff.x.signum();
    let y_sig = pos_diff.y.signum();

    match dir {
        Direction::Up => {
            if y_sig < 1 {
                return true;
            }
        }

        Direction::Down => {
            if y_sig > -1 {
                return true;
            }
        }

        Direction::Left => {
            if x_sig < 1 {
                return true;
            }
        }

        Direction::Right => {
            if x_sig > -1 {
                return true;
            }
        }
        Direction::DownLeft => {
            if pos_diff.x - pos_diff.y < 0 {
                return true;
            }
        }

        Direction::DownRight => {
            if pos_diff.x + pos_diff.y >= 0 {
                return true;
            }
        }

        Direction::UpLeft => {
            if pos_diff.x + pos_diff.y <= 0 {
                return true;
            }
        }

        Direction::UpRight => {
            if pos_diff.x - pos_diff.y > 0 {
                return true;
            }
        }
    }

    return false;
}

#[test]
pub fn test_visible_in_direction() {
    let dir = Direction::Up;

    let start_pos = Pos::new(0, 0);
    let end_pos = Pos::new(1, 0);
    assert!(visible_in_direction(start_pos, end_pos, dir));

    let end_pos = Pos::new(-1, 0);
    assert!(visible_in_direction(start_pos, end_pos, dir));

    let end_pos = Pos::new(-1, 0);
    assert!(visible_in_direction(start_pos, end_pos, dir));

    let end_pos = Pos::new(0, -1);
    assert!(visible_in_direction(start_pos, end_pos, dir));

    let end_pos = Pos::new(1, -1);
    assert!(visible_in_direction(start_pos, end_pos, dir));

    let end_pos = Pos::new(-1, -1);
    assert!(visible_in_direction(start_pos, end_pos, dir));

    let end_pos = Pos::new(2, -2);
    assert!(visible_in_direction(start_pos, end_pos, dir));

    let end_pos = Pos::new(-2, -2);
    assert!(visible_in_direction(start_pos, end_pos, dir));
}

pub fn near_tile_type(map: &Map, position: Pos, tile_type: TileType) -> bool {
    let neighbor_offsets: Vec<(i32, i32)>
        = vec!((1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1));

    let mut near_given_tile = false;

    for offset in neighbor_offsets {
        let offset = Pos::from(offset);
        let neighbor_position = move_by(position, offset);

        if map.is_within_bounds(neighbor_position) &&
           map[neighbor_position].tile_type == tile_type {
            near_given_tile = true;
            break;
        }
    }

    return near_given_tile;
}

pub fn random_offset(rng: &mut Rand32, radius: i32) -> Pos {
    return Pos::new(rng_range_i32(rng, -radius, radius),
                    rng_range_i32(rng, -radius, radius));
}

pub fn pos_in_radius(pos: Pos, radius: i32, rng: &mut Rand32) -> Pos {
    let offset = Vector2D::new(rng_range_i32(rng, -radius, radius),
                               rng_range_i32(rng, -radius, radius));
    return pos + offset;
}

