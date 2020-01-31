use sdl2::keyboard::Keycode;
use sdl2::keyboard::Mod;

use tcod::line::*;

use roguelike_core::types::*;
use roguelike_core::constants::*;
use roguelike_core::movement::*;
use roguelike_core::animation::Animation;


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

pub fn pick_item_up(object_id: ObjectId,
                    item_id: ObjectId,
                    objects: &mut ObjMap) {
    objects[object_id].inventory.push(item_id);
    objects[item_id].set_xy(-1, -1);
}

pub fn throw_stone(start_pos: Pos,
                   end_pos: Pos,
                   stone_handle: ObjectId,
                   game_data: &mut GameData) {
    let throw_line = Line::new(start_pos.to_tuple(), end_pos.to_tuple());

    // TODO draw line to end, and move until radius or hit wall

    // get target position in direction of player click
    let (target_x, target_y) =
        throw_line.into_iter().take(PLAYER_THROW_DIST).last().unwrap();

    game_data.objects[stone_handle].set_xy(target_x, target_y);

    // Create the stone throw animation
    game_data.objects[stone_handle].animation =
        Some(Animation::StoneThrow(start_pos, Pos::new(target_x, target_y)));

    // alert monsters within sound range
    for obj in game_data.objects.values_mut() {
        if distance(obj.pos(), end_pos) < SOUND_RADIUS as i32 {
            if obj.behavior == Some(Behavior::Idle) {
                obj.behavior = Some(Behavior::Investigating(Pos::from(end_pos)));
            }
        }
    }
}

