use std::str::FromStr;
use std::fmt;

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
    ClassMenu,
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

impl fmt::Display for InputAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputAction::Move(direction) => {
                match direction {
                    Direction::Left => write!(f, "left"),
                    Direction::Right => write!(f, "right"),
                    Direction::Up => write!(f, "up"),
                    Direction::Down => write!(f, "down"),
                    Direction::DownLeft => write!(f, "downleft"),
                    Direction::DownRight => write!(f, "downright"),
                    Direction::UpLeft => write!(f, "upleft"),
                    Direction::UpRight => write!(f, "upright"),
                }
            },
            InputAction::Pass => write!(f, "pass"),
            InputAction::MapClick(loc, cell) => write!(f, "click {} {} {} {}", loc.x, loc.y, cell.x, cell.y),
            InputAction::Pickup => write!(f, "pickup"),
            InputAction::DropItem => write!(f, "drop"),
            InputAction::SwapPrimaryItem => write!(f, "swapitem"),
            InputAction::Inventory => write!(f, "inventory"),
            InputAction::SkillMenu => write!(f, "skill"),
            InputAction::ClassMenu => write!(f, "class"),
            InputAction::Exit => write!(f, "exit"),
            InputAction::Esc => write!(f, "esc"),
            InputAction::ExploreAll => write!(f, "exploreall"),
            InputAction::RegenerateMap => write!(f, "regenmap"),
            InputAction::GodMode => write!(f, "godmode"),
            InputAction::RegenMap => write!(f, "regenmap"),
            InputAction::Yell => write!(f, "yell"),
            InputAction::IncreaseMoveMode => write!(f, "faster"),
            InputAction::DecreaseMoveMode => write!(f, "slower"),
            InputAction::OverlayOn => write!(f, "overlayon"),
            InputAction::OverlayOff => write!(f, "overlayoff"),
            InputAction::SelectItem(item) => write!(f, "selectitem {}", item),
            InputAction::ToggleConsole => write!(f, "toggleconsole"),
            InputAction::UseItem => write!(f, "use"),
            InputAction::Interact => write!(f, "interact"),
            InputAction::None => write!(f, "none"),
        }
    }
}

impl FromStr for InputAction {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();

        if s == "left" {
            return Ok(InputAction::Move(Direction::Left));
        } else if s == "right" {
            return Ok(InputAction::Move(Direction::Right));
        } else if s == "up" {
            return Ok(InputAction::Move(Direction::Up));
        } else if s == "down" {
            return Ok(InputAction::Move(Direction::Down));
        } else if s == "upleft" {
            return Ok(InputAction::Move(Direction::UpLeft));
        } else if s == "upright" {
            return Ok(InputAction::Move(Direction::UpRight));
        } else if s == "downleft" {
            return Ok(InputAction::Move(Direction::DownLeft));
        } else if s == "downright" {
            return Ok(InputAction::Move(Direction::DownRight));
        } else if s == "pass" {
            return Ok(InputAction::Pass);
        } else if s == "pickup" {
            return Ok(InputAction::Pickup);
        } else if s == "drop" {
            return Ok(InputAction::DropItem);
        } else if s == "yell" {
            return Ok(InputAction::Yell);
        } else if s == "inventory" {
            return Ok(InputAction::Inventory);
        } else if s == "use" {
            return Ok(InputAction::UseItem);
        } else if s.starts_with("selectitem") {
            let args = s.split(" ").collect::<Vec<&str>>();
            let selection = args[1].parse::<usize>().unwrap();
            return Ok(InputAction::SelectItem(selection));
        } else if s == "interact" {
            return Ok(InputAction::Interact);
        } else if s == "godmode" {
            return Ok(InputAction::GodMode);
        } else if s.starts_with("click") {
            let args = s.split(" ").collect::<Vec<&str>>();
            let loc_x = args[1].parse::<i32>().unwrap();
            let loc_y = args[2].parse::<i32>().unwrap();
            let cell_x = args[3].parse::<i32>().unwrap();
            let cell_y = args[4].parse::<i32>().unwrap();
            return Ok(InputAction::MapClick(Pos::new(loc_x, loc_y), Pos::new(cell_x, cell_y)));
        } else if s == "swapitem" {
            return Ok(InputAction::SwapPrimaryItem);
        } else if s == "skill" {
            return Ok(InputAction::SkillMenu);
        } else if s == "class" {
            return Ok(InputAction::ClassMenu);
        } else if s == "esc" {
            return Ok(InputAction::Esc);
        } else if s == "faster" {
            return Ok(InputAction::IncreaseMoveMode);
        } else if s == "slower" {
            return Ok(InputAction::DecreaseMoveMode);
        } else {
            return Err(format!("Could not parse '{}' as InputAction", s));
        }
    }
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

                match settings.inventory_action {
                    InventoryAction::Use => {
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

                    InventoryAction::Drop => {
                        let player_pos = data.entities.pos[&player_id];

                        // Find a place to drop the item, without placing it on the same tile
                        // as another item.
                        let mut found_tile = false;
                        let mut dist = 1;
                        while !found_tile && dist < 10 {
                            let positions = data.map.floodfill(player_pos, dist);

                            for pos in positions {
                                if data.item_at_pos(pos).is_none() {
                                    data.entities.remove_item(player_id, item_key);
                                    data.entities.set_pos(item_key, pos);

                                    settings.state = GameState::Playing;
                                    msg_log.log(Msg::GameState(settings.state));

                                    found_tile = true;
                                    break;
                                }
                            }

                            dist += 1;
                        }

                        if !found_tile {
                            msg_log.log(Msg::DropFailed(player_id));
                        }
                    }
                }
            }
            // if item index is not in the player's inventory, do nothing
        }

        InputAction::Esc => {
            settings.state = GameState::Playing;
            msg_log.log(Msg::GameState(settings.state));
        }

        InputAction::Exit => {
            settings.state = GameState::ConfirmQuit;
            msg_log.log(Msg::GameState(settings.state));
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
            if data.entities.energy[&player_id] > 0 {
                if skill_index < data.entities.skills[&player_id].len() {
                    settings.state = GameState::Selection;
                    settings.selection.only_visible = false;

                    let reach = Reach::single(1);

                    match data.entities.skills[&player_id][skill_index] {
                        Skill::GrassThrow => {
                            settings.selection =
                                Selection::new(SelectionType::WithinReach(reach), SelectionAction::GrassThrow);
                            settings.state = GameState::Selection;
                            msg_log.log(Msg::GameState(settings.state));
                        }

                        Skill::GrassBlade => {
                            player_turn = Action::GrassBlade(player_id);
                            settings.state = GameState::Playing;
                            msg_log.log(Msg::GameState(settings.state));
                        }

                        Skill::Blink => {
                            player_turn = Action::Blink(player_id);

                            settings.state = GameState::Playing;
                            msg_log.log(Msg::GameState(settings.state));
                        }

                        Skill::PassWall => {
                            settings.selection =
                                Selection::new(SelectionType::WithinReach(reach), SelectionAction::PassWall);
                            settings.state = GameState::Selection;
                            msg_log.log(Msg::GameState(settings.state));
                        }

                        Skill::Rubble => {
                            let reach = Reach::horiz(1);
                            settings.selection =
                                Selection::new(SelectionType::WithinReach(reach), SelectionAction::Rubble);
                            settings.state = GameState::Selection;
                            msg_log.log(Msg::GameState(settings.state));
                        }

                        Skill::Reform => {
                            settings.selection =
                                Selection::new(SelectionType::WithinReach(reach), SelectionAction::Reform);
                            settings.state = GameState::Selection;
                            msg_log.log(Msg::GameState(settings.state));
                        }

                        Skill::Swap => {
                            // NOTE make this a const or config item
                            let reach = Reach::single(4);
                            settings.selection =
                                Selection::new(SelectionType::WithinReach(reach), SelectionAction::Swap);
                            settings.state = GameState::Selection;
                            msg_log.log(Msg::GameState(settings.state));
                        }

                        Skill::Push => {
                            settings.selection =
                                Selection::new(SelectionType::WithinReach(reach), SelectionAction::Push);
                            settings.state = GameState::Selection;
                            msg_log.log(Msg::GameState(settings.state));
                        }
                    }

                    msg_log.log(Msg::GameState(settings.state));
                }
            } else {
                msg_log.log(Msg::NotEnoughEnergy(player_id));
            }
        }

        InputAction::Esc => {
            settings.state = GameState::Playing;
            msg_log.log(Msg::GameState(settings.state));
        }

        InputAction::Exit => {
            settings.state = GameState::ConfirmQuit;
            msg_log.log(Msg::GameState(settings.state));
        }

        _ => {
        }
    }

    return player_turn;
}

pub fn handle_input_class_menu(input: InputAction,
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

        InputAction::ClassMenu => {
            settings.state = GameState::Playing;
            msg_log.log(Msg::GameState(settings.state));
        }

        InputAction::SelectItem(class_index) => {
            let classes = EntityClass::classes();
            if class_index < classes.len() {
                settings.state = GameState::Selection;
                settings.selection.only_visible = false;

                match classes[class_index] {
                    EntityClass::General => {
                        data.entities.class[&player_id] = classes[class_index];
                        msg_log.log(Msg::GameState(GameState::Playing));
                    }

                    EntityClass::Monolith => {
                        data.entities.class[&player_id] = classes[class_index];
                        data.entities.skills[&player_id].push(Skill::PassWall);
                        data.entities.skills[&player_id].push(Skill::Rubble);
                        data.entities.skills[&player_id].push(Skill::Reform);
                        msg_log.log(Msg::GameState(GameState::Playing));
                    }

                    EntityClass::Grass => {
                        data.entities.class[&player_id] = classes[class_index];
                        data.entities.skills[&player_id].push(Skill::GrassThrow);
                        data.entities.skills[&player_id].push(Skill::GrassBlade);
                        msg_log.log(Msg::GameState(GameState::Playing));
                    }

                    EntityClass::Clockwork => {
                        data.entities.class[&player_id] = classes[class_index];
                        data.entities.skills[&player_id].push(Skill::Push);
                        msg_log.log(Msg::GameState(GameState::Playing));
                    }
                }
            }

            settings.state = GameState::Playing;
            msg_log.log(Msg::GameState(settings.state));
        }

        InputAction::Esc => {
            settings.state = GameState::Playing;
            msg_log.log(Msg::GameState(settings.state));
        }

        InputAction::Exit => {
            settings.state = GameState::ConfirmQuit;
            msg_log.log(Msg::GameState(settings.state));
        }

        _ => {
        }
    }

    return player_turn;
}

pub fn handle_input_confirm_quit(input: InputAction,
                                 _data: &mut GameData,
                                 settings: &mut GameSettings,
                                 msg_log: &mut MsgLog) -> Action {
    let player_turn: Action = Action::NoAction;

    match input {
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
            settings.state = GameState::ConfirmQuit;
            msg_log.log(Msg::GameState(settings.state));
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

            if let Some(movement) = maybe_movement {
                player_turn = Action::Move(movement);
            }
        }

        (InputAction::DropItem, true) => {
            game.settings.inventory_action = InventoryAction::Drop;
            game.settings.state = GameState::Inventory;
            game.msg_log.log(Msg::GameState(game.settings.state));
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
                game.msg_log.log(Msg::TriedRunWithHeavyEquipment);
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
            game.settings.inventory_action = InventoryAction::Use;
            game.settings.state = GameState::Inventory;
            game.msg_log.log(Msg::GameState(game.settings.state));
        }

        (InputAction::SkillMenu, true) => {
            game.settings.state = GameState::SkillMenu;
            game.msg_log.log(Msg::GameState(game.settings.state));
        }

        (InputAction::ClassMenu, true) => {
            game.settings.state = GameState::ClassMenu;
            game.msg_log.log(Msg::GameState(game.settings.state));
        }

        (InputAction::Exit, _) => {
            game.settings.state = GameState::ConfirmQuit;
            game.msg_log.log(Msg::GameState(game.settings.state));
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
            let player_id = game.data.find_player().unwrap();
            if let Some(ref mut fighter) = game.data.entities.fighter.get_mut(&player_id) {
                fighter.hp = god_mode_hp;
                fighter.max_hp = god_mode_hp;
            }
            game.data.entities.energy[&player_id] = 1000;

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
                    item_id: EntityId,
                    entities: &mut Entities,
                    _msg_log: &mut MsgLog) {
    // pick up item
    let item = entities.item[&item_id];
    let item_class = item.class();

    match item_class {
        ItemClass::Primary => {
            if item_primary_at(entity_id, entities, 0) &&
               item_primary_at(entity_id, entities, 1) {
                entities.inventory[&entity_id][0] = item_id;

                let obj_pos = entities.pos[&entity_id];
                entities.set_pos(entity_id, obj_pos);
            } else {
                entities.inventory[&entity_id].push_front(item_id);
            }
        }

        ItemClass::Secondary => {
            entities.inventory[&entity_id].push_back(item_id);
        }
    }

    entities.set_xy(item_id, -1, -1);
}

pub fn throw_item(player_id: EntityId,
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

    game_data.entities.remove_item(player_id, item_id);
}

pub fn place_trap(trap_id: EntityId,
                  place_pos: Pos,
                  game_data: &mut GameData) {
    game_data.entities.set_pos(trap_id, place_pos);
    game_data.entities.armed[&trap_id] = true;
}

