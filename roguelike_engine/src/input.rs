use std::time::Instant;
use std::collections::HashMap;
use std::cmp::Ord;

use serde::{Serialize, Deserialize};

use roguelike_core::types::*;
use roguelike_core::movement::Direction;
use roguelike_core::config::Config;

use crate::game::*;
use crate::actions::*;


const TARGET_CODES: &[char] = &['z', 'x', 'c', 'v', 'b'];

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum KeyDir {
    Up,
    Held,
    Down,
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
    Alt(KeyDir),
    MousePos(i32, i32),
    MouseButton(MouseClick, Pos, Option<Pos>, KeyDir), // button clicked, mouse position, screen square, keydir
    Esc,
    Tab,
    Quit,
}

#[derive(Clone, Debug)]
pub struct Input {
    pub chording: bool,
    pub mode: ActionMode,
    pub moding: bool,
    pub target: i32,
    pub char_held: HashMap<char, HeldState>,
}

impl Input {
    pub fn new() -> Input {
        return Input { chording: false,
                       mode: ActionMode::Primary,
                       moding: false,
                       target: -1,
                       char_held: HashMap::new()
        };
    }

    pub fn reset(&mut self) {
        self.target = -1;
    }

    pub fn handle_event(&mut self,
                        settings: &mut GameSettings,
                        event: InputEvent,
                        time: Instant,
                        config: &Config) -> InputAction {
        let mut action = InputAction::None;

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
                action = InputAction::SwapPrimaryItem;
            }

            InputEvent::Ctrl(dir) => {
                match dir {
                    KeyDir::Down => {
                        self.chording = true;
                    }

                    KeyDir::Up => {
                        self.chording = false;
                        self.reset();
                    }

                    KeyDir::Held => {}
                }
            }

            InputEvent::Alt(dir) => {
                if dir == KeyDir::Down {
                    self.mode = ActionMode::Alternate;
                } else if dir == KeyDir::Up {
                    self.mode = ActionMode::Primary;
                }
            }

            InputEvent::Char(chr, dir) => {
                match dir {
                    KeyDir::Up => {
                        // if key was held, do nothing when it is up to avoid a final press
                        if let Some(held_state) = self.char_held.get(&chr) {
                            if held_state.repetitions > 0 {
                                return InputAction::None;
                            }
                        }
                        self.char_held.remove(&chr);

                        // NOTE this could be moved to the normal mapping as it doesn't
                        // rely on ctrl anymore
                        for (index, target_chr) in TARGET_CODES.iter().enumerate() {
                            if chr == *target_chr {
                                self.target = index as i32;
                            }
                        }
                        action = self.key_to_action(chr, dir, settings, config); }

                    KeyDir::Down => {
                        if chr == 'o' {
                            action = InputAction::OverlayOn;
                        }
                    }

                    KeyDir::Held => {
                        if let Some(held_state) = self.char_held.get(&chr) {
                            let held_state = *held_state;
                            let time_since = time.duration_since(held_state.down_time).as_secs_f32();

                            let new_repeats = (time_since / config.repeat_delay) as usize;
                            if new_repeats > held_state.repetitions {
                                action = self.key_to_action(chr, dir, settings, config);

                                if action == InputAction::OverlayOff {
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

    fn key_to_action(&mut self, chr: char, _dir: KeyDir, settings: &GameSettings, config: &Config) -> InputAction {
        let mut action;

        if (self.chording || self.target != -1) && chr.is_ascii_digit() {
            let dir = from_digit(chr);
            action = InputAction::Chord(dir, self.mode, self.target);
            self.reset();
        } else if chr == ' ' {
            action = InputAction::CursorApply(self.mode, self.target);
            self.reset();
        } else {
            action = keyup_to_action(chr, settings.state);

            if config.use_cursor {
               if let InputAction::Move(dir) = action {
                    action = InputAction::CursorMove(dir);
               }
            }
        }

        return action;
    }
}

pub fn keyup_to_action(chr: char, game_state: GameState) -> InputAction {
    let input_action: InputAction;

    if chr.is_ascii_digit() {
        if game_state.is_menu() {
            return InputAction::SelectItem(chr.to_digit(10).unwrap() as usize);
        } else if chr == '5' {
            return InputAction::Pass;
        } else if let Some(dir) = from_digit(chr) {
            return InputAction::Move(dir);
        }
    }

    match chr {
        'a' => {
            input_action = InputAction::Interact;
        }

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

        '`' => {
            input_action = InputAction::ToggleConsole;
        }

        'u' => {
            input_action = InputAction::UseItem;
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

