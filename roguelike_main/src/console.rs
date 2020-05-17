use sdl2::keyboard::Keycode;

use roguelike_core::types::*;
use roguelike_core::messaging::*;
use roguelike_core::config::*;

use roguelike_engine::actions::KeyDirection;
use roguelike_engine::generation::*;
use roguelike_engine::game::GameSettings;

use crate::display::*;


#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Command {
    ListIds,
    SetXY,
    PlayerSee,
    PlayerXY,
    RenderMap,
    ListCommands,
    Gol,
    Elf,
    PlayerHp,
    Clear,
    Quit,
    UnknownCommand,
}

impl Command {
    pub fn from_str(command_str: &str) -> Command {
        match command_str {
            "list-ids" => Command::ListIds,
            "set-xy" => Command::SetXY,
            "player-see" => Command::PlayerSee,
            "player-xy" => Command::PlayerXY,
            "render-map" => Command::RenderMap,
            "help" => Command::ListCommands,
            "list" => Command::ListCommands,
            "gol" => Command::Gol,
            "elf" => Command::Elf,
            "player-hp" => Command::PlayerHp,
            "clear" => Command::Clear,
            "quit" => Command::Quit,
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
                data: &mut GameData,
                display_state: &mut DisplayState,
                settings: &mut GameSettings,
                config: &Config,
                msg_log: &mut MsgLog) {
        if let Some((command, mut args)) = self.key(key, dir) {
            self.execute(command, &mut args, data, display_state, settings, config, msg_log);
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

    pub fn execute(&mut self,
                   command: Command,
                   args: &mut Vec<String>,
                   data: &mut GameData,
                   display_state: &mut DisplayState,
                   settings: &mut GameSettings,
                   config: &Config,
                   msg_log: &mut MsgLog) {
        self.output.clear();

        match command {
            Command::ListIds => {
                let mut line = String::new();
                let mut count = 0;
                let combine = 2;
                for entity_id in data.entities.ids.iter() {
                    line.push_str(&format!("{:>3}: at ({:>2}, {:>2}) {:<16}",
                                           entity_id,
                                           data.entities.pos[entity_id].x,
                                           data.entities.pos[entity_id].y,
                                           data.entities.name[entity_id]));

                    count += 1;

                    if count == combine {
                        self.output.push(line.clone());
                        line.clear();
                        count = 0;
                    }
                }
            }

            Command::SetXY => {
                let y = args.pop().unwrap().parse::<i32>().unwrap();
                let x = args.pop().unwrap().parse::<i32>().unwrap();
                let id = args.pop().unwrap().parse::<u64>().unwrap();

                let mut entity_key = None;
                for key in data.entities.ids.iter() {
                    if *key == id {
                        entity_key = Some(key);
                    }
                }

                if let Some(key) = entity_key {
                    data.entities.set_xy(*key, x, y);
                    self.output.push(format!("{} moved to ({}, {})", id, x, y));
                } else {
                    self.output.push(format!("Id {} not found!", id));
                }
            }

            Command::PlayerSee => {
                let y = args.pop().unwrap().parse::<i32>().unwrap();
                let x = args.pop().unwrap().parse::<i32>().unwrap();

                let player = data.find_player().unwrap();

                let player_pos = data.entities.pos[&player];
                let can_see = data.map.is_in_fov(player_pos, Pos::new(x, y), config.fov_radius_player);

                if can_see {
                    self.output.push(format!("yes"));
                } else {
                    self.output.push(format!("no"));
                }
            }

            Command::PlayerXY => {
                let y = args.pop().unwrap().parse::<i32>().unwrap();
                let x = args.pop().unwrap().parse::<i32>().unwrap();

                let player = data.find_player().unwrap();

                data.entities.set_xy(player, x, y);
                self.output.push(format!("Player at ({}, {})", x, y));
            }

            Command::RenderMap => {
                let setting = args.pop().unwrap();

                if setting == "on" {
                    settings.render_map = true;
                } else if setting == "off" {
                    settings.render_map = false;
                } else {
                    self.output.push(format!("Expected 'on' or 'off', got '{}'", setting));
                }
            }

            Command::ListCommands => {
                self.output.push(format!("list-ids"));
                self.output.push(format!("set-xy id x y"));
                self.output.push(format!("player-see x y"));
                self.output.push(format!("render-map (on|off)"));
                self.output.push(format!("player-xy x y"));
                self.output.push(format!("gol x y"));
                self.output.push(format!("elf x y"));
                self.output.push(format!("player-hp (amount)"));
                self.output.push(format!("clear"));
                self.output.push(format!("list"));
                self.output.push(format!("help"));
                self.output.push(format!("quit"));
            }

            Command::Gol => {
                let y = args.pop().unwrap().parse::<i32>().unwrap();
                let x = args.pop().unwrap().parse::<i32>().unwrap();

                let gol = make_gol(&mut data.entities, config, Pos::new(x, y), msg_log);
                self.output.push(format!("Added gol at ({}, {}), id = {}", x, y, gol));
            }

            Command::Elf => {
                let y = args.pop().unwrap().parse::<i32>().unwrap();
                let x = args.pop().unwrap().parse::<i32>().unwrap();

                let elf = make_elf(&mut data.entities, config, Pos::new(x, y), msg_log);
                self.output.push(format!("Added elf at ({}, {}), id = {}", x, y, elf));
            }

            Command::PlayerHp => {
                let hp = args.pop().unwrap().parse::<i32>().unwrap();

                let player = data.find_player().unwrap();
                data.entities.fighter[&player].hp = hp;

                self.output.push(format!("Player HP set to {}", hp));
            }

            Command::Clear => {
                self.output.clear();
            }

            Command::Quit => {
                settings.exiting = true;
            }

            Command::UnknownCommand => {
                self.output.push("unknown command".to_string());
            }
        }
    }
}
