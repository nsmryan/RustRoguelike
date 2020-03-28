use rand::prelude::*;

use serde::{Serialize, Deserialize};

use roguelike_core::ai::{Behavior, Ai};
use roguelike_core::map::*;
use roguelike_core::types::*;
use roguelike_core::constants::*;
use roguelike_core::movement::*;
use roguelike_core::config::*;
use roguelike_core::animation::{Animation, Sprite};
use roguelike_core::utils::distance;

use crate::display::*;
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


//if we want to use a character sprite, a potential value is '\u{8B}'
pub fn make_player(config: &Config, display_state: &mut DisplayState) -> Object {
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

    let sprite = display_state.new_sprite("player_idle".to_string(), config.idle_speed)
                                     .expect("Could not find sprite 'player_idle'");

    let anim_key = display_state.play_animation(Animation::Loop(sprite));

    player.animation.push_front(anim_key);

    player
}

pub fn make_column(config: &Config, pos: Pos) -> Object {
    let object = Object::new(pos.x, pos.y, ObjType::Column, MAP_COLUMN as char, config.color_light_grey, "column", true);

    return object;
}

pub fn make_dagger(config: &Config, pos: Pos) -> Object {
    let mut object = Object::new(pos.x, pos.y, ObjType::Item, ENTITY_DAGGER as char, config.color_light_grey, "dagger", false);

    object.item = Some(Item::Dagger);

    return object;
}

pub fn make_goal(config: &Config, display_state: &mut DisplayState, pos: Pos) -> Object {
    let mut object = Object::new(pos.x, pos.y, ObjType::Item, ENTITY_GOAL as char, config.color_orange, "goal", false);
    object.item = Some(Item::Goal);

    let sprite = display_state.new_sprite("goal".to_string(), config.goal_speed)
                                     .expect("Could not find sprite 'goal'");

    let anim_key = display_state.play_animation(Animation::Loop(sprite));

    object.animation.push_front(anim_key);

    return object;
}

pub fn make_mouse(_config: &Config, _display_state: &mut DisplayState) -> Object {
    let mouse = Object::new(-1, -1, ObjType::Other, ' ', Color::white(), "mouse", false);

    mouse
}

pub fn make_gol(config: &Config, pos: Pos, display_state: &mut DisplayState) -> Object {
    let mut gol = Object::new(pos.x, pos.y, ObjType::Enemy, '\u{98}', config.color_orange, "gol", true);

    gol.fighter = Some( Fighter { max_hp: 10, hp: 10, defense: 0, power: 1, } );
    gol.ai = Some(Ai::Basic);
    gol.behavior = Some(Behavior::Idle);
    gol.color = config.color_light_orange;
    gol.movement = Some(Reach::Single(GOL_MOVE_DISTANCE));
    gol.attack = Some(Reach::Diag(GOL_ATTACK_DISTANCE));
    gol.alive = true;
    gol.direction = Some(Direction::Up);

    let sprite = display_state.new_sprite("gol_idle".to_string(), config.idle_speed)
                                     .expect("Could not find sprite 'gol_idle'");

    let anim_key = display_state.play_animation(Animation::Loop(sprite));

    gol.animation.push_front(anim_key);
    
    return gol;
} 

pub fn make_spire(config: &Config, pos: Pos) -> Object {
    let mut spire = Object::new(pos.x, pos.y, ObjType::Enemy, '\u{15}', config.color_orange, "spire", true);

    spire.fighter = Some( Fighter { max_hp: 16, hp: 16, defense: 0, power: 1, } );
    spire.ai = Some(Ai::Basic);
    spire.behavior = Some(Behavior::Idle);
    spire.color = config.color_mint_green;
    spire.movement = Some(Reach::Single(SPIRE_MOVE_DISTANCE));
    spire.attack = Some(Reach::Single(SPIRE_ATTACK_DISTANCE));
    spire.alive = true;
    spire.direction = Some(Direction::Up);

    return spire;
}

pub fn make_pawn(config: &Config, pos: Pos, display_state: &mut DisplayState) -> Object {
    let mut pawn = Object::new(pos.x, pos.y, ObjType::Enemy, '\u{A5}', config.color_orange, "pawn", true);

    pawn.fighter = Some( Fighter { max_hp: 16, hp: 16, defense: 0, power: 1, } );
    pawn.ai = Some(Ai::Basic);
    pawn.behavior = Some(Behavior::Idle);
    pawn.color = config.color_ice_blue;
    pawn.movement = Some(Reach::Single(PAWN_MOVE_DISTANCE));
    pawn.attack = Some(Reach::Single(PAWN_ATTACK_DISTANCE));
    pawn.alive = true;
    pawn.direction = Some(Direction::Up);

    let sprite = display_state.new_sprite("elf_idle".to_string(), config.idle_speed)
                                     .expect("Could not find sprite 'elf_idle'");
    let anim_key = display_state.play_animation(Animation::Loop(sprite));

    pawn.animation.push_front(anim_key);

    return pawn;
}

pub fn make_trap_sound(config: &Config, pos: Pos) -> Object {
    let mut sound = Object::new(pos.x, pos.y, ObjType::Enemy, ENTITY_TRAP_SOUND as char, config.color_ice_blue, "soudn", false);

    sound.trap = Some(Trap::Sound);

    return sound;
}

pub fn make_spikes(config: &Config, pos: Pos, display_state: &mut DisplayState) -> Object {
    let mut spikes = Object::new(pos.x, pos.y, ObjType::Enemy, MAP_TALL_SPIKES as char, config.color_ice_blue, "spike", false);

    spikes.trap = Some(Trap::Spikes);

    let sprite = display_state.new_sprite("spikes".to_string(), config.idle_speed)
                                     .expect("Could not find sprite 'spikes'");

    let anim_key = display_state.play_animation(Animation::Loop(sprite));

    spikes.animation.push_front(anim_key);

    return spikes;
}

pub fn make_key(config: &Config, pos: Pos, display_state: &mut DisplayState) -> Object {
    let mut pawn = Object::new(pos.x, pos.y, ObjType::Item, '\u{A5}', config.color_orange, "key", true);

    pawn.fighter = Some( Fighter { max_hp: 16, hp: 16, defense: 1, power: 5, } );
    pawn.ai = Some(Ai::Basic);
    pawn.behavior = Some(Behavior::Idle);
    pawn.color = config.color_ice_blue;
    pawn.movement = Some(Reach::Single(KEY_MOVE_DISTANCE));
    pawn.attack = Some(Reach::Single(KEY_ATTACK_DISTANCE));
    pawn.alive = true;

    let sprite = display_state.new_sprite("elf_idle".to_string(), config.idle_speed)
                              .expect("Could not find sprite 'elf_idle'");

    let anim_key = display_state.play_animation(Animation::Loop(sprite));

    pawn.animation.push_front(anim_key);

    return pawn;
}

pub fn make_stone(config: &Config, pos: Pos) -> Object {
    let mut stone = Object::new(pos.x, pos.y, ObjType::Item, ENTITY_STONE as char, config.color_light_grey, "stone", true);

    stone.item = Some(Item::Stone);
    stone.alive = false;
    stone.blocks = false;

    return stone;
}

pub fn make_map(map_type: &MapGenType,
                objects: &mut ObjMap,
                config: &Config,
                display_state: &mut DisplayState,
                rng: &mut SmallRng) -> (GameData, Pos) {
    let result;
    match map_type {
        MapGenType::WallTest => {
            let (map, player_position) = make_wall_test_map(objects, config, display_state);
            
            result = (GameData::new(map, objects.clone()), player_position);
        }

        MapGenType::CornerTest => {
            let (map, player_position) = make_wall_test_map(objects, config, display_state);
            
            result = (GameData::new(map, objects.clone()), player_position);
        }

        MapGenType::PlayerTest => {
            let (map, player_position) = make_player_test_map(objects, config, display_state);
            
            result = (GameData::new(map, objects.clone()), player_position);
        }

        MapGenType::Island => {
            let map = Map::from_dims(MAP_WIDTH as usize, MAP_HEIGHT as usize);

            let mut data = GameData::new(map, objects.clone());
            let starting_position = make_island(&mut data, config, display_state, rng);

            data.map[starting_position.to_tuple()].tile_type = TileType::Empty;

            data.map.update_map();

            result = (data, starting_position);
        }

        MapGenType::FromFile(file_name) => {
            let (new_objects, new_map, player_position) =
                read_map_xp(config, display_state, &file_name);

            let data = GameData::new(new_map, new_objects);

            result = (data, Pos::from(player_position));
        }

        MapGenType::Animations => {
            let (map, player_position) = make_player_test_map(objects, config, display_state);
            
            result = (GameData::new(map, objects.clone()), player_position);
        }
    }

    return result;
}

pub fn make_island(data: &mut GameData,
                   config: &Config,
                   display_state: &mut DisplayState,
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
            let mut stone = make_stone(config, pos);
            stone.item = Some(Item::Stone);
            data.objects.insert(stone);
        }
    }

    /* add monsters */
    for _ in 0..0 {
        loop {
            let pos = pos_in_radius(center, ISLAND_RADIUS, rng);

            if !data.is_blocked_tile(pos).is_some()  {
                let monster = make_gol(config, pos, display_state);
                data.objects.insert(monster);
                break;
            }
        }
    }

    for _ in 0..1 {
        loop {
            let pos = pos_in_radius(center, ISLAND_RADIUS, rng);

            if !data.is_blocked_tile(pos).is_some()  {
                let monster = make_pawn(config, pos, display_state);
                data.objects.insert(monster);
                break;
            }
        }
    }
    
    for _ in 0..0 {
        loop {
            let pos = pos_in_radius(center, ISLAND_RADIUS, rng);

            if !data.is_blocked_tile(pos).is_some() {
                let monster = make_spire(config, pos);
                data.objects.insert(monster);
                break;
            }
        }
    }

    let x = rng.gen_range(0, data.map.width());
    let y = rng.gen_range(0, data.map.height());
    let pos = Pos::new(x, y);

    if !data.is_blocked_tile(pos).is_some()  {
        let goal = make_goal(config, display_state, Pos::new(x, y));
        data.objects.insert(goal);
    }

    /* add goal object */
    let mut pos = pos_in_radius(center, ISLAND_RADIUS, rng);

    while !data.map.is_empty(pos) {
        pos = pos_in_radius(center, ISLAND_RADIUS, rng);
    }
    data.objects.insert(make_goal(&config, display_state, pos));

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
                            config: &Config,
                            _display_state: &DisplayState) -> (Map, Pos) {
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

    make_stone(config, Pos::new(1, 2));
    make_stone(config, Pos::new(4, 2));
    make_stone(config, Pos::new(3, 2));
  
    map.update_map();

    return (map, Pos::from(position));
}

pub fn make_animations_map(objects: &mut ObjMap,
                           config: &Config,
                           display_state: &mut DisplayState) -> (Map, Pos) {
    let num_animations = display_state.sprites.len() as i32;

    let dims = (num_animations as f32).sqrt() as usize + 1;

    let mut map = Map::from_dims(dims, dims);

    let mut index: i32 = 0;
    for sprite_key in display_state.sprites.keys().collect::<Vec<_>>().iter() {
        let x = index % dims as i32;
        let y = index / dims as i32;

        let num_sprites = display_state.sprites[*sprite_key].num_sprites;

        let mut obj = Object::new(x, y, ObjType::Other, ' ', Color::white(), "obj", false);

        let sprite =
            Sprite::make_sprite("".to_string(), *sprite_key, num_sprites as f32, config.idle_speed);
        let anim_key = display_state.play_animation(Animation::Loop(sprite));
        obj.animation.push_front(anim_key);

        objects.insert(obj);

        index += 1;
    }

    map.update_map();

    return (map, Pos::from((dims as i32 - 1, dims as i32 - 1)));
}

pub fn make_wall_test_map(objects: &mut ObjMap,
                          config: &Config,
                          display_state: &mut DisplayState) -> (Map, Pos) {
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
  
    objects.insert(make_gol(config, Pos::new(7, 5), display_state));

    let dagger = make_dagger(config, Pos::new(position.0, position.1));
    objects.insert(dagger);

    map.update_map();

    return (map, Pos::from(position));
}

pub fn make_corner_test_map(objects: &mut ObjMap,
                            config: &Config,
                            display_state: &mut DisplayState) -> (Map, Pos) {
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

  
    objects.insert(make_gol(config, Pos::new(7, 5), display_state));

    map.update_map();

    return (map, Pos::from(position));
}


