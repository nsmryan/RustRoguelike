use rand::prelude::*;

use tcod::line::*;

use roguelike_core::config::*;
use roguelike_core::movement::Action;
use roguelike_core::types::*;
use roguelike_core::movement;
use roguelike_core::utils::reach_by_mode;
use roguelike_core::messaging::{Msg, MsgLog};
use roguelike_core::constants::*;

use crate::game::*;
use crate::display::*;
use crate::input::*;
use crate::generation;


pub fn player_apply_action(action: Action,
                           game_data: &mut GameData,
                           msg_log: &mut MsgLog) {
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

        Action::ThrowItem(throw_pos, item_index) => {
            throw_item(player_handle, item_index, player_pos, throw_pos, game_data, msg_log);
        }

        Action::Yell => {
            msg_log.log(Msg::Yell(player_pos));
        }

        Action::Pass => {
            msg_log.log(Msg::Pass());
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

                game_data.objects[player_handle].selected_item =
                    Some(item_key);

                settings.state = GameState::Throwing;
                msg_log.log(Msg::GameState(settings.state));
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
            let item =
                game_data.objects[player_handle]
                         .selected_item
                         .expect("No item selected when throwing!");

            let item_index =
                game_data.objects[player_handle]
                         .inventory
                         .iter()
                         .position(|obj_id| *obj_id == item);

            if let Some(index) = item_index {
                player_turn = Action::ThrowItem(map_cell, index);

                // turn off throwing overlay
                settings.draw_throw_overlay = false;

                // exit throwing state
                settings.state = GameState::Playing;
                msg_log.log(Msg::GameState(settings.state));
            } else {
                panic!("Thrown item not found in inventory!");
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
        (InputAction::Pass, true) => {
            player_turn = Action::Pass;
        }

        (InputAction::Move(move_action), true) => {
            let player_handle = game_data.find_player().unwrap();

            let player_reach = game_data.objects[player_handle].movement.unwrap();
            let maybe_movement = 
                movement::calculate_move(move_action,
                                         player_reach,
                                         player_handle,
                                         game_data);

            // if moved, walked into enemy, and holding a dagger, then attack
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
            let holding_shield = game_data.holds(player_handle, Item::Shield);
            let player = &mut game_data.objects[player_handle];

            let move_mode = player.move_mode.expect("Player should have a move mode");
            let new_move_mode = move_mode.increase();

            if new_move_mode == movement::MoveMode::Run && holding_shield {
                msg_log.log(Msg::TriedRunWithShield);
            } else {
                player.move_mode = Some(new_move_mode);
                player.movement = Some(reach_by_mode(player.move_mode.unwrap()));

                msg_log.log(Msg::MoveMode(new_move_mode));
            }

            player_turn = Action::none();
        }

        (InputAction::DecreaseMoveMode, true) => {
            let player = &mut game_data.objects[player_handle];
            player.move_mode = player.move_mode.map(|mode| mode.decrease());
            player.movement = Some(reach_by_mode(player.move_mode.unwrap()));

            player_turn = Action::none();
        }

        (InputAction::OverlayOn, _) => {
            settings.overlay = true;

            player_turn = Action::none();
        }

        (InputAction::OverlayOff, _) => {
            settings.overlay = false;

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
                    game_data.map.set_cell(x as i32, y as i32, true);
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

pub fn throw_item(player_handle: ObjectId,
                  item_index: usize,
                  start_pos: Pos,
                  end_pos: Pos,
                  game_data: &mut GameData,
                  msg_log: &mut MsgLog) {
    let item_handle =
        game_data.objects[player_handle].inventory.remove(item_index);

    let throw_line = Line::new(start_pos.to_tuple(), end_pos.to_tuple());

    // get target position in direction of player click
    let mut end_pos =
        Pos::from(throw_line.into_iter().take(PLAYER_THROW_DIST).last().unwrap());

    let pos_diff = end_pos - start_pos;
    if let Some(blocked) = game_data.map.is_blocked_by_wall(start_pos, pos_diff.x, pos_diff.y) {
        // the start pos of the blocked struct is the last reached position
        end_pos = blocked.start_pos;
    }

    game_data.objects[item_handle].set_pos(end_pos);

    // log the item throw event
    msg_log.log(Msg::ItemThrow(player_handle, item_handle, start_pos, end_pos));
}

