use std::default::Default;

use oorandom::Rand32;

use serde::{Serialize, Deserialize};
use logging_timer::timer;

use roguelike_core::types::*;
use roguelike_core::config::*;
use roguelike_core::map::*;
use roguelike_core::messaging::MsgLog;
use roguelike_core::movement::{Direction, MoveMode};

use crate::actions;
use crate::actions::InputAction;
use crate::generation::*;
use crate::make_map::make_map;
use crate::step::step_logic;
use crate::input::*;
use crate::vault::*;


#[derive(Clone, Debug)]
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
    pub fn new(seed: u64, config: Config) -> Game {
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

        return state;
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

    pub fn step_game(&mut self, input_action: InputAction, dt: f32) -> bool {
        self.settings.dt = dt;
        self.settings.time += dt;

        let input_handled = actions::handle_input_universal(input_action, self);

        if !input_handled {
            let _input = timer!("INPUT");
            actions::handle_input(input_action,
                                  &self.data,
                                  &mut self.settings,
                                  &mut self.msg_log,
                                  &self.config);
        }

        if self.msg_log.messages.len() > 0 {
            let _step = timer!("STEP");
            let finished_level = step_logic(self);

            if finished_level {
                // NOTE this is not a very general way to handle ending a level.
                let player_id = self.data.find_by_name(EntityName::Player).unwrap();
                let key_id = self.data.is_in_inventory(player_id, Item::Key).expect("Won level without key!");
                self.data.entities.remove_item(player_id, key_id);

                self.settings.state = GameState::Playing;

                self.settings.level_num += 1;

                make_map(&self.config.map_load.clone(), self);
            }

            /* Check for explored tiles */
            let player_id = self.data.find_by_name(EntityName::Player).unwrap();

            let _explore = timer!("EXPLORE");
            for pos in self.data.map.get_all_pos() {
                // careful not to set map if not needed- this will clear the fov cache
                if !self.data.map[pos].explored {
                    let visible =
                        self.data.pos_in_fov(player_id, pos, &self.config) ||
                        self.settings.god_mode;
                    if visible {
                        self.data.map[pos].explored = true;
                    }
                }
            }
        }

        return self.settings.state != GameState::Exit;
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct GameSettings {
    pub turn_count: usize,
    pub god_mode: bool,
    pub map_type: MapGenType,
    pub state: GameState,
    pub overlay: bool,
    pub time: f32,
    pub dt: f32,
    pub render_map: bool,
    pub level_num: usize,
    pub running: bool,
    pub cursor: Option<Pos>,
    pub use_item_class: ItemClass,
    pub use_dir: Option<Direction>,
    pub move_mode: MoveMode,
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
            level_num: 0,
            running: true,
            cursor: None,
            use_item_class: ItemClass::Misc,
            use_dir: None,
            move_mode: MoveMode::Walk,
        };
    }
}

