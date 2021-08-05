use std::time::Instant;
use std::collections::HashMap;
use std::cmp::Ord;
use std::str::FromStr;

use serde::{Serialize, Deserialize};

use roguelike_core::types::*;
use roguelike_core::movement::Direction;
use roguelike_core::config::Config;
use roguelike_core::movement::MoveMode;

use crate::game::*;
use crate::actions::*;


const SKILL_KEYS: &[char] = &['a', 's', 'd'];
const ITEM_KEYS: &[char] = &['z', 'x', 'c'];


#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum KeyDir {
    Up,
    Held,
    Down,
}

impl FromStr for KeyDir {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();

        if s == "down" {
            return Ok(KeyDir::Down);
        } else if s == "held" {
            return Ok(KeyDir::Held);
        } else if s == "up" {
            return Ok(KeyDir::Up);
        }

        panic!(format!("KeyDir {} unexpected", s));
    }
}


#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum InputDirection {
    Dir(Direction),
    Current,
}

impl InputDirection {
    pub fn from_chr(chr: char) -> Option<InputDirection> {
        if let Some(dir) = direction_from_digit(chr) {
            return Some(InputDirection::Dir(dir));
        } else if chr == '5' {
            return Some(InputDirection::Current);
        } else {
            return None;
        }
    }
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum Target {
    Item(usize),
    Skill(usize),
}

impl Target {
    pub fn item(index: usize) -> Target {
        return Target::Item(index);
    }

    pub fn skill(index: usize) -> Target {
        return Target::Skill(index);
    }
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum MouseClick {
    Left,
    Right,
    Middle,
}

#[derive(Clone, Debug, Copy, PartialEq, PartialOrd)]
pub struct HeldState {
    down_time: Instant,
    repetitions: usize,
}

impl HeldState {
    pub fn new(down_time: Instant, repetitions: usize) -> HeldState {
        return HeldState { down_time, repetitions };
    }

    pub fn repeated(&self) -> HeldState {
        return HeldState::new(self.down_time, self.repetitions + 1);
    }
}

#[derive(Clone, Debug, Copy, Eq, PartialEq)]
pub enum InputEvent {
    Char(char, KeyDir),
    Ctrl(KeyDir),
    Shift(KeyDir),
    Alt(KeyDir),
    Enter(KeyDir),
    MousePos(i32, i32),
    MouseButton(MouseClick, Pos, Option<Pos>, KeyDir), // button clicked, mouse position, screen square, keydir
    Esc,
    Tab,
    Quit,
}

#[derive(Clone, Debug)]
pub struct Input {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub target: Option<Target>,
    pub direction: Option<InputDirection>,
    pub char_down_order: Vec<char>,
    pub cursor: bool,
    pub char_held: HashMap<char, HeldState>,
}

impl Input {
    pub fn new() -> Input {
        return Input { ctrl: false,
                       alt: false,
                       shift: false,
                       target: None,
                       direction: None,
                       char_down_order: Vec::new(),
                       cursor: false,
                       char_held: HashMap::new()
        };
    }

    pub fn move_mode(&self) -> MoveMode {
        if self.shift {
            return MoveMode::Run;
        } else if self.ctrl {
            return MoveMode::Sneak;
        } else {
            return MoveMode::Walk;
        }
    }

    pub fn action_mode(&self) -> ActionMode {
        if self.alt {
            return ActionMode::Alternate;
        } else {
            return ActionMode::Primary;
        }
    }

    pub fn is_held(&self, chr: char) -> bool {
        if let Some(held_state) = self.char_held.get(&chr) {
            return held_state.repetitions > 0;
        }

        return false;
    }

    pub fn handle_event(&mut self,
                        settings: &mut GameSettings,
                        event: InputEvent,
                        time: Instant,
                        config: &Config) -> InputAction {
        let mut action = InputAction::None;

        // remember characters that are pressed down
        if let InputEvent::Char(chr, dir) = event {
            if dir == KeyDir::Down {
                let held_state = HeldState { down_time: time, repetitions: 0 };
                self.char_held.insert(chr, held_state);
            }
        }

        match event {
            InputEvent::MousePos(_, _) => {
                // we don't use the mouse position within the game
            }

            InputEvent::Quit => {
                action = InputAction::ForceExit;
            }

            InputEvent::Esc => {
                action = InputAction::Esc;
            }

            InputEvent::Tab => {
                action = InputAction::CursorReturn;
            }

            InputEvent::Enter(dir) => {
                if dir == KeyDir::Up {
                    action = InputAction::MoveTowardsCursor(self.move_mode());
                }
            }

            InputEvent::Ctrl(dir) => {
                if dir != KeyDir::Held {
                    self.ctrl = dir == KeyDir::Down;
                }
            }

            InputEvent::Shift(dir) => {
                if dir != KeyDir::Held {
                    self.shift = dir == KeyDir::Down;
                }
            }

            InputEvent::Alt(dir) => {
                if dir != KeyDir::Held {
                    self.alt = dir == KeyDir::Down;
                }
            }

            InputEvent::Char(chr, dir) => {
                action = self.handle_char(chr, dir, time, settings, config);
            }

            InputEvent::MouseButton(clicked, mouse_pos, target_pos, dir) => {
                action = self.handle_mouse_button(clicked, mouse_pos, target_pos, dir);
            }
        }

        return action;
    }

    fn handle_mouse_button(&mut self, clicked: MouseClick, mouse_pos: Pos, target_pos: Option<Pos>, dir: KeyDir) -> InputAction {
        let mut action = InputAction::MouseButton(clicked, dir);

        let down = dir == KeyDir::Down;
        match clicked {
            MouseClick::Left => {

                if down {
                    if let Some(target_pos) = target_pos {
                        action = InputAction::MapClick(mouse_pos, target_pos);
                    }
                }
            }

            MouseClick::Middle => {
                action = InputAction::MouseButton(clicked, dir);
            }

            MouseClick::Right => {
                action = InputAction::MouseButton(clicked, dir);
            }
        }

        return action;
    }

    fn handle_char_up(&mut self, chr: char, settings: &GameSettings) -> InputAction {
        if let Some(index) = self.char_down_order.iter().position(|c| *c == chr) {
            self.char_down_order.remove(index);
        }

        let is_held = self.is_held(chr);
        self.char_held.remove(&chr);

        if settings.state.is_menu() {
            if chr.is_ascii_digit() {
                return InputAction::SelectEntry(chr.to_digit(10).unwrap() as usize);
            }

            return self.apply_char(chr, settings);
        } else if settings.state == GameState::Use {
            if let Some(index) = ITEM_KEYS.iter().position(|key| *key == chr) {
                self.clear_char_state(chr);
                return InputAction::FinalizeUse;
            }

            return InputAction::None;
        } else {
            // if key was held, do nothing when it is up to avoid a final press
            if is_held {
                self.clear_char_state(chr);
                return InputAction::None;
            } else {
                let action: InputAction = self.apply_char(chr, settings);

                self.clear_char_state(chr);

                return action;
            }
        }
    }

    /// Clear direction or target state for the given character, if applicable.
    fn clear_char_state(&mut self, chr: char) {
        if let Some(_input_dir) = InputDirection::from_chr(chr) {
            self.direction = None;
        }

        if let Some(_index) = SKILL_KEYS.iter().position(|key| *key == chr) {
            self.target = None;
        }

        if let Some(_index) = ITEM_KEYS.iter().position(|key| *key == chr) {
            self.target = None;
        }
    }

    fn apply_char(&mut self, chr: char, settings: &GameSettings) -> InputAction {
        let mut action: InputAction = InputAction::None;

        // check if the key being released is the one that set the input direction.
        if let Some(input_dir) = InputDirection::from_chr(chr) {
            if self.direction == Some(input_dir) {
                match input_dir {
                    InputDirection::Dir(dir) => {
                        if self.cursor {
                           action = InputAction::CursorMove(dir, self.ctrl, self.shift);
                        } else if self.alt {
                            action = InputAction::Interact(Some(dir));
                        } else {
                            action = InputAction::Move(dir, self.move_mode());
                        }
                    }

                    InputDirection::Current => {
                        // Pressing 5 in cursor mode does nothing, so only process
                        // if in direct mode.
                        if !self.cursor {
                            if self.alt {
                                action = InputAction::Interact(None);
                            } else {
                                action = InputAction::Pass(self.move_mode());
                            }
                        }
                    }
                }
            }
            // if releasing a key that is directional, but not the last directional key
            // pressed, then do nothing, waiting for the last key to be released instead.
        } else {
            // if releasing target, apply the skill or item
            if let Some(index) = SKILL_KEYS.iter().position(|key| *key == chr) {
                action = self.use_skill(index, settings);
            }

            // Item release does nothing outside of use-mode
            //if let Some(index) = ITEM_KEYS.iter().position(|key| *key == chr) {
            //    action = self.use_item(index, settings);
            //}

            // If we are not releasing a direction, skill, or item then try other keys.
            if action == InputAction::None {
                action = alpha_up_to_action(chr);
            }
        }

        return action;
    }

    fn use_item(&mut self, item_index: usize, settings: &GameSettings) -> InputAction {
        if self.cursor {
            if let Some(cursor_pos) = settings.cursor {
                // alternate is always used so you throw items
                return InputAction::ItemPos(cursor_pos, ActionMode::Alternate, item_index);
            } else {
                panic!("No cursor position while in cursor mode!");
            }
        } else {
            return InputAction::ItemFacing(self.action_mode(), item_index);
        }
    }

    fn use_skill(&mut self, skill_index: usize, settings: &GameSettings) -> InputAction {
        if self.cursor {
            if let Some(cursor_pos) = settings.cursor {
                return InputAction::SkillPos(cursor_pos, self.action_mode(), skill_index);
            } else {
                panic!("No cursor position while in cursor mode!");
            }
        } else {
            return InputAction::SkillFacing(self.action_mode(), skill_index);
        }
    }

    fn handle_char_down(&mut self, chr: char, settings: &GameSettings) -> InputAction {
        let mut action = InputAction::None;

        self.char_down_order.push(chr);

        if settings.state == GameState::Use {
            if let Some(input_dir) = InputDirection::from_chr(chr) {
                if let InputDirection::Dir(dir) = input_dir {
                    action = InputAction::StartUseDir(dir);
                }
            } else if chr == ' ' {
                action = InputAction::AbortUse;
            }
        } else if !settings.state.is_menu() {
            if chr == 'o' {
                action = InputAction::OverlayOn;
            }

            if chr == ' ' {
                self.cursor = !self.cursor;
                action = InputAction::CursorToggle;
            }

            if let Some(index) = SKILL_KEYS.iter().position(|key| *key == chr) {
                self.target = Some(Target::skill(index as usize));
            }

            if let Some(index) = ITEM_KEYS.iter().position(|key| *key == chr) {
                self.target = Some(Target::item(index as usize));

                if !self.cursor {
                    action = InputAction::StartUseItem(index as usize);
                    // directions are not used in use-mode
                    self.direction = None;
                }
            }

            if let Some(input_dir) = InputDirection::from_chr(chr) {
                self.direction = Some(input_dir);
            }
        }

        return action;
    }

    fn handle_char_held(&mut self, chr: char, time: Instant, settings: &GameSettings, config: &Config) -> InputAction {
        let mut action = InputAction::None;

        if let Some(held_state) = self.char_held.get(&chr) {
            // only process the last character as held
            if chr == *self.char_down_order.iter().last().unwrap() {
                let held_state = *held_state;
                let time_since = time.duration_since(held_state.down_time).as_secs_f32();

                let new_repeats = (time_since / config.repeat_delay) as usize;
                if new_repeats > held_state.repetitions {
                    action = self.apply_char(chr, settings);

                    if action == InputAction::OverlayOff   ||
                       action == InputAction::Inventory    ||
                       action == InputAction::SkillMenu    ||
                       action == InputAction::Exit         ||
                       action == InputAction::CursorToggle ||
                       action == InputAction::ClassMenu {
                        action = InputAction::None;
                    } else {
                        self.char_held.insert(chr, held_state.repeated());
                    }
                }
            }
        }

        return action;
    }

    fn handle_char(&mut self, chr: char, dir: KeyDir, time: Instant, settings: &GameSettings, config: &Config) -> InputAction {
        match dir {
            KeyDir::Up => {
                return self.handle_char_up(chr, settings);
            }

            KeyDir::Down => {
                return self.handle_char_down(chr, settings);
            }

            KeyDir::Held => {
                return self.handle_char_held(chr, time, settings, config);
            }
        }
    }
}

pub fn alpha_up_to_action(chr: char) -> InputAction {
    let input_action: InputAction;

    match chr {
        'q' => {
            input_action = InputAction::Exit;
        }

        'g' => {
            input_action = InputAction::Pickup;
        }

        'd' => {
            input_action = InputAction::DropItem;
        }

        'i' => {
            input_action = InputAction::Inventory;
        }

        'y' => {
            input_action = InputAction::Yell;
        }

        'l' => {
            input_action = InputAction::ExploreAll;
        }

        't' => {
            input_action = InputAction::GodMode;
        }

        'p' => {
            input_action = InputAction::RegenerateMap;
        }

        'e' => {
            input_action = InputAction::IncreaseMoveMode;
        }

        'w' => {
            input_action = InputAction::DecreaseMoveMode;
        }

        'o' => {
            input_action = InputAction::OverlayOff;
        }

        'j' => {
            input_action = InputAction::SkillMenu;
        }

        'h' => {
            input_action = InputAction::ClassMenu;
        }

        _ => {
            input_action = InputAction::None;
        }
    }

    return input_action;
}

fn direction_from_digit(chr: char) -> Option<Direction> {
    match chr {
        '4' => Some(Direction::Left),
        '6' => Some(Direction::Right),
        '8' => Some(Direction::Up),
        '2' => Some(Direction::Down),
        '1' => Some(Direction::DownLeft),
        '3' => Some(Direction::DownRight),
        '7' => Some(Direction::UpLeft),
        '9' => Some(Direction::UpRight),
        _ => None,
    }
}

