use tcod::line::*;

use noise::Perlin;
use noise::NoiseFn;

use sdl2::render::BlendMode;
use sdl2::rect::Rect;
use sdl2::pixels::{Color as Sdl2Color};

use roguelike_core::types::*;
use roguelike_core::map::*;
use roguelike_core::constants::*;
use roguelike_core::movement::*;
use roguelike_core::config::*;
use roguelike_core::animation::{Effect, Animation, AnimKey};
use roguelike_core::utils::{distance, move_towards, lerp_color};

use crate::plat::*;
use crate::display::*;
use crate::game::*;


pub fn get_objects_under_mouse(mouse_pos: Pos,
                               data: &mut GameData) -> Vec<ObjectId> {
    let mut object_ids = Vec::new();

    for key in data.objects.keys() {
        let pos = data.objects[key].pos();
        let is_mouse = data.objects[key].name == "mouse";

        if !is_mouse && mouse_pos == pos {
            if data.map.is_in_fov(pos, mouse_pos, FOV_RADIUS) {
                object_ids.push(key);
            }
        }
    }

    return object_ids;
}

pub fn empty_tile_color(config: &Config, pos: Pos, visible: bool) -> Color {
    let perlin = Perlin::new();

    let low_color;
    let high_color;
    if visible {
        low_color = config.color_tile_blue_light;
        high_color = config.color_tile_blue_dark;
    } else {
        low_color = config.color_tile_blue_dark;
        high_color = config.color_very_dark_blue;
    }
    let color =
        lerp_color(low_color,
                   high_color,
                   perlin.get([pos.x as f64 / config.tile_noise_scaler,
                               pos.y as f64 / config.tile_noise_scaler]) as f32);

   return color;
}

pub fn tile_color(config: &Config, _x: i32, _y: i32, tile: &Tile, visible: bool) -> Color {
    let color = match (tile.tile_type, visible) {
        (TileType::Wall, true) =>
            config.color_light_brown,
        (TileType::Wall, false) =>
            config.color_dark_brown,

        (TileType::Empty, true) =>
            config.color_light_brown,

        (TileType::Empty, false) =>
            config.color_dark_brown,

        (TileType::Water, true) =>
            config.color_blueish_grey,
        (TileType::Water, false) =>
            config.color_blueish_grey,

        (TileType::ShortWall, true) =>
            config.color_light_brown,
        (TileType::ShortWall, false) =>
            config.color_dark_brown,

        (TileType::Exit, true) =>
            config.color_orange,
        (TileType::Exit, false) =>
            config.color_red,
    };

    return color;
}

/// Draw an outline and title around an area of the screen
pub fn draw_placard(display_state: &mut DisplayState,
                    text: String,
                    area: &Area,
                    config: &Config) {
    let color = config.color_bone_white;
    
    // Draw a black background
    display_state.canvas.set_draw_color(Sdl2Color::RGBA(0, 0, 0, 255));
    display_state.canvas.fill_rect(Rect::new(area.x_offset + 5,
                                             area.y_offset + (area.font_height as i32 / 2),
                                             area.width as u32 - 10,
                                             area.height as u32 - 10)).unwrap();

    display_state.canvas.set_draw_color(Sdl2Color::RGBA(color.r, color.g, color.b, color.a));

    // Draw a thin line around the edges of the placard
    display_state.canvas.draw_rect(Rect::new(area.x_offset + 5,
                                             area.y_offset + (area.font_height as i32 / 2),
                                             area.width as u32 - 10,
                                             area.height as u32 - 10)).unwrap();

    // draw a rectangle around where the placard header text will be placed.
    let half_text = text.len() / 2;
    let text_offset = (area.width / 2) - (area.font_width * half_text);
    display_state.canvas.fill_rect(Rect::new(area.x_offset + text_offset as i32,
                                             area.y_offset,
                                             (text.len() * area.font_width) as u32,
                                             area.font_height as u32)).unwrap();

    // Draw header text
    let mid_char_offset = (area.width / area.font_width) / 2;
    let text_start = (mid_char_offset - half_text) as i32;

    let text_pos = Pos::new(text_start, 0);

    display_state.draw_text(text,
                           text_pos,
                           config.color_dark_blue,
                           area);
}

pub fn render_player(display_state: &mut DisplayState,
                     _mouse_xy: Option<Pos>,
                     game_data: &mut GameData,
                     area: &Area, 
                     config: &Config) {
    draw_placard(display_state,
                 "Player".to_string(),
                 area,
                 config);

    let player_handle = game_data.find_player().unwrap();

    let mut list = Vec::new();

    let color = config.color_warm_grey;
    let text_pos = Pos::new(1, 2);

    list.push(format!("position:"));

    list.push(format!(" ({}, {})", 
                      game_data.objects[player_handle].x,
                      game_data.objects[player_handle].y));

    list.push(format!(""));

    let move_mode = game_data.objects[player_handle].move_mode.unwrap();
    list.push(format!("{}", move_mode.to_string()));

    display_state.draw_text_list(list,
                                 text_pos,
                                 color,
                                 area);

}

pub fn render_info(display_state: &mut DisplayState,
                   mouse_xy: Option<Pos>,
                   data: &mut GameData,
                   area: &Area, 
                   config: &Config) {
    draw_placard(display_state,
                 "Info".to_string(),
                 area,
                 config);

    if let Some(mouse) = mouse_xy {
        let player_handle = data.find_player().unwrap();
        let player_pos = data.objects[player_handle].pos();

        let object_ids =
            get_objects_under_mouse(mouse, data);

        if let Some(obj_id) = object_ids.first() {
            let mut y_pos = 1;

            if let Some(fighter) = data.objects[*obj_id].fighter {
                y_pos += 1;

                let health_percent = fighter.hp as f32 / fighter.max_hp as f32;

                display_state.canvas.set_blend_mode(BlendMode::None);
                let color = Sdl2Color::RGBA(config.color_red.r, config.color_red.g, config.color_red.b, config.color_red.a);
                display_state.canvas.set_draw_color(color);
                let start = area.char_rect(1, y_pos);
                let width = area.width as u32  - 2 * start.width();
                let health_rect = Rect::new(start.x,
                                            start.y,
                                            (width as f32 * health_percent) as u32,
                                            start.height());
                display_state.canvas.fill_rect(health_rect).unwrap();

                let full_rect = Rect::new(start.x,
                                          start.y,
                                          width,
                                          start.height());
                let outline_color = Color::white();
                let color = Sdl2Color::RGBA(outline_color.r, outline_color.g, outline_color.b, config.color_red.a);
                display_state.canvas.set_draw_color(color);
                display_state.canvas.draw_rect(full_rect).unwrap();

                y_pos += 2;
            }

            let pos = data.objects[*obj_id].pos();

            if data.map.is_in_fov(player_pos, pos, FOV_RADIUS) {
                let mut text_list = Vec::new();

                let color = config.color_warm_grey;
                let text_pos = Pos::new(1, y_pos);
                text_list.push(format!("{}", data.objects[*obj_id].name));

                text_list.push(format!(""));

                if data.objects[*obj_id].fighter.map_or(false, |fighter| fighter.hp <= 0) {
                    text_list.push(format!("{}", "dead"));
                } else if let Some(behave) = data.objects[*obj_id].behavior {
                    text_list.push(format!("{}", behave.description()));
                }

                display_state.draw_text_list(text_list,
                                             text_pos,
                                             color,
                                             area);
            }
        }
    }
}

/// Render an inventory section within the given area
pub fn render_inventory(display_state: &mut DisplayState,
                        data: &mut GameData,
                        area: &Area, 
                        config: &Config) {

    // Render header
    draw_placard(display_state,
                 "Inventory".to_string(),
                 area,
                 config);

    let player_handle = data.find_player().unwrap();

    // Render each object's name in inventory
    let mut y_pos = 2;
    let mut item_index = 0;
    for obj_id in data.objects[player_handle].inventory.iter() {
        let obj = &data.objects[*obj_id];

        let color;
        match obj.item {
            Some(Item::Stone) => {
                color = config.color_light_grey;
            }

            Some(Item::Goal) => {
                color = config.color_red;
            }
            
            _ => {
                color = config.color_mint_green;
            }
        }

        // place prompt character
        display_state.draw_char(('0' as u8 + item_index) as char,
                               Pos::new(1, y_pos),
                               config.color_ice_blue,
                               area);

        // place object name
        let text_pos = Pos::new(2, y_pos);
        display_state.draw_text(format!(" {}", data.objects[*obj_id].name),
                               text_pos,
                               color,
                               area);
        
        y_pos += 1;

        item_index += 1;
    }

    if data.objects[player_handle].inventory.len() == 0 {
        let text_pos = Pos::new(1, y_pos);
        display_state.draw_text(format!("empty"),
                               text_pos,
                               config.color_ice_blue,
                               area);
    }
}

/// render the background files, including water tiles
pub fn render_background(display_state: &mut DisplayState,
                         data: &mut GameData,
                         area: &Area,
                         settings: &GameSettings,
                         config: &Config) {
    let player_handle = data.find_player().unwrap();
    let pos = data.objects[player_handle].pos();

    for y in 0..data.map.height() {
        for x in 0..data.map.width() {
            let map_pos = Pos::new(x, y);

            let visible =
                data.map.is_in_fov(pos, map_pos, FOV_RADIUS) ||
                settings.god_mode;

            display_state.draw_char(MAP_EMPTY_CHAR as char,
                                    map_pos,
                                    empty_tile_color(config, map_pos, visible),
                                    area);

            let tile = &data.map.tiles[x as usize][y as usize];
            if tile.tile_type == TileType::Water {
                let color = tile_color(config, x, y, tile, visible);
                let chr = tile.chr.map_or('+', |chr| chr);
                display_state.draw_char(chr, map_pos, color, area);
            }
        }
    }

}

/// Render the map, with environment and walls
pub fn render_map(display_state: &mut DisplayState,
                  data: &mut GameData,
                  area: &Area,
                  settings: &GameSettings,
                  config: &Config) {
    let map_width = data.map.width();
    let map_height = data.map.height();

    let player_handle = data.find_player().unwrap();
    let player_pos = data.objects[player_handle].pos();

    for y in 0..map_height {
        for x in 0..map_width {
            let pos = Pos::new(x, y);

            // Render game stuff
            let visible =
                data.map.is_in_fov(player_pos, pos, FOV_RADIUS) ||
                settings.god_mode;

            let tile = &data.map.tiles[x as usize][y as usize];

            let explored = data.map.tiles[x as usize][y as usize].explored || visible;

            let wall_color;
            if explored {
                wall_color = config.color_light_brown;
            } else {
                wall_color = config.color_dark_brown;
            }

            let chr = tile.chr.map_or('+', |chr| chr);

            // draw empty tile first, in case there is transparency in the character
            // draw_char(display_state, MAP_EMPTY_CHAR as char, x, y, empty_tile_color(config, x, y, visible));

            // if the tile is not empty or water, draw it
            if chr != MAP_EMPTY_CHAR as char && tile.tile_type != TileType::Water {
                let color = tile_color(config, x, y, tile, visible);
                display_state.draw_char(chr, pos, color, area);
            }

            // finally, draw the between-tile walls appropriate to this tile
            if tile.bottom_wall == Wall::ShortWall {
                display_state.draw_char(MAP_THIN_WALL_BOTTOM as char, pos, wall_color, area);
            } else if tile.bottom_wall == Wall::TallWall {
                display_state.draw_char(MAP_THICK_WALL_BOTTOM as char, pos, wall_color, area);
            }

            if tile.left_wall == Wall::ShortWall {
                display_state.draw_char(MAP_THIN_WALL_LEFT as char, pos, wall_color, area);
            } else if tile.left_wall == Wall::TallWall {
                display_state.draw_char(MAP_THICK_WALL_LEFT as char, pos, wall_color, area);
            }

            if x + 1 < map_width {
                let right_tile = &data.map.tiles[x as usize + 1][y as usize];
                if right_tile.left_wall == Wall::ShortWall {
                    display_state.draw_char(MAP_THIN_WALL_RIGHT as char, pos, wall_color, area);
                } else if right_tile.left_wall == Wall::TallWall {
                    display_state.draw_char(MAP_THICK_WALL_RIGHT as char, pos, wall_color, area);
                }
            }

            if y - 1 >= 0 {
                let above_tile = &data.map.tiles[x as usize][y as usize - 1];
                if above_tile.bottom_wall == Wall::ShortWall {
                    display_state.draw_char(MAP_THIN_WALL_TOP as char, pos, wall_color, area);
                } else if above_tile.bottom_wall == Wall::TallWall {
                    display_state.draw_char(MAP_THICK_WALL_TOP as char, pos, wall_color, area);
                }
            }

            data.map.tiles[x as usize][y as usize].explored = explored;


            // Draw a square around this tile to help distinguish it visually in the grid
            let outline_color = Color::white();
            let color;
            if visible && data.map.tiles[x as usize][y as usize].tile_type != TileType::Water {
                color = Sdl2Color::RGBA(outline_color.r, outline_color.g, outline_color.b, config.grid_alpha_visible);
            } else {
                color = Sdl2Color::RGBA(outline_color.r, outline_color.g, outline_color.b, config.grid_alpha);
            }

            display_state.canvas.set_blend_mode(BlendMode::Blend);
            display_state.canvas.set_draw_color(color);
            display_state.canvas.draw_rect(area.char_rect(x, y)).unwrap();
        }
    }
}

/// Render each effect currently playing in the game
/// The strategy here is to copy the effects vector, update all items,
/// and then remove finished effects from back to front. The
/// resulting vector of effects is then saved as the new effects vector.
pub fn render_effects(display_state: &mut DisplayState,
                      game_data: &mut GameData,
                      _settings: &GameSettings,
                      config: &Config,
                      area: &Area) {
    let mut remove_indices = Vec::new();

    let mut effects = display_state.effects.clone();

    for (index, effect) in effects.iter_mut().enumerate() {
        match effect {
            Effect::Sound(pos, ref mut current_radius, max_radius) => {
                let mut highlight_color = config.color_warm_grey;
                highlight_color.a = config.sound_alpha;

                if *current_radius < *max_radius {
                    *current_radius += 1;
                    for sound_x in 0..game_data.map.width() {
                        for sound_y in 0..game_data.map.height() {
                            let sound_pos = Pos::new(sound_x, sound_y);

                            if distance(Pos::new(pos.x, pos.y), Pos::new(sound_x, sound_y)) == *current_radius as i32 {
                                display_state.draw_char(MAP_EMPTY_CHAR as char, sound_pos, highlight_color, area);
                            }
                        }
                    }
                } else {
                    remove_indices.push(index);
                }
            }
        }
    }

    remove_indices.sort();
    remove_indices.reverse();
    for index in remove_indices {
        effects.swap_remove(index);
    }

    display_state.effects = effects;
}

/// Render each object in the game, filtering for objects not currently visible
pub fn render_objects(display_state: &mut DisplayState,
                      data: &mut GameData,
                      settings: &GameSettings,
                      config: &Config,
                      area: &Area) {
    let player_handle = data.find_player().unwrap();
    let player_pos = data.objects[player_handle].pos();

    // step each objects animation
    for object_id in data.objects.keys().collect::<Vec<ObjectId>>().iter() {
        let pos = data.objects[*object_id].pos();

        // only draw if within the map (outside is (-1, -1) like if in inventory).
        if data.map.is_within_bounds(pos) {
            let is_in_fov = 
               data.map.is_in_fov(player_pos,
                                  pos,
                                  FOV_RADIUS);

            if let Some(anim_key) = data.objects[*object_id].animation.get(0) {
                let done = 
                    step_animation(*anim_key, *object_id, is_in_fov, display_state, data, settings, config, area);

                if done {
                    data.objects[*object_id].animation.pop_front();
                }
            } else if data.objects[*object_id].alive {
                let color = data.objects[*object_id].color;
                display_state.draw_char(data.objects[*object_id].chr, pos, color, area);
            }
        }
    }
}

pub fn step_animation(anim_key: AnimKey,
                      object_id: ObjectId,
                      is_in_fov: bool,
                      display_state: &mut DisplayState,
                      data: &mut GameData,
                      settings: &GameSettings,
                      config: &Config,
                      area: &Area) -> bool {

    let pos = data.objects[object_id].pos();
    let color = data.objects[object_id].color;

    match display_state.animations[anim_key].clone() {
        Animation::Between(ref mut sprite, start, end, ref mut dist, blocks_per_sec) => {
           dbg!();
           if settings.god_mode || is_in_fov {
               dbg!();
               *dist = *dist + (blocks_per_sec / config.rate as f32); 
               let num_blocks = *dist as usize;

               let draw_pos = move_towards(start, end, num_blocks);

               display_state.draw_sprite(sprite,
                                         draw_pos,
                                         color,
                                         &area);

               sprite.step();

               display_state.animations[anim_key] =
                   Animation::Between(*sprite, start, end, *dist, blocks_per_sec);

               return *dist >= distance(start, end) as f32;
           }
        }

        Animation::Loop(ref mut sprite) => {
           if settings.god_mode || is_in_fov {
                display_state.draw_sprite(sprite,
                                          pos,
                                          color,
                                          &area);

                sprite.step();

                display_state.animations[anim_key] =
                   Animation::Loop(*sprite);

                // a looping animation never finishes
                return false;
            }
        }

        Animation::PlayEffect(effect) => {
            display_state.play_effect(effect);

            // true indicates that the animation is finished
            return true;
        }

        Animation::Once(ref mut sprite) => {
           if settings.god_mode || is_in_fov {
                display_state.draw_sprite(sprite,
                                          pos,
                                          color,
                                          &area);

                let sprite_done = sprite.step();

                display_state.animations[anim_key] =
                   Animation::Once(*sprite);

                return sprite_done;
            }
        }
    }

    // assume animation is 'not finished' if it doesn't return anything
    return false;
}

pub fn render_overlays(display_state: &mut DisplayState,
                       map_mouse_pos: Option<Pos>,
                       data: &mut GameData,
                       settings: &GameSettings,
                       area: &Area,
                       config: &Config) {
    let player_handle = data.find_player().unwrap();
    let player_pos = data.objects[player_handle].pos();

    // Draw player action overlay. Could draw arrows to indicate how to reach each location
    let mut highlight_color = config.color_warm_grey;
    highlight_color.a = config.highlight_alpha;

    // Draw player movement overlay
    for move_action in Direction::move_actions().iter() {
        // for all movements except staying still
        if *move_action != Direction::Center {
            // calculate the move that would occur
            if let Some(movement) =
                calculate_move(*move_action,
                               data.objects[player_handle].movement.unwrap(),
                               player_handle,
                               data) {
                // draw a highlight on that square
                let xy = movement.xy();

                // don't draw overlay on top of character
                if xy != data.objects[player_handle].pos()
                {
                    display_state.draw_char(MAP_EMPTY_CHAR as char, xy, highlight_color, area);
                }
            }
        }
    }

    // draw attack position highlights
    if let Some(mouse_xy) = map_mouse_pos {
        let mut attack_highlight_color = config.color_red;
        attack_highlight_color.a = config.highlight_alpha;
        // Draw monster attack overlay
        let object_ids =  get_objects_under_mouse(mouse_xy, data);
        for object_id in object_ids.iter() {
            let pos = data.objects[*object_id].pos();

            if data.map.is_in_fov(player_pos, pos, FOV_RADIUS) &&
               data.objects[*object_id].alive {
                if let Some(reach) = data.objects[*object_id].attack {
                    let attack_positions = 
                        reach.offsets()
                             .iter()
                             .map(|offset| Pos::new(mouse_xy.x as i32 + offset.x,
                                                    mouse_xy.y as i32 + offset.y))
                             // filter out positions that are outside of the map, or with no clear
                             // path from the entity to the reached position
                             .filter(|pos| data.map.is_within_bounds(*pos) &&
                                           data.clear_path(mouse_xy, *pos))
                             .collect::<Vec<Pos>>();

                    for position in attack_positions {
                        display_state.draw_char(MAP_EMPTY_CHAR as char, position, attack_highlight_color, area);
                    }
                }
            }
        }
    }

    // draw mouse path overlays
    if let Some(mouse_handle) = data.find_mouse() {
        let mouse_pos = data.objects[mouse_handle].pos();
        let player_pos = data.objects[player_handle].pos();

        if config.draw_star_path {
            let path = data.map.astar(player_pos, mouse_pos);
            for pos in path {
                display_state.draw_char(MAP_EMPTY_CHAR as char, pos, highlight_color, area);
            }
        }

        if config.draw_mouse_line || settings.draw_throw_overlay {
            // mouse pos at 0, 0 occurs when the mouse has not moved since startup.
            // this may cause a weirdness on the corner of the map
            if mouse_pos != Pos::new(0, 0) {
                let line = Line::new(player_pos.to_tuple(), mouse_pos.to_tuple()).into_iter();
                for pos in line {
                    let pos = Pos::from(pos);
                    display_state.draw_char(MAP_EMPTY_CHAR as char, pos, highlight_color, area);
                }
            }
        }
    }
}

pub fn render_all(display_state: &mut DisplayState,
                  mouse_state: &mut MouseState,
                  data: &mut GameData,
                  settings: &GameSettings,
                  config: &Config)  -> Result<(), String> {

    let player_handle = data.find_player().unwrap();

    data.map.compute_fov(data.objects[player_handle].pos(), FOV_RADIUS);

    let screen_rect = display_state.canvas.output_size()?;

    let plots = display_state.screen_sections
                             .plot(0,
                                   0,
                                   screen_rect.0 as usize,
                                   screen_rect.1 as usize);

    display_state.canvas.set_draw_color(Sdl2Color::RGB(0, 0, 0));
    display_state.canvas.clear();

    let zones = plots.collect::<Vec<Plot>>();

    let mut mouse_map_pos = None;
    for zone in zones.iter() {
        if zone.name == "map" && zone.contains(mouse_state.x as usize, mouse_state.y as usize) {
            let ((_x_offset, _y_offset), scaler) =
                zone.fit(data.map.width() as usize * FONT_WIDTH as usize,
                         data.map.height() as usize * FONT_HEIGHT as usize);

            let mouse_map_xy = zone.within(mouse_state.x as usize, mouse_state.y as usize);
            let map_x = mouse_map_xy.0 as f32 / (FONT_WIDTH as f32 * scaler);
            let map_y = mouse_map_xy.1 as f32 / (FONT_HEIGHT as f32 * scaler);
            mouse_map_pos = Some(Pos::new(map_x as i32, map_y as i32));

            if let Some(mouse_handle) = data.find_mouse() {
                data.objects[mouse_handle].set_xy(map_x as i32, map_y as i32);
            }
        }
    }

    // for each screen section, render its contents
    for plot in zones.iter() {
        match plot.name.as_str() {
            "screen" => {
            }

            "map" => {
                let ((x_offset, y_offset), scaler) =
                    plot.fit(data.map.width() as usize * FONT_WIDTH as usize,
                             data.map.height() as usize * FONT_HEIGHT as usize);

                let area = Area::new(x_offset as i32,
                                     y_offset as i32,
                                     plot.width,
                                     plot.height,
                                     (scaler * FONT_WIDTH as f32) as usize, 
                                     (scaler * FONT_WIDTH as f32) as usize);


                render_background(display_state, data, &area, settings, config);

                render_map(display_state, data, &area, settings, config);

                render_objects(display_state, data, settings, config, &area);

                render_effects(display_state, data, settings, config, &area);

                render_overlays(display_state, mouse_map_pos, data, settings, &area, config);
            }

            "inventory" => {
                let area = Area::new(plot.x as i32,
                                     plot.y as i32,
                                     plot.width,
                                     plot.height,
                                     FONT_WIDTH as usize,
                                     FONT_HEIGHT as usize);
                render_inventory(display_state, data, &area, config);
            }

            "player" => {
                let area = Area::new(plot.x as i32,
                                     plot.y as i32,
                                     plot.width,
                                     plot.height,
                                     FONT_WIDTH as usize,
                                     FONT_HEIGHT as usize);
                render_player(display_state, mouse_map_pos, data, &area, config);
            }

            "info" => {
                let area = Area::new(plot.x as i32,
                                     plot.y as i32,
                                     plot.width,
                                     plot.height,
                                     FONT_WIDTH as usize,
                                     FONT_HEIGHT as usize);
                render_info(display_state, mouse_map_pos, data, &area, config);
            }

            section_name => {
                panic!(format!("Unexpected screen section '{}'", section_name));
            }
        }
    }

    if settings.state == GameState::Inventory {
        let width: usize = 300;
        let height: usize = 500;
        let area = Area::new((SCREEN_WIDTH as i32 / 2) - (width as i32 / 2),
                             (SCREEN_HEIGHT as i32 / 2) - (height as i32 / 2),
                             width,
                             height,
                             FONT_WIDTH as usize,
                             FONT_HEIGHT as usize);

        render_inventory(display_state, data, &area, config);
    }

    display_state.canvas.present();

    display_state.zones = zones;

    Ok(())
}

pub fn engine_color(color: &Color) -> Sdl2Color {
    return Sdl2Color::RGBA(color.r, color.g, color.b, color.a);
}

