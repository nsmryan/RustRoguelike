use rand::prelude::*;

use tcod::line::*;

use roguelike_core::config::*;
use roguelike_core::movement::Action;
use roguelike_core::types::*;
use roguelike_core::movement;
use roguelike_core::utils::reach_by_mode;
use roguelike_core::messaging::{Msg, MsgLog};
use roguelike_core::constants::*;
use roguelike_core::map::{Aoe, AoeEffect};

use crate::game::*;
use crate::display::*;
use crate::input::*;
use crate::generation;


pub fn player_apply_action(action: Action, game_data: &mut GameData, msg_log: &mut MsgLog) {
    let player_handle = game_data.find_player().unwrap();
    let player_pos = game_data.objects[player_handle].pos();

    match action {
        Action::Move(movement) => {
            movement::player_move_or_attack(movement, game_data, msg_log);
        }

        Action::StateChange(_behavior) => {
            panic!("Player tried to change behavior?");
        }

        Action::Pickup(item_id) => {
            pick_item_up(player_handle, item_id, &mut game_data.objects);
            msg_log.log(Msg::PickedUp(player_handle, item_id));
        }

        Action::ThrowStone(throw_pos, stone_index) => {
            throw_stone(player_handle, stone_index, player_pos, throw_pos, game_data, msg_log);
        }

        Action::Yell => {
            msg_log.log(Msg::Yell(player_pos));
        }

        Action::NoAction => {
        }
    }
}

pub fn handle_input_inventory(input: InputAction,
                              game_data: &mut GameData,
                              settings: &mut GameSettings,
                              msg_log: &mut MsgLog) {
    let player_handle = game_data.find_player().unwrap();

    match input {
        InputAction::Inventory => {
            settings.state = GameState::Playing;
            msg_log.log(Msg::GameState(settings.state));
        }

        InputAction::SelectItem(item_index) => {
            if item_index < game_data.objects[player_handle].inventory.len() {
                let item_key = game_data.objects[player_handle].inventory[item_index];
                if game_data.objects[item_key].name == "stone" {
                    settings.state = GameState::Throwing;
                    msg_log.log(Msg::GameState(settings.state));
                }
            }
            // if item index is not in the player's inventory, do nothing
        }

        InputAction::Exit => {
            settings.exiting = true;
        }

        _ => {
        }
    }
}

pub fn handle_input_throwing(input: InputAction,
                             game_data: &mut GameData, 
                             settings: &mut GameSettings,
                             msg_log: &mut MsgLog) -> Action {
    let player_handle = game_data.find_player().unwrap();

    let mut player_turn: Action = Action::none();

    match input {
        InputAction::Inventory => {
            settings.state = GameState::Playing;
            msg_log.log(Msg::GameState(settings.state));
        }

        InputAction::Exit => {
            settings.exiting = true;
        }

        InputAction::MapClick(_map_loc, map_cell) => {
            // NOTE this does not use the selected item, it just finds the first stone
            let mut stone = None;
            let mut stone_index = None;
            for (index, obj_id) in game_data.objects[player_handle].inventory.iter().enumerate() {
                if let Some(Item::Stone) = game_data.objects[*obj_id].item {
                    stone = Some(*obj_id);
                    stone_index = Some(index);
                    break;
                }
            }

            if let (Some(stone_handle), Some(index)) = (stone, stone_index) {
                player_turn = Action::ThrowStone(map_cell, index);

                // turn off throwing overlay
                settings.draw_throw_overlay = false;

                // exit throwing state
                settings.state = GameState::Playing;
                msg_log.log(Msg::GameState(settings.state));
            }
        }

        _ => {
        }
    }

    return player_turn;
}

pub fn handle_input(input_action: InputAction,
                    game_data: &mut GameData, 
                    settings: &mut GameSettings,
                    display_state: &mut DisplayState,
                    msg_log: &mut MsgLog,
                    config: &Config) -> Action {
    let player_handle = game_data.find_player().unwrap();

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

        (InputAction::MapClick(_map_loc, _map_cell), _) => {
            player_turn = Action::none();
        }

        (InputAction::Yell, true) => {
            player_turn = Action::Yell;
        }

        (InputAction::IncreaseMoveMode, true) => {
            let player = &mut game_data.objects[player_handle];
            player.move_mode = player.move_mode.map(|mode| mode.increase());
            player.movement = Some(reach_by_mode(player.move_mode.unwrap()));
            

            player_turn = Action::none();
        }

        (InputAction::DecreaseMoveMode, true) => {
            let player = &mut game_data.objects[player_handle];
            player.move_mode = player.move_mode.map(|mode| mode.decrease());
            player.movement = Some(reach_by_mode(player.move_mode.unwrap()));

            player_turn = Action::none();
        }

        (InputAction::Inventory, true) => {
            settings.state = GameState::Inventory;
            msg_log.log(Msg::GameState(settings.state));
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

pub fn throw_stone(player_handle: ObjectId,
                   stone_index: usize,
                   start_pos: Pos,
                   end_pos: Pos,
                   game_data: &mut GameData,
                   msg_log: &mut MsgLog) {
    let stone_handle =
        game_data.objects[player_handle].inventory.remove(stone_index);

    let throw_line = Line::new(start_pos.to_tuple(), end_pos.to_tuple());

    // get target position in direction of player click
    let mut end_pos =
        Pos::from(throw_line.into_iter().take(PLAYER_THROW_DIST).last().unwrap());

    let pos_diff = end_pos - start_pos;
    if let Some(blocked) = game_data.map.is_blocked_by_wall(start_pos, pos_diff.x, pos_diff.y) {
        // the start pos of the blocked struct is the last reached position
        end_pos = blocked.start_pos;
    }

    game_data.objects[stone_handle].set_pos(end_pos);

    // log the stone throw event
    msg_log.log(Msg::StoneThrow(player_handle, stone_handle, start_pos, end_pos));
}
