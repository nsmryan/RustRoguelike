use sdl2::keyboard::Keycode;

use roguelike_core::types::*;

use crate::input::KeyDirection;


#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Command {
    ListEntities,
    UnknownCommand,
}

impl Command {
    pub fn from_str(command_str: &str) -> Command {
        match command_str {
            "list-entities" => Command::ListEntities,
            _ => Command::UnknownCommand,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Console {
    pub input: String,
    pub output: Vec<String>,
    pub history: Vec<String>,
    pub visible: bool,
    pub height: u32,
    pub time_at_open: f32,
}

impl Console {
    pub fn new() -> Console {
        return Console {
            input: String::new(),
            output: Vec::new(),
            history: Vec::new(),
            visible: false,
            height: 0,
            time_at_open: 0.0,
        };
    }

    pub fn eval(&mut self,
                key: Keycode,
                dir: KeyDirection,
                data: &mut GameData) {

        if let Some((command, args)) = self.key(key, dir) {
            self.execute(command, args, data);
        }
    }

    pub fn key(&mut self,
                key: Keycode,
                dir: KeyDirection) -> Option<(Command, Vec<String>)> {
        use Keycode::*;

        let mut command = None;
        
        if dir == KeyDirection::Down {
            match key {
                A => self.input.push('a'),
                B => self.input.push('b'),
                C => self.input.push('c'),
                D => self.input.push('d'),
                E => self.input.push('e'),
                F => self.input.push('f'),
                G => self.input.push('g'),
                H => self.input.push('h'),
                I => self.input.push('i'),
                J => self.input.push('j'),
                K => self.input.push('k'),
                L => self.input.push('l'),
                N => self.input.push('n'),
                M => self.input.push('m'),
                O => self.input.push('o'),
                P => self.input.push('p'),
                Q => self.input.push('q'),
                R => self.input.push('r'),
                S => self.input.push('s'),
                T => self.input.push('t'),
                U => self.input.push('u'),
                V => self.input.push('v'),
                W => self.input.push('w'),
                X => self.input.push('x'),
                Y => self.input.push('y'),
                Z => self.input.push('z'),
                Num1 => self.input.push('1'),
                Num2 => self.input.push('2'),
                Num3 => self.input.push('3'),
                Num4 => self.input.push('4'),
                Num5 => self.input.push('5'),
                Num6 => self.input.push('6'),
                Num7 => self.input.push('7'),
                Num8 => self.input.push('8'),
                Num9 => self.input.push('9'),
                Num0 => self.input.push('0'),
                Space => self.input.push(' '),
                KpMinus | Minus => self.input.push('-'),
                Backspace => {
                    self.input.pop();
                }
                KpEnter | Return => {
                    let input = self.input.clone();
                    self.input.clear();
                    let mut words = input.split_whitespace();
                    if let Some(command_str) = words.next() {
                        let args = words.map(|s| s.to_string()).collect::<Vec<String>>();
                        command = Some((Command::from_str(command_str), args));
                    }

                    self.input.clear();
                }
                _ => println!("Console can't handle '{:?}'", key),
            }
        }

        return command;
    }

    pub fn execute(&mut self, command: Command, args: Vec<String>, data: &mut GameData) {
        match command {
            Command::ListEntities => {
                self.output.push("listing entities:".to_string());
            }

            Command::UnknownCommand => {
                self.output.push("unknown command:".to_string());
            }
        }
    }
}
