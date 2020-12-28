use std::default::Default;

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
use crate::actions::InputAction;
use crate::generation::*;
use crate::make_map::{make_map, Vault, parse_vault};
use crate::selection::*;
use crate::step::step_logic;
#[cfg(test)]
use crate::make_map::*;


pub struct Game {
    pub config: Config,
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

    // TODO be sure to handle the input action exit, win, and lose
    // TODO returns whether to continue the game
    pub fn step_game(&mut self, input_action: InputAction, dt: f32) -> bool {
        self.settings.time += dt;

        // TODO these are universal actions- they can be executed regardless of mode
        //InputAction::ExploreAll
        //    for x in 0..game.data.map.width() {
        //        for y in 0..game.data.map.height() {
        //            let pos = Pos::new(x, y);
        //            game.data.map[pos].explored = true;
        //        }
        //    }
        //}
        //InputAction::RegenerateMap
        //    let _position = make_map::make_map(&game.config.map_load.clone(), game);
        //}
        //InputAction::GodMode
        //    // TODO hmmm... add a message, or resolve at higher level as a universal action
        //    let god_mode_hp = 10000;
        //    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
        //    game.data.entities.fighter[&player_id].hp = god_mode_hp;
        //    game.data.entities.fighter[&player_id].max_hp = god_mode_hp;
        //    game.data.entities.energy[&player_id] = 1000;

        //    // toggle god mode flag
        //    game.settings.god_mode = !game.settings.god_mode;
        //}

        // TODO console
        //InputAction::ToggleConsole
            //if game.settings.state == GameState::Console {
            //    game.settings.state = GameState::Playing;
            //} else {
            //    //game.console.time_at_open = game.settings.time;
            //    //game.console.height = 0;
            //    //game.settings.state = GameState::Console;
            //}
        //}

        let action_result;
        match self.settings.state {
            GameState::Playing => {
                action_result = actions::handle_input_playing(self, input_action);
            }

            GameState::Win => {
                action_result = Default::default();
            }

            GameState::Lose => {
                action_result = Default::default();
            }

            GameState::Inventory => {
                action_result = 
                    actions::handle_input_inventory(input,
                                                    &self.data,
                                                    &mut self.settings,
                                                    &mut self.msg_log);
            }

            GameState::Selection => {
                self.settings.draw_selection_overlay = true;

                let player_turn =
                    actions::handle_input_selection(input,
                                                   &self.data,
                                                   &mut self.settings,
                                                   &self.config,
                                                   &mut self.msg_log);
                action_result = ActionResult::new(player_turn, None);
            }

            GameState::SkillMenu => {
                let player_turn = 
                    actions::handle_input_skill_menu(input,
                                                     &self.data,
                                                     &mut self.settings,
                                                     &mut self.msg_log,
                                                     &self.config);
                action_result = ActionResult::new(player_turn, None);
            }

            GameState::ClassMenu => {
                let player_turn =
                    actions::handle_input_class_menu(input,
                                                     &self.data,
                                                     &mut self.settings,
                                                     &mut self.msg_log);
                action_result = ActionResult::new(player_turn, None);
            }

            GameState::ConfirmQuit => {
                let player_turn = actions::handle_input_confirm_quit(input);
                action_result = ActionResult::new(player_turn, None);
            }

            GameState::Exit => {
                action_result = Default::default();
            }
        }

        if let Some(state) = action_result.new_state {
            settings.state = state;
            msg_log.log(Msg::GameState(settings.state));
        }

        // TODO consider moving this out? is state changing separate from carrying out the game
        // logic?
        if action_result.turn == Action::NoAction {
            resolve_messages(&mut game.data,
                             &mut game.msg_log,
                             &mut game.settings,
                             &mut game.rng,
                             &game.config);
        } else {
            let finsished_level = step_logic(self, player_turn);
            if finsished_level {
                // TODO can we check for the last level, and either win or move on?
                //self.settings.state = GameState::Win;

                //let player_id = data.find_by_name(EntityName::Player).unwrap();
                //let key_id = data.is_in_inventory(player_id, Item::Goal).expect("Won level without goal!");
                //data.entities.remove_item(player_id, key_id);

                //settings.state = GameState::Playing;

                //settings.level_num += 1;

                //make_map(&config.map_load.clone(), game);
            }
        }

        //if self.settings.exiting {
        //    return GameResult::Stop;
        //}

        return game.settings.state != GameState::Exit;

        // TODO this shouldn't be necessary anymore
        //while let Some(msg) = self.msg_log.pop() {
        //    let msg_line = msg.msg_line(&self.data);
        //    if msg_line.len() > 0 {
        //        println!("msg: {}", msg_line);
        //    }
        //}
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
    pub cursor_pos,
}

impl GameSettings {
    pub fn new(turn_count: usize, god_mode: bool) -> GameSettings {
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
            cursor_pos: Pos::new(0, 0),
        };
    }
}

