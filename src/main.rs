extern crate tcod;
extern crate rand;
extern crate serde;
#[macro_use]extern crate serde_derive;
extern crate serde_json;

mod types;
mod constants;
mod map;


use std::cmp;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;

use rand::Rng;

use serde::{Serialize, Deserialize};

use tcod::map::{Map as FovMap};
use tcod::console::*;
use tcod::colors::*;
use tcod::input::Key;
use tcod::input::KeyCode::*;
use tcod::input::{self, Event, Mouse};

use types::*;
use constants::*;
use map::*;


#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ColorConfig {
    r: u8,
    g: u8,
    b: u8,
}

impl ColorConfig {
    pub fn color(&self) -> Color {
        Color::new(self.r, self.g, self.b)
    }
}


#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Config {
    color_dark_wall: ColorConfig,
    color_light_wall: ColorConfig,
    color_dark_ground: ColorConfig,
    color_light_ground: ColorConfig,
    color_dark_water: ColorConfig,
    color_light_water: ColorConfig,
}



pub fn player_death(player: &mut Object) {
    player.char = '%';
    player.color = DARK_RED;
}

pub fn monster_death(monster: &mut Object) {
    monster.char = '%';
    monster.color = DARK_RED;
    monster.blocks = false;
    monster.fighter = None;
    monster.ai = None;
    monster.name = format!("remains of {}", monster.name);
}

pub fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut [Object]) {
    let (x, y) = objects[id].pos();

    if !map.is_blocked(x + dx, y + dy, objects){
        objects[id].set_pos(x + dx, y + dy);
    }
}

pub fn move_towards(id: usize, target_x: i32, target_y: i32, map: &Map, objects: &mut [Object]) {
    let dx = target_x - objects[id].x;
    let dy = target_y - objects[id].y;
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();

    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;
    move_by(id, dx, dy, map, objects);
}

fn ai_take_turn(monster_id: usize, map: &Map, objects: &mut [Object], fov_map: &FovMap, messages: &mut Messages) {
    let (monster_x, monster_y) = objects[monster_id].pos();

    if fov_map.is_in_fov(monster_x, monster_y) {
        if objects[monster_id].distance_to(&objects[PLAYER]) >= 2.0 {
            let (player_x, player_y) = objects[PLAYER].pos();
            move_towards(monster_id, player_x, player_y, map, objects);
        } else if objects[PLAYER].fighter.map_or(false, |f| f.hp > 0) {
            let (monster, player) = mut_two(monster_id, PLAYER, objects);
            monster.attack(player, messages);
        }
    }
}

fn mut_two<T>(first_index: usize, second_index: usize, items: &mut [T]) -> (&mut T, &mut T) {
    assert!(first_index != second_index);

    let split_at_index = cmp::max(first_index, second_index);
    let (first_slice, second_slice) = items.split_at_mut(split_at_index);
    if first_index < second_index {
        (&mut first_slice[first_index], &mut second_slice[0])
    } else {
        (&mut second_slice[0], &mut first_slice[second_index])
    }
}

impl DeathCallback {
    fn callback(self, object: &mut Object) {
        use DeathCallback::*;
        let callback: fn(&mut Object) = match self {
            Player => player_death,
            Monster => monster_death,
        };
        callback(object);
    }
}


fn handle_keys(game: &mut Game,
               key: Key,
               map: &mut Map,
               objects: &mut Vec<Object>,
               inventory: &mut Vec<Object>,
               messages: &mut Messages) -> PlayerAction {
    use PlayerAction::*;

    let player_alive = objects[PLAYER].alive;

    match (key, player_alive) {
        (Key { code: Up,      .. }, true)  |
        (Key { code: Number8, .. }, true)  |
        (Key { code: NumPad8, .. }, true) => {
            player_move_or_attack(0, -1, map, objects, messages);
            TookTurn
        }

        (Key { code: Down,    .. }, true) |
        (Key { code: Number2, .. }, true) |
        (Key { code: NumPad2, .. }, true) => {
            player_move_or_attack(0, 1, map, objects, messages);
            TookTurn
        }

        (Key { code: Left,    .. }, true) |
        (Key { code: Number4, .. }, true) |
        (Key { code: NumPad4, .. }, true) => {
            player_move_or_attack(-1, 0, map, objects, messages);
            TookTurn
        }

        (Key { code: Right,   .. }, true) |
        (Key { code: Number6, .. }, true) |
        (Key { code: NumPad6, .. }, true) => {
            player_move_or_attack(1, 0, map, objects, messages);
            TookTurn
        }

        (Key { code: Number9, .. }, true)  |
        (Key { code: NumPad9, .. }, true) => {
            player_move_or_attack(1, -1, map, objects, messages);
            TookTurn
        }

        (Key { code: Number3, .. }, true) |
        (Key { code: NumPad3, .. }, true) => {
            player_move_or_attack(1, 1, map, objects, messages);
            TookTurn
        }

        (Key { code: Number1, .. }, true) |
        (Key { code: NumPad1, .. }, true) => {
            player_move_or_attack(-1, 1, map, objects, messages);
            TookTurn
        }

        (Key { code: Number7, .. }, true) |
        (Key { code: NumPad7, .. }, true) => {
            player_move_or_attack(-1, -1, map, objects, messages);
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

fn cast_heal(_inventory_id: usize, objects: &mut [Object], messages: &mut Messages) -> UseResult {
    if let Some(fighter) = objects[PLAYER].fighter {
        if fighter.hp == fighter.max_hp {
            messages.message("You are alrady at full health.", RED);
            return UseResult::Cancelled;
        }
        messages.message("Your wounds start to feel better!", LIGHT_VIOLET);
        objects[PLAYER].heal(HEAL_AMOUNT);
        return UseResult::UsedUp;
    }
    UseResult::Cancelled
}

fn menu<T: AsRef<str>>(header: &str, options: &[T], width: i32, root: &mut Root) -> Option<usize> {
    assert!(options.len() <= 26, "Cannot have a menu with more than 26 options");

    let header_height = root.get_height_rect(0, 0, width, SCREEN_HEIGHT, header);
    let height = options.len() as i32 + header_height;

    let mut window = Offscreen::new(width, height);

    window.set_default_foreground(WHITE);
    window.print_rect_ex(0, 0, width, height, BackgroundFlag::None, TextAlignment::Left, header);

    for (index, option_text) in options.iter().enumerate() {
        let menu_letter = (b'a' + index as u8) as char;
        let text = format!("({}) {}", menu_letter, option_text.as_ref());
        window.print_ex(0, header_height + index as i32,
                        BackgroundFlag::None, TextAlignment::Left, text);
    }

    let x = SCREEN_WIDTH / 2 - width / 2;
    let y = SCREEN_HEIGHT / 2 - height / 2;
    tcod::console::blit(&mut window, (0, 0), (width, height), root, (x, y), 1.0, 0.7);

    root.flush();
    let key = root.wait_for_keypress(true);

    if key.printable.is_alphabetic() {
        let index = key.printable.to_ascii_lowercase() as usize - 'a' as usize;
        if index < options.len() {
            Some(index)
        } else {
            None
        }
    } else {
        None
    }
}

fn inventory_menu(inventory: &[Object], header: &str, root: &mut Root) -> Option<usize> {
    let options = if inventory.len() == 0 {
        vec!["Inventory is empty.".into()]
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
        };
        match on_use(inventory_id, objects, messages) {
            UseResult::UsedUp => {
                inventory.remove(inventory_id);
            }
            UseResult::Cancelled => {
                messages.message("Cancelled", WHITE);
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
            move_by(PLAYER, dx, dy, map, objects);
        }
    }
}

fn render_all(game: &mut Game,
              objects: &[Object],
              map: &mut Map,
              messages: &mut Messages,
              fov_recompute: bool,
              config: &Config) {
    if fov_recompute {
        let player = &objects[PLAYER];
        game.fov.compute_fov(player.x, player.y, TORCH_RADIOUS, FOV_LIGHT_WALLS, FOV_ALGO);
    }

    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let visible = game.fov.is_in_fov(x, y);
            let wall = map.0[x as usize][y as usize].block_sight;
            let color = match (map.0[x as usize][y as usize].tile_type, visible) {
                (TileType::Wall, false) => config.color_dark_wall,
                (TileType::Empty, false) => config.color_dark_ground,
                (TileType::Water, false) => config.color_dark_water,
                (TileType::Wall, true) => config.color_light_wall,
                (TileType::Empty, true) => config.color_light_ground,
                (TileType::Water, true) => config.color_light_water,
            };

            let mut explored = map.0[x as usize][y as usize].explored;
            if visible {
                explored = true;
            }

            if explored {
                game.console.set_char_background(x, y, color.color(), BackgroundFlag::Set);
            }
            map.0[x as usize][y as usize].explored = explored;
        }
    }

    let mut to_draw: Vec<_> = objects.iter().filter(|o| game.fov.is_in_fov(o.x, o.y)).collect();
    to_draw.sort_by(|o1, o2| { o1.blocks.cmp(&o2.blocks) });

    for object in &to_draw {
        object.draw(&mut game.console);
    }

    game.panel.set_default_background(BLACK);
    game.panel.clear();

    let hp = objects[PLAYER].fighter.map_or(0, |f| f.hp);
    let max_hp = objects[PLAYER].fighter.map_or(0, |f| f.max_hp);
    render_bar(game, 1, 1, BAR_WIDTH, "HP", hp, max_hp, LIGHT_RED, DARK_RED);

    let mut y = MSG_HEIGHT as i32;
    for &(ref msg, color) in messages.0.iter().rev() {
        let msg_height = game.panel.get_height_rect(MSG_X, y, MSG_WIDTH, 0, msg);
        y -= msg_height;
        if y < 0 {
            break;
        }
        game.panel.set_default_foreground(color);
        game.panel.print_rect(MSG_X, y, MSG_WIDTH, 0, msg);
    }

    game.panel.set_default_foreground(LIGHT_GREY);
    game.panel.print_ex(1, 0, BackgroundFlag::None, TextAlignment::Left,
                   get_names_under_mouse(game.mouse, objects, &mut game.fov));

    blit(&mut game.console, (0, 0), (SCREEN_WIDTH, SCREEN_HEIGHT), &mut game.root, (0, 0), 1.0, 1.0);

    blit(&mut game.panel, (0, 0), (SCREEN_WIDTH, SCREEN_HEIGHT), &mut game.root, (0, PANEL_Y), 1.0, 1.0);
}

fn get_names_under_mouse(mouse: Mouse, objects: &[Object], fov_map: &FovMap) -> String {
    let (x, y) = (mouse.cx as i32, mouse.cy as i32);

    let names = objects.iter()
                       .filter(|obj| { obj.pos() == (x, y) && fov_map.is_in_fov(obj.x, obj.y)})
                       .map(|obj| obj.name.clone())
                       .collect::<Vec<_>>();

    names.join(", ")
}

fn render_bar(game: &mut Game,
              x: i32,
              y: i32,
              total_width: i32,
              name: &str,
              value: i32,
              maximum: i32,
              bar_color: Color,
              back_color: Color) {
    let bar_width = (value as f32 / maximum as f32 * total_width as f32) as i32;

    game.panel.set_default_background(back_color);
    game.panel.rect(x, y, total_width, 1, false, BackgroundFlag::Screen);

    game.panel.set_default_background(bar_color);
    if bar_width > 0 {
        game.panel.rect(x, y, bar_width, 1, false, BackgroundFlag::Screen);
    }

    game.panel.set_default_foreground(WHITE);
    game.panel.print_ex(x + total_width / 2,
                   y,
                   BackgroundFlag::None,
                   TextAlignment::Center,
                   &format!("{}: {}/{}", name, value, maximum));
}

fn main() {
    let mut previous_player_position = (-1, -1);


    let mut messages = Messages::new();

    let mut inventory = vec![];

    let mut config: Config;
    {
        let mut file = File::open("config.json").unwrap();
        let mut config_string = String::new();
        file.read_to_string(&mut config_string).unwrap();
        config = serde_json::from_str(&config_string).unwrap();
    }

    let mut player = Object::new(0, 0, '@', "player", WHITE, true);
    player.alive = true;
    player.fighter = Some(Fighter{max_hp: 50, hp: 50, defense: 2, power: 5, on_death: DeathCallback::Player });

    let mut objects = vec!(player);

    let (mut map, position) = make_map(&mut objects);
    let player_x = position.0;
    let player_y = position.1;
    objects[PLAYER].x = player_x;
    objects[PLAYER].y = player_y;

    let root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("Rogue-like")
        .init();

    let mut game = Game::with_root(root);

    for object in &objects {
        object.draw(&mut game.root);
    }
    game.root.flush();
        
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            game.fov.set(x, y,
                         !map.0[x as usize][y as usize].block_sight,
                         !map.0[x as usize][y as usize].blocked);
        }
    }

    messages.message("Welcome Stranger! Prepare to perish in the Desolation of Salt!", RED);

    let mut key = Default::default();

    while !game.root.window_closed() {
        match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
            Some((_, Event::Mouse(m))) => game.mouse = m,
            Some((_, Event::Key(k))) => key = k,
            _ => key = Default::default(),
        }

        let fov_recompute = previous_player_position != (objects[PLAYER].x, objects[PLAYER].y);
        render_all(&mut game, 
                   &objects, &mut map, &mut messages,
                   fov_recompute,
                   &config);

        game.root.flush();

        for object in &objects {
            object.clear(&mut game.console);
        }

        previous_player_position = (objects[PLAYER].x, objects[PLAYER].y);
        let player_action = handle_keys(&mut game, key, &mut map, &mut objects, &mut inventory, &mut messages);
        if player_action == PlayerAction::Exit {
            break;
        }

        if objects[PLAYER].alive && player_action != PlayerAction::DidntTakeTurn {
            for id in 1..objects.len() {
                if objects[id].ai.is_some() {
                    ai_take_turn(id, &map, &mut objects, &game.fov, &mut messages);
                }
            }
        }

        // reload configuration
        match File::open("config.json") {
            Ok(mut file) => {
                let mut config_string = String::new();
                file.read_to_string(&mut config_string).unwrap();
                config = serde_json::from_str(&config_string).unwrap();
            }
          _ => (),
        }
    }
}

