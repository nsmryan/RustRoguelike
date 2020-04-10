use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use tcod::line::*;

use crate::ai::Behavior;
use crate::types::*;
use crate::movement::{Reach, MoveMode};
use crate::messaging::*;


pub fn distance(pos1: Pos, pos2: Pos) -> i32 {
    return (((pos1.x - pos2.x).pow(2) + (pos1.y - pos2.y).pow(2)) as f32).sqrt() as i32;
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

pub fn push_attack(handle: ObjectId, target: ObjectId, delta_pos: Pos, data: &mut GameData, msg_log: &mut MsgLog) {
    let mut killed = false;
    let mut damage = 0;

    for _ in 0..pos_mag(delta_pos) {
        let pos = data.objects[handle].pos();
        let other_pos = data.objects[target].pos();
        let diff = other_pos - pos;

        let x_diff = signedness(diff.x);
        let y_diff = signedness(diff.y);

        let past_pos = move_by(other_pos, Pos::new(x_diff, y_diff));

        if data.map.is_blocked_by_wall(other_pos, x_diff, y_diff).is_some() ||
           data.is_blocked_tile(past_pos).is_some() {
            // if blocked by wall, kill entity and take their space
            data.objects[handle].move_to(other_pos);

            data.objects[target].alive = false;
            data.objects[target].blocks = false;
            damage = data.objects[target]
                         .fighter
                         .expect("Attacked a non-fighter?")
                         .hp;

            killed = true;
        } else {
            // if not blocked, push the other entity, taking their space
            data.objects[target].set_pos(past_pos);
            data.objects[handle].move_to(other_pos);
        }
    }

    if killed {
        msg_log.log(Msg::Killed(handle, target, damage));
    } else {
        data.objects[target].messages.push(Message::Attack(handle));
    }
}

pub fn crush(handle: ObjectId, target: ObjectId, objects: &mut ObjMap, msg_log: &mut MsgLog) {
    let damage = objects[target].fighter.map_or(0, |f| f.hp);
    if damage > 0 {
        objects[target].take_damage(damage);

        objects[target].alive = false;
        objects[target].blocks = false;

        msg_log.log(Msg::Killed(handle, target, damage));
    }
}

pub fn attack(handle: ObjectId, target: ObjectId, data: &mut GameData, msg_log: &mut MsgLog) {
    if data.using(target, Item::Shield) {
        let pos = data.objects[handle].pos();
        let other_pos = data.objects[target].pos();
        let diff = other_pos - pos;

        let x_diff = diff.x.signum();
        let y_diff = diff.y.signum();

        let past_pos = move_by(other_pos, Pos::new(x_diff, y_diff));

        if !data.map.is_blocked_by_wall(other_pos, x_diff, y_diff).is_some() &&
           !data.is_blocked_tile(past_pos).is_some() {
            data.objects[target].set_pos(past_pos);
            data.objects[handle].move_to(other_pos);

            data.objects[target].messages.push(Message::Attack(handle));
        }
    } else {
        let damage = data.objects[handle].fighter.map_or(0, |f| f.power) -
                     data.objects[target].fighter.map_or(0, |f| f.defense);
        if damage > 0 {
            data.objects[target].take_damage(damage);

            msg_log.log(Msg::Attack(handle, target, damage));
            if data.objects[target].fighter.unwrap().hp <= 0 {
                data.objects[target].alive = false;
                data.objects[target].blocks = false;

                msg_log.log(Msg::Killed(handle, target, damage));
            }

            data.objects[target].messages.push(Message::Attack(handle));
        }
    }
}

pub fn stab(handle: ObjectId, target: ObjectId, objects: &mut ObjMap, msg_log: &mut MsgLog) {
    let damage = objects[target].fighter.map_or(0, |f| f.hp);

    if damage != 0 {
        msg_log.log(Msg::Attack(handle, target, damage));

        objects[target].alive = false;
        objects[target].blocks = false;

        msg_log.log(Msg::Killed(handle, target, damage));

        objects[target].messages.push(Message::Attack(handle));
    } else {
        panic!("Stabbed an enemy with no hp?");
    }
}

pub fn item_primary_at(object_id: ObjectId, objects: &mut ObjMap, index: usize) -> bool {
    let inv_len = objects[object_id].inventory.len();

    if inv_len <= index {
        return false;
    }

    let item_id = objects[object_id].inventory[index];
    let is_primary =
        objects[item_id].item.unwrap().class() == ItemClass::Primary;

    return is_primary;
}

pub fn add_pos(pos1: Pos, pos2: Pos) -> Pos {
    return Pos::new(pos1.x + pos2.x, pos1.y + pos2.y);
}

pub fn sub_pos(pos1: Pos, pos2: Pos) -> Pos {
    return Pos::new(pos1.x - pos2.x, pos1.y - pos2.y);
}

pub fn move_towards(start: Pos, end: Pos, num_blocks: usize) -> Pos {
    let line = Line::new(start.to_tuple(), end.to_tuple()).into_iter();

    return Pos::from(line.skip(num_blocks).next().unwrap_or(end.to_tuple()));
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

pub fn can_stab(data: &GameData, entity: ObjectId, target: ObjectId) -> bool {
    // NOTE this is not generic- uses ObjType::Enemy
    return data.objects[target].typ == ObjType::Enemy &&
           data.using(entity, Item::Dagger) &&
           !matches!(data.objects[target].behavior, Some(Behavior::Attacking(_)));
}

pub fn dxy(start_pos: Pos, end_pos: Pos) -> (i32, i32) {
    return (end_pos.x - start_pos.x, end_pos.y - start_pos.y);
}

pub fn move_next_to(start_pos: Pos, end_pos: Pos) -> Pos {
    if distance(start_pos, end_pos) <= 1 {
        return start_pos;
    }

    let mut line = Line::new(start_pos.to_tuple(), end_pos.to_tuple()).into_iter();

    let mut second_to_last = line.next().unwrap();

    while let Some(pos) = line.next() {
        if pos != end_pos.to_tuple() {
            second_to_last = pos;
        }
    }

    return Pos::from(second_to_last);
}

#[test]
pub fn test_move_next_to() {
    assert_eq!(move_next_to(Pos::new(0, 0), Pos::new(5, 5)), Pos::new(4, 4));
    assert_eq!(move_next_to(Pos::new(0, 0), Pos::new(1, 1)), Pos::new(0, 0));
    assert_eq!(move_next_to(Pos::new(0, 0), Pos::new(-5, -5)), Pos::new(-4, -4));
}
