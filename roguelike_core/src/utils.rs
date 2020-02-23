use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use tcod::line::*;

use crate::types::*;
use crate::movement::{Reach, MoveMode};
use crate::messaging::*;


pub fn distance(pos1: Pos, pos2: Pos) -> i32 {
    return (((pos1.x - pos2.x).pow(2) + (pos1.y - pos2.y).pow(2)) as f32).sqrt() as i32;
}

pub fn push_attack(handle: ObjectId, other_handle: ObjectId, data: &mut GameData, msg_log: &mut MsgLog) {
    let pos = data.objects[handle].pos();
    let other_pos = data.objects[other_handle].pos();
    let diff = other_pos - pos;

    let x_diff = diff.x.signum();
    let y_diff = diff.y.signum();

    let past_pos = move_by(other_pos, Pos::new(x_diff, y_diff));

    if data.map.is_blocked_by_wall(other_pos, x_diff, y_diff).is_some() {
        // if blocked by wall, kill entity and take their space
        data.objects[handle].set_pos(other_pos);

        data.objects[other_handle].alive = false;
        let damage = data.objects[other_handle]
                         .fighter
                         .expect("Attacked a non-fighter?")
                         .hp;

        msg_log.log(Msg::Killed(handle, other_handle, damage));
    } else {
        // if not blocked, push the other entity, taking their space
        data.objects[other_handle].set_pos(past_pos);
        data.objects[handle].set_pos(other_pos);
    }
}

pub fn attack(handle: ObjectId, other_handle: ObjectId, objects: &mut ObjMap, msg_log: &mut MsgLog) {
    let damage = objects[handle].fighter.map_or(0, |f| f.power) -
                 objects[other_handle].fighter.map_or(0, |f| f.defense);
    if damage > 0 {
        objects[other_handle].take_damage(damage);

        msg_log.log(Msg::Attack(handle, other_handle, damage));
        if objects[other_handle].fighter.unwrap().hp <= 0 {
            objects[other_handle].alive = false;

            msg_log.log(Msg::Killed(handle, other_handle, damage));
        }
    }
}

pub fn add_pos(pos1: Pos, pos2: Pos) -> Pos {
    return Pos::new(pos1.x + pos2.x, pos1.y + pos2.y);
}

pub fn move_towards(start: Pos, end: Pos, num_blocks: usize) -> Pos {
    let line = Line::new(start.to_tuple(), end.to_tuple()).into_iter();

    return Pos::from(line.skip(num_blocks).next().unwrap_or(end.to_tuple()));
}

pub fn rand_from_x_y(x: i32, y: i32) -> f32 {
    let mut hasher = DefaultHasher::new();

    (x as u32).hash(&mut hasher);
    (y as u32).hash(&mut hasher);
 
    let result: u64 = hasher.finish();

    return ((result & 0xFFFFFFFF) as f32) / 4294967295.0;
}

pub fn lerp(first: f32, second: f32, scale: f32) -> f32 {
    return first + ((second - first) * scale);
}

pub fn lerp_color(color1: Color, color2: Color, scale: f32) -> Color {
    return Color {
        r: lerp(color1.r as f32, color2.r as f32, scale) as u8,
        g: lerp(color1.g as f32, color2.g as f32, scale) as u8,
        b: lerp(color1.b as f32, color2.b as f32, scale) as u8,
        a: lerp(color1.a as f32, color2.a as f32, scale) as u8,
    };
}

pub fn reach_by_mode(move_mode: MoveMode) -> Reach {
    match move_mode {
        MoveMode::Sneak => {
            Reach::Single(1)
        }

        MoveMode::Walk => {
            Reach::Single(1)
        }

        MoveMode::Run => {
            Reach::Single(2)
        }
    }
}

pub fn clamp<N: Ord>(val: N, min: N, max: N) -> N {
    if val < min {
        return min;
    } else if val > max {
        return max;
    } 

    return val;
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

