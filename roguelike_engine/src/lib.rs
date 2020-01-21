pub mod game;
pub mod display;
pub mod input;
pub mod plat;
pub mod read_map;
pub mod actions;
pub mod generation;
pub mod render;
mod throttler;

use std::time::Duration;

use sdl2::event::Event;
use sdl2::image::LoadTexture;
use sdl2::mouse::MouseButton;

use slotmap::dense::*;

use roguelike_core::types::*;
use roguelike_core::config::*;
use roguelike_core::constants::*;

use crate::display::*;
use crate::render::*;
use crate::plat::*;
use crate::game::*;
use crate::input::*;
use crate::throttler::*;


pub fn run(args: &Vec<String>, config: Config) -> Result<(), String> {
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
    let fps_throttler = Throttler::new(Duration::from_millis(1000 / 30));

    /* Load Textures */
    let font_image = texture_creator.load_texture("resources/rexpaint16x16.png")
        .map_err(|e| e.to_string())?;

    let player_idle = texture_creator.load_texture("animations/player/Player_Idle.png")
        .map_err(|e| e.to_string())?;

    let player_wall_kick = texture_creator.load_texture("animations/player/player_wallkick.png")
        .map_err(|e| e.to_string())?;

    let gol_idle = texture_creator.load_texture("animations/monster1/Gol_Idle.png")
        .map_err(|e| e.to_string())?;

    let elf_idle = texture_creator.load_texture("animations/monster3/Elf_Idle.png")
        .map_err(|e| e.to_string())?;

    let mut sprites = DenseSlotMap::new();
    sprites.insert(SpriteSheet::new("player_wall_kick".to_string(), player_wall_kick));
    sprites.insert(SpriteSheet::new("player_idle".to_string(), player_idle));
    sprites.insert(SpriteSheet::new("gol_idle".to_string(), gol_idle));
    sprites.insert(SpriteSheet::new("elf_idle".to_string(), elf_idle));

    /* Create Display Structures */
    let screen_sections =
        Plan::vert("screen", 0.80, Plan::zone("map"), 
                   Plan::split_horiz(0.3, Plan::zone("inventory"),
                                          Plan::split_horiz(0.5, Plan::zone("player"),
                                                                 Plan::zone("info"))));


    let display_state =
        DisplayState::new(screen_sections, font_image, sprites, canvas);

    let mut game = Game::new(args, config, display_state)?;

    /* Main Game Loop */
    let mut running = true;
    while running {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit{ .. }=> {
                    running = false;
                }

                Event::KeyDown{keycode, keymod, ..} => {
                    if let Some(keycode) = keycode {
                        game.input_action =
                            map_keycode_to_action(keycode, keymod);
                    }
                }

                Event::MouseMotion{x, y, ..} => {
                    game.mouse_state.x = x;
                    game.mouse_state.y = y;
                }

                Event::MouseButtonDown{mouse_btn, x, y, ..} => {
                    match mouse_btn {
                        MouseButton::Left => {
                            game.mouse_state.left_pressed = true;

                            let in_map =
                                game.display_state.zones.iter()
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

                        MouseButton::Middle => game.mouse_state.middle_pressed = true,

                        MouseButton::Right => game.mouse_state.right_pressed = true,
                        _ => {},
                    }
                }

                Event::MouseButtonUp{mouse_btn, ..} => {
                    match mouse_btn {
                        MouseButton::Left => game.mouse_state.left_pressed = false,
                        MouseButton::Middle => game.mouse_state.middle_pressed = false,
                        MouseButton::Right => game.mouse_state.right_pressed = false,
                        _ => {},
                    }
                }

                _ => {}
            }
        }

        let exit_game = game.step_game();
        if exit_game {
            break;
        }

        render_all(&mut game.display_state,
                   &mut game.mouse_state,
                   &mut game.data,
                   &game.settings,
                   &game.config)?;

        fps_throttler.wait();
    }

    return Ok(());
}

