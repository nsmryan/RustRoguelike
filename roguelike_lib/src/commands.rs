use std::str::FromStr;
use std::time::Instant;

use roguelike_core::types::*;
use roguelike_engine::input::*;
use roguelike_engine::game::*;
use roguelike_engine::generation::*;


// TODO
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
// convenience, like nearby entity ids, or all entity list

// MousePos(i32, i32),
// MouseButton(MouseClick, Pos, Option<Pos>, KeyDir),
// Esc,
// Tab,
// Quit,

//  Move(Direction),
//  Pass,
//  MapClick(Pos, Pos), // map loc, map cell
//  MouseButton(MouseClick, KeyDir),
//  Pickup,
//  DropItem,
//  SwapPrimaryItem,
//  Inventory,
//  SkillMenu,
//  ClassMenu,
//  Exit,
//  Esc,
//  ExploreAll,
//  RegenerateMap,
//  GodMode,
//  Yell,
//  IncreaseMoveMode,
//  DecreaseMoveMode,
//  OverlayOn,
//  OverlayOff,
//  SelectItem(usize),
//  ToggleConsole,
//  UseItem,
//  Interact,
//  Chord(Option<Direction>, ActionMode, ActionTarget),
//  CursorMove(Direction),
//  CursorApply(ActionMode, ActionTarget),
//  None,

pub enum GameCmd {
    PlayerId,
    Pos(u64),
    SetPos(u64, i32, i32),
    Make(EntityName, i32, i32),
    Remove(u64),
    Key(char, KeyDir),
    Ctrl(KeyDir),
    Alt(KeyDir),
    Exit,
}

impl FromStr for GameCmd {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();

        let args = s.split(" ").collect::<Vec<&str>>();
        let cmd = args[0];

        if cmd == "player_id" {
            return Ok(GameCmd::PlayerId);
        } else if cmd == "pos" {
            let id = args[1].parse::<u64>().unwrap();
            if args.len() == 2 {
                return Ok(GameCmd::Pos(id));
            } else {
                let x  = args[2].parse::<i32>().unwrap();
                let y  = args[3].parse::<i32>().unwrap();
                return Ok(GameCmd::SetPos(id, x, y));
            }
        } else if cmd == "make" {
            let entity_name = args[1].parse::<EntityName>().unwrap();
            let x  = args[2].parse::<i32>().unwrap();
            let y  = args[3].parse::<i32>().unwrap();

            return Ok(GameCmd::Make(entity_name, x, y));
        } else if cmd == "remove" {
            let id = args[1].parse::<u64>().unwrap();
            return Ok(GameCmd::Remove(id));
        } else if cmd == "key" {
            let chr = args[1].parse::<char>().unwrap();
            let dir = args[2].parse::<KeyDir>().unwrap();
            return Ok(GameCmd::Key(chr, dir));
        } else if cmd == "ctrl" {
            let dir = args[1].parse::<KeyDir>().unwrap();
            return Ok(GameCmd::Ctrl(dir));
        } else if cmd == "alt" {
            let dir = args[1].parse::<KeyDir>().unwrap();
            return Ok(GameCmd::Alt(dir));
        } else if cmd == "exit" {
            return Ok(GameCmd::Exit);
        }

        panic!(format!("GameCmd {} not expected!", cmd));
    }
}

pub fn execute_game_command(command: &GameCmd, game: &mut Game) -> String {
    match command {
        GameCmd::PlayerId => {
            let player_id = game.data.find_by_name(EntityName::Player).unwrap();
            return player_id.to_string();
        }

        GameCmd::Pos(id) => {
            let pos = game.data.entities.pos[id];
            return format!("{} {}", pos.x, pos.y);
        }

        GameCmd::SetPos(id, x, y) => {
            game.data.entities.pos[id] = Pos::new(*x, *y);
            return "".to_string();
        }

        GameCmd::Exit => {
            game.settings.running = false;
            return "".to_string();
        }

        GameCmd::Make(entity_name, x, y) => {
            let pos = Pos::new(*x, *y);
            let id = make_entity(&mut game.data.entities,
                                 &game.config,
                                 *entity_name,
                                 pos,
                                 &mut game.msg_log);
            return format!("{}", id);
        }
            
        GameCmd::Remove(id) => {
            game.data.remove_entity(*id);
            return "".to_string();
        }

        GameCmd::Key(chr, dir) => {
            // TODO this isn't really correct...
            let time = Instant::now();
            let dt = Instant::now().duration_since(time).as_secs_f32();

            let input_event = InputEvent::Char(*chr, *dir);
            let input_action = game.input.handle_event(&mut game.settings, input_event, time, &game.config);
            game.step_game(input_action, dt);
            return "".to_string();
        }

        GameCmd::Ctrl(dir) => {
            let time = Instant::now();
            let dt = Instant::now().duration_since(time).as_secs_f32();

            let input_event = InputEvent::Ctrl(*dir);
            let input_action = game.input.handle_event(&mut game.settings, input_event, time, &game.config);
            game.step_game(input_action, dt);
            return "".to_string();
        }

        GameCmd::Alt(dir) => {
            let time = Instant::now();
            let dt = Instant::now().duration_since(time).as_secs_f32();

            let input_event = InputEvent::Alt(*dir);
            let input_action = game.input.handle_event(&mut game.settings, input_event, time, &game.config);
            game.step_game(input_action, dt);
            return "".to_string();
        }

        // let action = game.input.handle_event(&mut game.settings, event, frame_time, &game.config);
        // game.step_game(input_action, dt);
    }
}

