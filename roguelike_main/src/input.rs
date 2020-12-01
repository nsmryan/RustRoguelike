use sdl2::event::Event;
use sdl2::mouse::MouseButton;
use sdl2::keyboard::{Mod, Keycode, Scancode};

use roguelike_core::types::*;
use roguelike_core::movement::Direction;

use roguelike_engine::game::*;
use roguelike_engine::actions::*;

use crate::display::*;


pub fn handle_sdl2_input(game: &mut Game, display: &mut Display, scancodes: Vec<Scancode>, event_pump: &mut sdl2::EventPump) {
    for event in event_pump.poll_iter() {
        match event {
            Event::Quit {..}=> {
                game.settings.running = false;
            }

            Event::KeyDown {keycode, keymod, ..} => {
                if let Some(keycode) = keycode {
                    game.input_action =
                        keydown_to_action(keycode, keymod);
                }
            }

            Event::KeyUp {keycode, keymod, ..} => {
                if let Some(keycode) = keycode {
                    game.input_action =
                        keyup_to_action(keycode, keymod, game.settings.state);

                    game.input_action = handle_chord(game.input_action, &scancodes);
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

pub fn handle_chord(input_action: InputAction, scancodes: &Vec<Scancode>) -> InputAction {
    let mut action = input_action;

    let mut is_chord: bool = false;
    let mut strength: ActionStrength = ActionStrength::Weak;
    let mut mode: ActionMode = ActionMode::Primary;
    let mut target = -1;

    if scancodes.iter().any(|s| *s == Scancode::LShift) ||
       scancodes.iter().any(|s| *s == Scancode::RShift) {
           is_chord = true;
           strength = ActionStrength::Strong;
    }

    if scancodes.iter().any(|s| *s == Scancode::LAlt) ||
       scancodes.iter().any(|s| *s == Scancode::RAlt) {
           is_chord = true;
           mode = ActionMode::Alternate;
    }

    let target_codes = &[Scancode::Z, Scancode::X, Scancode::C, Scancode::V, Scancode::B];
    for (index, code) in target_codes.iter().enumerate() {
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
        dbg!(action);
    }

    return action;
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

        Keycode::P => {
            input_action = InputAction::RegenerateMap;
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

