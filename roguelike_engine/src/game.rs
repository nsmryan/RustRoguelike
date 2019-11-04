use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use rand::prelude::*;

use ggez::{Context, GameResult};
use ggez::event::{self, EventsLoop, EventHandler, KeyCode, KeyMods, MouseButton};

use roguelike_core::constants::*;
use roguelike_core::generation::*;
use roguelike_core::types::*;
use roguelike_core::config::*;
use roguelike_core::ai::*;
use roguelike_core::map::*;

use crate::display::*;
use crate::input::*;
use crate::read_map::*;
use crate::actions::*;


pub struct Game {
    pub config: Config,

    pub input_action: InputAction,

    pub mouse_state: MouseState,

    pub display_state: DisplayState,

    pub data: GameData,

    pub settings: GameSettings,

    pub state: GameState,
}

impl Game {
    pub fn new(args: &Vec<String>,
               config: Config,
               display_state: DisplayState) -> GameResult<Game> {
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

        let mut objects = vec!(make_player(&config));

        let mut rng: SmallRng = SeedableRng::seed_from_u64(seed);

        let map;
        let player_position;
        if config.load_map_file {
            let (new_objects, new_map, mut position) = read_map_xp(&config, "map.xp");
            objects.extend(new_objects);
            map = new_map;
            if position == (0, 0) {
                position = (map.width() / 2, map.height() / 2);
            }
            player_position = position;
        } else {
            let (new_map, position) = make_map(&mut objects, &config, &mut rng);
            map = new_map;
            player_position = position.into_pair();
        }
        let player_x = player_position.0;
        let player_y = player_position.1;
        objects[PLAYER].x = player_x;
        objects[PLAYER].y = player_y;

        let data = GameData::new(map, objects);

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
        /* Player Action and Animations */
        self.settings.previous_player_position = (self.data.objects[PLAYER].x, self.data.objects[PLAYER].y);
        let player_action;
        player_action = handle_input(self.input_action,
                                     &mut self.mouse_state,
                                     &mut self.data,
                                     &mut self.settings.god_mode,
                                     &mut self.display_state.display_overlays,
                                     &self.config);
        match player_action {
            PlayerAction::Exit => {
                return false;
            }

            PlayerAction::TookTurn | PlayerAction::TookHalfTurn => {
                self.settings.turn_count += 1;
            }

            _ => {}
        }

        /* Check Exit Condition */
        if exit_condition_met(&self.data.map, &mut self.data.objects) {
            std::process::exit(0);
        }

        /* AI */
        if self.data.objects[PLAYER].alive && player_action == PlayerAction::TookTurn {
            for id in 1..self.data.objects.len() {
                if self.data.objects[id].ai.is_some() {
                    ai_take_turn(id, &mut self.data.map, &mut self.data.objects);
                    if let Some(fighter) = self.data.objects[id].fighter {
                        if fighter.hp <= 0 {
                            self.data.objects[id].alive = false;
                            self.data.objects[id].chr = '%';
                            self.data.objects[id].color = self.config.color_red;
                            self.data.objects[id].fighter = None;
                        }
                    }
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
            let player = self.data.objects[0].clone();
            self.data.objects.clear();
            self.data.objects.push(player);
            self.data.objects.extend(new_objects);
        }

        /* Recompute FOV */
        if self.settings.previous_player_position != (self.data.objects[PLAYER].x, self.data.objects[PLAYER].y) {
            self.data.map.compute_fov(self.data.objects[PLAYER].x, self.data.objects[PLAYER].y, FOV_RADIUS);
        }

        self.input_action = InputAction::None;

        return false; 
    }
}

impl EventHandler for Game {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        self.step_game();

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        render_all(ctx,
                   &mut self.mouse_state,
                   &self.data.objects,
                   &mut self.data.map,
                   &mut self.display_state,
                   &self.config)
    }

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

    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        keycode: KeyCode,
        keymods: KeyMods,
        _repeat: bool,
        ) {
        self.input_action = map_keycode_to_action(keycode, keymods);
    }
}

// TODO figure out where to put this. should depend on state
/// Check whether the exit condition for the game is met.
fn exit_condition_met(map: &Map, objects: &[Object]) -> bool {
    // loop over objects in inventory, and check whether any
    // are the goal object.
    //let has_goal =
    //inventory.iter().any(|obj| obj.item.map_or(false, |item| item == Item::Goal));
    // TODO add back in with new inventory!
    let has_goal = false;

    let player_pos = (objects[PLAYER].x, objects[PLAYER].y);
    let on_exit_tile = map[player_pos].tile_type == TileType::Exit;

    let exit_condition = has_goal && on_exit_tile;

    return exit_condition;
}
