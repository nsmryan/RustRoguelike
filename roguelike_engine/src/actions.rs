use std::str::FromStr;
use std::fmt;

use serde::{Serialize, Deserialize};

use roguelike_core::movement::{Direction, Reach, MoveMode, MoveType};
use roguelike_core::types::*;
use roguelike_core::messaging::{Msg, MsgLog};
use roguelike_core::constants::*;
use roguelike_core::config::Config;
use roguelike_core::utils::{scale_pos, distance, sub_pos, add_pos, next_from_to, move_towards};
use roguelike_core::map::{astar_next_pos};

use crate::game::*;
use crate::input::*;
use crate::make_map;



#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionLoc {
    Dir(Direction),
    Place(Pos),
    Facing,
    None
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum InputAction {
    Move(Direction, MoveMode),
    MoveTowardsCursor(MoveMode),
    SkillDir(Direction, ActionMode, usize),
    ItemDir(Direction, ActionMode, usize),
    SkillPos(Pos, ActionMode, usize),
    ItemPos(Pos, ActionMode, usize),
    SkillFacing(ActionMode, usize),
    ItemFacing(ActionMode, usize),
    StartUseItem(usize),
    StartUseDir(Direction),
    FinalizeUse,
    AbortUse,
    Pass(MoveMode),
    Pickup,
    DropItem,
    DropItemByIndex(usize),
    Yell,
    UseItem(Direction, usize),
    Interact(Option<Direction>),
    CursorMove(Direction, bool, bool), // move direction, is relative, is long
    CursorReturn,
    CursorToggle,
    MapClick(Pos, Pos), // map loc, map cell
    MouseButton(MouseClick, KeyDir),
    Inventory,
    SkillMenu,
    ClassMenu,
    Exit,
    Esc,
    ForceExit,
    ExploreAll,
    RegenerateMap,
    GodMode,
    IncreaseMoveMode,
    DecreaseMoveMode,
    OverlayOn,
    OverlayOff,
    SelectEntry(usize),
    None,
}

impl fmt::Display for InputAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputAction::Move(direction, move_mode) => {
                match direction {
                    Direction::Left => write!(f, "left {}", move_mode),
                    Direction::Right => write!(f, "right {}", move_mode),
                    Direction::Up => write!(f, "up {}", move_mode),
                    Direction::Down => write!(f, "down {}", move_mode),
                    Direction::DownLeft => write!(f, "downleft {}", move_mode),
                    Direction::DownRight => write!(f, "downright {}", move_mode),
                    Direction::UpLeft => write!(f, "upleft {}", move_mode),
                    Direction::UpRight => write!(f, "upright {}", move_mode),
                }
            },
            InputAction::MoveTowardsCursor(move_mode) => write!(f, "movetowardscursor {}", move_mode),
            InputAction::SkillDir(dir, action_mode, index) => write!(f, "skilldir {} {} {}", dir, action_mode, index),
            InputAction::ItemDir(dir, action_mode, index) => write!(f, "itemdir {} {} {}", dir, action_mode, index),
            InputAction::SkillPos(pos, action_mode, index) => write!(f, "skillpos {} {} {} {}", pos.x, pos.y, action_mode, index),
            InputAction::ItemPos(pos, action_mode, index) => write!(f, "itempos {} {} {} {}", pos.x, pos.y, action_mode, index),
            InputAction::SkillFacing(action_mode, index) => write!(f, "skill {} {}", action_mode, index),
            InputAction::ItemFacing(action_mode, index) => write!(f, "itemdir {} {}", action_mode, index),
            InputAction::StartUseItem(index) => write!(f, "startuseitem {}", index),
            InputAction::StartUseDir(dir) => write!(f, "startusedir {}", dir),
            InputAction::FinalizeUse => write!(f, "finalizeuse"),
            InputAction::AbortUse => write!(f, "abortuse"),
            InputAction::Pass(move_mode) => write!(f, "pass {}", move_mode),
            InputAction::MapClick(loc, cell) => write!(f, "click {} {} {} {}", loc.x, loc.y, cell.x, cell.y),
            InputAction::MouseButton(click, keydir) => write!(f, "mousebutton {:?} {:?}", click, keydir),
            InputAction::Pickup => write!(f, "pickup"),
            InputAction::DropItem => write!(f, "drop"),
            InputAction::DropItemByIndex(target) => write!(f, "droptarget {}", target),
            InputAction::Inventory => write!(f, "inventory"),
            InputAction::SkillMenu => write!(f, "skill"),
            InputAction::ClassMenu => write!(f, "class"),
            InputAction::Exit => write!(f, "exit"),
            InputAction::Esc => write!(f, "esc"),
            InputAction::ForceExit => write!(f, "force_exit"),
            InputAction::ExploreAll => write!(f, "exploreall"),
            InputAction::RegenerateMap => write!(f, "regenmap"),
            InputAction::GodMode => write!(f, "godmode"),
            InputAction::Yell => write!(f, "yell"),
            InputAction::IncreaseMoveMode => write!(f, "faster"),
            InputAction::DecreaseMoveMode => write!(f, "slower"),
            InputAction::OverlayOn => write!(f, "overlayon"),
            InputAction::OverlayOff => write!(f, "overlayoff"),
            InputAction::SelectEntry(item) => write!(f, "selectentry {}", item),
            InputAction::UseItem(dir, target) => write!(f, "use, {:?} {}", dir, target),
            InputAction::Interact(dir) => write!(f, "interact {:?}", dir),
            InputAction::CursorMove(dir, relative, long) => write!(f, "cursormove {:?} {} {}", dir, relative, long),
            InputAction::CursorReturn => write!(f, "cursorreturn"),
            InputAction::CursorToggle => write!(f, "cursortoggle"),
            InputAction::None => write!(f, "none"),
        }
    }
}

impl FromStr for InputAction {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();
        let args = s.split(" ").collect::<Vec<&str>>();

        if args[0] == "left" {
            let move_mode = args[1].parse::<MoveMode>().unwrap();
            return Ok(InputAction::Move(Direction::Left, move_mode));
        } else if args[0] == "right" {
            let move_mode = args[1].parse::<MoveMode>().unwrap();
            return Ok(InputAction::Move(Direction::Right, move_mode));
        } else if args[0] == "up" {
            let move_mode = args[1].parse::<MoveMode>().unwrap();
            return Ok(InputAction::Move(Direction::Up, move_mode));
        } else if args[0] == "down" {
            let move_mode = args[1].parse::<MoveMode>().unwrap();
            return Ok(InputAction::Move(Direction::Down, move_mode));
        } else if args[0] == "upleft" {
            let move_mode = args[1].parse::<MoveMode>().unwrap();
            return Ok(InputAction::Move(Direction::UpLeft, move_mode));
        } else if args[0] == "upright" {
            let move_mode = args[1].parse::<MoveMode>().unwrap();
            return Ok(InputAction::Move(Direction::UpRight, move_mode));
        } else if args[0] == "downleft" {
            let move_mode = args[1].parse::<MoveMode>().unwrap();
            return Ok(InputAction::Move(Direction::DownLeft, move_mode));
        } else if args[0] == "downright" {
            let move_mode = args[1].parse::<MoveMode>().unwrap();
            return Ok(InputAction::Move(Direction::DownRight, move_mode));
        } else if args[0] == "pass" {
            let move_mode = args[1].parse::<MoveMode>().unwrap();
            return Ok(InputAction::Pass(move_mode));
        } else if args[0] == "movetowardscursor" {
            let move_mode = args[1].parse::<MoveMode>().unwrap();
            return Ok(InputAction::MoveTowardsCursor(move_mode));
        } else if args[0] == "skilldir" {
            let dir = args[1].parse::<Direction>().unwrap();
            let action_mode = args[2].parse::<ActionMode>().unwrap();
            let index = args[3].parse::<usize>().unwrap();
            return Ok(InputAction::SkillDir(dir, action_mode, index));
        } else if args[0] == "itemdir" {
            let dir = args[1].parse::<Direction>().unwrap();
            let action_mode = args[2].parse::<ActionMode>().unwrap();
            let index = args[3].parse::<usize>().unwrap();
            return Ok(InputAction::ItemDir(dir, action_mode, index));
        } else if args[0] == "skillpos" {
            let x = args[1].parse::<i32>().unwrap();
            let y = args[2].parse::<i32>().unwrap();
            let action_mode = args[3].parse::<ActionMode>().unwrap();
            let index = args[4].parse::<usize>().unwrap();
            return Ok(InputAction::SkillPos(Pos::new(x, y), action_mode, index));
        } else if args[0] == "itempos" {
            let x = args[1].parse::<i32>().unwrap();
            let y = args[2].parse::<i32>().unwrap();
            let action_mode = args[3].parse::<ActionMode>().unwrap();
            let index = args[4].parse::<usize>().unwrap();
            return Ok(InputAction::ItemPos(Pos::new(x, y), action_mode, index));
        } else if args[0] == "skillfacing" {
            let action_mode = args[1].parse::<ActionMode>().unwrap();
            let index = args[2].parse::<usize>().unwrap();
            return Ok(InputAction::SkillFacing(action_mode, index));
        } else if args[0] == "itemfacing" {
            let action_mode = args[1].parse::<ActionMode>().unwrap();
            let index = args[2].parse::<usize>().unwrap();
            return Ok(InputAction::ItemFacing(action_mode, index));
        } else if args[0] == "startuseitem" {
            let index = args[1].parse::<usize>().unwrap();
            return Ok(InputAction::StartUseItem(index));
        } else if args[0] == "startusedir" {
            let dir = args[1].parse::<Direction>().unwrap();
            return Ok(InputAction::StartUseDir(dir));
        } else if args[0] == "finalizeuse" {
            return Ok(InputAction::FinalizeUse);
        } else if args[0] == "abortuse" {
            return Ok(InputAction::AbortUse);
        } else if args[0] == "pickup" {
            return Ok(InputAction::Pickup);
        } else if args[0] == "drop" {
            return Ok(InputAction::DropItem);
        } else if args[0] == "droptarget" {
            let target = args[1].parse::<usize>().unwrap();
            return Ok(InputAction::DropItemByIndex(target));
        } else if args[0] == "yell" {
            return Ok(InputAction::Yell);
        } else if args[0] == "inventory" {
            return Ok(InputAction::Inventory);
        } else if args[0] == "use" {
            let direction = args[1].parse::<Direction>().unwrap();
            let target = args[1].parse::<usize>().unwrap();
            return Ok(InputAction::UseItem(direction, target));
        } else if s.starts_with("selectentry") {
            let selection = args[1].parse::<usize>().unwrap();
            return Ok(InputAction::SelectEntry(selection));
        } else if args[0] == "interact" {
            let dir = args[1].parse::<Direction>().ok();
            return Ok(InputAction::Interact(dir));
        } else if args[0] == "godmode" {
            return Ok(InputAction::GodMode);
        } else if s.starts_with("click") {
            let loc_x = args[1].parse::<i32>().unwrap();
            let loc_y = args[2].parse::<i32>().unwrap();
            let cell_x = args[3].parse::<i32>().unwrap();
            let cell_y = args[4].parse::<i32>().unwrap();
            return Ok(InputAction::MapClick(Pos::new(loc_x, loc_y), Pos::new(cell_x, cell_y)));
        } else if args[0] == "skill" {
            return Ok(InputAction::SkillMenu);
        } else if args[0] == "class" {
            return Ok(InputAction::ClassMenu);
        } else if args[0] == "esc" {
            return Ok(InputAction::Esc);
        } else if args[0] == "force_exit" {
            return Ok(InputAction::ForceExit);
        } else if args[0] == "faster" {
            return Ok(InputAction::IncreaseMoveMode);
        } else if args[0] == "slower" {
            return Ok(InputAction::DecreaseMoveMode);
        } else if args[0] == "cursormove" {
            let dir = Direction::from_str(args[1]).unwrap();
            let relative = bool::from_str(args[2]).unwrap();
            let long = bool::from_str(args[3]).unwrap();
            return Ok(InputAction::CursorMove(dir, relative, long));
        } else if args[0] == "cursorreturn" {
            return Ok(InputAction::CursorReturn);
        } else if args[0] == "cursortoggle" {
            return Ok(InputAction::CursorToggle);
        } else {
            return Err(format!("Could not parse '{}' as InputAction", s));
        }
    }
}

pub fn handle_input_universal(input_action: InputAction, game: &mut Game) -> bool {
    match input_action {
        InputAction::ExploreAll => {
            for x in 0..game.data.map.width() {
                for y in 0..game.data.map.height() {
                    let pos = Pos::new(x, y);
                    game.data.map[pos].explored = true;
                }
            }

            return true;
        }

        InputAction::RegenerateMap => {
            let _position = make_map::make_map(&game.config.map_load.clone(), game);
            game.msg_log.log(Msg::NewLevel);
            return true;
        }

        InputAction::GodMode => {
            let god_mode_hp = 10000;
            let player_id = game.data.find_by_name(EntityName::Player).unwrap();
            game.data.entities.fighter[&player_id].hp = god_mode_hp;
            game.data.entities.fighter[&player_id].max_hp = god_mode_hp;
            game.data.entities.energy[&player_id] = 1000;

            // toggle god mode flag
            game.settings.god_mode = !game.settings.god_mode;
            return true;
        }

        InputAction::ForceExit => {
            change_state(&mut game.settings, GameState::Exit);
            return true;
        }

        InputAction::Exit => {
            if game.settings.state != GameState::ConfirmQuit {
                change_state(&mut game.settings, GameState::ConfirmQuit);
                return true;
            } else {
                return false;
            }
        }

        _ => {
            return false;
        }
    }
}

pub fn handle_input_inventory(input: InputAction, settings: &mut GameSettings) {
    match input {
        InputAction::Inventory => {
            change_state(settings, GameState::Playing);
        }

        InputAction::Esc => {
            change_state(settings, GameState::Playing);
        }

        InputAction::SkillMenu => {
            change_state(settings, GameState::SkillMenu);
        }

        InputAction::ClassMenu => {
            change_state(settings, GameState::ClassMenu);
        }

        _ => {
        }
    }
}

pub fn handle_input_skill_menu(input: InputAction,
                               data: &GameData,
                               settings: &mut GameSettings,
                               msg_log: &mut MsgLog,
                               _config: &Config) {
    match input {
        InputAction::Inventory => {
            change_state(settings, GameState::Inventory);
        }

        InputAction::SkillMenu => {
            change_state(settings, GameState::Playing);
        }

        InputAction::ClassMenu => {
            change_state(settings, GameState::ClassMenu);
        }

        InputAction::SelectEntry(skill_index) => {
            handle_skill(skill_index, ActionLoc::None, ActionMode::Primary, data, msg_log);
            change_state(settings, GameState::Playing);
        }

        InputAction::Esc => {
            change_state(settings, GameState::Playing);
        }

        _ => {
        }
    }
}

pub fn handle_input_class_menu(input: InputAction,
                               _data: &GameData,
                               settings: &mut GameSettings,
                               msg_log: &mut MsgLog) {
    match input {
        InputAction::Inventory => {
            change_state(settings, GameState::Inventory);
        }

        InputAction::ClassMenu => {
            change_state(settings, GameState::Playing);
        }

        InputAction::SkillMenu => {
            change_state(settings, GameState::SkillMenu);
        }

        InputAction::SelectEntry(class_index) => {
            let classes = EntityClass::classes();
            if class_index < classes.len() {
                // give player skills from a particular class
                msg_log.log(Msg::AddClass(classes[class_index]));

                change_state(settings, GameState::Playing);
            }
        }

        InputAction::Esc => {
            change_state(settings, GameState::Playing);
        }

        _ => {
        }
    }
}

pub fn handle_input_confirm_quit(input: InputAction, settings: &mut GameSettings) {
    match input {
        InputAction::Esc => {
            change_state(settings, GameState::Playing);
        }

        InputAction::Exit => {
            change_state(settings, GameState::Exit);
        }

        _ => {
        }
    }
}

pub fn handle_input(input_action: InputAction,
                    data: &GameData,
                    settings: &mut GameSettings,
                    msg_log: &mut MsgLog,
                    config: &Config) {
    match settings.state {
        GameState::Playing => {
            handle_input_playing(input_action, data, settings, msg_log, config);
        }

        GameState::Use => {
            handle_input_use(input_action, data, settings, msg_log, config);
        }

        GameState::Win => {
        }

        GameState::Lose => {
        }

        GameState::Inventory => {
            handle_input_inventory(input_action, settings);
        }

        GameState::SkillMenu => {
            handle_input_skill_menu(input_action, data, settings, msg_log, config);
        }

        GameState::ClassMenu => {
            handle_input_class_menu(input_action, data, settings, msg_log);
        }

        GameState::ConfirmQuit => {
            handle_input_confirm_quit(input_action, settings);
        }

        GameState::Exit => {
        }
    }
}

pub fn handle_input_use(input_action: InputAction,
                        data: &GameData,
                        settings: &mut GameSettings,
                        msg_log: &mut MsgLog,
                        config: &Config) {
    let player_id = data.find_by_name(EntityName::Player).unwrap();
    let player_pos = data.entities.pos[&player_id];

    let player_alive = data.entities.status[&player_id].alive;

    match (input_action, player_alive) {
        (InputAction::StartUseItem(item_index), true) => {
            dbg!(item_index);
            start_use_item(item_index, data, settings, msg_log);
        }

        (InputAction::StartUseDir(dir), true) => {
            dbg!(dir);
            start_use_dir(dir, data, settings, msg_log);
        }

        (InputAction::FinalizeUse, true) => {
            dbg!("finalize use");
            // TODO apply item
            change_state(settings, GameState::Playing);

            if let Some(dir) = settings.use_dir {
                if settings.use_index >= 0 {
                    finalize_use_item(settings.use_index, dir, data, settings, msg_log);
                }
            }
        }

        (InputAction::AbortUse, true) => {
            dbg!("abort use");
            // TODO do we need to clear any state here?
            change_state(settings, GameState::Playing);
        }

        (InputAction::Esc, true) => {
            change_state(settings, GameState::Playing);
        }

        (_, _) => {
        }
    }
}

pub fn handle_input_playing(input_action: InputAction,
                            data: &GameData,
                            settings: &mut GameSettings,
                            msg_log: &mut MsgLog,
                            config: &Config) {
    let player_id = data.find_by_name(EntityName::Player).unwrap();
    let player_pos = data.entities.pos[&player_id];

    let player_alive = data.entities.status[&player_id].alive;

    match (input_action, player_alive) {
        (InputAction::Move(direction, move_mode), true) => {
            let move_amount = move_mode.move_amount();
            msg_log.log(Msg::TryMove(player_id, direction, move_amount, move_mode));
        }

        (InputAction::MoveTowardsCursor(move_mode), true) => {
            if let Some(cursor_pos) = settings.cursor {
                let maybe_next_pos = astar_next_pos(&data.map, player_pos, cursor_pos, None, None);
                if let Some(next_pos) = maybe_next_pos {
                    if let Some(direction) = Direction::from_positions(player_pos, next_pos) {
                        let move_amount = move_mode.move_amount();
                        msg_log.log(Msg::TryMove(player_id, direction, move_amount, move_mode));
                    }
                }
            }
        }

        (InputAction::ItemDir(dir, action_mode, item_index), true) => {
            handle_item(item_index, ActionLoc::Dir(dir), action_mode, data, msg_log);
        }

        (InputAction::SkillDir(dir, action_mode, skill_index), true) => {
            handle_skill(skill_index, ActionLoc::Dir(dir), action_mode, data, msg_log);
        }

        (InputAction::ItemPos(pos, action_mode, item_index), true) => {
            handle_item(item_index, ActionLoc::Place(pos), action_mode, data, msg_log);
        }

        (InputAction::SkillPos(pos, action_mode, skill_index), true) => {
            handle_skill(skill_index, ActionLoc::Place(pos), action_mode, data, msg_log);
        }

        (InputAction::ItemFacing(action_mode, item_index), true) => {
            handle_item(item_index, ActionLoc::Facing, action_mode, data, msg_log);
        }

        (InputAction::SkillFacing(action_mode, skill_index), true) => {
            handle_skill(skill_index, ActionLoc::Facing, action_mode, data, msg_log);
        }

        (InputAction::StartUseItem(item_index), true) => {
            dbg!();
            start_use_item(item_index, data, settings, msg_log);
        }

        (InputAction::CursorReturn, _) => {
            if settings.cursor.is_some() {
                settings.cursor = Some(player_pos);
            }
        }

        (InputAction::CursorMove(dir, relative, long), _) => {
            if let Some(cursor_pos) = settings.cursor {
                let dist =
                    if long {
                        config.cursor_long
                    } else {
                        1
                    };

                let dir_move: Pos = scale_pos(dir.into_move(), dist);

                let new_pos;
                if relative {
                    new_pos = add_pos(player_pos, dir_move);
                } else {
                    new_pos = add_pos(cursor_pos, dir_move);
                }

                settings.cursor = Some(data.map.clamp(new_pos));
            }
        }

        (InputAction::CursorToggle, true) => {
            if settings.cursor.is_none() {
                settings.cursor = Some(player_pos);
            } else {
                settings.cursor = None;
            }
        }

        (InputAction::DropItemByIndex(target), true) => {
            if target < data.entities.inventory[&player_id].len() {
                let item_id = data.entities.inventory[&player_id][target];
                msg_log.log(Msg::DropItem(player_id, item_id));
            }
        }

        (InputAction::Pass(move_mode), true) => {
            let direction = data.entities.direction[&player_id];
            msg_log.log(Msg::TryMove(player_id, direction, 0, move_mode));
        }

        (InputAction::DropItem, true) => {
            settings.inventory_action = InventoryAction::Drop;
            change_state(settings, GameState::Inventory);
        }

        (InputAction::Pickup, true) => {
            msg_log.log(Msg::PickUp(player_id));
        }

        // NOTE this should be removeable
        (InputAction::MapClick(_map_loc, _map_cell), _) => {
        }

        (InputAction::Yell, true) => {
            msg_log.log(Msg::Yell(player_id));
        }

        (InputAction::IncreaseMoveMode, true) => {
            msg_log.log(Msg::ChangeMoveMode(player_id, true));
        }

        (InputAction::DecreaseMoveMode, true) => {
            msg_log.log(Msg::ChangeMoveMode(player_id, false));
        }

        (InputAction::OverlayOn, _) => {
            settings.overlay = true;
        }

        (InputAction::OverlayOff, _) => {
            settings.overlay = false;
        }

        (InputAction::Inventory, true) => {
            settings.inventory_action = InventoryAction::Use;
            change_state(settings, GameState::Inventory);
        }

        (InputAction::SkillMenu, true) => {
            change_state(settings, GameState::SkillMenu);
        }

        (InputAction::ClassMenu, true) => {
            change_state(settings, GameState::ClassMenu);
        }

        (InputAction::Interact(dir), _) => {
            let pos = data.entities.pos[&player_id];

            let interact_pos = 
                if let Some(dir) = dir {
                    dir.offset_pos(pos, 1)
                } else {
                    pos
                };

                msg_log.log(Msg::Interact(player_id, interact_pos));
        }

        (_, _) => {
        }
    }
}

fn start_use_dir(dir: Direction, data: &GameData, settings: &mut GameSettings, _msg_log: &mut MsgLog) {
    let player_id = data.find_by_name(EntityName::Player).unwrap();

    if data.calculate_use_move(player_id, settings.use_index as usize, dir).is_some() {
        settings.use_dir = Some(dir);
    }
}

fn finalize_use_item(item_index: i32, dir: Direction, data: &GameData, settings: &mut GameSettings, msg_log: &mut MsgLog) {
    let player_id = data.find_by_name(EntityName::Player).unwrap();

    let item_index = settings.use_index as usize;

    let item_id = data.entities.inventory[&player_id][item_index];

    let item = data.entities.item[&item_id];
    match item {
        Item::Dagger => {
            msg_log.log(Msg::DaggerStab(player_id, item_index, dir));
        }

        Item::Shield => {
            msg_log.log(Msg::ShieldSmash(player_id, item_index, dir));
        }

        Item::Hammer => {
            msg_log.log(Msg::HammerRaise(player_id, item_index, dir));
        }

        Item::Sword => {
            msg_log.log(Msg::SwordStep(player_id, item_index, dir));
        }

        _ => {
            panic!(format!("Tried to use {} in use-mode!", item));
        }
    }
}

fn start_use_item(item_index: usize, data: &GameData, settings: &mut GameSettings, msg_log: &mut MsgLog) {
    let player_id = data.find_by_name(EntityName::Player).unwrap();

    let num_items_in_inventory = data.entities.inventory[&player_id].len();

    if item_index >= num_items_in_inventory {
        return;
    }

    settings.use_index = item_index as i32;

    settings.cursor = None;

    change_state(settings, GameState::Use);

    let item_id = data.entities.inventory[&player_id][item_index as usize];
    msg_log.log(Msg::StartUseItem(item_id));
}

pub fn handle_skill(skill_index: usize,
                    action_loc: ActionLoc,
                    action_mode: ActionMode,
                    data: &GameData, 
                    msg_log: &mut MsgLog) {
    let player_id = data.find_by_name(EntityName::Player).unwrap();

    /* Check for Valid Skill Use */
    // # check if we have enough energy to carry out the skill
    if data.entities.energy[&player_id] <= 0 {
        msg_log.log(Msg::NotEnoughEnergy(player_id));
        return;
    }

    // get the skill in the player's list of skills
    if skill_index >= data.entities.skills[&player_id].len() {
        // NOTE we may want a message indicating that the skill index was invalid
        return;
    }

    let reach = Reach::single(1);

    /* Determine Position Effected */
    // NOTE we may want a message indicating that the skill was invalid
    let skill_pos;
    match action_loc {
        ActionLoc::Dir(dir) => {
            let player_pos = data.entities.pos[&player_id];
            if let Some(pos) = reach.furthest_in_direction(player_pos, dir) {
                skill_pos = pos;
            } else {
                return;
            }
        }

        ActionLoc::Place(pos) => {
            skill_pos = pos;
        }

        ActionLoc::Facing => {
            let dir = data.entities.direction[&player_id];
            let player_pos = data.entities.pos[&player_id];
            if let Some(pos) = reach.furthest_in_direction(player_pos, dir) {
                skill_pos = pos;
            } else {
                return;
            }
        }

        ActionLoc::None => {
            //NOTE this used to return, but now uses current position.
            skill_pos = data.entities.pos[&player_id];
        }
    }

    let player_id = data.find_by_name(EntityName::Player).unwrap();
    let player_pos = data.entities.pos[&player_id];
    let dxy = sub_pos(skill_pos, player_pos);
    let direction: Option<Direction> = Direction::from_dxy(dxy.x, dxy.y);

    /* Carry Out Skill */
    match data.entities.skills[&player_id][skill_index] {
        Skill::GrassThrow => {
            if let Some(direction) = Direction::from_dxy(dxy.x, dxy.y) {
                msg_log.log(Msg::GrassThrow(player_id, direction));
            }
        }

        Skill::GrassBlade => {
            if let Some(direction) = Direction::from_dxy(dxy.x, dxy.y) {
                msg_log.log(Msg::GrassBlade(player_id, action_mode, direction));
            }
        }

        Skill::Blink => {
            msg_log.log(Msg::Blink(player_id));
        }

        Skill::GrassShoes => {
            msg_log.log(Msg::GrassShoes(player_id, action_mode));
        }

        Skill::PassWall => {
            let player_id = data.find_by_name(EntityName::Player).unwrap();
            let player_pos = data.entities.pos[&player_id];

            if distance(player_pos, skill_pos) == 1 {
                let blocked = data.map.path_blocked_move(player_pos, skill_pos);
                
                if let Some(blocked) = blocked {
                    if data.map[blocked.end_pos].block_move {
                        let next = next_from_to(player_pos, blocked.end_pos);
                        if  !data.map[next].block_move {
                            msg_log.log(Msg::PassWall(player_id, next));
                        }
                    } else {
                        msg_log.log(Msg::PassWall(player_id, skill_pos));
                    }
                }
            }
        }

        Skill::Rubble => {
            let player_id = data.find_by_name(EntityName::Player).unwrap();
            let player_pos = data.entities.pos[&player_id];

            if distance(player_pos, skill_pos) == 1 {
                msg_log.log(Msg::Rubble(player_id, skill_pos));
            }
        }

        Skill::Reform => {
            let player_id = data.find_by_name(EntityName::Player).unwrap();
            let player_pos = data.entities.pos[&player_id];

            if distance(player_pos, skill_pos) == 1 {
                msg_log.log(Msg::Reform(player_id, skill_pos));
            }
        }

        Skill::Swap => {
            let player_id = data.find_by_name(EntityName::Player).unwrap();
            if let Some(entity_id) = data.has_blocking_entity(skill_pos) {
                msg_log.log(Msg::Swap(player_id, entity_id));
            }
        }

        Skill::Push => {
            let push_amount = 1;
            if let Some(direction) = direction {
                msg_log.log(Msg::Push(player_id, direction, push_amount));
            }
        }

        Skill::Illuminate => {
            msg_log.log(Msg::Illuminate(player_id, skill_pos, ILLUMINATE_AMOUNT));
        }

        Skill::Heal => {
            msg_log.log(Msg::Heal(player_id, SKILL_HEAL_AMOUNT));
        }

        Skill::FarSight => {
            msg_log.log(Msg::FarSight(player_id, SKILL_FARSIGHT_FOV_AMOUNT));
        }

        Skill::Sprint => {
            if let Some(direction) = direction {
                msg_log.log(Msg::Sprint(player_id, direction, SKILL_SPRINT_AMOUNT));
            }
        }
    }
}

fn handle_move(loc: ActionLoc,
               mode: ActionMode,
               data: &GameData,
               msg_log: &mut MsgLog) {
    let player_id = data.find_by_name(EntityName::Player).unwrap();
    let player_pos = data.entities.pos[&player_id];

    let move_mode;
    match mode {
        ActionMode::Primary => {
            move_mode = MoveMode::Sneak;
        }

        ActionMode::Alternate => {
            move_mode = MoveMode::Run;
        }
    }

    match loc {
        ActionLoc::Place(pos) => {
            if pos == player_pos {
                msg_log.log(Msg::Moved(player_id, MoveType::Pass, pos));
            } else {
                let dxy = sub_pos(pos, player_pos);
                let direction = Direction::from_dxy(dxy.x, dxy.y).unwrap();

                let move_amount = move_mode.move_amount();
                msg_log.log(Msg::TryMove(player_id, direction, move_amount, move_mode));
            }
        }

        ActionLoc::Dir(direction) => {
            let move_amount = move_mode.move_amount();
            msg_log.log(Msg::TryMove(player_id, direction, move_amount, move_mode));
        }

        ActionLoc::Facing => {
            let direction = data.entities.direction[&player_id];
            let move_amount = move_mode.move_amount();
            msg_log.log(Msg::TryMove(player_id, direction, move_amount, move_mode));
        }

        ActionLoc::None => {
            let direction = data.entities.direction[&player_id];
            msg_log.log(Msg::TryMove(player_id, direction, 0, move_mode));
        }
    }
}

fn handle_item(target: usize,
               loc: ActionLoc,
               mode: ActionMode,
               data: &GameData,
               msg_log: &mut MsgLog) {
    let player_id = data.find_by_name(EntityName::Player).unwrap();

    let num_items_in_inventory = data.entities.inventory[&player_id].len();

    if target >= num_items_in_inventory {
        return;
    }

    let item_id = data.entities.inventory[&player_id][target as usize];
    
    match mode {
        ActionMode::Primary => {
            match loc {
                ActionLoc::Dir(dir) => {
                    // primary item use is the item's main action
                    let pos = data.entities.pos[&player_id];
                    let use_pos = dir.offset_pos(pos, 1);
                    msg_log.log(Msg::UseItem(player_id, use_pos, item_id));
                }

                ActionLoc::Place(pos) => {
                    let start_pos = data.entities.pos[&player_id];
                    let use_pos = move_towards(start_pos, pos, 1);
                    msg_log.log(Msg::UseItem(player_id, use_pos, item_id));
                }

                ActionLoc::Facing => {
                    let pos = data.entities.pos[&player_id];
                    let dir = data.entities.direction[&player_id];
                    let use_pos = dir.offset_pos(pos, 1);
                    msg_log.log(Msg::UseItem(player_id, use_pos, item_id));
                }

                _ => panic!("Is this even possible anymore?"),
            }
        }

        ActionMode::Alternate => {
            let player_pos = data.entities.pos[&player_id];

            // alternate item use means drop or throw item
            match loc {
                ActionLoc::None => {
                    msg_log.log(Msg::DropItem(player_id, item_id));
                }

                ActionLoc::Place(pos) => {
                    msg_log.log(Msg::ItemThrow(player_id, item_id, player_pos, pos));
                }

                ActionLoc::Facing => {
                    let start = data.entities.pos[&player_id];
                    let direction = data.entities.direction[&player_id];
                    let max_end = direction.offset_pos(start, PLAYER_THROW_DIST as i32);
                    let end = data.map.path_blocked_move(start, max_end)
                                           .map_or(max_end, |b| b.end_pos);

                    msg_log.log(Msg::ItemThrow(player_id, item_id, player_pos, end));
                }

                ActionLoc::Dir(direction) => {
                    let start = data.entities.pos[&player_id];
                    let max_end = direction.offset_pos(start, PLAYER_THROW_DIST as i32);
                    let end = data.map.path_blocked_move(start, max_end)
                                           .map_or(max_end, |b| b.end_pos);

                    msg_log.log(Msg::ItemThrow(player_id, item_id, player_pos, end));
                }
            }
        }
    }
}

// TODO is this println okay to leave in? seems like it should be in stderr?
fn change_state(settings: &mut GameSettings, new_state: GameState) {
    if new_state != settings.state {
        settings.state = new_state;

        match new_state {
            GameState::Inventory => {
                println!("CONSOLE: Opened Inventory");
            }

            GameState::Playing => {
            }

            GameState::SkillMenu => {
                println!("CONSOLE: Selecting a skill");
            }

            GameState::ClassMenu => {
                println!("CONSOLE: Selecting a class");
            }

            GameState::ConfirmQuit => {
                println!("CONSOLE: Confirm quit");
            }

            GameState::Win => {
                println!("CONSOLE: Won Level!");
            }

            GameState::Lose => {
                println!("CONSOLE: Lost Level!");
            }

            GameState::Exit => {
                println!("CONSOLE: Exiting");
            }

            GameState::Use => {
                println!("CONSOLE: Use");
            }
        }
    }
}

