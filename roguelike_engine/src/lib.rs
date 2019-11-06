pub mod game;
pub mod display;
pub mod input;
pub mod plat;
pub mod read_map;
pub mod actions;

use sdl2::event::Event;
use sdl2::image::LoadTexture;
use sdl2::rect::{Rect, Point};

use roguelike_core::config::*;
use roguelike_core::constants::*;
use roguelike_core::types::*;

use crate::display::*;
use crate::plat::*;
use crate::game::*;
use crate::input::*;


pub fn run(args: &Vec<String>, config: Config) -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video = sdl_context.video()?;
    let window = video.window("Rust Roguelike", 800, 600)
        .position_centered().build().map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas()
        .accelerated().build().map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();

    let mut event_pump = sdl_context.event_pump()?;

    let font_image = texture_creator.load_texture("rexpaint16x16.png")
        .map_err(|e| e.to_string())?;

    let screen_sections =
        Plan::vert("screen", 0.80, Plan::zone("map"), Plan::zone("inspector"));

    let display_state = DisplayState::new(screen_sections, font_image, canvas);

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

                _ => {}
            }
        }

        game.step_game();

        game.display_state.canvas.set_draw_color(sdl2::pixels::Color::RGBA(0, 0, 0, 255));
        game.display_state.canvas.clear();

        game.display_state.canvas.set_draw_color(sdl2::pixels::Color::RGBA(120, 40, 0, 255));

        game.display_state.font_image.set_color_mod(120, 40, 0);

        render_all(&mut game.display_state,
                   &mut game.mouse_state,
                   &game.data.objects,
                   &mut game.data.map,
                   &game.config);

        game.display_state.canvas.present();
    }

    return Ok(());
}

