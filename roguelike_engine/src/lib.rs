pub mod display;
pub mod input;
pub mod gui;
pub mod plat;
mod style;

/*
use ggez::{Context, GameResult};
use ggez::event::{self, EventsLoop, EventHandler, KeyCode, KeyMods, MouseButton};
use ggez::graphics::Image;

use roguelike_core::constants::*;

use crate::display::*;


trait Game {
    fn step(game: &mut Self);
    fn display(game: &mut Self);
    fn input(game: &mut Self, input_action: InputAction);
}

struct Engine<G> {
    display_state: DisplayState,
    context: Context,
    event_loop: EventsLoop,
    game: G,
}

impl<G> Engine<G> {
    pub fn new(game: G) -> Engine<G> {
        let mut window_mode: ggez::conf::WindowMode = Default::default();
        window_mode.width = ((SCREEN_WIDTH - 1) * FONT_WIDTH) as f32;
        window_mode.height = (SCREEN_HEIGHT * FONT_HEIGHT)  as f32;

        let cb = ggez::ContextBuilder::new("Roguelike", "like")
            .window_mode(window_mode);
        let (mut ctx, event_loop) = cb.build().unwrap();

        // TODO move to display in roguelike_engine
        let font_image = Image::new(&mut ctx, "/rexpaint16x16.png").unwrap();
        let display_state = DisplayState::new(font_image, &mut ctx);

        return Engine {
            context: ctx,
            display_state,
            event_loop,
            game,
        };
    }
}

impl EventHandler for Engine<G> where G: Game {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        self.game.step();

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        self.game.display();
    }

    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32, _dx: f32, _dy: f32) {
        self.mouse_state.pos = (x as i32, y as i32);
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        button: MouseButton,
        _x: f32,
        _y: f32,
        ) {
        self.mouse_state.pressed =
            (button == MouseButton::Left,
             button == MouseButton::Right,
             button == MouseButton::Middle);
    }

    fn mouse_button_up_event(
        &mut self,
        _ctx: &mut Context,
        _button: MouseButton,
        _x: f32,
        _y: f32,
        ) {
        self.mouse_state.pressed = (false, false, false);
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
*/
