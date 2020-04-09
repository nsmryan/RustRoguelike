use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use rand::prelude::*;

use slotmap::dense::*;

use serde::{Serialize, Deserialize};

use roguelike_core::constants::*;
use roguelike_core::types::*;
use roguelike_core::config::*;
use roguelike_core::ai::*;
use roguelike_core::map::*;
use roguelike_core::messaging::{Msg, MsgLog};
use roguelike_core::movement::Action;

use crate::actions;
use crate::generation::*;
use crate::display::*;
use crate::input::*;
use crate::read_map::read_map_xp;


#[derive(Copy, Clone, PartialEq, Serialize, Deserialize)]
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
    pub draw_throw_overlay: bool,
    pub overlay: bool,
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
            draw_throw_overlay: false,
            overlay: false,
        };
    }
}

pub struct Game {
    pub config: Config,

    pub input_action: InputAction,

    pub mouse_state: MouseState,

    pub display_state: DisplayState,

    pub data: GameData,

    pub settings: GameSettings,

    pub msg_log: MsgLog,
}

impl Game {
    pub fn new(args: &Vec<String>,
               config: Config,
               mut display_state: DisplayState) -> Result<Game, String> {
        // Create seed for random number generator, either from
        // user input or randomly
        let seed: u64;
        if args.len() > 1 {
            let mut hasher = DefaultHasher::new();
            args[1].hash(&mut hasher);
            seed = hasher.finish();
        } else {
            seed = rand::thread_rng().gen();
        }
        println!("Seed: {} (0x{:X})", seed, seed);

        let mut objects = DenseSlotMap::with_capacity(INITIAL_OBJECT_CAPACITY);
        let mut rng: SmallRng = SeedableRng::seed_from_u64(seed);

        let mut map;
        let player_position;
        match config.map_load {
            MapLoadConfig::FromFile => {
                let (new_objects, new_map, mut position) = read_map_xp(&config, &mut display_state, "resources/map.xp");
                objects.clear();
                for object in new_objects.values() {
                    objects.insert(object.clone());
                }
                map = new_map;
                if position == (0, 0) {
                    position = (map.width() / 2, map.height() / 2);
                }
                player_position = position;

                objects.insert(make_goal(&config, &mut display_state, Pos::new(player_position.0 - 1, player_position.1)));
                objects.insert(make_mouse(&config, &mut display_state));
                objects.insert(make_spikes(&config, Pos::new(player_position.0, player_position.1 - 2), &mut display_state));
                objects.insert(make_dagger(&config, Pos::new(player_position.0, player_position.1 - 3)));
                objects.insert(make_dagger(&config, Pos::new(player_position.0 + 1, player_position.1 - 2)));
                objects.insert(make_shield(&config, Pos::new(player_position.0 + 2, player_position.1 - 1)));

                let exit_position = (player_position.0 + 1, player_position.1 - 1);
                map[exit_position].tile_type = TileType::Exit;
                map[exit_position].chr = Some(MAP_ORB as char);
            }

            MapLoadConfig::Random => {
                let (game_data, position) =
                    make_map(&MapGenType::Island, &mut objects, &config, &mut display_state, &mut rng);
                // TODO consider using objects as well here on regen?
                map = game_data.map;
                player_position = position.to_tuple();
            }

            MapLoadConfig::TestWall => {
                let (new_map, position) = make_wall_test_map(&mut objects, &config, &mut display_state);
                map = new_map;
                player_position = position.to_tuple();
            }

            MapLoadConfig::TestPlayer => {
                let (new_map, position) = make_player_test_map(&mut objects, &config, &mut display_state);
                map = new_map;
                player_position = position.to_tuple();
            }

            MapLoadConfig::TestCorner => {
                let (new_map, position) = make_corner_test_map(&mut objects, &config, &mut display_state);
                map = new_map;
                player_position = position.to_tuple();
                objects.insert(make_mouse(&config, &mut display_state));
            }

            MapLoadConfig::Animations => {
                let (new_map, position) = make_animations_map(&mut objects, &config, &mut display_state);
                map = new_map;
                player_position = position.to_tuple();
            }
        }

        let mut data = GameData::new(map, objects);

        let player_handle = data.objects.insert(make_player(&config, &mut display_state));
        data.objects[player_handle].x = player_position.0;
        data.objects[player_handle].y = player_position.1;

        let stone_handle = data.objects.insert(make_stone(&config, Pos::new(-1, -1)));
        data.objects[player_handle].inventory.push_back(stone_handle);

        let state = Game {
            config,
            input_action: InputAction::None,
            data,
            display_state,
            settings: GameSettings::new(0, false),
            mouse_state: Default::default(),
            msg_log: MsgLog::new(),
        };

        return Ok(state);
    }

    pub fn step_game(&mut self) -> GameResult {

        match self.settings.state {
            GameState::Playing => {
                return self.step_playing();
            }

            GameState::Win => {
                return self.step_win();
            }

            GameState::Lose => {
                return self.step_lose();
            }

            GameState::Inventory => {
                return self.step_inventory();
            }

            GameState::Throwing => {
                return self.step_throwing();
            }
        }
    }

    fn step_win(&mut self) -> GameResult {

        if matches!(self.input_action, InputAction::Exit) {
            return GameResult::Stop;
        }

        let player_handle = self.data.find_player().unwrap();

        let (new_objects, new_map, _) =
            read_map_xp(&self.config, &mut self.display_state, "resources/map.xp");

        self.data.map = new_map;
        self.data.objects[player_handle].inventory.clear();
        let player = self.data.objects[player_handle].clone();
        self.data.objects.clear();
        self.data.objects.insert(player);
        for key in new_objects.keys() {
            self.data.objects.insert(new_objects[key].clone());
        }

        self.settings.state = GameState::Playing;

        // NOTE Exit game on win for now
        return GameResult::Stop;
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

    fn step_throwing(&mut self) -> GameResult {
        let input = self.input_action;
        self.input_action = InputAction::None;

        self.settings.draw_throw_overlay = true;

        let player_action =
            actions::handle_input_throwing(input,
                                           &mut self.data,
                                           &mut self.settings,
                                           &mut self.msg_log);

        if player_action != Action::NoAction {
            step_logic(player_action,
                       &mut self.data,
                       &mut self.settings,
                       &self.config,
                       &mut self.msg_log);
        }

        if self.settings.exiting {
            return GameResult::Stop;
        }

        return GameResult::Continue;
    }

    fn step_playing(&mut self) -> GameResult {
        // clear input action
        let input = self.input_action;
        self.input_action = InputAction::None;

        let player_action =
            actions::handle_input(input,
                                  &mut self.data,
                                  &mut self.settings,
                                  &mut self.display_state,
                                  &mut self.msg_log,
                                  &self.config);

        if player_action != Action::NoAction {
            step_logic(player_action,
                       &mut self.data,
                       &mut self.settings,
                       &self.config,
                       &mut self.msg_log);

            if win_condition_met(&self.data) {
                self.settings.state = GameState::Win;
            }
            self.settings.turn_count += 1;
        }

        if self.settings.exiting {
            return GameResult::Stop;
        }

        return GameResult::Continue;
    }
}

/// Check whether the exit condition for the game is met.
fn win_condition_met(data: &GameData) -> bool {
    // loop over objects in inventory, and check whether any
    // are the goal object.
    //let has_goal =
    //inventory.iter().any(|obj| obj.item.map_or(false, |item| item == Item::Goal));
    // TODO add back in with new inventory!
    let player_handle = data.find_player().unwrap();

    let has_goal = 
        data.objects[player_handle].inventory.iter().any(|item_handle| {
            data.objects[*item_handle].item == Some(Item::Goal)
        });

    let player_pos = data.objects[player_handle].pos();
    let on_exit_tile = data.map[player_pos].tile_type == TileType::Exit;

    let exit_condition = has_goal && on_exit_tile;

    return exit_condition;
}

pub fn step_logic(player_action: Action,
                  game_data: &mut GameData, 
                  settings: &mut GameSettings,
                  config: &Config,
                  msg_log: &mut MsgLog) {
    let player_handle = game_data.find_player().unwrap();

    let previous_player_position =
        game_data.objects[player_handle].pos();

    actions::player_apply_action(player_action, game_data, msg_log);

    /* AI */
    if game_data.objects[player_handle].alive {
        let mut ai_handles = Vec::new();

        for key in game_data.objects.keys() {
            if game_data.objects[key].ai.is_some() &&
               game_data.objects[key].alive        &&
               game_data.objects[key].fighter.is_some() {
               ai_handles.push(key);
           }
        }

        for key in ai_handles {
            ai_take_turn(key, game_data, config, msg_log);

            // check if fighter needs to be removed
            if let Some(fighter) = game_data.objects[key].fighter {
                if fighter.hp <= 0 {
                    game_data.objects[key].alive = false;
                    game_data.objects[key].blocks = false;
                    game_data.objects[key].chr = '%';
                    //game_data.objects[key].color = config.color_red;
                    game_data.objects[key].fighter = None;
                }
            }
        }
    }

    /* Traps */
    let mut traps = Vec::new();
    for key in game_data.objects.keys() {
        for other in game_data.objects.keys() {
            if game_data.objects[key].trap.is_some() && // key is a trap
               game_data.objects[other].alive && // entity is alive
               game_data.objects[other].fighter.is_some() && // entity is a fighter
               game_data.objects[key].pos() == game_data.objects[other].pos() {
                traps.push((key, other));
            }
        }
    }

    for (trap, entity) in traps.iter() {
        match game_data.objects[*trap].trap.unwrap() {
            Trap::Spikes => {
                game_data.objects[*entity].take_damage(SPIKE_DAMAGE);

                msg_log.log(Msg::SpikeTrapTriggered(*trap, *entity));
            }

            Trap::Sound => {
                msg_log.log(Msg::SoundTrapTriggered(*trap, *entity));
            }
        }
    }

    // TODO move enemy health checks here for trap damage

    // check if player lost all hp
    if let Some(fighter) = game_data.objects[player_handle].fighter {
        if fighter.hp <= 0 {
            // modify player
            {
                let player = &mut game_data.objects[player_handle];
                player.alive = false;
                player.color = config.color_red;
                player.fighter = None;
            }

            if settings.state == GameState::Playing {
                settings.state = GameState::Lose;
            }
        }
    }

    let mut to_remove = Vec::new();
    for (entity_key, entity) in game_data.objects.iter_mut() {
        if let Some(ref mut count) = entity.count_down {
            if *count == 0 {
                to_remove.push(entity_key);
            } else {
                *count -= 1;
            }
        }
    }
    for key in to_remove {
        game_data.objects.remove(key);
    }

    /* Recompute FOV */
    let player_pos = game_data.objects[player_handle].pos();
    if previous_player_position != player_pos {
        game_data.map.compute_fov(player_pos, PLAYER_FOV_RADIUS);
    }
}

