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
use roguelike_core::utils::{item_primary_at, distance, move_towards, lerp_color};

use crate::plat::*;
use crate::display::*;
use crate::game::*;


pub fn render_all(game: &mut Game)  -> Result<(), String> {

    let player_id = game.data.find_player().unwrap();

    game.data.map.compute_fov(game.data.objects[player_id].pos(), PLAYER_FOV_RADIUS);

    let screen_rect = game.display_state.canvas.output_size()?;

    let plots = game.display_state
                    .screen_sections
                    .plot(0,
                          0,
                          screen_rect.0 as usize,
                          screen_rect.1 as usize);

    game.display_state.canvas.set_draw_color(Sdl2Color::RGB(0, 0, 0));
    game.display_state.canvas.clear();

    let zones = plots.collect::<Vec<Plot>>();

    let mut mouse_map_pos = None;
    for zone in zones.iter() {
        if zone.name == "map" && zone.contains(game.mouse_state.x as usize, game.mouse_state.y as usize) {
            let ((_x_offset, _y_offset), scaler) =
                zone.fit(game.data.map.width() as usize * FONT_WIDTH as usize,
                         game.data.map.height() as usize * FONT_HEIGHT as usize);

            let mouse_map_xy = zone.within(game.mouse_state.x as usize, game.mouse_state.y as usize);
            let map_x = mouse_map_xy.0 as f32 / (FONT_WIDTH as f32 * scaler);
            let map_y = mouse_map_xy.1 as f32 / (FONT_HEIGHT as f32 * scaler);
            mouse_map_pos = Some(Pos::new(map_x as i32, map_y as i32));

            if let Some(mouse_id) = game.data.find_mouse() {
                game.data.objects[mouse_id].set_xy(map_x as i32, map_y as i32);
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
                    plot.fit(game.data.map.width() as usize * FONT_WIDTH as usize,
                             game.data.map.height() as usize * FONT_HEIGHT as usize);

                let area = Area::new(x_offset as i32,
                                     y_offset as i32,
                                     plot.width,
                                     plot.height,
                                     (scaler * FONT_WIDTH as f32) as usize, 
                                     (scaler * FONT_WIDTH as f32) as usize);


                render_background(game, &area);

                render_map(game, &area);

                render_objects(game, &area);

                render_effects(game, &area);

                render_overlays(game, mouse_map_pos, &area);
            }

            "inventory" => {
                let area = Area::new(plot.x as i32,
                                     plot.y as i32,
                                     plot.width,
                                     plot.height,
                                     FONT_WIDTH as usize,
                                     FONT_HEIGHT as usize);
                render_inventory(game, &area);
            }

            "player" => {
                let area = Area::new(plot.x as i32,
                                     plot.y as i32,
                                     plot.width,
                                     plot.height,
                                     FONT_WIDTH as usize,
                                     FONT_HEIGHT as usize);
                render_player(game, &area);
            }

            "info" => {
                let area = Area::new(plot.x as i32,
                                     plot.y as i32,
                                     plot.width,
                                     plot.height,
                                     FONT_WIDTH as usize,
                                     FONT_HEIGHT as usize);
                render_info(game, mouse_map_pos, &area);
            }

            section_name => {
                panic!(format!("Unexpected screen section '{}'", section_name));
            }
        }
    }

    if game.settings.state == GameState::Inventory {
        let width: usize = 300;
        let height: usize = 500;
        let area = Area::new((SCREEN_WIDTH as i32 / 2) - (width as i32 / 2),
                             (SCREEN_HEIGHT as i32 / 2) - (height as i32 / 2),
                             width,
                             height,
                             FONT_WIDTH as usize,
                             FONT_HEIGHT as usize);

        render_inventory(game, &area);
    }

    game.display_state.canvas.present();

    game.display_state.zones = zones;

    Ok(())
}

fn render_player(game: &mut Game, area: &Area) {
    draw_placard(&mut game.display_state,
                 "Player".to_string(),
                 area,
                 &game.config);

    let player_id = game.data.find_player().unwrap();

    let mut list = Vec::new();

    let color = game.config.color_warm_grey;
    let text_pos = Pos::new(1, 4);

    if let Some(fighter) = game.data.objects[player_id].fighter {
        let health_percent = fighter.hp as f32 / fighter.max_hp as f32;

        render_bar(&mut game.display_state, health_percent, 2, game.config.color_red, Color::white(), area);
    }

    list.push(format!("position:"));

    list.push(format!(" ({}, {})", 
                      game.data.objects[player_id].x,
                      game.data.objects[player_id].y));

    list.push(format!(""));

    let move_mode = game.data.objects[player_id].move_mode.unwrap();
    list.push(format!("{}", move_mode.to_string()));

    game.display_state.draw_text_list(list,
                                      text_pos,
                                      color,
                                      area);

}

fn render_info(game: &mut Game,
               mouse_xy: Option<Pos>,
               area: &Area) {
    draw_placard(&mut game.display_state,
                 "Info".to_string(),
                 area,
                 &game.config);

    if let Some(mouse) = mouse_xy {
        let player_id = game.data.find_player().unwrap();
        let player_pos = game.data.objects[player_id].pos();

        let object_ids =
            get_objects_under_mouse(mouse, &mut game.data);

        // only display first object
        if let Some(obj_id) = object_ids.first() {
            let mut y_pos = 1;

            let pos = game.data.objects[*obj_id].pos();

            // only display things in the player's FOV
            if game.data.map.is_in_fov(player_pos, pos, PLAYER_FOV_RADIUS) {
                if let Some(fighter) = game.data.objects[*obj_id].fighter {
                    y_pos += 1;

                    let health_percent = fighter.hp as f32 / fighter.max_hp as f32;

                    render_bar(&mut game.display_state,
                               health_percent,
                               y_pos,
                               game.config.color_red,
                               Color::white(), area);

                    y_pos += 2;
                }

                let mut text_list = Vec::new();

                let color = game.config.color_warm_grey;
                let text_pos = Pos::new(1, y_pos);
                text_list.push(format!("{}", game.data.objects[*obj_id].name));

                text_list.push(format!(""));

                if game.data.objects[*obj_id].fighter.map_or(false, |fighter| fighter.hp <= 0) {
                    text_list.push(format!("{}", "dead"));
                } else if let Some(behave) = game.data.objects[*obj_id].behavior {
                    text_list.push(format!("{}", behave.description()));
                }

                game.display_state.draw_text_list(text_list,
                                                  text_pos,
                                                  color,
                                                  area);
            }
        }
    }
}

/// Render an inventory section within the given area
fn render_inventory(game: &mut Game, area: &Area) {
    // Render header
    draw_placard(&mut game.display_state,
                 "Inventory".to_string(),
                 area,
                 &game.config);

    let player_id = game.data.find_player().unwrap();

    // Render each object's name in inventory
    let mut y_pos = 2;
    let mut item_index = 0;
    let item_ids = game.data.objects[player_id].inventory.clone();

    for (index, obj_id) in item_ids.iter().enumerate() {
        let obj = &game.data.objects[*obj_id];

        let color;
        match obj.item {
            Some(Item::Stone) => {
                color = game.config.color_light_grey;
            }

            Some(Item::Goal) => {
                color = game.config.color_red;
            }
            
            _ => {
                color = game.config.color_mint_green;
            }
        }

        // place prompt character
        game.display_state.draw_char(('0' as u8 + item_index) as char,
                               Pos::new(1, y_pos),
                               game.config.color_ice_blue,
                               area);

        // place object name
        let text_pos = Pos::new(2, y_pos);
        let item_marker =
            if index == 0 && item_primary_at(player_id, &mut game.data.objects, 0) {
                "*"
            } else {
                ""
            };
        let item_text = format!(" {} {}", game.data.objects[*obj_id].name, item_marker);
        game.display_state.draw_text(item_text,
                                     text_pos,
                                     color,
                                     area);
        
        y_pos += 1;

        item_index += 1;
    }

    if game.data.objects[player_id].inventory.len() == 0 {
        let text_pos = Pos::new(1, y_pos);
        game.display_state.draw_text(format!("empty"),
                               text_pos,
                               game.config.color_ice_blue,
                               area);
    }
}

/// render the background files, including water tiles
fn render_background(game: &mut Game, area: &Area) {
    let player_id = game.data.find_player().unwrap();
    let pos = game.data.objects[player_id].pos();

    if let Some(background) = &mut game.display_state.background {
        let src = area.get_rect();

        let dst = area.get_rect();

        game.display_state
            .canvas
            .copy_ex(background,
                     Some(src),
                     Some(dst),
                     0.0,
                     None,
                     false,
                     false).unwrap();
    } else {
        let pixel_format = game.display_state.texture_creator.default_pixel_format();

        let mut background =
            game.display_state
                .texture_creator
                .create_texture_target(pixel_format,
                                       area.width as u32,
                                       area.height as u32).unwrap();

        {
            // unpack fields to prevent borrowing issues
            let (canvas, font_image, map, settings, config) =
                (&mut game.display_state.canvas,
                 &mut game.display_state.font_image,
                 &mut game.data.map,
                 &game.settings,
                 &game.config);

            canvas.with_texture_canvas(&mut background, |canvas| {
                for y in 0..map.height() {
                    for x in 0..map.width() {
                        let map_pos = Pos::new(x, y);

                        let visible =
                            map.is_in_fov(pos, map_pos, PLAYER_FOV_RADIUS) ||
                            settings.god_mode;

                        draw_char(canvas,
                                  font_image,
                                  MAP_EMPTY_CHAR as char,
                                  map_pos,
                                  empty_tile_color(&config, map_pos, visible),
                                  area);

                        let tile = &map.tiles[x as usize][y as usize];
                        if tile.tile_type == TileType::Water {
                            let color = tile_color(&config, x, y, tile, visible);
                            let chr = tile.chr.map_or('+', |chr| chr);
                            draw_char(canvas, font_image, chr, map_pos, color, area);
                        }
                    }
                }
            }).unwrap();
        }

        game.display_state.background = Some(background);
    }
}

/// Render the map, with environment and walls
fn render_map(game: &mut Game, area: &Area) {
    let map_width = game.data.map.width();
    let map_height = game.data.map.height();

    let player_id = game.data.find_player().unwrap();
    let player_pos = game.data.objects[player_id].pos();

    for y in 0..map_height {
        for x in 0..map_width {
            let pos = Pos::new(x, y);

            // Render game stuff
            let visible =
                game.data.map.is_in_fov(player_pos, pos, PLAYER_FOV_RADIUS) ||
                game.settings.god_mode;

            game.data.map[pos].explored |= visible;

            let explored = game.data.map[pos].explored || visible;

            let tile = &game.data.map[pos];

            let wall_color =
                if explored {
                    game.config.color_light_brown
                } else {
                    game.config.color_dark_brown
                };

            let chr = tile.chr.map_or('+', |chr| chr);

            // draw empty tile first, in case there is transparency in the character
            // draw_char(display_state, MAP_EMPTY_CHAR as char, x, y, empty_tile_color(config, x, y, visible));

            // if the tile is not empty or water, draw it
            let color = tile_color(&game.config, x, y, tile, visible);
            if chr != MAP_EMPTY_CHAR as char && tile.tile_type != TileType::Water {
                game.display_state.draw_char(chr, pos, color, area);
            }

            if tile.surface == Surface::Rubble {
                game.display_state.draw_char(MAP_RUBBLE as char, pos, color, area);
            }

            // finally, draw the between-tile walls appropriate to this tile
            if tile.bottom_wall == Wall::ShortWall {
                game.display_state.draw_char(MAP_THIN_WALL_BOTTOM as char, pos, wall_color, area);
            } else if tile.bottom_wall == Wall::TallWall {
                game.display_state.draw_char(MAP_THICK_WALL_BOTTOM as char, pos, wall_color, area);
            }

            if tile.left_wall == Wall::ShortWall {
                game.display_state.draw_char(MAP_THIN_WALL_LEFT as char, pos, wall_color, area);
            } else if tile.left_wall == Wall::TallWall {
                game.display_state.draw_char(MAP_THICK_WALL_LEFT as char, pos, wall_color, area);
            }

            if x + 1 < map_width {
                let right_tile = &game.data.map.tiles[x as usize + 1][y as usize];
                if right_tile.left_wall == Wall::ShortWall {
                    game.display_state.draw_char(MAP_THIN_WALL_RIGHT as char, pos, wall_color, area);
                } else if right_tile.left_wall == Wall::TallWall {
                    game.display_state.draw_char(MAP_THICK_WALL_RIGHT as char, pos, wall_color, area);
                }
            }

            if y - 1 >= 0 {
                let above_tile = &game.data.map.tiles[x as usize][y as usize - 1];
                if above_tile.bottom_wall == Wall::ShortWall {
                    game.display_state.draw_char(MAP_THIN_WALL_TOP as char, pos, wall_color, area);
                } else if above_tile.bottom_wall == Wall::TallWall {
                    game.display_state.draw_char(MAP_THICK_WALL_TOP as char, pos, wall_color, area);
                }
            }

            // Draw a square around this tile to help distinguish it visually in the grid
            let outline_color = Color::white();
            let alpha;
            if visible && game.data.map[pos].tile_type != TileType::Water {
                if game.settings.overlay {
                    alpha = game.config.grid_alpha_overlay;
                } else {
                    alpha = game.config.grid_alpha_visible;
                }
            } else {
                alpha = game.config.grid_alpha;
            }
            let color = Sdl2Color::RGBA(outline_color.r, outline_color.g, outline_color.b, alpha);

            if game.config.fog_of_war && !visible {
                let mut blackout_color = Color::black();
                if game.data.map[pos].explored {
                    blackout_color.a = game.config.explored_alpha
                }
                game.display_state.draw_char(MAP_EMPTY_CHAR as char, pos, blackout_color, area);
            }

            // draw an outline around the tile
            game.display_state.canvas.set_blend_mode(BlendMode::Blend);
            game.display_state.canvas.set_draw_color(color);
            game.display_state.canvas.draw_rect(area.char_rect(x, y)).unwrap();
        }
    }
}

/// Render each effect currently playing in the game
/// The strategy here is to copy the effects vector, update all items,
/// and then remove finished effects from back to front. The
/// resulting vector of effects is then saved as the new effects vector.
fn render_effects(game: &mut Game, area: &Area) {
    let mut remove_indices = Vec::new();

    let mut effects = game.display_state.effects.clone();

    for (index, effect) in effects.iter_mut().enumerate() {
        match effect {
            Effect::HeardSomething(pos, created_turn) => {
                game.display_state.draw_char(ENTITY_ELF as char,
                                             *pos,
                                             game.config.color_warm_grey,
                                             area);

                if *created_turn != game.settings.turn_count {
                    dbg!(*created_turn, game.settings.turn_count);
                    remove_indices.push(index);
                }
            }

            Effect::Sound(sound_aoe, sound_dt) => {
                let mut highlight_color = game.config.color_warm_grey;

                let radius = sound_aoe.positions.len();
                let sound_interval = game.config.sound_timeout / radius as f32;
                let cur_dist = *sound_dt / sound_interval;
                for (dist, dist_positions) in sound_aoe.positions.iter().enumerate() {
                    highlight_color.a =
                        game.config.sound_alpha / ((dist as i16 - cur_dist as i16).abs() as u8 + 1);

                    let player_id = game.data.find_player().unwrap();
                    let player_pos = game.data.objects[player_id].pos();
                    for pos in dist_positions.iter() {
                        if !game.data.map[*pos].blocked &&
                           game.data.map.is_in_fov(player_pos, *pos, PLAYER_FOV_RADIUS) {
                            game.display_state.draw_char(MAP_EMPTY_CHAR as char, *pos, highlight_color, area);
                        }
                    }
                }

                if *sound_dt >= game.config.sound_timeout {
                    remove_indices.push(index);
                } else {
                    *sound_dt += 1.0 / game.config.rate as f32;
                }
            }
        }
    }

    remove_indices.sort();
    remove_indices.reverse();
    for index in remove_indices {
        effects.swap_remove(index);
    }

    game.display_state.effects = effects;
}

/// Render each object in the game, filtering for objects not currently visible
fn render_objects(game: &mut Game, area: &Area) {
    let player_id = game.data.find_player().unwrap();
    let player_pos = game.data.objects[player_id].pos();

    // step each objects animation
    for object_id in game.data.objects.keys().collect::<Vec<ObjectId>>().iter() {
        let pos = game.data.objects[*object_id].pos();

        // only draw if within the map (outside is (-1, -1) like if in inventory).
        if game.data.map.is_within_bounds(pos) {
            let is_in_fov = 
               game.data.map.is_in_fov(player_pos, pos, PLAYER_FOV_RADIUS);

            if let Some(anim_key) = game.data.objects[*object_id].animation.get(0) {
                let done = 
                    step_animation(*anim_key,
                                   *object_id,
                                   is_in_fov,
                                   &mut game.display_state,
                                   &mut game.data,
                                   &game.settings,
                                   &game.config,
                                   area);

                if done {
                    game.data.objects[*object_id].animation.pop_front();
                }
            } else {
                if is_in_fov {
                    let color = game.data.objects[*object_id].color;
                    game.display_state.draw_char(game.data.objects[*object_id].chr, pos, color, area);
                }
            }
        }
    }
}

fn step_animation(anim_key: AnimKey,
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
           if settings.god_mode || is_in_fov {
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

fn render_overlays(game: &mut Game,
                   map_mouse_pos: Option<Pos>,
                   area: &Area) {
    let player_id = game.data.find_player().unwrap();
    let player_pos = game.data.objects[player_id].pos();

    // render a grid of numbers if enabled
    if game.config.overlay_directions {
        let map_width = game.data.map.width();
        let map_height = game.data.map.height();
        for y in 0..map_height {
            for x in 0..map_width {
                let pos = Pos::new(x, y);
                let x_diff = x - player_pos.x;
                let y_diff = y - player_pos.y;

                if x_diff.abs() < 5 && y_diff.abs() < 5 {
                    let res: i8 = x_diff as i8 - y_diff as i8;
                    if res <= 0 {
                        game.display_state.draw_char(MAP_GROUND as char, pos, game.config.color_light_green, area);
                    } else {
                        game.display_state.draw_char(MAP_GROUND as char, pos, game.config.color_light_grey, area);
                    }
                    game.display_state.draw_char(('0' as u8 + res.abs() as u8) as char, pos, game.config.color_red, area);
                }
            }
        }
    }

    // render FOV if enabled
    if game.config.overlay_player_fov {
        let map_width = game.data.map.width();
        let map_height = game.data.map.height();
        for y in 0..map_height {
            for x in 0..map_width {
                let pos = Pos::new(x, y);

                let dir = game.data.objects[player_id].direction.unwrap();
                let is_in_fov =
                    game.data.map.is_in_fov_direction(player_pos,
                                                      pos,
                                                      PLAYER_FOV_RADIUS,
                                                      dir);
                if is_in_fov {
                    game.display_state.draw_char(MAP_GROUND as char, pos, game.config.color_light_green, area);
                }
            }
        }
    }

    // Draw player action overlay. Could draw arrows to indicate how to reach each location
    let mut highlight_color: Color = game.config.color_warm_grey;
    highlight_color.a = game.config.highlight_player_move;

    // draw direction overlays
    let mut direction_color = game.config.color_soft_green;
    direction_color.a /= 2;
    for object_id in game.data.objects.keys().collect::<Vec<ObjectId>>().iter() {
        let pos = game.data.objects[*object_id].pos();

        if pos.x == -1 && pos.y == -1 {
            continue;
        }

        if game.data.map.is_in_fov(player_pos, pos, PLAYER_FOV_RADIUS) &&
           game.data.objects[*object_id].alive {
            if let Some(dir) = game.data.objects[*object_id].direction {
                // display_state.draw_tile_edge(pos, area, direction_color, dir);

                let rotation = match dir {
                    Direction::Up => -90.0,
                    Direction::Down => 90.0,
                    Direction::Right => 0.0,
                    Direction::Left => 180.0,
                    Direction::DownLeft => 135.0,
                    Direction::DownRight => 45.0,
                    Direction::UpLeft => -135.0,
                    Direction::UpRight => -45.0,
                };

                game.display_state.draw_char_with_rotation(ARROW_RIGHT as char, pos, direction_color, area, rotation);
            }
        }
    }

    // draw attack position highlights
    if let Some(mouse_xy) = map_mouse_pos {
        // Draw monster attack overlay
        let object_ids = get_objects_under_mouse(mouse_xy, &mut game.data);
        for object_id in object_ids.iter() {
            let pos = game.data.objects[*object_id].pos();

            if game.data.map.is_in_fov(player_pos, pos, PLAYER_FOV_RADIUS) &&
               game.data.objects[*object_id].alive {
               render_attack_overlay(game,
                                     *object_id,
                                     area);
            }
        }
    }

    if game.settings.overlay {
        let keys = game.data.objects.keys().collect::<Vec<ObjectId>>();
        for object_id in keys {
            let pos = game.data.objects[object_id].pos();

            if game.data.map.is_within_bounds(pos) &&
               game.data.map.is_in_fov(player_pos, pos, PLAYER_FOV_RADIUS) &&
               game.data.objects[object_id].alive {
               render_attack_overlay(game,
                                     object_id,
                                     area);
            }
        }
    }

    // draw mouse path overlays
    if let Some(mouse_id) = game.data.find_mouse() {
        let mouse_pos = game.data.objects[mouse_id].pos();
        let player_pos = game.data.objects[player_id].pos();

        if game.config.draw_star_path {
            let path = game.data.map.astar(player_pos, mouse_pos);
            for pos in path {
                game.display_state.draw_char(MAP_EMPTY_CHAR as char, pos, highlight_color, area);
            }
        }

        if game.config.draw_mouse_line || game.settings.draw_throw_overlay {
            // mouse pos at 0, 0 occurs when the mouse has not moved since startup.
            // this may cause a weirdness on the corner of the map
            if mouse_pos != Pos::new(0, 0) {
                let line = Line::new(player_pos.to_tuple(), mouse_pos.to_tuple()).into_iter();
                for pos in line {
                    let pos = Pos::from(pos);
                    game.display_state.draw_char(MAP_EMPTY_CHAR as char, pos, highlight_color, area);
                }
            }
        }
    }

    // Draw player movement overlay
    if game.settings.overlay {
        for move_action in Direction::move_actions().iter() {
            // for all movements except staying still
            // calculate the move that would occur
            if let Some(movement) =
                calculate_move(*move_action,
                               game.data.objects[player_id].movement.unwrap(),
                               player_id,
                               &mut game.data) {
                // draw a highlight on that square
                // don't draw overlay on top of character
                if movement.pos != game.data.objects[player_id].pos()
                {
                    game.display_state.draw_tile_outline(movement.pos, area, highlight_color);
                }
            }
        }
    }

}

//fn engine_color(color: &Color) -> Sdl2Color {
//    return Sdl2Color::RGBA(color.r, color.g, color.b, color.a);
//}

fn get_objects_under_mouse(mouse_pos: Pos,
                           data: &mut GameData) -> Vec<ObjectId> {
    let mut object_ids = Vec::new();

    for key in data.objects.keys() {
        let pos = data.objects[key].pos();
        let is_mouse = data.objects[key].name == "mouse";

        if !is_mouse && mouse_pos == pos {
            if data.map.is_in_fov(pos, mouse_pos, PLAYER_FOV_RADIUS) {
                object_ids.push(key);
            }
        }
    }

    return object_ids;
}

fn empty_tile_color(config: &Config, pos: Pos, visible: bool) -> Color {
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

fn tile_color(config: &Config, _x: i32, _y: i32, tile: &Tile, visible: bool) -> Color {
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
fn draw_placard(display_state: &mut DisplayState,
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

fn render_bar(display_state: &mut DisplayState,
              percent: f32,
              y_pos: i32,
              fg_color: Color,
              bg_color: Color,
              area: &Area) {
    display_state.canvas.set_blend_mode(BlendMode::None);
    let color = Sdl2Color::RGBA(fg_color.r, fg_color.g, fg_color.b, fg_color.a);
    display_state.canvas.set_draw_color(color);
    let start = area.char_rect(1, y_pos);
    let width = area.width as u32  - 2 * start.width();
    let health_rect = Rect::new(start.x,
                                start.y,
                                (width as f32 * percent) as u32,
                                start.height());
    display_state.canvas.fill_rect(health_rect).unwrap();

    let full_rect = Rect::new(start.x,
                              start.y,
                              width,
                              start.height());
    let color = Sdl2Color::RGBA(bg_color.r, bg_color.g, bg_color.b, bg_color.a);
    display_state.canvas.set_draw_color(color);
    display_state.canvas.draw_rect(full_rect).unwrap();
}

fn render_attack_overlay(game: &mut Game,
                         object_id: ObjectId,
                         area: &Area) {
    let player_id = game.data.find_player().unwrap();
    let player_pos = game.data.objects[player_id].pos();

    let object_pos = game.data.objects[object_id].pos();

    let mut attack_highlight_color = game.config.color_red;
    attack_highlight_color.a = game.config.highlight_attack;

    if let Some(reach) = game.data.objects[object_id].attack {
        let attack_positions = 
            reach.offsets()
                 .iter()
                 .map(|offset| Pos::new(object_pos.x as i32 + offset.x,
                                        object_pos.y as i32 + offset.y))
                 // filter out positions that are outside of the map, or with no clear
                 // path from the entity to the reached position
                 .filter(|pos| {
                     let in_bounds = game.data.map.is_within_bounds(*pos);
                     let clear = game.data.clear_path(object_pos, *pos);
                     let player_can_see = game.data.map.is_in_fov(player_pos, *pos, PLAYER_FOV_RADIUS);
                     // check for player position so it gets highligted, even
                     // though the player causes 'clear_path' to fail.
                     return player_can_see && in_bounds && (clear || *pos == player_pos);
                 })
                 .collect::<Vec<Pos>>();

        for position in attack_positions {
            game.display_state.draw_char(MAP_EMPTY_CHAR as char, position, attack_highlight_color, area);
        }
    }
}

