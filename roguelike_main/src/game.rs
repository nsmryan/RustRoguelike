use rand::Rng;
use rand::prelude::SliceRandom;
use rand::prelude::*;

use tcod::line::*;

use roguelike_core::map::*;
use roguelike_core::types::*;

use crate::engine::types::*;
use crate::engine::input::*;
use crate::movement::*;
use crate::constants::*;


//if we want to use a character sprite, a potential value is '\u{8B}'
pub fn make_player(config: &Config) -> Object {
    let mut player = Object::new(0, 0, '@', config.color_ice_blue, "player", true);
    player.alive = true;
    player.fighter = Some(Fighter{max_hp: 50, hp: 50, defense: 2, power: 5 });
    player.momentum = Some(Default::default());
    player.movement = Some(Reach::Single(1));
    player.attack = Some(Reach::Single(1));

    player
}

pub fn make_orc(config: &Config, x: i32, y :i32) -> Object {
    let mut orc = Object::new(x, y, '\u{98}', config.color_orange, "orc", true);
    orc.fighter = Some( Fighter { max_hp: 10, hp: 10, defense: 0, power: 5, } );
    orc.ai = Some(Ai::Basic);
    orc.behavior = Some(Behavior::Idle);
    orc.color = config.color_light_orange;
    orc.movement = Some(Reach::Single(1));
    orc.attack = Some(Reach::Diag(5));
    orc.alive = true;
    orc
} 

pub fn make_troll(config: &Config, x: i32, y :i32) -> Object {
    let mut troll = Object::new(x, y, '\u{15}', config.color_orange, "troll", true);
    troll.fighter = Some( Fighter { max_hp: 16, hp: 16, defense: 1, power: 10, } );
    troll.ai = Some(Ai::Basic);
    troll.behavior = Some(Behavior::Idle);
    troll.color = config.color_mint_green;
    troll.movement = Some(Reach::Single(1));
    troll.attack = Some(Reach::Diag(5));
    troll.alive = true;
    troll
}

pub fn make_kobold(config: &Config, x: i32, y :i32) -> Object {
    let mut kobold = Object::new(x, y, '\u{A5}', config.color_orange, "kobold", true);
    kobold.fighter = Some( Fighter { max_hp: 16, hp: 16, defense: 1, power: 5, } );
    kobold.ai = Some(Ai::Basic);
    kobold.behavior = Some(Behavior::Idle);
    kobold.color = config.color_ice_blue;
    kobold.movement = Some(Reach::Horiz(1));
    kobold.attack = Some(Reach::Horiz(5));
    kobold.alive = true;
    kobold
}

pub fn make_map(objects: &mut Vec<Object>, config: &Config, rng: &mut SmallRng) -> (Map, Position) {
    let mut map = Map::with_vec(vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize]);

    let starting_position = make_island(&mut map, objects, config, rng);

    map[starting_position.pair()].tile_type = TileType::Empty;

    map.update_map();

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
        let rand_pos = random_offset(rng, ISLAND_RADIUS);
        let pos = Position(center.0 + rand_pos.0, center.1 + rand_pos.1);

        let obstacle = *obstacles.choose(rng).unwrap();

        // Buildings are generated separately, so don't add them in random generation
        if obstacle != Obstacle::Building {
            add_obstacle(map, pos.pair(), obstacle, rng);
        }
    }

    /* add buildings */
    for _ in 0..rng.gen_range(3, 5) {
        let rand_pos = random_offset(rng, ISLAND_RADIUS);
        let pos = Position(center.0 + rand_pos.0, center.1 + rand_pos.1);
        add_obstacle(map, pos.pair(), Obstacle::Building, rng);
    }

    /* random subtraction */
    for _ in 0..ISLAND_NUM_SUBTRACTIONS_ATTEMPTS {
        let pos = pos_in_radius(center.pair(), ISLAND_RADIUS, rng);

        if map.tiles[pos.0 as usize][pos.1 as usize].tile_type == TileType::Wall {
            map.tiles[pos.0 as usize][pos.1 as usize] = Tile::empty();
        }
    }

    /* random additions */
    for _ in 0..ISLAND_NUM_ADDITION_ATTEMPTS {
        let pos = pos_in_radius(center.pair(), ISLAND_RADIUS, rng);
        let obstacle = *obstacles.choose(rng).unwrap();

        if map.tiles[pos.0 as usize][pos.1 as usize].tile_type == TileType::Wall {
            add_obstacle(map, pos, obstacle, rng);
        }
    }

    /* random stones */
    for _ in 0..10 {
        let pos = pos_in_radius(center.pair(), ISLAND_RADIUS, rng);

        if map.is_empty(pos.0, pos.1) {
            let mut stone = Object::make_stone(config, pos.0, pos.1);
            stone.item = Some(Item::Stone);
            objects.push(stone);
        }
    }

    /* add monsters */
    for _ in 0..0 {
        loop {
            let (x, y) = pos_in_radius(center.pair(), ISLAND_RADIUS, rng);

            if !is_blocked(map, x, y, objects) {
                let monster = make_orc(config,x,y);
                objects.push(monster);
                break;
            }
        }
    }

    for _ in 0..1 {
        loop {
            let (x, y) = pos_in_radius(center.pair(), ISLAND_RADIUS, rng);

            if !is_blocked(map, x, y, objects) {
                let monster = make_kobold(config,x,y);
                objects.push(monster);
                break;
            }
        }
    }
    
    for _ in 0..0 {
        loop {
            let (x, y) = pos_in_radius(center.pair(), ISLAND_RADIUS, rng);

            if !is_blocked(map, x, y, objects) {
                let monster = make_troll(config,x,y);
                objects.push(monster);
                break;
            }
        }
    }

    let x = rng.gen_range(0, map.width());
    let y = rng.gen_range(0, map.height());

    if !is_blocked(map, x, y, objects) {
        let mut object = Object::new(x, y, ENTITY_GOAL as char, config.color_red, "goal", false);
        object.item = Some(Item::Goal);
        objects.push(object);
    }

    /* add goal object */
    let (mut x, mut y) = pos_in_radius(center.pair(), ISLAND_RADIUS, rng);
    while !map.is_empty(x, y) {
        let pos = pos_in_radius(center.pair(), ISLAND_RADIUS, rng);
        x = pos.0;
        y = pos.1;
    }
    let mut object = Object::new(x, y, ENTITY_GOAL as char, config.color_red, "goal", false);
    object.item = Some(Item::Goal);
    objects.push(object);

    /* add exit */
    // find edge of island
    let map_size = map.size();
    let mut edge_positions = Vec::new();
    for x in 0..map_size.0 {
        for y in 0..map_size.1 {
            let pos = Position::from_pair((x, y));
            if !(map[(x, y)].tile_type == TileType::Water) &&
                 near_tile_type(&map, pos.pair(), TileType::Water) {
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

#[derive(Clone, Debug, PartialEq)]
pub struct Object {
    pub x: i32,
    pub y: i32,
    pub chr: char,
    pub color: Color,
    pub name: String,
    pub blocks: bool,
    pub alive: bool,
    pub fighter: Option<Fighter>,
    pub ai: Option<Ai>,
    pub behavior: Option<Behavior>,
    pub item: Option<Item>,
    pub momentum: Option<Momentum>,
    pub movement: Option<Reach>,
    pub attack: Option<Reach>,
    pub animation: Option<Animation>,
}

impl Object {
    pub fn new(x: i32, y: i32, chr: char, color: Color, name: &str, blocks: bool) -> Self {
        Object {
            x,
            y,
            chr,
            color,
            name: name.into(),
            blocks,
            alive: false,
            fighter: None,
            ai: None,
            behavior: None,
            item: None,        
            momentum: None,
            movement: None,
            attack: None,
            animation: None,
        }
    }

    pub fn pos(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    pub fn distance_to(&self, other: &Object) -> f32 {
        return self.distance(&Position::new(other.x, other.y));
    }

    pub fn distance(&self, other: &Position) -> f32 {
        let dx = other.0 - self.x;
        let dy = other.1 - self.y;
        return ((dx.pow(2) + dy.pow(2)) as f32).sqrt();
    }

    pub fn take_damage(&mut self, damage: i32) {
        if let Some(fighter) = self.fighter.as_mut() {
            if damage > 0 {
                fighter.hp -= damage;
            }
        }

        if let Some(fighter) = self.fighter {
            if fighter.hp <= 0 {
                self.alive = false;
            }
        }
    }

    pub fn attack(&mut self, target: &mut Object) {
        let damage = self.fighter.map_or(0, |f| f.power) - target.fighter.map_or(0, |f| f.defense);

        if damage > 0 {
            //messages.message(format!("{} attacks {} for {} hit points.", self.name, target.name, damage), WHITE);
            target.take_damage(damage);
        } else {
            //messages.message(format!("{} attacks {} but it has no effect!", self.name, target.name), WHITE);
        }
    }

    pub fn heal(&mut self, amount: i32) {
        if let Some(ref mut fighter) = self.fighter {
            fighter.hp += amount;
            if fighter.hp > fighter.max_hp {
                fighter.hp = fighter.max_hp;
            }
        }
    }

    pub fn make_stone(config: &Config, x: i32, y: i32) -> Object {
        Object::new(x, y, 'o', config.color_light_grey, "stone", false)
    }
}

// TODO should put in some kind of utilities module
pub fn is_blocked(map: &Map, x: i32, y: i32, objects: &[Object]) -> bool {
    if map[(x, y)].blocked {
        return true;
    }

    let mut is_blocked = false;
    for object in objects.iter() {
        if object.blocks && object.pos() == (x, y) {
            is_blocked = true;
            break;
        }
    }

    return is_blocked;
}

pub fn clear_path(map: &Map, start: (i32, i32), end: (i32, i32), objects: &[Object]) -> bool {
    let line = Line::new((start.0, start.1), (end.0, end.1));

    let path_blocked =
        line.into_iter().any(|point| is_blocked(map, point.0, point.1, objects));

    return !path_blocked;
}

pub struct GameData {
    pub map: Map,
    pub objects: Vec<Object>,
}

impl GameData {
    pub fn new(map: Map, objects: Vec<Object>) -> GameData {
        GameData {
            map,
            objects,
        }
    }
}

pub fn handle_input(input_action: InputAction,
                    mouse_state: &MouseState,
                    game_data: &mut GameData, 
                    god_mode: &mut bool,
                    display_overlays: &mut bool,
                    config: &Config) -> PlayerAction {
    use PlayerAction::*;

    let player_action: PlayerAction;

    let player_alive = game_data.objects[PLAYER].alive;

    if mouse_state.pressed.0 {
        // TODO add back in with new inventory
        /*
        let mut found_stone = false;
        for index in 0..inventory.len() {
            let (mx, my) = (game.mouse_state.pos.0, game.mouse_state.pos.1);
            if inventory[index].item == Some(Item::Stone) {
                let item = inventory.swap_remove(index);
                throw_stone((mx as i32, my as i32), item, game, map, objects);
                found_stone = true;
                break;
            }
        }

        if found_stone {
            player_action = TookTurn;
        } else {
            player_action = DidntTakeTurn;
        }
        */
        player_action = DidntTakeTurn;
    } else {
        match (input_action, player_alive) {
            (InputAction::Move(move_action), true) => {
                player_action = player_move_or_attack(move_action,
                                                      &mut game_data.map,
                                                      &mut game_data.objects);
            }

            (InputAction::FullScreen, _) => {
                // TODO don't know how to do this in ggez...
                player_action = DidntTakeTurn;
            },

            (InputAction::Pickup, true) => {
                let item_id = game_data.objects.iter().position(|object| {
                    object.pos() == game_data.objects[PLAYER].pos() && object.item.is_some()
                });
                if let Some(_item_id) = item_id {
                    // TODO add back in with new inventory
                    //pick_item_up(item_id, objects, inventory, config);
                }
                player_action = DidntTakeTurn;
            }

            (InputAction::Inventory, true) => {
                player_action = DidntTakeTurn;
            }

            (InputAction::Exit, _) => {
                player_action = Exit;
            }

            (InputAction::ExploreAll, _) => {
                for x in 0..game_data.map.width() {
                    for y in 0..game_data.map.height() {
                        game_data.map.tiles[x as usize][y as usize].explored = true;
                    }
                }
                player_action = DidntTakeTurn;
            }

            (InputAction::RegenerateMap, _) => {
                let mut rng: SmallRng = SeedableRng::seed_from_u64(2);
                let (map_regen, _position) = make_map(&mut game_data.objects, config, &mut rng);
                game_data.map.tiles = map_regen.tiles;
                player_action = DidntTakeTurn;
            }

            (InputAction::ToggleOverlays, _) => {
                *display_overlays = !(*display_overlays);

                player_action = DidntTakeTurn;
            }

            (InputAction::GodMode, true) => {
                let fighter = game_data.objects[PLAYER].fighter.unwrap();
                let god_mode_hp = 1000000;
                game_data.objects[PLAYER].fighter =
                    Some(Fighter { hp: god_mode_hp, max_hp: god_mode_hp, ..fighter });

                // set god mode flag
                *god_mode = true;

                // set all tiles to be transparent and walkable. walkable is not current used
                // anywhere
                for x in 0..game_data.map.tiles.len() {
                    for y in 0..game_data.map.tiles[0].len() {
                        game_data.map.set_cell(x as i32, y as i32, true, true);
                    }
                }

                player_action = TookTurn;
            }

            (_, _) => {
                player_action = DidntTakeTurn;
            }
        }
    }

    return player_action;
}

fn use_item(inventory_id: usize,
            inventory: &mut Vec<Object>,
            objects: &mut [Object],
            config: &Config) {
    use Item::*;

    if let Some(item) = inventory[inventory_id].item {
        let on_use = match item {
            Stone => unimplemented!(),
            Goal => gather_goal,
        };
        match on_use(inventory_id, objects, config) {
            UseResult::UsedUp => {
                inventory.remove(inventory_id);
            }
            UseResult::Cancelled => {
                // messages.message("Cancelled", WHITE);
            }

            UseResult::Keep => {
            }
        }
    } else {
        // messages.message(format!("The {} cannot be used.", inventory[inventory_id].name), WHITE);
    }
}

fn gather_goal(_inventory_id: usize, _objects: &mut [Object], _config: &Config) -> UseResult {
    // messages.message("You've got the goal object! Nice work.", config.color_orange.color());
    UseResult::Keep
}

fn pick_item_up(object_id: usize,
                objects: &mut Vec<Object>,
                inventory: &mut Vec<Object>,
                _config: &Config) {
    if inventory.len() >= 26 {
        // messages.message(format!("Your inventory is full, cannot pick up {}", objects[object_id].name), config.color_red.color());
    } else {
        let item = objects.swap_remove(object_id);
        // messages.message(format!("You picked up a {}!", item.name), config.color_light_green.color());
        inventory.push(item);
    }
}

pub fn throw_stone(pos: (i32, i32),
                   mut stone: Object,
                   map: &mut Map,
                   objects: &mut Vec<Object>) {
    let (mx, my) = pos;

    let start_x = objects[PLAYER].x;
    let start_y = objects[PLAYER].y;
    let end_x = mx / FONT_WIDTH;
    let end_y = my / FONT_HEIGHT;
    let throw_line = Line::new((start_x, start_y), (end_x, end_y));

    // get target position in direction of player click
    let (target_x, target_y) =
        throw_line.into_iter().take(PLAYER_THROW_DIST).last().unwrap();

    stone.x = start_x;
    stone.y = start_y;
    objects.push(stone);

    // add animation to animation list
    // TODO add back in with animations
    //let animation =
    //    Animation::Thrown(obj_id,
    //                      Line::new((start_x, start_y),
    //                      (target_x, target_y)));
    //game.animations.push(animation);

    // add sound to map
    for pos in map.pos_in_radius((target_x, target_y), STONE_SOUND_RADIUS) {
        map[pos].sound = Some((target_x, target_y));
    }
}

