use std::collections::HashSet; use std::collections::HashMap;

use roguelike_utils::comp::*;
use roguelike_utils::rng::*;
use roguelike_utils::math::*;

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
        let diff = sub_pos(other_pos, pos);

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
        if data.entities.take_damage(target, damage) {
            msg_log.log(Msg::Attack(entity, target, damage));
            // TODO consider moving this to the Attack msg
            if data.entities.hp[&target].hp <= 0 {
                msg_log.log(Msg::Killed(entity, target, damage));
            }

            data.entities.messages[&target].push(Message::Attack(entity));
        }
    }
}

pub fn stab(entity_id: EntityId, target: EntityId, entities: &mut Entities, msg_log: &mut MsgLog) {
    if matches!(entities.behavior[&target], Behavior::Attacking(_)) {
        let mut stun_turns = STAB_STUN_TURNS;

        // Add an extra turn if the golem is idle.
        if entities.behavior[&target] == Behavior::Idle {
            stun_turns += 1;
        }

        // Add an extra turn if the golem is in the current direction that the player is facing.
        let entity_pos = entities.pos[&entity_id];
        let target_pos = entities.pos[&target];
        if Direction::from_positions(entity_pos, target_pos) == Some(entities.direction[&entity_id]) {
            stun_turns += 1;
        }

        msg_log.log(Msg::Froze(target, stun_turns))
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
            // Full tile wall.
            dampen += config.dampen_blocked_tile;
        } else if blocked.wall_type == Wall::TallWall {
            // Tall inter-tile wall.
            dampen += config.dampen_tall_wall;
        } else if blocked.wall_type == Wall::ShortWall {
            // Short inter-tile wall.
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

pub fn make_move_sound(entity_id: EntityId,
                       original_pos: Pos,
                       pos: Pos,
                       move_mode: MoveMode,
                       level: &mut Level,
                       msg_log: &mut MsgLog,
                       config: &Config) {
    let mut sound_radius;

    match move_mode {
        MoveMode::Sneak => sound_radius = config.sound_radius_sneak,
        MoveMode::Walk => sound_radius = config.sound_radius_walk,
        MoveMode::Run => sound_radius = config.sound_radius_run,
    }

    if level.map[pos].surface == Surface::Rubble {
        // If the entity has no passives, or they do but are not sure footed.
        if level.entities.passive.get(&entity_id).is_none() || !level.entities.passive[&entity_id].sure_footed {
            sound_radius += config.sound_rubble_radius;
        }
    } else if level.map[pos].surface == Surface::Grass {
        sound_radius -= config.sound_grass_radius;
    }

    if sound_radius > 0 && level.entities.status[&entity_id].soft_steps > 0 {
        sound_radius -= 1;
    }

    if sound_radius > 0 && level.entities.passive[&entity_id].soft_shoes {
        sound_radius -= 1;
    }

    msg_log.log_front(Msg::Sound(entity_id, pos, sound_radius));
    msg_log.log_front(Msg::Sound(entity_id, original_pos, sound_radius));
}

pub fn trample_grass_move(level: &mut Level, start_pos: Pos, end_pos: Pos) {
    let wall_pos;
    let is_left_wall;
    match Direction::from_positions(start_pos, end_pos).unwrap() {
        Direction::Left => {
            wall_pos = start_pos;
            is_left_wall = true;
        }

        Direction::Right => {
            wall_pos = end_pos;
            is_left_wall = true;
        }

        Direction::Up => {
            wall_pos = end_pos;
            is_left_wall = false;
        }

        Direction::Down => {
            wall_pos = start_pos;
            is_left_wall = false;
        }

        Direction::DownLeft | Direction::DownRight | Direction::UpRight | Direction::UpLeft => {
            panic!("Trampling a grass wall on a diagonal isn't possible!");
        }
    }

    let material;
    if is_left_wall {
        material = level.map[wall_pos].left_material;
    } else {
        material = level.map[wall_pos].bottom_material;
    }

    if material == Surface::Grass {
        if is_left_wall {
            level.map[wall_pos].left_material = Surface::Floor;
            level.map[wall_pos].left_wall = Wall::Empty;
        } else {
            level.map[wall_pos].bottom_material = Surface::Floor;
            level.map[wall_pos].bottom_wall = Wall::Empty;
        }
    }
}

pub fn trample_grass_walls(level: &mut Level, start_pos: Pos, end_pos: Pos) {
    match Direction::from_positions(start_pos, end_pos).unwrap() {
        Direction::Left | Direction::Right | Direction::Up | Direction::Down => {
            trample_grass_move(level, start_pos, end_pos);
        }

        Direction::DownLeft | Direction::DownRight => {
            trample_grass_move(level, start_pos, move_y(start_pos, 1));
            trample_grass_move(level, move_y(start_pos, 1), end_pos);
        }

        Direction::UpLeft | Direction::UpRight => {
            trample_grass_move(level, start_pos, move_y(start_pos, -1));
            trample_grass_move(level, move_y(start_pos, -1), end_pos);
        }
    }
}

pub fn inventory_drop_item(entity_id: EntityId,
                           item_index: usize,
                           level: &mut Level,
                           msg_log: &mut MsgLog) {
    let entity_pos = level.entities.pos[&entity_id];

    if let Some(item_id) = level.entities.inventory[&entity_id].get(item_index).map(|v| *v) {
        // Find a place to drop the item, without placing it on the same tile
        // as another item.
        let mut found_tile = false;
        let mut dist = 1;
        while !found_tile && dist < 10 {
            let positions = floodfill(&level.map, entity_pos, dist);

            for pos in positions {
                if level.item_at_pos(pos).is_none() {
                    level.entities.remove_item(entity_id, item_id);
                    level.entities.set_pos(item_id, pos);

                    msg_log.log(Msg::DroppedItem(entity_id, item_id));
                    msg_log.log(Msg::Moved(item_id, MoveType::Blink, MoveMode::Walk, pos));
                    found_tile = true;
                    break;
                }
            }

            dist += 1;
        }

        if !found_tile {
            msg_log.log(Msg::DropFailed(entity_id));
        }
    }
}

pub fn change_move_mode(entity_id: EntityId,
                        increase: bool,
                        level: &mut Level,
                        msg_log: &mut MsgLog) {
    if increase {
        let holding_shield = level.using(entity_id, Item::Shield).is_some();
        let holding_hammer = level.using(entity_id, Item::Hammer).is_some();

        let move_mode = level.entities 
                            .move_mode
                            .get(&entity_id)
                            .expect("Entity should have had a move mode!");
        let new_move_mode = move_mode.increase();

        if new_move_mode == MoveMode::Run && (holding_shield || holding_hammer) {
            msg_log.log(Msg::TriedRunWithHeavyEquipment);
        } else {
            msg_log.log(Msg::MoveMode(entity_id, new_move_mode));
        }
    } else {
        let new_move_mode = level.entities.move_mode[&entity_id].decrease();
        msg_log.log(Msg::MoveMode(entity_id, new_move_mode));
    }
}

pub fn find_blink_pos(pos: Pos, rng: &mut Rand32, level: &mut Level) -> Option<Pos> {
    let mut potential_positions = floodfill(&level.map, pos, BLINK_RADIUS);
    while potential_positions.len() > 0 {
        let ix = rng_range_u32(rng, 0, potential_positions.len() as u32) as usize;
        let rand_pos = potential_positions[ix];

        if level.has_blocking_entity(rand_pos).is_none() &&
           level.map.path_blocked_move(pos, rand_pos).is_none() {
               return Some(rand_pos);
        }

        potential_positions.swap_remove(ix);
    }
    
    return None;
}

pub fn hammer_swing(entity_id: EntityId, item_id: EntityId, pos: Pos, level: &mut Level, msg_log: &mut MsgLog) {
    let entity_pos = level.entities.pos[&entity_id];

    msg_log.log_front(Msg::Blunt(entity_pos, pos));

    if let Some(blocked) = level.map.path_blocked_move(entity_pos, pos) {
        msg_log.log_front(Msg::HammerHitWall(entity_id, blocked));
        level.used_up_item(entity_id, item_id);
    } else if let Some(hit_entity) = level.has_blocking_entity(pos) {
        // we hit another entity!
        msg_log.log_front(Msg::HammerHitEntity(entity_id, hit_entity));
        level.used_up_item(entity_id, item_id);
    }

    level.entities.took_turn[&entity_id] = true;
}

pub fn hammer_hit_entity(entity_id: EntityId, hit_entity: EntityId, level: &mut Level, msg_log: &mut MsgLog, config: &Config) {
    let first = level.entities.pos[&entity_id];
    let second = level.entities.pos[&hit_entity];

    let dxy = sub_pos(second, first);
    let direction = Direction::from_dxy(dxy.x, dxy.y).unwrap();
    let amount = 1;
    msg_log.log(Msg::Pushed(entity_id, hit_entity, direction, amount, false));
    msg_log.log_front(Msg::Sound(entity_id, second, config.sound_radius_hammer));

    if let Some(hp) = level.entities.hp.get(&hit_entity) {
        let damage = hp.hp;

        msg_log.log(Msg::Killed(entity_id, hit_entity, damage));
        msg_log.log(Msg::Sound(entity_id, second, config.sound_radius_blunt));
    }
}

pub fn freeze_trap_triggered(trap: EntityId, cause_id: EntityId, level: &mut Level, msg_log: &mut MsgLog, config: &Config) {
    let source_pos = level.entities.pos[&trap];

    let freeze_aoe =
        aoe_fill(&level.map, AoeEffect::Freeze, source_pos, config.freeze_trap_radius, config);

    let who_hit =
        level.within_aoe(&freeze_aoe);

    for obj_id in who_hit {
        // TODO probably need to filter out a bit more
        if obj_id != cause_id && level.entities.status[&obj_id].alive {
            msg_log.log(Msg::Froze(obj_id, FREEZE_TRAP_NUM_TURNS));
        }
    }
}

pub fn place_rubble(pos: Pos, map: &mut Map) {
    map[pos].surface = Surface::Rubble;
    map[pos].block_move = false;
    map[pos].tile_type = TileType::Empty;
}

pub fn try_use_energy(entity_id: EntityId, skill: Skill, level: &mut Level, msg_log: &mut MsgLog) -> bool {
    let pos = level.entities.pos[&entity_id];

    // Use the Skill's own class instead of the entities.
    //let class = level.entities.class[&entity_id];
    let class = skill.class();

    // NOTE this uses the entity's class, not the skill's class
    let has_energy = level.entities.status[&entity_id].test_mode || level.entities.energy[&entity_id] > 0;
    let mut enough_energy: bool = false;
    let mut used_energy: bool = false;
    match class {
        EntityClass::General => {
            if has_energy {
                enough_energy = true;
                used_energy = true;
                level.entities.use_energy(entity_id);
            }
        }

        EntityClass::Grass => {
            let free_energy = level.map[pos].surface == Surface::Grass;
            if free_energy || has_energy {
                if !free_energy && has_energy {
                    used_energy = true;
                    level.entities.use_energy(entity_id);
                }

                enough_energy = true;
                level.map[pos].surface = Surface::Floor;

                if let Some(grass_id) = level.entities.get_names_at_pos(pos, EntityName::Grass).get(0) {
                    msg_log.log(Msg::Remove(*grass_id));
                }
            }
        }

        EntityClass::Monolith => {
            let free_energy = level.map[pos].surface == Surface::Rubble;
            if free_energy || has_energy {
                if !free_energy && has_energy {
                    level.entities.use_energy(entity_id);
                    used_energy = true;
                }

                enough_energy = true;
                level.map[pos].surface = Surface::Floor;
            }
        }

        EntityClass::Clockwork => {
            if has_energy {
                enough_energy = true;
                used_energy = true;
                level.entities.use_energy(entity_id);
            }
        }

        EntityClass::Hierophant => {
            if has_energy {
                enough_energy = true;
                used_energy = true;
                level.entities.use_energy(entity_id);
            }
        }

        EntityClass::Wind => {
            // The wind class does not use energy.
            enough_energy = true;
        }
    }

    if used_energy {
        msg_log.log(Msg::UsedEnergy(entity_id));
    }

    return enough_energy;
}

pub fn remove_entity(entity_id: EntityId, level: &mut Level) {
    // The entity can already be removed if the removal message was logged
    // to indicate to other systems an internal change in state such as a new map.
    if level.entities.ids.contains(&entity_id) {
        level.entities.status[&entity_id].alive = false;

        level.entities.blocks[&entity_id] = false;

        level.entities.mark_for_removal(entity_id);
    }
}

pub fn lerp_color(color1: Color, color2: Color, scale: f32) -> Color {
    return Color {
        r: lerp(color1.r as f32, color2.r as f32, scale) as u8,
        g: lerp(color1.g as f32, color2.g as f32, scale) as u8,
        b: lerp(color1.b as f32, color2.b as f32, scale) as u8,
        a: lerp(color1.a as f32, color2.a as f32, scale) as u8,
    };
}

