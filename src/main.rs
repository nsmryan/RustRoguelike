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
extern crate rodio;
extern crate mint;

mod engine;
mod constants;
mod input;
mod game;
mod imgui_wrapper;

#[cfg(test)]
mod tests;


use std::env;
use std::path::Path;
use std::io::Write;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::fs::File;
use std::io::BufReader;
use std::io::BufRead;
use std::io::Read;
use std::sync::mpsc::channel;
use std::time::Instant;

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
use ggez::graphics;
use ggez::graphics::Image;
use ggez::graphics::spritebatch::SpriteBatch;
use ggez::graphics::Drawable;

use timer::*;

use rodio::Source;

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

/// Play a sound file.
/// This implementation is inefficient, but simple.
pub fn play_sound(file_name: &str) {
    let device = rodio::default_output_device().unwrap();
    let file = File::open(file_name).unwrap();
    let source = rodio::Decoder::new(BufReader::new(file)).unwrap();
    rodio::play_raw(&device, source.convert_samples());
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
    /* Handle Inputs */
    // TODO removed for ggez, should go somewhere else
    //let mut key = Default::default();
    //match tcod::input::check_for_event(tcod::input::MOUSE | tcod::input::KEY_PRESS) {
    //    Some((_, Event::Mouse(m))) => game.mouse = m,
    //    Some((_, Event::Key(k))) => key = k,
    //    _ => {
    //        key = Default::default();
    //        game.mouse.lbutton_pressed = false;
    //        game.mouse.rbutton_pressed = false;
    //    },
    //}
    //dbg!(key.printable);

    /* Display */
    let fov_recompute = *previous_player_position != (objects[PLAYER].x, objects[PLAYER].y);
    // TODO removed for ggez
    //render_all(game, 
    //           objects,
    //           map,
    //           messages,
    //           fov_recompute,
    //           config);

    // TODO removed for ggez
    // game.root.flush();

    for object in objects.iter() {
        // TODO removed for ggez
        // object.clear(&mut game.console);
    }

    for clearable in game.needs_clear.iter() {
        // TODO removed for ggez
        // game.console.put_char(clearable.0, clearable.1, ' ', BackgroundFlag::None);
    }
    // TODO removed for ggez
    //  game.needs_clear.clear();

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
                ai_take_turn(id, map, objects, &game.fov, &mut game.animations);
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

  if config.load_map_file && Path::new("map.csv").exists() {
      *map = read_map("map.csv");
  }

  // TODO removed for ggez
  // return !game.root.window_closed();
  return false; 
}

pub fn read_map(file_name: &str) -> Map {
    let file = File::open(file_name).unwrap();
    let file = BufReader::new(file);
    let mut map = Map::empty();
    let mut rot_map = Vec::new();

    for line in file.lines() {
        let mut rot_line = Vec::new();

        for chr in line.unwrap().chars() {
            let tile = 
                match chr {
                    ' ' => Tile::empty(),
                    '.' => Tile::short_wall(),
                    '#' => Tile::wall(),
                    'w' => Tile::water(),
                    'x' => Tile::exit(),
                    _ => panic!("Unexpected char in map!"),
                };
            rot_line.push(tile);
        }
        rot_map.push(rot_line)
    }

    for x in 0..MAP_WIDTH {
        let mut line = Vec::new();
        for y in 0..MAP_HEIGHT {
            line.push(rot_map[y as usize][x as usize]);
        }
        map.tiles.push(line);
    }

    return map;
}

pub fn write_map(file_name: &str, map: &Map) {
    // write out map to a file
    let mut map_file = File::create(file_name).unwrap();
    let mut map_vec = Vec::new();

    for row in map.tiles.iter() {
        let mut line_vec = Vec::new();

        for tile in row.iter() {
            let tile_char = match tile.tile_type {
                TileType::Empty => ' ',
                TileType::ShortWall => '.',
                TileType::Wall => '#',
                TileType::Water => 'w',
                TileType::Exit => 'x',
            };
            line_vec.push(tile_char as u8);
        }

        map_vec.push(line_vec);
    }

    let mut final_map_vec = Vec::new();
    println!("MAP_HEIGHT = {}", MAP_HEIGHT);
    println!("MAP_WIDTH = {}", MAP_WIDTH);
    println!("map_vec.len() = {}", map_vec.len());
    println!("map_vec[0].len() = {}", map_vec[0].len());
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            final_map_vec.push(map_vec[x as usize][y as usize]);
        }
        final_map_vec.push('\n' as u8);
    }
    map_file.write(&final_map_vec).unwrap();
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
    imgui_wrapper: ImGuiWrapper,
    font_image: Image,
    sprite_batch: SpriteBatch,
    input_action: InputAction,
}

impl GameState {
    fn new(mut ctx: &mut Context, args: &Vec<String>) -> GameResult<GameState> {
        let imgui_wrapper = ImGuiWrapper::new(&mut ctx);

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

        let mut inventory = vec![Object::make_stone(0, 0)];

        let config: Config;
        {
            let mut file = File::open("config.json").expect("Could not open/parse config file config.json");
            let mut config_string = String::new();
            file.read_to_string(&mut config_string).expect("Could not read contents of config.json");
            config = serde_json::from_str(&config_string).expect("Could not parse config.json file!");
        }

        let mut objects = vec!(make_player());

        let mut rng: SmallRng = SeedableRng::seed_from_u64(seed);

        let (map, position) = make_map(&mut objects, &config, &mut rng);
        let player_x = position.0;
        let player_y = position.1;
        objects[PLAYER].x = player_x;
        objects[PLAYER].y = player_y;

        // write out map to a file
        write_map("map.csv", &map);

        let mut game = Game::new();

        setup_fov(&mut game.fov, &map);
        let fov_distance = config.fov_distance;
        game.fov.compute_fov(player_x, player_y, fov_distance, FOV_LIGHT_WALLS, FOV_ALGO);

        messages.message("Welcome Stranger! Prepare to perish in the Desolation of Salt!", ORANGE);

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
        let fov_recompute = self.previous_player_position != (self.objects[PLAYER].x, self.objects[PLAYER].y);

        render_all(ctx,
                   &mut self.game,
                   &self.objects,
                   &mut self.map,
                   &mut self.messages,
                   &mut self.imgui_wrapper,
                   &mut self.sprite_batch,
                   fov_recompute,
                   &self.config)
    }

    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32, _dx: f32, _dy: f32) {
        self.imgui_wrapper.update_mouse_pos(x, y);
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
            button == MouseButton::Middle,
        ));
    }

    fn mouse_button_up_event(
        &mut self,
        _ctx: &mut Context,
        _button: MouseButton,
        _x: f32,
        _y: f32,
    ) {
        self.imgui_wrapper.update_mouse_down((false, false, false));
    }

    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        keycode: KeyCode,
        keymods: KeyMods,
        _repeat: bool,
    ) {
        match keycode {
            KeyCode::Key8 | KeyCode::Numpad8 | KeyCode::Up => {
                self.input_action = InputAction::Up;
            }

            KeyCode::Key8 | KeyCode::Numpad8 | KeyCode::Right => {
                self.input_action = InputAction::Right;
            }

            KeyCode::Key2 | KeyCode::Numpad2 | KeyCode::Down => {
                self.input_action = InputAction::Down;
            }

            KeyCode::Key4 | KeyCode::Numpad4 | KeyCode::Left => {
                self.input_action = InputAction::Left;
            }

            KeyCode::Key7 | KeyCode::Numpad7 => {
                self.input_action = InputAction::UpLeft;
            }

            KeyCode::Key9 | KeyCode::Numpad9 => {
                self.input_action = InputAction::UpRight;
            }

            KeyCode::Key3 | KeyCode::Numpad3 => {
                self.input_action = InputAction::DownRight;
            }

            KeyCode::Key1 | KeyCode::Numpad1 => {
                self.input_action = InputAction::DownLeft;
            }

            KeyCode::Key5 | KeyCode::Numpad5 => {
                self.input_action = InputAction::Center;
            }

            KeyCode::Return => {
                if keymods.contains(KeyMods::ALT) {
                    self.input_action = InputAction::FullScreen;
                }
            }

            KeyCode::G => {
                self.input_action = InputAction::Pickup;
            }

            KeyCode::I => {
                self.input_action = InputAction::Inventory;
            }

            KeyCode::V => {
                self.input_action = InputAction::ExploreAll;
            }

            KeyCode::Escape => {
                self.input_action = InputAction::Exit;
            }

            KeyCode::R => {
                self.input_action = InputAction::RegenerateMap;
            }

            KeyCode::Add => {
                self.input_action = InputAction::ToggleOverlays;
            }

            KeyCode::T => {
                self.input_action = InputAction::GodMode;
            }

            _ => {
                self.input_action = InputAction::None;
            }
        }
    }
}

