use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use rand::prelude::*;

use slotmap::dense::*;

use roguelike_core::constants::*;
use roguelike_core::generation::*;
use roguelike_core::types::*;
use roguelike_core::config::*;
use roguelike_core::ai::*;
use roguelike_core::map::*;

use crate::display::*;
use crate::input::*;
use crate::read_map::*;
use crate::actions;


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

        let previous_player_position = (-1, -1);

        let mut objects = DenseSlotMap::with_capacity(INITIAL_OBJECT_CAPACITY);
        let mut rng: SmallRng = SeedableRng::seed_from_u64(seed);

        let map;
        let player_position;
        match config.map_load {
            MapLoadConfig::FromFile => {
                let (new_objects, new_map, mut position) = read_map_xp(&config, "map.xp");
                objects.clear();
                for object in new_objects.values() {
                    objects.insert(object.clone());
                }
                map = new_map;
                if position == (0, 0) {
                    position = (map.width() / 2, map.height() / 2);
                }
                player_position = position;
            }

            MapLoadConfig::Random => {
                let (game_data, position) = make_map(&mut objects, &config, &mut rng);
                // TODO consider using objects as well here on regen?
                map = game_data.map;
                player_position = position.into_pair();
            }

            MapLoadConfig::TestWall => {
                let (new_map, position) = make_wall_test_map(&mut objects, &config);
                map = new_map;
                player_position = position.into_pair();
            }
        }

        let mut data = GameData::new(map, objects);

        let player_handle = data.objects.insert(make_player(&config));
        data.objects[player_handle].x = player_position.0;
        data.objects[player_handle].y = player_position.1;

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

    pub fn step_game(&mut self) -> bool {

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

    pub fn step_win(&mut self) -> bool {

        match self.input_action {
            InputAction::Exit => {
                return true;
            }

            _ => {},
        }

        return false;
    }

    pub fn step_lose(&mut self) -> bool {
        match self.input_action {
            InputAction::Exit => {
                return true;
            }

            _ => {},
        }

        return false;
    }

    pub fn step_playing(&mut self) -> bool {
        let player_handle = self.data.find_player().unwrap();

        /* Player Action and Animations */
        self.settings.previous_player_position =
            (self.data.objects[player_handle].x, self.data.objects[player_handle].y);

        let player_action;
        player_action = 
            actions::handle_input(self.input_action,
                                  &mut self.mouse_state,
                                  &mut self.data,
                                  &mut self.settings.god_mode,
                                  &mut self.display_state.display_overlays,
                                  &self.config);
        match player_action {
            PlayerAction::Exit => {
                return true;
            }

            PlayerAction::TookTurn | PlayerAction::TookHalfTurn => {
                self.settings.turn_count += 1;
            }

            _ => {}
        }

        /* Check Exit Condition */
        if exit_condition_met(&self.data) {
            self.state = GameState::Win;
        }

        /* AI */
        if self.data.objects[player_handle].alive && player_action == PlayerAction::TookTurn {
            let mut ai_handles = Vec::new();

            for key in self.data.objects.keys() {
                if self.data.objects[key].ai.is_some() &&
                   self.data.objects[key].fighter.is_some() {
                       ai_handles.push(key);
                   }
            }

            for key in ai_handles {
                ai_take_turn(key, &mut self.data);
                if let Some(fighter) = self.data.objects[key].fighter {
                    if fighter.hp <= 0 {
                        self.data.objects[key].alive = false;
                        self.data.objects[key].chr = '%';
                        self.data.objects[key].color = self.config.color_red;
                        self.data.objects[key].fighter = None;
                    }
                }
            }
        }

        // check is player lost all hp
        if let Some(fighter) = self.data.objects[player_handle].fighter {
            if fighter.hp <= 0 {
                {
                    let player = self.data.objects.get_mut(player_handle).unwrap();
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

        if self.config.load_map_file_every_frame && Path::new("map.xp").exists() {
            let (new_objects, new_map, _) = read_map_xp(&self.config, "map.xp");
            self.data.map = new_map;
            self.data.objects.clear();
            for key in new_objects.keys() {
                self.data.objects.insert(new_objects[key].clone());
            }
        }

        /* Recompute FOV */
        if self.settings.previous_player_position != (self.data.objects.get(player_handle).unwrap().x, self.data.objects.get(player_handle).unwrap().y) {
            self.data.map.compute_fov(self.data.objects[player_handle].x,
                                      self.data.objects[player_handle].y,
                                      FOV_RADIUS);
        }

        self.input_action = InputAction::None;

        return false; 
    }
}
/*
    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32, _dx: f32, _dy: f32) {
        self.mouse_state.pos = (x as i32, y as i32);
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        button: MouseButton,
        _x: f32,
        _y: f32,
        ) {
        self.mouse_state.pressed =
            (button == MouseButton::Left,
             button == MouseButton::Right,
             button == MouseButton::Middle);
    }

    fn mouse_button_up_event(
        &mut self,
        _ctx: &mut Context,
        _button: MouseButton,
        _x: f32,
        _y: f32,
        ) {
        self.mouse_state.pressed = (false, false, false);
    }
}
*/

// TODO figure out where to put this. should depend on state
/// Check whether the exit condition for the game is met.
fn exit_condition_met(data: &GameData) -> bool {
    // loop over objects in inventory, and check whether any
    // are the goal object.
    //let has_goal =
    //inventory.iter().any(|obj| obj.item.map_or(false, |item| item == Item::Goal));
    // TODO add back in with new inventory!
    let has_goal = false;

    let player_handle = data.find_player().unwrap();
    let player_pos = (data.objects[player_handle].x, data.objects[player_handle].y);
    let on_exit_tile = data.map[player_pos].tile_type == TileType::Exit;

    let exit_condition = has_goal && on_exit_tile;

    return exit_condition;
}

