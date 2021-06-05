use std::time::Instant;
use std::collections::HashMap;
use std::cmp::Ord;
use std::str::FromStr;

use serde::{Serialize, Deserialize};

use roguelike_core::constants::*;
use roguelike_core::types::*;
use roguelike_core::movement::Direction;
use roguelike_core::config::Config;
use roguelike_core::movement::MoveMode;

use crate::game::*;
use crate::actions::*;


const TARGET_CODES: &[char] = &['z', 'x', 'c', 'v', 'b', 'n'];

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
        } else if s == "up" {
            return Ok(KeyDir::Up);
        }

        panic!(format!("KeyDir {} unexpected", s));
    }
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum Target {
    Item(usize),
    Skill(usize),
}

impl Target {
    pub fn from_index(index: usize) -> Target {
        if index < PLAYER_MAX_ITEMS {
            return Target::Item(index);
        } else {
            return Target::Skill(index - PLAYER_MAX_ITEMS);
        }
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
    pub cursor: bool,
    pub char_held: HashMap<char, HeldState>,
}

impl Input {
    pub fn new() -> Input {
        return Input { ctrl: false,
                       alt: false,
                       shift: false,
                       target: None,
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
                action = InputAction::Exit;
            }

            InputEvent::Esc => {
                action = InputAction::Esc;
            }

            InputEvent::Tab => {
                action = InputAction::CursorReturn;
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
                match dir {
                    KeyDir::Up => {
                        // if key was held, do nothing when it is up to avoid a final press
                        if self.is_held(chr) {
                            return InputAction::None;
                        }
                        self.char_held.remove(&chr);

                        // NOTE this could be moved to the normal mapping
                        for (index, target_chr) in TARGET_CODES.iter().enumerate() {
                            if chr == *target_chr {
                                let target = Target::from_index(index as usize);

                                if self.cursor {
                                    self.target = None;

                                    if let Target::Item(index) = target {
                                        // alternate is used so you throw items
                                        return InputAction::CursorApplyItem(ActionMode::Alternate, index);
                                    } else if let Target::Skill(index) = target {
                                        // TODO should this be primary or alternate?
                                        return InputAction::CursorApplySkill(ActionMode::Alternate, index);
                                    }
                                } else {
                                    // target keys don't do anything outside of cursor mode,
                                    // so just return here.
                                    // NOTE should we still allow skills, but only ones that make
                                    // sense with no location?
                                    return InputAction::None;
                                }
                            }
                        }

                        action = self.key_to_action(chr, settings);
                    }

                    KeyDir::Down => {
                        if chr == 'o' {
                            action = InputAction::OverlayOn;
                        }

                        for (index, target_chr) in TARGET_CODES.iter().enumerate() {
                            if chr == *target_chr {
                                self.target = Some(Target::from_index(index as usize));
                            }
                        }
                    }

                    KeyDir::Held => {
                        if let Some(held_state) = self.char_held.get(&chr) {
                            let held_state = *held_state;
                            let time_since = time.duration_since(held_state.down_time).as_secs_f32();

                            let new_repeats = (time_since / config.repeat_delay) as usize;
                            if new_repeats > held_state.repetitions {
                                action = self.key_to_action(chr, settings);

                                if action == InputAction::OverlayOff   ||
                                   action == InputAction::Inventory    ||
                                   action == InputAction::SkillMenu    ||
                                   action == InputAction::ClassMenu {
                                    action = InputAction::None;
                                } else if action == InputAction::CursorToggle {
                                    // this is a little kludgy, but we have to untoggle cursor
                                    // mode as it was toggled by key_to_action.
                                    self.cursor = !self.cursor;
                                    action = InputAction::None;
                                } else {
                                    self.char_held.insert(chr, held_state.repeated());
                                }
                            }
                        }
                    }
                }
            }

            InputEvent::MouseButton(clicked, mouse_pos, target_pos, dir) => {
                action = InputAction::MouseButton(clicked, dir);

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
            }
        }

        return action;
    }

    fn key_to_action(&mut self, chr: char, settings: &GameSettings) -> InputAction {
        let action;

        // handle numeric characters first
        if chr.is_ascii_digit() {
            if settings.state.is_menu() {
                action = InputAction::SelectItem(chr.to_digit(10).unwrap() as usize);
            } else if chr == '5' {
                if self.alt {
                    action = InputAction::Interact(None);
                } else {
                    if let Some(Target::Item(index)) = self.target {
                        action = InputAction::DropItemByIndex(index);
                    } else {
                        action = InputAction::Pass;
                    }
                }
            } else if let Some(dir) = from_digit(chr) {
                if self.cursor {
                   action = InputAction::CursorMove(dir, self.ctrl, self.shift);
                } else if self.alt {
                    action = InputAction::Interact(Some(dir));
                } else if let Some(Target::Item(index)) = self.target {
                    action = InputAction::UseItem(dir, index);
                    self.target = None;
                } else {
                    action = InputAction::Move(dir, self.move_mode());
                }
            } else {
                action = InputAction::None;
            }
        } else if chr == ' ' {
            self.cursor = !self.cursor;
            action = InputAction::CursorToggle;
        } else {
            action = alpha_up_to_action(chr);
        }

        return action;
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

        's' => {
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

pub fn from_digit(chr: char) -> Option<Direction> {
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

