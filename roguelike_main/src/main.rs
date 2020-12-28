#![allow(dead_code)]
mod throttler;
mod render;
mod console;
mod display;
mod keyboard;

use std::fs;
use std::io::{BufRead, Write};
use std::time::{Duration, Instant};
use std::path::Path;
//use std::collections::hash_map::DefaultHasher;
//use std::hash::{Hash, Hasher};
use std::str::FromStr;

use sdl2::image::LoadTexture;
use sdl2::render::{WindowCanvas, Texture, TextureCreator};
use sdl2::video::WindowContext;
use sdl2::ttf::Sdl2TtfContext;

use walkdir::WalkDir;

use log::LevelFilter;
use simple_logging;
use logging_timer::timer;

use gumdrop::Options;

use roguelike_core::types::*;
use roguelike_core::config::Config;
use roguelike_core::constants::*;
use roguelike_core::map::MapLoadConfig;

use roguelike_engine::game::*;
use roguelike_engine::generation::*;
use roguelike_engine::actions::*;
use roguelike_engine::make_map::{make_map, read_map_xp};
use roguelike_engine::input::*;

use crate::throttler::*;
use crate::render::*;
use crate::display::*;
use crate::keyboard::*;


const CONFIG_NAME: &str = "config.yaml";


#[derive(Debug, Clone, Options)]
pub struct GameOptions {
    #[options(help = "replay from an input log file")]
    pub replay: Option<String>,

    #[options(help = "log level to record in game.log (OFF, ERROR, WARN, INFO, DEBUG, TRACE)")]
    pub log_level: Option<String>,

    #[options(help = "use a given seed for random number generation")]
    pub seed: Option<u64>,

    #[options(help = "take a screenshot and exit", short="t")]
    pub screenshot: bool,

    #[options(help = "procgen map config", short="m")]
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
            // could add string input as a seed generator
            // let mut hasher = DefaultHasher::new();
            // args[1].hash(&mut hasher);
            // hasher.finish()
        };

    eprintln!("Seed: {} (0x{:X})", seed, seed);

    let log_level =
        opts.log_level.clone().map_or(LevelFilter::Trace,
                                      |level_str| LevelFilter::from_str(&level_str).expect("Log level unexpected!"));
    simple_logging::log_to_file("game.log", log_level).unwrap();

    run(seed, opts).unwrap();
}

pub fn run(seed: u64, opts: GameOptions) -> Result<(), String> {
    let config = Config::from_file(CONFIG_NAME);

    /* Create SDL Context */
    let sdl_context = sdl2::init()?;
    let video = sdl_context.video()?;
    let window = video.window("Rust Roguelike", SCREEN_WIDTH, SCREEN_HEIGHT)
        .position_centered().build().map_err(|e| e.to_string())?;

    let canvas = window.into_canvas()
        .accelerated().build().map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();

    /* Create Display Structures */
    let mut display = Display::new(canvas);

    /* Load Textures */
    load_sprites(&texture_creator, &mut display);

    load_sprite(&texture_creator, &mut display, "resources/rexpaint16x16.png", "tiles", 16);

    let ttf_context = sdl2::ttf::init().expect("Could not init SDL2 TTF!");
    let font_texture = load_font(&ttf_context,
                                 &texture_creator,
                                 &mut display.targets.canvas_panel.target,
                                 "Monoid.ttf".to_string(),
                                 16);
    display.add_spritesheet("font".to_string(), font_texture, 16);

    /* Create Game Structure */
    let mut game = Game::new(seed, config.clone())?;
    let stone_id = make_stone(&mut game.data.entities, &game.config, Pos::new(-1, -1), &mut game.msg_log);
    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    game.data.entities.inventory[&player_id].push_back(stone_id);

    game.load_vaults("resources/vaults/");

    make_mouse(&mut game.data.entities, &game.config, &mut game.msg_log);

    /* Load Procgen Data */
    if let Some(procgen_map) = opts.procgen_map.clone() {
        let map_config = MapLoadConfig::ProcGen(procgen_map);
        make_map(&map_config, &mut game);
    } else {
        make_map(&config.map_load, &mut game);
    }

    if opts.screenshot {
        take_screenshot(&mut game, &mut display).unwrap();
        return Ok(());
    }

    return game_loop(game, display, opts, sdl_context);
}

pub fn game_loop(mut game: Game, mut display: Display, opts: GameOptions, sdl_context: sdl2::Sdl) -> Result<(), String> {
    // read in the recorded action log, if one is provided
    let mut starting_actions = Vec::new();
    dbg!(&opts.replay);
    if let Some(replay_file) = opts.replay {
        let file =
            std::fs::File::open(&replay_file).expect(&format!("Could not open replay file '{}'", &replay_file));
        for line in std::io::BufReader::new(file).lines() {
            dbg!(&line);
            if let Ok(action) = InputAction::from_str(&line.unwrap()) {
                starting_actions.push(action);
            }
        }
    }
    dbg!(&starting_actions);

    let mut config_modified_time = fs::metadata(CONFIG_NAME).unwrap().modified().unwrap();

    // reverse the input log so we can pop actions off start-to-end
    starting_actions.reverse();

    /* Action Log */
    let mut action_log = std::fs::File::create("action_log.txt").unwrap();

    /* Setup FPS Throttling */
    let frame_ms = 1000 / game.config.frame_rate as u64;
    let fps_throttler = Throttler::new(Duration::from_millis(frame_ms));

    let mut event_pump = sdl_context.event_pump()?;

    let mut frame_time = Instant::now();

    let mut input = Input::new();

    /* Main Game Loop */
    while game.settings.running {
        let _loop_timer = timer!("GAME_LOOP");

        let mut input_action: InputAction = InputAction::None;
        /* Handle Events */
        {
            let _input_timer = timer!("INPUT");
            for sdl2_event in event_pump.poll_iter() {
                if let Some(event) = translate_event(sdl2_event, &mut game, &mut display) {
                    let action = input.handle_event(&mut game, event);
                    // TODO may lose inputs if multiple events create actions!
                    input_action = action;
                }
            }
        }

        // if there are starting actions to read, pop one off to play
        if let Some(action) = starting_actions.pop() {
            input_action = action;
        }

       if input_action == InputAction::Exit {
            game.settings.running = false;
       }

        /* Record Inputs to Log File */
        if input_action != InputAction::None &&
           input_action != InputAction::Exit {
            action_log.write(input_action.to_string().as_bytes()).unwrap();
            action_log.write("\n".as_bytes()).unwrap();
        }

        /* Step the Game Forward */
        {
            let _logic_timer = timer!("LOGIC");
            let dt = Instant::now().duration_since(frame_time);
            game.step_game(input_action, dt.as_secs_f32());
            frame_time = Instant::now();
        }

        if game.settings.state == GameState::Win {
            display.clear_level_state();
        } else if game.settings.state == GameState::Exit || game.settings.exiting {
            // TODO settings.exiting is overtaken by GameState::Exit
            game.settings.running = false;
        }

        /* Update Display */
        {
            let _display_timer = timer!("DISPLAY");
            for msg in game.msg_log.turn_messages.iter() {
                display.process_message(*msg, &mut game.data, &game.config);
            }

            /* Draw the Game to the Screen */
            render_all(&mut display, &mut game)?;

            display.update_display();
        }

        game.msg_log.clear();

        /* Reload map if configured to do so */
        let config_timer = timer!("CONFIG");
        if game.config.load_map_file_every_frame && Path::new("resources/map.xp").exists() {
            let player = game.data.find_by_name(EntityName::Player).unwrap();

            let map_file = format!("resources/{}", game.config.map_file);
            game.data.entities.clear();
            let player_pos = read_map_xp(&game.config, &mut game.data, &mut game.msg_log, &map_file);
            game.data.entities.set_pos(player, Pos::from(player_pos));
        }

        /* Reload Configuration */
        if let Ok(current_config_modified_time) = fs::metadata(CONFIG_NAME) {
            let current_config_modified_time = current_config_modified_time.modified().unwrap();
            if current_config_modified_time != config_modified_time {
                config_modified_time = current_config_modified_time;
                game.config = Config::from_file(CONFIG_NAME);
            }
        }
        drop(config_timer);

        /* Wait until the next tick to loop */
        let wait_timer = timer!("WAIT");
        fps_throttler.wait();
        drop(wait_timer);
    }

    return Ok(());
}

pub fn take_screenshot(game: &mut Game, display: &mut Display) -> Result<(), String> {
    game.settings.god_mode = true;

    game.step_game(InputAction::None, 0.0);
    render_all(display, game)?;

    display.save_screenshot("screenshot");

    return Ok(());
}

fn load_font(ttf_context: &Sdl2TtfContext,
             texture_creator: &TextureCreator<WindowContext>,
             canvas: &mut WindowCanvas,
             file_name: String,
             font_size: u16) -> Texture {
    let font = ttf_context.load_font(format!("resources/{}", file_name), font_size).expect("Could not load font file!");

    let pixel_format = texture_creator.default_pixel_format();

    // assumes monospace font- otherwise none of this works
    let (char_width, char_height) = font.size_of_char('a').unwrap();

    let mut font_texture =
        texture_creator.create_texture_target(pixel_format,
                                              char_width as u32 * FONT_WIDTH as u32,
                                              char_height as u32 * FONT_HEIGHT as u32).unwrap();

    canvas.with_texture_canvas(&mut font_texture, |canvas| {
        canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 0));
        canvas.clear();
        for chr_ix in 1..=255u8 {
            let chr_surface = font.render_latin1(&[chr_ix]).solid(sdl2::pixels::Color::WHITE).unwrap();
            let char_texture = chr_surface.as_texture(&texture_creator).unwrap();

            let char_rect = sdl2::rect::Rect::new(chr_ix as i32 % 16, chr_ix as i32 / 16, char_width, char_height);

            canvas.copy(&char_texture, None, char_rect).unwrap();
        }
    }).unwrap();

    return font_texture;
}

fn load_sprites(texture_creator: &TextureCreator<WindowContext>, display: &mut Display) {
    load_sprite(texture_creator, display, "animations/player/Player_Idle.png", "player_idle", 1);
    load_sprite(texture_creator, display, "animations/player/player_attack.png", "player_attack", 1);
    load_sprite(texture_creator, display, "animations/player/player_attack_Hammer.png", "player_attack_hammer", 1);
    load_sprite(texture_creator, display, "animations/player/player_attack_Dagger.png", "player_attack_dagger", 1);
    load_sprite(texture_creator, display, "animations/player/Player_Idle_Dagger.png", "player_idle_dagger", 1);
    load_sprite(texture_creator, display, "animations/player/Player_Idle_Hammer.png", "player_idle_hammer", 1);
    load_sprite(texture_creator, display, "animations/player/Player_Idle_Shield.png", "player_idle_shield", 1);
    load_sprite(texture_creator, display, "animations/player/player_vault.png", "player_vault", 1);
    load_sprite(texture_creator, display, "animations/player/player_wallkick.png", "player_wall_kick", 1);
    load_sprite(texture_creator, display, "animations/monster1/Gol_Idle.png", "gol_idle", 1);
    load_sprite(texture_creator, display, "animations/monster1/Gol_Die.png", "gol_die", 1);
    load_sprite(texture_creator, display, "animations/monster3/Pawn_Idle.png", "elf_idle", 1);
    load_sprite(texture_creator, display, "animations/traps/DamageTrap.png", "spikes", 1);
    load_sprite(texture_creator, display, "resources/rexpaint16x16.png", "font", 16);
    load_sprite(texture_creator, display, "animations/traps/McMuffin.png", "key", 1);

    for entry in WalkDir::new("animations/autoload/") {
        let entry = entry.unwrap();
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_file() && file_name.ends_with("png") {
                load_sprite(texture_creator, display, path.to_str().unwrap(), &file_name, 1);
            }
        }
    }
}

fn load_sprite(texture_creator: &TextureCreator<WindowContext>,
               display: &mut Display,
               path: &str,
               sprite_name: &str,
               rows: usize) {
    let texture = texture_creator.load_texture(path).expect("Could not load texture!");
    display.add_spritesheet(sprite_name.to_string(), texture, rows);
}

