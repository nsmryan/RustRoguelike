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
// use ggez::graphics::spritebatch::SpriteBatch;
use ggez::graphics::Drawable;
use ggez::graphics::DrawParam;

use mint::Point2;

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

        let mut previous_player_position = (-1, -1);

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

        let input_action = InputAction::None;

        let state = GameState {
            game,
            config,
            previous_player_position,
            map,
            objects,
            messages,
            inventory,
            font_image,
            imgui_wrapper,
            input_action,
        };

        Ok(state)
    }
}

// TODO this could be done with a spritebatch, likely
// speeding this up.
// there may be some performance issues in debug, but there
// are currenly lots of performance issues in debug so it might
// still be better.
fn draw_char(ctx: &mut Context,
             font_image: &Image,
             chr: char,
             x: i32,
             y: i32,
             color: Color) {
    let chr_x = (chr as i32) % 16;
    let chr_y = (chr as i32) / 16;
    let draw_params =
        DrawParam {
            src: ggez::graphics::Rect {
                x: (chr_x as f32) / 16.0,
                y: (chr_y as f32) / 16.0,
                w: 1.0 / 16.0,
                h: 1.0 / 16.0,
            },
            dest: Point2 { x: x as f32 * 16.0, y: y as f32 * 16.0} ,
            rotation: 0.0,
            scale: mint::Vector2 { x: 1.0, y: 1.0 },
            offset: Point2 { x: 1.0, y: 1.0 },
            color: ggez::graphics::Color::new(color.r as f32 / 256.0,
                                              color.g as f32 / 256.0,
                                              color.b as f32 / 256.0,
                                              1.0),
        };

    font_image.draw(ctx, draw_params);
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
        let start_time = Instant::now();

        let fov_recompute = self.previous_player_position != (self.objects[PLAYER].x, self.objects[PLAYER].y);
        if fov_recompute {
            let player = &self.objects[PLAYER];
            let mut fov_distance = self.config.fov_distance;
            if self.game.god_mode {
                fov_distance = std::cmp::max(SCREEN_WIDTH, SCREEN_HEIGHT);
            }
            self.game.fov.compute_fov(player.x, player.y, fov_distance, FOV_LIGHT_WALLS, FOV_ALGO);
        }

        graphics::clear(ctx, graphics::BLACK);

        let (w, h) = graphics::drawable_size(ctx);
        let block_w = w / MAP_WIDTH as f32;
        let block_h = h / MAP_HEIGHT as f32;

        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                let chr;

                // Render game stuff
                let tile_type = self.map[(x, y)].tile_type;
                let visible = self.game.fov.is_in_fov(x, y);

                let mut color = match (self.map.tiles[x as usize][y as usize].tile_type, visible) {
                    (TileType::Wall, true) =>
                        self.config.color_light_brown.color(),
                    (TileType::Wall, false) =>
                        self.config.color_dark_brown.color(),

                    (TileType::Empty, true) =>
                        lerp(self.config.color_tile_blue_light.color(), self.config.color_tile_blue_dark.color(), rand_from_x_y(x, y)),
                    (TileType::Empty, false) =>
                        self.config.color_very_dark_blue.color(),

                    (TileType::Water, true) =>
                        self.config.color_blueish_grey.color(),
                    (TileType::Water, false) =>
                        self.config.color_dark_brown.color(),

                    (TileType::ShortWall, true) =>
                        self.config.color_light_brown.color(),
                    (TileType::ShortWall, false) =>
                        self.config.color_dark_brown.color(),

                    (TileType::Exit, true) =>
                        self.config.color_orange.color(),
                    (TileType::Exit, false) =>
                        self.config.color_red.color(),
                };
                //println!("color = {:?}", color);

                // TODO removed while working out rendering
                let mut explored = self.map.tiles[x as usize][y as usize].explored;
                if visible {
                    explored = true;
                }

                //if explored 

                match tile_type {
                    TileType::Empty => {
                        let has_bottom_wall = self.map.tiles[x as usize][y as usize].bottom_wall != Wall::Empty;
                        let has_left_wall = self.map.tiles[x as usize][y as usize].left_wall != Wall::Empty;

                        if  has_bottom_wall && has_left_wall {
                            // TODO this is a solid wall- there is no joint left/bottom wall tile
                            // yet
                            chr = '\u{DB}';
                        } else if has_left_wall {
                            chr = '\u{DD}';
                        } else if has_bottom_wall {
                            chr = '\u{DC}';
                        } else {
                            chr = '\u{AB}';
                        }

                        //console.put_char(x, y, chr, BackgroundFlag::None);
                        //console.set_char_background(x, y, color, BackgroundFlag::Set);
                    }

                    TileType::Water | TileType::Exit => {
                        //console.put_char(x, y, ' ', BackgroundFlag::None);
                        //console.set_char_background(x, y, color, BackgroundFlag::Set);
                        chr = '\u{AB}';
                    }

                    TileType::ShortWall | TileType::Wall => {
                        if visible {
                            //console.set_char_background(x, y, config.color_tile_blue_light.color(), BackgroundFlag::Set);
                            color = self.config.color_tile_blue_light.color();
                        } else {
                            //console.set_char_background(x, y, config.color_very_dark_blue.color(), BackgroundFlag::Set);
                            color = self.config.color_very_dark_blue.color();
                        }

                        let left = self.map[(x - 1, y)].tile_type == tile_type;
                        let right = self.map[(x + 1, y)].tile_type == tile_type;
                        let horiz = left || right;

                        let above = self.map[(x, y + 1)].tile_type == tile_type;
                        let below = self.map[(x, y - 1)].tile_type == tile_type;
                        let vert = above || below;

                        if tile_type == TileType::Wall {
                            if horiz && vert {
                               chr = '\u{DC}';
                            } else if horiz {
                               chr = '\u{EC}';
                            } else if vert {
                               chr = '\u{ED}';
                            } else {
                               chr = '\u{FE}';
                            }
                        } else {
                            if horiz && vert {
                               chr = tcod::chars::CROSS
                            } else if horiz {
                               chr = tcod::chars::HLINE;
                            } else if vert {
                               chr = tcod::chars::VLINE;
                            } else {
                               chr = tcod::chars::VLINE;
                            }
                        };
                    }
                }

                //if chr != ' ' {
                    // NOTE: this takes a good bit of time to create
                    let chr_text = graphics::Text::new(
                        format!("{}", chr)
                    );

                    draw_char(ctx, &self.font_image, chr, x, y, color);
                //}

                // TODO removed while working out rendering
                self.map.tiles[x as usize][y as usize].explored = explored;
            }
        }
        //dbg!(start_time.elapsed().as_millis());

        /* from render_objects */
        let mut to_draw: Vec<_> =
            self.objects.iter().filter(|o| {
                // TODO removed while testing rendering
                true // self.game.fov.is_in_fov(o.x, o.y)
            }).collect();
        to_draw.sort_by(|o1, o2| { o1.blocks.cmp(&o2.blocks) });

        for object in &to_draw {
            draw_char(ctx, &self.font_image, object.char, object.x, object.y, object.color);
        }

        // Render game ui
        // this takes around 70ms on last measurement
        self.imgui_wrapper.render(ctx);

        graphics::present(ctx)?;

        Ok(())
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

