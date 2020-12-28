use serde::{Serialize, Deserialize};

use roguelike_core::types::*;
use roguelike_core::movement::Direction;

use crate::game::*;
use crate::actions::*;


const TARGET_CODES: &[char] = &['z', 'x', 'c', 'v', 'b'];

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum KeyDir {
    Up,
    Down,
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum MouseClick {
    Left,
    Right,
    Middle,
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

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Input {
    chording: bool,
    chorded: bool,
    mode: ActionMode,
    moding: bool,
    target: i32,
}

impl Input {
    pub fn new() -> Input {
        return Input { chording: false, chorded: false, mode: ActionMode::Primary, moding: false, target: -1 };
    }

    pub fn reset(&mut self) {
        if !self.chording {
            self.chorded = false;
        }

        if !self.moding {
            self.mode = ActionMode::Primary;
        }

        self.target = -1;
    }

    pub fn handle_event(&mut self,
                        game: &Game,
                        event: InputEvent) -> InputAction {
        let mut action = InputAction::None;
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
                        self.chorded = true;
                        self.mode = ActionMode::Primary;
                    }

                    KeyDir::Up => {
                        self.chording = false;
                    }
                }
            }

            InputEvent::Alt(dir) => {
                if dir == KeyDir::Down && self.chording {
                    self.mode = ActionMode::Alternate;
                }

                self.moding = dir == KeyDir::Down;
            }

            InputEvent::Char(chr, dir) => {
                match dir {
                    KeyDir::Up => {
                        if (self.chording || self.target != -1) && chr.is_ascii_digit() {
                            let dir = from_digit(chr);
                            action = InputAction::Chord(dir, self.mode, self.target);
                            self.reset();
                        } else if self.chording {
                            for (index, target_chr) in TARGET_CODES.iter().enumerate() {
                                if chr == *target_chr {
                                       self.target = index as i32;
                                }
                            }
                        } else if chr == ' ' {
                            action = InputAction::CursorApply(self.mode, self.target);
                            self.reset();
                        } else {
                            action = keyup_to_action(chr, game.settings.state);

                            if game.config.use_cursor {
                               if let InputAction::Move(dir) = action {
                                    action = InputAction::CursorMove(dir);
                               }
                            }
                        }
                    }

                    KeyDir::Down => {
                        if chr == 'o' {
                            action = InputAction::OverlayOn;
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

        'v' => {
            input_action = InputAction::ExploreAll;
        }

        't' => {
            input_action = InputAction::GodMode;
        }

        'p' => {
            input_action = InputAction::RegenerateMap;
        }

        'x' => {
            input_action = InputAction::IncreaseMoveMode;
        }

        'z' => {
            input_action = InputAction::DecreaseMoveMode;
        }

        'o' => {
            input_action = InputAction::OverlayOff;
        }

        's' => {
            input_action = InputAction::SkillMenu;
        }

        'c' => {
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

