use rand::prelude::*;

use serde::{Serialize, Deserialize};

use roguelike_core::ai::{Behavior, Ai};
use roguelike_core::map::*;
use roguelike_core::types::*;
use roguelike_core::constants::*;
use roguelike_core::movement::*;
use roguelike_core::messaging::*;
use roguelike_core::config::*;
use roguelike_core::animation::{Animation, Sprite};
use roguelike_core::utils::{rand_from_pos, distance};

use crate::read_map::*;


#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum MapGenType {
    Island,
    WallTest,
    CornerTest,
    PlayerTest,
    FromFile(String),
    Animations,
}


pub fn make_player(entities: &mut Entities, config: &Config, msg_log: &mut MsgLog) -> Object {
    let mut player = Object::new(0, 0, ObjType::Player, '@', Color::white(), "player", true);

    player.alive = true;
    player.fighter =
        Some(Fighter { max_hp: config.player_health,
                       hp: config.player_health,
                       defense: 0,
                       power: 5 });
    player.movement = Some(Reach::Single(1));
    player.attack = Some(Reach::Single(1));
    player.move_mode = Some(MoveMode::Walk);
    player.direction = Some(Direction::Up);

    msg_log.log(Msg::SpawnedObject(player.id));

    player
}

pub fn make_column(entities: &mut Entities, config: &Config, pos: Pos) -> Object {
    let object = Object::new(pos.x, pos.y, ObjType::Column, MAP_COLUMN as char, config.color_light_grey, "column", true);

    return object;
}

pub fn make_dagger(entities: &mut Entities, config: &Config, pos: Pos) -> Object {
    let object = entities::create_entity(pos.x, pos.y, ObjType::Item, ENTITY_DAGGER as char, config.color_light_grey, "dagger", false);

    entities.item[object] = Item::Dagger;

    return object;
}

pub fn make_shield(entities: &mut Entities, config: &Config, pos: Pos) -> Object {
    let mut object = Object::new(pos.x, pos.y, ObjType::Item, ENTITY_SHIELD as char, config.color_light_grey, "shield", false);

    object.item = Some(Item::Shield);

    return object;
}

pub fn make_key(entities: &mut Entities, config: &Config, pos: Pos, msg_log: &mut MsgLog) -> EntityId {
    let key = entities::create_entity(pos.x, pos.y, ObjType::Item, ENTITY_KEY as char, config.color_orange, "key", false); object.item = Some(Item::Goal);

    msg_log.log(Msg::SpawnedObject(key));

    return key;
}

pub fn make_mouse(entities: &mut Entities, _config: &Config) -> Object {
    let mouse = Object::new(-1, -1, ObjType::Other, ' ', Color::white(), "mouse", false);

    mouse
}

pub fn make_gol(entities: &mut Entities, config: &Config, pos: Pos, msg_log: &mut MsgLog) -> Object {
    let gol = entities::create_entities(pos.x, pos.y, ObjType::Enemy, '\u{98}', config.color_orange, "gol", true);

    entities.fighter[gol] = Fighter { max_hp: 10, hp: 10, defense: 0, power: 1, };
    entities.ai[gol] = Ai::Basic;
    entities.behavior[gol] = Behavior::Idle;
    entities.color[gol] = config.color_light_orange;
    entities.movement[gol] = Reach::Single(GOL_MOVE_DISTANCE);
    entities.attack[gol] = Reach::Diag(GOL_ATTACK_DISTANCE);
    entities.alive[gol] = true;
    entities.direction[gol] = Direction::from_f32(rand_from_pos(pos));

    msg_log.log(Msg::SpawnedObject(*gol));
    
    return gol;
} 

pub fn make_spire(entities: &mut Entities, config: &Config, pos: Pos) -> Object {
    let spire = entities::create_entity(pos.x, pos.y, ObjType::Enemy, '\u{15}', config.color_orange, "spire", true);

    entities.fighter[spire] = Fighter { max_hp: 16, hp: 16, defense: 0, power: 1, };
    entities.ai[spire] = Ai::Basic;
    entities.behavior[spire] = Behavior::Idle;
    entities.color[spire] = config.color_mint_green;
    entities.movement[spire] = Reach::Single(SPIRE_MOVE_DISTANCE);
    entities.attack[spire] = Reach::Single(SPIRE_ATTACK_DISTANCE);
    entities.alive[spire] = true;
    entities.direction[spire] = Direction::Up;

    return spire;
}

pub fn make_elf(entities: &mut Entities, config: &Config, pos: Pos, msg_log: &mut MsgLog) -> Object {
    let elf = entities::create_entity(pos.x, pos.y, ObjType::Enemy, '\u{A5}', config.color_orange, "elf", true);

    entities.fighter[eld] = Some( Fighter { max_hp: 16, hp: 16, defense: 0, power: 1, } );
    entities.ai[eld] = Some(Ai::Basic);
    entities.behavior[eld] = Some(Behavior::Idle);
    entities.color[eld] = config.color_ice_blue;
    entities.movement[eld] = Some(Reach::Single(PAWN_MOVE_DISTANCE));
    entities.attack[eld] = Some(Reach::Single(PAWN_ATTACK_DISTANCE));
    entities.alive[eld] = true;
    entities.direction[eld] = Some(Direction::from_f32(rand_from_pos(pos)));

    msg_log.log(Msg::SpawnedObject(elf));

    return elf;
}

pub fn make_trap_sound(entities: &mut Entities, config: &Config, pos: Pos) -> Object {
    let mut sound = Object::new(pos.x, pos.y, ObjType::Enemy, ENTITY_TRAP_SOUND as char, config.color_ice_blue, "soudn", false);

    sound.trap = Some(Trap::Sound);

    return sound;
}

pub fn make_spikes(entities: &mut Entities, config: &Config, pos: Pos, msg_log: &mut MsgLog) -> Object {
    let mut spikes = Object::new(pos.x, pos.y, ObjType::Enemy, MAP_TALL_SPIKES as char, config.color_ice_blue, "spike", false);

    spikes.trap = Some(Trap::Spikes);

    msg_log.log(Msg::SpawnedObject(spikes.id));

    return spikes;
}

pub fn make_exit(entities: &mut Entities, config: &Config, pos: Pos) -> Object {
    let mut exit = Object::new(pos.x, pos.y, ObjType::Item, ENTITY_EXIT as char, config.color_orange, "exit", false);

    exit.color = config.color_ice_blue;

    return exit;
}

pub fn make_stone(entities: &mut Entities, config: &Config, pos: Pos) -> Object {
    let stone = entities::create_entity(pos.x, pos.y, ObjType::Item, ENTITY_STONE as char, config.color_light_grey, "stone", true);

    entities.item[stone] = Item::Stone;
    entities.alive[stone] = false;
    entities.blocks[stone] = false;

    return stone;
}

pub fn make_map(map_type: &MapGenType,
                objects: &mut ObjMap,
                config: &Config,
                msg_log: &mut MsgLog,
                rng: &mut SmallRng) -> (GameData, Pos) {
    let result;
    match map_type {
        MapGenType::WallTest => {
            let (map, player_position) = make_wall_test_map(objects, config, msg_log);
            
            result = (GameData::new(map, objects.clone()), player_position);
        }

        MapGenType::CornerTest => {
            let (map, player_position) = make_corner_test_map(objects, config, msg_log);
            
            result = (GameData::new(map, objects.clone()), player_position);
        }

        MapGenType::PlayerTest => {
            let (map, player_position) = make_player_test_map(objects, config);
            
            result = (GameData::new(map, objects.clone()), player_position);
        }

        MapGenType::Island => {
            let map = Map::from_dims(MAP_WIDTH as usize, MAP_HEIGHT as usize);

            let mut data = GameData::new(map, objects.clone());
            let starting_position = make_island(&mut data, config, msg_log, rng);

            data.map[starting_position.to_tuple()].tile_type = TileType::Empty;

            data.map.update_map();

            result = (data, starting_position);
        }

        MapGenType::FromFile(file_name) => {
            let (new_objects, new_map, player_position) =
                read_map_xp(config, msg_log, &file_name);

            let data = GameData::new(new_map, new_objects);

            result = (data, Pos::from(player_position));
        }

        MapGenType::Animations => {
            let (map, player_position) = make_player_test_map(objects, config);
            
            result = (GameData::new(map, objects.clone()), player_position);
        }
    }

    return result;
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
            make_stone(&mut data.entities, config, pos);
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
                make_elf(&mut data.entities, config, pos, msg_log);
                break;
            }
        }
    }
    
    for _ in 0..0 {
        loop {
            let pos = pos_in_radius(center, ISLAND_RADIUS, rng);

            if !data.has_blocking_entity(pos).is_some() {
                make_spire(&mut data.entities, config, pos);
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

pub fn make_player_test_map(_objects: &mut ObjMap,
                            config: &Config) -> (Map, Pos) {
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

    make_stone(objects, config, Pos::new(1, 2));
    make_stone(objects, config, Pos::new(4, 2));
    make_stone(objects, config, Pos::new(3, 2));
  
    map.update_map();

    return (map, Pos::from(position));
}

pub fn make_wall_test_map(objects: &mut ObjMap,
                          config: &Config,
                          msg_log: &mut MsgLog) -> (Map, Pos) {
    let mut map = Map::from_dims(10, 10);
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
  
    make_gol(objects, config, Pos::new(6, 5), msg_log);

    make_dagger(objects, config, Pos::new(position.0, position.1));

    map.update_map();

    return (map, Pos::from(position));
}

pub fn make_corner_test_map(objects: &mut ObjMap,
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
    map[(position.0 + 2, position.1 + 2)].blocked = true;
    map[(position.0 + 2, position.1 + 2)].block_sight = true;
    map[(position.0 + 2, position.1 + 2)].chr = Some(MAP_WALL as char);

  
    make_gol(objects, config, Pos::new(7, 5), msg_log);

    map.update_map();

    return (map, Pos::from(position));
}


