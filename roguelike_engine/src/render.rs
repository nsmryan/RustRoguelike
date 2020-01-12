use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

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

use crate::plat::*;
use crate::display::*;
use crate::game::*;


pub fn get_objects_under_mouse(x: i32,
                               y: i32,
                               data: &mut GameData) -> Vec<ObjectId> {
    let mut object_ids = Vec::new();

    for key in data.objects.keys() {
        let pos = data.objects[key].pos();
        let is_mouse = data.objects[key].name == "mouse";
        if !is_mouse && pos == (x, y) {
            if data.map.is_in_fov(pos.0, pos.1, x, y, FOV_RADIUS) {
                object_ids.push(key);
            }
        }
    }

    return object_ids;
}

pub fn rand_from_x_y(x: i32, y: i32) -> f32 {
    let mut hasher = DefaultHasher::new();

    (x as u32).hash(&mut hasher);
    (y as u32).hash(&mut hasher);
 
    let result: u64 = hasher.finish();

    return ((result & 0xFFFFFFFF) as f32) / 4294967295.0;
}

pub fn lerp(first: f32, second: f32, scale: f32) -> f32 {
    return first + ((second - first) * scale);
}

pub fn lerp_color(color1: Color, color2: Color, scale: f32) -> Color {
    return Color {
        r: lerp(color1.r as f32, color2.r as f32, scale) as u8,
        g: lerp(color1.g as f32, color2.g as f32, scale) as u8,
        b: lerp(color1.b as f32, color2.b as f32, scale) as u8,
        a: lerp(color1.a as f32, color2.a as f32, scale) as u8,
    };
}

pub fn empty_tile_color(config: &Config, x: i32, y: i32, visible: bool) -> Color {
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
                   perlin.get([x as f64 / config.tile_noise_scaler,
                               y as f64 / config.tile_noise_scaler]) as f32);

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
    display_state.canvas.set_draw_color(Sdl2Color::RGBA(color.r, color.g, color.b, color.a));
    
    display_state.canvas.draw_rect(Rect::new(area.x_offset + 5,
                                             area.y_offset + (area.font_height as i32 / 2),
                                             area.width as u32 - 10,
                                             area.height as u32 - 10)).unwrap();

    let half_text = text.len() / 2;
    let text_offset = (area.width / 2) - (area.font_width * half_text);
    display_state.canvas.fill_rect(Rect::new(area.x_offset + text_offset as i32,
                                             area.y_offset,
                                             (text.len() * area.font_width) as u32,
                                             area.font_height as u32)).unwrap();

    let mid_char_offset = (area.width / area.font_width) / 2;
    let text_start = (mid_char_offset - half_text) as i32;
    draw_text(display_state,
              text,
              text_start,
              0,
              config.color_dark_blue,
              area);
}

pub fn render_player(display_state: &mut DisplayState,
                   _mouse_xy: Option<(usize, usize)>,
                   _data: &mut GameData,
                   area: &Area, 
                   config: &Config) {
    draw_placard(display_state,
                 "Player".to_string(),
                 area,
                 config);
}

pub fn render_info(display_state: &mut DisplayState,
                   mouse_xy: Option<(usize, usize)>,
                   data: &mut GameData,
                   area: &Area, 
                   config: &Config) {
    draw_placard(display_state,
                 "Info".to_string(),
                 area,
                 config);

    if let Some(mouse) = mouse_xy {
        let player_handle = data.find_player().unwrap();
        let player_x = data.objects[player_handle].x;
        let player_y = data.objects[player_handle].y;

        let object_ids =
            get_objects_under_mouse(mouse.0 as i32, mouse.1 as i32, data);

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

            if data.map.is_in_fov(player_x, player_y, pos.0, pos.1, FOV_RADIUS) {
                let color = config.color_warm_grey;
                draw_text(display_state,
                          format!(" {}", data.objects[*obj_id].name),
                          0,
                          y_pos,
                          color,
                          area);

                y_pos += 1;

                if let Some(behave) = data.objects[*obj_id].behavior {
                    draw_text(display_state,
                              format!(" {}", behave.description()),
                              1,
                              y_pos,
                              color,
                              area);

                    y_pos += 1;
                }

                if let Some(momentum) = data.objects[*obj_id].momentum {
                    draw_text(display_state,
                              format!("momentum:"),
                              1,
                              y_pos,
                              color,
                              area);

                    y_pos += 1;
                    draw_text(display_state,
                              format!(" {} ({}, {})", momentum.magnitude(), momentum.mx, momentum.my),
                              1,
                              y_pos,
                              color,
                              area);
                }
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
        draw_char(display_state,
                  '*',
                  1,
                  y_pos,
                  config.color_ice_blue,
                  area);

        // place object name
        draw_text(display_state,
                  format!(" {}", data.objects[*obj_id].name),
                  2,
                  y_pos,
                  color,
                  area);
        
        y_pos += 1;
    }

    if data.objects[player_handle].inventory.len() == 0 {
        draw_text(display_state,
                  format!("empty"),
                  1,
                  y_pos,
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
            let visible =
                data.map.is_in_fov(pos.0, pos.1, x, y, FOV_RADIUS) ||
                settings.god_mode;

            draw_char(display_state,
                      MAP_EMPTY_CHAR as char,
                      x,
                      y,
                      empty_tile_color(config, x, y, visible),
                      area);

            let tile = &data.map.tiles[x as usize][y as usize];
            if tile.tile_type == TileType::Water {
                let color = tile_color(config, x, y, tile, visible);
                let chr = tile.chr.map_or('+', |chr| chr);
                draw_char(display_state, chr, x, y, color, area);
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
            // Render game stuff
            let visible =
                data.map.is_in_fov(player_pos.0, player_pos.1, x, y, FOV_RADIUS) ||
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
                draw_char(display_state, chr, x, y, color, area);
            }

            // finally, draw the between-tile walls appropriate to this tile
            if tile.bottom_wall == Wall::ShortWall {
                draw_char(display_state, MAP_THIN_WALL_BOTTOM as char, x, y, wall_color, area);
            } else if tile.bottom_wall == Wall::TallWall {
                draw_char(display_state, MAP_THICK_WALL_BOTTOM as char, x, y, wall_color, area);
            }

            if tile.left_wall == Wall::ShortWall {
                draw_char(display_state, MAP_THIN_WALL_LEFT as char, x, y, wall_color, area);
            } else if tile.left_wall == Wall::TallWall {
                draw_char(display_state, MAP_THICK_WALL_LEFT as char, x, y, wall_color, area);
            }

            if x + 1 < map_width {
                let right_tile = &data.map.tiles[x as usize + 1][y as usize];
                if right_tile.left_wall == Wall::ShortWall {
                    draw_char(display_state, MAP_THIN_WALL_RIGHT as char, x, y, wall_color, area);
                } else if right_tile.left_wall == Wall::TallWall {
                    draw_char(display_state, MAP_THICK_WALL_RIGHT as char, x, y, wall_color, area);
                }
            }

            if y - 1 >= 0 {
                let above_tile = &data.map.tiles[x as usize][y as usize - 1];
                if above_tile.bottom_wall == Wall::ShortWall {
                    draw_char(display_state, MAP_THIN_WALL_TOP as char, x, y, wall_color, area);
                } else if above_tile.bottom_wall == Wall::TallWall {
                    draw_char(display_state, MAP_THICK_WALL_TOP as char, x, y, wall_color, area);
                }
            }

            data.map.tiles[x as usize][y as usize].explored = explored;


            // Draw a square around this tile to help distinguish it visually in the grid
            let outline_color = Color::white();
            let color = Sdl2Color::RGBA(outline_color.r, outline_color.g, outline_color.b, config.grid_alpha);

            display_state.canvas.set_blend_mode(BlendMode::Blend);
            display_state.canvas.set_draw_color(color);
            display_state.canvas.draw_rect(area.char_rect(x, y)).unwrap();
        }
    }
}

/// Render each object in the game, filtering for objects not currently visible
pub fn render_objects(display_state: &mut DisplayState,
                      data: &mut GameData,
                      settings: &GameSettings,
                      config: &Config,
                      area: &Area) {
    let player_handle = data.find_player().unwrap();
    let player_pos = data.objects[player_handle].pos();

    let mut new_objects = Vec::new();

    for object in data.objects.values_mut() {
        let x = object.x;
        let y = object.y;

        if data.map.is_within_bounds(x, y) {
            let is_in_fov = 
               data.map.is_in_fov(player_pos.0,
                                  player_pos.1,
                                  x,
                                  y,
                                  FOV_RADIUS);

           // TODO consider make FOV a setting in map, which is set by god_mode
            match object.animation {
                Some(Animation::StoneThrow(start, end)) => {
                    draw_char(display_state, object.chr, start.0, start.1, object.color, area);
                    if start == end {
                        object.animation = None;

                        let mut sound = Object::new(end.0, end.1, ' ' as char, Color::white(), "sound", false);
                        sound.animation = Some(Animation::Sound(0, STONE_SOUND_RADIUS));
                        new_objects.push(sound);

                    } else {
                        let new_start = (start.0 + direction(end.0 - start.0),
                                         start.1 + direction(end.1 - start.1));
                        object.animation = Some(Animation::StoneThrow(new_start, end));
                    }
                }

                Some(Animation::Idle(sprite_key, ref mut sprite_val)) => {
                   if settings.god_mode || is_in_fov {
                        let sprite_index = (*sprite_val) as i32;
                        draw_sprite(display_state,
                                    sprite_key,
                                    sprite_index,
                                    x,
                                    y,
                                    object.color,
                                    &area);
                        *sprite_val = *sprite_val + config.idle_speed;
                        if *sprite_val as usize >= display_state.sprites[sprite_key].num_sprites {
                            *sprite_val = 0.0;
                        }
                    }
                }

                Some(Animation::WallKick(sprite_key, ref mut sprite_val, _start, end)) => {
                    if settings.god_mode || is_in_fov {
                        let sprite_index = (*sprite_val) as i32;
                        draw_sprite(display_state,
                                    sprite_key,
                                    sprite_index,
                                    end.0,
                                    end.1,
                                    object.color,
                                    &area);
                        *sprite_val = *sprite_val + config.idle_speed;
                        if *sprite_val as usize >= display_state.sprites[sprite_key].num_sprites {
                            *sprite_val = 0.0;
                        }
                    }
                }

                Some(Animation::Sound(ref mut current_radius, max_radius)) => {
                    let mut highlight_color = config.color_warm_grey;
                    highlight_color.a = config.sound_alpha;

                    if *current_radius <= max_radius {
                        *current_radius += 1;
                        for sound_x in 0..data.map.width() {
                            for sound_y in 0..data.map.height() {
                                if distance((x, y), (sound_x, sound_y)) == *current_radius as i32 {
                                    draw_char(display_state, MAP_EMPTY_CHAR as char, sound_x, sound_y, highlight_color, area);
                                }
                            }
                        }
                    } else {
                        // TODO find a way to remove the sound object
                    }
                }

                // otherwise just draw the objects character (the default)
                _ => {
                    draw_char(display_state, object.chr, object.x, object.y, object.color, area);
                }
            }
        }
    }

    for obj in new_objects {
        data.objects.insert(obj);
    }
}

pub fn render_overlays(display_state: &mut DisplayState,
                       map_mouse_pos: Option<(usize, usize)>,
                       data: &mut GameData,
                       area: &Area,
                       config: &Config) {
    let player_handle = data.find_player().unwrap();
    let player_x = data.objects[player_handle].x;
    let player_y = data.objects[player_handle].y;

    // Draw player action overlay. Could draw arrows to indicate how to reach each location
    let mut highlight_color = config.color_warm_grey;
    highlight_color.a = config.highlight_alpha;

    // Draw player movement overlay
    for move_action in MoveAction::move_actions().iter() {
        // for all movements except staying still
        if *move_action != MoveAction::Center {
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
                    draw_char(display_state, MAP_EMPTY_CHAR as char, xy.0, xy.1, highlight_color, area);
                }
            }
        }
    }

    // draw attack position highlights
    if let Some(mouse_xy) = map_mouse_pos {
        let mut attack_highlight_color = config.color_red;
        attack_highlight_color.a = config.highlight_alpha;
        // Draw monster attack overlay
        let object_ids =  get_objects_under_mouse(mouse_xy.0 as i32, mouse_xy.1 as i32, data);
        for object_id in object_ids.iter() {
            let pos = data.objects[*object_id].pos();

            if data.map.is_in_fov(player_x, player_y, pos.0, pos.1, FOV_RADIUS) {
                if let Some(reach) = data.objects[*object_id].attack {
                    let attack_positions = 
                        reach.offsets()
                             .iter()
                             .map(|offset| (mouse_xy.0 as i32 + offset.0,
                                            mouse_xy.1 as i32 + offset.1))
                             // filter out positions that are outside of the map, or with no clear
                             // path from the entity to the reached position
                             .filter(|pos| data.map.is_within_bounds(pos.0, pos.1) &&
                                           data.clear_path((mouse_xy.0 as i32, mouse_xy.1 as i32), *pos))
                             .collect::<Vec<(i32, i32)>>();

                    for position in attack_positions {
                        draw_char(display_state, MAP_EMPTY_CHAR as char, position.0, position.1, attack_highlight_color, area);
                    }
                }
            }
        }
    }

    if let Some(mouse_handle) = data.find_mouse() {
        let mouse_pos = data.objects[mouse_handle].pos();
        let player_pos = data.objects[player_handle].pos();

        if config.draw_star_path {
            let path = data.map.astar(player_pos, mouse_pos);
            for pos in path {
                draw_char(display_state, MAP_EMPTY_CHAR as char, pos.0, pos.1, highlight_color, area);
            }
        }

        if config.draw_mouse_line {
            let line = Line::new(player_pos, mouse_pos).into_iter();
            for pos in line {
                draw_char(display_state, MAP_EMPTY_CHAR as char, pos.0, pos.1, highlight_color, area);
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

    data.map.compute_fov(data.objects[player_handle].x,
                         data.objects[player_handle].y,
                         FOV_RADIUS);

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
            mouse_map_pos = Some((map_x as usize, map_y as usize));

            if let Some(mouse_handle) = data.find_mouse() {
                data.objects[mouse_handle].set_pos(map_x as i32, map_y as i32);
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

                render_overlays(display_state, mouse_map_pos, data, &area, config);
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

    display_state.canvas.present();

    display_state.zones = zones;

    Ok(())
}

pub fn engine_color(color: &Color) -> Sdl2Color {
    return Sdl2Color::RGBA(color.r, color.g, color.b, color.a);
}

