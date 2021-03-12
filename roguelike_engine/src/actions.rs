use std::str::FromStr;
use std::fmt;

use serde::{Serialize, Deserialize};

use roguelike_core::movement::{Direction, Action, Reach, MoveMode};
use roguelike_core::types::*;
use roguelike_core::utils::{scale_pos};
use roguelike_core::messaging::{Msg, MsgLog};
use roguelike_core::constants::*;
use roguelike_core::config::Config;
use roguelike_core::utils::{sub_pos, add_pos};

use crate::game::*;
use crate::selection::*;
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
    Yell,
    UseItem,
    Interact,
    Chord(Option<Direction>, ActionMode, ActionTarget),
    CursorMove(Direction, bool, bool), // move direction, is relative, is long
    CursorReturn,
    CursorToggle,
    CursorApply(ActionMode, ActionTarget),
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
            InputAction::UseItem => write!(f, "use"),
            InputAction::Interact => write!(f, "interact"),
            InputAction::Chord(dir, mode, target) => write!(f, "chord {:?} {:?} {:?}", dir, mode, target),
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
        } else if s.starts_with("chord") {
            let args = s.split(" ").collect::<Vec<&str>>();
            let dir = Direction::from_str(args[1]).ok();
            let mode = ActionMode::from_str(args[2]).unwrap();
            let target = args[3].parse::<i32>().unwrap();
            return Ok(InputAction::Chord(dir, mode, target));
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

pub fn inventory_use_item(item_index: usize,
                          data: &GameData,
                          settings: &mut GameSettings,
                          _msg_log: &mut MsgLog) -> ActionResult {
    let mut action_result: ActionResult = Default::default();

    let player_id = data.find_by_name(EntityName::Player).unwrap();
    let item_id = data.entities.inventory[&player_id][item_index];
    settings.selection.item = Some(item_id);

    settings.selection.only_visible = false;

    // if the item is a trap, set it.
    // otherwise, throw it.
    if data.entities.trap.get(&item_id).is_some() {
        settings.selection =
            Selection::new(SelectionType::WithinReach(Reach::single(1)), SelectionAction::PlaceTrap);
    } else {
        settings.selection =
            Selection::new(SelectionType::WithinRadius(PLAYER_THROW_DIST), SelectionAction::Throw);
    }

    action_result.new_state = Some(GameState::Selection);

    return action_result;
}

pub fn inventory_select_item(item_index: usize,
                             data: &GameData,
                             settings: &mut GameSettings,
                             msg_log: &mut MsgLog) -> ActionResult {
    let mut action_result: ActionResult = Default::default();

    let player_id = data.find_by_name(EntityName::Player).unwrap();
    let num_items = data.entities.inventory[&player_id].len();

    // if item index is not in the player's inventory, do nothing
    if item_index < num_items {
        match settings.inventory_action {
            InventoryAction::Use => {
                action_result = inventory_use_item(item_index, data, settings, msg_log);
            }

            InventoryAction::Drop => {
                msg_log.log(Msg::DropItem(player_id, item_index as u64));
                settings.state = GameState::Playing;
                msg_log.log(Msg::GameState(settings.state));
            }
        }
    }

    return action_result;
}

pub fn handle_input_inventory(input: InputAction,
                              data: &GameData,
                              settings: &mut GameSettings,
                              msg_log: &mut MsgLog) -> ActionResult {
    let mut action_result: ActionResult = Default::default();

    match input {
        InputAction::Inventory => {
            action_result.new_state = Some(GameState::Playing);
        }

        InputAction::SelectItem(item_index) => {
            inventory_select_item(item_index, data, settings, msg_log);
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
            // NOTE there is no way to select the alternate use of a skill!
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
                               settings: &mut GameSettings,
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
                // TODO comment what this means, and how it is used
                settings.selection.only_visible = false;

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

pub fn handle_input_selection(input: InputAction,
                              data: &GameData, 
                              settings: &mut GameSettings,
                              config: &Config,
                              msg_log: &mut MsgLog) -> ActionResult {
    let mut action_result: ActionResult = Default::default();

    let player_id = data.find_by_name(EntityName::Player).unwrap();

    match input {
        InputAction::Inventory => {
            action_result.new_state = Some(GameState::Inventory);
        }

        InputAction::Exit => {
            action_result.new_state = Some(GameState::ConfirmQuit);
        }

        InputAction::Esc => {
            action_result.new_state = Some(GameState::Playing);
            settings.draw_selection_overlay = false;
        }

        InputAction::MapClick(_map_loc, map_cell) => {
            let player_pos = data.entities.pos[&player_id];

            let maybe_action =
                settings.selection.select(player_pos,
                                          map_cell,
                                          config.fov_radius_player,
                                          data,
                                          config);
            if let Some(action) = maybe_action {
                action_result.new_state = Some(GameState::Playing);
                msg_log.log(Msg::GameState(settings.state));
                // TODO(&mut) move to resolve, or allow settings to change
                settings.draw_selection_overlay = false;
                action_result.turn = action;
            }
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

        GameState::Selection => {
            settings.draw_selection_overlay = true;

            action_result =
                handle_input_selection(input_action, data, settings, config, msg_log);
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

        (InputAction::Chord(dir, mode, target), true) => {
            let loc = dir.map_or(ActionLoc::None, |dir| ActionLoc::Dir(dir));
            action_result.turn =
                chord(loc,
                      mode,
                      target, 
                      data,
                      settings,
                      config,
                      msg_log);
        }

        (InputAction::Pass, true) => {
            action_result.turn = Action::Pass;
        }

        (InputAction::Move(direction, move_mode), true) => {
            action_result.turn = Action::MoveDir(direction, move_mode);
        }

        (InputAction::DropItem, true) => {
            settings.inventory_action = InventoryAction::Drop;
            action_result.new_state = Some(GameState::Inventory);
        }

        (InputAction::Pickup, true) => {
            action_result.turn = Action::Pickup;
        }

        // TODO this should be removeable
        (InputAction::MapClick(_map_loc, _map_cell), _) => {
            action_result.turn = Action::none();
        }

        (InputAction::Yell, true) => {
            action_result.turn = Action::Yell;
        }

        (InputAction::IncreaseMoveMode, true) => {
            // NOTE this could be an action instead of direct message
            msg_log.log(Msg::ChangeMoveMode(player_id, true));
        }

        (InputAction::DecreaseMoveMode, true) => {
            // NOTE this could be an action instead of direct message
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

        (InputAction::Interact, _) => {
            action_result.new_state = Some(GameState::Selection);
            let reach = Reach::single(1);
            settings.selection =
                Selection::new(SelectionType::WithinReach(reach), SelectionAction::Interact);
        }

        (InputAction::UseItem, _) => {
            let pos = data.entities.pos[&player_id];
            action_result.turn = Action::UseItem(pos);
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
                    config: &Config) -> Action {
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
    match data.entities.skills[&player_id][skill_index] {
        Skill::GrassThrow => {
            selection =
                Some(Selection::new(SelectionType::WithinReach(reach), SelectionAction::GrassThrow));
        }

        Skill::GrassBlade => {
            turn = Action::GrassBlade(player_id, action_mode);
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
            let reach = Reach::single(config.swap_radius);
            selection =
                Some(Selection::new(SelectionType::WithinReach(reach), SelectionAction::Swap));
        }

        Skill::Push => {
            selection =
                Some(Selection::new(SelectionType::WithinReach(reach), SelectionAction::Push));
        }
    }

    if let Some(selection) = selection {
        match action_loc {
            ActionLoc::Place(pos) => {
                turn = selection.action.action_from_pos(pos, selection.item, data);
            }

            ActionLoc::Dir(dir) => {
                let player_pos = data.entities.pos[&player_id];
                if let Some(pos) = selection.typ.offset_pos(player_pos, dir) {
                    turn = selection.action.action_from_pos(pos, selection.item, data);
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

    return turn;
}

// TODO consider creating a Chord struct with loc, mode, target
// to simplify passing around and calling this function
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
        turn = chord_move(loc, mode, data);
    } else {
        turn = chord_selection(loc, mode, target, data, settings, config, msg_log);
    }

    return turn;
}

fn chord_move(loc: ActionLoc,
              mode: ActionMode,
              data: &GameData) -> Action {
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
            turn = Action::Pass;
        }

        ActionLoc::Dir(direction) => {
            turn = Action::MoveDir(direction, move_mode); // handle_move(player_id, direction, data);
        }

        ActionLoc::Place(pos) => {
            if pos == player_pos {
                turn = Action::Pass;
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

    let num_items_in_inventory = data.entities.inventory[&player_id].len() as i32;
    if target >= 2 {
        // the minus 2 here comes from the primary and secondary item, after which comes
        // the skills
        let skill_index = (target - 2) as usize;
        if skill_index < data.entities.skills[&player_id].len() {
            turn = handle_skill(skill_index, loc, mode, data, settings, msg_log, config);
        }
    } else if target < num_items_in_inventory {
        match mode {
            ActionMode::Primary => {
                // primary item use is the item's main action
                let pos = data.entities.pos[&player_id];
                turn = Action::UseItem(pos);
            }

            ActionMode::Alternate => {
                // alternate item use means drop or throw item
                match loc {
                    ActionLoc::None => {
                        msg_log.log(Msg::DropItem(player_id, target as u64));
                    }

                    ActionLoc::Place(pos) => {
                        let item_id = data.entities.inventory[&player_id][target as usize];
                        turn = Action::ThrowItem(pos, item_id);
                    }

                    ActionLoc::Dir(direction) => {
                        let start = data.entities.pos[&player_id];
                        let max_end = direction.offset_pos(start, PLAYER_THROW_DIST as i32);
                        let end = data.map.path_blocked_move(start, max_end)
                                               .map_or(max_end, |b| b.end_pos);
                        let item_id = data.entities.inventory[&player_id][target as usize];
                        turn = Action::ThrowItem(end, item_id);
                    }
                }
            }
        }
    }

    return turn;
}

