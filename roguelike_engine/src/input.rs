use std::time::Instant;
use std::collections::HashMap;
use std::cmp::Ord;
use std::str::FromStr;

use serde::{Serialize, Deserialize};

use roguelike_core::types::*;
use roguelike_core::movement::Direction;
use roguelike_core::config::Config;
use roguelike_core::utils::{add_pos, sub_pos, scale_pos, in_direction_of};

use crate::game::*;
use crate::actions::*;


const TARGET_CODES: &[char] = &['z', 'x', 'c', 'v', 'b'];

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

#[derive(Clone, Debug, Copy, PartialEq, Serialize, Deserialize)]
pub enum Cursor {
    Absolute(Pos),
    Relative(Pos),
}

impl Cursor {
    pub fn new() -> Cursor {
        return Cursor::Absolute(Pos::new(0, 0));
    }

    pub fn pos(&self, target_pos: Pos) -> Pos {
        match self {
            Cursor::Absolute(pos) => *pos,
            Cursor::Relative(offset) => add_pos(target_pos, *offset),
        }
    }

    pub fn toggle(&mut self, target_pos: Pos) {
        match self {
            Cursor::Absolute(pos) => {
                let towards_pos = in_direction_of(target_pos, *pos);
                *self = Cursor::Relative(sub_pos(towards_pos, target_pos));
            }

            Cursor::Relative(offset) => {
                *self = Cursor::Absolute(add_pos(target_pos, *offset));
            }
        }
    }

    pub fn return_cursor(&mut self, target_pos: Pos) {
        match self {
            Cursor::Absolute(_) => *self = Cursor::Absolute(target_pos),
            Cursor::Relative(_) => *self = Cursor::Relative(Pos::new(0, 0)),
        }
    }

    pub fn move_by(&mut self, target_pos: Pos, dir: Direction, dist: i32, map_size: (i32, i32)) {
        let mut new_pos: Pos = self.pos(target_pos);

        let dir_move: Pos = scale_pos(dir.into_move(), dist);

        let moved_pos: Pos = 
            match self {
                Cursor::Absolute(pos) => add_pos(*pos, dir_move),
                Cursor::Relative(_) => add_pos(target_pos, dir_move),
            };

        if moved_pos.x >= 0 && moved_pos.y >= 0 &&
           moved_pos.x < map_size.0 && moved_pos.y < map_size.1 {
            new_pos = moved_pos;
        }

        match self {
            Cursor::Absolute(_) => *self = Cursor::Absolute(new_pos),
            Cursor::Relative(offset) => *self = Cursor::Relative(sub_pos(new_pos, target_pos)),
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
    pub chording: bool,
    pub mode: ActionMode,
    pub moding: bool,
    pub shifting: bool,
    pub target: i32,
    pub char_held: HashMap<char, HeldState>,
}

impl Input {
    pub fn new() -> Input {
        return Input { chording: false,
                       mode: ActionMode::Primary,
                       moding: false,
                       shifting: false,
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
                action = InputAction::CursorReturn;
            }

            InputEvent::Ctrl(dir) => {
                match dir {
                    KeyDir::Down => {
                        self.chording = true;
                        self.mode = ActionMode::Primary;
                    }

                    KeyDir::Up => {
                        self.chording = false;
                        self.reset();
                    }

                    KeyDir::Held => {}
                }
            }

            InputEvent::Shift(dir) => {
                if dir != KeyDir::Held {
                    self.shifting = dir == KeyDir::Down;
                }
            }

            InputEvent::Alt(dir) => {
                if dir == KeyDir::Down {
                    self.mode = ActionMode::Alternate;
                    self.moding = true;
                } else if dir == KeyDir::Up {
                    self.moding = false;
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
                        action = self.key_to_action(chr, dir, settings, config);

                        if !self.moding {
                          self.mode = ActionMode::Primary;
                        }
                    }

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

                                self.char_held.insert(chr, held_state.repeated());
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

        if (self.chording || self.moding || self.target != -1) && chr.is_ascii_digit() {
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
                   action = InputAction::CursorMove(dir, self.shifting);
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

        'r' => {
            input_action = InputAction::CursorStateToggle;
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

