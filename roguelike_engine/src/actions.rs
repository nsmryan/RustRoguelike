use rand::prelude::*;

use tcod::line::*;

use sdl2::keyboard::Keycode;

use roguelike_core::config::*;
use roguelike_core::movement::{Direction, Action};
use roguelike_core::types::*;
use roguelike_core::movement;
use roguelike_core::utils::{reach_by_mode, item_primary_at};
use roguelike_core::messaging::{Msg, MsgLog};
use roguelike_core::constants::*;

use crate::game::*;
use crate::display::*;
use crate::generation;
use crate::console::Console;


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyDirection {
    Down,
    Up
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputAction {
    Move(Direction),
    Pass,
    MapClick(Pos, Pos),
    Pickup,
    DropItem,
    SwapPrimaryItem,
    Inventory,
    Exit,
    Esc,
    ExploreAll,
    RegenerateMap,
    GodMode,
    RegenMap,
    Yell,
    IncreaseMoveMode,
    DecreaseMoveMode,
    OverlayOn,
    OverlayOff,
    SelectItem(usize),
    ToggleConsole,
    None,
}

pub fn player_apply_action(action: Action,
                           game_data: &mut GameData,
                           msg_log: &mut MsgLog) {
    let player_id = game_data.find_player().unwrap();
    let player_pos = game_data.objects[player_id].pos();

    match action {
        Action::Move(movement) => {
            movement::player_move_or_attack(movement, game_data, msg_log);
        }

        Action::StateChange(_behavior) => {
            panic!("Player tried to change behavior?");
        }

        Action::Pickup(item_id) => {
            pick_item_up(player_id, item_id, &mut game_data.objects);
            msg_log.log(Msg::PickedUp(player_id, item_id));
        }

        Action::ThrowItem(throw_pos, item_index) => {
            throw_item(player_id, item_index, player_pos, throw_pos, game_data, msg_log);
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

pub fn handle_input_console(input: InputAction,
                            key_input: &mut Vec<(KeyDirection, Keycode)>,
                            console: &mut Console,
                            game_data: &mut GameData,
                            display_state: &mut DisplayState,
                            settings: &mut GameSettings,
                            config: &Config,
                            msg_log: &mut MsgLog) {
    match input {
        InputAction::ToggleConsole => {
            settings.state = GameState::Playing;
        }

        _ => {
            for (dir, key) in key_input.iter() {
                console.eval(*key, *dir, game_data, display_state, settings, config, msg_log);
            }
        }
    }
}

pub fn handle_input_inventory(input: InputAction,
                              game_data: &mut GameData,
                              settings: &mut GameSettings,
                              msg_log: &mut MsgLog) {
    let player_id = game_data.find_player().unwrap();

    match input {
        InputAction::Inventory => {
            settings.state = GameState::Playing;
            msg_log.log(Msg::GameState(settings.state));
        }

        InputAction::SelectItem(item_index) => {
            if item_index < game_data.objects[player_id].inventory.len() {
                let item_key = game_data.objects[player_id].inventory[item_index];

                game_data.objects[player_id].selected_item =
                    Some(item_key);

                settings.state = GameState::Throwing;
                msg_log.log(Msg::GameState(settings.state));
            }
            // if item index is not in the player's inventory, do nothing
        }

        InputAction::Esc => {
            settings.state = GameState::Playing;
            msg_log.log(Msg::GameState(settings.state));
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
    let player_id = game_data.find_player().unwrap();

    let mut player_turn: Action = Action::none();

    match input {
        InputAction::Inventory => {
            settings.state = GameState::Playing;
            msg_log.log(Msg::GameState(settings.state));
        }

        InputAction::Exit => {
            settings.exiting = true;
        }

        InputAction::Esc => {
            settings.state = GameState::Playing;
            msg_log.log(Msg::GameState(settings.state));
            settings.draw_throw_overlay = false;
        }

        InputAction::MapClick(_map_loc, map_cell) => {
            let item =
                game_data.objects[player_id]
                         .selected_item
                         .expect("No item selected when throwing!");

            let item_index =
                game_data.objects[player_id]
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

pub fn handle_input(game: &mut Game) -> Action {
    let player_id = game.data.find_player().unwrap();

    let mut player_turn: Action = Action::none();

    let player_alive = game.data.objects[player_id].alive;

    match (game.input_action, player_alive) {
        (InputAction::Pass, true) => {
            player_turn = Action::Pass;
        }

        (InputAction::Move(move_action), true) => {
            let player_id = game.data.find_player().unwrap();

            let player_reach = game.data.objects[player_id].movement.unwrap();
            let maybe_movement = 
                movement::calculate_move(move_action,
                                         player_reach,
                                         player_id,
                                         &mut game.data);

            // if moved, walked into enemy, and holding a dagger, then attack
            if let Some(movement) = maybe_movement {
                player_turn = Action::Move(movement);
            }
        }

        (InputAction::DropItem, true) => {
            if let Some(item_id) = game.data.objects[player_id].inventory.remove(0) {
               let player_pos = game.data.objects[player_id].pos();
               game.data.objects[item_id].set_pos(player_pos);
            }
        }

        (InputAction::Pickup, true) => {
            let player = &game.data.objects[player_id];
            let item_id = game.data.objects.keys().filter(|key| {
                return (game.data.objects[*key].pos() == player.pos()) && game.data.objects[*key].item.is_some();
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
            let holding_shield = game.data.using(player_id, Item::Shield);
            let player = &mut game.data.objects[player_id];

            let move_mode = player.move_mode.expect("Player should have a move mode");
            let new_move_mode = move_mode.increase();

            if new_move_mode == movement::MoveMode::Run && holding_shield {
                game.msg_log.log(Msg::TriedRunWithShield);
            } else {
                player.move_mode = Some(new_move_mode);
                player.movement = Some(reach_by_mode(player.move_mode.unwrap()));

                game.msg_log.log(Msg::MoveMode(new_move_mode));
            }

            player_turn = Action::none();
        }

        (InputAction::DecreaseMoveMode, true) => {
            let player = &mut game.data.objects[player_id];
            player.move_mode = player.move_mode.map(|mode| mode.decrease());
            player.movement = Some(reach_by_mode(player.move_mode.unwrap()));

            player_turn = Action::none();
        }

        (InputAction::OverlayOn, _) => {
            game.settings.overlay = true;

            player_turn = Action::none();
        }

        (InputAction::OverlayOff, _) => {
            game.settings.overlay = false;

            player_turn = Action::none();
        }

        (InputAction::Inventory, true) => {
            game.settings.state = GameState::Inventory;
            game.msg_log.log(Msg::GameState(game.settings.state));
        }

        (InputAction::Exit, _) => {
            game.settings.exiting = true;
        }

        (InputAction::ExploreAll, _) => {
            for x in 0..game.data.map.width() {
                for y in 0..game.data.map.height() {
                    game.data.map.tiles[x as usize][y as usize].explored = true;
                }
            }
        }

        (InputAction::SwapPrimaryItem, _) => {
            if item_primary_at(player_id, &mut game.data.objects, 0) &&
               item_primary_at(player_id, &mut game.data.objects, 1) {
                   let temp_id = game.data.objects[player_id].inventory[0];

                   game.data.objects[player_id].inventory[0] = 
                       game.data.objects[player_id].inventory[1];

                   game.data.objects[player_id].inventory[1] = temp_id;
           }
        }

        (InputAction::RegenerateMap, _) => {
            let mut rng: SmallRng = SeedableRng::seed_from_u64(2);
            let (data, _position) =
                generation::make_map(&game.settings.map_type,
                                     &mut game.data.objects,
                                     &game.config,
                                     &mut game.display_state,
                                     &mut rng);
            game.data.map = data.map;
        }

        (InputAction::GodMode, true) => {
            let god_mode_hp = 1000000;
            let handle = game.data.find_player().unwrap();
            if let Some(ref mut fighter) = game.data.objects[handle].fighter {
                fighter.hp = god_mode_hp;
                fighter.max_hp = god_mode_hp;
            }

            // set god mode flag
            game.settings.god_mode = true;

            // set all tiles to be transparent and walkable. walkable is not current used
            // anywhere
            for x in 0..game.data.map.tiles.len() {
                for y in 0..game.data.map.tiles[0].len() {
                    game.data.map.set_cell(x as i32, y as i32, true);
                }
            }
            game.data.map.update_map();
        }

        (InputAction::ToggleConsole, _) => {
            if game.settings.state == GameState::Console {
                game.settings.state = GameState::Playing;
            } else {
                game.console.time_at_open = game.settings.time;
                game.console.height = 0;
                game.settings.state = GameState::Console;
            }
        }

        (_, _) => {
        }
    }

    return player_turn;
}

pub fn pick_item_up(object_id: ObjectId,
                    item_id: ObjectId,
                    objects: &mut ObjMap) {
    let pickup_class = objects[item_id].item.unwrap().class();

    match pickup_class {
        ItemClass::Primary => {
            if item_primary_at(object_id, objects, 0) &&
               item_primary_at(object_id, objects, 1) {
                let old_primary = objects[object_id].inventory[0];

                objects[object_id].inventory[0] = item_id;

                let obj_pos = objects[object_id].pos();
                objects[old_primary].set_pos(obj_pos);
            } else {
                objects[object_id].inventory.push_front(item_id);
            }
        }

        ItemClass::Secondary => {
            objects[object_id].inventory.push_back(item_id);
        }
    }

    objects[item_id].set_xy(-1, -1);
}

pub fn throw_item(player_id: ObjectId,
                  item_index: usize,
                  start_pos: Pos,
                  end_pos: Pos,
                  game_data: &mut GameData,
                  msg_log: &mut MsgLog) {
    let item_id =
        game_data.objects[player_id].inventory.remove(item_index).unwrap();

    let throw_line = Line::new(start_pos.to_tuple(), end_pos.to_tuple());

    // get target position in direction of player click
    let mut end_pos =
        Pos::from(throw_line.into_iter().take(PLAYER_THROW_DIST).last().unwrap());

    let pos_diff = end_pos - start_pos;
    if let Some(blocked) = game_data.map.is_blocked_by_wall(start_pos, pos_diff.x, pos_diff.y) {
        // the start pos of the blocked struct is the last reached position
        end_pos = blocked.start_pos;
    }

    game_data.objects[item_id].set_pos(end_pos);

    // log the item throw event
    msg_log.log(Msg::ItemThrow(player_id, item_id, start_pos, end_pos));
}

