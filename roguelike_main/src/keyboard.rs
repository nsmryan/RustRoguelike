use sdl2::event::Event;
use sdl2::mouse::MouseButton;
use sdl2::keyboard::Keycode;

use roguelike_map::map::Pos;

use roguelike_engine::input::*;


pub fn translate_event(event: Event) -> Option<InputEvent> {
    match event {
        Event::Quit {..} => {
            return Some(InputEvent::Quit);
        }

        Event::KeyDown {keycode, repeat, ..} => {
            let mut dir = KeyDir::Down;

            if repeat {
                dir = KeyDir::Held;
            }

            if let Some(key) = keycode {
                if let Some(chr) = keycode_to_char(key) {
                    return Some(InputEvent::Char(chr, dir));
                } else if key == Keycode::LCtrl || key == Keycode::RCtrl {
                    return Some(InputEvent::Ctrl(dir));
                } else if key == Keycode::LAlt || key == Keycode::RAlt {
                    return Some(InputEvent::Alt(dir));
                } else if key == Keycode::LShift || key == Keycode::RShift {
                    return Some(InputEvent::Shift(dir));
                } else if key == Keycode::KpEnter || key == Keycode::Return {
                    return Some(InputEvent::Enter(KeyDir::Down));
                } else {
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
                } else if key == Keycode::LShift || key == Keycode::RShift {
                    return Some(InputEvent::Shift(KeyDir::Up));
                } else if key == Keycode::KpEnter || key == Keycode::Return {
                    return Some(InputEvent::Enter(KeyDir::Up));
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

        Event::MouseButtonDown {mouse_btn, x, y, ..} => {
            let click;
            match mouse_btn {
                MouseButton::Left => {
                    click = MouseClick::Left;
                }

                MouseButton::Right => {
                    click = MouseClick::Right;
                }

                MouseButton::Middle => {
                    click = MouseClick::Middle;
                }

                _ => return None,
            };

            let mouse_pos = Pos::new(x, y);
            return Some(InputEvent::MouseButton(click, mouse_pos, KeyDir::Down));
        }

        Event::MouseButtonUp {mouse_btn, x, y, ..} => {
            let click;
            match mouse_btn {
                MouseButton::Left => {
                    click = MouseClick::Left;
                }

                MouseButton::Right => {
                    click = MouseClick::Right;
                }

                MouseButton::Middle => {
                    click = MouseClick::Middle;
                }

                _ => return None,
            };

            let mouse_pos = Pos::new(x, y);
            return Some(InputEvent::MouseButton(click, mouse_pos, KeyDir::Up));
        }

        _ => {
            return None;
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
        Keycode::LeftBracket => Some('['),
        Keycode::RightBracket => Some(']'),
        Keycode::Backquote => Some('`'),
        Keycode::Backslash => Some('\\'),
        Keycode::Question => Some('?'),
        Keycode::Slash => Some('/'),
        _ => None,
    }
}

