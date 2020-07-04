use serde::{Serialize, Deserialize};

use roguelike_core::movement::{Direction, Action, Reach};
use roguelike_core::types::*;
use roguelike_core::movement;
use roguelike_core::utils::{reach_by_mode, item_primary_at, line};
use roguelike_core::messaging::{Msg, MsgLog};
use roguelike_core::constants::*;
use roguelike_core::config::Config;

use crate::game::*;
use crate::make_map;


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyDirection {
    Down,
    Up
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum InputAction {
    Move(Direction),
    Pass,
    MapClick(Pos, Pos), // map loc, map cell
    Pickup,
    DropItem,
    SwapPrimaryItem,
    Inventory,
    SkillMenu,
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
    UseItem,
    Interact,
    None,
}

//pub fn handle_input_console(input: InputAction,
//                            key_input: &mut Vec<(KeyDirection, Keycode)>,
//                            console: &mut Console,
//                            game_data: &mut GameData,
//                            display_state: &mut DisplayState,
//                            settings: &mut GameSettings,
//                            config: &Config,
//                            msg_log: &mut MsgLog) {
//    match input {
//        InputAction::ToggleConsole => {
//            settings.state = GameState::Playing;
//        }
//
//        _ => {
//            for (dir, key) in key_input.iter() {
//                console.eval(*key, *dir, game_data, display_state, settings, config, msg_log);
//            }
//        }
//    }
//}

pub fn handle_input_inventory(input: InputAction,
                              data: &mut GameData,
                              settings: &mut GameSettings,
                              msg_log: &mut MsgLog) {
    let player_id = data.find_player().unwrap();

    match input {
        InputAction::Inventory => {
            settings.state = GameState::Playing;
            msg_log.log(Msg::GameState(settings.state));
        }

        InputAction::SelectItem(item_index) => {
            if item_index < data.entities.inventory[&player_id].len() {
                let item_key = data.entities.inventory[&player_id][item_index];

                data.entities.selected_item.insert(player_id, item_key);

                settings.state = GameState::Selection;
                settings.selection.only_visible = false;

                // if the item is a trap, set it.
                // otherwise, throw it.
                if data.entities.trap.get(&item_key).is_some() {
                    settings.selection =
                        Selection::new(SelectionType::WithinReach(Reach::single(1)), SelectionAction::PlaceTrap);
                } else {
                    settings.selection =
                        Selection::new(SelectionType::WithinRadius(PLAYER_THROW_DIST), SelectionAction::Throw);
                }

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

pub fn handle_input_skill_menu(input: InputAction,
                               data: &mut GameData,
                               settings: &mut GameSettings,
                               msg_log: &mut MsgLog) -> Action {
    let mut player_turn: Action = Action::NoAction;
    let player_id = data.find_player().unwrap();

    match input {
        InputAction::Inventory => {
            settings.state = GameState::Inventory;
            msg_log.log(Msg::GameState(settings.state));
        }

        InputAction::SkillMenu => {
            settings.state = GameState::Playing;
            msg_log.log(Msg::GameState(settings.state));
        }

        InputAction::SelectItem(skill_index) => {
            if skill_index < data.entities.skills[&player_id].len() {
                settings.state = GameState::Selection;
                settings.selection.only_visible = false;

                let reach = Reach::single(1);

                match data.entities.skills[&player_id][skill_index] {
                    Skill::GrassThrow => {
                        settings.selection =
                            Selection::new(SelectionType::WithinReach(reach), SelectionAction::GrassThrow);
                    }

                    Skill::Blink => {
                        player_turn = Action::Blink(player_id);
                    }
                }

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

    return player_turn;
}

pub fn handle_input_selection(input: InputAction,
                              data: &mut GameData, 
                              settings: &mut GameSettings,
                              config: &Config,
                              msg_log: &mut MsgLog) -> Action {
    let player_id = data.find_player().unwrap();

    let mut player_turn: Action = Action::none();

    match input {
        InputAction::Inventory => {
            settings.state = GameState::Inventory;
            msg_log.log(Msg::GameState(settings.state));
        }

        InputAction::Exit => {
            settings.exiting = true;
        }

        InputAction::Esc => {
            settings.state = GameState::Playing;
            msg_log.log(Msg::GameState(settings.state));
            settings.draw_selection_overlay = false;
        }

        InputAction::MapClick(_map_loc, map_cell) => {
            let player_pos = data.entities.pos[&player_id];

            if let Some(action) = settings.selection.select(player_pos, map_cell, config.fov_radius_player, data) {
                // exit selection state
                settings.state = GameState::Playing;
                msg_log.log(Msg::GameState(settings.state));
                settings.draw_selection_overlay = false;
                player_turn = action;
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

    let player_alive = game.data.entities.alive[&player_id];

    match (game.input_action, player_alive) {
        (InputAction::Pass, true) => {
            player_turn = Action::Pass;
        }

        (InputAction::Move(move_action), true) => {
            let player_id = game.data.find_player().unwrap();

            let player_reach = game.data.entities.movement[&player_id];
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
            if let Some(item_id) = game.data.entities.inventory[&player_id].remove(0) {
               let player_pos = game.data.entities.pos[&player_id];
               game.data.entities.set_pos(item_id, player_pos);
            }
        }

        (InputAction::Pickup, true) => {
            let pos = game.data.entities.pos[&player_id];

            for key in game.data.entities.pos.keys() {
                let is_item = game.data.entities.item.get(key).is_some();
                let is_disarmed_trap =
                    game.data.entities.trap.get(key).is_some() &&
                    game.data.entities.armed.get(key) == Some(&false);

                if game.data.entities.pos[key] == pos && (is_item || is_disarmed_trap) {
                    player_turn = Action::Pickup(*key);
                    break;
                }
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
            let holding_hammer = game.data.using(player_id, Item::Hammer);

            let move_mode = game.data.entities.move_mode.get(&player_id).expect("Player should have a move mode");
            let new_move_mode = move_mode.increase();

            if new_move_mode == movement::MoveMode::Run && (holding_shield || holding_hammer) {
                game.msg_log.log(Msg::TriedRunWithShield);
            } else {
                game.data.entities.move_mode[&player_id] = new_move_mode;
                game.data.entities.movement[&player_id] = reach_by_mode(game.data.entities.move_mode[&player_id]);

                game.msg_log.log(Msg::MoveMode(new_move_mode));
            }

            player_turn = Action::none();
        }

        (InputAction::DecreaseMoveMode, true) => {
            game.data.entities.move_mode[&player_id] =
                game.data.entities.move_mode[&player_id].decrease();

            game.data.entities.movement[&player_id] =
                reach_by_mode(game.data.entities.move_mode[&player_id]);

            game.msg_log.log(Msg::MoveMode(game.data.entities.move_mode[&player_id]));

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

        (InputAction::SkillMenu, true) => {
            game.settings.state = GameState::SkillMenu;
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
            if item_primary_at(player_id, &mut game.data.entities, 0) &&
               item_primary_at(player_id, &mut game.data.entities, 1) {
                   let temp_id = game.data.entities.inventory[&player_id][0];

                   game.data.entities.inventory[&player_id][0] = 
                       game.data.entities.inventory[&player_id][1];

                   game.data.entities.inventory[&player_id][1] = temp_id;
           }
        }

        (InputAction::RegenerateMap, _) => {
            let _position = make_map::make_map(&game.config.map_load.clone(), game);
        }

        (InputAction::GodMode, true) => {
            let god_mode_hp = 1000000;
            let handle = game.data.find_player().unwrap();
            if let Some(ref mut fighter) = game.data.entities.fighter.get_mut(&handle) {
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

        // TODO console
        (InputAction::ToggleConsole, _) => {
            // TODO console
            //if game.settings.state == GameState::Console {
            //    game.settings.state = GameState::Playing;
            //} else {
            //    //game.console.time_at_open = game.settings.time;
            //    //game.console.height = 0;
            //    //game.settings.state = GameState::Console;
            //}
        }

        (InputAction::Interact, _) => {
            game.settings.state = GameState::Selection;
            let reach = Reach::single(1);
            game.settings.selection =
                Selection::new(SelectionType::WithinReach(reach), SelectionAction::Interact);
            game.msg_log.log(Msg::GameState(game.settings.state));
        }

        (InputAction::UseItem, _) => {
            if game.data.using(player_id, Item::Hammer) {
                game.settings.state = GameState::Selection;
                let reach = Reach::Horiz(1);
                game.settings.selection =
                    Selection::new(SelectionType::WithinReach(reach), SelectionAction::Hammer);
                game.msg_log.log(Msg::GameState(game.settings.state));
            } else if game.data.using(player_id, Item::Sword) {
                let player_pos = game.data.entities.pos[&player_id];
                player_turn = Action::UseItem(player_pos);
            }
        }

        (_, _) => {
        }
    }

    return player_turn;
}

pub fn pick_item_up(entity_id: EntityId,
                    pickedup_id: EntityId,
                    entities: &mut Entities) {
    // pick up item
    let item = entities.item[&pickedup_id];
    let item_class = item.class();

    match item_class {
        ItemClass::Primary => {
            if item_primary_at(entity_id, entities, 0) &&
               item_primary_at(entity_id, entities, 1) {
                entities.inventory[&entity_id][0] = pickedup_id;

                let obj_pos = entities.pos[&entity_id];
                entities.set_pos(entity_id, obj_pos);
            } else {
                entities.inventory[&entity_id].push_front(pickedup_id);
            }
        }

        ItemClass::Secondary => {
            entities.inventory[&entity_id].push_back(pickedup_id);
        }
    } 

    entities.set_xy(pickedup_id, -1, -1);
}

pub fn throw_item(_player_id: EntityId,
                  item_id: EntityId,
                  start_pos: Pos,
                  end_pos: Pos,
                  game_data: &mut GameData) {
    let throw_line = line(start_pos, end_pos);

    // get target position in direction of player click
    let mut end_pos =
        Pos::from(throw_line.into_iter().take(PLAYER_THROW_DIST).last().unwrap());

    let pos_diff = end_pos - start_pos;
    if let Some(blocked) = game_data.map.is_blocked_by_wall(start_pos, pos_diff.x, pos_diff.y) {
        // the start pos of the blocked struct is the last reached position
        end_pos = blocked.start_pos;
    }

    game_data.entities.set_pos(item_id, end_pos);
}

pub fn place_trap(trap_id: EntityId,
                  place_pos: Pos,
                  game_data: &mut GameData) {
    game_data.entities.set_pos(trap_id, place_pos);
    game_data.entities.armed[&trap_id] = true;
}

