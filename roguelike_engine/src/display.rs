use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use noise::Perlin;
use noise::NoiseFn;

use sdl2::render::{Texture, WindowCanvas};
use sdl2::rect::Rect;
use sdl2::pixels::{Color as Sdl2Color};

use roguelike_core::types::*;
use roguelike_core::map::*;
use roguelike_core::constants::*;
use roguelike_core::movement::*;
use roguelike_core::config::*;

use crate::plat::*;


pub struct DisplayState<'a> {
    pub font_image: Texture<'a>,
    pub display_overlays: bool,
    pub screen_sections: Plan,
    pub canvas: WindowCanvas,
}

impl<'a> DisplayState<'a> {
    pub fn new(screen_sections: Plan,
               font_image: Texture<'a>,
               canvas: WindowCanvas) -> DisplayState<'a> {

        return DisplayState {
            font_image,
            display_overlays: false,
            screen_sections,
            canvas,
        };
    }
}


pub struct Area {
    x_offset: f32,
    y_offset: f32,
    font_width: usize,
    font_height: usize,
}

impl Area {
    pub fn new(x_offset: f32, y_offset: f32, font_width: usize, font_height: usize) -> Area {
        Area { x_offset,
               y_offset,
               font_width,
               font_height,
        }
    }
}


pub fn get_objects_under_mouse(x: i32,
                               y: i32,
                               data: &mut GameData) -> Vec<ObjectId> {
    let mut object_ids = Vec::new();

    for key in data.objects.keys() {
        let pos = data.objects[key].pos();
        if pos == (x, y) {
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

// TODO merge this with current movement overlay-
// it uses a highlight color, which is nice, and 
// checks for clear paths.
pub fn draw_movement_overlay(display_state: &mut DisplayState,
                             data: &GameData,
                             id: ObjectId,
                             area: &Area,
                             config: &Config) -> Vec<(i32, i32)> {
    let mut added_positions = Vec::new();

    let color = config.color_warm_grey;

    if let Some(movement) = data.objects[id].movement {
        let offsets = movement.offsets();
        for offset in offsets {
            let x = data.objects[id].x as i32 + offset.0;
            let y = data.objects[id].y as i32 + offset.1;

            if clear_path(&data.map,
                          (data.objects[id].x as i32, data.objects[id].y as i32), 
                          (x, y),
                          &data.objects) {
                draw_char(display_state, '.', x, y, color, area);

                added_positions.push((x, y));
            }
        }
    }

    return added_positions;
}

pub fn draw_attack_overlay(display_state: &mut DisplayState,
                           id: ObjectId,
                           config: &Config,
                           area: &Area,
                           data: &GameData) -> Vec<(i32, i32)> {
    let mut added_positions = Vec::new();

    let color = config.color_warm_grey;

    if let Some(attack) = data.objects[id].attack {
        let offsets = attack.offsets();
        for offset in offsets {
            let x = data.objects[id].x as i32 + offset.0;
            let y = data.objects[id].y as i32 + offset.1;

            if clear_path(&data.map,
                          (data.objects[id].x as i32, data.objects[id].y as i32), 
                          (x, y),
                          &data.objects) {
                draw_char(display_state, 'x', x, y, color, area);

                added_positions.push((x, y));
            }
        }
    }

    return added_positions;
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
        }
    }
}

pub fn render_objects(display_state: &mut DisplayState,
                      data: &mut GameData,
                      settings: &GameSettings,
                      area: &Area) {
    let player_handle = data.find_player().unwrap();
    let player_pos = data.objects[player_handle].pos();

    for key in data.objects.keys() {
        if settings.god_mode ||
           data.map.is_in_fov(player_pos.0,
                              player_pos.1,
                              data.objects[key].x,
                              data.objects[key].y,
                              FOV_RADIUS) {
            let object = &data.objects[key];
            draw_char(display_state, object.chr, object.x, object.y, object.color, area);
        }
    }
}

pub fn render_overlays(display_state: &mut DisplayState,
                       mouse_state: &MouseState,
                       data: &mut GameData,
                       area: &Area,
                       config: &Config) {
    let player_handle = data.find_player().unwrap();

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
                draw_char(display_state, MAP_EMPTY_CHAR as char, xy.0, xy.1, highlight_color, area);
            }
        }
    }

    let mut attack_highlight_color = config.color_red;
    attack_highlight_color.a = config.highlight_alpha;
    // Draw monster attack overlay
    let font_width = (SCREEN_WIDTH / data.map.width() as u32) as i32;
    let font_height = (SCREEN_HEIGHT / data.map.height() as u32) as i32;
    let mouse_x = mouse_state.x as i32 / font_width;
    let mouse_y = mouse_state.y as i32 / font_height;
    let object_ids =  get_objects_under_mouse(mouse_x, mouse_y, data);
    for object_id in object_ids.iter() {
        if let Some(reach) = data.objects[*object_id].attack {
            let attack_positions = 
                reach.offsets()
                     .iter()
                     .map(|offset| (mouse_x + offset.0,
                                    mouse_y + offset.1))
                     .collect::<Vec<(i32, i32)>>();

            for position in attack_positions {
                draw_char(display_state, MAP_EMPTY_CHAR as char, position.0, position.1, attack_highlight_color, area);
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
    let area = Area::new(0.0,
                         0.0,
                         (screen_rect.0 as f32 / data.map.width() as f32) as usize,
                         (screen_rect.1 as f32 / data.map.height() as f32) as usize);

    render_background(display_state,
                      data,
                      &area,
                      settings,
                      config);

    render_map(display_state,
               data,
               &area,
               settings,
               config);

    render_objects(display_state,
                   data,
                   settings,
                   &area);

    render_overlays(display_state,
                    mouse_state,
                    data,
                    &area,
                    config);

    let screen_rect = display_state.canvas.logical_size();

    let plots = display_state.screen_sections
                             .plot(0,
                                   0,
                                   screen_rect.0 as usize,
                                   screen_rect.1 as usize);
    for plot in plots {
        // let plot_rect = Rect::new(plot.x as f32, plot.y as f32, plot.width as f32, plot.height as f32);

        //if plot.contains(mouse_state.x, mouse_state.y) {
        //    let (new_x, y_new) = plot.within(mouse_state.x, mouse_state.y);
        //    mouse_state.x_within = new_x;
        //    mouse_state.y_within = new_y;
        //    mouse_state.area_name = plot.name();
        //}

        match plot.name.as_str() {
            "screen" => {
            }

            "map" => {
                let ((_x_offset, _y_offset), _scaler) =
                    plot.fit(data.map.width() as usize * FONT_WIDTH as usize,
                             data.map.height() as usize * FONT_HEIGHT as usize);
            }

            "inspector" => {
                // Render game ui
                // display_state.gui.render(ctx, map, objects, mouse_state, plot.dims(), plot.pos());
            }

            section_name => {
                panic!(format!("Unexpected screen section '{}'", section_name));
            }
        }
    }

    display_state.canvas.present();

    Ok(())
}

pub fn engine_color(color: &Color) -> Sdl2Color {
    return Sdl2Color::RGBA(color.r, color.g, color.b, color.a);
}

pub fn draw_char(display_state: &mut DisplayState,
                 chr: char,
                 x: i32,
                 y: i32,
                 color: Color,
                 area: &Area) {
    let chr_x = (chr as i32) % FONT_WIDTH;
    let chr_y = (chr as i32) / FONT_HEIGHT;

    let src = Rect::new((chr_x * FONT_WIDTH) as i32,
                        (chr_y * FONT_HEIGHT) as i32,
                        FONT_WIDTH as u32,
                        FONT_HEIGHT as u32);

    let dst = Rect::new(x * area.font_width as i32,
                        y * area.font_height as i32,
                        area.font_width as u32,
                        area.font_height as u32);

    display_state.font_image.set_color_mod(color.r, color.g, color.b);
    display_state.font_image.set_alpha_mod(color.a);

    display_state.canvas.copy_ex(&display_state.font_image,
                                 Some(src),
                                 Some(dst),
                                 0.0,
                                 None,
                                 false,
                                 false).unwrap();
}