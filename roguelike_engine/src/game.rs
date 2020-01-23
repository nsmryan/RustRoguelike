use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use rand::prelude::*;

use slotmap::dense::*;

use roguelike_core::constants::*;
use roguelike_core::types::*;
use roguelike_core::config::*;
use roguelike_core::ai::*;
use roguelike_core::map::*;

use crate::generation::*;
use crate::display::*;
use crate::input::*;
use crate::read_map::*;
use crate::actions;


#[derive(Copy, Clone, PartialEq)]
pub enum GameResult {
    Continue,
    Stop,
}

pub struct GameSettings {
    pub previous_player_position: Pos,
    pub turn_count: usize,
    pub god_mode: bool,
    pub map_type: MapGenType,
    pub exiting: bool,
}

impl GameSettings {
    pub fn new(previous_player_position: Pos,
               turn_count: usize,
               god_mode: bool) -> GameSettings {
        GameSettings {
            previous_player_position,
            turn_count,
            god_mode,
            map_type: MapGenType::Island,
            exiting: false,
        }
    }
}

pub struct Game<'a> {
    pub config: Config,

    pub input_action: InputAction,

    pub mouse_state: MouseState,

    pub display_state: DisplayState<'a>,

    pub data: GameData,

    pub settings: GameSettings,

    pub state: GameState,
}

impl<'a> Game<'a> {
    pub fn new(args: &Vec<String>,
               config: Config,
               display_state: DisplayState<'a>) -> Result<Game<'a>, String> {
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

        let previous_player_position = Pos::new(-1, -1);

        let mut objects = DenseSlotMap::with_capacity(INITIAL_OBJECT_CAPACITY);
        let mut rng: SmallRng = SeedableRng::seed_from_u64(seed);

        let mut map;
        let player_position;
        match config.map_load {
            MapLoadConfig::FromFile => {
                let (new_objects, new_map, mut position) = read_map_xp(&config, &display_state, "resources/map.xp");
                objects.clear();
                for object in new_objects.values() {
                    objects.insert(object.clone());
                }
                map = new_map;
                if position == (0, 0) {
                    position = (map.width() / 2, map.height() / 2);
                }
                player_position = position;

                objects.insert(make_goal(&config, Pos::new(player_position.0 - 1, player_position.1)));
                objects.insert(make_mouse(&config, &display_state));
                let exit_position = (player_position.0 + 1, player_position.1 - 1);
                map[exit_position].tile_type = TileType::Exit;
                map[exit_position].chr = Some(MAP_ORB as char);
            }

            MapLoadConfig::Random => {
                let (game_data, position) =
                    make_map(&MapGenType::Island, &mut objects, &config, &display_state, &mut rng);
                // TODO consider using objects as well here on regen?
                map = game_data.map;
                player_position = position.to_tuple();
            }

            MapLoadConfig::TestWall => {
                let (new_map, position) = make_wall_test_map(&mut objects, &config, &display_state);
                map = new_map;
                player_position = position.to_tuple();
            }

            MapLoadConfig::TestCorner => {
                let (new_map, position) = make_corner_test_map(&mut objects, &config, &display_state);
                map = new_map;
                player_position = position.to_tuple();
                objects.insert(make_mouse(&config, &display_state));
            }
        }

        let mut data = GameData::new(map, objects);

        let player_handle = data.objects.insert(make_player(&config, &display_state));
        data.objects[player_handle].x = player_position.0;
        data.objects[player_handle].y = player_position.1;

        let stone_handle = data.objects.insert(make_stone(&config, Pos::new(-1, -1)));
        data.objects[player_handle].inventory.push(stone_handle);

        let state = Game {
            config,
            input_action: InputAction::None,
            data,
            display_state,
            settings: GameSettings::new(previous_player_position, 0, false),
            mouse_state: Default::default(),
            state: GameState::Playing,
        };

        Ok(state)
    }

    pub fn step_game(&mut self) -> GameResult {

        match self.state {
            GameState::Playing => {
                return self.step_playing();
            }

            GameState::Win => {
                return self.step_win();
            }

            GameState::Lose => {
                return self.step_lose();
            }
        }
    }

    fn step_win(&mut self) -> GameResult {

        match self.input_action {
            InputAction::Exit => {
                return GameResult::Stop;
            }

            _ => {},
        }

        let player_handle = self.data.find_player().unwrap();

        let (new_objects, new_map, _) = read_map_xp(&self.config, &self.display_state, "resources/map.xp");
        self.data.map = new_map;
        self.data.objects[player_handle].inventory.clear();
        let player = self.data.objects[player_handle].clone();
        self.data.objects.clear();
        self.data.objects.insert(player);
        for key in new_objects.keys() {
            self.data.objects.insert(new_objects[key].clone());
        }

        self.state = GameState::Playing;

        // NOTE Exit game on win for now
        return GameResult::Stop;
    }

    fn step_lose(&mut self) -> GameResult {
        if self.input_action == InputAction::Exit {
            return GameResult::Stop;
        }

        return GameResult::Continue;
    }

    fn step_playing(&mut self) -> GameResult {
        let player_handle = self.data.find_player().unwrap();

        /* Player Action and Animations */
        self.settings.previous_player_position =
            self.data.objects[player_handle].pos();

        let player_action =
            actions::handle_input(self.input_action,
                                  &mut self.mouse_state,
                                  &mut self.data,
                                  &mut self.settings,
                                  &mut self.display_state,
                                  &self.config);
        if player_action != Action::NoAction {
            self.settings.turn_count += 1;
        }

        if self.settings.exiting {
            return GameResult::Stop;
        }

        /* Check Exit Condition */
        if exit_condition_met(&self.data) {
            self.state = GameState::Win;
        }

        /* AI */
        if self.data.objects[player_handle].alive && player_action != Action::NoAction {
            let mut ai_handles = Vec::new();

            for key in self.data.objects.keys() {
                if self.data.objects[key].ai.is_some() &&
                   self.data.objects[key].fighter.is_some() {
                       ai_handles.push(key);
                   }
            }

            for key in ai_handles {
                ai_take_turn(key, &mut self.data);

                // check if fighter needs to be removed
                if let Some(fighter) = self.data.objects[key].fighter {
                    if fighter.hp <= 0 {
                        self.data.objects[key].alive = false;
                        self.data.objects[key].chr = '%';
                        self.data.objects[key].color = self.config.color_red;
                        self.data.objects[key].fighter = None;

                        self.data.objects.remove(key);
                    }
                }
            }
        }

        // check if player lost all hp
        if let Some(fighter) = self.data.objects[player_handle].fighter {
            if fighter.hp <= 0 {
                {
                    let player = &mut self.data.objects[player_handle];
                    player.alive = false;
                    player.chr = '%';
                    player.color = self.config.color_red;
                    player.fighter = None;
                }

                if self.state == GameState::Playing {
                    self.state = GameState::Lose;
                }
            }
        }

        /* Reload Configuration */
        match File::open("config.json") {
            Ok(mut file) => {
                let mut config_string = String::new();
                file.read_to_string(&mut config_string).expect("Could not read config file!");
                self.config = serde_json::from_str(&config_string).expect("Could not read JSON- config.json has a parsing error!");
            }
            _ => (),
        }

        /* Reload map if configured to do so */
        if self.config.load_map_file_every_frame && Path::new("resources/map.xp").exists() {
            let (new_objects, new_map, _) = read_map_xp(&self.config, &self.display_state, "resources/map.xp");
            self.data.map = new_map;
            self.data.objects[player_handle].inventory.clear();
            let player = self.data.objects[player_handle].clone();
            self.data.objects.clear();
            for key in new_objects.keys() {
                self.data.objects.insert(new_objects[key].clone());
            }
            self.data.objects.insert(player);
        }

        /* Recompute FOV */
        let player_pos = self.data.objects[player_handle].pos();
        if self.settings.previous_player_position != player_pos {
            self.data.map.compute_fov(player_pos, FOV_RADIUS);
        }

        self.input_action = InputAction::None;

        return GameResult::Continue;
    }
}

/// Check whether the exit condition for the game is met.
fn exit_condition_met(data: &GameData) -> bool {
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

