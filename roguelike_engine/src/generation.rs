use rand::prelude::*;

use roguelike_core::map::*;
use roguelike_core::types::*;
use roguelike_core::constants::*;
use roguelike_core::movement::*;
use roguelike_core::config::*;

use crate::display::*;
use crate::read_map::*;


pub enum MapGenType {
    Island,
    WallTest,
    FromFile(String),
}


//if we want to use a character sprite, a potential value is '\u{8B}'
pub fn make_player(config: &Config, display_state: &DisplayState) -> Object {
    let mut player = Object::new(0, 0, '@', Color::white(), "player", true);

    player.alive = true;
    player.fighter = Some(Fighter{max_hp: 50, hp: 50, defense: 2, power: 5 });
    player.momentum = Some(Default::default());
    player.movement = Some(Reach::Single(1));
    player.attack = Some(Reach::Single(1));

    let sprite_handle = display_state.lookup_sprite("player_idle".to_string())
                                     .expect("Could not find sprite 'player_idle'");
    player.animation = Some(Animation::Idle(sprite_handle, 0.0));

    player
}

pub fn make_goal(config: &Config, x: i32, y :i32) -> Object {
    let mut object = Object::new(x, y, ENTITY_GOAL as char, config.color_red, "goal", false);
    object.item = Some(Item::Goal);

    return object;
}

pub fn make_gol(config: &Config, x: i32, y :i32, display_state: &DisplayState) -> Object {
    let mut gol = Object::new(x, y, '\u{98}', config.color_orange, "gol", true);

    gol.fighter = Some( Fighter { max_hp: 10, hp: 10, defense: 0, power: 5, } );
    gol.ai = Some(Ai::Basic);
    gol.behavior = Some(Behavior::Idle);
    gol.color = config.color_light_orange;
    gol.movement = Some(Reach::Single(1));
    gol.attack = Some(Reach::Diag(4));
    gol.alive = true;

    let sprite_handle = display_state.lookup_sprite("gol_idle".to_string())
                                     .expect("Could not find sprite 'gol_idle'");
    gol.animation = Some(Animation::Idle(sprite_handle, 0.0));
    
    return gol;
} 

pub fn make_troll(config: &Config, x: i32, y :i32) -> Object {
    let mut troll = Object::new(x, y, '\u{15}', config.color_orange, "troll", true);

    troll.fighter = Some( Fighter { max_hp: 16, hp: 16, defense: 1, power: 10, } );
    troll.ai = Some(Ai::Basic);
    troll.behavior = Some(Behavior::Idle);
    troll.color = config.color_mint_green;
    troll.movement = Some(Reach::Single(1));
    troll.attack = Some(Reach::Diag(4));
    troll.alive = true;

    return troll;
}

pub fn make_elf(config: &Config, x: i32, y :i32, display_state: &DisplayState) -> Object {
    let mut elf = Object::new(x, y, '\u{A5}', config.color_orange, "elf", true);

    elf.fighter = Some( Fighter { max_hp: 16, hp: 16, defense: 1, power: 5, } );
    elf.ai = Some(Ai::Basic);
    elf.behavior = Some(Behavior::Idle);
    elf.color = config.color_ice_blue;
    elf.movement = Some(Reach::Horiz(1));
    elf.attack = Some(Reach::Horiz(4));
    elf.alive = true;

    let sprite_handle = display_state.lookup_sprite("elf_idle".to_string())
                                     .expect("Could not find sprite 'elf_idle'");
    elf.animation = Some(Animation::Idle(sprite_handle, 0.0));

    return elf;
}

pub fn make_stone(config: &Config, x: i32, y :i32) -> Object {
    let mut stone = Object::new(x, y, '\u{A5}', config.color_orange, "stone", true);

    stone.color = config.color_light_grey;
    stone.item = Some(Item::Stone);
    stone.alive = false;
    stone.blocks = false;

    return stone;
}

pub fn make_map(map_type: &MapGenType,
                objects: &mut ObjMap,
                config: &Config,
                display_state: &DisplayState,
                rng: &mut SmallRng) -> (GameData, Position) {
    let result;
    match map_type {
        MapGenType::WallTest => {
            let (map, player_position) = make_wall_test_map(objects, config, display_state);
            
            result = (GameData::new(map, objects.clone()), player_position);
        }

        MapGenType::Island => {
            let mut map = Map::from_dims(MAP_WIDTH as usize, MAP_HEIGHT as usize);

            let mut data = GameData::new(map, objects.clone());
            let starting_position = make_island(&mut data, config, display_state, rng);

            data.map[starting_position.pair()].tile_type = TileType::Empty;

            data.map.update_map();

            result = (data, starting_position);
        }

        MapGenType::FromFile(file_name) => {
            let (new_objects, new_map, player_position) =
                read_map_xp(config, display_state, &file_name);

            let data = GameData::new(new_map, new_objects);

            result = (data, Position::from_pair(player_position));
        }
    }

    return result;
}

pub fn make_island(data: &mut GameData,
                   config: &Config,
                   display_state: &DisplayState,
                   rng: &mut SmallRng) -> Position {
    let center = Position(data.map.width() / 2, data.map.height() / 2);

    let mut water_tile_positions = Vec::new();

    /* Create Island */
    // the center has land, the remaining square are filled with water
    for x in 0..data.map.width() {
        for y in 0..data.map.height() {
            let pos = Position(x, y);
            if pos.distance(&center) <= ISLAND_RADIUS {
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
        let pos = Position(center.0 + rand_pos.0, center.1 + rand_pos.1);

        let obstacle = *obstacles.choose(rng).unwrap();

        // Buildings are generated separately, so don't add them in random generation
        if obstacle != Obstacle::Building {
            add_obstacle(&mut data.map, pos.pair(), obstacle, rng);
        }
    }

    /* add buildings */
    for _ in 0..rng.gen_range(3, 5) {
        let rand_pos = random_offset(rng, ISLAND_RADIUS);
        let pos = Position(center.0 + rand_pos.0, center.1 + rand_pos.1);
        add_obstacle(&mut data.map, pos.pair(), Obstacle::Building, rng);
    }

    /* random subtraction */
    for _ in 0..ISLAND_NUM_SUBTRACTIONS_ATTEMPTS {
        let pos = pos_in_radius(center.pair(), ISLAND_RADIUS, rng);

        if data.map.tiles[pos.0 as usize][pos.1 as usize].tile_type == TileType::Wall {
            data.map.tiles[pos.0 as usize][pos.1 as usize] = Tile::empty();
        }
    }

    /* random additions */
    for _ in 0..ISLAND_NUM_ADDITION_ATTEMPTS {
        let pos = pos_in_radius(center.pair(), ISLAND_RADIUS, rng);
        let obstacle = *obstacles.choose(rng).unwrap();

        if data.map.tiles[pos.0 as usize][pos.1 as usize].tile_type == TileType::Wall {
            add_obstacle(&mut data.map, pos, obstacle, rng);
        }
    }

    /* random stones */
    for _ in 0..10 {
        let pos = pos_in_radius(center.pair(), ISLAND_RADIUS, rng);

        if data.map.is_empty(pos.0, pos.1) {
            let mut stone = make_stone(config, pos.0, pos.1);
            stone.item = Some(Item::Stone);
            data.objects.insert(stone);
        }
    }

    /* add monsters */
    for _ in 0..0 {
        loop {
            let (x, y) = pos_in_radius(center.pair(), ISLAND_RADIUS, rng);

            if !is_blocked(x, y, data) {
                let monster = make_gol(config, x, y, display_state);
                data.objects.insert(monster);
                break;
            }
        }
    }

    for _ in 0..1 {
        loop {
            let (x, y) = pos_in_radius(center.pair(), ISLAND_RADIUS, rng);

            if !is_blocked(x, y, data) {
                let monster = make_elf(config, x, y, display_state);
                data.objects.insert(monster);
                break;
            }
        }
    }
    
    for _ in 0..0 {
        loop {
            let (x, y) = pos_in_radius(center.pair(), ISLAND_RADIUS, rng);

            if !is_blocked(x, y, data) {
                let monster = make_troll(config,x,y);
                data.objects.insert(monster);
                break;
            }
        }
    }

    let x = rng.gen_range(0, data.map.width());
    let y = rng.gen_range(0, data.map.height());

    if !is_blocked(x, y, data) {
        let mut object = Object::new(x, y, ENTITY_GOAL as char, config.color_red, "goal", false);
        object.item = Some(Item::Goal);
        data.objects.insert(object);
    }

    /* add goal object */
    let (mut x, mut y) = pos_in_radius(center.pair(), ISLAND_RADIUS, rng);
    while !data.map.is_empty(x, y) {
        let pos = pos_in_radius(center.pair(), ISLAND_RADIUS, rng);
        x = pos.0;
        y = pos.1;
    }
    data.objects.insert(make_goal(&config, x, y));

    /* add exit */
    // find edge of island
    let map_size = data.map.size();
    let mut edge_positions = Vec::new();
    for x in 0..map_size.0 {
        for y in 0..map_size.1 {
            let pos = Position::from_pair((x, y));
            if !(data.map[(x, y)].tile_type == TileType::Water) &&
                 near_tile_type(&data.map, pos.pair(), TileType::Water) {
                edge_positions.push(pos);
            }
        }
    }
    // choose a random edge position
    let edge_pos = edge_positions[rng.gen_range(0, edge_positions.len())];

    // make the random edge position the exit
    data.map.tiles[edge_pos.0 as usize][edge_pos.1 as usize] = Tile::exit();

    /* Ensure that objects placed outside of the island are removed */
    for pos in water_tile_positions {
        data.map[pos].tile_type = TileType::Water;
    }

    return center;
}

pub fn make_wall_test_map(objects: &mut ObjMap,
                          config: &Config,
                          display_state: &DisplayState) -> (Map, Position) {
    let mut map = Map::from_dims(10, 10);
    let position = (1, 5);

    for wall_y_pos in 2..8 {
        let pos: (i32, i32) = (5, wall_y_pos);
        map[pos] = Tile::empty();
        map[pos].left_wall = Wall::ShortWall;
    }
  
    objects.insert(make_gol(config, 7, 5, display_state));

    map.update_map();

    return (map, Position::from_pair(position));
}

