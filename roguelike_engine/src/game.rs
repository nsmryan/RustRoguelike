use serde::{Serialize, Deserialize};
use logging_timer::timer;

use roguelike_utils::rng::Rand32;
use roguelike_utils::comp::*;

use roguelike_map::*;

use roguelike_core::utils::*;
use roguelike_core::types::*;
use roguelike_core::config::*;
use roguelike_core::messaging::MsgLog;
use roguelike_core::movement::{MoveMode, Reach};
use roguelike_core::entities::*;
use roguelike_core::messaging::*;
use roguelike_core::level::*;

use crate::actions;
use crate::actions::InputAction;
use crate::generation::*;
use crate::map_construct::map_construct;
use crate::step::step_logic;
use crate::input::*;
use crate::vault::*;


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub config: Config,
    pub level: Level,
    pub settings: Settings,
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

        let mut level = Level::new(map, entities);

        let player_id = make_player(&mut level.entities, &config, &mut msg_log);
        level.entities.pos[&player_id] = Pos::new(-1, -1);

        let vaults: Vec<Vault> = Vec::new();

        let state = Game {
            config,
            level,
            settings: Settings::new(),
            msg_log,
            rng: rng,
            vaults,
            input: Input::new(),
        };

        return state;
    }

    pub fn clear_level_except_player(&mut self) {
        let mut dont_clear: Vec<EntityId> = Vec::new();

        let player_id = self.level.find_by_name(EntityName::Player).unwrap();
        dont_clear.push(player_id);
        for item_id in self.level.entities.inventory[&player_id].iter() {
            dont_clear.push(*item_id);
        }

        for id in self.level.entities.ids.clone().iter() {
            if !dont_clear.contains(id) {
                self.msg_log.log(Msg::RemovedEntity(*id));
                self.level.entities.remove_entity(*id);
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
        let player_id = self.level.find_by_name(EntityName::Player).unwrap();

        let input_handled = actions::handle_input_universal(input_action, self);

        if !input_handled {
            let _input = timer!("INPUT");
            actions::handle_input(input_action,
                                  &self.level,
                                  &mut self.settings,
                                  &mut self.msg_log,
                                  &self.config);
        }

        if self.msg_log.turn_messages.len() > 0 {
            let _step = timer!("STEP");
            let finished_level = step_logic(self);

            if finished_level {
                // NOTE this is not a very general way to handle ending a level.
                let player_id = self.level.find_by_name(EntityName::Player).unwrap();
                let key_id = self.level.is_in_inventory(player_id, Item::Key).expect("Won level without key!");
                self.level.entities.remove_item(player_id, key_id);

                self.settings.state = GameState::Playing;

                self.settings.level_num += 1;

                map_construct(&self.config.map_load.clone(), self);
            }

            /* Check for explored tiles */
            let player_id = self.level.find_by_name(EntityName::Player).unwrap();

            let _explore = timer!("EXPLORE");
            for pos in self.level.map.get_all_pos() {
                // careful not to set map if not needed- this will clear the fov cache
                if !self.level.map[pos].explored {
                    let visible =
                        self.level.pos_in_fov(player_id, pos) ||
                        self.settings.test_mode;
                    if visible {
                        self.level.map[pos].explored = true;
                    }
                }
            }

            if self.level.entities.took_turn[&player_id] {
                self.msg_log.log(Msg::StartTurn);
            }
        }

        if input_action != InputAction::None {
            self.emit_any_action_state();
        }

        if self.level.entities.took_turn[&player_id] || self.settings.map_changed {
            self.emit_took_turn_state();
        }

        self.settings.map_changed = false;

        // send PlayerTurn action in case there is cleanup to perform, or another system
        // needs to know that the turn is finished.
        if self.level.entities.took_turn[&player_id] {
            self.msg_log.log(Msg::PlayerTurn);
        }

        return self.settings.state != GameState::Exit;
    }

    pub fn emit_state_messages(&mut self) {
        self.msg_log.log(Msg::StartTurn);
        self.emit_took_turn_state();
        self.emit_any_action_state();
    }

    fn emit_player_fov(self: &mut Game) -> Vec<Pos> {
        let player_id = self.level.find_by_name(EntityName::Player).unwrap();

        let mut player_fov = Vec::new();

        let (map_width, map_height) = self.level.map.size();
        for y in 0..map_height {
            for x in 0..map_width {
                let pos = Pos::new(x, y);
                let fov_result;
                if self.settings.test_mode {
                    fov_result = FovResult::Inside;
                } else {
                    fov_result = self.level.pos_in_fov_edge(player_id, pos);
                }

                // only send if inside or on edge- outside is most common, so it is assumed
                // if no message is sent.
                if fov_result != FovResult::Outside {
                    self.msg_log.log(Msg::TileFov(pos, fov_result));
                }

                // TODO should this be != Outside, to include Edge?
                if fov_result == FovResult::Inside {
                    player_fov.push(pos);
                }
            }
        }

        return player_fov;
    }

    fn emit_entity_information(self: &mut Game, entity_id: EntityId, player_fov: &Vec<Pos>) {
        let player_id = self.level.find_by_name(EntityName::Player).unwrap();
        let player_pos = self.level.entities.pos[&player_id];

        // emit whether entity is in player FOV
        let mut in_fov = self.level.is_in_fov(player_id, entity_id);
        if self.settings.test_mode {
            in_fov = FovResult::Inside;
        }

        // outside is the most common fov result, so it is assumed if no entry is sent.
        if in_fov != FovResult::Outside {
            self.msg_log.log(Msg::EntityInFov(entity_id, in_fov));
        }

        // Only report movement and attack information for the player and golems.
        let typ = self.level.entities.typ[&entity_id];
        if typ != EntityType::Player && typ != EntityType::Enemy {
            return;
        }
        let entity_pos = self.level.entities.pos[&entity_id];

        // emit visible movement positions
        if in_fov == FovResult::Inside {
            if let Some(reach) = self.level.entities.movement.get(&entity_id) {
                let mut reach = *reach;

                // The player's reach is reported using the move mode instead of stance.
                if entity_id == player_id {
                    reach = reach_by_mode(self.settings.move_mode);
                }

                for move_pos in reach.reachables(entity_pos) {
                    if !self.level.map.is_within_bounds(move_pos) {
                        continue;
                    }

                    if self.level.pos_in_fov(player_id, move_pos) {
                        self.msg_log.log(Msg::EntityMovement(entity_id, move_pos));
                    }
                }
            }
        }

        // emit visible attack positions
        if in_fov == FovResult::Inside {
            if let Some(reach) = self.level.entities.attack.get(&entity_id) {
                for attack_pos in reach.reachables(entity_pos) {
                    if !self.level.map.is_within_bounds(attack_pos) {
                        continue;
                    }

                    if self.level.pos_in_fov(entity_id, attack_pos) &&
                       (self.level.clear_path(entity_pos, attack_pos, false) || attack_pos == player_pos) {
                        self.msg_log.log(Msg::EntityAttack(entity_id, attack_pos));
                    }
                }
            }
        }

        // emit visible tiles for entity that are visible to player
        if in_fov == FovResult::Inside && entity_id != player_id {
            for pos in player_fov.iter() {
                if self.level.pos_in_fov(entity_id, *pos) {
                    self.msg_log.log(Msg::EntityFov(entity_id, *pos));
                }
            }
        }
    }

    fn emit_took_turn_state(self: &mut Game) {
        // indicate FoV information
        let player_fov = self.emit_player_fov();

        // NOTE(perf) unnecessary clone
        for entity_id in self.level.entities.ids.clone().iter() {
            let entity_pos = self.level.entities.pos[&entity_id];
            if !self.level.map.is_within_bounds(entity_pos) {
                continue;
            }
            self.emit_entity_information(*entity_id, &player_fov);
        }
    }

    fn emit_any_action_state(self: &mut Game) {
        self.msg_log.log(Msg::PlayerAction);

        if self.settings.state == GameState::Use {
            self.emit_use_mode_messages();
        }

        // indicate player ghost position based on cursor, if in cursor mode
        self.emit_cursor_ghost_position();

        // report entities at the cursor position
        self.emit_entities_at_cursor();

        // report current player inventory
        // this is here because picking up and dropping items does not take a turn
        self.emit_inventory();
    }

    pub fn emit_turn_messages(&mut self) {
        let player_id = self.level.find_by_name(EntityName::Player).unwrap();
        let player_pos = self.level.entities.pos[&player_id];
        let reach = reach_by_mode(self.settings.move_mode);
        for move_pos in reach.reachables(player_pos) {
            if !self.level.map.is_within_bounds(move_pos) {
                continue;
            }

            if self.level.pos_in_fov(player_id, move_pos) {
                self.msg_log.log(Msg::EntityMovement(player_id, move_pos));
            }
        }
    }

    fn emit_inventory(self: &mut Game) {
        let player_id = self.level.find_by_name(EntityName::Player).unwrap();

        for item_id in self.level.entities.inventory[&player_id].iter() {
            let item = self.level.entities.item[&item_id];
            let item_class = item.class();
            self.msg_log.log(Msg::InventoryItem(item, item_class));
        }
    }

    fn emit_entities_at_cursor(self: &mut Game) {
        if let Some(cursor_pos) = self.settings.cursor {
            let entities = self.level.get_entities_at_pos(cursor_pos);
            for entity in entities {
                self.msg_log.log(Msg::EntityAtCursor(entity));
            }
        }
    }

    fn emit_use_mode_messages(self: &mut Game) {
        let player_id = self.level.find_by_name(EntityName::Player).unwrap();

        // if in use-mode, output use-direction.
        if let UseAction::Item(item_class) = self.settings.use_action {
            if let Some(item_index) = self.level.find_item(item_class) {
                if let Some(use_dir) = self.settings.use_dir {
                    let use_result = self.level.calculate_use_move(player_id,
                                                                   item_index,
                                                                   use_dir,
                                                                   self.settings.move_mode);

                    if let Some(pos) = use_result.pos {
                        self.msg_log.log(Msg::UsePos(pos));
                    }

                    if let Some(dir) = self.settings.use_dir {
                        self.msg_log.log(Msg::UseDir(dir));
                    }

                    self.msg_log.log(Msg::UseHitPosClear);
                    for pos in use_result.hit_positions {
                        self.msg_log.log(Msg::UseHitPos(pos));
                    }
                }
            }
        } else if self.settings.use_action == UseAction::Interact {
            if let Some(dir) = self.settings.use_dir {
                self.msg_log.log(Msg::UseDir(dir));
            }

            let player_pos = self.level.entities.pos[&player_id];
            for pos in Reach::Single(1).reachables(player_pos) {
                //self.msg_log.log(Msg::UseHitPos(pos));
                //self.msg_log.log(Msg::UsePos(pos));
                let dir = Direction::from_positions(player_pos, pos).unwrap();
                self.msg_log.log(Msg::UseOption(pos, dir));
            }
        }
    }
    
    fn emit_cursor_ghost_position(self: &mut Game) {
        let player_id = self.level.find_by_name(EntityName::Player).unwrap();
        let player_pos = self.level.entities.pos[&player_id];
        
        if let Some(cursor_pos) = self.settings.cursor {
            if cursor_pos != player_pos && self.input.target == None {
                let maybe_next_pos = astar_next_pos(&self.level.map, player_pos, cursor_pos, None, None);
                if let Some(next_pos) = maybe_next_pos {
                    let dxy = sub_pos(next_pos, player_pos);
                    let direction = Direction::from_dxy(dxy.x, dxy.y).unwrap();

                    let mut reach = reach_by_mode(MoveMode::Sneak);
                    if !self.input.cursor && self.input.shift {
                        reach = reach_by_mode(MoveMode::Run);
                    }

                    if let Some(player_ghost_pos) = reach.furthest_in_direction(player_pos, direction) {
                        self.msg_log.log(Msg::PlayerGhost(player_ghost_pos));
                    }
                }
            }
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
pub struct Settings {
    pub turn_count: usize,
    pub test_mode: bool,
    pub map_type: MapGenType,
    pub state: GameState,
    pub overlay: bool,
    pub level_num: usize,
    pub running: bool,
    pub cursor: Option<Pos>,
    pub use_action: UseAction,
    pub use_dir: Option<Direction>,
    pub move_mode: MoveMode,
    pub debug_enabled: bool,
    pub map_load_config: MapLoadConfig,
    pub map_changed: bool,
}

impl Settings {
    pub fn new() -> Settings {
        return Settings {
            turn_count: 0,
            test_mode: false,
            map_type: MapGenType::Island,
            state: GameState::Playing,
            overlay: false,
            level_num: 0,
            running: true,
            cursor: None,
            use_action: UseAction::Interact,
            use_dir: None,
            move_mode: MoveMode::Walk,
            debug_enabled: false,
            map_load_config: MapLoadConfig::Empty,
            map_changed: false,
        };
    }
}

