use std::str::FromStr;
use std::fmt;

use serde::{Serialize, Deserialize};

use roguelike_core::movement::{Direction, Reach, MoveMode};
use roguelike_core::types::*;
use roguelike_core::messaging::{Msg, MsgLog};
use roguelike_core::constants::*;
use roguelike_core::config::Config;
use roguelike_core::utils::{scale_pos, distance, sub_pos, add_pos, next_from_to};
use roguelike_core::map::{astar_next_pos, Surface};

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
    Run,
    Sneak,
    Walk,
    Alt,
    Move(Direction),
    MoveTowardsCursor(),
    SkillPos(Pos, ActionMode, usize),
    SkillFacing(ActionMode, usize),
    StartUseItem(ItemClass),
    StartUseInteract,
    UseDir(Direction),
    FinalizeUse,
    AbortUse,
    Pass,
    ThrowItem(Pos, ItemClass),
    Pickup,
    DropItem,
    Yell,
    Interact(Option<Direction>),
    CursorMove(Direction, bool, bool), // move direction, is relative, is long
    CursorReturn,
    CursorToggle,
    MousePos(Pos),
    MouseButton(MouseClick, KeyDir),
    Inventory,
    SkillMenu,
    ClassMenu,
    Exit,
    Esc,
    ForceExit,
    ExploreAll,
    RegenerateMap,
    TestMode,
    IncreaseMoveMode,
    DecreaseMoveMode,
    OverlayToggle,
    SelectEntry(usize),
    DebugToggle,
    Restart,
    None,
}

impl fmt::Display for InputAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputAction::Run => write!(f, "run"),
            InputAction::Sneak => write!(f, "sneak"),
            InputAction::Walk => write!(f, "walk"),
            InputAction::Alt => write!(f, "alt"),
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
            InputAction::MoveTowardsCursor() => write!(f, "movetowardscursor"),
            InputAction::SkillPos(pos, action_mode, index) => write!(f, "skillpos {} {} {} {}", pos.x, pos.y, action_mode, index),
            InputAction::SkillFacing(action_mode, index) => write!(f, "skill {} {}", action_mode, index),
            InputAction::StartUseItem(item_class) => write!(f, "startuseitem {}", item_class),
            InputAction::StartUseInteract => write!(f, "startuseinteract"),
            InputAction::UseDir(dir) => write!(f, "usedir {}", dir),
            InputAction::FinalizeUse => write!(f, "finalizeuse"),
            InputAction::AbortUse => write!(f, "abortuse"),
            InputAction::Pass => write!(f, "pass"),
            InputAction::ThrowItem(pos, item_class) => write!(f, "throwitem {} {} {}", pos.x, pos.y, item_class),
            InputAction::MousePos(pos) => write!(f, "mousepos {:?} {:?}", pos.x, pos.y),
            InputAction::MouseButton(click, keydir) => write!(f, "mousebutton {:?} {:?}", click, keydir),
            InputAction::Pickup => write!(f, "pickup"),
            InputAction::DropItem => write!(f, "drop"),
            InputAction::Inventory => write!(f, "inventory"),
            InputAction::SkillMenu => write!(f, "skill"),
            InputAction::ClassMenu => write!(f, "class"),
            InputAction::Exit => write!(f, "exit"),
            InputAction::Esc => write!(f, "esc"),
            InputAction::ForceExit => write!(f, "force_exit"),
            InputAction::ExploreAll => write!(f, "exploreall"),
            InputAction::RegenerateMap => write!(f, "regenmap"),
            InputAction::TestMode => write!(f, "testmode"),
            InputAction::Yell => write!(f, "yell"),
            InputAction::IncreaseMoveMode => write!(f, "faster"),
            InputAction::DecreaseMoveMode => write!(f, "slower"),
            InputAction::OverlayToggle => write!(f, "overlaytoggle"),
            InputAction::SelectEntry(item) => write!(f, "selectentry {}", item),
            InputAction::Interact(dir) => write!(f, "interact {:?}", dir),
            InputAction::CursorMove(dir, relative, long) => write!(f, "cursormove {:?} {} {}", dir, relative, long),
            InputAction::CursorReturn => write!(f, "cursorreturn"),
            InputAction::CursorToggle => write!(f, "cursortoggle"),
            InputAction::DebugToggle => write!(f, "debugtoggle"),
            InputAction::Restart => write!(f, "restart"),
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
            return Ok(InputAction::Move(Direction::Left));
        } else if args[0] == "right" {
            return Ok(InputAction::Move(Direction::Right));
        } else if args[0] == "up" {
            return Ok(InputAction::Move(Direction::Up));
        } else if args[0] == "down" {
            return Ok(InputAction::Move(Direction::Down));
        } else if args[0] == "upleft" {
            return Ok(InputAction::Move(Direction::UpLeft));
        } else if args[0] == "upright" {
            return Ok(InputAction::Move(Direction::UpRight));
        } else if args[0] == "downleft" {
            return Ok(InputAction::Move(Direction::DownLeft));
        } else if args[0] == "downright" {
            return Ok(InputAction::Move(Direction::DownRight));
        } else if args[0] == "run" {
            return Ok(InputAction::Run);
        } else if args[0] == "sneak" {
            return Ok(InputAction::Sneak);
        } else if args[0] == "walk" {
            return Ok(InputAction::Walk);
        } else if args[0] == "alt" {
            return Ok(InputAction::Alt);
        } else if args[0] == "pass" {
            return Ok(InputAction::Pass);
        } else if args[0] == "throwitem" {
            let x = args[1].parse::<i32>().unwrap();
            let y = args[1].parse::<i32>().unwrap();
            let item_class = args[1].parse::<ItemClass>().unwrap();
            return Ok(InputAction::ThrowItem(Pos::new(x, y), item_class));
        } else if args[0] == "movetowardscursor" {
            return Ok(InputAction::MoveTowardsCursor());
        } else if args[0] == "skillpos" {
            let x = args[1].parse::<i32>().unwrap();
            let y = args[2].parse::<i32>().unwrap();
            let action_mode = args[3].parse::<ActionMode>().unwrap();
            let index = args[4].parse::<usize>().unwrap();
            return Ok(InputAction::SkillPos(Pos::new(x, y), action_mode, index));
        } else if args[0] == "skillfacing" {
            let action_mode = args[1].parse::<ActionMode>().unwrap();
            let index = args[2].parse::<usize>().unwrap();
            return Ok(InputAction::SkillFacing(action_mode, index));
        } else if args[0] == "startuseitem" {
            let class = args[1].parse::<ItemClass>().unwrap();
            return Ok(InputAction::StartUseItem(class));
        } else if args[0] == "usedir" {
            let dir = args[1].parse::<Direction>().unwrap();
            return Ok(InputAction::UseDir(dir));
        } else if args[0] == "finalizeuse" {
            return Ok(InputAction::FinalizeUse);
        } else if args[0] == "abortuse" {
            return Ok(InputAction::AbortUse);
        } else if args[0] == "pickup" {
            return Ok(InputAction::Pickup);
        } else if args[0] == "drop" {
            return Ok(InputAction::DropItem);
        } else if args[0] == "yell" {
            return Ok(InputAction::Yell);
        } else if args[0] == "inventory" {
            return Ok(InputAction::Inventory);
        } else if s.starts_with("selectentry") {
            let selection = args[1].parse::<usize>().unwrap();
            return Ok(InputAction::SelectEntry(selection));
        } else if args[0] == "interact" {
            let dir = args[1].parse::<Direction>().ok();
            return Ok(InputAction::Interact(dir));
        } else if args[0] == "testmode" {
            return Ok(InputAction::TestMode);
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
        } else if args[0] == "debugtoggle" {
            return Ok(InputAction::DebugToggle);
        } else if args[0] == "restart" {
            return Ok(InputAction::Restart);
        } else {
            return Err(format!("Could not parse '{}' as InputAction", s));
        }
    }
}

/// Handle inputs that are the same regardless of game mode.
/// This function returns whether or not the input was handled here (true),
/// or if it needs to be passes to mode-specific handling code (false).
pub fn handle_input_universal(input_action: InputAction, game: &mut Game) -> bool {
    match input_action {
        InputAction::ExploreAll => {
            for x in 0..game.level.map.width() {
                for y in 0..game.level.map.height() {
                    let pos = Pos::new(x, y);
                    game.level.map[pos].explored = true;
                }
            }

            return true;
        }

        InputAction::RegenerateMap => {
            let _position = make_map::make_map(&game.config.map_load.clone(), game);
            game.msg_log.log_front(Msg::NewLevel);
            return true;
        }

        InputAction::TestMode => {
            // toggle god mode flag
            game.settings.test_mode = !game.settings.test_mode;

            let player_id = game.level.find_by_name(EntityName::Player).unwrap();
            game.level.entities.status[&player_id].test_mode = 
                !game.level.entities.status[&player_id].test_mode;

            game.msg_log.log(Msg::TestMode(game.settings.test_mode));
            
            // this causes FoV information to be emitted.
            game.settings.map_changed = true;

            return true;
        }

        InputAction::ForceExit => {
            change_state(&mut game.settings, GameState::Exit, &mut game.msg_log);
            return true;
        }

        InputAction::Exit => {
            if game.settings.state != GameState::ConfirmQuit {
                change_state(&mut game.settings, GameState::ConfirmQuit, &mut game.msg_log);
                return true;
            } else {
                return false;
            }
        }

        InputAction::DebugToggle => {
            game.settings.debug_enabled = !game.settings.debug_enabled;
            game.msg_log.log(Msg::DebugEnabled(game.settings.debug_enabled));
            return true;
        }

        _ => {
            return false;
        }
    }
}

pub fn handle_input_inventory(input: InputAction, settings: &mut GameSettings, msg_log: &mut MsgLog) {
    match input {
        InputAction::Inventory => {
            change_state(settings, GameState::Playing, msg_log);
        }

        InputAction::Esc => {
            change_state(settings, GameState::Playing, msg_log);
        }

        InputAction::SkillMenu => {
            change_state(settings, GameState::SkillMenu, msg_log);
        }

        InputAction::ClassMenu => {
            change_state(settings, GameState::ClassMenu, msg_log);
        }

        _ => {
        }
    }
}

pub fn handle_input_skill_menu(input: InputAction,
                               level: &Level,
                               settings: &mut GameSettings,
                               msg_log: &mut MsgLog) {
                               
    match input {
        InputAction::Inventory => {
            change_state(settings, GameState::Inventory, msg_log);
        }

        InputAction::SkillMenu => {
            change_state(settings, GameState::Playing, msg_log);
        }

        InputAction::ClassMenu => {
            change_state(settings, GameState::ClassMenu, msg_log);
        }

        InputAction::SelectEntry(skill_index) => {
            handle_skill(skill_index, ActionLoc::None, ActionMode::Primary, level, msg_log);
            change_state(settings, GameState::Playing, msg_log);
        }

        InputAction::Esc => {
            change_state(settings, GameState::Playing, msg_log);
        }

        _ => {
        }
    }
}

pub fn handle_input_class_menu(input: InputAction,
                               settings: &mut GameSettings,
                               msg_log: &mut MsgLog) {
    match input {
        InputAction::Inventory => {
            change_state(settings, GameState::Inventory, msg_log);
        }

        InputAction::ClassMenu => {
            change_state(settings, GameState::Playing, msg_log);
        }

        InputAction::SkillMenu => {
            change_state(settings, GameState::SkillMenu, msg_log);
        }

        InputAction::SelectEntry(class_index) => {
            let classes = EntityClass::classes();
            if class_index < classes.len() {
                // give player skills from a particular class
                msg_log.log(Msg::AddClass(classes[class_index]));

                change_state(settings, GameState::Playing, msg_log);
            }
        }

        InputAction::Esc => {
            change_state(settings, GameState::Playing, msg_log);
        }

        _ => {
        }
    }
}

pub fn handle_input_confirm_quit(input: InputAction, settings: &mut GameSettings, msg_log: &mut MsgLog) {
    match input {
        InputAction::Esc => {
            change_state(settings, GameState::Playing, msg_log);
        }

        InputAction::Exit => {
            change_state(settings, GameState::Exit, msg_log);
        }

        InputAction::Restart => {
            msg_log.log(Msg::Restart);
            change_state(settings, GameState::Playing, msg_log);
        }

        _ => {
        }
    }
}

pub fn handle_input(input_action: InputAction,
                    level: &Level,
                    settings: &mut GameSettings,
                    msg_log: &mut MsgLog,
                    config: &Config) {
    match settings.state {
        GameState::Playing => {
            handle_input_playing(input_action, level, settings, msg_log, config);
        }

        GameState::Use => {
            handle_input_use(input_action, level, settings, msg_log, config);
        }

        GameState::Win => {
        }

        GameState::Lose => {
        }

        GameState::Inventory => {
            handle_input_inventory(input_action, settings, msg_log);
        }

        GameState::SkillMenu => {
            handle_input_skill_menu(input_action, level, settings, msg_log);
        }

        GameState::ClassMenu => {
            handle_input_class_menu(input_action, settings, msg_log);
        }

        GameState::ConfirmQuit => {
            handle_input_confirm_quit(input_action, settings, msg_log);
        }

        GameState::Exit => {
        }
    }
}

pub fn handle_input_use(input_action: InputAction,
                        level: &Level,
                        settings: &mut GameSettings,
                        msg_log: &mut MsgLog,
                        _config: &Config) {
    let player_id = level.find_by_name(EntityName::Player).unwrap();

    let player_alive = level.entities.status[&player_id].alive;

    match (input_action, player_alive) {
        // TODO remove if removing old input system with held state...
        //(InputAction::Run, true) => {
        //    if settings.move_mode == MoveMode::Run {
        //        settings.move_mode = MoveMode::Walk;
        //    } else {
        //        settings.move_mode = MoveMode::Run;
        //    }
        //    msg_log.log(Msg::NextMoveMode(settings.move_mode));
        //}

        //(InputAction::Sneak, true) => {
        //    if settings.move_mode == MoveMode::Sneak {
        //        settings.move_mode = MoveMode::Walk;
        //    } else {
        //        settings.move_mode = MoveMode::Sneak;
        //    }
        //    msg_log.log(Msg::NextMoveMode(settings.move_mode));
        //}
        (InputAction::Run, true) => {
            settings.move_mode = MoveMode::Run;
            msg_log.log(Msg::NextMoveMode(settings.move_mode));
        }

        (InputAction::Sneak, true) => {
            settings.move_mode = MoveMode::Sneak;
            msg_log.log(Msg::NextMoveMode(settings.move_mode));
        }

        (InputAction::Walk, true) => {
            settings.move_mode = MoveMode::Walk;
            msg_log.log(Msg::NextMoveMode(settings.move_mode));
        }

        (InputAction::DropItem, true) => {
            if let UseAction::Item(item_class) = settings.use_action {
                if let Some(item_index) = level.find_item(item_class) {
                    msg_log.log(Msg::DropItem(player_id, item_index as u64));

                    settings.use_dir = None;
                    msg_log.log(Msg::UseDirClear);

                    change_state(settings, GameState::Playing, msg_log);
                }
            }
        }

        (InputAction::StartUseItem(item_class), true) => {
            start_use_item(item_class, level, settings, msg_log);
        }

        (InputAction::UseDir(dir), true) => {
            use_dir(dir, level, settings, msg_log);
        }

        (InputAction::FinalizeUse, true) => {
            change_state(settings, GameState::Playing, msg_log);

            finalize_use_item(level, settings, msg_log);
        }

        (InputAction::AbortUse, true) => {
            settings.use_dir = None;
            msg_log.log(Msg::UseDirClear);

            change_state(settings, GameState::Playing, msg_log);
        }

        (InputAction::Esc, true) => {
            change_state(settings, GameState::Playing, msg_log);
        }

        (InputAction::OverlayToggle, _) => {
            settings.overlay = !settings.overlay;
            msg_log.log(Msg::Overlay(settings.overlay));
        }

        (_, _) => {
        }
    }
}

pub fn handle_input_playing(input_action: InputAction,
                            level: &Level,
                            settings: &mut GameSettings,
                            msg_log: &mut MsgLog,
                            config: &Config) {
    let player_id = level.find_by_name(EntityName::Player).unwrap();
    let player_pos = level.entities.pos[&player_id];

    let player_alive = level.entities.status[&player_id].alive;

    match (input_action, player_alive) {
        // TODO remove if removing old input system with held state...
        //(InputAction::Run, true) => {
        //    if settings.move_mode == MoveMode::Run {
        //        settings.move_mode = MoveMode::Walk;
        //    } else {
        //        settings.move_mode = MoveMode::Run;
        //    }
        //    msg_log.log(Msg::NextMoveMode(settings.move_mode));
        //}

        //(InputAction::Sneak, true) => {
        //    if settings.move_mode == MoveMode::Sneak {
        //        settings.move_mode = MoveMode::Walk;
        //    } else {
        //        settings.move_mode = MoveMode::Sneak;
        //    }
        //    msg_log.log(Msg::NextMoveMode(settings.move_mode));
        //}
        (InputAction::Run, true) => {
            settings.move_mode = MoveMode::Run;
            msg_log.log(Msg::NextMoveMode(settings.move_mode));
        }

        (InputAction::Sneak, true) => {
            settings.move_mode = MoveMode::Sneak;
            msg_log.log(Msg::NextMoveMode(settings.move_mode));
        }

        (InputAction::Walk, true) => {
            settings.move_mode = MoveMode::Walk;
            msg_log.log(Msg::NextMoveMode(settings.move_mode));
        }


        (InputAction::Move(direction), true) => {
            let move_amount = move_amount(settings.move_mode, config);
            //let move_amount = settings.move_mode.move_amount();
            msg_log.log(Msg::TryMove(player_id, direction, move_amount, settings.move_mode));
        }

        (InputAction::MoveTowardsCursor(), true) => {
            if let Some(cursor_pos) = settings.cursor {
                let maybe_next_pos = astar_next_pos(&level.map, player_pos, cursor_pos, None, None);
                if let Some(next_pos) = maybe_next_pos {
                    if let Some(direction) = Direction::from_positions(player_pos, next_pos) {
                        let move_amount = move_amount(settings.move_mode, config);
                        msg_log.log(Msg::TryMove(player_id, direction, move_amount, settings.move_mode));
                    }
                }
            }
        }

        (InputAction::SkillPos(pos, action_mode, skill_index), true) => {
            handle_skill(skill_index, ActionLoc::Place(pos), action_mode, level, msg_log);
        }

        (InputAction::SkillFacing(action_mode, skill_index), true) => {
            handle_skill(skill_index, ActionLoc::Facing, action_mode, level, msg_log);
        }

        (InputAction::StartUseInteract, true) => {
            settings.use_action = UseAction::Interact;
            msg_log.log(Msg::UseAction(settings.use_action));

            settings.use_dir = None;
            msg_log.log(Msg::UseDirClear);

            ensure_leave_cursor(settings, msg_log);
            change_state(settings, GameState::Use, msg_log);
            msg_log.log(Msg::StartUseInteract);
        }

        (InputAction::StartUseItem(item_class), true) => {
            start_use_item(item_class, level, settings, msg_log);
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
                        config.cursor_fast_move_dist
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

                let new_pos = level.map.clamp(new_pos);
                settings.cursor = Some(new_pos);
                msg_log.log(Msg::CursorMove(new_pos));
            }
        }

        (InputAction::CursorToggle, true) => {
            let cursor_pos;
            if let Some(current_pos) = settings.cursor {
                cursor_pos = current_pos;
                settings.cursor = None;
            } else {
                settings.cursor = Some(player_pos);
                cursor_pos = player_pos;
            }
            msg_log.log(Msg::CursorState(settings.cursor.is_some(), cursor_pos));
        }

        (InputAction::Pass, true) => {
            let direction = level.entities.direction[&player_id];
            msg_log.log(Msg::TryMove(player_id, direction, 0, settings.move_mode));
        }

        (InputAction::ThrowItem(throw_pos, item_class), true) => {
            if let Some(item_index) = level.find_item(item_class) { 
                let player_pos = level.entities.pos[&player_id];
                let item_id = level.entities.inventory[&player_id][item_index];
                msg_log.log(Msg::ItemThrow(player_id, item_id, player_pos, throw_pos, false));
            }
        }

        (InputAction::Pickup, true) => {
            msg_log.log(Msg::PickUp(player_id));
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

        (InputAction::OverlayToggle, _) => {
            settings.overlay = !settings.overlay;
            msg_log.log(Msg::Overlay(settings.overlay));
        }

        (InputAction::Inventory, true) => {
            change_state(settings, GameState::Inventory, msg_log);
        }

        (InputAction::SkillMenu, true) => {
            change_state(settings, GameState::SkillMenu, msg_log);
        }

        (InputAction::ClassMenu, true) => {
            change_state(settings, GameState::ClassMenu, msg_log);
        }

        (InputAction::Interact(dir), _) => {
            let pos = level.entities.pos[&player_id];

            let interact_pos = 
                if let Some(dir) = dir {
                    dir.offset_pos(pos, 1)
                } else {
                    pos
                };

                msg_log.log(Msg::Interact(player_id, interact_pos));
        }

        (InputAction::Esc, true) => {
            ensure_leave_cursor(settings, msg_log);
        }

        (_, _) => {
        }
    }
}

fn ensure_leave_cursor(settings: &mut GameSettings, msg_log: &mut MsgLog) {
    if let Some(pos) = settings.cursor {
        msg_log.log(Msg::CursorState(false, pos));
        settings.cursor = None;
    }
}

fn use_dir(dir: Direction, level: &Level, settings: &mut GameSettings, msg_log: &mut MsgLog) {
    let player_id = level.find_by_name(EntityName::Player).unwrap();

    if let UseAction::Item(item_class) = settings.use_action {
        if let Some(item_index) = level.find_item(item_class) {
            let use_result = level.calculate_use_move(player_id, item_index as usize, dir, settings.move_mode);

            msg_log.log(Msg::UseDirClear);
            if let Some(use_pos) = use_result.pos {
                settings.use_dir = Some(dir);
                msg_log.log(Msg::UseDir(dir));
                msg_log.log(Msg::UsePos(use_pos));
            }

            msg_log.log(Msg::UseHitPosClear);
            for pos in use_result.hit_positions.iter() {
                msg_log.log(Msg::UseHitPos(*pos));
            }
        }
    } else if settings.use_action == UseAction::Interact {
        settings.use_dir = Some(dir);
        msg_log.log(Msg::UseDir(dir));
    } else {
        panic!("Using an item, but no such item in inventory!");
    }
}

fn finalize_use_item(level: &Level, settings: &mut GameSettings, msg_log: &mut MsgLog) {
    let player_id = level.find_by_name(EntityName::Player).unwrap();
    let player_pos = level.entities.pos[&player_id];

    // If there is no direction, the user tried an invalid movement.
    // Returning here will just end use-mode.
    if settings.use_dir.is_none() {
        return;
    }

    if let UseAction::Item(item_class) = settings.use_action {
        // NOTE there should be no way to get here without a direction
        let dir = settings.use_dir.expect("Finalizing use mode for an item with no direction to take!");

        if let Some(item_index) = level.find_item(item_class) {
            let item_id = level.entities.inventory[&player_id][item_index];
            let item = level.entities.item[&item_id];

            let use_result = level.calculate_use_move(player_id, item_index, dir, settings.move_mode);

            // determine action to take based on weapon type
            if item == Item::Hammer {
                msg_log.log(Msg::HammerRaise(player_id, item_index, dir));
            } else if item == Item::SpikeTrap || item == Item::SoundTrap || item == Item::BlinkTrap || item == Item::FreezeTrap {
                let place_pos = dir.offset_pos(player_pos, 1);
                msg_log.log(Msg::PlaceTrap(player_id, place_pos, item_id));
            } else if item == Item::Stone || item == Item::Lantern || 
                      item == Item::SeedOfStone ||item == Item::SeedCache || 
                      item == Item::Herb || item == Item::GlassEye ||
                      item == Item::SmokeBomb || item == Item::LookingGlass ||
                      item == Item::Thumper {
                let throw_pos = dir.offset_pos(player_pos, PLAYER_THROW_DIST as i32);
                msg_log.log(Msg::ItemThrow(player_id, item_id, player_pos, throw_pos, false));
            } else if item == Item::Sling {
                let throw_pos = dir.offset_pos(player_pos, SLING_THROW_DIST as i32);
                if let Some(stone_id) = level.has_item_in_inventory(player_id, Item::Stone) {
                    msg_log.log(Msg::ItemThrow(player_id, stone_id, player_pos, throw_pos, true));
                }
            } else {
                // we should not be able to finalize use mode without a valid move position.
                let move_pos = use_result.pos.expect("Using an item with no move position?!");
                let player_pos = level.entities.pos[&player_id];
                if move_pos != player_pos {
                    let move_dir = Direction::from_positions(player_pos, move_pos).unwrap();
                    let dist = distance(move_pos, player_pos) as usize;
                    msg_log.log(Msg::TryMove(player_id, move_dir, dist, settings.move_mode));
                }

                let weapon_type = item.weapon_type().expect("This item does not have a weapon type!");
                let mut attack_type = AttackStyle::Normal;
                if item == Item::Spear && settings.move_mode == MoveMode::Run {
                    attack_type = AttackStyle::Strong;
                } else if item == Item::Dagger {
                    attack_type = AttackStyle::Stealth;
                }

                for hit_pos in use_result.hit_positions {
                    msg_log.log(Msg::Hit(player_id, hit_pos, weapon_type, attack_type));
                }
            }
        }
    } else if settings.use_action == UseAction::Interact {
        if let Some(dir) = settings.use_dir {
            let target_pos = dir.offset_pos(player_pos, 1);
            if let Some(item_id) = level.item_at_pos(target_pos) {
                if level.entities.trap.get(&item_id).is_some() {
                    // if there is a trap, interact with it
                    msg_log.log(Msg::Interact(player_id, target_pos));
                } else {
                    // move to the item and pick it up
                    msg_log.log(Msg::TryMove(player_id, dir, 1, MoveMode::Walk));
                    msg_log.log(Msg::PickUp(player_id));
                }
            } else {
                // if there is no item, just try to interact
                msg_log.log(Msg::Interact(player_id, target_pos));
            }
        } else {
            // if there is no direction, apply to current tile
            msg_log.log(Msg::PickUp(player_id));
        }
    } else {
        panic!("Using an item, but no such item in inventory!");
    }
}

fn start_use_item(item_class: ItemClass, level: &Level, settings: &mut GameSettings, msg_log: &mut MsgLog) {
    let player_id = level.find_by_name(EntityName::Player).unwrap();

    ensure_leave_cursor(settings, msg_log);

    //if let Some(item_index) = level.entities.item_by_class(player_id, item_class) {
    if let Some(item_index) = level.find_item(item_class) {
        let item_id = level.entities.inventory[&player_id][item_index as usize];

        if level.entities.item[&item_id] == Item::Herb {
            msg_log.log(Msg::EatHerb(player_id, item_id));
        } else {
            // Allow entering use-mode even if there are no places to move
            // in case the player wants to check pressing shift or ctrl
            // for additional spaces.
            settings.use_action = UseAction::Item(item_class);
            msg_log.log(Msg::UseAction(settings.use_action));

            settings.use_dir = None;
            msg_log.log(Msg::UseDirClear);

            for dir in Direction::move_actions().iter() {
                let use_result = level.calculate_use_move(player_id,
                                                         item_index,
                                                         *dir,
                                                         settings.move_mode);
                if let Some(hit_pos) = use_result.pos {
                    msg_log.log(Msg::UseHitPos(hit_pos));
                    msg_log.log(Msg::UseOption(hit_pos, *dir));
                }
            }

            change_state(settings, GameState::Use, msg_log);

            msg_log.log(Msg::StartUseItem(item_id));
        }
    }
}

pub fn handle_skill(skill_index: usize,
                    action_loc: ActionLoc,
                    action_mode: ActionMode,
                    level: &Level, 
                    msg_log: &mut MsgLog) {
    let player_id = level.find_by_name(EntityName::Player).unwrap();

    /* Check for Valid Skill Use */
    // # check if we have enough energy to carry out the skill
    if level.entities.energy[&player_id] <= 0 {
        msg_log.log(Msg::NotEnoughEnergy(player_id));
        return;
    }

    // get the skill in the player's list of skills
    if skill_index >= level.entities.skills[&player_id].len() {
        // NOTE we may want a message indicating that the skill index was invalid
        return;
    }

    let reach = Reach::single(1);

    /* Determine Position Effected */
    // NOTE we may want a message indicating that the skill was invalid
    let skill_pos;
    match action_loc {
        ActionLoc::Dir(dir) => {
            let player_pos = level.entities.pos[&player_id];
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
            let dir = level.entities.direction[&player_id];
            let player_pos = level.entities.pos[&player_id];
            if let Some(pos) = reach.furthest_in_direction(player_pos, dir) {
                skill_pos = pos;
            } else {
                return;
            }
        }

        ActionLoc::None => {
            //NOTE this used to return, but now uses current position.
            skill_pos = level.entities.pos[&player_id];
        }
    }

    let player_id = level.find_by_name(EntityName::Player).unwrap();
    let player_pos = level.entities.pos[&player_id];
    let dxy = sub_pos(skill_pos, player_pos);
    let direction: Option<Direction> = Direction::from_dxy(dxy.x, dxy.y);

    /* Carry Out Skill */
    match level.entities.skills[&player_id][skill_index] {
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

        Skill::GrassWall => {
            if let Some(direction) = Direction::from_dxy(dxy.x, dxy.y) {
                msg_log.log(Msg::GrassWall(player_id, direction));
            }
        }

        Skill::GrassCover => {
            msg_log.log(Msg::GrassCover(player_id, action_mode));
        }

        Skill::PassWall => {
            let player_id = level.find_by_name(EntityName::Player).unwrap();
            let player_pos = level.entities.pos[&player_id];

            if let Some(dir) = Direction::from_positions(player_pos, skill_pos) {
                let target_pos = dir.offset_pos(player_pos, 1);

                let blocked = level.map.path_blocked_move(player_pos, target_pos);
                
                if let Some(blocked) = blocked {
                    if level.map[blocked.end_pos].block_move {
                        let next = next_from_to(player_pos, blocked.end_pos);
                        if  !level.map[next].block_move {
                            msg_log.log(Msg::PassWall(player_id, next));
                        }
                    } else {
                        msg_log.log(Msg::PassWall(player_id, skill_pos));
                    }
                }
            }
        }

        Skill::Rubble => {
            let player_id = level.find_by_name(EntityName::Player).unwrap();
            let player_pos = level.entities.pos[&player_id];

            if distance(player_pos, skill_pos) == 1 {
                msg_log.log(Msg::Rubble(player_id, skill_pos));
            }
        }

        Skill::Reform => {
            let player_id = level.find_by_name(EntityName::Player).unwrap();
            let player_pos = level.entities.pos[&player_id];

            if distance(player_pos, skill_pos) == 1 {
                msg_log.log(Msg::Reform(player_id, skill_pos));
            }
        }

        Skill::StoneThrow => {
            let player_pos = level.entities.pos[&player_id];
            let mut near_rubble = level.map[player_pos].surface == Surface::Rubble;
            for pos in level.map.neighbors(player_pos) {
                if level.map[pos].surface == Surface::Rubble {
                    near_rubble = true;
                }
                if near_rubble {
                    break;
                }
            }

            if let Some(dir) = Direction::from_positions(player_pos, skill_pos) {
                let target_pos = dir.offset_pos(player_pos, 1);

                msg_log.log(Msg::StoneThrow(player_id, target_pos));
            }
        }

        Skill::StoneSkin => {
            let player_id = level.find_by_name(EntityName::Player).unwrap();
            msg_log.log(Msg::StoneSkin(player_id));
        }

        Skill::Swap => {
            let player_id = level.find_by_name(EntityName::Player).unwrap();
            if let Some(entity_id) = level.has_blocking_entity(skill_pos) {
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

        Skill::Ping => {
            msg_log.log(Msg::Ping(player_id, skill_pos));
        }

        Skill::Heal => {
            msg_log.log(Msg::HealSkill(player_id, SKILL_HEAL_AMOUNT));
        }

        Skill::FarSight => {
            msg_log.log(Msg::FarSight(player_id, SKILL_FARSIGHT_FOV_AMOUNT));
        }

        Skill::Sprint => {
            if let Some(direction) = direction {
                msg_log.log(Msg::Sprint(player_id, direction, SKILL_SPRINT_AMOUNT));
            }
        }

        Skill::Roll => {
            if let Some(direction) = direction {
                msg_log.log(Msg::Roll(player_id, direction, SKILL_ROLL_AMOUNT));
            }
        }

        Skill::PassThrough => {
            if let Some(direction) = direction {
                msg_log.log(Msg::PassThrough(player_id, direction));
            }
        }

        Skill::WhirlWind => {
            if level.map.is_within_bounds(skill_pos) {
                msg_log.log(Msg::WhirlWind(player_id, skill_pos));
            }
        }

        Skill::Swift => {
            if let Some(direction) = direction {
                msg_log.log(Msg::Swift(player_id, direction));
            }
        }
    }
}

// TODO is this println okay to leave in? seems like it should be in stderr?
fn change_state(settings: &mut GameSettings, new_state: GameState, msg_log: &mut MsgLog) {
    if new_state != settings.state {
        settings.state = new_state;

        msg_log.log(Msg::GameState(new_state));

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

fn move_amount(move_mode: MoveMode, config: &Config) -> usize {
    match move_mode {
        MoveMode::Sneak => return config.move_tiles_sneak,
        MoveMode::Walk => return config.move_tiles_walk,
        MoveMode::Run => return config.move_tiles_run,
    }
}
