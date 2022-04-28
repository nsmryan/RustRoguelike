mod throttler;
mod keyboard;
mod replay;

use std::fs;
use std::io::{BufRead, Write, Cursor};
use std::time::{Duration, Instant, SystemTime};
use std::path::Path;
use std::str::FromStr;
use std::thread;
use std::sync::mpsc::{self, channel, Receiver};

use log::LevelFilter;
use simple_logging;
use logging_timer::timer;

use gumdrop::Options;

use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};

use roguelike_map::MapLoadConfig;

use roguelike_core::types::*;
use roguelike_core::config::Config;
use roguelike_core::constants::*;

use roguelike_engine::game::*;
use roguelike_engine::actions::*;
use roguelike_engine::input::*;
use roguelike_engine::map_construct::map_construct;
use roguelike_engine::log::*;

use roguelike_lib::commands::*;

use roguelike_display::load::*;
use roguelike_display::display::*;
use roguelike_display::render::*;

use crate::throttler::*;
use crate::replay::*;


pub const CONFIG_NAME: &str = "config.yaml";
pub const GAME_SAVE_FILE: &str = "game.save";

#[derive(Debug, Clone, Options)]
pub struct GameOptions {
    #[options(help = "replay from an input log file", short="r")]
    pub replay: Option<String>,

    #[options(help = "record a session with the given name", short="d")]
    pub record: Option<String>,

    #[options(help = "re-record a session with the given name", short="o")]
    pub rerecord: Option<String>,

    #[options(help = "check a previous recorded session against current version", short="c")]
    pub check: Option<String>,

    #[options(help = "delay value in milliseconds used when replaying commands", short="y")]
    pub delay: Option<u64>,

    #[options(help = "load using the given map configuration", short="m")]
    pub map_config: Option<String>,

    #[options(help = "log level to record in game.log (OFF, ERROR, WARN, INFO, DEBUG, TRACE)")]
    pub log_level: Option<String>,

    #[options(help = "use a given seed for random number generation")]
    pub seed: Option<u64>,

    #[options(help = "take a screenshot and exit", short="t")]
    pub screenshot: bool,

    #[options(help = "procgen map config", short="g")]
    pub procgen_map: Option<String>,

    #[options(help = "display help text")]
    pub help: bool,
}


fn main() {
    let opts = GameOptions::parse_args_default_or_exit();

    let seed: u64 =
        if let Some(given_seed) = opts.seed {
            given_seed
        } else {
            1
        };

    eprintln!("Seed: {} (0x{:X})", seed, seed);

    let log_level =
        opts.log_level.clone().map_or(LevelFilter::Off,
                                      |level_str| LevelFilter::from_str(&level_str).expect("Log level unexpected!"));
    simple_logging::log_to_file("game.log", log_level).unwrap();

    run(seed, opts).unwrap();
}

pub fn run(seed: u64, opts: GameOptions) -> Result<(), String> {
    /* Create SDL Context */
    let sdl_context = sdl2::init()?;

    let mut display;
    let mut timer;
    let canvas;

    {
        let video = sdl_context.video()?;
        timer = sdl_context.timer()?;
        let window = video.window("Rust Roguelike", SCREEN_WIDTH, SCREEN_HEIGHT)
                          .position_centered().build().map_err(|e| e.to_string())?;

        canvas = window.into_canvas()
                       .accelerated()
                       .build()
                       .map_err(|e| e.to_string())?;

        let texture_creator = canvas.texture_creator();

        /* Create Display Structures */
        display = Display::new(canvas);

        /* Load Textures */
        let sheet = true;
        load_sprites(&texture_creator, &mut display);
        load_sprite(&texture_creator, sheet, &mut display, "resources/rustrogueliketiles.png", "tiles");
        load_sprite(&texture_creator, sheet, &mut display, "resources/shadowtiles.png", "shadows");
        load_sprite(&texture_creator, sheet, &mut display, "resources/Particle_Speck.png", "particle_speck");

        let ttf_context = sdl2::ttf::init().expect("Could not init SDL2 TTF!");
        let font_texture = load_font(&ttf_context,
                                     &texture_creator,
                                     "Inconsolata-Bold.ttf".to_string(),
                                     24);
        display.add_spritesheet("font".to_string(), font_texture);
    }

    /* Create Game Structure */
    let config = Config::from_file(CONFIG_NAME);

    let mut game = Game::new(seed, config.clone());
    game.load_vaults("resources/vaults/");

    let mut game_from_file = false;
    if config.save_load {
        if let Some((game_loaded, display_loaded)) = load_save(GAME_SAVE_FILE) {
            game = game_loaded;
            display.state = display_loaded;
            game_from_file = true;
        }
    }

    /* Create Map */
    let map_config: MapLoadConfig = create_map_config(&opts, &config);

    // save map config to a file
    let mut map_config_file = std::fs::File::create(MAP_CONFIG_NAME).unwrap();
    map_config_file.write_all(map_config.to_string().as_bytes()).unwrap();

    /* Run Game, Check Recording, or Rerecord */
    if let Some(record_name) = opts.check {
        /* Check Recording */
        let delay = opts.delay.unwrap_or(0);
        let mut event_pump = sdl_context.event_pump().unwrap();

        if record_name == "all" {
            return check_all_records(&mut game, &mut display, &mut event_pump, delay);
        } else {
            return check_single_record(&mut game, &mut display, &mut event_pump, &record_name, delay);
        }
    } else if let Some(record_name) = opts.rerecord {
        /* Re-record */
        let delay = opts.delay.unwrap_or(0);
        let mut event_pump = sdl_context.event_pump().unwrap();
        if record_name == "all" {
            return rerecord_all(&mut game, &mut display, &mut event_pump, delay);
        } else {
            return rerecord_single(&mut game, &mut display, &mut event_pump, &record_name, delay);
        }
    } else {
        /* Run Game */
        if !game_from_file {
            map_construct(&map_config, &mut game);
        }
        let event_pump = sdl_context.event_pump().unwrap();
        return game_loop(game, display, opts, &mut timer, event_pump);
    }
}

pub fn game_loop(mut game: Game, mut display: Display, opts: GameOptions, timer: &mut sdl2::TimerSubsystem, mut event_pump: sdl2::EventPump) -> Result<(), String> {
    // read in the recorded action log, if one is provided
    let mut starting_actions = Vec::new();
    if let Some(replay_file) = &opts.replay {
        starting_actions = read_action_log(&replay_file);
    }

    let mut config_modified_time = fs::metadata(CONFIG_NAME).unwrap().modified().unwrap();

    let mut log = Log::new();
    let mut recording = Recording::new(&game, &display.state);

    /* Setup FPS Throttling */
    let frame_ms = 1000 / game.config.frame_rate as u64;
    let fps_throttler = Throttler::new(Duration::from_millis(frame_ms));

    /* Set up Input Handling */
    let io_recv = spawn_input_reader();

    /* Game Save Thread */
    // Serialization and storage take at least 6 ms, so this is done
    // in a separate thread to prevent taking time from the main loop.
    let (game_sender, game_receiver) = channel::<(Game, DisplayState)>();
    let _save_thread = thread::spawn(move || { save_game_thread(game_receiver); });

    // running the post step first sets up the game before the first turn.
    game.emit_state_messages();
    update_display(&mut game, &mut display, 0.1)?;
    game.msg_log.clear();
    display.clear_console_messages();

    /* Main Game Loop */
    let mut frame_time = Instant::now();
    while game.settings.running {
        let mut any_updates = false;

        let _loop_timer = timer!("GAME_LOOP");
        let frame_start_time = Instant::now();

        /* Input */
        let mut input_actions: Vec<InputAction> = Vec::new();
        {
            let _input_timer = timer!("INPUT");

            // check for commands to execute
            any_updates |= process_commands(&io_recv, &mut game, &mut log);

            // This is not the best timer, but input should not occur faster than 1 ms apart. Using
            // ticks is better then Instant for serialization.
            let ticks = timer.ticks();
            for sdl2_event in event_pump.poll_iter() {
                if let Some(event) = keyboard::translate_event(sdl2_event) {
                    // First check for [ and ], which are processed outside of the normal input
                    // system.
                    if game.config.recording && matches!(event, InputEvent::Char('[', KeyDir::Up)) {
                        let (new_game, new_display_state) = recording.backward();
                        game = new_game;
                        display.state = new_display_state;
                        any_updates = true;
                    } else if game.config.recording && matches!(event, InputEvent::Char(']', KeyDir::Up)) {
                        if let Some((new_game, new_display_state)) = recording.forward() {
                            game = new_game;
                            display.state = new_display_state;
                        }
                        any_updates = true;
                    } else {
                        let input_action = game.input.handle_event(&mut game.settings, event, ticks, &game.config);
                        input_actions.push(input_action);

                        any_updates |= input_action != InputAction::None;
                    }
                }
            }
        }

        /* Misc */
        {
            let _misc_timer = timer!("MISC");

            // if there are starting actions to read, pop one off to play
            if let Some(action) = starting_actions.pop() {
                input_actions.push(action);
            }

           /* Record Inputs to Log File */
            for input_action in input_actions.iter() {
                log.log_action(*input_action);
            }
        }

        /* Logic */
        {
            let _logic_timer = timer!("LOGIC");

            // if no actions, make sure to step the game anyway
            if input_actions.len() == 0 {
                game.step_game(InputAction::None);
            }

            for input_action in input_actions {
                game.step_game(input_action);
                
                if game.config.recording && input_action != InputAction::None {
                    recording.action(&game, &display.state, input_action);
                }

                log.console_output_only = !game.config.minimal_output;
                for msg_index in 0..game.msg_log.turn_messages.len() {
                    let msg = game.msg_log.turn_messages[msg_index];
                    let msg_line = &msg.msg_line(&game.level);
                    if msg_line.len() > 0 {
                        log.log_console(msg_line);
                    }

                    if !game.config.minimal_output {
                        log.log_msg(&format!("{}", msg));
                    }
                }

                if game.settings.state == GameState::Win {
                    // TODO probably not necessary, given level messaging
                    display.clear_level_state();
                    recording.clear();
                } else if game.settings.state == GameState::Exit {
                    game.settings.running = false;
                }
            }
        }

        /* Display */
        {
            let logic_time = Instant::now().duration_since(frame_start_time).as_secs_f32();
            display.state.show_debug("lt", format!("{}", logic_time));

            let _display_timer = timer!("DISPLAY");
            let dt = Instant::now().duration_since(frame_time).as_secs_f32();
            frame_time = Instant::now();

            display.state.show_debug("dt", format!("{}", dt));

            update_display(&mut game, &mut display, dt)?;

            let disp_time = Instant::now().duration_since(frame_time).as_secs_f32();
            display.state.show_debug("dr", format!("{}", disp_time));
        }

        game.msg_log.clear();

        /* Configuration */
        {
            let _config_timer = timer!("CONFIG");
            reload_config(&mut config_modified_time, &mut game);
        }

        /* Save Game */
        let player_id = game.level.find_by_name(EntityName::Player).unwrap();
        if game.level.entities.hp[&player_id].hp > 0 {
            if game.settings.running && any_updates && game.config.save_load {
                let old_state = game.settings.state;
                game.settings.state = GameState::Playing;
                display.state.state = GameState::Playing;
                game_sender.send((game.clone(), display.state.clone())).unwrap();
                game.settings.state = old_state;
                display.state.state = old_state;
            }
        } else {
            let path = Path::new(GAME_SAVE_FILE);
            if path.exists() {
                std::fs::remove_file(path).unwrap();
            }
        }

        /* Wait until the next tick to loop */
        {
            let frame_time = Instant::now().duration_since(frame_start_time).as_secs_f32();
            display.state.show_debug("ft", format!("{}", frame_time));

            let _wait_timer = timer!("WAIT");
            fps_throttler.wait();
        }
    }

    // NOTE we could also just put these files in the right place to begin with...
    if let Some(record_name) = opts.record {
        // save recorded logs
        save_record(&record_name);
    }

    return Ok(());
}

fn save_record(record_name: &str) {
    // create log directory if it doesn't exist
    let path = format!("resources/test_logs/{}", record_name);
    std::fs::create_dir_all(&path).expect("Could not create record directory!");

    // save all files to the new directory 
    std::fs::copy(Log::ACTION_LOG_NAME, format!("{}/{}", &path, Log::ACTION_LOG_NAME))
            .expect("Could not save action log!");

    std::fs::copy(Log::MESSAGE_LOG_NAME, format!("{}/{}", &path, Log::MESSAGE_LOG_NAME))
            .expect("Could not save message log!");

    std::fs::copy(MAP_CONFIG_NAME, format!("{}/{}", &path, MAP_CONFIG_NAME))
            .expect("Could not save map config!");
}

fn reload_config(config_modified_time: &mut SystemTime, game: &mut Game) {
    /* Reload Configuration */
    if let Ok(current_config_modified_time) = fs::metadata(CONFIG_NAME) {
        let current_config_modified_time = current_config_modified_time.modified().unwrap();
        if current_config_modified_time != *config_modified_time {
            *config_modified_time = current_config_modified_time;
            game.config = Config::from_file(CONFIG_NAME);
        }
    }
}

pub fn take_screenshot(game: &mut Game, display: &mut Display) -> Result<(), String> {
    game.settings.test_mode = true;

    game.step_game(InputAction::None);
    render_all(&mut display.panels, &mut display.state, &display.sprites, &game.config, 0.1)?;

    display.save_screenshot("screenshot");

    return Ok(());
}

fn update_display(game: &mut Game, display: &mut Display, dt: f32) -> Result<(), String> {
    for msg in game.msg_log.turn_messages.iter() {
        display.process_message(*msg, &game.level.map, &game.config);
        display.console_message(msg.msg_line(&game.level), &game.config);
    }

    let map_str = game.level.map.compact_chrs();
    display.map_message(&map_str);

    /* Draw the Game to the Screen */
    {
        let command_time = Instant::now();
        let _render_timer = timer!("RENDER");
        render_all(&mut display.panels, &mut display.state, &display.sprites, &game.config, dt)?;
        let ct = Instant::now().duration_since(command_time).as_secs_f32();
        display.state.show_debug("ct", format!("{}", ct));
    }

    {
        let _draw_timer = timer!("DRAW");
        let update_time = Instant::now();
        display.draw_all(game.level.map.size(), &mut game.rng, &game.config);

        {
            let _present_timer = timer!("PRESENT");
            display.update_display();
            let ut = Instant::now().duration_since(update_time).as_secs_f32();
            display.state.show_debug("ut", format!("{}", ut));
        }
    }

    return Ok(());
}

fn load_save(filename: &str) -> Option<(Game, DisplayState)> {
    if let Ok(bytes) = std::fs::read(filename) {
        let cur = Cursor::new(&bytes[..]);
        let mut de = Deserializer::new(cur);
        if let Ok((game_loaded, display_loaded)) = Deserialize::deserialize(&mut de) {
            return Some((game_loaded, display_loaded));
        } 
    }
    return None
}

fn create_map_config(opts: &GameOptions, config: &Config) -> MapLoadConfig {
    let map_config: MapLoadConfig;

    if let Some(procgen_map) = opts.procgen_map.clone() {
        map_config = MapLoadConfig::ProcGen(procgen_map);
    } else {
        if let Some(map_config_str) = &opts.map_config {
            let cli_map_config = map_config_str.parse::<MapLoadConfig>()
                                               .expect("Could not parse map config option!");
            map_config = cli_map_config;
        } else {
            map_config = config.map_load.clone();
        }
    }

    return map_config;
}

fn save_game_thread(game_receiver: Receiver<(Game, DisplayState)>) {
    loop {
        if let Ok(game) = game_receiver.recv() {
            let mut buf = Vec::new();
            game.serialize(&mut Serializer::new(&mut buf)).unwrap();
            let mut save_game_file = std::fs::File::create(GAME_SAVE_FILE).unwrap();
            save_game_file.write_all(&buf).unwrap();
        } else {
            break;
        }
    }
}

fn process_commands(io_recv: &Receiver<String>, game: &mut Game, log: &mut Log) -> bool {
    let mut any_updates = false;
    if let Ok(msg) = io_recv.recv_timeout(Duration::from_millis(0)) {
        if let Ok(cmd) = msg.parse::<GameCmd>() {
            let result = execute_game_command(&cmd, game);
            log.log_output(&result);
            any_updates = true;
        } else {
            log.log_output(&format!("error '{}' unexpected", msg));
        }
    }

    return any_updates;
}

fn spawn_input_reader() -> Receiver<String> {
    let (io_send, io_recv) = mpsc::channel();

    thread::spawn(move || {
        let stdin = std::io::stdin();
        let stdin = stdin.lock().lines();

        for line in stdin {
            let text = line.unwrap();
            if !text.is_empty() {
                io_send.send(text).unwrap();
            }
        }
    });

    return io_recv;
}

