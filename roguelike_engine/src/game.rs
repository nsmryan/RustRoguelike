use rand::prelude::*;

use serde::{Serialize, Deserialize};

use roguelike_core::types::*;
use roguelike_core::config::*;
use roguelike_core::map::*;
use roguelike_core::messaging::{Msg, MsgLog};
use roguelike_core::movement::Action;
#[cfg(test)]
use roguelike_core::movement::*;


use crate::actions;
use crate::actions::InputAction; //, KeyDirection};
use crate::generation::*;
use crate::make_map::{make_map, Vault, parse_vault};
use crate::selection::*;
use crate::step::step_logic;
#[cfg(test)]
use crate::make_map::*;


pub struct Game {
    pub config: Config,
    pub input_action: InputAction,
    pub data: GameData,
    pub settings: GameSettings,
    pub msg_log: MsgLog,
    pub rng: SmallRng,
    pub vaults: Vec<Vault>,
}

impl Game {
    pub fn new(seed: u64, config: Config) -> Result<Game, String> {
        let entities = Entities::new();
        let rng: SmallRng = SeedableRng::seed_from_u64(seed);

        let mut msg_log = MsgLog::new();

        let map = Map::empty();

        let mut data = GameData::new(map, entities);

        let player_id = make_player(&mut data.entities, &config, &mut msg_log);
        data.entities.pos[&player_id] = Pos::new(-1, -1);

        let vaults = Vec::new();

        let state = Game {
            config,
            input_action: InputAction::None,
            data,
            settings: GameSettings::new(0, false),
            msg_log,
            rng: rng,
            vaults,
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

    pub fn step_game(&mut self, dt: f32) -> GameResult {
        self.settings.time += dt;

        let result;
        match self.settings.state {
            GameState::Playing => {
                result = self.step_playing();
            }

            GameState::Win => {
                result = self.step_win();
            }

            GameState::Lose => {
                result = self.step_lose();
            }

            GameState::Inventory => {
                result = self.step_inventory();
            }

            GameState::Selection => {
                result = self.step_selection();
            }

            GameState::SkillMenu => {
                result = self.step_skill_menu();
            }

            GameState::ClassMenu => {
                result = self.step_class_menu();
            }

            GameState::ConfirmQuit => {
                result = self.step_confirm_quit();
            }
        }

        while let Some(msg) = self.msg_log.pop() {
            let msg_line = msg.msg_line(&self.data);
            if msg_line.len() > 0 {
                println!("msg: {}", msg_line);
            }
        }

        return result;
    }

    fn step_win(&mut self) -> GameResult {
        if matches!(self.input_action, InputAction::Exit) {
            return GameResult::Stop;
        }

        self.msg_log.log(Msg::ChangeLevel());

        let player_id = self.data.find_by_name(EntityName::Player).unwrap();
        let key_id = self.data.is_in_inventory(player_id, Item::Goal).expect("Won level without goal!");
        self.data.entities.remove_item(player_id, key_id);

        self.settings.state = GameState::Playing;

        self.settings.level_num += 1;

        make_map(&self.config.map_load.clone(), self);

        return GameResult::Continue;
    }

    fn step_lose(&mut self) -> GameResult {
        if self.input_action == InputAction::Exit {
            return GameResult::Stop;
        }

        return GameResult::Continue;
    }

    fn step_inventory(&mut self) -> GameResult {
        let input = self.input_action;
        self.input_action = InputAction::None;

        actions::handle_input_inventory(input, &mut self.data, &mut self.settings, &mut self.msg_log);

        if self.settings.exiting {
            return GameResult::Stop;
        }

        return GameResult::Continue;
    }

    fn step_skill_menu(&mut self) -> GameResult {
        let input = self.input_action;
        self.input_action = InputAction::None;

        let player_action =
            actions::handle_input_skill_menu(input, &mut self.data, &mut self.settings, &mut self.msg_log);

        if player_action != Action::NoAction {
            let win = step_logic(self, player_action);

            if win {
                self.settings.state = GameState::Win;
            }
        }

        if self.settings.exiting {
            return GameResult::Stop;
        }

        return GameResult::Continue;
    }

    fn step_class_menu(&mut self) -> GameResult {
        let input = self.input_action;
        self.input_action = InputAction::None;

        let player_action =
            actions::handle_input_class_menu(input, &mut self.data, &mut self.settings, &mut self.msg_log);

        if player_action != Action::NoAction {
            let win = step_logic(self, player_action);

            if win {
                self.settings.state = GameState::Win;
            }
        }

        if self.settings.exiting {
            return GameResult::Stop;
        }

        return GameResult::Continue;
    }

    fn step_confirm_quit(&mut self) -> GameResult {
        let input = self.input_action;
        self.input_action = InputAction::None;

        actions::handle_input_confirm_quit(input, &mut self.data, &mut self.settings, &mut self.msg_log);

        if self.settings.exiting {
            return GameResult::Stop;
        }

        return GameResult::Continue;
    }

    fn step_selection(&mut self) -> GameResult {
        let input = self.input_action;
        self.input_action = InputAction::None;

        self.settings.draw_selection_overlay = true;

        let player_action =
            actions::handle_input_selection(input,
                                           &mut self.data,
                                           &mut self.settings,
                                           &self.config,
                                           &mut self.msg_log);

        if player_action != Action::NoAction {
            let win = step_logic(self, player_action);
            if win {
                self.settings.state = GameState::Win;
            }
        }

        if self.settings.exiting {
            return GameResult::Stop;
        }

        return GameResult::Continue;
    }

//    fn step_console(&mut self) -> GameResult {
//        let input = self.input_action;
//        self.input_action = InputAction::None;
//
//        let time_since_open = self.settings.time - self.console.time_at_open;
//        let lerp_amount = clampf(time_since_open / self.config.console_speed, 0.0, 1.0);
//        self.console.height = lerp(self.console.height as f32,
//                                   self.config.console_max_height as f32,
//                                   lerp_amount) as u32;
//        if (self.console.height as i32 - self.config.console_max_height as i32).abs() < 2 {
//            self.console.height = self.config.console_max_height;
//        }
//
//        if self.key_input.len() > 0 {
//            // TODO add console back in
//            //actions::handle_input_console(input,
//            //                              &mut self.key_input,
//            //                              &mut self.console,
//            //                              &mut self.data,
//            //                              &mut self.display,
//            //                              &mut self.settings,
//            //                              &self.config,
//            //                              &mut self.msg_log);
//        }
//
//        return GameResult::Continue;
//    }

    fn step_playing(&mut self) -> GameResult {
        let player_action = actions::handle_input(self);

        if player_action != Action::NoAction {
            let win = step_logic(self, player_action);
            if win {
                self.settings.state = GameState::Win;
            }
        }

        if self.settings.exiting {
            return GameResult::Stop;
        }

        self.input_action = InputAction::None;

        return GameResult::Continue;
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum GameResult {
    Continue,
    Stop,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct GameSettings {
    pub turn_count: usize,
    pub god_mode: bool,
    pub map_type: MapGenType,
    pub exiting: bool,
    pub state: GameState,
    pub draw_selection_overlay: bool,
    pub overlay: bool,
    pub console: bool,
    pub time: f32,
    pub render_map: bool,
    pub selection: Selection,
    pub inventory_action: InventoryAction,
    pub level_num: usize,
    pub running: bool,
}

impl GameSettings {
    pub fn new(turn_count: usize,
               god_mode: bool) -> GameSettings {
        return GameSettings {
            turn_count,
            god_mode,
            map_type: MapGenType::Island,
            exiting: false,
            state: GameState::Playing,
            draw_selection_overlay: false,
            overlay: false,
            console: false,
            time: 0.0,
            render_map: true,
            selection: Selection::default(),
            inventory_action: InventoryAction::default(),
            level_num: 0,
            running: true,
        };
    }
}

