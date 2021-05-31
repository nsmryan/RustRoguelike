use std::fs;
use std::time::Duration;
use std::io::BufRead;
use std::str::FromStr;
use std::cmp;

use roguelike_core::map::MapLoadConfig;
#[cfg(test)]
use roguelike_core::config::*;
#[cfg(test)]
use roguelike_core::movement::*;
#[cfg(test)]
use roguelike_core::types::*;

use roguelike_engine::game::*;
use roguelike_engine::actions::*;
use roguelike_engine::make_map::make_map;
use roguelike_engine::log::*;

use crate::display::*;
use crate::render::*;


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ReplayResult {
    Same,
    NewSubsetOld,
    OldSubsetNew,
    Different,
}

#[derive(Clone, Debug)]
pub struct Recording {
    states: Vec<Game>,
    inputs: Vec<InputAction>,
    cursor: usize,
}

impl Recording {
    pub fn new(game: &Game) -> Recording {
        let mut states = Vec::new();
        states.push(game.clone());
        return Recording {
            states: states,
            inputs: Vec::new(),
            cursor: 0,
        };
    }

    pub fn forward(self: &mut Recording) -> Option<Game> {
        if let Some(action) = self.inputs.get(self.cursor) {
            let mut game = self.states[self.states.len() - 1].clone();
            game.step_game(*action, 0.1);
            let return_game = game.clone();
            self.states.push(game);
            self.cursor = std::cmp::min(self.cursor + 1, self.inputs.len());
            return Some(return_game);
        } else {
            return None;
        }
    }
    
    pub fn backward(self: &mut Recording) -> Option<Game> {
        if self.cursor > 0 {
            self.cursor = self.cursor - 1;
            self.states.pop();
        }

        return Some(self.states[self.states.len() - 1].clone());
    }

    pub fn action(self: &mut Recording, game: &Game, action: InputAction) {
        self.states.push(game.clone());
        self.inputs.insert(self.cursor, action);
        self.cursor += 1;
    }
}

#[test]
pub fn test_recording() {
    let mut config = Config::from_file("../config.yaml");
    config.map_load = MapLoadConfig::TestPlayer;
    let mut game = Game::new(0, config.clone()).unwrap();
    let mut input_action;

    make_map(&config.map_load, &mut game);

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    let starting_pos = game.data.entities.pos[&player_id];

    
    // starting at (0,0)
    let mut recording = Recording::new(&game);

    // walk right (1, 0)
    input_action = InputAction::Move(Direction::Right, MoveMode::Walk);
    game.step_game(input_action, 0.1);
    recording.action(&game, input_action);
    let step1_pos = game.data.entities.pos[&player_id];
    assert_eq!(starting_pos.x + 1, step1_pos.x);
    assert_eq!(starting_pos.y, step1_pos.y);

    // walk down (1, 1)
    input_action = InputAction::Move(Direction::Down, MoveMode::Walk);
    game.step_game(input_action, 0.1);
    recording.action(&game, input_action);
    let step2_pos = game.data.entities.pos[&player_id];
    assert_eq!(starting_pos.x + 1, step2_pos.x);
    assert_eq!(starting_pos.y + 1, step2_pos.y);

    // undo the walk down (1, 0)
    let game = recording.backward().unwrap();
    let step1_pos = game.data.entities.pos[&player_id];
    assert_eq!(starting_pos.x + 1, step1_pos.x);
    assert_eq!(starting_pos.y, step1_pos.y);

    // undo the walk right (0, 0)
    let mut game = recording.backward().unwrap();
    let step0_pos = game.data.entities.pos[&player_id];
    assert_eq!(starting_pos.x, step0_pos.x);
    assert_eq!(starting_pos.y, step0_pos.y);

    // go down first, and then replay the previous actions (0, 1)
    input_action = InputAction::Move(Direction::Down, MoveMode::Walk);
    game.step_game(input_action, 0.1);
    recording.action(&game, input_action);
    let step1_2_pos = game.data.entities.pos[&player_id];
    assert_eq!(starting_pos.x, step1_2_pos.x);
    assert_eq!(starting_pos.y + 1, step1_2_pos.y);

    // replay walk right (1, 1)
    let game = recording.forward().unwrap();
    let step2_2_pos = game.data.entities.pos[&player_id];
    assert_eq!(starting_pos.x + 1, step2_2_pos.x);
    assert_eq!(starting_pos.y + 1, step2_2_pos.y);

    // replay walk down (1, 2)
    let game = recording.forward().unwrap();
    let step3_2_pos = game.data.entities.pos[&player_id];
    assert_eq!(starting_pos.x + 1, step3_2_pos.x);
    assert_eq!(starting_pos.y + 2, step3_2_pos.y);
}


pub const MAP_CONFIG_NAME: &str = "map_config.txt";

pub fn check_all_records(game: &mut Game, display: &mut Display, event_pump: &mut sdl2::EventPump, delay_ms: u64) -> Result<(), String> {
    let mut results: Vec<(String, ReplayResult)> = Vec::new();

    for entry in fs::read_dir("resources/test_logs/").unwrap() {
        let path = entry.unwrap().path();

        let record_path = path.as_path().to_str().unwrap();
        let record_name = record_path.rsplit("/").next().unwrap();

        let mut local_game = game.clone();
        let result = check_record(&mut local_game, display, event_pump, record_name, delay_ms);

        results.push((record_name.to_string(), result));
    }

    let num_cases = results.len();

    let mut count_differ = 0;
    for (record_name, result) in results {
        eprintln!("test '{}': {:?}", record_name, result);

        if result != ReplayResult::Same {
            count_differ += 1;
        }
    }

    if count_differ != 0 {
        eprintln!("{}/{} tests had differing logs!", count_differ, num_cases);
    } else {
        eprintln!("{} all same!", num_cases);
    }

    return Ok(());
}

pub fn check_single_record(game: &mut Game, display: &mut Display, event_pump: &mut sdl2::EventPump, record_name: &str, delay_ms: u64) -> Result<(), String> {
    check_record(game, display, event_pump, record_name, delay_ms);
    return Ok(());
}

fn check_record(game: &mut Game, display: &mut Display, event_pump: &mut sdl2::EventPump, record_name: &str, delay_ms: u64) -> ReplayResult {
    let path = format!("resources/test_logs/{}", record_name);

    let map_config_path = format!("{}/{}", path, MAP_CONFIG_NAME);
    let map_config_string = std::fs::read_to_string(map_config_path).unwrap();
    let map_config = map_config_string.parse::<MapLoadConfig>().expect("Could not parse map config");
    eprintln!("Using map config: {}", &map_config);
    make_map(&map_config, game);

    let action_path = format!("{}/{}", path, Log::ACTION_LOG_NAME);
    let actions = read_action_log(&action_path);

    let message_path = format!("{}/{}", path, Log::MESSAGE_LOG_NAME);
    let logged_lines = read_message_log(&message_path);

    let prefix = "MSG: ";
    let mut old_messages = logged_lines.iter()
                                       .filter(|line| line.starts_with(prefix))
                                       .map(|line| line[prefix.len()..].to_string())
                                       .collect::<Vec<String>>();
    old_messages.reverse();
    let old_messages = old_messages;

    let mut new_messages: Vec<String> = Vec::new();

    let delay = Duration::from_millis(delay_ms);
    for action in actions {
        if action == InputAction::Exit {
            break;
        }

        game.step_game(action, delay_ms as f32);

        for _sdl2_event in event_pump.poll_iter() {
        }

        update_display(game, display).unwrap();

        for msg in &game.msg_log.turn_messages {
            new_messages.push(msg.to_string());
        }
        game.msg_log.clear();
        std::thread::sleep(delay);
    }

    /* Compare Logs */ 
    eprintln!("");
    let mut logs_differ = false; 
    let mut first_diff_index = 0;
    if old_messages.len() != new_messages.len() {
        eprintln!("Old log had {} messages, new log has {} messages", old_messages.len(), new_messages.len());
        logs_differ = true;
    }

    let mut msg_index = 0;
    while msg_index < old_messages.len() {
        if msg_index >= new_messages.len() {
            eprintln!("Reached end of new messages");
            logs_differ = true;
            first_diff_index = msg_index;
            break;
        }

        if old_messages[msg_index] != new_messages[msg_index] {
            eprintln!("First difference on line {}", msg_index);
            eprintln!("Old log '{}'", old_messages[msg_index]);
            eprintln!("New log '{}'", new_messages[msg_index]);
            logs_differ = true;
            first_diff_index = msg_index;
            break;
        }
        msg_index += 1;
    }

    let mut log = Log::new();
    eprintln!("\nNew Log:");
    for msg in new_messages.iter() {
        log.log_msg(&format!("{}", msg));
    }

    let mut result: ReplayResult;
    if logs_differ {
        result = ReplayResult::Different;

        let start_diff = if first_diff_index > 5 { first_diff_index } else { 0 };
        let end_diff = cmp::min(cmp::min(first_diff_index + 5, old_messages.len()), new_messages.len());
        eprintln!("");
        eprintln!("The old log starts with:");
        for msg_index in start_diff..end_diff {
            eprintln!("{}", old_messages[msg_index]);
        }

        eprintln!("");

        eprintln!("The new log starts with:");
        for msg_index in start_diff..end_diff {
            eprintln!("{}", new_messages[msg_index]);
        }

        {
            let mut new_index = 0;
            let mut old_index = 0;
            while old_index < old_messages.len() && new_index < new_messages.len() {
                if old_messages[old_index] == new_messages[new_index] {
                    new_index += 1;
                }
                old_index += 1;
            }
            if new_index != new_messages.len() {
                eprintln!("New log is not a subset of the old log!");
            } else {
                eprintln!("New log is a subset of the old log!");
                result = ReplayResult::NewSubsetOld;
            }
        }

        {
            let mut new_index = 0;
            let mut old_index = 0;
            while old_index < old_messages.len() && new_index < new_messages.len() {
                if old_messages[old_index] == new_messages[new_index] {
                    old_index += 1;
                }
                new_index += 1;
            }
            if old_index != old_messages.len() {
                eprintln!("Old log is not a subset of the new log!");
            } else {
                eprintln!("Old log is a subset of the new log!");
                result = ReplayResult::OldSubsetNew;
            }
        }
    } else {
        eprintln!("Logs same!");
        result = ReplayResult::Same;
    }

    return result;
}

pub fn rerecord_all(game: &mut Game, display: &mut Display, event_pump: &mut sdl2::EventPump, delay_ms: u64) -> Result<(), String> {
    let mut results: Vec<String> = Vec::new();

    for entry in fs::read_dir("resources/test_logs/").unwrap() {
        let path = entry.unwrap().path();

        let record_path = path.as_path().to_str().unwrap();
        let record_name = record_path.rsplit("/").next().unwrap();

        let mut local_game = game.clone();
        rerecord_single(&mut local_game, display, event_pump, record_name, delay_ms)?;

        results.push(record_name.to_string());
    }

    for result in results {
        eprintln!("re-recorded '{}'", result);
    }

    return Ok(());
}

pub fn rerecord_single(game: &mut Game, display: &mut Display, event_pump: &mut sdl2::EventPump, record_name: &str, delay_ms: u64) -> Result<(), String> {
    return rerecord(game, display, event_pump, record_name, delay_ms);
}

fn rerecord(game: &mut Game, display: &mut Display, event_pump: &mut sdl2::EventPump, record_name: &str, delay_ms: u64) -> Result<(), String> {
    let path = format!("resources/test_logs/{}", record_name);

    let map_config_path = format!("{}/{}", path, MAP_CONFIG_NAME);
    let map_config_string = std::fs::read_to_string(map_config_path).unwrap();
    let map_config = map_config_string.parse::<MapLoadConfig>().expect("Could not parse map config");
    eprintln!("Using map config: {}", &map_config);
    make_map(&map_config, game);

    let action_path = format!("{}/{}", path, Log::ACTION_LOG_NAME);
    let actions = read_action_log(&action_path);

    let message_path = format!("{}/{}", path, Log::MESSAGE_LOG_NAME);

    let mut log = Log::new();

    let delay = Duration::from_millis(delay_ms);
    for action in actions {
        game.step_game(action, delay_ms as f32);

        for _sdl2_event in event_pump.poll_iter() { }

        update_display(game, display)?;

        for msg in &game.msg_log.turn_messages {
            log.log_msg(&format!("{}", msg));
        }
        game.msg_log.clear();
        std::thread::sleep(delay);
    }
    game.step_game(InputAction::Exit, delay_ms as f32);
    for msg in &game.msg_log.turn_messages {
        log.log_msg(&format!("{}", msg));
    }

    std::fs::copy(Log::MESSAGE_LOG_NAME, message_path)
            .expect("Could not save message log!");

    return Ok(());
}

pub fn read_action_log(replay_file: &str) -> Vec<InputAction> {
    let mut starting_actions = Vec::new();

    let file =
        std::fs::File::open(&replay_file).expect(&format!("Could not open replay file '{}'", &replay_file));
    for line in std::io::BufReader::new(file).lines() {
        if let Ok(action) = InputAction::from_str(&line.unwrap()) { 
            starting_actions.push(action);
        }
    }

    return starting_actions;
}

pub fn read_message_log(message_file: &str) -> Vec<String> {
    let mut message_lines = Vec::new();

    let file =
        std::fs::File::open(&message_file).expect(&format!("Could not open message file '{}'", &message_file));
    for line in std::io::BufReader::new(file).lines() {
        if let Ok(line) = line {
            message_lines.push(line);
        }
    }

    message_lines.reverse();
    return message_lines;
}

// NOTE duplicate code in main.rs
fn update_display(game: &mut Game, display: &mut Display) -> Result<(), String> {
    for msg in game.msg_log.turn_messages.iter() {
        display.process_message(*msg, &mut game.data, &game.config);
    }

    /* Draw the Game to the Screen */
    render_all(display, game)?;

    display.update_display();

    return Ok(());
}

