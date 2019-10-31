#![allow(dead_code)]

extern crate ggez;
extern crate rand;
extern crate serde;
#[macro_use]extern crate serde_derive;
extern crate serde_json;
extern crate num;
extern crate timer;
extern crate chrono;
extern crate mint;
extern crate rexpaint;

mod engine;
mod constants;
mod input;
mod game;
mod imgui_wrapper;
mod style;
mod plat;

#[cfg(test)]
mod tests;


use std::env;
use std::path::Path;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::sync::mpsc::channel;

use rand::prelude::*;

#[allow(unused_imports)]use tcod::map::{Map as FovMap};
#[allow(unused_imports)]use tcod::console::*;
#[allow(unused_imports)]use tcod::colors::*;
#[allow(unused_imports)]use tcod::input::Key;
#[allow(unused_imports)]use tcod::input::KeyCode::*;
#[allow(unused_imports)]use tcod::input::{Event, Mouse};
#[allow(unused_imports)]use tcod::AsNative;
#[allow(unused_imports)]use tcod::image;

use ggez::event::{self, EventHandler, KeyCode, KeyMods, MouseButton};
use ggez::{Context, GameResult};
use ggez::graphics::{Canvas, Image};

use timer::*;

use rexpaint::*;

use engine::types::*;
use constants::*;
use engine::display::*;
use engine::map::*;
use engine::ai::*;
use input::*;
use game::*;
use plat::*;


/// Check whether the exit condition for the game is met.
fn exit_condition_met(map: &Map, objects: &[Object]) -> bool {
    // loop over objects in inventory, and check whether any
    // are the goal object.
    //let has_goal =
    //inventory.iter().any(|obj| obj.item.map_or(false, |item| item == Item::Goal));
    // TODO add back in with new inventory!
    let has_goal = false;

    let player_pos = (objects[PLAYER].x, objects[PLAYER].y);
    let on_exit_tile = map[player_pos].tile_type == TileType::Exit;

    let exit_condition = has_goal && on_exit_tile;

    return exit_condition;
}

pub fn run_game<F>(mut step: F)
    where F: FnMut() -> bool {
        // start game tick timer
        let timer = Timer::new();
        let (tick_sender, tick_receiver) = channel();
        let _guard = 
            timer.schedule_repeating(chrono::Duration::milliseconds(TIME_BETWEEN_FRAMES_MS), move || {
                tick_sender.send(0).unwrap();
            });

        /* main game loop */
        let mut running = true;
        while running {
            /* fps limiting */
            tick_receiver.recv().unwrap();

            running = step();
        }
    }

pub fn step_game(game: &mut Game) -> bool {
    /* Display */

    /* Player Action and Animations */
    // If there is an animation playing, let it finish
    game.settings.previous_player_position = (game.data.objects[PLAYER].x, game.data.objects[PLAYER].y);
    let player_action;
    // TODO animations removed
    /*
    if game.animations.len() > 0 {
        player_action = PlayerAction::DidntTakeTurn;
    } else {
    */
    player_action = handle_input(game.input_action,
                                 &mut game.mouse_state,
                                 &mut game.data,
                                 &mut game.settings.god_mode,
                                 &mut game.display_state.display_overlays,
                                 &game.config);
    match player_action {
        PlayerAction::Exit => {
            return false;
        }

        PlayerAction::TookTurn | PlayerAction::TookHalfTurn => {
            game.settings.turn_count += 1;
        }

        _ => {}
    }

    /* Check Exit Condition */
    if exit_condition_met(&game.data.map, &mut game.data.objects) {
        std::process::exit(0);
    }

    /* AI */
    if game.data.objects[PLAYER].alive && player_action == PlayerAction::TookTurn {
        for id in 1..game.data.objects.len() {
            if game.data.objects[id].ai.is_some() {
                ai_take_turn(id, &mut game.data.map, &mut game.data.objects, &mut game.data.fov, &game.config);
            }
        }
    }

    /* Reload Configuration */
    match File::open("config.json") {
        Ok(mut file) => {
            let mut config_string = String::new();
            file.read_to_string(&mut config_string).expect("Could not read config file!");
            game.config = serde_json::from_str(&config_string).expect("Could not read JSON- config.json has a parsing error!");
        }
        _ => (),
    }

    if game.config.load_map_file_every_frame && Path::new("map.xp").exists() {
        let (new_objects, new_map, _) = read_map_xp(&game.config, "map.xp");
        game.data.map = new_map;
        let player = game.data.objects[0].clone();
        game.data.objects.clear();
        game.data.objects.push(player);
        game.data.objects.extend(new_objects);

        game.data.fov = FovMap::new(game.data.map.width(), game.data.map.height());
        setup_fov(&mut game.data.fov, &game.data.map);
        let fov_distance = game.config.fov_distance;
        game.data.fov.compute_fov(game.data.objects[PLAYER].x,
                                        game.data.objects[PLAYER].y,
                                        fov_distance,
                                        FOV_LIGHT_WALLS,
                                        FOV_ALGO);
    }

    if game.settings.previous_player_position != (game.data.objects[PLAYER].x, game.data.objects[PLAYER].y) {
        let player = &game.data.objects[PLAYER];
        let mut fov_distance = game.config.fov_distance;
        if game.settings.god_mode {
            fov_distance = std::cmp::max(SCREEN_WIDTH, SCREEN_HEIGHT);
        }
        game.data.fov.compute_fov(player.x, player.y, fov_distance, FOV_LIGHT_WALLS, FOV_ALGO);
    }

    if game.settings.god_mode {
        game.data.fov.compute_fov(game.data.objects[PLAYER].x, game.data.objects[PLAYER].y, 1000, FOV_LIGHT_WALLS, FOV_ALGO);
    }

    return false; 
}

pub fn read_map_xp(config: &Config, file_name: &str) -> (Vec<Object>, Map, (i32, i32)) {
    let file = File::open(file_name).unwrap();
    let mut buf_reader = BufReader::new(file);
    let xp = XpFile::read(&mut buf_reader).unwrap();

    let mut map = Map::from_dims(xp.layers[0].width, xp.layers[0].height);
    let mut objects = Vec::new();
    let mut player_position = (0, 0);


    for (layer_index, layer) in xp.layers.iter().enumerate() {
        let width = layer.width;
        let height = layer.height;

        for x in 0..width {
            for y in 0..height {
                let index = y + height * x;
                let cell = layer.cells[index];

                let chr = std::char::from_u32(cell.ch).unwrap();

                match layer_index {
                    MAP_LAYER_GROUND => {
                        match chr as u8 {
                            0 => {
                            }

                            MAP_GROUND => {
                            }

                            MAP_WATER => {
                                map[(x, y)] = Tile::water();
                                map[(x, y)].chr = Some(chr);
                            }

                            _ => {
                                panic!(format!("Unexpected character {} in ground layer!", chr as u8));
                            }
                        }
                    }

                    MAP_LAYER_ENVIRONMENT => {
                        match chr as u8 {
                            MAP_THIN_WALL_TOP => {
                                map[(x, y)].chr = Some(chr);
                            }

                            MAP_THIN_WALL_BOTTOM => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_LEFT => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].left_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_RIGHT => {
                                map[(x, y)].chr = Some(chr);
                            }

                            MAP_THIN_WALL_TOP_LEFT => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].left_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_BOTTOM_LEFT => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].left_wall = Wall::ShortWall;
                                map[(x, y)].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_TOP_RIGHT => {
                                map[(x, y)].chr = Some(chr);
                            }

                            MAP_THIN_WALL_BOTTOM_RIGHT => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].bottom_wall = Wall::ShortWall;
                            }


                            MAP_THICK_WALL_TOP => {
                                map[(x, y)].chr = Some(chr);
                            }

                            MAP_THICK_WALL_LEFT => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].left_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_RIGHT => {
                                map[(x, y)].chr = Some(chr);
                            }

                            MAP_THICK_WALL_BOTTOM => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].bottom_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_TOP_LEFT => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].left_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_BOTTOM_LEFT => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].bottom_wall = Wall::TallWall;
                                map[(x, y)].left_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_TOP_RIGHT => {
                                map[(x, y)].chr = Some(chr);
                            }

                            MAP_THICK_WALL_BOTTOM_RIGHT => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].bottom_wall = Wall::TallWall;
                            }

                            MAP_DOT_TOP_LEFT => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].blocked = true;
                            }

                            MAP_DOT_TOP_RIGHT => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].blocked = true;
                            }

                            MAP_DOT_BOTTOM_LEFT => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].blocked = true;
                            }

                            MAP_DOT_BOTTOM_RIGHT => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].blocked = true;
                            }

                            MAP_ROOK => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].blocked = true;
                            }

                            MAP_DOT_MIDDLE | MAP_ORB => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].blocked = true;
                            }

                            MAP_EMPTY => {
                                map[(x, y)].chr = Some(MAP_EMPTY_CHAR as char);
                            }

                            MAP_STATUE_1 | MAP_STATUE_2 | MAP_STATUE_3 |
                                MAP_STATUE_4 | MAP_STATUE_5 | MAP_STATUE_6 => {
                                    map[(x, y)].chr = Some(chr);
                                    map[(x, y)].blocked = true;
                                }

                            MAP_WIDE_SPIKES| MAP_TALL_SPIKES => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].blocked = true;
                            }

                            MAP_WALL => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].blocked = true;
                            }

                            ENTITY_HERO => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].blocked = true;
                            }

                            ENTITY_CLOAK_GUY => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].blocked = true;
                            }

                            // TODO This should be in entity layer...
                            ENTITY_PLAYER => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].blocked = true;
                            }

                            _ => {
                                panic!(format!("Unexpected character {} in environment layer!", chr as u8));
                            }
                        }
                    }

                    MAP_LAYER_ENTITIES => {
                        match chr as u8 {
                            ENTITY_ORC => {
                                objects.push(make_orc(config, x as i32, y as i32));
                            }

                            ENTITY_SWIRL_CIRCLE => {
                                objects.push(make_kobold(config, x as i32, y as i32));
                            }

                            ENTITY_ORB => {
                                // TODO should be an objective
                            }

                            ENTITY_GOBLIN => {
                                // TODO should be different from kobold
                                objects.push(make_kobold(config, x as i32, y as i32));
                            }

                            MAP_EMPTY => {
                                // Nothing to do here...
                            }

                            ENTITY_PLAYER => {
                                player_position = (x as i32, y as i32);
                            }

                            _ => {
                                panic!(format!("Unexpected character {} in entities layer!", chr as u8));
                            }
                        }
                    }

                    _ => {
                        panic!(format!("Layer {} not expected in map file!", layer_index));
                    }
                }
            }
        }
    }

    return (objects, map, player_position);
}

fn main() {
    let args = env::args().collect::<Vec<String>>();

    let mut window_mode: ggez::conf::WindowMode = Default::default();
    window_mode.width = ((SCREEN_WIDTH - 1) * FONT_WIDTH) as f32;
    window_mode.height = (SCREEN_HEIGHT * FONT_HEIGHT)  as f32;

    let cb = ggez::ContextBuilder::new("Roguelike", "like")
        .window_mode(window_mode);
    let (ref mut ctx, event_loop) = &mut cb.build().unwrap();

    let config: Config;
    {
        let mut file = File::open("config.json").expect("Could not open/parse config file config.json");
        let mut config_string = String::new();
        file.read_to_string(&mut config_string).expect("Could not read contents of config.json");
        config = serde_json::from_str(&config_string).expect("Could not parse config.json file!");
    }

    let state = &mut Game::new(ctx, &args, config).unwrap();
    event::run(ctx, event_loop, state).unwrap();
}


pub struct Game {
    pub config: Config,

    pub input_action: InputAction,

    pub mouse_state: MouseState,

    pub display_state: DisplayState,

    pub data: GameData,

    pub settings: GameSettings,

    pub state: GameState,
}

impl Game {
    pub fn new(ctx: &mut Context, args: &Vec<String>, config: Config) -> GameResult<Game> {
        // Create seed for random number generator, either from
        // user input or randomly
        let seed: u64;
        if args.len() > 1 {
            let mut hasher = DefaultHasher::new();
            args[1].hash(&mut hasher);
            seed = hasher.finish();
        } else {
            seed = rand::thread_rng().gen();
        }
        println!("Seed: {} (0x{:X})", seed, seed);

        let previous_player_position = (-1, -1);

        let mut objects = vec!(make_player());

        let mut rng: SmallRng = SeedableRng::seed_from_u64(seed);

        let map;
        let player_position;
        if config.load_map_file {
            let (new_objects, new_map, mut position) = read_map_xp(&config, "map.xp");
            objects.extend(new_objects);
            map = new_map;
            if position == (0, 0) {
                position = (map.width() / 2, map.height() / 2);
            }
            player_position = position;
        } else {
            let (new_map, position) = make_map(&mut objects, &config, &mut rng);
            map = new_map;
            player_position = position.into_pair();
        }
        let player_x = player_position.0;
        let player_y = player_position.1;
        objects[PLAYER].x = player_x;
        objects[PLAYER].y = player_y;

        let mut fov = FovMap::new(map.width(), map.height());
        setup_fov(&mut fov, &map);
        let fov_distance = config.fov_distance;
        fov.compute_fov(player_x, player_y, fov_distance, FOV_LIGHT_WALLS, FOV_ALGO);

        let font_image = Image::new(ctx, "/rexpaint16x16.png").unwrap();

        let input_action = InputAction::None;

        let display_state = DisplayState::new(font_image, ctx);

        let data = GameData::new(map, objects, fov);

        let mut state = Game {
            config,
            input_action,
            data,
            display_state: display_state,
            settings: GameSettings::new(previous_player_position, 0, false),
            mouse_state: Default::default(),
            state: GameState::Playing,
        };

        state.display_state.screen_sections =
            Plan::vert("screen", 0.80, Plan::zone("map"), Plan::zone("inspector"));

        Ok(state)
    }
}


impl EventHandler for Game {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        step_game(self);

        self.input_action = InputAction::None;

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        render_all(ctx,
                   &mut self.mouse_state,
                   &self.data.objects,
                   &mut self.data.map,
                   &self.data.fov,
                   &mut self.display_state,
                   &self.config)
    }

    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32, _dx: f32, _dy: f32) {
        self.display_state.imgui_wrapper.update_mouse_pos(x, y, &mut self.mouse_state);
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        button: MouseButton,
        _x: f32,
        _y: f32,
        ) {
        self.display_state.imgui_wrapper.update_mouse_down((
                button == MouseButton::Left,
                button == MouseButton::Right,
                button == MouseButton::Middle),
                &mut self.mouse_state,
                );
    }

    fn mouse_button_up_event(
        &mut self,
        _ctx: &mut Context,
        _button: MouseButton,
        _x: f32,
        _y: f32,
        ) {
        self.display_state.imgui_wrapper.update_mouse_down((false, false, false), &mut self.mouse_state);
    }

    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        keycode: KeyCode,
        keymods: KeyMods,
        _repeat: bool,
        ) {
        self.input_action = map_keycode_to_action(keycode, keymods);
    }
}

