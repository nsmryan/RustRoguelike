#![allow(dead_code)]

extern crate tcod;
extern crate rand;
extern crate serde;
#[macro_use]extern crate serde_derive;
extern crate serde_json;
extern crate num;
extern crate timer;
extern crate chrono;
extern crate rodio;

mod types;
mod constants;
mod display;
mod map;
mod ai;


#[allow(unused_imports)]use std::cmp;
#[allow(unused_imports)]use std::fs::File;
#[allow(unused_imports)]use std::io::BufReader;
#[allow(unused_imports)]use std::io::Read;
#[allow(unused_imports)]use std::sync::mpsc::channel;

#[allow(unused_imports)]use tcod::map::{Map as FovMap};
#[allow(unused_imports)]use tcod::console::*;
#[allow(unused_imports)]use tcod::colors::*;
#[allow(unused_imports)]use tcod::input::Key;
#[allow(unused_imports)]use tcod::input::KeyCode::*;
#[allow(unused_imports)]use tcod::input::{self, Event, Mouse};
#[allow(unused_imports)]use tcod::AsNative;
#[allow(unused_imports)]use tcod::image;
use tcod::line::*;

use timer::*;

use rodio::Source;

use num::clamp;

use types::*;
use constants::*;
use display::*;
use map::*;
use ai::*;
use std::print;

fn handle_input(game: &mut Game,
                key: Key,
                map: &mut Map,
                objects: &mut Vec<Object>,
                inventory: &mut Vec<Object>,
                messages: &mut Messages) -> PlayerAction {
    use PlayerAction::*;

    let player_alive = objects[PLAYER].alive;

    if game.mouse.lbutton_pressed {
        for index in 0..inventory.len() {
            let (mx, my) = (game.mouse.x, game.mouse.y);
            if inventory[index].item == Some(Item::Stone) {
                let mut item = inventory.swap_remove(index);
                let obj_id = objects.len();

                // TODO this enforces a square limit, not a radius, on throw distance
                let start_x = objects[PLAYER].x;
                let start_y = objects[PLAYER].y;
                let end_x = mx as i32 / FONT_WIDTH;
                let end_y = my as i32 / FONT_HEIGHT;
                let throw_dist =
                    Position::new(start_x, start_y).distance(&Position::new(end_x, end_y));
                let target_x = start_x + clamp(end_x - start_x, -PLAYER_THROW_DIST, PLAYER_THROW_DIST);
                let target_y = start_y + clamp(end_y - start_y, -PLAYER_THROW_DIST, PLAYER_THROW_DIST);

                item.x = start_x;
                item.y = start_y;
                objects.push(item);

                let animation =
                    Animation::Thrown(obj_id,
                                      Line::new((start_x, start_y),
                                                (target_x, target_y)));
                game.animations.push(animation);
                break;
            }
        }

        dbg!(());
        TookTurn
    } else {
        match (key, player_alive) {
            (Key { code: Up,      .. }, true)  |
            (Key { code: Number8, .. }, true)  |
            (Key { code: NumPad8, .. }, true) => {
                player_move_or_attack(0, -1, map, objects, messages);
        dbg!(());
                TookTurn
            }

            (Key { code: Down,    .. }, true) |
            (Key { code: Number2, .. }, true) |
            (Key { code: NumPad2, .. }, true) => {
                player_move_or_attack(0, 1, map, objects, messages);
        dbg!(());
                TookTurn
            }
            (Key { code: Left,    .. }, true) |
            (Key { code: Number4, .. }, true) |
            (Key { code: NumPad4, .. }, true) => {
                player_move_or_attack(-1, 0, map, objects, messages);
        dbg!(());
                TookTurn
            }

            (Key { code: Right,   .. }, true) |
            (Key { code: Number6, .. }, true) |
            (Key { code: NumPad6, .. }, true) => {
                player_move_or_attack(1, 0, map, objects, messages);
        dbg!(());
                TookTurn
            }

            (Key { code: Number9, .. }, true)  |
            (Key { code: NumPad9, .. }, true) => {
                player_move_or_attack(1, -1, map, objects, messages);
        dbg!(());
                TookTurn
            }

            (Key { code: Number3, .. }, true) |
            (Key { code: NumPad3, .. }, true) => {
                player_move_or_attack(1, 1, map, objects, messages);
        dbg!(());
                TookTurn
            }

            (Key { code: Number1, .. }, true) |
            (Key { code: NumPad1, .. }, true) => {
                player_move_or_attack(-1, 1, map, objects, messages);
        dbg!(());
                TookTurn
            }

            (Key { code: Number7, .. }, true) |
            (Key { code: NumPad7, .. }, true) => {
                player_move_or_attack(-1, -1, map, objects, messages);
        dbg!(());
                TookTurn
            }

            (Key { code: Number5, .. }, true) |
            (Key { code: NumPad5, .. }, true) => {
                objects[PLAYER].momentum = Some((0, 0));
                dbg!(());
                TookTurn
            }

            (Key { code: Enter, alt: true, .. }, _) => {
                let fullscreen = game.root.is_fullscreen();
                game.root.set_default_foreground(WHITE);
                game.root.set_fullscreen(!fullscreen);
                DidntTakeTurn
            },

            (Key {printable: 'g', .. }, true) => {
                let item_id = objects.iter().position(|object| {
                    object.pos() == objects[PLAYER].pos() && object.item.is_some()
                });
                if let Some(item_id) = item_id {
                    pick_item_up(item_id, objects, inventory, messages);
                }
                DidntTakeTurn
            }

            (Key {printable: 'i', .. }, true) => {
                let inventory_index =
                    inventory_menu(inventory,
                                   "Press the key next to an item to use it, or any other to cancel.\n",
                                   &mut game.root);
                if let Some(inventory_index) = inventory_index {
                    use_item(inventory_index, inventory, objects, messages);
                }
                DidntTakeTurn
            }

            (Key { code: Escape, .. }, _) => Exit,

            (Key {printable: 'v', .. }, true) => {
                for x in 0..MAP_WIDTH {
                    for y in 0..MAP_HEIGHT {
                        map.0[x as usize][y as usize].explored = true;
                    }
                }
                DidntTakeTurn
            }

            (_, _) => DidntTakeTurn,
        }
    }
}

fn gather_goal(_inventory_id: usize, _objects: &mut [Object], messages: &mut Messages) -> UseResult {
    messages.message("You've got the goal object! Nice work.", LIGHT_VIOLET);
    UseResult::Keep
}

fn cast_heal(_inventory_id: usize, objects: &mut [Object], messages: &mut Messages) -> UseResult {
    if let Some(fighter) = objects[PLAYER].fighter {
        if fighter.hp == fighter.max_hp {
            messages.message("You are already at full health.", RED);
            return UseResult::Cancelled;
        }
        messages.message("Your wounds start to feel better!", LIGHT_VIOLET);
        objects[PLAYER].heal(HEAL_AMOUNT);
        return UseResult::UsedUp;
    }

    UseResult::Cancelled
}

fn inventory_menu(inventory: &[Object], header: &str, root: &mut Root) -> Option<usize> {
    let options = if inventory.len() == 0 {
        vec!("Inventory is empty.".into())
    } else {
        inventory.iter().map(|item| { item.name.clone() }).collect()
    };

    let inventory_index = menu(header, &options, INVENTORY_WIDTH, root);

    if inventory.len() > 0 {
        inventory_index
    } else {
        None
    }
}

fn use_item(inventory_id: usize,
            inventory: &mut Vec<Object>,
            objects: &mut [Object],
            messages: &mut Messages) {
    use Item::*;

    if let Some(item) = inventory[inventory_id].item {
        let on_use = match item {
            Heal => cast_heal,
            Stone => unimplemented!(),
            Goal => gather_goal,
        };
        match on_use(inventory_id, objects, messages) {
            UseResult::UsedUp => {
                inventory.remove(inventory_id);
            }
            UseResult::Cancelled => {
                messages.message("Cancelled", WHITE);
            }

            UseResult::Keep => {
            }
        }
    } else {
        messages.message(format!("The {} cannot be used.", inventory[inventory_id].name), WHITE);
    }
}

fn pick_item_up(object_id: usize,
                objects: &mut Vec<Object>,
                inventory: &mut Vec<Object>,
                messages: &mut Messages) {
    if inventory.len() >= 26 {
        messages.message(format!("Your inventory is full, cannot pick up {}", objects[object_id].name), RED);
    } else {
        let item = objects.swap_remove(object_id);
        messages.message(format!("You picked up a {}!", item.name), GREEN);
        inventory.push(item);
    }
}

fn player_move_or_attack(dx: i32, dy: i32, map: &Map, objects: &mut [Object], messages: &mut Messages) {
    let x = objects[PLAYER].x + dx;
    let y = objects[PLAYER].y + dy;
    let target_id = objects.iter().position(|object| {
        object.fighter.is_some() && object.pos() == (x, y)
    });

    match target_id {
        Some(target_id) => {
            let (player, target) = mut_two(PLAYER, target_id, objects);
             player.attack(target, messages);
        }

        None => {
            move_player_by(objects, map, dx, dy);
        }
    }
}
//if we want to use a character sprite, a potential value is '\u{8B}'
pub fn make_player() -> Object {
    let mut player = Object::new(0, 0, '@', "player", WHITE, true);
    player.alive = true;
    player.fighter = Some(Fighter{max_hp: 50, hp: 50, defense: 2, power: 5, on_death: DeathCallback::Player });
    player.momentum = Some((0, 0));

    player
}

pub fn setup_fov(fov: &mut FovMap, map: &Map) {
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            fov.set(x,
                    y,
                    !map.0[x as usize][y as usize].block_sight,
                    !map.0[x as usize][y as usize].blocked);
        }
    }
}

fn step_animation(objects: &mut [Object], map: &Map, animation: &mut Animation) -> bool {
    match animation {
        Animation::Thrown(obj_id, line) => {
            match line.step() {
                Some(next) => {
                    if map.0[next.0 as usize][next.1 as usize].block_sight {
                        true
                    } else {
                        objects[*obj_id].x = next.0;
                        objects[*obj_id].y = next.1;
                        false
                    }
                },

                None => {
                    true
                },
            }
        }
    }
}

/// Check whether the exit condition for the game is met.
fn exit_condition_met(inventory: &[Object], map: &Map, objects: &[Object]) -> bool {
    // loop over objects in inventory, and check whether any
    // are the goal object.
    let has_goal =
        inventory.iter().any(|obj| obj.item.map_or(false, |item| item == Item::Goal));

    let player_pos = (objects[PLAYER].x, objects[PLAYER].y);
    let on_exit_tile = map[player_pos].tile_type == TileType::Exit;

    let exit_condition = has_goal && on_exit_tile;

    return exit_condition;
}

/// Play a sound file.
/// This implementation is inefficient, but simple.
pub fn play_sound(file_name: &str) {
    let device = rodio::default_output_device().unwrap();
    let file = File::open(file_name).unwrap();
    let source = rodio::Decoder::new(BufReader::new(file)).unwrap();
    rodio::play_raw(&device, source.convert_samples());
}

fn main() {
    let mut previous_player_position = (-1, -1);

    let mut messages = Messages::new();

    let mut inventory = vec![Object::make_stone(0, 0)];

    let mut config: Config;
    {
        let mut file = File::open("config.json").expect("Could not open/parse config file config.json");
        let mut config_string = String::new();
        file.read_to_string(&mut config_string).expect("Could not read contents of config.json");
        config = serde_json::from_str(&config_string).expect("Could not parse config.json file!");
    }

    let mut objects = vec!(make_player());

    let (mut map, position) = make_map(&mut objects, &config);
    let player_x = position.0;
    let player_y = position.1;
    objects[PLAYER].x = player_x;
    objects[PLAYER].y = player_y;

    let root = Root::initializer()
        .font("rexpaint16x16.png", FontLayout::AsciiInRow)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("Rogue-like")
        .init();

    let mut game = Game::with_root(root);

    setup_fov(&mut game.fov, &map);

    messages.message("Welcome Stranger! Prepare to perish in the Desolation of Salt!", ORANGE);

    let mut key = Default::default();

    // Start game tick timer
    let timer = Timer::new();
    let (tick_sender, tick_receiver) = channel();
    let _guard = 
        timer.schedule_repeating(chrono::Duration::milliseconds(TIME_BETWEEN_FRAMES_MS), move || {
            tick_sender.send(0).unwrap();
        });

    /* Main Game Loop */
    while !game.root.window_closed() {
        /* FPS Limiting */
        tick_receiver.recv().unwrap();

        match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
            Some((_, Event::Mouse(m))) => game.mouse = m,
            Some((_, Event::Key(k))) => key = k,
            _ => {
                key = Default::default();
                game.mouse = Default::default();
            },
        }

        /* Display */
        let fov_recompute = previous_player_position != (objects[PLAYER].x, objects[PLAYER].y);
        render_all(&mut game, 
                   &objects, &mut map, &mut messages,
                   fov_recompute,
                   &config);

        game.root.flush();

        for object in objects.iter() {
            object.clear(&mut game.console);
        }

        for clearable in game.needs_clear.iter() {
            game.console.put_char(clearable.0, clearable.1, ' ', BackgroundFlag::None);
        }
        game.needs_clear.clear();

        /* Player Action and Animations */
        // If there is an animation playing, let it finish
        let player_action;
        if game.animations.len() > 0 {
            let mut finished_ixs = Vec::new();
            let mut ix = 0; 
            for mut animation in game.animations.iter_mut() {
              let finished = step_animation(&mut objects, &map, &mut animation);
              if finished {
                  finished_ixs.push(ix)
              }
              ix += 1;
            }
            finished_ixs.sort_unstable();
            for ix in finished_ixs.iter().rev() {
                game.animations.swap_remove(*ix);
            }
            player_action = PlayerAction::DidntTakeTurn;
        } else {
            previous_player_position = (objects[PLAYER].x, objects[PLAYER].y);
            player_action = handle_input(&mut game, key, &mut map, &mut objects, &mut inventory, &mut messages);
            match player_action {
              PlayerAction::Exit => {
                break;
              }

              PlayerAction::TookTurn => {
                  game.turn_count += 1;
              }
              
              _ => {}
            }
        }

        // check exit condition
        if exit_condition_met(&inventory, &map, &objects) {
            std::process::exit(0);
        }

        /* AI */
        if objects[PLAYER].alive && player_action != PlayerAction::DidntTakeTurn {
            for id in 1..objects.len() {
                if objects[id].ai.is_some() {
                    ai_take_turn(id, &map, &mut objects, &game.fov, &mut messages, &mut game.animations);
                }
            }
        }

        // reload configuration
        match File::open("config.json") {
            Ok(mut file) => {
                let mut config_string = String::new();
                file.read_to_string(&mut config_string).expect("Could not read config file!");
                config = serde_json::from_str(&config_string).expect("Could not read JSON- config.json has a parsing error!");
            }
          _ => (),
        }
    }
}

