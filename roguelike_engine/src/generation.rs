use std::collections::VecDeque;

use rand::prelude::*;

use serde::{Serialize, Deserialize};

use roguelike_core::ai::{Behavior, Ai};
use roguelike_core::map::*;
use roguelike_core::types::*;
use roguelike_core::constants::*;
use roguelike_core::movement::*;
use roguelike_core::messaging::*;
use roguelike_core::config::*;
use roguelike_core::utils::{rand_from_pos, distance};

use crate::game::*;


#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum MapGenType {
    Island,
    WallTest,
    CornerTest,
    PlayerTest,
    FromFile(String),
    Animations,
}


pub fn make_player(entities: &mut Entities, config: &Config, msg_log: &mut MsgLog) -> EntityId {
    let player = entities.create_entity(0, 0, EntityType::Player, '@', Color::white(), EntityName::Player, true);

    entities.status[&player].alive = true;
    entities.fighter.insert(player,
        Fighter { max_hp: config.player_health,
                  hp: config.player_health,
                  defense: 0,
                  power: 5 });
    entities.movement.insert(player,  Reach::Single(1));
    entities.attack.insert(player,  Reach::Single(1));
    entities.move_mode.insert(player,  MoveMode::Sneak);
    entities.direction.insert(player,  Direction::Up);
    entities.inventory.insert(player,  VecDeque::new());

    let skill_set = Vec::new();
    entities.skills.insert(player,  skill_set);

    entities.class.insert(player, EntityClass::General);

    entities.energy.insert(player, 3);

    msg_log.log(Msg::SpawnedObject(player, entities.typ[&player], Pos::new(0, 0), EntityName::Player));

    return player;
}

pub fn make_column(entities: &mut Entities, config: &Config, pos: Pos, msg_log: &mut MsgLog) -> EntityId {
    let object = entities.create_entity(pos.x, pos.y, EntityType::Column, MAP_COLUMN as char, config.color_light_grey, EntityName::Column, true);

    msg_log.log(Msg::SpawnedObject(object, entities.typ[&object], pos, EntityName::Column));

    return object;
}

pub fn make_energy(entities: &mut Entities, config: &Config, pos: Pos, msg_log: &mut MsgLog) -> EntityId {
    let object = entities.create_entity(pos.x, pos.y, EntityType::Energy, ENTITY_BLINK_TRAP as char, config.color_light_green, EntityName::Energy, false);

    msg_log.log(Msg::SpawnedObject(object, entities.typ[&object], pos, EntityName::Energy));

    return object;
}

pub fn make_dagger(entities: &mut Entities, config: &Config, pos: Pos, msg_log: &mut MsgLog) -> EntityId {
    let object = entities.create_entity(pos.x, pos.y, EntityType::Item, ENTITY_DAGGER as char, config.color_light_grey, EntityName::Dagger, false);

    entities.item.insert(object,  Item::Dagger);

    msg_log.log(Msg::SpawnedObject(object, entities.typ[&object], pos, EntityName::Dagger));

    return object;
}

pub fn make_hammer(entities: &mut Entities, config: &Config, pos: Pos, msg_log: &mut MsgLog) -> EntityId {
    let object = entities.create_entity(pos.x, pos.y, EntityType::Item, ENTITY_HAMMER as char, config.color_light_grey, EntityName::Hammer, false);

    entities.item.insert(object,  Item::Hammer);

    msg_log.log(Msg::SpawnedObject(object, entities.typ[&object], pos, EntityName::Hammer));

    return object;
}

pub fn make_sword(entities: &mut Entities, config: &Config, pos: Pos, msg_log: &mut MsgLog) -> EntityId {
    let object = entities.create_entity(pos.x, pos.y, EntityType::Item, ENTITY_SWORD as char, config.color_light_grey, EntityName::Sword, false);

    entities.item.insert(object,  Item::Sword);

    msg_log.log(Msg::SpawnedObject(object, entities.typ[&object], pos, EntityName::Sword));

    return object;
}

pub fn make_shield(entities: &mut Entities, config: &Config, pos: Pos, msg_log: &mut MsgLog) -> EntityId {
    let object = entities.create_entity(pos.x, pos.y, EntityType::Item, ENTITY_SHIELD as char, config.color_light_grey, EntityName::Shield, false);

    entities.item.insert(object,  Item::Shield);

    msg_log.log(Msg::SpawnedObject(object, entities.typ[&object], pos, EntityName::Shield));

    return object;
}

pub fn make_key(entities: &mut Entities, config: &Config, pos: Pos, msg_log: &mut MsgLog) -> EntityId {
    let key = entities.create_entity(pos.x, pos.y, EntityType::Item, ENTITY_KEY as char, config.color_orange, EntityName::Key, false);
    
    entities.item.insert(key,  Item::Goal);

    msg_log.log(Msg::SpawnedObject(key, entities.typ[&key], pos, EntityName::Key));

    return key;
}

pub fn make_mouse(entities: &mut Entities, _config: &Config, msg_log: &mut MsgLog) -> EntityId {
    let mouse = entities.create_entity(-1, -1, EntityType::Other, ' ', Color::white(), EntityName::Mouse, false);

    msg_log.log(Msg::SpawnedObject(mouse, entities.typ[&mouse], Pos::new(-1, -1), EntityName::Mouse));

    mouse
}

pub fn make_gol(entities: &mut Entities, config: &Config, pos: Pos, msg_log: &mut MsgLog) -> EntityId {
    let gol = entities.create_entity(pos.x, pos.y, EntityType::Enemy, '\u{98}', config.color_orange, EntityName::Gol, true);

    entities.fighter.insert(gol,  Fighter { max_hp: 10, hp: 10, defense: 0, power: 1, });
    entities.ai.insert(gol,  Ai::Basic);
    entities.behavior.insert(gol,  Behavior::Idle);
    entities.color.insert(gol,  config.color_light_orange);
    entities.movement.insert(gol,  Reach::Single(GOL_MOVE_DISTANCE));
    entities.attack.insert(gol,  Reach::Diag(GOL_ATTACK_DISTANCE));
    entities.status[&gol].alive = true;
    entities.direction.insert(gol,  Direction::from_f32(rand_from_pos(pos)));

    msg_log.log(Msg::SpawnedObject(gol, entities.typ[&gol], pos, EntityName::Gol));
    
    return gol;
} 

pub fn make_spire(entities: &mut Entities, config: &Config, pos: Pos, msg_log: &mut MsgLog) -> EntityId {
    let spire = entities.create_entity(pos.x, pos.y, EntityType::Enemy, '\u{15}', config.color_orange, EntityName::Spire, true);

    entities.fighter.insert(spire,  Fighter { max_hp: 16, hp: 16, defense: 0, power: 1, });
    entities.ai.insert(spire,  Ai::Basic);
    entities.behavior.insert(spire,  Behavior::Idle);
    entities.color.insert(spire,  config.color_mint_green);
    entities.movement.insert(spire,  Reach::Single(SPIRE_MOVE_DISTANCE));
    entities.attack.insert(spire,  Reach::Single(SPIRE_ATTACK_DISTANCE));
    entities.status[&spire].alive = true;
    entities.direction.insert(spire,  Direction::Up);

    msg_log.log(Msg::SpawnedObject(spire, entities.typ[&spire], pos, EntityName::Spire));

    return spire;
}

pub fn make_pawn(entities: &mut Entities, config: &Config, pos: Pos, msg_log: &mut MsgLog) -> EntityId {
    let elf = entities.create_entity(pos.x, pos.y, EntityType::Enemy, '\u{A5}', config.color_orange, EntityName::Pawn, true);

    entities.fighter.insert(elf,  Fighter { max_hp: 16, hp: 16, defense: 0, power: 1, });
    entities.ai.insert(elf,  Ai::Basic);
    entities.behavior.insert(elf,  Behavior::Idle);
    entities.color.insert(elf,  config.color_ice_blue);
    entities.movement.insert(elf,  Reach::Single(PAWN_MOVE_DISTANCE));
    entities.attack.insert(elf,  Reach::Single(PAWN_ATTACK_DISTANCE));
    entities.status[&elf].alive = true;
    entities.direction.insert(elf,  Direction::from_f32(rand_from_pos(pos)));

    msg_log.log(Msg::SpawnedObject(elf, entities.typ[&elf], pos, EntityName::Pawn));

    return elf;
}

pub fn make_trap_sound(entities: &mut Entities, config: &Config, pos: Pos, msg_log: &mut MsgLog) -> EntityId {
    let sound = entities.create_entity(pos.x, pos.y, EntityType::Item, ENTITY_TRAP_SOUND as char, config.color_ice_blue, EntityName::SoundTrap, false);

    entities.trap.insert(sound,  Trap::Sound);
    entities.armed.insert(sound,  true);
    entities.item.insert(sound,  Item::SoundTrap);

    msg_log.log(Msg::SpawnedObject(sound, entities.typ[&sound], pos, EntityName::SoundTrap));

    return sound;
}

pub fn make_spikes(entities: &mut Entities, config: &Config, pos: Pos, msg_log: &mut MsgLog) -> EntityId {
    let spikes = entities.create_entity(pos.x, pos.y, EntityType::Item, MAP_TALL_SPIKES as char, config.color_ice_blue, EntityName::SpikeTrap, false);

    entities.trap.insert(spikes,  Trap::Spikes);
    entities.armed.insert(spikes,  true);
    entities.item.insert(spikes,  Item::SpikeTrap);

    msg_log.log(Msg::SpawnedObject(spikes, entities.typ[&spikes], pos, EntityName::SpikeTrap));

    return spikes;
}

pub fn make_blink_trap(entities: &mut Entities, config: &Config, pos: Pos, msg_log: &mut MsgLog) -> EntityId {
    let blink = entities.create_entity(pos.x, pos.y, EntityType::Item, ENTITY_BLINK_TRAP as char, config.color_ice_blue, EntityName::BlinkTrap, false);

    entities.trap.insert(blink,  Trap::Blink);
    entities.armed.insert(blink,  true);
    entities.item.insert(blink,  Item::BlinkTrap);

    msg_log.log(Msg::SpawnedObject(blink, entities.typ[&blink], pos, EntityName::BlinkTrap));

    return blink;
}

pub fn make_freeze_trap(entities: &mut Entities, config: &Config, pos: Pos, msg_log: &mut MsgLog) -> EntityId {
    let freeze = entities.create_entity(pos.x, pos.y, EntityType::Item, ENTITY_FREEZE_TRAP as char, config.color_ice_blue, EntityName::FreezeTrap, false);

    entities.trap.insert(freeze,  Trap::Freeze);
    entities.armed.insert(freeze,  true);
    entities.item.insert(freeze,  Item::FreezeTrap);

    msg_log.log(Msg::SpawnedObject(freeze, entities.typ[&freeze], pos, EntityName::FreezeTrap));

    return freeze;
}

pub fn make_exit(entities: &mut Entities, config: &Config, pos: Pos, msg_log: &mut MsgLog) -> EntityId {
    let exit = entities.create_entity(pos.x, pos.y, EntityType::Item, ENTITY_EXIT as char, config.color_orange, EntityName::Exit, false);

    entities.color.insert(exit,  config.color_ice_blue);

    msg_log.log(Msg::SpawnedObject(exit, entities.typ[&exit], pos, EntityName::Exit));

    return exit;
}

pub fn make_stone(entities: &mut Entities, config: &Config, pos: Pos, msg_log: &mut MsgLog) -> EntityId {
    let stone = entities.create_entity(pos.x, pos.y, EntityType::Item, ENTITY_STONE as char, config.color_light_grey, EntityName::Stone, true);

    entities.item.insert(stone,  Item::Stone);
    entities.status[&stone].alive = false;
    entities.blocks.insert(stone,  false);

    msg_log.log(Msg::SpawnedObject(stone, entities.typ[&stone], pos, EntityName::Stone));

    return stone;
}

pub fn make_island(data: &mut GameData,
                   config: &Config,
                   msg_log: &mut MsgLog,
                   rng: &mut SmallRng) -> Pos {
    let center = Pos::new(data.map.width() / 2, data.map.height() / 2);

    let mut water_tile_positions = Vec::new();

    /* Create Island */
    // the center has land, the remaining square are filled with water
    for x in 0..data.map.width() {
        for y in 0..data.map.height() {
            let pos = Pos::new(x, y);

            if distance(pos, center) <= ISLAND_RADIUS {
                data.map.tiles[x as usize][y as usize] = Tile::empty();
            } else {
                data.map.tiles[x as usize][y as usize] = Tile::water();
                water_tile_positions.push((x, y));
            }
        }
    }

    /* add obstacles */
    let obstacles = Obstacle::all_obstacles();

    for _ in 0..ISLAND_NUM_OBSTACLES {
        let rand_pos = random_offset(rng, ISLAND_RADIUS);
        let pos = Pos::new(center.x + rand_pos.x, center.y + rand_pos.y);

        let obstacle = *obstacles.choose(rng).unwrap();

        // Buildings are generated separately, so don't add them in random generation
        if obstacle != Obstacle::Building {
            add_obstacle(&mut data.map, pos, obstacle, rng);
        }
    }

    /* add buildings */
    for _ in 0..rng.gen_range(3, 5) {
        let rand_pos = random_offset(rng, ISLAND_RADIUS);
        let pos = Pos::new(center.x + rand_pos.x, center.y + rand_pos.y);
        add_obstacle(&mut data.map, pos, Obstacle::Building, rng);
    }

    /* random subtraction */
    for _ in 0..ISLAND_NUM_SUBTRACTIONS_ATTEMPTS {
        let pos = pos_in_radius(center, ISLAND_RADIUS, rng);

        if data.map[pos].tile_type == TileType::Wall {
            data.map[pos] = Tile::empty();
        }
    }

    /* random additions */
    for _ in 0..ISLAND_NUM_ADDITION_ATTEMPTS {
        let pos = pos_in_radius(center, ISLAND_RADIUS, rng);
        let obstacle = *obstacles.choose(rng).unwrap();

        if data.map[pos].tile_type == TileType::Wall {
            add_obstacle(&mut data.map, pos, obstacle, rng);
        }
    }

    /* random stones */
    for _ in 0..10 {
        let pos = pos_in_radius(center, ISLAND_RADIUS, rng);

        if data.map.is_empty(pos) {
            make_stone(&mut data.entities, config, pos, msg_log);
        }
    }

    /* add monsters */
    for _ in 0..0 {
        loop {
            let pos = pos_in_radius(center, ISLAND_RADIUS, rng);

            if !data.has_blocking_entity(pos).is_some()  {
                make_gol(&mut data.entities, config, pos, msg_log);
                break;
            }
        }
    }

    for _ in 0..1 {
        loop {
            let pos = pos_in_radius(center, ISLAND_RADIUS, rng);

            if !data.has_blocking_entity(pos).is_some()  {
                make_pawn(&mut data.entities, config, pos, msg_log);
                break;
            }
        }
    }
    
    for _ in 0..0 {
        loop {
            let pos = pos_in_radius(center, ISLAND_RADIUS, rng);

            if !data.has_blocking_entity(pos).is_some() {
                make_spire(&mut data.entities, config, pos, msg_log);
                break;
            }
        }
    }

    let x = rng.gen_range(0, data.map.width());
    let y = rng.gen_range(0, data.map.height());
    let pos = Pos::new(x, y);

    if !data.has_blocking_entity(pos).is_some()  {
        make_key(&mut data.entities, config, Pos::new(x, y), msg_log);
    }

    /* add key object */
    let mut pos = pos_in_radius(center, ISLAND_RADIUS, rng);

    while !data.map.is_empty(pos) {
        pos = pos_in_radius(center, ISLAND_RADIUS, rng);
    }
    make_key(&mut data.entities, &config, pos, msg_log);

    /* add exit */
    // find edge of island
    let map_size = data.map.size();
    let mut edge_positions = Vec::new();
    for x in 0..map_size.0 {
        for y in 0..map_size.1 {
            let pos = Pos::from((x, y));

            if !(data.map[pos].tile_type == TileType::Water) &&
                 near_tile_type(&data.map, pos, TileType::Water) {
                edge_positions.push(pos);
            }
        }
    }
    // choose a random edge position
    let edge_pos = edge_positions[rng.gen_range(0, edge_positions.len())];

    // make the random edge position the exit
    data.map.tiles[edge_pos.x as usize][edge_pos.y as usize] = Tile::exit();

    /* Ensure that objects placed outside of the island are removed */
    for pos in water_tile_positions {
        data.map[pos].tile_type = TileType::Water;
    }

    return center;
}

pub fn make_player_test_map(entities: &mut Entities,
                            config: &Config,
                            msg_log: &mut MsgLog) -> (Map, Pos) {
    let mut map = Map::from_dims(10, 10);
    let position = (1, 5);

    for wall_y_pos in 2..8 {
        let pos: (i32, i32) = (5, wall_y_pos);
        map[pos] = Tile::empty();
        map[pos].left_wall = Wall::ShortWall;
    }

    for wall_x_pos in 3..7 {
        let pos: (i32, i32) = (wall_x_pos, 5);
        map[pos] = Tile::empty();
        map[pos].bottom_wall = Wall::ShortWall;
    }

    make_stone(entities, config, Pos::new(1, 2), msg_log);
    make_stone(entities, config, Pos::new(4, 2), msg_log);
    make_stone(entities, config, Pos::new(3, 2), msg_log);
  
    return (map, Pos::from(position));
}

pub fn make_test_map(game: &mut Game) {
    //right down left should work
    //down will hit wall
    //running should jump over wall
    //back up should also work
    //up then down should jump over wall
    game.data.map[(0, 1)].bottom_wall = Wall::ShortWall;

    // right hit wall, moves to (2,2)
    // right again hit wall, doesn't move player
    game.data.map[(3, 2)].block_move = true;
    game.data.map[(3, 2)].tile_type = TileType::Wall;

    // move down works- left wall is next to player
    game.data.map[(2, 3)].left_wall = Wall::ShortWall;

    // add walls -- --
    //             v
    //             s
    //
    //where v is a monster facing down,
    //s is a spike trap.
    //make sure that the trap triggers and hurts the monster
    game.data.map[(0, 8)].block_move = true;
    game.data.map[(0, 8)].tile_type = TileType::Wall;
    game.data.map[(1, 8)].block_move = true;
    game.data.map[(1, 8)].tile_type = TileType::Wall;
    game.data.map[(3, 8)].block_move = true;
    game.data.map[(3, 8)].tile_type = TileType::Wall;

    let elf = make_pawn(&mut game.data.entities, &game.config, Pos::new(2, 9), &mut game.msg_log);
    game.data.entities.direction[&elf] = Direction::Down;
    make_spikes(&mut game.data.entities, &game.config, Pos::new(2, 10), &mut game.msg_log);


    make_dagger(&mut game.data.entities, &game.config, Pos::new(3, 10), &mut game.msg_log);

    game.data.map[(8, 10)].left_wall = Wall::ShortWall;
    make_gol(&mut game.data.entities, &game.config, Pos::new(8, 10), &mut game.msg_log);

    // test columns falling into each other
    make_column(&mut game.data.entities, &game.config, Pos::new(7, 5), &mut game.msg_log);
    make_column(&mut game.data.entities, &game.config, Pos::new(7, 6), &mut game.msg_log);

    // could add surface testing- sounds are loud on rubble, soft on grass
    
    // could add monsters and check their facing, tracking player,
    // attacking, item use, etc
}

pub fn make_wall_test_map(entities: &mut Entities,
                          config: &Config,
                          msg_log: &mut MsgLog) -> (Map, Pos) {
    let mut map = Map::from_dims(11, 11);
    let position = (1, 5);

    for wall_y_pos in 2..8 {
        let pos: (i32, i32) = (5, wall_y_pos);
        map[pos] = Tile::empty();
        map[pos].left_wall = Wall::ShortWall;
    }
    map[(4, 7)].bottom_wall = Wall::ShortWall;
    map[(4, 1)].bottom_wall = Wall::ShortWall;

    map[(4, 4)].bottom_wall = Wall::ShortWall;
    map[(5, 4)].bottom_wall = Wall::ShortWall;
  
    make_gol(entities, config, Pos::new(5, 5), msg_log);
    make_column(entities, config, Pos::new(6, 4), msg_log);

    map[(2, 6)].block_move = true;
    map[(2, 6)].chr = MAP_STATUE_1 as u8;

    make_dagger(entities, config, Pos::new(position.0, position.1), msg_log);
    make_sword(entities, config, Pos::new(1, 1), msg_log);

    return (map, Pos::from(position));
}

pub fn make_corner_test_map(entities: &mut Entities,
                            config: &Config,
                            msg_log: &mut MsgLog) -> (Map, Pos) {
    let mut map = Map::from_dims(15, 15);
    let position = (1, 5);

    let x_pos = 5;
    let y_start = 2;
    let y_end = 8;
    for wall_y_pos in y_start..=y_end {
        let pos: (i32, i32) = (x_pos, wall_y_pos);
        map[pos] = Tile::empty();
        map[pos].left_wall = Wall::ShortWall;
    }
    map[(x_pos - 1, y_end)].bottom_wall = Wall::ShortWall;
    map[(x_pos, y_end)].bottom_wall = Wall::ShortWall;
    map[(x_pos - 1, y_start - 1)].bottom_wall = Wall::ShortWall;
    map[(x_pos, y_start - 1)].bottom_wall = Wall::ShortWall;

    map[(position.0 + 1, position.1 + 2)].bottom_wall = Wall::ShortWall;
    map[(position.0 + 2, position.1 + 2)].block_move = true;
    map[(position.0 + 2, position.1 + 2)].block_sight = true;
    map[(position.0 + 2, position.1 + 2)].chr = MAP_WALL;

  
    make_gol(entities, config, Pos::new(7, 5), msg_log);

    return (map, Pos::from(position));
}

