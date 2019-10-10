use std::fs::File;
use std::io::BufReader;

use rand::Rng;
use rand::prelude::SliceRandom;
use rand::prelude::*;

use rexpaint::*;

#[allow(unused_imports)]use tcod::map::{Map as FovMap};

use ggez::graphics::WHITE;

use crate::engine::types::*;
use crate::constants::*;
use crate::engine::map::*;


//if we want to use a character sprite, a potential value is '\u{8B}'
pub fn make_player() -> Object {
    let mut player = Object::new(0, 0, '@', "player", WHITE, true);
    player.alive = true;
    player.fighter = Some(Fighter{max_hp: 50, hp: 50, defense: 2, power: 5, on_death: DeathCallback::Player });
    player.momentum = Some(Default::default());
    player.movement = Some(Reach::Single(1));
    player.attack = Some(Reach::Single(1));

    player
}

pub fn make_orc(config: &Config, x: i32, y :i32) -> Object {
    let mut orc = Object::new(x, y, '\u{98}', "orc", config.color_light_orange.color(), true);
    orc.fighter = Some( Fighter { max_hp: 10, hp: 10, defense: 0, power: 5, on_death: DeathCallback::Monster } );
    orc.ai = Some(Ai::Basic);
    orc.behavior = Some(Behavior::Idle);
    orc.color = config.color_light_orange.color();
    orc.movement = Some(Reach::Single(1));
    orc.attack = Some(Reach::Diag(5));
    orc.alive = true;
    orc
} 

pub fn make_troll(config: &Config, x: i32, y :i32) -> Object {
    let mut troll = Object::new(x, y, '\u{15}', "troll", config.color_mint_green.color(), true);
    troll.fighter = Some( Fighter { max_hp: 16, hp: 16, defense: 1, power: 10, on_death: DeathCallback::Monster } );
    troll.ai = Some(Ai::Basic);
    troll.behavior = Some(Behavior::Idle);
    troll.color = config.color_mint_green.color();
    troll.movement = Some(Reach::Single(1));
    troll.attack = Some(Reach::Diag(5));
    troll.alive = true;
    troll
}

pub fn make_kobold(config: &Config, x: i32, y :i32) -> Object {
    let mut kobold = Object::new(x, y, '\u{A5}', "kobold", config.color_ice_blue.color(), true);
    kobold.fighter = Some( Fighter { max_hp: 16, hp: 16, defense: 1, power: 5, on_death: DeathCallback::Monster } );
    kobold.ai = Some(Ai::Basic);
    kobold.behavior = Some(Behavior::Idle);
    kobold.color = config.color_ice_blue.color();
    kobold.movement = Some(Reach::Horiz(1));
    kobold.attack = Some(Reach::Horiz(5));
    kobold.alive = true;
    kobold
}

pub fn setup_fov(fov: &mut FovMap, map: &Map) {
    let dims = map.size();

    for y in 0..dims.1 {
        for x in 0..dims.0 {
            fov.set(x,
                    y,
                    !map.tiles[x as usize][y as usize].block_sight,
                    !map.tiles[x as usize][y as usize].blocked);
        }
    }
}

pub fn make_map(objects: &mut Vec<Object>, config: &Config, rng: &mut SmallRng) -> (Map, Position) {
    let mut map = Map::with_vec(vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize]);

    let starting_position = make_island(&mut map, objects, config, rng);

    // TODO TESTING: remove once between tile walls work
    map[starting_position].bottom_wall = Wall::TallWall;
    map[(starting_position.0, starting_position.1 + 1)].left_wall   = Wall::TallWall;
    
    map[starting_position].tile_type = TileType::Empty;

    (map, starting_position)
}

pub fn make_island(map: &mut Map,
                   objects: &mut Vec<Object>,
                   config: &Config,
                   rng: &mut SmallRng) -> Position {
    let center = Position(map.width() / 2, map.height() / 2);

    let mut water_tile_positions = Vec::new();

    /* Create Island */
    // the center has land, the remaining square are filled with water
    for x in 0..map.width() {
        for y in 0..map.height() {
            let pos = Position(x, y);
            if pos.distance(&center) <= ISLAND_RADIUS {
                map.tiles[x as usize][y as usize] = Tile::empty();
            } else {
                map.tiles[x as usize][y as usize] = Tile::water();
                water_tile_positions.push((x, y));
            }
        }
    }

    /* add obstacles */
    let obstacles = Obstacle::all_obstacles();

    for _ in 0..ISLAND_NUM_OBSTACLES {
        let rand_pos = random_offset(rng);
        let pos = Position(center.0 + rand_pos.0, center.1 + rand_pos.1);

        let obstacle = *obstacles.choose(rng).unwrap();

        // Buildings are generated separately, so don't add them in random generation
        if obstacle != Obstacle::Building {
            add_obstacle(map, &pos, obstacle, rng);
        }
    }

    /* add buildings */
    for _ in 0..rng.gen_range(3, 5) {
        let rand_pos = random_offset(rng);
        let pos = Position(center.0 + rand_pos.0, center.1 + rand_pos.1);
        add_obstacle(map, &pos, Obstacle::Building, rng);
    }

    /* random subtraction */
    for _ in 0..ISLAND_NUM_SUBTRACTIONS_ATTEMPTS {
        let pos = pos_in_radius(center, ISLAND_RADIUS, rng);

        if map.tiles[pos.0 as usize][pos.1 as usize].tile_type == TileType::Wall {
            map.tiles[pos.0 as usize][pos.1 as usize] = Tile::empty();
        }
    }

    /* random additions */
    for _ in 0..ISLAND_NUM_ADDITION_ATTEMPTS {
        let pos = pos_in_radius(center, ISLAND_RADIUS, rng);
        let obstacle = *obstacles.choose(rng).unwrap();

        if map.tiles[pos.0 as usize][pos.1 as usize].tile_type == TileType::Wall {
            add_obstacle(map, &pos, obstacle, rng);
        }
    }

    /* random stones */
    for _ in 0..10 {
        let pos = pos_in_radius(center, ISLAND_RADIUS, rng);

        if map.is_empty(pos.0, pos.1, &objects) {
            let mut stone = Object::make_stone(pos.0, pos.1, config);
            stone.item = Some(Item::Stone);
            objects.push(stone);
        }
    }

    /* add monsters */
    for _ in 0..2 {
        loop {
            let (x, y) = pos_in_radius(center, ISLAND_RADIUS, rng).pair();

            if !map.is_blocked(x, y, objects) {
                let monster = make_orc(config,x,y);
                objects.push(monster);
                break;
            }
        }
    }

    for _ in 0..2 {
        loop {
            let (x, y) = pos_in_radius(center, ISLAND_RADIUS, rng).pair();

            if !map.is_blocked(x, y, objects) {
                let monster = make_kobold(config,x,y);
                objects.push(monster);
                break;
            }
        }
    }
    
    for _ in 0..2 {
        loop {
            let (x, y) = pos_in_radius(center, ISLAND_RADIUS, rng).pair();

            if !map.is_blocked(x, y, objects) {
                let monster = make_troll(config,x,y);
                objects.push(monster);
                break;
            }
        }
    }

    let x = rng.gen_range(0, map.width());
    let y = rng.gen_range(0, map.height());

    if !map.is_blocked(x, y, objects) {
        let mut object = Object::new(x,y, '\u{FD}', "goal", config.color_red.color(), false);
        object.item = Some(Item::Goal);
        objects.push(object);
    }

    /* add goal object */
    let (mut x, mut y) = pos_in_radius(center, ISLAND_RADIUS, rng).pair();
    while !map.is_empty(x, y, &objects) {
        let pos = pos_in_radius(center, ISLAND_RADIUS, rng).pair();
        x = pos.0;
        y = pos.1;
    }
    let mut object = Object::new(x, y, '\u{FD}', "goal", config.color_red.color(), false);
    object.item = Some(Item::Goal);
    objects.push(object);

    /* add exit */
    // find edge of island
    let map_size = map.size();
    let mut edge_positions = Vec::new();
    for x in 0..map_size.0 {
        for y in 0..map_size.1 {
            let pos = Position::from_pair(&(x, y));
            if !(map[(x, y)].tile_type == TileType::Water) && near_tile_type(&map, pos, TileType::Water) {
                edge_positions.push(pos);
            }
        }
    }
    // choose a random edge position
    let edge_pos = edge_positions[rng.gen_range(0, edge_positions.len())];

    // make the random edge position the exit
    map.tiles[edge_pos.0 as usize][edge_pos.1 as usize] = Tile::exit();

    /* Ensure that objects placed outside of the island are removed */
    for pos in water_tile_positions {
        map[pos].tile_type = TileType::Water;
    }

    return center;
}

/// Check whether the exit condition for the game is met.
pub fn exit_condition_met(inventory: &[Object], map: &Map, objects: &[Object]) -> bool {
    // loop over objects in inventory, and check whether any
    // are the goal object.
    let has_goal =
        inventory.iter().any(|obj| obj.item.map_or(false, |item| item == Item::Goal));

    let player_pos = (objects[PLAYER].x, objects[PLAYER].y);
    let on_exit_tile = map[player_pos].tile_type == TileType::Exit;

    let exit_condition = has_goal && on_exit_tile;

    return exit_condition;
}

pub fn read_map_xp(file_name: &str) -> Map {
    let file = File::open(file_name).unwrap();
    let mut buf_reader = BufReader::new(file);
    let mut map_lines = Vec::new();

    let xp = XpFile::read(&mut buf_reader).unwrap();

    for layer in xp.layers {
        let width = layer.width;
        let height = layer.height;

        for x in 0..width {
            let mut line = Vec::new();

            for y in 0..height {
                let index = width * y + x;
                let cell = layer.cells[index];

                let chr = std::char::from_u32(cell.ch).unwrap();
                let tile = 
                    match chr {
                        ' ' => Tile::empty(),
                        '\u{9C}' | '\u{9D}' | '\u{9E}' | '\u{9F}' => Tile::short_wall_with(Some(chr)),
                        '#' | '\u{DC}' | '\u{EC}' | '\u{ED}' | '\u{FE}' => Tile::wall_with(Some(chr)),
                        'w' | '\u{AB}' => Tile::water(),
                        'x' => Tile::exit(),
                        '\u{DB}' | '\u{DD}' => Tile::empty(),
                        '\u{99}' => Tile::empty(), // TODO torch?
                        '\u{8d}' => Tile::empty(), // TODO dot?
                        '\u{91}' => Tile::empty(), // TODO orc?
                        '\u{a1}' => Tile::empty(), // TODO mage?
                        '\u{92}' => Tile::empty(), // TODO rocks?
                        '\u{a2}' => Tile::empty(), // TODO idk?
                        '\u{9f}' => Tile::empty(), // TODO idk?
                        '\u{86}' => Tile::empty(), // TODO idk?
                        _ => Tile::empty(), // panic!(format!("Unexpected char '{}' ({}) in map!", chr, cell.ch)),
                    };

                line.push(tile);
            }

            map_lines.push(line);
        }
    }

    let map = Map::with_vec(map_lines);

    //for x in 0..MAP_WIDTH {
    //    let mut line = Vec::new();
    //    for y in 0..MAP_HEIGHT {
    //        line.push(rot_map[y as usize][x as usize]);
    //    }
    //    map.tiles.push(line);
    //}

    return map;
}
