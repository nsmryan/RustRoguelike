use std::collections::HashSet; use std::collections::HashMap;

use roguelike_utils::comp::*;

use roguelike_map::*;

use crate::ai::Behavior;
use crate::types::*;
use crate::movement::{Reach, MoveMode, check_collision, MoveType};
use crate::messaging::*;
use crate::config::Config;
use crate::constants::*;
use crate::entities::*;
use crate::level::*;


pub fn reduce_item_durability(data: &mut Level, entity_id: EntityId, item_id: EntityId) {
    if let Some(durability) = data.entities.durability.get_mut(&item_id) {
        if *durability > 1 {
            *durability -= 1;
        } else {
            data.entities.remove_item(entity_id, item_id);
        }
    }
}

pub fn update_stance(move_type: MoveType, move_mode: MoveMode, stance: Stance) -> Stance {
    let mut new_stance = stance;

    if move_type == MoveType::Pass && move_mode != MoveMode::Sneak {
        new_stance = Stance::Standing;
    } else if move_type == MoveType::Pass {
        new_stance = stance.waited(move_mode);
    } else if move_mode == MoveMode::Run {
        new_stance = Stance::Running;
    } else if move_mode == MoveMode::Sneak {
        new_stance = Stance::Crouching;
    } else if move_mode == MoveMode::Walk {
        new_stance = Stance::Standing;
    }

    return new_stance;
}

#[test]
fn test_update_stance() {
    assert_eq!(Stance::Crouching, update_stance(MoveType::Pass, MoveMode::Sneak, Stance::Standing));
    assert_eq!(Stance::Crouching, update_stance(MoveType::Move, MoveMode::Sneak, Stance::Standing));
    assert_eq!(Stance::Standing, update_stance(MoveType::Pass, MoveMode::Walk, Stance::Crouching));
    assert_eq!(Stance::Standing, update_stance(MoveType::Pass, MoveMode::Walk, Stance::Standing));
    assert_eq!(Stance::Running, update_stance(MoveType::Move, MoveMode::Run, Stance::Standing));
    assert_eq!(Stance::Running, update_stance(MoveType::Move, MoveMode::Run, Stance::Crouching));
}

pub fn push_attack(entity_id: EntityId,
                   target: EntityId,
                   direction: Direction,
                   move_into: bool,
                   data: &mut Level,
                   config: &Config,
                   msg_log: &mut MsgLog) -> bool {
    let mut continue_push = true;

    let mut killed = false;
    let mut damage = 0;

    let pos = data.entities.pos[&entity_id];
    let other_pos = data.entities.pos[&target];

    let push_dxy = direction.into_move();
    let x_diff = signedness(push_dxy.x);
    let y_diff = signedness(push_dxy.y);

    let move_result = check_collision(other_pos, x_diff, y_diff, data);

    let past_pos = move_by(other_pos, Pos::new(x_diff, y_diff));

    if move_result.no_collision() {
        if move_into {
            let move_into_pos = move_towards(pos, other_pos, 1);
            msg_log.log_front(Msg::Moved(entity_id, MoveType::Move, MoveMode::Walk, move_into_pos));
        }

        data.entities.status[&target].frozen += config.push_stun_turns;

        msg_log.log(Msg::Moved(target, MoveType::Move, MoveMode::Walk, past_pos));
    } else {
        // otherwise crush them against the wall/entity
        damage = data.entities.hp[&target].hp;

        killed = true;
        msg_log.log_front(Msg::Crushed(target, other_pos));

        // once we crush an entity, we lose the rest of the move
        continue_push = false;
    }

    if killed {
        msg_log.log(Msg::Killed(entity_id, target, damage));
    } else {
        data.entities.messages[&target].push(Message::Attack(entity_id));
    }

    return continue_push;
}

pub fn crush(handle: EntityId, target: EntityId, entities: &mut Entities, msg_log: &mut MsgLog) {
    let damage = entities.hp.get(&target).map_or(0, |f| f.hp);
    if damage > 0 {
        entities.take_damage(target, damage);

        entities.status[&target].alive = false;
        entities.blocks[&target] = false;

        msg_log.log(Msg::Killed(handle, target, damage));
    }
}

pub fn attack(entity: EntityId, target: EntityId, data: &mut Level, msg_log: &mut MsgLog) {
    if data.using(entity, Item::Hammer).is_some() {
        data.entities.status[&target].alive = false;
        data.entities.blocks[&target] = false;

        data.entities.take_damage(target, HAMMER_DAMAGE);
        data.entities.messages[&target].push(Message::Attack(entity));

        // NOTE assumes that this kills the enemy
        msg_log.log(Msg::Killed(entity, target, HAMMER_DAMAGE));

        let hit_pos = data.entities.pos[&target];
        // NOTE this creates rubble even if the player somehow is hit by a hammer...
        if data.map[hit_pos].surface == Surface::Floor {
            data.map[hit_pos].surface = Surface::Rubble;
        }
    } else if data.using(target, Item::Shield).is_some() {
        let pos = data.entities.pos[&entity];
        let other_pos = data.entities.pos[&target];
        let diff = other_pos - pos;

        let x_diff = diff.x.signum();
        let y_diff = diff.y.signum();

        let past_pos = move_by(other_pos, Pos::new(x_diff, y_diff));

        if !data.map.path_blocked_move(other_pos, Pos::new(x_diff, y_diff)).is_some() &&
           !data.has_blocking_entity(past_pos).is_some() {
            data.entities.set_pos(target, past_pos);
            data.entities.set_pos(entity, other_pos);

            data.entities.messages[&target].push(Message::Attack(entity));
        }
    } else if data.using(entity, Item::Sword).is_some() {
        msg_log.log(Msg::Attack(entity, target, SWORD_DAMAGE));
        msg_log.log(Msg::Killed(entity, target, SWORD_DAMAGE));
    } else {
        // NOTE could add another section for the sword- currently the same as normal attacks
        let damage = 1;
        if damage > 0 && data.entities.status[&target].alive && data.entities.status[&target].stone == 0 {
            data.entities.take_damage(target, damage);

            msg_log.log(Msg::Attack(entity, target, damage));
            // TODO consider moving this to the Attack msg
            if data.entities.hp[&target].hp <= 0 {
                data.entities.status[&target].alive = false;
                data.entities.blocks[&target] = false;

                msg_log.log(Msg::Killed(entity, target, damage));
            }

            data.entities.messages[&target].push(Message::Attack(entity));
        }
    }
}

pub fn stab(entity_id: EntityId, target: EntityId, entities: &mut Entities, msg_log: &mut MsgLog) {
    let damage = entities.hp.get(&target).map_or(0, |f| f.hp);

    if damage != 0 {
        if entities.behavior[&target] == Behavior::Idle {
            msg_log.log(Msg::Attack(entity_id, target, damage));

            entities.status[&target].alive = false;
            entities.blocks[&target] = false;

            msg_log.log(Msg::Killed(entity_id, target, damage));

            entities.messages[&target].push(Message::Attack(entity_id));
        } else {
            msg_log.log(Msg::Froze(target, STAB_STUN_TURNS))
        }
    } else {
        panic!("Stabbed an enemy with no hp?");
    }
}

pub fn item_type_available(entity_id: EntityId, entities: &mut Entities, item_class: ItemClass) -> Option<usize> {
    let inv_len = entities.inventory[&entity_id].len();

    for ix in 0..inv_len {
        let item_id = entities.inventory[&entity_id][ix];
        if entities.item[&item_id].class() == item_class {
            return Some(ix);
        }
    }

    return None;
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

pub fn clampf(val: f32, min: f32, max: f32) -> f32 {
    if val < min {
        return min;
    } else if val > max {
        return max;
    } 

    return val;
}

pub fn can_stab(data: &Level, entity: EntityId, target: EntityId) -> bool {
    let entity_pos = data.entities.pos[&entity];
    let target_pos = data.entities.pos[&target];

    // NOTE this is not generic- uses EntityType::Enemy
    let is_enemy = data.entities.typ[&target] == EntityType::Enemy;
    let using_dagger = data.using(entity, Item::Dagger).is_some();
    let clear_path = data.clear_path_up_to(entity_pos, target_pos, false);
    let not_attacking = !matches!(data.entities.behavior.get(&target), Some(Behavior::Attacking(_)));

    return is_enemy && using_dagger && clear_path && not_attacking;
}

pub fn sound_dampening(map: &Map, start_pos: Pos, end_pos: Pos, config: &Config) -> i32 {
    if distance(start_pos, end_pos) > 1 {
        panic!("Sound dampening may not work for distances longer then one tile!");
    }

    let mut dampen = 0;
    if let Some(blocked) = map.path_blocked_move(start_pos, end_pos) {
        if blocked.blocked_tile {
            dampen += config.dampen_blocked_tile;
        } else if blocked.wall_type == Wall::TallWall {
            dampen += config.dampen_tall_wall;
        } else if blocked.wall_type == Wall::ShortWall {
            dampen += config.dampen_short_wall;
        }
    }

    return dampen;
}

// AOE fill uses a floodfill to get potential positions.
// For Sound, the floodfill dampens based on objects in the environment.
// For all others, only positions that can be reached from the start position are kept
pub fn aoe_fill(map: &Map, aoe_effect: AoeEffect, start: Pos, radius: usize, config: &Config) -> Aoe {
    let flood = 
        if aoe_effect == AoeEffect::Sound {
            floodfill_sound(map, start, radius, config)
        } else {
            floodfill(map, start, radius)
        };

    let mut aoe_dists = vec![Vec::new(); radius + 1];

    for pos in flood.iter() {
        let dist = distance(start, *pos);

        let mut aoe_hit = true;
        if aoe_effect != AoeEffect::Sound {
            // must be blocked to and from a position to be blocked.
            let is_blocked_to = map.path_blocked_move(start, *pos).is_some();
            let is_blocked_from = map.path_blocked_move(*pos, start).is_some();

            let is_blocked = is_blocked_to && is_blocked_from;
            if !is_blocked && dist <= radius as i32 {
                aoe_hit = true;
            } else {
                aoe_hit = false;
            }
        }

        if aoe_hit {
            aoe_dists[dist as usize].push(*pos);
        }
    }

    return Aoe::new(aoe_effect, aoe_dists);
}

pub fn floodfill_sound(map: &Map, start: Pos, radius: usize, config: &Config) -> Vec<Pos> {
    let mut flood: Vec<Pos> = Vec::new();

    let mut seen: HashSet<(Pos, i32)> = HashSet::new();
    let mut current: Vec<(Pos, i32)> = Vec::new();
    current.push((start, 0));
    seen.insert((start, 0));
    flood.push(start);

    for _index in 0..radius {
        let last = current.clone();
        current.clear();
        for (pos, cost) in last.iter() {
            let adjacents = map.neighbors(*pos);

            for next_pos in adjacents {
                if pos.x < 0 || pos.y < 0 || next_pos.x < 0 || next_pos.y < 0 {
                    dbg!(pos, next_pos);
                    panic!("sound damping will crash with this input!");
                }

                let new_cost = 1 + cost + sound_dampening(map, *pos, next_pos, config);

                if new_cost > radius as i32 {
                    continue;
                }

                // check if we have seen this position before
                let maybe_seen = seen.iter()
                                     .filter(|(last_pos, _last_cost)| *last_pos == next_pos)
                                     .next()
                                     .map(|pair| *pair);
                if let Some((last_pos, last_cost)) = maybe_seen {
                    // if we have seen it before, but we reached it with more force, still
                    // mark as seen, but enqueue again.
                    if last_cost > new_cost {
                        seen.remove(&(last_pos, last_cost));
                        seen.insert((next_pos, new_cost));
                        current.push((next_pos, new_cost));

                        // no need to queue to flood again- the position was already seen
                    }
                } else {
                    // record having seen this position.
                    seen.insert((next_pos, new_cost));
                    current.push((next_pos, new_cost));
                    flood.push(next_pos);
                }
            }
        }
    }

    return flood;
}

#[test]
fn test_floodfill_sound_1() {
    let config = Config::from_file("../config.yaml");

    let mut map = Map::from_dims(10, 10);
    // s#..
    // x#..
    // ....

    map[(1, 0)].block_move = true;
    map[(1, 1)].block_move = true;

    let start = Pos::new(0, 0);
    let radius = 1;
    let hits = floodfill_sound(&map, start, radius, &config);
    assert_eq!(2, hits.len());
    assert!(hits.contains(&Pos::new(0, 0)));
    assert!(hits.contains(&Pos::new(0, 1)));
}

#[test]
fn test_floodfill_sound_2() {
    let config = Config::from_file("../config.yaml");

    let mut map = Map::from_dims(10, 10);
    // s#..
    // x#..
    // xx..

    map[(1, 0)].block_move = true;
    map[(1, 1)].block_move = true;

    let start = Pos::new(0, 0);
    let radius = 2;
    let hits = floodfill_sound(&map, start, radius, &config);
    assert_eq!(4, hits.len());
    assert!(hits.contains(&Pos::new(0, 0)));
    assert!(hits.contains(&Pos::new(0, 1)));
    assert!(hits.contains(&Pos::new(0, 2)));
    assert!(hits.contains(&Pos::new(1, 2)));
}

#[test]
fn test_floodfill_sound_3() {
    let config = Config::from_file("../config.yaml");

    let mut map = Map::from_dims(10, 10);
    // s#..
    // x#x.
    // xxx.
    // xxx.

    map[(1, 0)].block_move = true;
    map[(1, 1)].block_move = true;

    let start = Pos::new(0, 0);
    let radius = 3;
    let hits = floodfill_sound(&map, start, radius, &config);
    assert_eq!(9, hits.len());
    assert!(hits.contains(&Pos::new(0, 0)));
    assert!(hits.contains(&Pos::new(0, 1)));
    assert!(hits.contains(&Pos::new(0, 2)));
    assert!(hits.contains(&Pos::new(0, 3)));

    assert!(hits.contains(&Pos::new(1, 2)));
    assert!(hits.contains(&Pos::new(1, 3)));

    assert!(hits.contains(&Pos::new(2, 1)));
    assert!(hits.contains(&Pos::new(2, 2)));
    assert!(hits.contains(&Pos::new(2, 3)));
}

#[test]
fn test_floodfill_sound_through_wall() {
    let config = Config::from_file("../config.yaml");

    let mut map = Map::from_dims(10, 10);
    // the sound can only reach (0, 2) by going through
    // the wall, as it would need to travel 7 tiles
    // to reach it otherwise
    // s#..
    // .#..
    // .#..
    // .#..

    map[(1, 0)].block_move = true;
    map[(1, 1)].block_move = true;
    map[(1, 2)].block_move = true;
    map[(1, 3)].block_move = true;

    let start = Pos::new(0, 0);
    let radius = config.dampen_blocked_tile as usize + 2;
    let hits = floodfill_sound(&map, start, radius, &config);

    assert!(hits.contains(&Pos::new(2, 0)));
}

#[test]
fn test_floodfill_sound_through_tall_wall() {
    let config = Config::from_file("../config.yaml");

    let mut map = Map::from_dims(10, 10);
    // the sound can only reach (0, 2) by going through
    // the wall. The radius is set to just barely allow
    // the sound to pass through
    map[(1, 0)].left_wall = Wall::TallWall;
    map[(1, 1)].left_wall = Wall::TallWall;
    map[(1, 2)].left_wall = Wall::TallWall;
    map[(1, 3)].left_wall = Wall::TallWall;

    let start = Pos::new(0, 0);
    let radius = config.dampen_tall_wall as usize + 2;
    let hits = floodfill_sound(&map, start, radius, &config);

    assert!(hits.contains(&Pos::new(2, 0)));
}

#[test]
fn test_floodfill_sound_through_short_wall() {
    let config = Config::from_file("../config.yaml");

    let mut map = Map::from_dims(10, 10);
    // the sound can only reach (0, 2) by going through
    // the wall. The radius is set to just barely allow
    // the sound to pass through
    map[(1, 0)].left_wall = Wall::ShortWall;
    map[(1, 1)].left_wall = Wall::ShortWall;
    map[(1, 2)].left_wall = Wall::ShortWall;
    map[(1, 3)].left_wall = Wall::ShortWall;

    let start = Pos::new(0, 0);
    let radius = config.dampen_short_wall as usize + 2;
    let hits = floodfill_sound(&map, start, radius, &config);

    assert!(hits.contains(&Pos::new(2, 0)));
}

#[test]
fn test_floodfill_sound_not_through_blocked() {
    let config = Config::from_file("../config.yaml");

    let mut map = Map::from_dims(10, 10);
    // the sound can only reach (0, 2) by going through
    // the wall. The radius is set to just barely allow
    // passing a short wall, but not enough for a blocked
    // tile.
    map[(1, 0)].block_move = true;
    map[(1, 1)].block_move = true;
    map[(1, 2)].block_move = true;
    map[(1, 3)].block_move = true;

    let start = Pos::new(0, 0);
    let radius = config.dampen_short_wall as usize + 2;
    let hits = floodfill_sound(&map, start, radius, &config);

    assert!(!hits.contains(&Pos::new(2, 0)));
}

pub fn floodfill(map: &Map, start: Pos, radius: usize) -> Vec<Pos> {
    let mut flood: Vec<Pos> = Vec::new();

    let mut seen: Vec<Pos> = Vec::new();
    let mut current: Vec<Pos> = Vec::new();
    current.push(start);
    seen.push(start);
    flood.push(start);

    for _index in 0..radius {
        let last = current.clone();
        current.clear();
        for pos in last.iter() {
            let adj = astar_neighbors(map, start, *pos, Some(radius as i32));
            for (next_pos, _cost) in adj {
                if !seen.contains(&next_pos) {
                    // record having seen this position.
                    seen.push(next_pos);
                    current.push(next_pos);
                    flood.push(next_pos);
                }
            }
        }
    }

    return flood;
}

#[test]
fn test_floodfill() {
    let mut map = Map::from_dims(10, 10);

    let start = Pos::new(5, 5);

    let flood: Vec<Pos> = floodfill(&map, start, 0);
    assert_eq!(vec!(start), flood);

    let flood: Vec<Pos> = floodfill(&map, start, 1);
    assert_eq!(9, flood.len());

    map[(5, 5)].left_wall = Wall::ShortWall;
    map[(5, 6)].left_wall = Wall::ShortWall;
    map[(5, 4)].left_wall = Wall::ShortWall;
    let flood: Vec<Pos> = floodfill(&map, start, 1);
    assert_eq!(6, flood.len());

    map[(6, 3)].left_wall = Wall::ShortWall;
    map[(5, 3)].left_wall = Wall::ShortWall;

    map[(6, 4)].left_wall = Wall::ShortWall;
    map[(5, 4)].left_wall = Wall::ShortWall;

    map[(6, 5)].left_wall = Wall::ShortWall;
    map[(5, 5)].left_wall = Wall::ShortWall;
    map[start].bottom_wall = Wall::ShortWall;
    let flood: Vec<Pos> = floodfill(&map, start, 2);
    assert!(flood.contains(&start));
    assert!(flood.contains(&Pos::new(5, 4)));
    assert!(flood.contains(&Pos::new(5, 3)));

    let flood: Vec<Pos> = floodfill(&map, start, 3);
    assert_eq!(6, flood.len());
}

pub struct Cone {
    start: Pos,
    dir: Direction,
    length: i32,
    cur_length: i32,
    within_length: i32,
}

impl Cone {
    pub fn new(start: Pos, dir: Direction, length: i32) -> Cone {
        return Cone { start, dir, length, cur_length: 0, within_length: 0 };
    }
}

impl Iterator for Cone {
    type Item = Pos;

    fn next(&mut self) -> Option<Self::Item> {
        let cur_pos;
        let cone_width;

        if self.dir.diag() {
            cone_width = self.cur_length + 1;

            let left_dir = self.dir.counterclockwise();
            let leftmost_pos = left_dir.offset_pos(self.start, self.cur_length);

            let right_dir = self.dir.clockwise().clockwise();
            cur_pos = right_dir.offset_pos(leftmost_pos, self.within_length);
        } else {
            cone_width = self.cur_length * 2 + 1;

            let len_pos = self.dir.offset_pos(self.start, self.cur_length);

            let left_dir = self.dir.counterclockwise().counterclockwise();
            let left_most = left_dir.offset_pos(len_pos, self.cur_length);

            let right_dir = self.dir.clockwise().clockwise();
            cur_pos = right_dir.offset_pos(left_most, self.within_length);
        }

        if self.within_length + 1 == cone_width {
            self.cur_length += 1;
            self.within_length = 0;
        } else {
            self.within_length += 1;
        }

        if self.cur_length == self.length && self.within_length > 0 {
            return None;
        } else {
            return Some(cur_pos);
        }
    }
}

#[test]
pub fn test_cone_up() {
    let start = Pos::new(0, 0);
    let cone = Cone::new(start, Direction::Up, 3); 
    let positions = cone.collect::<Vec<Pos>>();
    dbg!(&positions);

    assert_eq!(9, positions.len());

    assert_eq!(start, positions[0]);

    assert_eq!(Pos::new(-1, -1), positions[1]);
    assert_eq!(Pos::new(0, -1), positions[2]);
    assert_eq!(Pos::new(1, -1), positions[3]);

    assert_eq!(Pos::new(-2, -2), positions[4]);
    assert_eq!(Pos::new(-1, -2), positions[5]);
    assert_eq!(Pos::new(0, -2), positions[6]);
    assert_eq!(Pos::new(1, -2), positions[7]);
    assert_eq!(Pos::new(2, -2), positions[8]);
}

pub fn map_fill_metric(map: &Map) -> HashMap<Pos, usize> {
    let mut metric_map: HashMap<Pos, usize> = HashMap::new();

    for y in 0..map.height() {
        for x in 0..map.width() {
            let pos = Pos::new(x, y);
            let fill_metric = tile_fill_metric(map, pos);
            metric_map.insert(pos, fill_metric);
        }
    }

    return metric_map;
}

pub fn tile_fill_metric(map: &Map, pos: Pos) -> usize {
    if !map[pos].block_move && map[pos].tile_type != TileType::Water {
        let near_count = floodfill(map, pos, TILE_FILL_METRIC_DIST).len();

        return near_count;
    }
    return 0
}

