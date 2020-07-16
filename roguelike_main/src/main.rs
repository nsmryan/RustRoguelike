#![allow(dead_code)]
mod throttler;
mod render;
mod console;
mod display;
mod plat;

use std::env;
use std::fs;
use std::time::{Duration, Instant};
use std::path::Path;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use sdl2::event::Event;
use sdl2::image::LoadTexture;
use sdl2::mouse::MouseButton;
use sdl2::keyboard::{Mod, Keycode};
use sdl2::render::{TextureCreator};
use sdl2::video::WindowContext;

use rand::prelude::*;

use walkdir::WalkDir;

use log::LevelFilter;
use simple_logging;
use logging_timer::timer;

use roguelike_core::types::*;
use roguelike_core::config::Config;
use roguelike_core::constants::*;
use roguelike_core::movement::Direction;
use roguelike_core::utils::{add_pos};

use roguelike_engine::game::*;
use roguelike_engine::generation::*;
use roguelike_engine::actions::*;
use roguelike_engine::make_map::{make_map, read_map_xp};

use crate::throttler::*;
use crate::render::*;
//use crate::console::*;
use crate::display::*;
use crate::plat::*;


const CONFIG_NAME: &str = "config.yaml";
const LOG_LEVEL: LevelFilter = LevelFilter::Trace;


fn main() {
    let args = env::args().collect::<Vec<String>>();

    let seed: u64;
    if args.len() > 1 {
        let mut hasher = DefaultHasher::new();
        args[1].hash(&mut hasher);
        seed = hasher.finish();
    } else {
        seed = rand::thread_rng().gen();
    }
    println!("Seed: {} (0x{:X})", seed, seed);

    simple_logging::log_to_file("game.log", LOG_LEVEL).unwrap();

    run(seed).unwrap();
}

pub fn run(seed: u64) -> Result<(), String> {
    let config = Config::from_file(CONFIG_NAME);
    let mut config_modified_time = fs::metadata(CONFIG_NAME).unwrap().modified().unwrap();

    /* Create SDL Context */
    let sdl_context = sdl2::init()?;
    let video = sdl_context.video()?;
    let window = video.window("Rust Roguelike", SCREEN_WIDTH, SCREEN_HEIGHT)
        .position_centered().build().map_err(|e| e.to_string())?;

    let canvas = window.into_canvas()
        .accelerated().build().map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();

    let mut event_pump = sdl_context.event_pump()?;

    /* Setup FPS Throttling */
    let fps_throttler = Throttler::new(Duration::from_millis(1000 / config.rate as u64));

    /* Create Display Structures */
    let screen_sections =
        Plan::vert("screen", 0.80, Plan::zone("map"),
                   Plan::split_horiz(0.3, Plan::zone("inventory"),
                                          Plan::split_horiz(0.5, Plan::zone("player"),
                                                                 Plan::zone("info"))));

    let font_image = texture_creator.load_texture("resources/rexpaint16x16.png")
        .expect("Could not load texture!");

    let mut display_state =
        DisplayState::new(screen_sections, font_image, canvas);

    /* Load Textures */
    load_sprites(&texture_creator, &mut display_state);

    let mut game = Game::new(seed, config.clone())?;

    make_map(&config.map_load, &mut game);

    let player_id = game.data.find_player().unwrap();
    let player_pos = game.data.entities.pos[&player_id];

    //make_mouse(&mut game.data.entities, &config, &mut game.msg_log);
    //make_hammer(&mut game.data.entities, &config, add_pos(player_pos, Pos::new(-1, 0)),  &mut game.msg_log);

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

        /* Step the Game Forward */
        let logic_timer = timer!("LOGIC");
        let dt = Instant::now().duration_since(frame_time);
        let game_result = game.step_game(dt.as_secs_f32());
        frame_time = Instant::now();
        drop(logic_timer);

        if game.settings.state == GameState::Win {
            dbg!("Won");
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
            if game_state == GameState::Inventory ||
               game_state == GameState::SkillMenu {
                input_action = InputAction::SelectItem(8);
            } else {
                input_action = InputAction::Move(Direction::Up);
            }
        }

        Keycode::Kp6 | Keycode::Num6 | Keycode::Right => {
            if game_state == GameState::Inventory ||
               game_state == GameState::SkillMenu {
                input_action = InputAction::SelectItem(6);
            } else {
                input_action = InputAction::Move(Direction::Right);
            }
        }

        Keycode::Kp2 | Keycode::Num2 | Keycode::Down => {
            if game_state == GameState::Inventory ||
               game_state == GameState::SkillMenu {
                input_action = InputAction::SelectItem(2);
            } else {
                input_action = InputAction::Move(Direction::Down);
            }
        }

        Keycode::Kp4 | Keycode::Num4 | Keycode::Left => {
            if game_state == GameState::Inventory ||
               game_state == GameState::SkillMenu {
                input_action = InputAction::SelectItem(4);
            } else {
                input_action = InputAction::Move(Direction::Left);
            }
        }

        Keycode::Kp7 | Keycode::Num7 => {
            if game_state == GameState::Inventory ||
               game_state == GameState::SkillMenu {
                input_action = InputAction::SelectItem(7);
            } else {
                input_action = InputAction::Move(Direction::UpLeft);
            }
        }

        Keycode::Kp9 | Keycode::Num9 => {
            if game_state == GameState::Inventory ||
               game_state == GameState::SkillMenu {
                input_action = InputAction::SelectItem(9);
            } else {
                input_action = InputAction::Move(Direction::UpRight);
            }
        }

        Keycode::Kp3 | Keycode::Num3 => {
            if game_state == GameState::Inventory ||
               game_state == GameState::SkillMenu {
                input_action = InputAction::SelectItem(3);
            } else {
                input_action = InputAction::Move(Direction::DownRight);
            }
        }

        Keycode::Kp1 | Keycode::Num1 => {
            if game_state == GameState::Inventory ||
               game_state == GameState::SkillMenu {
                input_action = InputAction::SelectItem(1);
            } else {
                input_action = InputAction::Move(Direction::DownLeft);
            }
        }

        Keycode::Kp5 | Keycode::Num5 | Keycode::Kp0 | Keycode::Num0 => {
            if game_state == GameState::Inventory ||
               game_state == GameState::SkillMenu {
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

fn load_sprites(texture_creator: &TextureCreator<WindowContext>,
                display_state: &mut DisplayState) {
    let player_idle = texture_creator.load_texture("animations/player/Player_Idle.png")
        .expect("Could not load texture!");

    let player_attack = texture_creator.load_texture("animations/player/player_attack.png")
        .expect("Could not load texture!");

    let player_vault = texture_creator.load_texture("animations/player/player_vault.png")
        .expect("Could not load texture!");

    let player_wall_kick = texture_creator.load_texture("animations/player/player_wallkick.png")
        .expect("Could not load texture!");

    let gol_idle = texture_creator.load_texture("animations/monster1/Gol_Idle.png")
        .expect("Could not load texture!");

    let gol_die = texture_creator.load_texture("animations/monster1/Gol_Die.png")
        .expect("Could not load texture!");

    let elf_idle = texture_creator.load_texture("animations/monster3/Elf_Idle.png")
        .expect("Could not load texture!");

    let spikes_anim = texture_creator.load_texture("animations/traps/DamageTrap.png")
        .expect("Could not load texture!");

    let font_as_sprite = texture_creator.load_texture("resources/rexpaint16x16.png")
        .expect("Could not load texture!");

    let mcmuffin = texture_creator.load_texture("animations/traps/McMuffin.png")
        .expect("Could not load texture!");

    display_state.add_sprite(SpriteSheet::new("player_wall_kick".to_string(), player_wall_kick, 1));
    display_state.add_sprite(SpriteSheet::new("player_idle".to_string(),      player_idle,      1));
    display_state.add_sprite(SpriteSheet::new("player_attack".to_string(),    player_attack,    1));
    display_state.add_sprite(SpriteSheet::new("player_vault".to_string(),     player_vault,     1));
    display_state.add_sprite(SpriteSheet::new("gol_idle".to_string(),         gol_idle,         1));
    display_state.add_sprite(SpriteSheet::new("gol_die".to_string(),          gol_die,          1));
    display_state.add_sprite(SpriteSheet::new("elf_idle".to_string(),         elf_idle,         1));
    display_state.add_sprite(SpriteSheet::new("spikes".to_string(),           spikes_anim,      1));
    display_state.add_sprite(SpriteSheet::new("font".to_string(),             font_as_sprite,   16));
    display_state.add_sprite(SpriteSheet::new("key".to_string(),              mcmuffin,         1));

    // load any animations in the autoload directory.
    for entry in WalkDir::new("animations/autoload/") {
        let entry = entry.unwrap();
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_file() && file_name.ends_with("png") {
                let sprite =
                    texture_creator.load_texture(path).expect("Could not load texture!");

                display_state.add_sprite(SpriteSheet::new(file_name, sprite, 1));
            }
        }
    }
}

