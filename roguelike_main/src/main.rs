#![allow(dead_code)]
mod throttler;
mod render;
mod console;
mod display;
mod plat;

use std::fs;
use std::io::{BufRead, Write};
use std::time::{Duration, Instant};
use std::path::Path;
use std::collections::HashMap;
//use std::collections::hash_map::DefaultHasher;
//use std::hash::{Hash, Hasher};
use std::str::FromStr;

use sdl2::event::Event;
use sdl2::image::LoadTexture;
use sdl2::mouse::MouseButton;
use sdl2::keyboard::{Mod, Keycode};
use sdl2::render::{TextureCreator};
use sdl2::video::WindowContext;
use sdl2::render::Texture;

use walkdir::WalkDir;
use bmp;

use log::LevelFilter;
use simple_logging;
use logging_timer::timer;

use gumdrop::Options;

use roguelike_core::types::*;
use roguelike_core::config::Config;
use roguelike_core::constants::*;
use roguelike_core::movement::Direction;

use roguelike_engine::game::*;
use roguelike_engine::generation::*;
use roguelike_engine::actions::*;
use roguelike_engine::make_map::{make_map, read_map_xp};

use crate::throttler::*;
use crate::render::*;
use crate::display::*;
use crate::plat::*;


const CONFIG_NAME: &str = "config.yaml";


#[derive(Debug, Clone, Options)]
pub struct GameOptions {
    #[options(help = "replay from an input log file")]
    pub replay: Option<String>,

    #[options(help = "log level to record in game.log (OFF, ERROR, WARN, INFO, DEBUG, TRACE)")]
    pub log_level: Option<String>,

    #[options(help = "use a given seed for random number generation")]
    pub seed: Option<u64>,

    #[options(help = "display help text")]
    pub help: bool,
}


fn main() {
    //let args = env::args().collect::<Vec<String>>();

    let opts = GameOptions::parse_args_default_or_exit();

    let seed: u64 =
        if let Some(given_seed) = opts.seed {
            given_seed
        } else {
            1
            // could add string input as a seed generator
            // let mut hasher = DefaultHasher::new();
            // args[1].hash(&mut hasher);
            // hasher.finish()
        };

    println!("Seed: {} (0x{:X})", seed, seed);

    let log_level =
        opts.log_level.clone().map_or(LevelFilter::Trace,
                                      |level_str| LevelFilter::from_str(&level_str).expect("Log level unexpected!"));
    simple_logging::log_to_file("game.log", log_level).unwrap();

    run(seed, opts).unwrap();
}

pub fn run(seed: u64, opts: GameOptions) -> Result<(), String> {
    let config = Config::from_file(CONFIG_NAME);

    /* Create SDL Context */
    let sdl_context = sdl2::init()?;
    let video = sdl_context.video()?;
    let window = video.window("Rust Roguelike", SCREEN_WIDTH, SCREEN_HEIGHT)
        .position_centered().build().map_err(|e| e.to_string())?;

    let canvas = window.into_canvas()
        .accelerated().build().map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();

    let ttf_context = sdl2::ttf::init().expect("Could not init SDL2 TTF!");
    let font_map = FontMap::new(&ttf_context, &texture_creator, "Monoid.ttf".to_string(), 16);

    /* Create Display Structures */
    let screen_sections =
        Plan::vert("screen", 0.80, Plan::zone("map"),
                   Plan::split_horiz(0.3, Plan::zone("inventory"),
                                          Plan::split_horiz(0.5, Plan::zone("player"),
                                                                 Plan::zone("info"))));

    let mut display_state =
        DisplayState::new(screen_sections, font_map, canvas);

    /* Load Textures */
    load_sprites(&texture_creator, &mut display_state);

    load_sprite(&texture_creator, &mut display_state, "resources/rexpaint16x16.png", "tiles", 16);

    let mut game = Game::new(seed, config.clone())?;

    make_mouse(&mut game.data.entities, &game.config, &mut game.msg_log);

    make_map(&config.map_load, &mut game);

    if game.config.take_screenshot {
        take_screenshot(&mut game, &mut display_state);
        return Ok(());
    }

    return game_loop(game, display_state, opts, sdl_context);
}

pub fn game_loop(mut game: Game, mut display_state: DisplayState, opts: GameOptions, sdl_context: sdl2::Sdl) -> Result<(), String> {
    // read in the recorded action log, if one is provided
    let mut starting_actions = Vec::new();
    if let Some(replay_file) = opts.replay {
        let file =
            std::fs::File::open(&replay_file).expect(&format!("Could not open replay file '{}'", &replay_file));
        for line in std::io::BufReader::new(file).lines() {
            if let Ok(action) = InputAction::from_str(&line.unwrap()) {
                starting_actions.push(action);
            }
        }
    }

    let mut config_modified_time = fs::metadata(CONFIG_NAME).unwrap().modified().unwrap();

    // reverse the input log so we can pop actions off start-to-end
    starting_actions.reverse();

    /* Action Log */
    let mut action_log = std::fs::File::create("action_log.txt").unwrap();

    /* Setup FPS Throttling */
    let fps_throttler = Throttler::new(Duration::from_millis(1000 / game.config.rate as u64));

    let mut event_pump = sdl_context.event_pump()?;

    let mut frame_time = Instant::now();

    /* Main Game Loop */
    let mut running = true;
    while running {
        let _loop_timer = timer!("GAME_LOOP");

        let input_timer = timer!("INPUT");
        /* Handle Events */
        //game.key_input.clear();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..}=> {
                    running = false;
                }

                Event::KeyDown {keycode, keymod, ..} => {
                    if let Some(keycode) = keycode {
                        //game.key_input.push((KeyDirection::Down, keycode));
                        game.input_action =
                            keydown_to_action(keycode, keymod);
                    }
                }

                Event::KeyUp {keycode, keymod, ..} => {
                    if let Some(keycode) = keycode {
                        //game.key_input.push((KeyDirection::Up, keycode));
                        game.input_action =
                            keyup_to_action(keycode, keymod, game.settings.state);
                    }
                }

                Event::MouseMotion {x, y, ..} => {
                    game.mouse_state.x = x;
                    game.mouse_state.y = y;
                }

                Event::MouseButtonDown {mouse_btn, x, y, ..} => {
                    match mouse_btn {
                        MouseButton::Left => {
                            game.mouse_state.left_pressed = true;

                            // Find the region where the mouse click occurred.
                            // If the click is within the map, generate a map click event.
                            let in_map =
                                display_state.zones.iter()
                                                   .filter(|zone| zone.contains(x as usize, y as usize) &&
                                                                  zone.name == "map")
                                                   .next();

                            if let Some(map_zone) = in_map {
                                let map_loc = map_zone.within(x as usize, y as usize);
                                let map_cell = (((map_loc.0 as f32 / map_zone.width as f32) * (game.data.map.width() as f32)) as i32,
                                                ((map_loc.1 as f32 / map_zone.height as f32) * (game.data.map.height() as f32)) as i32);
                                game.input_action =
                                  InputAction::MapClick(Pos::new(map_loc.0 as i32, map_loc.1 as i32),
                                                        Pos::new(map_cell.0 as i32, map_cell.1 as i32));
                            }
                        }

                        MouseButton::Middle => {
                            game.mouse_state.middle_pressed = true;
                        }

                        MouseButton::Right => {
                            game.mouse_state.right_pressed = true;
                        }

                        _ => {
                        },
                    }
                }

                Event::MouseButtonUp {mouse_btn, ..} => {
                    match mouse_btn {
                        MouseButton::Left => {
                            game.mouse_state.left_pressed = false;
                        }

                        MouseButton::Middle => {
                            game.mouse_state.middle_pressed = false;
                        }

                        MouseButton::Right => {
                            game.mouse_state.right_pressed = false;
                        }

                        _ => {},
                    }
                }

                _ => {}
            }
        }
        drop(input_timer);

        // if there are starting actions to read, pop one off to play
        if let Some(action) = starting_actions.pop() {
            game.input_action = action;
        }

        /* Record Inputs to Log File */
        if game.input_action != InputAction::None &&
           game.input_action != InputAction::Exit {
            action_log.write(game.input_action.to_string().as_bytes()).unwrap();
            action_log.write("\n".as_bytes()).unwrap();
        }

        /* Step the Game Forward */
        let logic_timer = timer!("LOGIC");
        let dt = Instant::now().duration_since(frame_time);
        let game_result = game.step_game(dt.as_secs_f32());
        frame_time = Instant::now();
        drop(logic_timer);

        if game.settings.state == GameState::Win {
            dbg!("Won");
            display_state.clear_level_state();
        } else if game_result == GameResult::Stop || game.settings.exiting {
            running = false;
        }

        let display_timer = timer!("DISPLAY");
        // TODO consider moving this within an update function for the display system
        for msg in game.msg_log.turn_messages.iter() {
            display_state.process_message(*msg, &mut game.data, &game.config);
        }

        /* Draw the Game to the Screen */
        render_all(&mut display_state, &mut game)?;

        //display_state.canvas.copy(&display_state.text_font,
        //                          Some(sdl2::rect::Rect::new('a' as i32 * 8 - 8, 0, 8, 18)),
        //                          Some(sdl2::rect::Rect::new(10, 10, 8, 18)));

        display_state.update_display();

        drop(display_timer);

        game.msg_log.clear();

        /* Reload map if configured to do so */
        let config_timer = timer!("CONFIG");
        if game.config.load_map_file_every_frame && Path::new("resources/map.xp").exists() {
            let player = game.data.find_player().unwrap();

            let map_file = format!("resources/{}", game.config.map_file);
            game.data.entities.clear();
            let player_pos = read_map_xp(&game.config, &mut game.data, &mut game.msg_log, &map_file);
            game.data.entities.set_pos(player, Pos::from(player_pos));
        }

        /* Reload Configuration */
        let current_config_modified_time = fs::metadata(CONFIG_NAME).unwrap().modified().unwrap();
        if current_config_modified_time != config_modified_time {
            config_modified_time = current_config_modified_time;
            game.config = Config::from_file(CONFIG_NAME);
        }
        drop(config_timer);

        /* Wait until the next tick to loop */
        let wait_timer = timer!("WAIT");
        fps_throttler.wait();
        drop(wait_timer);
    }

    return Ok(());
}

pub fn keyup_to_action(keycode: Keycode,
                       _keymods: Mod,
                       game_state: GameState) -> InputAction {
    let input_action: InputAction;

    match keycode {
        Keycode::Kp8 | Keycode::Num8 | Keycode::Up => {
            if game_state.is_menu() {
                input_action = InputAction::SelectItem(8);
            } else {
                input_action = InputAction::Move(Direction::Up);
            }
        }

        Keycode::Kp6 | Keycode::Num6 | Keycode::Right => {
            if game_state.is_menu() {
                input_action = InputAction::SelectItem(6);
            } else {
                input_action = InputAction::Move(Direction::Right);
            }
        }

        Keycode::Kp2 | Keycode::Num2 | Keycode::Down => {
            if game_state.is_menu() {
                input_action = InputAction::SelectItem(2);
            } else {
                input_action = InputAction::Move(Direction::Down);
            }
        }

        Keycode::Kp4 | Keycode::Num4 | Keycode::Left => {
            if game_state.is_menu() {
                input_action = InputAction::SelectItem(4);
            } else {
                input_action = InputAction::Move(Direction::Left);
            }
        }

        Keycode::Kp7 | Keycode::Num7 => {
            if game_state.is_menu() {
                input_action = InputAction::SelectItem(7);
            } else {
                input_action = InputAction::Move(Direction::UpLeft);
            }
        }

        Keycode::Kp9 | Keycode::Num9 => {
            if game_state.is_menu() {
                input_action = InputAction::SelectItem(9);
            } else {
                input_action = InputAction::Move(Direction::UpRight);
            }
        }

        Keycode::Kp3 | Keycode::Num3 => {
            if game_state.is_menu() {
                input_action = InputAction::SelectItem(3);
            } else {
                input_action = InputAction::Move(Direction::DownRight);
            }
        }

        Keycode::Kp1 | Keycode::Num1 => {
            if game_state.is_menu() {
                input_action = InputAction::SelectItem(1);
            } else {
                input_action = InputAction::Move(Direction::DownLeft);
            }
        }

        Keycode::Kp5 | Keycode::Num5 | Keycode::Kp0 | Keycode::Num0 => {
            if game_state.is_menu() {
                input_action = InputAction::SelectItem(0);
            } else {
                input_action = InputAction::Pass;
            }
        }

        Keycode::Return => {
            input_action = InputAction::None;
        }

        Keycode::A => {
            input_action = InputAction::Interact;
        }

        Keycode::Q => {
            input_action = InputAction::Exit;
        }

        Keycode::G => {
            input_action = InputAction::Pickup;
        }

        Keycode::D => {
            input_action = InputAction::DropItem;
        }

        Keycode::I => {
            input_action = InputAction::Inventory;
        }

        Keycode::Y => {
            input_action = InputAction::Yell;
        }

        Keycode::V => {
            input_action = InputAction::ExploreAll;
        }

        Keycode::Escape => {
            input_action = InputAction::Esc;
        }

        Keycode::Tab => {
            input_action = InputAction::SwapPrimaryItem;
        }

        Keycode::T => {
            input_action = InputAction::GodMode;
        }

        Keycode::X => {
            input_action = InputAction::IncreaseMoveMode;
        }

        Keycode::Z => {
            input_action = InputAction::DecreaseMoveMode;
        }

        Keycode::Space => {
            input_action = InputAction::OverlayOff;
        }

        Keycode::S => {
            input_action = InputAction::SkillMenu;
        }

        Keycode::C => {
            input_action = InputAction::ClassMenu;
        }

        Keycode::Backquote => {
            input_action = InputAction::ToggleConsole;
        }

        Keycode::U => {
            input_action = InputAction::UseItem;
        }

        _ => {
            input_action = InputAction::None;
        }
    }

    return input_action;
}

pub fn keydown_to_action(keycode: Keycode,
                         _keymods: Mod) -> InputAction {
    let input_action: InputAction;

    match keycode {
        Keycode::Space => {
            input_action = InputAction::OverlayOn;
        }

        _ => {
            input_action = InputAction::None;
        }
    }

    return input_action;
}

pub fn take_screenshot(game: &mut Game, display_state: &mut DisplayState) -> Result<(), String> {
    game.settings.god_mode = true;

    game.step_game(0.0);
    render_all(display_state, game)?;

    let pixels = display_state.canvas.read_pixels(None, sdl2::pixels::PixelFormatEnum::RGB24).unwrap();
    let (width, height) = display_state.canvas.output_size().unwrap();
    let mut image = bmp::Image::new(width, height);
    for index in 0..(width * height) {
        let byte_index = 3 * index as usize;
        let pixel = bmp::Pixel::new(pixels[byte_index], pixels[byte_index + 1], pixels[byte_index + 2]);
        image.set_pixel(index % width, index / width, pixel);
    }
    image.save("screenshot.bmp");

    return Ok(());
}

fn load_sprites(texture_creator: &TextureCreator<WindowContext>, display_state: &mut DisplayState) {
    load_sprite(texture_creator, display_state, "animations/player/Player_Idle.png", "player_idle", 1);
    load_sprite(texture_creator, display_state, "animations/player/player_attack.png", "player_attack", 1);
    load_sprite(texture_creator, display_state, "animations/player/player_attack_Hammer.png", "player_attack_hammer", 1);
    load_sprite(texture_creator, display_state, "animations/player/player_attack_Dagger.png", "player_attack_dagger", 1);
    load_sprite(texture_creator, display_state, "animations/player/Player_Idle_Dagger.png", "player_idle_dagger", 1);
    load_sprite(texture_creator, display_state, "animations/player/Player_Idle_Hammer.png", "player_idle_hammer", 1);
    load_sprite(texture_creator, display_state, "animations/player/Player_Idle_Shield.png", "player_idle_shield", 1);
    load_sprite(texture_creator, display_state, "animations/player/player_vault.png", "player_vault", 1);
    load_sprite(texture_creator, display_state, "animations/player/player_wallkick.png", "player_wall_kick", 1);
    load_sprite(texture_creator, display_state, "animations/monster1/Gol_Idle.png", "gol_idle", 1);
    load_sprite(texture_creator, display_state, "animations/monster1/Gol_Die.png", "gol_die", 1);
    load_sprite(texture_creator, display_state, "animations/monster3/Elf_Idle.png", "elf_idle", 1);
    load_sprite(texture_creator, display_state, "animations/traps/DamageTrap.png", "spikes", 1);
    load_sprite(texture_creator, display_state, "resources/rexpaint16x16.png", "font", 16);
    load_sprite(texture_creator, display_state, "animations/traps/McMuffin.png", "key", 1);

    for entry in WalkDir::new("animations/autoload/") {
        let entry = entry.unwrap();
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_file() && file_name.ends_with("png") {
                load_sprite(texture_creator, display_state, path.to_str().unwrap(), &file_name, 1);
            }
        }
    }
}

fn load_sprite(texture_creator: &TextureCreator<WindowContext>,
               display_state: &mut DisplayState,
               path: &str,
               sprite_name: &str,
               rows: usize) {
    let texture = texture_creator.load_texture(path).expect("Could not load texture!");
    display_state.add_spritesheet(sprite_name.to_string(), texture, rows);
}

