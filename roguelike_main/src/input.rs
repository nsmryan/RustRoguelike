use sdl2::event::Event;
use sdl2::mouse::MouseButton;
use sdl2::keyboard::{Keycode, Scancode, Mod};

use roguelike_core::types::*;
use roguelike_core::movement::Direction;
use roguelike_core::config::Config;

use roguelike_engine::game::*;
use roguelike_engine::actions::*;

use crate::display::*;


const TARGET_CODES: &[Scancode] = &[Scancode::Z, Scancode::X, Scancode::C, Scancode::V, Scancode::B];
const MODIFIERS: &[Scancode] = &[Scancode::LAlt, Scancode::RAlt, Scancode::RCtrl, Scancode::LCtrl];


#[derive(Clone, Debug)]
pub struct Input {
    chording: bool,
    chorded: bool,
    mode: ActionMode,
    target: i32,
}

impl Input {
    pub fn new() -> Input {
        return Input { chording: false, chorded: false, mode: ActionMode::Primary, target: 0 };
    }

    // TODO return an Option<InputAction> instead of modifying the game.
    // possibly make game & instead of &mut
    // try to reduce inputs and dependencies here
    pub fn handle_sdl2_input(&mut self,
                             game: &mut Game,
                             display: &mut Display,
                             scancodes: &Vec<Scancode>,
                             event_pump: &mut sdl2::EventPump) {
        let current_events = event_pump.poll_iter().collect::<Vec<Event>>();
        for event in current_events {
            match event {
                Event::Quit {..}=> {
                    game.settings.running = false;
                }

                Event::KeyDown {keycode, keymod, repeat, ..} => {
                    if repeat {
                        continue;
                    }

                    let chord_down = keymod.contains(Mod::RCTRLMOD) | keymod.contains(Mod::LCTRLMOD);
                    if chord_down {
                        if !self.chording {
                            self.chording = true;
                            self.chorded = true;
                            self.mode = ActionMode::Primary;
                        }
                    } else {
                        if let Some(keycode) = keycode {
                            game.input_action =
                                keydown_to_action(keycode, keymod);
                        }
                    }
                }

                Event::KeyUp {keycode, keymod, repeat, ..} => {
                    if repeat {
                        continue;
                    }

                    if !keymod.contains(Mod::RCTRLMOD) & !keymod.contains(Mod::LCTRLMOD) {
                        self.chording = false;
                    }

                    if self.chording {
                        if keymod.contains(Mod::RALTMOD) | keymod.contains(Mod::LALTMOD) {
                            self.mode = ActionMode::Alternate;
                        }

                        for (index, code) in TARGET_CODES.iter().enumerate() {
                            if scancodes.iter().any(|s| *s == *code) {
                                   self.target = index as i32;
                            }
                        }
                    }

                    if let Some(keycode) = keycode {
                        game.input_action =
                            keyup_to_action(keycode, keymod, scancodes, game.settings.state);

                        if self.chorded {
                            let strength = ActionStrength::Weak;
                            // TODO this should also catch the situation where you don't need a
                            // move in order to carry out the chord, such as certain skills.
                            match game.input_action {
                                InputAction::Move(dir) => {
                                    game.input_action = InputAction::Chord(Some(dir), strength, self.mode, self.target);
                                }

                                InputAction::Pass => {
                                    game.input_action = InputAction::Chord(None, strength, self.mode, self.target);
                                }

                                _ => {}
                            }
                        } else if game.config.use_cursor {
                           if let InputAction::Move(dir) = game.input_action {
                                game.input_action = InputAction::CursorMove(dir);
                           }
                        }
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

                            let (map_width, map_height) = game.data.map.size();
                            if let Some(mouse_cell) = display.targets.mouse_pos(x, y, map_width, map_height) {
                                let screen_pos = Pos::new(x, y);
                                let mouse_pos = Pos::new(mouse_cell.0, mouse_cell.1);
                                game.input_action = InputAction::MapClick(screen_pos, mouse_pos);
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
    }
}


pub fn handle_cursor(input_action: InputAction, scancodes: &Vec<Scancode>, config: &Config) -> InputAction {
    let mut action = input_action;

    if let InputAction::Move(dir) = input_action {
        action = InputAction::CursorMove(dir);
    }

    return action;
}

pub fn handle_chord(input_action: InputAction, scancodes: &Vec<Scancode>) -> InputAction {
    let mut action = input_action;

    let mut is_chord: bool = false;
    let mut strength: ActionStrength = ActionStrength::Weak;
    let mut mode: ActionMode = ActionMode::Primary;
    let mut target = -1;

    if scancodes.iter().any(|s| *s == Scancode::LCtrl) ||
       scancodes.iter().any(|s| *s == Scancode::RCtrl) {
           is_chord = true;
    }

    if scancodes.iter().any(|s| *s == Scancode::LAlt) ||
       scancodes.iter().any(|s| *s == Scancode::RAlt) {
           is_chord = true;
           mode = ActionMode::Alternate;
    }

    for (index, code) in TARGET_CODES.iter().enumerate() {
        if scancodes.iter().any(|s| *s == *code) {
               target = index as i32;
        }
    }

    if is_chord {
        let direction = match input_action {
            InputAction::Move(dir) => Some(dir),
            _ => None,
        };
        action = InputAction::Chord(direction, strength, mode, target);
    }

    return action;
}

pub fn keyup_to_action(keycode: Keycode,
                       keymods: Mod,
                       scancodes: &Vec<Scancode>,
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

        Keycode::P => {
            input_action = InputAction::RegenerateMap;
        }

        Keycode::X => {
            input_action = InputAction::IncreaseMoveMode;
        }

        Keycode::Z => {
            input_action = InputAction::DecreaseMoveMode;
        }

        Keycode::O => {
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

        Keycode::Space => {
            let mut mode = ActionMode::Primary;
            if keymods.contains(Mod::LALTMOD) || keymods.contains(Mod::RALTMOD) {
                mode = ActionMode::Alternate;
            }

            let mut target = -1;
            for (index, code) in TARGET_CODES.iter().enumerate() {
                if scancodes.iter().any(|s| *s == *code) {
                       target = index as i32;
                }
            }
            input_action = InputAction::CursorApply(mode, target);
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
        Keycode::O => {
            input_action = InputAction::OverlayOn;
        }

        _ => {
            input_action = InputAction::None;
        }
    }

    return input_action;
}

