use tcod::line::*;

use crate::types::*;
use crate::messaging::*;


pub fn distance(pos1: Pos, pos2: Pos) -> i32 {
    return (((pos1.x - pos2.x).pow(2) + (pos1.y - pos2.y).pow(2)) as f32).sqrt() as i32;
}

pub fn attack(handle: ObjectId, other_handle: ObjectId, objects: &mut ObjMap, msg_log: &mut MsgLog) {
    let damage = objects[handle].fighter.map_or(0, |f| f.power) -
                 objects[other_handle].fighter.map_or(0, |f| f.defense);
    if damage > 0 {
        objects[other_handle].take_damage(damage);

        if objects[other_handle].fighter.unwrap().hp <= 0 {
            objects[other_handle].alive = false;

            msg_log.log(Msg::Killed(handle, other_handle, damage));
        } else {
            msg_log.log(Msg::Attack(handle, other_handle, damage));
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
