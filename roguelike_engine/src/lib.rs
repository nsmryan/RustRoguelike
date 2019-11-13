pub mod game;
pub mod display;
pub mod input;
pub mod plat;
pub mod read_map;
pub mod actions;

use sdl2::event::Event;
use sdl2::image::LoadTexture;
use sdl2::mouse::MouseButton;

use roguelike_core::config::*;
use roguelike_core::constants::*;

use crate::display::*;
use crate::plat::*;
use crate::game::*;
use crate::input::*;


pub fn run(args: &Vec<String>, config: Config) -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video = sdl_context.video()?;
    let window = video.window("Rust Roguelike", SCREEN_WIDTH, SCREEN_HEIGHT)
        .position_centered().build().map_err(|e| e.to_string())?;

    let canvas = window.into_canvas()
        .accelerated().build().map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();

    let mut event_pump = sdl_context.event_pump()?;

    let font_image = texture_creator.load_texture("rexpaint16x16.png")
        .map_err(|e| e.to_string())?;

    let screen_sections =
        Plan::vert("screen", 0.80, Plan::zone("map"), Plan::zone("inspector"));

    let display_state =
        DisplayState::new(screen_sections, font_image, canvas);

    let mut game = Game::new(args, config, display_state)?;

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

                Event::MouseButtonDown{mouse_btn, ..} => {
                    match mouse_btn {
                        MouseButton::Left => game.mouse_state.left_pressed = true,
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
    }

    return Ok(());
}

