pub mod game;
pub mod display;
pub mod input;
pub mod gui;
pub mod plat;
pub mod read_map;
pub mod actions;
mod style;

use std::boxed::Box;

use ggez::event::{self, EventsLoop};
use ggez::{Context, GameResult};
use ggez::graphics::Image;

use roguelike_core::constants::*;
use roguelike_core::config::*;

use crate::display::*;
use crate::plat::*;
use crate::input::*;
use crate::game::*;


pub struct Engine {
    pub game: Game,
    event_loop: EventsLoop,
    ctx: Context,
}

impl Engine {
    pub fn new(args: &Vec<String>, config: Config) -> GameResult<Engine> {
        let mut window_mode: ggez::conf::WindowMode = Default::default();
        window_mode.width = ((SCREEN_WIDTH - 1) * FONT_WIDTH) as f32;
        window_mode.height = (SCREEN_HEIGHT * FONT_HEIGHT)  as f32;

        let cb = ggez::ContextBuilder::new("Roguelike", "like")
            .window_mode(window_mode);
        let (mut ctx, event_loop) = cb.build().unwrap();

        let font_image = Image::new(&mut ctx, "/rexpaint16x16.png").unwrap();
        let screen_sections =
            Plan::vert("screen", 0.80, Plan::zone("map"), Plan::zone("inspector"));

        let display_state = DisplayState::new(screen_sections, font_image, &mut ctx);

        let game = Game::new(args, config, display_state)?;

        return Ok(Engine { game,
                           event_loop,
                           ctx,
        });
    }

    pub fn run(&mut self) {
        event::run(&mut self.ctx, &mut self.event_loop, &mut self.game).unwrap();
    }
}

