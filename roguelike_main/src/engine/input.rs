use ggez::event::{KeyCode, KeyMods};

use roguelike_core::types::*;


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputAction {
    Move(MoveAction),
    Pickup,
    Inventory,
    Exit,
    ExploreAll,
    RegenerateMap,
    ToggleOverlays,
    GodMode,
    FullScreen,
    None,
}

pub fn map_keycode_to_action(keycode: KeyCode, keymods: KeyMods) -> InputAction {
    let input_action: InputAction;

    match keycode {
        KeyCode::Key8 | KeyCode::Numpad8 | KeyCode::Up => {
            input_action = InputAction::Move(MoveAction::Up);
        }

        KeyCode::Key6 | KeyCode::Numpad6 | KeyCode::Right => {
            input_action = InputAction::Move(MoveAction::Right);
        }

        KeyCode::Key2 | KeyCode::Numpad2 | KeyCode::Down => {
            input_action = InputAction::Move(MoveAction::Down);
        }

        KeyCode::Key4 | KeyCode::Numpad4 | KeyCode::Left => {
            input_action = InputAction::Move(MoveAction::Left);
        }

        KeyCode::Key7 | KeyCode::Numpad7 => {
            input_action = InputAction::Move(MoveAction::UpLeft);
        }

        KeyCode::Key9 | KeyCode::Numpad9 => {
            input_action = InputAction::Move(MoveAction::UpRight);
        }

        KeyCode::Key3 | KeyCode::Numpad3 => {
            input_action = InputAction::Move(MoveAction::DownRight);
        }

        KeyCode::Key1 | KeyCode::Numpad1 => {
            input_action = InputAction::Move(MoveAction::DownLeft);
        }

        KeyCode::Key5 | KeyCode::Numpad5 => {
            input_action = InputAction::Move(MoveAction::Center);
        }

        KeyCode::Return => {
            if keymods.contains(KeyMods::ALT) {
                input_action = InputAction::FullScreen;
            } else {
                input_action = InputAction::None;
            }
        }

        KeyCode::G => {
            input_action = InputAction::Pickup;
        }

        KeyCode::I => {
            input_action = InputAction::Inventory;
        }

        KeyCode::V => {
            input_action = InputAction::ExploreAll;
        }

        KeyCode::Escape => {
            input_action = InputAction::Exit;
        }

        KeyCode::R => {
            input_action = InputAction::RegenerateMap;
        }

        KeyCode::Add => {
            input_action = InputAction::ToggleOverlays;
        }

        KeyCode::T => {
            input_action = InputAction::GodMode;
        }

        _ => {
            input_action = InputAction::None;
        }
    }

    return input_action;
}

