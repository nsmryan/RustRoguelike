use std::str::FromStr;

use roguelike_core::types::*;
use roguelike_engine::game::*;


// TODO
// generation: make each entity, remove entity
// components: get and set
// map: create from MapConfigs
//      redo map with current config
//      get/set tiles
// input: step from key
// action: step from InputAction
//
// possibly:
// inject messages?
// change settings
// change config
// line positions?
// blocked pos, blocked path, etc
// fov

pub enum Command {
    PlayerId,
    Pos(u64),
    SetPos(u64, i32, i32),
    Exit,
}

impl FromStr for Command {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();

        let args = s.split(" ").collect::<Vec<&str>>();
        let cmd = args[0];

        if cmd == "player_id" {
            return Ok(Command::PlayerId);
        } else if cmd == "pos" {
            let id = args[1].parse::<u64>().unwrap();
            if args.len() == 2 {
                return Ok(Command::Pos(id));
            } else {
                let x  = args[2].parse::<i32>().unwrap();
                let y  = args[3].parse::<i32>().unwrap();
                return Ok(Command::SetPos(id, x, y));
            }
        } else if cmd == "exit" {
            return Ok(Command::Exit);
        }
        panic!(format!("Command {} not expected!", cmd));
    }
}

pub fn execute(command: &Command, game: &mut Game) -> String {
    match command {
        Command::PlayerId => {
            let player_id = game.data.find_by_name(EntityName::Player).unwrap();
            return player_id.to_string();
        }

        Command::Pos(id) => {
            let pos = game.data.entities.pos[id];
            return format!("{} {}", pos.x, pos.y);
        }

        Command::SetPos(id, x, y) => {
            game.data.entities.pos[id] = Pos::new(*x, *y);
            return "".to_string();
        }

        Command::Exit => {
            game.settings.running = false;
            return "".to_string();
        }

        // let action = game.input.handle_event(&mut game.settings, event, frame_time, &game.config);
        // game.step_game(input_action, dt);
    }
}

