use sdl2::event::Event;
use sdl2::mouse::MouseButton;
use sdl2::keyboard::{Keycode, Scancode, Mod};

use roguelike_core::types::*;
use roguelike_core::movement::Direction;
use roguelike_core::config::Config;

use roguelike_engine::game::*;
use roguelike_engine::actions::*;

use crate::display::*;


#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum KeyDir {
    Up,
    Down,
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum MouseClick {
    Left,
    Right,
    Middle,
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum InputEvent {
    Char(char, KeyDir),
    Ctrl(KeyDir),
    Alt(KeyDir),
    MousePos(i32, i32),
    MouseButton(MouseClick, KeyDir),
    Esc,
    Tab,
    Quit,
}

impl InputEvent {
    pub fn read_event(event: Event) -> Option<InputEvent> {
        match event {
            Event::Quit {..} => {
                return Some(InputEvent::Quit);
            }

            // TODO could merge KeyDown and KeyUp
            Event::KeyDown {keycode, repeat, ..} => {
                if repeat {
                    return None;
                }

                if let Some(key) = keycode {
                    if let Some(chr) = keycode_to_char(key) {
                        return Some(InputEvent::Char(chr, KeyDir::Down));
                    } else if key == Keycode::LCtrl || key == Keycode::RCtrl {
                        return Some(InputEvent::Ctrl(KeyDir::Down));
                    } else if key == Keycode::LAlt || key == Keycode::RAlt {
                        return Some(InputEvent::Alt(KeyDir::Down));
                    } else {
                        // NOTE could check for LShift, RShift
                        return None;
                    }
                }

                return None;
            }

            Event::KeyUp {keycode, repeat, ..} => {
                if repeat {
                    return None;
                }

                if let Some(key) = keycode {
                    if let Some(chr) = keycode_to_char(key) {
                        return Some(InputEvent::Char(chr, KeyDir::Up));
                    } else if key == Keycode::LCtrl || key == Keycode::RCtrl {
                        return Some(InputEvent::Ctrl(KeyDir::Up));
                    } else if key == Keycode::LAlt || key == Keycode::RAlt {
                        return Some(InputEvent::Alt(KeyDir::Up));
                    } else if key == Keycode::Tab {
                        return Some(InputEvent::Tab);
                    } else if key == Keycode::Escape {
                        return Some(InputEvent::Esc);
                    } else {
                        // NOTE could check for LShift, RShift
                        return None;
                    }
                }

                return None;
            }

            Event::MouseMotion {x, y, ..} => {
                return Some(InputEvent::MousePos(x, y));
            }

            Event::MouseButtonDown {mouse_btn, ..} => {
                let click = match mouse_btn {
                    MouseButton::Left => MouseClick::Left,
                    MouseButton::Right => MouseClick::Right,
                    MouseButton::Middle => MouseClick::Middle,
                    _ => return None,
                };
                return Some(InputEvent::MouseButton(click, KeyDir::Down));
            }

            Event::MouseButtonUp {mouse_btn, ..} => {
                let click = match mouse_btn {
                    MouseButton::Left => MouseClick::Left,
                    MouseButton::Right => MouseClick::Right,
                    MouseButton::Middle => MouseClick::Middle,
                    _ => return None,
                };
                return Some(InputEvent::MouseButton(click, KeyDir::Up));
            }

            _ => {
                return None;
            }
        }
    }
}

pub fn keycode_to_char(key: Keycode) -> Option<char> {
    match key {
        Keycode::Space => Some(' '),
        Keycode::Comma => Some(','),
        Keycode::Minus => Some('-'),
        Keycode::Period => Some('.'),
        Keycode::Num0 => Some('0'),
        Keycode::Num1 => Some('1'),
        Keycode::Num2 => Some('2'),
        Keycode::Num3 => Some('3'),
        Keycode::Num4 => Some('4'),
        Keycode::Num5 => Some('5'),
        Keycode::Num6 => Some('6'),
        Keycode::Num7 => Some('7'),
        Keycode::Num8 => Some('8'),
        Keycode::Num9 => Some('9'),
        Keycode::A => Some('a'),
        Keycode::B => Some('b'),
        Keycode::C => Some('c'),
        Keycode::D => Some('d'),
        Keycode::E => Some('e'),
        Keycode::F => Some('f'),
        Keycode::G => Some('g'),
        Keycode::H => Some('h'),
        Keycode::I => Some('i'),
        Keycode::J => Some('j'),
        Keycode::K => Some('k'),
        Keycode::L => Some('l'),
        Keycode::M => Some('m'),
        Keycode::N => Some('n'),
        Keycode::O => Some('o'),
        Keycode::P => Some('p'),
        Keycode::Q => Some('q'),
        Keycode::R => Some('r'),
        Keycode::S => Some('s'),
        Keycode::T => Some('t'),
        Keycode::U => Some('u'),
        Keycode::V => Some('v'),
        Keycode::W => Some('w'),
        Keycode::X => Some('x'),
        Keycode::Y => Some('y'),
        Keycode::Z => Some('z'),
        Keycode::Right => Some('6'),
        Keycode::Left => Some('4'),
        Keycode::Down => Some('2'),
        Keycode::Up => Some('8'),
        Keycode::Kp0 => Some('0'),
        Keycode::Kp1 => Some('1'),
        Keycode::Kp2 => Some('2'),
        Keycode::Kp3 => Some('3'),
        Keycode::Kp4 => Some('4'),
        Keycode::Kp5 => Some('5'),
        Keycode::Kp6 => Some('6'),
        Keycode::Kp7 => Some('7'),
        Keycode::Kp8 => Some('8'),
        Keycode::Kp9 => Some('9'),
        Keycode::KpPeriod => Some('.'),
        Keycode::KpSpace => Some(' '),
        Keycode::Backquote => Some('`'),
        _ => None,
    }
}

const TARGET_CODES: &[char] = &['z', 'x', 'c', 'v', 'b'];


#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Input {
    chording: bool,
    chorded: bool,
    mode: ActionMode,
    target: i32,
}

impl Input {
    pub fn new() -> Input {
        return Input { chording: false, chorded: false, mode: ActionMode::Primary, target: -1 };
    }

    pub fn reset(&mut self) {
        self.chording = false;
        self.chorded = false;
        self.mode = ActionMode::Primary;
        self.target = -1;
    }

    // TODO return an Option<InputAction> instead of modifying the game.
    // possibly make game & instead of &mut
    // try to reduce inputs and dependencies here
    pub fn handle_event(&mut self,
                        game: &mut Game,
                        display: &mut Display,
                        event: InputEvent) {
        match event {
            InputEvent::Quit => {
                game.settings.running = false;
            }

            InputEvent::Esc => {
                game.input_action = InputAction::Esc;
            }

            InputEvent::Tab => {
                game.input_action = InputAction::SwapPrimaryItem;
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
            }

            InputEvent::Char(chr, dir) => {
                match dir {
                    KeyDir::Up => {
                        if self.chorded && chr.is_ascii_digit() {
                            let dir = from_digit(chr);
                            game.input_action = InputAction::Chord(dir, ActionStrength::Weak, self.mode, self.target);
                            self.reset();
                        } else if self.chording {
                            for (index, target_chr) in TARGET_CODES.iter().enumerate() {
                                if chr == *target_chr {
                                       self.target = index as i32;
                                }
                            }
                        } else if chr == ' ' {
                            game.input_action = InputAction::CursorApply(self.mode, self.target);
                            self.reset();
                        } else {
                            game.input_action = keyup_to_action(chr, game.settings.state);

                            if game.config.use_cursor {
                               if let InputAction::Move(dir) = game.input_action {
                                    game.input_action = InputAction::CursorMove(dir);
                               }
                            }
                        }
                    }

                    KeyDir::Down => {
                        if chr == 'o' {
                            game.input_action = InputAction::OverlayOn;
                        }
                    }
                }
            }

            InputEvent::MousePos(x, y) => {
                game.mouse_state.x = x;
                game.mouse_state.y = y;
            }

            InputEvent::MouseButton(clicked, dir) => {
                let down = dir == KeyDir::Down;
                match clicked {
                    MouseClick::Left => {
                        game.mouse_state.left_pressed = down;

                        if down {
                            let (map_width, map_height) = game.data.map.size();
                            let option_mouse_cell =
                                display.targets.mouse_pos(game.mouse_state.x, game.mouse_state.y, map_width, map_height);

                            if let Some(mouse_cell) = option_mouse_cell {
                                let screen_pos = Pos::new(game.mouse_state.x, game.mouse_state.y);
                                let mouse_pos = Pos::new(mouse_cell.0, mouse_cell.1);
                                game.input_action = InputAction::MapClick(screen_pos, mouse_pos);
                            }
                        }
                    }

                    MouseClick::Middle => {
                        game.mouse_state.middle_pressed = down;
                    }

                    MouseClick::Right => {
                        game.mouse_state.right_pressed = down;
                    }
                }
            }
        }
    }
}

// TODO game_state might not be necessary- split out processing to states themselves
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

