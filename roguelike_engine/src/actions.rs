use std::str::FromStr;
use std::fmt;

use serde::{Serialize, Deserialize};

use roguelike_core::movement::{Direction, Action, Reach, MoveMode, MoveType};
use roguelike_core::types::*;
use roguelike_core::utils::{scale_pos};
use roguelike_core::messaging::{Msg, MsgLog};
use roguelike_core::constants::*;
use roguelike_core::config::Config;
use roguelike_core::utils::{sub_pos, add_pos, next_from_to};
use roguelike_core::map::{Surface};

use crate::game::*;
use crate::input::*;
use crate::make_map;


#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionLoc {
    Dir(Direction),
    Place(Pos),
    None
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ActionResult {
    pub turn: Action,
    pub new_state: Option<GameState>,
}

impl Default for ActionResult {
    fn default() -> ActionResult {
        return ActionResult::new(Action::NoAction, None);
    }
}

impl ActionResult {
    pub fn new(turn: Action, new_state: Option<GameState>) -> ActionResult {
        return ActionResult { turn, new_state };
    }
}


pub type ActionTarget = i32;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum InputAction {
    Move(Direction, MoveMode),
    TileApply(ActionTarget),
    Pass,
    Pickup,
    DropItem,
    DropTargetItem(i32),
    Yell,
    UseItem(Direction, ActionTarget),
    Interact(Option<Direction>),
    CursorMove(Direction, bool, bool), // move direction, is relative, is long
    CursorReturn,
    CursorToggle,
    CursorApply(ActionMode, ActionTarget),
    // TODO this may be unused and should be removed
    MapClick(Pos, Pos), // map loc, map cell
    MouseButton(MouseClick, KeyDir),
    Inventory,
    SkillMenu,
    ClassMenu,
    Exit,
    Esc,
    ExploreAll,
    RegenerateMap,
    GodMode,
    IncreaseMoveMode,
    DecreaseMoveMode,
    OverlayOn,
    OverlayOff,
    SelectItem(usize),
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
            InputAction::TileApply(action_target) => write!(f, "tileapply {}", action_target),
            InputAction::Pass => write!(f, "pass"),
            InputAction::MapClick(loc, cell) => write!(f, "click {} {} {} {}", loc.x, loc.y, cell.x, cell.y),
            InputAction::MouseButton(click, keydir) => write!(f, "mousebutton {:?} {:?}", click, keydir),
            InputAction::Pickup => write!(f, "pickup"),
            InputAction::DropItem => write!(f, "drop"),
            InputAction::DropTargetItem(target) => write!(f, "droptarget {}", target),
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
            InputAction::UseItem(dir, target) => write!(f, "use, {:?} {}", dir, target),
            InputAction::Interact(dir) => write!(f, "interact {:?}", dir),
            InputAction::CursorMove(dir, relative, long) => write!(f, "cursormove {:?} {} {}", dir, relative, long),
            InputAction::CursorReturn => write!(f, "cursorreturn"),
            InputAction::CursorApply(mode, target) => write!(f, "cursorapply {:?} {:?}", mode, target),
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

        if s == "left" {
            let args = s.split(" ").collect::<Vec<&str>>();
            let move_mode = args[1].parse::<MoveMode>().unwrap();
            return Ok(InputAction::Move(Direction::Left, move_mode));
        } else if s == "right" {
            let args = s.split(" ").collect::<Vec<&str>>();
            let move_mode = args[1].parse::<MoveMode>().unwrap();
            return Ok(InputAction::Move(Direction::Right, move_mode));
        } else if s == "up" {
            let args = s.split(" ").collect::<Vec<&str>>();
            let move_mode = args[1].parse::<MoveMode>().unwrap();
            return Ok(InputAction::Move(Direction::Up, move_mode));
        } else if s == "down" {
            let args = s.split(" ").collect::<Vec<&str>>();
            let move_mode = args[1].parse::<MoveMode>().unwrap();
            return Ok(InputAction::Move(Direction::Down, move_mode));
        } else if s == "upleft" {
            let args = s.split(" ").collect::<Vec<&str>>();
            let move_mode = args[1].parse::<MoveMode>().unwrap();
            return Ok(InputAction::Move(Direction::UpLeft, move_mode));
        } else if s == "upright" {
            let args = s.split(" ").collect::<Vec<&str>>();
            let move_mode = args[1].parse::<MoveMode>().unwrap();
            return Ok(InputAction::Move(Direction::UpRight, move_mode));
        } else if s == "downleft" {
            let args = s.split(" ").collect::<Vec<&str>>();
            let move_mode = args[1].parse::<MoveMode>().unwrap();
            return Ok(InputAction::Move(Direction::DownLeft, move_mode));
        } else if s == "downright" {
            let args = s.split(" ").collect::<Vec<&str>>();
            let move_mode = args[1].parse::<MoveMode>().unwrap();
            return Ok(InputAction::Move(Direction::DownRight, move_mode));
        } else if s == "tileapply" {
            let args = s.split(" ").collect::<Vec<&str>>();
            let target = args[1].parse::<ActionTarget>().unwrap();
            return Ok(InputAction::TileApply(target));
        } else if s == "pass" {
            return Ok(InputAction::Pass);
        } else if s == "pickup" {
            return Ok(InputAction::Pickup);
        } else if s == "drop" {
            return Ok(InputAction::DropItem);
        } else if s == "droptarget" {
            let args = s.split(" ").collect::<Vec<&str>>();
            let target = args[1].parse::<ActionTarget>().unwrap();
            return Ok(InputAction::DropTargetItem(target));
        } else if s == "yell" {
            return Ok(InputAction::Yell);
        } else if s == "inventory" {
            return Ok(InputAction::Inventory);
        } else if s == "use" {
            let args = s.split(" ").collect::<Vec<&str>>();
            let direction = args[1].parse::<Direction>().unwrap();
            let target = args[1].parse::<ActionTarget>().unwrap();
            return Ok(InputAction::UseItem(direction, target));
        } else if s.starts_with("selectitem") {
            let args = s.split(" ").collect::<Vec<&str>>();
            let selection = args[1].parse::<usize>().unwrap();
            return Ok(InputAction::SelectItem(selection));
        } else if s == "interact" {
            let args = s.split(" ").collect::<Vec<&str>>();
            let dir = args[1].parse::<Direction>().ok();
            return Ok(InputAction::Interact(dir));
        } else if s == "godmode" {
            return Ok(InputAction::GodMode);
        } else if s.starts_with("click") {
            let args = s.split(" ").collect::<Vec<&str>>();
            let loc_x = args[1].parse::<i32>().unwrap();
            let loc_y = args[2].parse::<i32>().unwrap();
            let cell_x = args[3].parse::<i32>().unwrap();
            let cell_y = args[4].parse::<i32>().unwrap();
            return Ok(InputAction::MapClick(Pos::new(loc_x, loc_y), Pos::new(cell_x, cell_y)));
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
        } else if s.starts_with("cursormove") {
            let args = s.split(" ").collect::<Vec<&str>>();
            let dir = Direction::from_str(args[1]).unwrap();
            let relative = bool::from_str(args[2]).unwrap();
            let long = bool::from_str(args[3]).unwrap();
            return Ok(InputAction::CursorMove(dir, relative, long));
        } else if s.starts_with("cursorreturn") {
            return Ok(InputAction::CursorReturn);
        } else if s.starts_with("cursorapply") {
            let args = s.split(" ").collect::<Vec<&str>>();
            let mode = ActionMode::from_str(args[1]).unwrap();
            let target = args[2].parse::<i32>().unwrap();
            return Ok(InputAction::CursorApply(mode, target));
        } else if s.starts_with("cursortoggle") {
            return Ok(InputAction::CursorToggle);
        } else {
            return Err(format!("Could not parse '{}' as InputAction", s));
        }
    }
}

pub fn handle_input_universal(input_action: InputAction, game: &mut Game) {
    match input_action {
        InputAction::ExploreAll => {
            for x in 0..game.data.map.width() {
                for y in 0..game.data.map.height() {
                    let pos = Pos::new(x, y);
                    game.data.map[pos].explored = true;
                }
            }
        }

        InputAction::RegenerateMap => {
            let _position = make_map::make_map(&game.config.map_load.clone(), game);
        }

        InputAction::GodMode => {
            // TODO hmmm... add a message, or resolve at higher level as a universal action
            let god_mode_hp = 10000;
            let player_id = game.data.find_by_name(EntityName::Player).unwrap();
            game.data.entities.fighter[&player_id].hp = god_mode_hp;
            game.data.entities.fighter[&player_id].max_hp = god_mode_hp;
            game.data.entities.energy[&player_id] = 1000;

            // toggle god mode flag
            game.settings.god_mode = !game.settings.god_mode;
        }

        _ => {}
    }
}

// TODO remove with selection system
//pub fn inventory_use_item(item_index: usize,
//                          data: &GameData,
//                          settings: &mut GameSettings,
//                          _msg_log: &mut MsgLog) -> ActionResult {
//    let mut action_result: ActionResult = Default::default();
//
//    let player_id = data.find_by_name(EntityName::Player).unwrap();
//    let item_id = data.entities.inventory[&player_id][item_index];
//    settings.selection.item = Some(item_id);
//
//    settings.selection.only_visible = false;
//
//    // if the item is a trap, set it.
//    // otherwise, throw it.
//    if data.entities.trap.get(&item_id).is_some() {
//        settings.selection =
//            Selection::new(SelectionType::WithinReach(Reach::single(1)), SelectionAction::PlaceTrap);
//    } else {
//        settings.selection =
//            Selection::new(SelectionType::WithinRadius(PLAYER_THROW_DIST), SelectionAction::Throw);
//    }
//
//    action_result.new_state = Some(GameState::Selection);
//
//    return action_result;
//}
//
//// TODO remove with selection system
//pub fn inventory_select_item(item_index: usize,
//                             data: &GameData,
//                             settings: &mut GameSettings,
//                             msg_log: &mut MsgLog) -> ActionResult {
//    let mut action_result: ActionResult = Default::default();
//
//    let player_id = data.find_by_name(EntityName::Player).unwrap();
//    let num_items = data.entities.inventory[&player_id].len();
//
//    // if item index is not in the player's inventory, do nothing
//    if item_index < num_items {
//        match settings.inventory_action {
//            InventoryAction::Use => {
//                action_result = inventory_use_item(item_index, data, settings, msg_log);
//            }
//
//            InventoryAction::Drop => {
//                msg_log.log(Msg::DropItem(player_id, item_index as u64));
//                settings.state = GameState::Playing;
//                msg_log.log(Msg::GameState(settings.state));
//            }
//        }
//    }
//
//    return action_result;
//}

pub fn handle_input_inventory(input: InputAction,
                              _data: &GameData,
                              _settings: &mut GameSettings,
                              _msg_log: &mut MsgLog) -> ActionResult {
    let mut action_result: ActionResult = Default::default();

    match input {
        InputAction::Inventory => {
            action_result.new_state = Some(GameState::Playing);
        }

        InputAction::SelectItem(_item_index) => {
            // TODO remove with selection system
            //inventory_select_item(item_index, data, settings, msg_log);
        }

        InputAction::Esc => {
            action_result.new_state = Some(GameState::Playing);
        }

        InputAction::Exit => {
            action_result.new_state = Some(GameState::ConfirmQuit);
        }

        _ => {
        }
    }

    return action_result;
}

pub fn handle_input_skill_menu(input: InputAction,
                               data: &GameData,
                               settings: &mut GameSettings,
                               msg_log: &mut MsgLog,
                               config: &Config) -> ActionResult {
    let mut action_result: ActionResult = Default::default();

    match input {
        InputAction::Inventory => {
            action_result.new_state = Some(GameState::Inventory);
        }

        InputAction::SkillMenu => {
            action_result.new_state = Some(GameState::Playing);
        }

        InputAction::SelectItem(skill_index) => {
            action_result.turn =
                handle_skill(skill_index, ActionLoc::None, ActionMode::Primary, data, settings, msg_log, config);
        }

        InputAction::Esc => {
            action_result.new_state = Some(GameState::Playing);
        }

        InputAction::Exit => {
            action_result.new_state = Some(GameState::ConfirmQuit);
        }

        _ => {
        }
    }

    return action_result;
}

pub fn handle_input_class_menu(input: InputAction,
                               _data: &GameData,
                               _settings: &mut GameSettings,
                               msg_log: &mut MsgLog) -> ActionResult {
    let mut action_result: ActionResult = Default::default();

    match input {
        InputAction::Inventory => {
            action_result.new_state = Some(GameState::Inventory);
        }

        InputAction::ClassMenu => {
            action_result.new_state = Some(GameState::Playing);
        }

        InputAction::SelectItem(class_index) => {
            let classes = EntityClass::classes();
            if class_index < classes.len() {
                // give player skills from a particular class
                msg_log.log(Msg::AddClass(classes[class_index]));
                action_result.new_state = Some(GameState::Playing);
            }
        }

        InputAction::Esc => {
            action_result.new_state = Some(GameState::Playing);
        }

        InputAction::Exit => {
            action_result.new_state = Some(GameState::ConfirmQuit);
        }

        _ => {
        }
    }

    return action_result;
}

pub fn handle_input_confirm_quit(input: InputAction) -> ActionResult {
    let mut action_result: ActionResult = Default::default();

    match input {
        InputAction::Esc => {
            action_result.new_state = Some(GameState::Playing);
        }

        InputAction::Exit => {
            action_result.new_state = Some(GameState::Exit);
        }

        _ => {
        }
    }

    return action_result;
}

pub fn handle_input(input_action: InputAction,
                    data: &GameData,
                    settings: &mut GameSettings,
                    msg_log: &mut MsgLog,
                    config: &Config) -> ActionResult {
    let action_result: ActionResult;

    match settings.state {
        GameState::Playing => {
            action_result =
                handle_input_playing(input_action, data, settings, msg_log, config);
        }

        GameState::Win => {
            action_result = Default::default();
        }

        GameState::Lose => {
            action_result = Default::default();
        }

        GameState::Inventory => {
            action_result = 
                handle_input_inventory(input_action, data, settings, msg_log);
        }

        GameState::SkillMenu => {
            action_result = 
                handle_input_skill_menu(input_action, data, settings, msg_log, config);
        }

        GameState::ClassMenu => {
            action_result =
                handle_input_class_menu(input_action, data, settings, msg_log);
        }

        GameState::ConfirmQuit => {
            action_result = handle_input_confirm_quit(input_action);
        }

        GameState::Exit => {
            action_result = Default::default();
        }
    }

    return action_result;
}

pub fn handle_input_playing(input_action: InputAction,
                            data: &GameData,
                            settings: &mut GameSettings,
                            msg_log: &mut MsgLog,
                            config: &Config) -> ActionResult {
    let mut action_result: ActionResult = Default::default();

    let player_id = data.find_by_name(EntityName::Player).unwrap();
    let player_pos = data.entities.pos[&player_id];

    let player_alive = data.entities.status[&player_id].alive;

    match (input_action, player_alive) {
        (InputAction::CursorReturn, _) => {
            if settings.cursor.is_some() {
                settings.cursor = Some(player_pos);
            }
        }

        (InputAction::CursorMove(dir, relative, long), _) => {
            let cursor_pos = settings.cursor.expect("CursorMove outside of cursor mode?");

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

        (InputAction::TileApply(target), true) => {
            let cursor_pos = settings.cursor.expect("TileApply outside of cursor mode?");

            action_result.turn =
                chord(ActionLoc::Place(cursor_pos),
                      ActionMode::Alternate,
                      target,
                      data,
                      settings,
                      config,
                      msg_log);
        }

        (InputAction::CursorApply(mode, target), true) => {
            let cursor_pos = settings.cursor.expect("CursorApply outside of cursor mode?");

            action_result.turn =
                chord(ActionLoc::Place(cursor_pos),
                      mode,
                      target,
                      data,
                      settings,
                      config,
                      msg_log);
        }

        (InputAction::CursorToggle, true) => {
            if settings.cursor.is_none() {
                settings.cursor = Some(player_pos);
            } else {
                settings.cursor = None;
            }
        }

        (InputAction::DropTargetItem(target), true) => {
            msg_log.log(Msg::DropItem(player_id, target as u64));
        }

        (InputAction::Pass, true) => {
            msg_log.log(Msg::Moved(player_id, MoveType::Pass, player_pos));
        }

        (InputAction::Move(direction, move_mode), true) => {
            action_result.turn = Action::MoveDir(direction, move_mode);
        }

        (InputAction::DropItem, true) => {
            settings.inventory_action = InventoryAction::Drop;
            action_result.new_state = Some(GameState::Inventory);
        }

        (InputAction::Pickup, true) => {
            msg_log.log(Msg::PickUp(player_id));
        }

        // TODO this should be removeable
        (InputAction::MapClick(_map_loc, _map_cell), _) => {
            action_result.turn = Action::none();
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
            action_result.new_state = Some(GameState::Inventory);
        }

        (InputAction::SkillMenu, true) => {
            action_result.new_state = Some(GameState::SkillMenu);
        }

        (InputAction::ClassMenu, true) => {
            action_result.new_state = Some(GameState::ClassMenu);
        }

        (InputAction::Exit, _) => {
            action_result.new_state = Some(GameState::ConfirmQuit);
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

        (InputAction::UseItem(dir, target), _) => {
            let pos = data.entities.pos[&player_id];
            let use_pos = dir.offset_pos(pos, 1);
            if let Some(item_id) = data.entities.inventory[&player_id].get(target as usize) {
                action_result.turn = Action::UseItem(use_pos, *item_id);
            }
        }

        (_, _) => {
        }
    }

    return action_result;
}

pub fn handle_skill(skill_index: usize,
                    action_loc: ActionLoc,
                    action_mode: ActionMode,
                    data: &GameData, 
                    settings: &mut GameSettings, 
                    msg_log: &mut MsgLog,
                    _config: &Config) -> Action {
    let player_id = data.find_by_name(EntityName::Player).unwrap();

    /* Check for Valid Skill Use */
    // # check if we have enough energy to carry out the skill
    if data.entities.energy[&player_id] <= 0 {
        msg_log.log(Msg::NotEnoughEnergy(player_id));
        return Action::none();
    }

    // get the skill in the player's list of skills
    if skill_index >= data.entities.skills[&player_id].len() {
        // NOTE we may want a message indicating that the skill index was invalid
        return Action::none();
    }

    let reach = Reach::single(1);

    /* Determine Position Effected */
    // NOTE we may want a message indicating that the skill was invalid
    let skill_pos;
    match action_loc {
        ActionLoc::Place(pos) => {
            skill_pos = pos;
        }

        ActionLoc::Dir(dir) => {
            let player_pos = data.entities.pos[&player_id];
            if let Some(pos) = reach.furthest_in_direction(player_pos, dir) {
                skill_pos = pos;
            } else {
                return Action::none();
            }
        }

        ActionLoc::None => {
            return Action::none();
        }
    }

    /* Carry Out Skill */
    let mut turn: Action = Action::none();

    match data.entities.skills[&player_id][skill_index] {
        Skill::GrassThrow => {
            let player_id = data.find_by_name(EntityName::Player).unwrap();
            let player_pos = data.entities.pos[&player_id];
            let dxy = sub_pos(skill_pos, player_pos);
            let direction = Direction::from_dxy(dxy.x, dxy.y).unwrap();
            turn = Action::GrassThrow(player_id, direction);
        }

        Skill::GrassBlade => {
            turn = Action::GrassBlade(player_id, action_mode);
            settings.state = GameState::Playing;
            msg_log.log(Msg::GameState(settings.state));
        }

        Skill::Blink => {
            msg_log.log(Msg::Blink(player_id));

            // TODO this state change is no longer necessary when Selection
            // is removed. same goes for other skills
            settings.state = GameState::Playing;
            msg_log.log(Msg::GameState(settings.state));
        }

        Skill::PassWall => {
            turn = Action::NoAction;

            let player_id = data.find_by_name(EntityName::Player).unwrap();
            let player_pos = data.entities.pos[&player_id];
            let blocked = data.map.path_blocked_move(player_pos, skill_pos);
            
            if let Some(blocked) = blocked {
                if data.map[blocked.end_pos].block_move {
                    let next = next_from_to(player_pos, blocked.end_pos);
                    dbg!(skill_pos, blocked.end_pos, next);
                    if  !data.map[next].block_move {
                        turn = Action::PassWall(player_id, next);
                    }
                } else {
                    turn = Action::PassWall(player_id, skill_pos);
                }
            }
        }

        Skill::Rubble => {
            let player_id = data.find_by_name(EntityName::Player).unwrap();
            let player_pos = data.entities.pos[&player_id];
            let blocked = data.map.path_blocked_move(player_pos, skill_pos);

            if let Some(blocked) = blocked {
                if data.has_blocking_entity(skill_pos).is_none() {
                    turn = Action::Rubble(player_id, blocked);
                }
            }
        }

        Skill::Reform => {
            turn = Action::NoAction;

            let player_id = data.find_by_name(EntityName::Player).unwrap();
            if data.map[skill_pos].surface == Surface::Rubble &&
               data.has_blocking_entity(skill_pos).is_none() {
                turn = Action::Reform(player_id, skill_pos);
            }
        }

        Skill::Swap => {
            turn = Action::NoAction;

            let player_id = data.find_by_name(EntityName::Player).unwrap();
            if let Some(entity_id) = data.has_blocking_entity(skill_pos) {
                turn = Action::Swap(player_id, entity_id);
            }
        }

        Skill::Push => {
            let player_id = data.find_by_name(EntityName::Player).unwrap();
            let player_pos = data.entities.pos[&player_id];
            let dxy = sub_pos(skill_pos, player_pos);
            let direction = Direction::from_dxy(dxy.x, dxy.y).unwrap();
            // TODO this might need to be carried around?
            let push_amount = 1;
            turn = Action::Push(player_id, direction, push_amount);
        }
    }

    return turn;
}

pub fn chord(loc: ActionLoc,
             mode: ActionMode,
             target: i32,
             data: &GameData,
             settings: &mut GameSettings,
             config: &Config,
             msg_log: &mut MsgLog) -> Action {
    // if no target selection, then it is a move
    let turn;
    if target == -1 {
        turn = chord_move(loc, mode, data, msg_log);
    } else {
        turn = chord_selection(loc, mode, target, data, settings, config, msg_log);
    }

    return turn;
}

fn chord_move(loc: ActionLoc,
              mode: ActionMode,
              data: &GameData,
              msg_log: &mut MsgLog) -> Action {
    let turn;
    let player_id = data.find_by_name(EntityName::Player).unwrap();
    let player_pos = data.entities.pos[&player_id];

    let move_mode;
    match mode {
        ActionMode::Primary => {
            move_mode = MoveMode::Sneak;
            //msg_log.log(Msg::ChangeMoveMode(player_id, false));
        }

        ActionMode::Alternate => {
            move_mode = MoveMode::Run;
            //msg_log.log(Msg::ChangeMoveMode(player_id, true));
        }
    }

    match loc {
        ActionLoc::None => {
            msg_log.log(Msg::Moved(player_id, MoveType::Pass, player_pos));
            turn = Action::none();
        }

        ActionLoc::Dir(direction) => {
            turn = Action::MoveDir(direction, move_mode); // handle_move(player_id, direction, data);
        }

        ActionLoc::Place(pos) => {
            if pos == player_pos {
                msg_log.log(Msg::Moved(player_id, MoveType::Pass, pos));
                turn = Action::none();
            } else {
                let dxy = sub_pos(pos, player_pos);
                let direction = Direction::from_dxy(dxy.x, dxy.y).unwrap();

                turn = Action::MoveDir(direction, move_mode);
            }
        }
    }

    return turn;
}

fn chord_selection(loc: ActionLoc,
                   mode: ActionMode,
                   target: i32,
                   data: &GameData,
                   settings: &mut GameSettings,
                   config: &Config,
                   msg_log: &mut MsgLog) -> Action {
    let mut turn = Action::none();
    let player_id = data.find_by_name(EntityName::Player).unwrap();

    if target >= 2 {
        // the minus 2 here comes from the primary and secondary item, after which comes
        // the skills
        let skill_index = (target - 2) as usize;
        if skill_index < data.entities.skills[&player_id].len() {
            turn = handle_skill(skill_index, loc, mode, data, settings, msg_log, config);
        }
    } else {
        let num_items_in_inventory = data.entities.inventory[&player_id].len() as i32;

        if target >= num_items_in_inventory {
            return Action::none();
        }

        let item_id = data.entities.inventory[&player_id][target as usize];
        
        match mode {
            ActionMode::Primary => {
                match loc {
                    ActionLoc::Dir(dir) => {
                        // primary item use is the item's main action
                        let pos = data.entities.pos[&player_id];
                        let use_pos = dir.offset_pos(pos, 1);
                        turn = Action::UseItem(use_pos, item_id);
                    }

                    _ => panic!("Is this even possible anymore?"),
                }
            }

            ActionMode::Alternate => {
                let player_pos = data.entities.pos[&player_id];

                // alternate item use means drop or throw item
                match loc {
                    ActionLoc::None => {
                        msg_log.log(Msg::DropItem(player_id, target as u64));
                    }

                    ActionLoc::Place(pos) => {
                        msg_log.log(Msg::ItemThrow(player_id, item_id, player_pos, pos));
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

    return turn;
}

