use std::default::Default;
use std::fmt;
use std::io::{stdout, Write};

use oorandom::Rand32;

use serde::{Serialize, Deserialize};

use roguelike_core::types::*;
use roguelike_core::config::*;
use roguelike_core::map::*;
use roguelike_core::messaging::{Msg, MsgLog};
use roguelike_core::movement::*;

use crate::actions;
use crate::actions::{InputAction, ActionResult};
use crate::generation::*;
use crate::make_map::make_map;
use crate::step::step_logic;
use crate::input::*;
use crate::vault::*;


pub struct Game {
    pub config: Config,
    pub data: GameData,
    pub settings: GameSettings,
    pub msg_log: MsgLog,
    pub rng: Rand32,
    pub vaults: Vec<Vault>,
    pub input: Input,
}

impl Game {
    pub fn new(seed: u64, config: Config) -> Result<Game, String> {
        let entities = Entities::new();
        let rng: Rand32 = Rand32::new(seed);

        let mut msg_log = MsgLog::new();

        let map = Map::empty();

        let mut data = GameData::new(map, entities);

        let player_id = make_player(&mut data.entities, &config, &mut msg_log);
        data.entities.pos[&player_id] = Pos::new(-1, -1);

        let vaults: Vec<Vault> = Vec::new();

        let state = Game {
            config,
            data,
            settings: GameSettings::new(0, false),
            msg_log,
            rng: rng,
            vaults,
            input: Input::new(),
        };

        return Ok(state);
    }

    pub fn load_vaults(&mut self, path: &str) {
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let vault_file_name = path.to_str().unwrap();
            if !vault_file_name.ends_with(".csv") {
                continue;
            }
            self.vaults.push(parse_vault(vault_file_name, &self.config));
        }
    }

    pub fn log_output(&mut self, log_message: &str) {
        self.log(LogMsgType::Output, log_message);
    }

    pub fn log_console(&mut self, log_message: &str) {
        self.log(LogMsgType::Console, log_message);
    }

    pub fn log_msg(&mut self, log_message: &str) {
        self.log(LogMsgType::Msg, log_message);
    }

    pub fn log_key(&mut self, log_message: &str) {
        self.log(LogMsgType::Key, log_message);
    }

    pub fn log(&mut self, typ: LogMsgType, log_message: &str) {
        println!("{}: {}", typ, log_message);
        stdout().flush().unwrap();
    }
        
    pub fn step_game(&mut self, input_action: InputAction, dt: f32) -> bool {
        self.settings.dt = dt;
        self.settings.time += dt;

        actions::handle_input_universal(input_action, self);

        let action_result: ActionResult =
            actions::handle_input(input_action,
                                  &self.data,
                                  &mut self.settings,
                                  &mut self.msg_log,
                                  &self.config);

        if let Some(state) = action_result.new_state {
            self.settings.state = state;
            self.msg_log.log(Msg::GameState(self.settings.state));
        }

        if self.msg_log.messages.len() > 0 || action_result.turn != Action::NoAction {
            let finsished_level = step_logic(self, action_result.turn);
            if finsished_level {
                let player_id = self.data.find_by_name(EntityName::Player).unwrap();
                let key_id = self.data.is_in_inventory(player_id, Item::Key).expect("Won level without key!");
                self.data.entities.remove_item(player_id, key_id);

                self.settings.state = GameState::Playing;

                self.settings.level_num += 1;

                make_map(&self.config.map_load.clone(), self);
            }

            for msg_index in 0..self.msg_log.turn_messages.len() {
                let msg = self.msg_log.turn_messages[msg_index];
                let msg_line = &msg.msg_line(&self.data);
                if msg_line.len() > 0 {
                    self.log_console(msg_line);
                }
                self.log_msg(&format!("{}", msg));
            }
        }

        /* Check for explored tiles */
        let player_id = self.data.find_by_name(EntityName::Player).unwrap();
        for pos in self.data.map.get_all_pos() {
            let visible =
                self.data.is_in_fov(player_id, pos, &self.config) ||
                self.settings.god_mode;

            // careful not to set map if not needed- this will clear the fov cache
            if visible && !self.data.map[pos].explored {
                self.data.map[pos].explored = visible;
            }
        }

        return self.settings.state != GameState::Exit;
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct GameSettings {
    pub turn_count: usize,
    pub god_mode: bool,
    pub map_type: MapGenType,
    pub state: GameState,
    pub overlay: bool,
    pub time: f32,
    pub dt: f32,
    pub render_map: bool,
    pub inventory_action: InventoryAction,
    pub level_num: usize,
    pub running: bool,
    pub cursor: Option<Pos>,
}

impl GameSettings {
    pub fn new(turn_count: usize, god_mode: bool) -> GameSettings {
        return GameSettings {
            turn_count,
            god_mode,
            map_type: MapGenType::Island,
            state: GameState::Playing,
            overlay: false,
            time: 0.0,
            dt: 0.0,
            render_map: true,
            inventory_action: InventoryAction::default(),
            level_num: 0,
            running: true,
            cursor: None,
        };
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LogMsgType {
    Output,
    Console,
    Msg,
    Key,
}

impl fmt::Display for LogMsgType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogMsgType::Output => write!(f, "OUTPUT"),
            LogMsgType::Console => write!(f, "CONSOLE"),
            LogMsgType::Msg => write!(f, "MSG"),
            LogMsgType::Key => write!(f, "KEY"),
        }
    }
}

#[test]
pub fn test_action_log() {
    let file =
        std::fs::File::open(&"../resources/test_actions/actions.txt")
                      .expect(&format!("Could not open replay file '{}'", &replay_file));
    let mut actions = Vec::new();
    for line in std::io::BufReader::new(file).lines() {
        if let Ok(action) = InputAction::from_str(&line.unwrap()) {
            actions.push(action);
        }
    }

    let mut logged_messages: Vec<Msg> = Vec::new();
    for action in actions {
        game.step_game(action, 0.1);
        logged_messages.extend(game.msg_log.turn_messages);
    }

    dbg!(&logged_messages);
}

