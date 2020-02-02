use sdl2::keyboard::Keycode;
use sdl2::keyboard::Mod;

use roguelike_core::types::*;
use roguelike_core::movement::MoveAction;


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputAction {
    Move(MoveAction),
    MapClick(Pos, Pos),
    Pickup,
    Inventory,
    Exit,
    ExploreAll,
    RegenerateMap,
    ToggleOverlays,
    GodMode,
    RegenMap,
    Yell,
    None,
}

// TODO input modes where keys are mapped to certain actions.
//      also effects rendering for the line
pub fn map_keycode_to_action(keycode: Keycode, keymods: Mod) -> InputAction {
    let input_action: InputAction;

    match keycode {
        Keycode::Kp8 | Keycode::Num8 | Keycode::Up => {
            input_action = InputAction::Move(MoveAction::Up);
        }

        Keycode::Kp6 | Keycode::Num6 | Keycode::Right => {
            input_action = InputAction::Move(MoveAction::Right);
        }

        Keycode::Kp2 | Keycode::Num2 | Keycode::Down => {
            input_action = InputAction::Move(MoveAction::Down);
        }

        Keycode::Kp4 | Keycode::Num4 | Keycode::Left => {
            input_action = InputAction::Move(MoveAction::Left);
        }

        Keycode::Kp7 | Keycode::Num7 => {
            input_action = InputAction::Move(MoveAction::UpLeft);
        }

        Keycode::Kp9 | Keycode::Num9 => {
            input_action = InputAction::Move(MoveAction::UpRight);
        }

        Keycode::Kp3 | Keycode::Num3 => {
            input_action = InputAction::Move(MoveAction::DownRight);
        }

        Keycode::Kp1 | Keycode::Num1 => {
            input_action = InputAction::Move(MoveAction::DownLeft);
        }

        Keycode::Kp5 | Keycode::Num5 => {
            input_action = InputAction::Move(MoveAction::Center);
        }

        Keycode::Return => {
            input_action = InputAction::None;
        }

        Keycode::Q => {
            input_action = InputAction::Exit;
        }

        Keycode::G => {
            input_action = InputAction::Pickup;
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
            input_action = InputAction::Exit;
        }

        Keycode::R => {
            input_action = InputAction::RegenerateMap;
        }

        Keycode::Plus => {
            input_action = InputAction::ToggleOverlays;
        }

        Keycode::T => {
            input_action = InputAction::GodMode;
        }

        _ => {
            input_action = InputAction::None;
        }
    }

    return input_action;
}

