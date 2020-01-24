use rand::prelude::*;

use sdl2::keyboard::Keycode;
use sdl2::keyboard::Mod;

use roguelike_core::types::*;
use roguelike_core::constants::*;
use roguelike_core::movement::*;

use crate::generation;


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

pub fn handle_input(input_action: InputAction,
                    _mouse_state: &MouseState,
                    game_data: &mut GameData, 
                    settings: &mut GameSettings,
                    display_state: &mut DisplayState,
                    config: &Config) -> Action {
    use Action::*;
    use Movement::*;

    let player_handle = game_data.find_player().unwrap();
    let player_pos = game_data.objects[player_handle].pos();

    let mut player_turn: Action = Action::none();

    let player_alive = game_data.objects[player_handle].alive;

    match (input_action, player_alive) {
        (InputAction::Move(move_action), true) => {
            let player_handle = game_data.find_player().unwrap();

            let player_reach = game_data.objects[player_handle].movement.unwrap();
            let maybe_movement = 
                movement::calculate_move(move_action,
                                         player_reach,
                                         player_handle,
                                         game_data);


            if let Some(movement) = maybe_movement {
                player_turn = Action::Move(movement);
            }
        }

        (InputAction::Pickup, true) => {
            let player = &game_data.objects[player_handle];
            let item_id = game_data.objects.keys().filter(|key| {
                return (game_data.objects[*key].pos() == player.pos()) && game_data.objects[*key].item.is_some();
            }).next();
            if let Some(key) = item_id {
                player_turn = Action::Pickup(key);
            }
        }

        (InputAction::MapClick(_map_loc, map_cell), _) => {
            let mut stone = None;
            let mut stone_index = None;
            for (index, obj_id) in game_data.objects[player_handle].inventory.iter().enumerate() {
                if let Some(Item::Stone) = game_data.objects[*obj_id].item {
                    stone = Some(obj_id);
                    stone_index = Some(index);
                    break;
                }
            }

            if let (Some(stone_handle), Some(index)) = (stone, stone_index) {
                player_turn = Action::ThrowStone(map_cell, *stone_handle);
                game_data.objects[player_handle].inventory.remove(index);
            }
        }

        (InputAction::Inventory, true) => {
        }

        (InputAction::Exit, _) => {
            settings.exiting = true;
        }

        (InputAction::ExploreAll, _) => {
            for x in 0..game_data.map.width() {
                for y in 0..game_data.map.height() {
                    game_data.map.tiles[x as usize][y as usize].explored = true;
                }
            }
        }

        (InputAction::RegenerateMap, _) => {
            let mut rng: SmallRng = SeedableRng::seed_from_u64(2);
            let (data, _position) =
                generation::make_map(&settings.map_type, &mut game_data.objects, config, display_state, &mut rng);
            game_data.map = data.map;
        }

        (InputAction::ToggleOverlays, _) => {
            display_state.display_overlays = !(display_state.display_overlays);

        }

        (InputAction::GodMode, true) => {
            let god_mode_hp = 1000000;
            let handle = game_data.find_player().unwrap();
            if let Some(ref mut fighter) = game_data.objects[handle].fighter {
                fighter.hp = god_mode_hp;
                fighter.max_hp = god_mode_hp;
            }

            // set god mode flag
            settings.god_mode = true;

            // set all tiles to be transparent and walkable. walkable is not current used
            // anywhere
            for x in 0..game_data.map.tiles.len() {
                for y in 0..game_data.map.tiles[0].len() {
                    game_data.map.set_cell(x as i32, y as i32, true, true);
                }
            }
            game_data.map.update_map();
        }

        (_, _) => {
        }
    }

    return player_turn;
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
        if distance(obj.pos(), end_pos) <  SOUND_RADIUS as i32 {
            if obj.behavior == Some(Behavior::Idle) {
                obj.behavior = Some(Behavior::Investigating(Pos::from(end_pos)));
            }
        }
    }
}

