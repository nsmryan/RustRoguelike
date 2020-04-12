#![allow(dead_code)]

use std::env;
use std::fs::File;
use std::time::{Duration, Instant};
use std::io::Read;
use std::path::Path;

use sdl2::event::Event;
use sdl2::image::LoadTexture;
use sdl2::mouse::MouseButton;

use slotmap::dense::*;

use serde_yaml;

use walkdir::WalkDir;

use roguelike_core::types::*;
use roguelike_core::map::Surface;
use roguelike_core::config::Config;
use roguelike_core::messaging::Msg;
use roguelike_core::constants::*;
use roguelike_core::animation::{Effect, Animation};
use roguelike_core::movement::{MoveMode, MoveType};

use roguelike_engine::display::*;
use roguelike_engine::render::*;
use roguelike_engine::plat::*;
use roguelike_engine::game::*;
use roguelike_engine::input::*;
use roguelike_engine::throttler::*;
use roguelike_engine::read_map::read_map_xp;


fn main() {
    let args = env::args().collect::<Vec<String>>();

    let config: Config;
    {
        let mut file =
            File::open("config.yaml").expect("Could not open/parse config file config.yaml");
        let mut config_string = String::new();
        file.read_to_string(&mut config_string)
            .expect("Could not read contents of config.yaml");
        config = serde_yaml::from_str(&config_string).expect("Could not parse config.yaml file!");
    }

    run(&args, config).unwrap();
}

pub fn run(args: &Vec<String>, config: Config) -> Result<(), String> {
    /* Create SDL Context */
    let sdl_context = sdl2::init()?;
    let video = sdl_context.video()?;
    let window = video.window("Rust Roguelike", SCREEN_WIDTH, SCREEN_HEIGHT)
        .position_centered().build().map_err(|e| e.to_string())?;

    let canvas = window.into_canvas()
        .accelerated().build().map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();

    let mut event_pump = sdl_context.event_pump()?;

    /* Setup FPS Throttling */
    let fps_throttler = Throttler::new(Duration::from_millis(1000 / config.rate as u64));

    /* Load Textures */
    let font_image = texture_creator.load_texture("resources/rexpaint16x16.png")
        .map_err(|e| e.to_string())?;

    let player_idle = texture_creator.load_texture("animations/player/Player_Idle.png")
        .map_err(|e| e.to_string())?;

    let player_attack = texture_creator.load_texture("animations/player/player_attack.png")
        .map_err(|e| e.to_string())?;

    let player_vault = texture_creator.load_texture("animations/player/player_vault.png")
        .map_err(|e| e.to_string())?;

    let player_wall_kick = texture_creator.load_texture("animations/player/player_wallkick.png")
        .map_err(|e| e.to_string())?;

    let gol_idle = texture_creator.load_texture("animations/monster1/Gol_Idle.png")
        .map_err(|e| e.to_string())?;

    let gol_die = texture_creator.load_texture("animations/monster1/Gol_Die.png")
        .map_err(|e| e.to_string())?;

    let elf_idle = texture_creator.load_texture("animations/monster3/Elf_Idle.png")
        .map_err(|e| e.to_string())?;

    let spikes_anim = texture_creator.load_texture("animations/traps/DamageTrap.png")
        .map_err(|e| e.to_string())?;

    let font_as_sprite = texture_creator.load_texture("resources/rexpaint16x16.png")
        .map_err(|e| e.to_string())?;

    let mcmuffin = texture_creator.load_texture("animations/traps/McMuffin.png")
        .map_err(|e| e.to_string())?;


    let mut sprites = DenseSlotMap::new();
    sprites.insert(SpriteSheet::new("player_wall_kick".to_string(), player_wall_kick, 1));
    sprites.insert(SpriteSheet::new("player_idle".to_string(),      player_idle,      1));
    sprites.insert(SpriteSheet::new("player_attack".to_string(),    player_attack,    1));
    sprites.insert(SpriteSheet::new("player_vault".to_string(),     player_vault,     1));
    sprites.insert(SpriteSheet::new("gol_idle".to_string(),         gol_idle,         1));
    sprites.insert(SpriteSheet::new("gol_die".to_string(),          gol_die,          1));
    sprites.insert(SpriteSheet::new("elf_idle".to_string(),         elf_idle,         1));
    sprites.insert(SpriteSheet::new("spikes".to_string(),           spikes_anim,      1));
    sprites.insert(SpriteSheet::new("font".to_string(),             font_as_sprite,   16));
    sprites.insert(SpriteSheet::new("key".to_string(),              mcmuffin,         1));

    // load any animations in the autoload directory.
    for entry in WalkDir::new("animations/autoload/") {
        let entry = entry.unwrap();
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_file() && file_name.ends_with("png") {
                let sprite =
                    texture_creator.load_texture(path).map_err(|e| e.to_string())?;

                sprites.insert(SpriteSheet::new(file_name, sprite, 1));
            }
        }
    }

    /* Create Display Structures */
    let screen_sections =
        Plan::vert("screen", 0.80, Plan::zone("map"),
                   Plan::split_horiz(0.3, Plan::zone("inventory"),
                                          Plan::split_horiz(0.5, Plan::zone("player"),
                                                                 Plan::zone("info"))));


    let display_state =
        DisplayState::new(screen_sections, font_image, sprites, canvas);

    let mut game = Game::new(args, config.clone(), display_state)?;

    let start_time = Instant::now();
    let mut frame_time = Instant::now();

    /* Main Game Loop */
    let mut running = true;
    while running {
        let tick_start = Instant::now();

        /* Handle Events */
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..}=> {
                    running = false;
                }

                Event::KeyDown {keycode, keymod, ..} => {
                    if let Some(keycode) = keycode {
                        game.keycode = Some((KeyDirection::Down, keycode));
                        game.input_action =
                            keydown_to_action(keycode, keymod);
                    }
                }

                Event::KeyUp {keycode, keymod, ..} => {
                    if let Some(keycode) = keycode {
                        game.keycode = Some((KeyDirection::Up, keycode));
                        game.input_action =
                            keyup_to_action(keycode, keymod, game.settings.state);
                    }
                }

                Event::MouseMotion {x, y, ..} => {
                    game.mouse_state.x = x;
                    game.mouse_state.y = y;
                }

                Event::MouseButtonDown {mouse_btn, x, y, ..} => {
                    match mouse_btn {
                        MouseButton::Left => {
                            game.mouse_state.left_pressed = true;

                            // Find the region where the mouse click occurred.
                            // If the click is within the map, generate a map click event.
                            let in_map =
                                game.display_state.zones.iter()
                                                        .filter(|zone| zone.contains(x as usize, y as usize) &&
                                                                       zone.name == "map")
                                                        .next();

                            if let Some(map_zone) = in_map {
                                let map_loc = map_zone.within(x as usize, y as usize);
                                let map_cell = (((map_loc.0 as f32 / map_zone.width as f32) * (game.data.map.width() as f32)) as i32,
                                                ((map_loc.1 as f32 / map_zone.height as f32) * (game.data.map.height() as f32)) as i32);
                                game.input_action =
                                  InputAction::MapClick(Pos::new(map_loc.0 as i32, map_loc.1 as i32),
                                                        Pos::new(map_cell.0 as i32, map_cell.1 as i32));
                            }
                        }

                        MouseButton::Middle => {
                            game.mouse_state.middle_pressed = true;
                        }

                        MouseButton::Right => {
                            game.mouse_state.right_pressed = true;
                        }

                        _ => {
                        },
                    }
                }

                Event::MouseButtonUp {mouse_btn, ..} => {
                    match mouse_btn {
                        MouseButton::Left => {
                            game.mouse_state.left_pressed = false;
                        }

                        MouseButton::Middle => {
                            game.mouse_state.middle_pressed = false;
                        }

                        MouseButton::Right => {
                            game.mouse_state.right_pressed = false;
                        }

                        _ => {},
                    }
                }

                _ => {}
            }
        }

        /* Step the Game Forward */
        let dt = Instant::now().duration_since(frame_time);
        let game_result = game.step_game(dt.as_secs_f32());
        frame_time = Instant::now();

        if game_result == GameResult::Stop {
            running = false;
        }

        /* Handle Message Log */
        for msg in game.msg_log.messages.iter() {
            println!("msg: {}", msg.msg_line(&game.data));

            match msg {
                Msg::Crushed(object_id, pos, _obj_type) => {
                    game.data.map[*pos].surface = Surface::Rubble;

                    let sound_aoe = game.data.sound_at(*object_id, *pos, config.sound_radius_crushed);

                    let sound_effect = Effect::Sound(sound_aoe, 0.0);
                    game.display_state.play_effect(sound_effect);
                }

                Msg::ItemThrow(thrower, item_id, start, end) => {
                    // NOTE the radius here is the stone radius, regardless of item type

                    // lay down sound objects on all tiles which can hear the sound.
                    // these dissapate with a count_down
                    let sound_aoe = game.data.sound_at(*thrower, *end, SOUND_RADIUS_STONE);

                    let chr = game.data.objects[*item_id].chr;
                    let item_sprite =
                        game.display_state.font_sprite(chr)
                            .expect("Could not find item sprite!");

                    let move_anim = Animation::Between(item_sprite, *start, *end, 0.0, config.item_throw_speed);
                    let item_anim = Animation::PlayEffect(Effect::Sound(sound_aoe, 0.0));
                    let loop_anim = Animation::Loop(item_sprite);

                    let move_key = game.display_state.play_animation(move_anim);
                    let item_key = game.display_state.play_animation(item_anim);
                    let loop_key = game.display_state.play_animation(loop_anim);

                    game.data.objects[*item_id].animation.clear();
                    game.data.objects[*item_id].animation.push_back(move_key);
                    game.data.objects[*item_id].animation.push_back(item_key);
                    game.data.objects[*item_id].animation.push_back(loop_key);
                }

                Msg::Moved(object_id, movement, pos) => {
                    let player_handle = game.data.find_player().unwrap();
                    if *object_id == player_handle {

                        if matches!(movement.typ, MoveType::Pass) {
                            if game.data.objects[player_handle].move_mode.unwrap() ==
                               MoveMode::Run {
                                let player = &mut game.data.objects[player_handle];
                                player.move_mode = player.move_mode.map(|mode| mode.decrease());
                            }
                            // this is just to pattern match on movement
                        } else { // monster moved
                            let mut sound_radius;
                            match game.data.objects[player_handle].move_mode.unwrap() {
                                MoveMode::Sneak => sound_radius = SOUND_RADIUS_SNEAK,
                                MoveMode::Walk => sound_radius = SOUND_RADIUS_WALK,
                                MoveMode::Run => sound_radius = SOUND_RADIUS_RUN,
                            }

                            if game.data.map[*pos].surface == Surface::Rubble {
                                sound_radius += config.sound_rubble_radius;
                            } else if game.data.map[*pos].surface == Surface::Grass {
                                sound_radius -= config.sound_grass_radius;
                            }

                            let idle_sprite =
                                game.display_state.new_sprite("player_idle".to_string(), config.idle_speed)
                                                  .unwrap();
                            let idle_anim = Animation::Loop(idle_sprite);
                            let idle_key = game.display_state.play_animation(idle_anim);

                            game.data.objects[player_handle].animation.clear();
                            game.data.objects[player_handle].animation.push_back(idle_key);

                            // add sound for movement
                            let sound_aoe = game.data.sound_at(*object_id, *pos, sound_radius);

                            let sound_effect = Effect::Sound(sound_aoe, 0.0);
                            game.display_state.play_effect(sound_effect);
                        }
                    } else {
                        let sound_aoe = game.data.sound_at(*object_id, *pos, SOUND_RADIUS_MONSTER_MOVE);

                        let sound_effect = Effect::Sound(sound_aoe, 0.0);
                        game.display_state.play_effect(sound_effect);
                    }
                }

                Msg::Yell(pos) => {
                    // NOTE this assumes that only the player yells
                    let player_handle = game.data.find_player().unwrap();

                    let sound_aoe = game.data.sound_at(player_handle, *pos, config.player_yell_radius);

                    let sound_effect = Effect::Sound(sound_aoe, 0.0);
                    game.display_state.play_effect(sound_effect);
                }

                Msg::Killed(_attacker, attacked, _damage) => {
                    if game.data.objects[*attacked].typ != ObjType::Player {
                        game.data.objects[*attacked].animation.clear();

                        let sprite_name = format!("{}_die", game.data.objects[*attacked].name);
                        let maybe_sprite = game.display_state.new_sprite(sprite_name, 1.0);
                        if let Some(sprite) = maybe_sprite {
                            let anim = game.display_state.play_animation(Animation::Once(sprite));
                            game.data.objects[*attacked].animation.clear();
                            game.data.objects[*attacked].animation.push_front(anim);
                        }

                        let pos = game.data.objects[*attacked].pos();

                        game.data.map[pos].surface = Surface::Rubble;
                    }

                    game.data.objects[*attacked].needs_removal = true;
                }

                Msg::Attack(attacker, attacked, _damage) => {
                    if game.data.objects[*attacker].typ == ObjType::Player {
                        let attack_sprite =
                            game.display_state.new_sprite("player_attack".to_string(), config.player_attack_speed)
                                              .unwrap();
                        let attack_anim = Animation::Once(attack_sprite);
                        let attack_key = game.display_state.play_animation(attack_anim);

                        let idle_sprite =
                            game.display_state.new_sprite("player_idle".to_string(), config.idle_speed)
                                              .unwrap();
                        let idle_anim = Animation::Loop(idle_sprite);
                        let idle_key = game.display_state.play_animation(idle_anim);

                        game.data.objects[*attacker].animation.clear();
                        game.data.objects[*attacker].animation.push_back(attack_key);
                        game.data.objects[*attacker].animation.push_back(idle_key);
                    }

                    let pos = game.data.objects[*attacked].pos();
                    let sound_aoe = game.data.sound_at(*attacker, pos, config.sound_radius_attack);

                    let sound_effect = Effect::Sound(sound_aoe, 0.0);
                    game.display_state.play_effect(sound_effect);
                }

                Msg::JumpWall(jumper, start, end) => {
                    if game.data.objects[*jumper].typ == ObjType::Player {
                        let jump_sprite =
                            game.display_state.new_sprite("player_vault".to_string(), config.player_vault_sprite_speed)
                                              .unwrap();
                        let jump_anim = Animation::Between(jump_sprite, *start, *end, 0.0, config.player_vault_move_speed);
                        let jump_key = game.display_state.play_animation(jump_anim);

                        let idle_sprite =
                            game.display_state.new_sprite("player_idle".to_string(), config.idle_speed)
                                              .unwrap();
                        let idle_anim = Animation::Loop(idle_sprite);
                        let idle_key = game.display_state.play_animation(idle_anim);

                        game.data.objects[*jumper].animation.clear();
                        game.data.objects[*jumper].animation.push_back(jump_key);
                        game.data.objects[*jumper].animation.push_back(idle_key);
                    }
                }

                _ => {
                }
            }
        }
        game.msg_log.clear();

        /* Draw the Game to the Screen */
        render_all(&mut game)?;

        /* Reload map if configured to do so */
        if game.config.load_map_file_every_frame && Path::new("resources/map.xp").exists() {
            let player_handle = game.data.find_player().unwrap();

            let map_file = format!("resources/{}", game.config.map_file);
            let (new_objects, new_map, player_position) = read_map_xp(&game.config, &mut game.display_state, &map_file);
            game.data.map = new_map;
            game.data.objects[player_handle].inventory.clear();
            let mut player = game.data.objects[player_handle].clone();
            game.data.objects.clear();
            for key in new_objects.keys() {
                game.data.objects.insert(new_objects[key].clone());
            }
            player.set_pos(Pos::from(player_position));
            game.data.objects.insert(player);
        }

        /* Process Player Messages */
        let player_handle = game.data.find_player().unwrap();
        for message in game.data.objects[player_handle].messages.iter() {
            if let Message::Sound(obj_id, pos) = message {
                if *obj_id == player_handle {
                    panic!("Player sent themselves a message?")
                }

                let player_pos = game.data.objects[player_handle].pos();
                if !game.data.map.is_in_fov(player_pos, *pos, game.config.fov_radius_player) {
                    let heard = Effect::HeardSomething(*pos, game.settings.turn_count);
                    game.display_state.effects.push(heard);
                }
            }
        }
        game.data.objects[player_handle].messages.clear();

        /* Remove objects are awaiting removal */
        {
            let mut removals = Vec::new();
            for key in game.data.objects.keys() {
                if game.data.objects[key].needs_removal &&
                   game.data.objects[key].animation.len() == 0 {
                       removals.push(key);
                   }
            }

            for key in removals {
               game.data.objects.remove(key);
            }
        }

        /* Reload Configuration */
        if let Ok(mut file) = File::open("config.yaml") {
            let mut config_string = String::new();
            file.read_to_string(&mut config_string).expect("Could not read config file!");
            game.config = serde_yaml::from_str(&config_string).expect("Could not read JSON- config.json has a parsing error!");

            //let config_yaml = serde_yaml::to_string(&game.config).expect("didn't serialize");
            //println!("{}", config_yaml);
            //let mut file = File::create("config.yaml").unwrap();
            //file.write(&config_yaml.as_bytes());
        }

        /* Wait until the next tick to loop */
        fps_throttler.wait();
    }

    return Ok(());
}

