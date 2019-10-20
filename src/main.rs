#![allow(dead_code)]

extern crate ggez;
extern crate tcod;
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
use ggez::graphics::Image;
use ggez::graphics::spritebatch::SpriteBatch;

use timer::*;

use rexpaint::*;

use engine::types::*;
use constants::*;
use engine::display::*;
use engine::map::*;
use engine::ai::*;
use input::*;
use game::*;
use imgui_wrapper::*;


fn step_animation(objects: &mut [Object], map: &Map, animation: &mut Animation) -> bool {
    match animation {
        Animation::Thrown(obj_id, line) => {
            match line.step() {
                Some(next) => {
                    if map.tiles[next.0 as usize][next.1 as usize].block_sight {
                        return true;
                    } else {
                        objects[*obj_id].x = next.0;
                        objects[*obj_id].y = next.1;
                        return false;
                    }
                },

                None => {
                    return true;
                },
            }
        }
    }
}

/// Check whether the exit condition for the game is met.
fn exit_condition_met(inventory: &[Object], map: &Map, objects: &[Object]) -> bool {
    // loop over objects in inventory, and check whether any
    // are the goal object.
    let has_goal =
        inventory.iter().any(|obj| obj.item.map_or(false, |item| item == Item::Goal));

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

fn animations(game: &mut Game, objects: &mut [Object], map: &Map) {
    let mut finished_ixs = Vec::new();
    let mut ix = 0; 

    /* For each animation, step its state */
    for mut animation in game.animations.iter_mut() {
      let finished = step_animation(objects, map, &mut animation);

      // for a finished animation, record that it should be removed
      if finished {
          finished_ixs.push(ix)
      }
      ix += 1;
    }

    // remove finished animations
    finished_ixs.sort_unstable();
    for ix in finished_ixs.iter().rev() {
        game.animations.swap_remove(*ix);
    }
}

pub fn step_game(game: &mut Game,
                 config: &mut Config,
                 previous_player_position: &mut (i32, i32),
                 map: &mut Map,
                 objects: &mut Vec<Object>,
                 messages: &mut Messages,
                 inventory: &mut Vec<Object>,
                 input_action: InputAction) -> bool {
    /* Display */

    /* Player Action and Animations */
    // If there is an animation playing, let it finish
    *previous_player_position = (objects[PLAYER].x, objects[PLAYER].y);
    let player_action;
    if game.animations.len() > 0 {
        animations(game, objects, map);
        player_action = PlayerAction::DidntTakeTurn;
    } else {
        player_action = handle_input(game, input_action, map, objects, inventory, messages, config);
        match player_action {
          PlayerAction::Exit => {
            return false;
          }

          PlayerAction::TookTurn | PlayerAction::TookHalfTurn => {
              game.turn_count += 1;
          }
          
          _ => {}
        }
    }

    /* Check Exit Condition */
    if exit_condition_met(&inventory, map, objects) {
        std::process::exit(0);
    }

    /* AI */
    if objects[PLAYER].alive && player_action == PlayerAction::TookTurn {
        for id in 1..objects.len() {
            if objects[id].ai.is_some() {
                ai_take_turn(id, map, objects, &game.fov, &mut game.animations, config);
            }
        }
    }

    /* Reload Configuration */
    match File::open("config.json") {
        Ok(mut file) => {
            let mut config_string = String::new();
            file.read_to_string(&mut config_string).expect("Could not read config file!");
            *config = serde_json::from_str(&config_string).expect("Could not read JSON- config.json has a parsing error!");
        }
      _ => (),
    }

  if config.load_map_file_every_frame && Path::new("map.xp").exists() {
      let (new_objects, new_map, _) = read_map_xp(&config, "map.xp");
      *map = new_map;
      let player = objects[0].clone();
      objects.clear();
      objects.push(player);
      objects.extend(new_objects);

      let dims = map.size();
      game.fov = FovMap::new(dims.0, dims.1);
      setup_fov(&mut game.fov, &map);
      let fov_distance = config.fov_distance;
      game.fov.compute_fov(objects[PLAYER].x, objects[PLAYER].y, fov_distance, FOV_LIGHT_WALLS, FOV_ALGO);
  }

    if *previous_player_position != (objects[PLAYER].x, objects[PLAYER].y) {
    let player = &objects[PLAYER];
    let mut fov_distance = config.fov_distance;
    if game.god_mode {
        fov_distance = std::cmp::max(SCREEN_WIDTH, SCREEN_HEIGHT);
    }
    game.fov.compute_fov(player.x, player.y, fov_distance, FOV_LIGHT_WALLS, FOV_ALGO);
    }

    if game.god_mode {
        game.fov.compute_fov(objects[PLAYER].x, objects[PLAYER].y, 1000, FOV_LIGHT_WALLS, FOV_ALGO);
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
    let state = &mut GameState::new(ctx, &args).unwrap();
    event::run(ctx, event_loop, state).unwrap();
}

struct GameState {
    game: Game,
    config: Config,
    previous_player_position: (i32, i32),
    map: Map,
    objects: Vec<Object>,
    messages: Messages,
    inventory: Vec<Object>,
    imgui_wrapper: Gui,
    font_image: Image,
    sprite_batch: SpriteBatch,
    input_action: InputAction,
}

impl GameState {
    fn new(mut ctx: &mut Context, args: &Vec<String>) -> GameResult<GameState> {
        let imgui_wrapper = Gui::new(&mut ctx);

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

        let mut messages = Messages::new();

        let config: Config;
        {
            let mut file = File::open("config.json").expect("Could not open/parse config file config.json");
            let mut config_string = String::new();
            file.read_to_string(&mut config_string).expect("Could not read contents of config.json");
            config = serde_json::from_str(&config_string).expect("Could not parse config.json file!");
        }

        let inventory = vec![Object::make_stone(0, 0, &config)];

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

        let mut game = Game::new();

        setup_fov(&mut game.fov, &map);
        let fov_distance = config.fov_distance;
        game.fov.compute_fov(player_x, player_y, fov_distance, FOV_LIGHT_WALLS, FOV_ALGO);

        messages.message("Welcome Stranger! Prepare to perish in the Desolation of Salt!", config.color_orange.color());

        let font_image = Image::new(ctx, "/rexpaint16x16.png").unwrap();

        let sprite_batch = SpriteBatch::new(font_image.clone());

        let input_action = InputAction::None;

        let state = GameState {
            game,
            config,
            previous_player_position,
            map,
            objects,
            messages,
            inventory,
            imgui_wrapper,
            font_image,
            sprite_batch,
            input_action,
        };

        Ok(state)
    }
}


impl EventHandler for GameState {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        step_game(&mut self.game,
                  &mut self.config,
                  &mut self.previous_player_position,
                  &mut self.map,
                  &mut self.objects,
                  &mut self.messages,
                  &mut self.inventory,
                  self.input_action);

        self.input_action = InputAction::None;

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        render_all(ctx,
                   &mut self.game,
                   &self.objects,
                   &mut self.map,
                   &mut self.messages,
                   &mut self.imgui_wrapper,
                   &mut self.sprite_batch,
                   &self.config)
    }

    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32, _dx: f32, _dy: f32) {
        self.imgui_wrapper.update_mouse_pos(x, y, &mut self.game.mouse_state);
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        button: MouseButton,
        _x: f32,
        _y: f32,
    ) {
        self.imgui_wrapper.update_mouse_down((
            button == MouseButton::Left,
            button == MouseButton::Right,
            button == MouseButton::Middle),
            &mut self.game.mouse_state,
        );
    }

    fn mouse_button_up_event(
        &mut self,
        _ctx: &mut Context,
        _button: MouseButton,
        _x: f32,
        _y: f32,
    ) {
        self.imgui_wrapper.update_mouse_down((false, false, false), &mut self.game.mouse_state);
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

