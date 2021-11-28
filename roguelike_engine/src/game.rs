use serde::{Serialize, Deserialize};
use logging_timer::timer;

use roguelike_core::types::*;
use roguelike_core::config::*;
use roguelike_core::map::*;
use roguelike_core::messaging::MsgLog;
use roguelike_core::movement::{Direction, MoveMode};
use roguelike_core::rng::Rand32;
use roguelike_core::messaging::*;

use crate::actions;
use crate::actions::InputAction;
use crate::generation::*;
use crate::make_map::make_map;
use crate::step::step_logic;
use crate::input::*;
use crate::vault::*;


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub config: Config,
    pub data: Level,
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

        let mut data = Level::new(map, entities);

        let player_id = make_player(&mut data.entities, &config, &mut msg_log);
        data.entities.pos[&player_id] = Pos::new(-1, -1);

        let vaults: Vec<Vault> = Vec::new();

        let state = Game {
            config,
            data,
            settings: GameSettings::new(),
            msg_log,
            rng: rng,
            vaults,
            input: Input::new(),
        };

        return state;
    }

    pub fn clear_level_except_player(&mut self) {
        let mut dont_clear: Vec<EntityId> = Vec::new();

        let player_id = self.data.find_by_name(EntityName::Player).unwrap();
        dont_clear.push(player_id);
        for item_id in self.data.entities.inventory[&player_id].iter() {
            dont_clear.push(*item_id);
        }

        for id in self.data.entities.ids.clone().iter() {
            if !dont_clear.contains(id) {
                self.msg_log.log(Msg::RemovedEntity(*id));
                self.data.entities.remove_entity(*id);
            }
        }
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

    pub fn step_game(&mut self, input_action: InputAction) -> bool {
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

            self.emit_state_messages(MsgLogDir::Back);
        }

        return self.settings.state != GameState::Exit;
    }

    pub fn emit_state_messages(&mut self, log_dir: MsgLogDir) {
        let player_id = self.data.find_by_name(EntityName::Player).unwrap();
        let player_pos = self.data.entities.pos[&player_id];
        let (map_width, map_height) = self.data.map.size();

        let mut player_fov = Vec::new();

        if self.data.entities.took_turn[&player_id] && log_dir == MsgLogDir::Back {
            self.msg_log.log(Msg::StartTurn);
        }

        // indicate FoV information
        for y in 0..map_height {
            for x in 0..map_width {
                let pos = Pos::new(x, y);
                let fov_result;
                if self.settings.god_mode {
                    fov_result = FovResult::Inside;
                } else {
                    fov_result = self.data.pos_in_fov_edge(player_id, pos, &self.config);
                }

                // only send if inside or on edge- outside is most common, so it is assumed
                // if no message is sent.
                if fov_result != FovResult::Outside {
                    self.msg_log.log_dir(Msg::TileFov(pos, fov_result), log_dir);
                }

                // TODO should this be != Outside, to include Edge?
                if fov_result == FovResult::Inside {
                    player_fov.push(pos);
                }
            }
        }

        for entity_id in self.data.entities.ids.iter() {
            let typ = self.data.entities.typ[&entity_id];
            let entity_pos = self.data.entities.pos[&entity_id];
            if !self.data.map.is_within_bounds(entity_pos) {
                continue;
            }

            // emit whether entity is in player FOV
            let mut in_fov = self.data.is_in_fov(player_id, *entity_id, &self.config);
            if self.settings.god_mode {
                in_fov = FovResult::Inside;
            }

            // outside is the most common fov result, so it is assumed if no entry is sent.
            if in_fov != FovResult::Outside {
                self.msg_log.log_dir(Msg::EntityInFov(*entity_id, in_fov), log_dir);
            }

            // There is no need for movement or attack information for these types of entities.
            if typ == EntityType::Environment ||
               typ == EntityType::Trigger ||
               typ == EntityType::Column ||
               typ == EntityType::Energy ||
               typ == EntityType::Item {
                continue;
            }

            // emit visible movement positions
            if in_fov == FovResult::Inside {
                if let Some(reach) = self.data.entities.movement.get(&entity_id) {
                    for move_pos in reach.reachables(entity_pos) {
                        if !self.data.map.is_within_bounds(move_pos) {
                            continue;
                        }

                        if self.data.pos_in_fov(player_id, move_pos, &self.config) {
                            self.msg_log.log_dir(Msg::EntityMovement(*entity_id, move_pos), log_dir);
                        }
                    }
                }
            }

            if typ != EntityType::Player && typ != EntityType::Enemy {
                continue;
            }

            // emit visible attack positions
            if in_fov == FovResult::Inside {
                if let Some(reach) = self.data.entities.attack.get(&entity_id) {
                    for attack_pos in reach.reachables(entity_pos) {
                        if !self.data.map.is_within_bounds(attack_pos) {
                            continue;
                        }

                        if self.data.pos_in_fov(*entity_id, attack_pos, &self.config) &&
                           (self.data.clear_path(entity_pos, attack_pos, false) || attack_pos == player_pos) {
                            self.msg_log.log_dir(Msg::EntityAttack(*entity_id, attack_pos), log_dir);
                        }
                    }
                }
            }

            // emit visible tiles for entity that are visible to player
            if in_fov == FovResult::Inside && *entity_id != player_id {
                for pos in player_fov.iter() {
                    if self.data.pos_in_fov(*entity_id, *pos, &self.config) {
                        self.msg_log.log_dir(Msg::EntityFov(*entity_id, *pos), log_dir);
                    }
                }
            }
        }

        // if in use-mode, output use-direction.
        if let UseAction::Item(item_class) = self.settings.use_action {
            if let Some(item_index) = self.data.find_item(item_class) {
                if let Some(use_dir) = self.settings.use_dir {
                    let use_result = self.data.calculate_use_move(player_id,
                                                                  item_index,
                                                                  use_dir,
                                                                  self.settings.move_mode);
                    if let Some(pos) = use_result.pos {
                        self.msg_log.log_dir(Msg::UsePos(pos), log_dir);
                    }

                    if let Some(dir) = self.settings.use_dir {
                        self.msg_log.log_dir(Msg::UseDir(dir), log_dir);
                    }

                    for pos in use_result.hit_positions {
                        self.msg_log.log_dir(Msg::UseHitPos(pos), log_dir);
                    }
                }
            }
        }

        // report entities at the cursor position
        if let Some(cursor_pos) = self.settings.cursor {
            let entities = self.data.get_entities_at_pos(cursor_pos);
            for entity in entities {
                self.msg_log.log_dir(Msg::EntityAtCursor(entity), log_dir);
            }
        }

        if self.data.entities.took_turn[&player_id] && log_dir == MsgLogDir::Front {
            self.msg_log.log_front(Msg::StartTurn);
        }
    }

    pub fn save_as_string(&self) -> String {
        return serde_yaml::to_string(self).unwrap().to_string();
    }

    pub fn load_from_string(game_str: &str) -> Game {
        return serde_yaml::from_str(game_str).unwrap();
    }
}
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum UseAction {
    Item(ItemClass),
    Interact,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct GameSettings {
    pub turn_count: usize,
    pub god_mode: bool,
    pub map_type: MapGenType,
    pub state: GameState,
    pub overlay: bool,
    pub render_map: bool,
    pub level_num: usize,
    pub running: bool,
    pub cursor: Option<Pos>,
    pub use_action: UseAction,
    pub use_dir: Option<Direction>,
    pub move_mode: MoveMode,
    pub debug_enabled: bool,
    pub map_load_config: MapLoadConfig,
}

impl GameSettings {
    pub fn new() -> GameSettings {
        return GameSettings {
            turn_count: 0,
            god_mode: false,
            map_type: MapGenType::Island,
            state: GameState::Playing,
            overlay: false,
            render_map: true,
            level_num: 0,
            running: true,
            cursor: None,
            use_action: UseAction::Interact,
            use_dir: None,
            move_mode: MoveMode::Walk,
            debug_enabled: false,
            map_load_config: MapLoadConfig::Empty,
        };
    }
}

