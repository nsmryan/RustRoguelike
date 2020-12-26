use std::str::FromStr;
use std::fmt;

use serde::{Serialize, Deserialize};

use roguelike_core::movement::{Direction, Action, Reach, Movement};
use roguelike_core::types::*;
use roguelike_core::movement;
use roguelike_core::utils::{reach_by_mode, item_primary_at, add_pos, sub_pos};
use roguelike_core::messaging::{Msg, MsgLog};
use roguelike_core::constants::*;
use roguelike_core::config::Config;
use roguelike_core::line::line;

use crate::game::*;
use crate::selection::*;
use crate::make_map;


// TODO move to input.rs when ready
#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum KeyDir {
    Up,
    Down,
}

// TODO move to input.rs when ready
#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum MouseClick {
    Left,
    Right,
    Middle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum KeyDirection {
    Down,
    Up
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ActionStrength {
    Weak,
    Strong,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ActionMode {
    Primary,
    Alternate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionLoc {
    Dir(Direction),
    Place(Pos),
    None
}

pub type ActionTarget = i32;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum InputAction {
    Move(Direction),
    Pass,
    MapClick(Pos, Pos), // map loc, map cell
    MouseButton(MouseClick, KeyDir),
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
    Yell,
    IncreaseMoveMode,
    DecreaseMoveMode,
    OverlayOn,
    OverlayOff,
    SelectItem(usize),
    ToggleConsole,
    UseItem,
    Interact,
    Chord(Option<Direction>, ActionStrength, ActionMode, ActionTarget),
    CursorMove(Direction),
    CursorApply(ActionMode, ActionTarget),
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
            InputAction::MouseButton(click, keydir) => write!(f, "mousebutton {:?} {:?}", click, keydir),
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
            InputAction::Yell => write!(f, "yell"),
            InputAction::IncreaseMoveMode => write!(f, "faster"),
            InputAction::DecreaseMoveMode => write!(f, "slower"),
            InputAction::OverlayOn => write!(f, "overlayon"),
            InputAction::OverlayOff => write!(f, "overlayoff"),
            InputAction::SelectItem(item) => write!(f, "selectitem {}", item),
            InputAction::ToggleConsole => write!(f, "toggleconsole"),
            InputAction::UseItem => write!(f, "use"),
            InputAction::Interact => write!(f, "interact"),
            InputAction::Chord(dir, stren, mode, target) => write!(f, "chord {:?} {:?} {:?} {:?}", dir, stren, mode, target),
            InputAction::CursorMove(dir) => write!(f, "cursormove {:?}", dir),
            InputAction::CursorApply(mode, target) => write!(f, "chord {:?} {:?}", mode, target),
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

pub fn inventory_use_item(item_index: usize,
                          data: &mut GameData,
                          settings: &mut GameSettings,
                          msg_log: &mut MsgLog) {
    let player_id = data.find_by_name(EntityName::Player).unwrap();
    let item_key = data.entities.inventory[&player_id][item_index];
    data.entities.selected_item.swap_remove(&player_id);
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

pub fn inventory_drop_item(item_index: usize,
                           data: &mut GameData,
                           settings: &mut GameSettings,
                           msg_log: &mut MsgLog) {
    let player_id = data.find_by_name(EntityName::Player).unwrap();
    let player_pos = data.entities.pos[&player_id];
    let item_key = data.entities.inventory[&player_id][item_index];

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

pub fn inventory_select_item(item_index: usize,
                             data: &mut GameData,
                             settings: &mut GameSettings,
                             msg_log: &mut MsgLog) {
    let player_id = data.find_by_name(EntityName::Player).unwrap();

    if item_index < data.entities.inventory[&player_id].len() {
        match settings.inventory_action {
            InventoryAction::Use => {
                inventory_use_item(item_index, data, settings, msg_log);
            }

            InventoryAction::Drop => {
                inventory_drop_item(item_index, data, settings, msg_log);
            }
        }
    }
    // if item index is not in the player's inventory, do nothing
}

pub fn handle_input_inventory(input: InputAction,
                              data: &mut GameData,
                              settings: &mut GameSettings,
                              msg_log: &mut MsgLog) {
    match input {
        InputAction::Inventory => {
            settings.state = GameState::Playing;
            msg_log.log(Msg::GameState(settings.state));
        }

        InputAction::SelectItem(item_index) => {
            inventory_select_item(item_index, data, settings, msg_log);
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
            player_turn = handle_skill(skill_index, ActionLoc::None, data, settings, msg_log);
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
    let player_turn: Action = Action::NoAction;
    let player_id = data.find_by_name(EntityName::Player).unwrap();

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
                        data.entities.add_skill(player_id, Skill::Blink);
                        msg_log.log(Msg::GameState(GameState::Playing));
                    }

                    EntityClass::Monolith => {
                        data.entities.class[&player_id] = classes[class_index];
                        data.entities.add_skill(player_id, Skill::PassWall);
                        data.entities.add_skill(player_id, Skill::Rubble);
                        data.entities.add_skill(player_id, Skill::Reform);
                        msg_log.log(Msg::GameState(GameState::Playing));
                    }

                    EntityClass::Grass => {
                        data.entities.class[&player_id] = classes[class_index];
                        data.entities.add_skill(player_id, Skill::GrassThrow);
                        data.entities.add_skill(player_id, Skill::GrassBlade);
                        msg_log.log(Msg::GameState(GameState::Playing));
                    }

                    EntityClass::Clockwork => {
                        data.entities.class[&player_id] = classes[class_index];
                        data.entities.add_skill(player_id, Skill::Push);
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
    let player_id = data.find_by_name(EntityName::Player).unwrap();

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

pub fn handle_input(game: &mut Game, input_action: InputAction) -> Action {
    let player_id = game.data.find_by_name(EntityName::Player).unwrap();

    let mut player_turn: Action = Action::none();

    let player_alive = game.data.entities.status[&player_id].alive;

    match (input_action, player_alive) {
        (InputAction::CursorMove(dir), true) => {
            dbg!(input_action);
            let cursor_id = game.data.find_by_name(EntityName::Cursor).unwrap();

            let pos = game.data.entities.pos[&cursor_id];
            game.data.entities.pos[&cursor_id] = add_pos(pos, dir.into_move());
        }

        (InputAction::CursorApply(mode, target), true) => {
            let cursor_id = game.data.find_by_name(EntityName::Cursor).unwrap();

            let cursor_pos = game.data.entities.pos[&cursor_id];
            let player_pos = game.data.entities.pos[&player_id];
            let must_reach = false;
            let traps_block = true;
            let path = game.data.path_between(player_pos, cursor_pos, Reach::single(1), must_reach, traps_block, None);

            // TODO is this needed somewhere to pathfind?
            //let mut action_loc = ActionLoc::None;
            //if path.len() > 1 {
            //    let target_pos = path[1];
            //    action_loc = ActionLoc::Place(target_pos);
            //}

            player_turn = chord(ActionLoc::Place(cursor_pos), ActionStrength::Weak, mode, target, game);
            dbg!(player_turn);
        }

        (InputAction::Chord(dir, strength, mode, target), true) => {
            let loc = dir.map_or(ActionLoc::None, |dir| ActionLoc::Dir(dir));
            player_turn = chord(loc, strength, mode, target, game);
        }

        (InputAction::Pass, true) => {
            player_turn = Action::Pass;
        }

        (InputAction::Move(move_action), true) => {
            dbg!();
            let player_id = game.data.find_by_name(EntityName::Player).unwrap();

            player_turn = handle_move(player_id, move_action, game);
        }

        (InputAction::DropItem, true) => {
            game.settings.inventory_action = InventoryAction::Drop;
            game.settings.state = GameState::Inventory;
            game.msg_log.log(Msg::GameState(game.settings.state));
        }

        (InputAction::Pickup, true) => {
            player_turn = pickup_item(player_id, game);
        }

        (InputAction::MapClick(_map_loc, _map_cell), _) => {
            player_turn = Action::none();
        }

        (InputAction::Yell, true) => {
            player_turn = Action::Yell;
        }

        (InputAction::IncreaseMoveMode, true) => {
            increase_move_mode(player_id, game);
            player_turn = Action::none();
        }

        (InputAction::DecreaseMoveMode, true) => {
            decrease_move_mode(player_id, game);
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
                    let pos = Pos::new(x, y);
                    game.data.map[pos].explored = true;
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
            let god_mode_hp = 10000;
            let player_id = game.data.find_by_name(EntityName::Player).unwrap();
            game.data.entities.fighter[&player_id].hp = god_mode_hp;
            game.data.entities.fighter[&player_id].max_hp = god_mode_hp;
            game.data.entities.energy[&player_id] = 1000;

            // toggle god mode flag
            game.settings.god_mode = !game.settings.god_mode;
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
            player_turn = use_item(player_id, game);
        }

        (_, _) => {
        }
    }

    return player_turn;
}

pub fn handle_move(entity_id: EntityId, move_action: Direction, game: &mut Game) -> Action {
    let mut turn = Action::none();
    
    let player_reach = game.data.entities.movement[&entity_id];
    let maybe_movement = 
        movement::calculate_move(move_action,
                                 player_reach,
                                 entity_id,
                                 &mut game.data);

    if let Some(movement) = maybe_movement {
        turn = Action::Move(movement);
    }

    return turn;
}

pub fn pickup_item(entity_id: EntityId, game: &mut Game) -> Action {
    let mut turn: Action = Action::none();

    let pos = game.data.entities.pos[&entity_id];

    for key in game.data.entities.pos.keys() {
        let is_item = game.data.entities.item.get(key).is_some();
        let is_disarmed_trap =
            game.data.entities.trap.get(key).is_some() &&
            game.data.entities.armed.get(key) == Some(&false);

        if game.data.entities.pos[key] == pos && (is_item || is_disarmed_trap) {
            turn = Action::Pickup(*key);
            break;
        }
    }

    return turn;
}

pub fn decrease_move_mode(entity_id: EntityId, game: &mut Game) {
    game.data.entities.move_mode[&entity_id] =
        game.data.entities.move_mode[&entity_id].decrease();

    game.data.entities.movement[&entity_id] =
        reach_by_mode(game.data.entities.move_mode[&entity_id]);

    game.msg_log.log(Msg::MoveMode(game.data.entities.move_mode[&entity_id]));
}

pub fn increase_move_mode(entity_id: EntityId, game: &mut Game) {
    let holding_shield = game.data.using(entity_id, Item::Shield);
    let holding_hammer = game.data.using(entity_id, Item::Hammer);

    let move_mode = game.data.entities.move_mode.get(&entity_id).expect("Player should have a move mode");
    let new_move_mode = move_mode.increase();

    if new_move_mode == movement::MoveMode::Run && (holding_shield || holding_hammer) {
        game.msg_log.log(Msg::TriedRunWithHeavyEquipment);
    } else {
        game.data.entities.move_mode[&entity_id] = new_move_mode;
        game.data.entities.movement[&entity_id] = reach_by_mode(game.data.entities.move_mode[&entity_id]);

        game.msg_log.log(Msg::MoveMode(new_move_mode));
    }
}

pub fn use_item(entity_id: EntityId, game: &mut Game) -> Action {
    let mut turn: Action = Action::none();

    if game.data.using(entity_id, Item::Hammer) {
        game.settings.state = GameState::Selection;

        let reach = Reach::Horiz(1);
        game.settings.selection =
            Selection::new(SelectionType::WithinReach(reach), SelectionAction::Hammer);

        game.msg_log.log(Msg::GameState(game.settings.state));
    } else if game.data.using(entity_id, Item::Sword) {
        let pos = game.data.entities.pos[&entity_id];
        turn = Action::UseItem(pos);
    }

    return turn;
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
                  game_data: &mut GameData,
                  msg_log: &mut MsgLog) {
    let throw_line = line(start_pos, end_pos);

    // get target position in direction of player click
    let mut end_pos =
        Pos::from(throw_line.into_iter().take(PLAYER_THROW_DIST).last().unwrap());

    if let Some(blocked) = game_data.map.path_blocked_move(start_pos, end_pos) {
        // the start pos of the blocked struct is the last reached position
        end_pos = blocked.start_pos;
    }

    game_data.entities.set_pos(item_id, start_pos);

    let movement = Movement::step_to(end_pos);
    msg_log.log(Msg::Moved(item_id, movement, end_pos));

    game_data.entities.remove_item(player_id, item_id);
}

pub fn place_trap(trap_id: EntityId,
                  place_pos: Pos,
                  game_data: &mut GameData) {
    game_data.entities.set_pos(trap_id, place_pos);
    game_data.entities.armed[&trap_id] = true;
}

pub fn handle_skill(skill_index: usize,
                    action_loc: ActionLoc,
                    data: &mut GameData, 
                    settings: &mut GameSettings, 
                    msg_log: &mut MsgLog) -> Action {
    let player_id = data.find_by_name(EntityName::Player).unwrap();

    // # check if we have enough energy to carry out the skill
    if data.entities.energy[&player_id] <= 0 {
        msg_log.log(Msg::NotEnoughEnergy(player_id));
        return Action::none();
    }

    // get the skill in the player's list of skills
    if skill_index >= data.entities.skills[&player_id].len() {
        return Action::none();
    }

    let mut turn: Action = Action::none();

    let reach = Reach::single(1);

    let mut selection = None;
    // TODO add input to this function for whether a selection has been made.
    // instead of entering selection mode, apply the action immediately
    match data.entities.skills[&player_id][skill_index] {
        Skill::GrassThrow => {
            selection =
                Some(Selection::new(SelectionType::WithinReach(reach), SelectionAction::GrassThrow));
        }

        Skill::GrassBlade => {
            turn = Action::GrassBlade(player_id);
            settings.state = GameState::Playing;
            msg_log.log(Msg::GameState(settings.state));
        }

        Skill::Blink => {
            turn = Action::Blink(player_id);

            settings.state = GameState::Playing;
            msg_log.log(Msg::GameState(settings.state));
        }

        Skill::PassWall => {
            selection =
                Some(Selection::new(SelectionType::WithinReach(reach), SelectionAction::PassWall));
        }

        Skill::Rubble => {
            let reach = Reach::horiz(1);
            selection =
                Some(Selection::new(SelectionType::WithinReach(reach), SelectionAction::Rubble));
        }

        Skill::Reform => {
            selection =
                Some(Selection::new(SelectionType::WithinReach(reach), SelectionAction::Reform));
        }

        Skill::Swap => {
            // NOTE make this a const or config item
            let reach = Reach::single(4);
            selection =
                Some(Selection::new(SelectionType::WithinReach(reach), SelectionAction::Swap));
        }

        Skill::Push => {
            selection =
                Some(Selection::new(SelectionType::WithinReach(reach), SelectionAction::Push));
        }
    }

    dbg!(selection, action_loc);
    if let Some(selection) = selection {
        match action_loc {
            ActionLoc::Place(pos) => {
                turn = selection.action.action_from_pos(pos, data);
                dbg!(turn);
            }

            ActionLoc::Dir(dir) => {
                let player_pos = data.entities.pos[&player_id];
                if let Some(pos) = selection.typ.offset_pos(player_pos, dir) {
                    turn = selection.action.action_from_pos(pos, data);
                    dbg!(turn, dir, pos, player_pos);
                }
            }

            ActionLoc::None => {
                // if no action location is given, enter selection mode
                settings.selection = selection;
                settings.state = GameState::Selection;
                settings.selection.only_visible = false;
                msg_log.log(Msg::GameState(settings.state));

            }
        }
    }
    dbg!(settings.state);

    return turn;
}

pub fn chord(loc: ActionLoc,
             _strength: ActionStrength,
             mode: ActionMode,
             target: i32,
             game: &mut Game) -> Action {
    let mut turn = Action::none();
    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    let player_pos = game.data.entities.pos[&player_id];

    // if no target selection, then it is a move
    if target == -1 {
        dbg!(loc);
        match mode {
            ActionMode::Primary => {
                decrease_move_mode(player_id, game);
            }

            ActionMode::Alternate => {
                increase_move_mode(player_id, game);
            }
        }

        match loc {
            ActionLoc::None => {
                turn = Action::Pass;
            }

            ActionLoc::Dir(direction) => {
                turn = handle_move(player_id, direction, game);
            }

            ActionLoc::Place(pos) => {
                let dxy = sub_pos(pos, player_pos);
                let direction = Direction::from_dxy(dxy.x, dxy.y).unwrap();

                turn = handle_move(player_id, direction, game);
            }

        }
    } else {
        dbg!(loc, target);
        let num_items_in_inventory = game.data.entities.inventory[&player_id].len() as i32;
        if target >= 2 {
            // the minus 2 here comes from the primary and secondary item, after which comes
            // the skills
            let skill_index = (target - 2) as usize;
            if skill_index < game.data.entities.skills[&player_id].len() {
                turn = handle_skill(skill_index, loc, &mut game.data, &mut game.settings, &mut game.msg_log);
                dbg!(&turn);
            }
        } else if target < num_items_in_inventory {
            match mode {
                ActionMode::Primary => {
                    // primary item use is the item's main action
                    turn = use_item(player_id, game);
                }

                ActionMode::Alternate => {
                    // alternate item use means drop or throw item
                    match loc {
                        ActionLoc::None => {
                            inventory_drop_item(target as usize, &mut game.data, &mut game.settings, &mut game.msg_log);
                        }

                        ActionLoc::Place(pos) => {
                            let item_id = game.data.entities.inventory[&player_id][target as usize];
                            turn = Action::ThrowItem(pos, item_id);
                        }

                        ActionLoc::Dir(direction) => {
                            let start = game.data.entities.pos[&player_id];
                            let max_end = direction.offset_pos(start, PLAYER_THROW_DIST as i32);
                            let end = game.data.map.path_blocked_move(start, max_end)
                                                   .map_or(max_end, |b| b.end_pos);
                            let item_id = game.data.entities.inventory[&player_id][target as usize];
                            turn = Action::ThrowItem(end, item_id);
                            dbg!(&turn);
                        }
                    }
                }
            }
        }
    }

    return turn;
}
