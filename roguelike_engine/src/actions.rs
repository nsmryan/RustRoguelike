use parse_display::{Display, FromStr};

use serde::{Serialize, Deserialize};

use roguelike_utils::math::*;

use roguelike_map::*;

use roguelike_core::movement::{Reach, MoveMode};
use roguelike_core::types::*;
use roguelike_core::messaging::{Msg, InfoMsg, MsgLog};
use roguelike_core::constants::*;
use roguelike_core::config::Config;
use roguelike_core::level::*;

use crate::game::*;
use crate::input::*;
use crate::map_construct::map_construct;


#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionLoc {
    Dir(Direction),
    Place(Pos),
    Facing,
    None
}

#[derive(Clone, Copy, Debug, PartialEq, Display, FromStr, Serialize, Deserialize)]
#[display(style = "lowercase")]
pub enum InputAction {
    Run,
    Sneak,
    Walk,
    Alt,
    #[display("{0}")]
    Move(Direction),
    MoveTowardsCursor,
    #[display("skillpos {0} {1} {2}")]
    SkillPos(Pos, ActionMode, usize),
    #[display("skill {0} {1}")]
    SkillFacing(ActionMode, usize),
    #[display("startuseitem {0}")]
    StartUseItem(ItemClass),
    #[display("startuseskill {0} {1}")]
    StartUseSkill(usize, ActionMode),
    #[display("startusetalent {0}")]
    StartUseTalent(usize),
    #[display("usedir {0}")]
    UseDir(Direction),
    FinalizeUse,
    AbortUse,
    Pass,
    #[display("throwitem {0} {1}")]
    ThrowItem(Pos, ItemClass),
    Pickup,
    DropItem,
    Yell,
    #[display("cursormove {0} {1} {2}")]
    CursorMove(Direction, bool, bool), // move direction, is relative, is long
    CursorReturn,
    CursorToggle,
    #[display("mousepos {0}")]
    MousePos(Pos),
    #[display("mousebutton {0} {1}")]
    MouseButton(MouseClick, KeyDir),
    Inventory,
    SkillMenu,
    ClassMenu,
    HelpMenu,
    Exit,
    Esc,
    ForceExit,
    ExploreAll,
    RegenerateMap,
    TestMode,
    OverlayToggle,
    #[display("selectentry {0}")]
    SelectEntry(usize),
    DebugToggle,
    Restart,
    None,
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
            let _position = map_construct(&game.config.map_load.clone(), game);
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

pub fn handle_input_inventory(input: InputAction, settings: &mut Settings, msg_log: &mut MsgLog) {
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

        InputAction::HelpMenu => {
            change_state(settings, GameState::HelpMenu, msg_log);
        }

        _ => {
        }
    }
}

pub fn handle_input_skill_menu(input: InputAction,
                               level: &Level,
                               settings: &mut Settings,
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

        InputAction::HelpMenu => {
            change_state(settings, GameState::HelpMenu, msg_log);
        }

        InputAction::SelectEntry(skill_index) => {
            handle_skill_index(skill_index, ActionLoc::None, ActionMode::Primary, level, settings, msg_log);
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
                               settings: &mut Settings,
                               msg_log: &mut MsgLog) {
    match input {
        InputAction::Inventory => {
            change_state(settings, GameState::Inventory, msg_log);
        }

        InputAction::ClassMenu => {
            change_state(settings, GameState::Playing, msg_log);
        }

        InputAction::HelpMenu => {
            change_state(settings, GameState::HelpMenu, msg_log);
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

pub fn handle_input_help_menu(input: InputAction,
                              settings: &mut Settings,
                              msg_log: &mut MsgLog) {
    match input {
        InputAction::Inventory => {
            change_state(settings, GameState::Inventory, msg_log);
        }

        InputAction::ClassMenu => {
            change_state(settings, GameState::Playing, msg_log);
        }

        InputAction::HelpMenu => {
            change_state(settings, GameState::Playing, msg_log);
        }

        InputAction::SkillMenu => {
            change_state(settings, GameState::SkillMenu, msg_log);
        }

        InputAction::Esc => {
            change_state(settings, GameState::Playing, msg_log);
        }

        _ => {
        }
    }
}

pub fn handle_input_confirm_quit(input: InputAction, settings: &mut Settings, msg_log: &mut MsgLog) {
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
                    settings: &mut Settings,
                    msg_log: &mut MsgLog,
                    config: &Config) {
    match settings.state {
        GameState::Playing => {
            handle_input_playing(input_action, level, settings, msg_log, config);
        }

        GameState::Use => {
            handle_input_use(input_action, level, settings, msg_log, config);
        }

        GameState::Win | GameState::Lose => {
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

        GameState::HelpMenu => {
            handle_input_help_menu(input_action, settings, msg_log);
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
                        settings: &mut Settings,
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
                    msg_log.log_info(InfoMsg::UseDirClear);

                    change_state(settings, GameState::Playing, msg_log);
                }
            }
        }

        (InputAction::StartUseItem(item_class), true) => {
            start_use_item(item_class, level, settings, msg_log);
        }

        (InputAction::StartUseSkill(index, action_mode), true) => {
            start_use_skill(index, action_mode, level, settings, msg_log);
        }

        (InputAction::StartUseTalent(index), true) => {
            start_use_talent(index, level, settings, msg_log);
        }

        (InputAction::UseDir(dir), true) => {
            use_dir(dir, level, settings, msg_log);
        }

        (InputAction::FinalizeUse, true) => {
            change_state(settings, GameState::Playing, msg_log);

            finalize_use(level, settings, msg_log);
        }

        (InputAction::AbortUse, true) => {
            settings.use_dir = None;
            msg_log.log_info(InfoMsg::UseDirClear);

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
                            settings: &mut Settings,
                            msg_log: &mut MsgLog,
                            config: &Config) {
    let player_id = level.find_by_name(EntityName::Player).unwrap();
    let player_pos = level.entities.pos[&player_id];

    let player_alive = level.entities.status[&player_id].alive;

    match (input_action, player_alive) {
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
            msg_log.log(Msg::TryMove(player_id, direction, move_amount, settings.move_mode));
        }

        (InputAction::MoveTowardsCursor, true) => {
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
            handle_skill_index(skill_index, ActionLoc::Place(pos), action_mode, level, settings, msg_log);
        }

        (InputAction::SkillFacing(action_mode, skill_index), true) => {
            handle_skill_index(skill_index, ActionLoc::Facing, action_mode, level, settings, msg_log);
        }


        (InputAction::StartUseItem(item_class), true) => {
            start_use_item(item_class, level, settings, msg_log);
        }

        (InputAction::StartUseSkill(index, action_mode), true) => {
            start_use_skill(index, action_mode, level, settings, msg_log);
        }

        (InputAction::StartUseTalent(index), true) => {
            start_use_talent(index, level, settings, msg_log);
        }

        (InputAction::CursorReturn, _) => {
            if settings.cursor.is_some() {
                settings.cursor = Some(player_pos);
                msg_log.log(Msg::CursorMove(player_pos));
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
            let entering_cursor_mode = settings.cursor.is_some();
            msg_log.log(Msg::CursorState(entering_cursor_mode, cursor_pos));

            // If exiting cursor mode, and there is a registered UseAction, apply it.
            if !entering_cursor_mode {
                match settings.cursor_action {
                    Some(UseAction::Skill(skill, action_mode)) => {
                        handle_skill(skill,
                                     ActionLoc::Place(cursor_pos),
                                     action_mode,
                                     level,
                                     settings,
                                     msg_log);
                    }

                    Some(UseAction::Item(item_class)) => {
                        if let Some(item_index) = level.find_item(item_class) { 
                            let player_pos = level.entities.pos[&player_id];
                            let item_id = level.entities.inventory[&player_id][item_index];
                            let throw_pos = cursor_pos;

                            // Throwing to the current tile does nothing.
                            if player_pos != throw_pos {
                                msg_log.log(Msg::ItemThrow(player_id, item_id, player_pos, throw_pos, false));
                            }
                        } else {
                            panic!("Throwing an item, but no item available of that type!");
                        }
                    }

                    None => {},

                    _ => {
                        panic!("Cursor mode are not supported for this action!");
                    }
                }
            }
        }

        (InputAction::Pass, true) => {
            let direction = level.entities.direction[&player_id];
            msg_log.log(Msg::TryMove(player_id, direction, 0, settings.move_mode));
        }

        (InputAction::ThrowItem(_throw_pos, item_class), true) => {
            handle_throw_item(item_class, level, msg_log, settings);
        }

        (InputAction::Pickup, true) => {
            msg_log.log(Msg::PickUp(player_id));
        }

        (InputAction::Yell, true) => {
            msg_log.log(Msg::Yell(player_id));
        }

        (InputAction::OverlayToggle, _) => {
            settings.overlay = !settings.overlay;
            msg_log.log(Msg::Overlay(settings.overlay));
        }

        (InputAction::Inventory, _) => {
            change_state(settings, GameState::Inventory, msg_log);
        }

        (InputAction::SkillMenu, _) => {
            change_state(settings, GameState::SkillMenu, msg_log);
        }

        (InputAction::ClassMenu, _) => {
            change_state(settings, GameState::ClassMenu, msg_log);
        }

        (InputAction::HelpMenu, _) => {
            change_state(settings, GameState::HelpMenu, msg_log);
        }

        (InputAction::Esc, _) => {
            if settings.cursor.is_none() {
                change_state(settings, GameState::ConfirmQuit, msg_log);
            }

            ensure_leave_cursor(settings, msg_log);
        }

        (_, _) => {
        }
    }
}

fn handle_throw_item(item_class: ItemClass, level: &Level, msg_log: &mut MsgLog, settings: &mut Settings) {
    if let Some(_item_index) = level.find_item(item_class) { 
        if settings.cursor == None {
            let player_id = level.find_by_name(EntityName::Player).unwrap();
            let player_pos = level.entities.pos[&player_id];
            settings.cursor = Some(player_pos);
            msg_log.log(Msg::CursorState(true, player_pos));
        }

        // Record skill as a use_action.
        let use_action = UseAction::Item(item_class);
        settings.cursor_action = Some(use_action);
        msg_log.log(Msg::CursorAction(use_action));
    }
}

fn ensure_leave_cursor(settings: &mut Settings, msg_log: &mut MsgLog) {
    if let Some(pos) = settings.cursor {
        msg_log.log(Msg::CursorState(false, pos));
        settings.cursor = None;
        settings.cursor_action = None;
    }
}

fn log_use_result_messages(use_result: UseResult, dir: Direction, settings: &mut Settings, msg_log: &mut MsgLog) {
    msg_log.log_info(InfoMsg::UseDirClear);

    if let Some(use_pos) = use_result.pos {
        msg_log.log_info(InfoMsg::UseDir(dir));
        msg_log.log_info(InfoMsg::UsePos(use_pos));
        settings.use_dir = Some(dir);
    }

    msg_log.log_info(InfoMsg::UseHitPosClear);
    for pos in use_result.hit_positions.iter() {
        msg_log.log_info(InfoMsg::UseHitPos(*pos));
    }
}

fn use_dir(dir: Direction, level: &Level, settings: &mut Settings, msg_log: &mut MsgLog) {
    let player_id = level.find_by_name(EntityName::Player).unwrap();

    let use_action = settings.use_action;
    if let UseAction::Item(item_class) = use_action {
        if let Some(item_index) = level.find_item(item_class) {
            let use_result = level.calculate_use_item(player_id, item_index as usize, dir, settings.move_mode);
            log_use_result_messages(use_result, dir, settings, msg_log);
        }
    } else if let UseAction::Skill(skill, _action_mode) = use_action {
        let use_result = level.calculate_use_skill(player_id, skill, dir, settings.move_mode);
        log_use_result_messages(use_result, dir, settings, msg_log);
    }
}

fn finalize_use_skill(skill: Skill, action_mode: ActionMode, level: &Level, settings: &mut Settings, msg_log: &mut MsgLog) {
    let player_id = level.find_by_name(EntityName::Player).unwrap();

    let dir = settings.use_dir.expect("Finalizing use mode for an skill with no direction to take!");
    let use_result = level.calculate_use_skill(player_id, skill, dir, settings.move_mode);

    if use_result.hit_positions.len() > 0 {
        let hit_pos = use_result.hit_positions[0];
        handle_skill(skill, ActionLoc::Place(hit_pos), action_mode, level, settings, msg_log);
    }
}

fn use_item_throwable(item: Item) -> bool {
    return item == Item::Stone || item == Item::Lantern || item == Item::SeedOfStone || item == Item::SeedCache || 
           item == Item::Herb || item == Item::GlassEye || item == Item::SmokeBomb || item == Item::LookingGlass || 
           item == Item::Thumper;
}

fn finalize_use_item(item_class: ItemClass, level: &Level, settings: &mut Settings, msg_log: &mut MsgLog) {
    let player_id = level.find_by_name(EntityName::Player).unwrap();
    let player_pos = level.entities.pos[&player_id];

    if let Some(item_index) = level.find_item(item_class) {
        let item_id = level.entities.inventory[&player_id][item_index];
        let item = level.entities.item[&item_id];

        // NOTE there should be no way to get here without a direction
        let dir = settings.use_dir.expect("Finalizing use mode for an item with no direction to take!");

        let use_result = level.calculate_use_item(player_id, item_index, dir, settings.move_mode);

        // determine action to take based on weapon type
        if item == Item::Hammer {
            if level.entities.has_enough_stamina(player_id, 1) {
                // Stamina is used on hammer strike
                msg_log.log(Msg::HammerRaise(player_id, item_index, dir));
            } else {
                msg_log.log(Msg::NotEnoughStamina(player_id));
            }
        } else if item == Item::SpikeTrap || item == Item::SoundTrap || item == Item::BlinkTrap || item == Item::FreezeTrap {
            let place_pos = dir.offset_pos(player_pos, 1);
            msg_log.log(Msg::PlaceTrap(player_id, place_pos, item_id));
        } else if use_item_throwable(item) {
            let throw_pos = dir.offset_pos(player_pos, PLAYER_THROW_DIST as i32);
            msg_log.log(Msg::ItemThrow(player_id, item_id, player_pos, throw_pos, false));
        } else if item == Item::Sling {
            let throw_pos = dir.offset_pos(player_pos, SLING_THROW_DIST as i32);
            if let Some(stone_id) = level.has_item_in_inventory(player_id, Item::Stone) {
                msg_log.log(Msg::ItemThrow(player_id, stone_id, player_pos, throw_pos, true));
            }
        } else {
            // It is possible to select a direction, then press shift, causing the move to be
            // invalid. In this case we just suppress the action, and return to playing.
            // Otherwise, process the move below.
            if let Some(move_pos) = use_result.pos {
                if level.entities.has_enough_stamina(player_id, 1) {
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
                } else {
                    msg_log.log(Msg::NotEnoughStamina(player_id));
                }
            }
        }
    }
}

fn finalize_use(level: &Level, settings: &mut Settings, msg_log: &mut MsgLog) {
    // If there is no direction, the user tried an invalid movement.
    // Returning here will just end use-mode.
    if settings.use_dir.is_none() {
        return;
    }

    if let UseAction::Item(item_class) = settings.use_action {
        finalize_use_item(item_class, level, settings, msg_log);
    } else if let UseAction::Skill(skill, action_mode) = settings.use_action {
        finalize_use_skill(skill, action_mode, level, settings, msg_log);
    } else {
        panic!("How did we get here? UseAction = '{}'", settings.use_action);
    }
}

fn initialize_use_mode(use_action: UseAction, settings: &mut Settings, msg_log: &mut MsgLog) {
    ensure_leave_cursor(settings, msg_log);

    settings.use_action = use_action;
    msg_log.log_info(InfoMsg::UseAction(settings.use_action));

    settings.use_dir = None;
    msg_log.log_info(InfoMsg::UseDirClear);
    msg_log.log_info(InfoMsg::UseHitPosClear);
}

fn start_use_talent(index: usize, level: &Level, _settings: &mut Settings, msg_log: &mut MsgLog) {
    let player_id = level.find_by_name(EntityName::Player).unwrap();

    if let Some(talent) = level.find_talent(index) {
        match talent {
            Talent::Invigorate => {
                msg_log.log(Msg::RefillStamina(player_id));
            }

            Talent::StrongAttack => {
                // TODO extra attack, perhaps as a status checked later?
            }

            Talent::Sprint => {
                // TODO extra sprint, perhaps as a status checked later?
            }

            Talent::Push => {
                // TODO push, but with some extra rules. Start with push message from use-mode
            }

            Talent::EnergyShield => {
                // TODO add blue health concept. Likely a status effect used when reducing
                // hp, and get it into the display.
            }
        }
    }
}

fn start_use_skill(index: usize, action_mode: ActionMode, level: &Level, settings: &mut Settings, msg_log: &mut MsgLog) {
    let player_id = level.find_by_name(EntityName::Player).unwrap();

    if let Some(skill) = level.find_skill(index) {
        let use_action = UseAction::Skill(skill, action_mode);

        match skill.mode() {
            SkillMode::Direction => {
                initialize_use_mode(use_action, settings, msg_log);

                for dir in Direction::move_actions().iter() {
                    let use_result = 
                        level.calculate_use_skill(player_id, skill, *dir, settings.move_mode);

                    if let Some(hit_pos) = use_result.pos {
                        msg_log.log_info(InfoMsg::UseHitPos(hit_pos));
                        msg_log.log_info(InfoMsg::UseOption(hit_pos, *dir));
                    }

                    // TODO this will highlight in red all tiles hittable from any chose of direction.
                    for hit_pos in use_result.hit_positions.iter() {
                        msg_log.log_info(InfoMsg::UseHitPos(*hit_pos));
                    }
                }

                change_state(settings, GameState::Use, msg_log);

                msg_log.log(Msg::StartUseSkill(player_id));
            }

            SkillMode::Immediate => {
                // Handle the skill immediately, with no action location as the skill should not be
                // directional or based on a position.
                handle_skill_index(index, ActionLoc::None, action_mode, level, settings, msg_log);
            }

            SkillMode::Cursor => {
                if settings.cursor == None {
                    let player_pos = level.entities.pos[&player_id];
                    settings.cursor = Some(player_pos);
                    msg_log.log(Msg::CursorState(true, player_pos));
                }

                // Record skill as a use_action.
                settings.cursor_action = Some(use_action);
                msg_log.log(Msg::CursorAction(use_action));
            }
        }
    }
}

fn start_use_item(item_class: ItemClass, level: &Level, settings: &mut Settings, msg_log: &mut MsgLog) {
    let player_id = level.find_by_name(EntityName::Player).unwrap();

    if let Some(item_index) = level.find_item(item_class) {
        let item_id = level.entities.inventory[&player_id][item_index as usize];

        if level.entities.item[&item_id] == Item::Herb {
            msg_log.log(Msg::EatHerb(player_id, item_id));
        } else if level.entities.item[&item_id] == Item::Stone {
            handle_throw_item(item_class, level, msg_log, settings);
        } else {
            initialize_use_mode(UseAction::Item(item_class), settings, msg_log);

            for dir in Direction::move_actions().iter() {
                let use_result = level.calculate_use_item(player_id,
                                                         item_index,
                                                         *dir,
                                                         settings.move_mode);
                if let Some(hit_pos) = use_result.pos {
                    msg_log.log_info(InfoMsg::UseOption(hit_pos, *dir));
                }

                for hit_pos in use_result.hit_positions.iter() {
                    msg_log.log_info(InfoMsg::UseHitPos(*hit_pos));
                }
            }

            change_state(settings, GameState::Use, msg_log);

            msg_log.log(Msg::StartUseItem(item_id));
        }
    }
}

pub fn handle_skill_index(skill_index: usize,
                          action_loc: ActionLoc,
                          action_mode: ActionMode,
                          level: &Level, 
                          settings: &mut Settings,
                          msg_log: &mut MsgLog) {
    let player_id = level.find_by_name(EntityName::Player).unwrap();

    /* Check for Valid Skill Use */
    // Get the skill in the player's list of skills.
    if skill_index >= level.entities.skills[&player_id].len() {
        // NOTE we may want a message indicating that the skill index was invalid
        return;
    }

    let skill = level.entities.skills[&player_id][skill_index];
    handle_skill(skill, action_loc, action_mode, level, settings, msg_log);
}

pub fn handle_skill(skill: Skill,
                    action_loc: ActionLoc,
                    action_mode: ActionMode,
                    level: &Level, 
                    _settings: &mut Settings,
                    msg_log: &mut MsgLog) {
    let player_id = level.find_by_name(EntityName::Player).unwrap();

    let reach = Reach::single(1);

    /* Determine Position Effected */
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

    let player_pos = level.entities.pos[&player_id];
    let dxy = sub_pos(skill_pos, player_pos);
    let direction: Option<Direction> = Direction::from_dxy(dxy.x, dxy.y);

    /* Carry Out Skill */
    match skill {
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
            // TODO should this stay here, or go to StartUseSkill?
            //settings.use_action = UseAction::Skill(skill, action_mode);
            //msg_log.log_info(InfoMsg::UseAction(settings.use_action));
            //settings.use_dir = None;
            //msg_log.log_info(InfoMsg::UseDirClear);
            //change_state(settings, GameState::Use, msg_log);
            // TODO remove when GrassWall is fully implemented with use-mode.
            // Unless skill use is left as-is in which case remove the code above.
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
                    if level.map.tile_is_blocking(blocked.end_pos) {
                        let next = next_from_to(player_pos, blocked.end_pos);
                        if  !level.map.tile_is_blocking(next) {
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

        Skill::Traps => {
            if let Some(direction) = direction {
                msg_log.log(Msg::InteractTrap(player_id, direction));
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
            msg_log.log(Msg::TryFarSight(player_id, SKILL_FARSIGHT_FOV_AMOUNT));
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
                msg_log.log(Msg::TryPassThrough(player_id, direction));
            }
        }

        Skill::WhirlWind => {
            if level.map.is_within_bounds(skill_pos) {
                msg_log.log(Msg::WhirlWind(player_id, skill_pos));
            }
        }

        Skill::Swift => {
            if let Some(direction) = direction {
                msg_log.log(Msg::TrySwift(player_id, direction));
            }
        }
    }
}

// TODO is this println okay to leave in? seems like it should be in stderr?
fn change_state(settings: &mut Settings, new_state: GameState, msg_log: &mut MsgLog) {
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

            GameState::HelpMenu => {
                println!("CONSOLE: Help menu");
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
