use std::cmp::Ordering;

use noise::NoiseFn;
use noise::Perlin;

use sdl2::render::{BlendMode, WindowCanvas};
use sdl2::rect::Rect;
use sdl2::pixels::{Color as Sdl2Color};

use roguelike_core::types::*;
use roguelike_core::map::*;
use roguelike_core::constants::*;
use roguelike_core::movement::*;
use roguelike_core::config::*;
use roguelike_core::animation::{Sprite, Effect, Animation, AnimKey};
use roguelike_core::utils::{item_primary_at, distance, move_towards, lerp_color, move_x, move_y, sub_pos, floodfill, reach_by_mode, map_fill_metric};
use roguelike_core::line::line;

use roguelike_engine::game::*;

use crate::display::*;


// 10ms to display on last check
pub fn render_all(display: &mut Display, game: &mut Game)  -> Result<(), String> {
    display.targets.canvas_panel.target.set_draw_color(Sdl2Color::RGB(0, 0, 0));
    display.targets.canvas_panel.target.clear();

    display.state.update_animations(game.settings.dt);

    /* Split Screen Into Sections */
    let map_rect = display.targets.canvas_panel.get_rect_from_area(&display.targets.map_area);

    /* Draw Background */
    render_background(display, game);

    /* Draw Map */
    render_panels(display, game, map_rect);

    /* Paste Panels on Screen */
    let map_size = game.data.map.size();
    render_screen(&mut display.targets, map_size, map_rect);

    /* Draw Menus */
    render_menus(display, game);

    Ok(())
}


fn render_panels(display: &mut Display, game: &mut Game, map_rect: Rect) {
    /* Determine Mouse Position */
    /* Removed in favor of the cursor
    let mut mouse_map_pos = None;
    if let Some(mouse_id) = game.data.find_by_name(EntityName::Mouse) {
        let mouse_pos = (display.mouse_state.x, display.mouse_state.y);
        if let Some(mouse_cell) = cell_within_rect(map_rect, game.data.map.size(), mouse_pos) {
            // NOTE this should be done as an action MapClick
            game.data.entities.set_xy(mouse_id, mouse_cell.0 as i32, mouse_cell.1 as i32);
            mouse_map_pos = Some(Pos::new(mouse_cell.0 as i32, mouse_cell.1 as i32));
        }
    }
    let mouse_map_pos = mouse_map_pos;
    */

    let mouse_map_pos = Some(game.settings.cursor_pos); //mouse_map_pos;

    let canvas = &mut display.targets.canvas_panel.target;
    let display_state = &mut display.state;

    if game.settings.render_map {
        let background = &mut display.targets.background_panel;
        let panel = background.unit();

        canvas.with_texture_canvas(&mut display.targets.map_panel.target, |canvas| {
            canvas.set_draw_color(Sdl2Color::RGB(0, 0, 0));
            canvas.clear();

            canvas.copy(&background.target, None, None).unwrap();

            let mut panel = panel.with_target(canvas);
            render_map(&mut panel, display_state, game);
            render_entities(&mut panel, display_state, game);
            render_impressions(&mut panel, display_state, game);
            render_effects(&mut panel, display_state, game);
            render_overlays(&mut panel, display_state, game, mouse_map_pos);
        }).unwrap();
    }

    /* Draw Player Info */
    {
        let player_panel = &mut display.targets.player_panel;
        let panel = player_panel.unit();

        canvas.with_texture_canvas(&mut display.targets.player_panel.target, |canvas| {
            let mut panel = panel.with_target(canvas);
            render_player_info(&mut panel, display_state, game);
        }).unwrap();
    }

    /* Draw Inventory */
    {
        let inventory_panel = &mut display.targets.inventory_panel;
        let panel = inventory_panel.unit();

        canvas.with_texture_canvas(&mut display.targets.inventory_panel.target, |canvas| {
            let mut panel = panel.with_target(canvas);
            render_inventory(&mut panel, display_state, game);
        }).unwrap();
    }

    /* Draw Game Info */
    {
        let info_panel = &mut display.targets.info_panel;
        let panel = info_panel.unit();

        canvas.with_texture_canvas(&mut display.targets.info_panel.target, |canvas| {
            let mut panel = panel.with_target(canvas);
            render_info(&mut panel, display_state, game, mouse_map_pos);
        }).unwrap();
    }
}


fn render_screen(targets: &mut DisplayTargets, map_size: (i32, i32), map_rect: Rect) {
    // TODO just make the map panel the right size in the first place
    // and re-create it when the map changes.
    let src = targets.map_panel.get_rect_up_left(map_size.0 as usize, map_size.1 as usize);
    targets.canvas_panel.target.copy(&targets.map_panel.target, src, map_rect).unwrap();

    /* Draw Inventory Panel */
    let dst = targets.canvas_panel.get_rect_within(&targets.inventory_area,
                                                   targets.inventory_panel.num_pixels);
    targets.canvas_panel.target.copy(&targets.inventory_panel.target, None, dst).unwrap();

    /* Draw Game Info Panel */
    let dst = targets.canvas_panel.get_rect_within(&targets.info_area,
                                                   targets.info_panel.num_pixels);
    targets.canvas_panel.target.copy(&targets.info_panel.target, None, dst).unwrap();

    /* Draw Player Info Panel */
    let dst = targets.canvas_panel.get_rect_within(&targets.player_area,
                                                   targets.player_panel.num_pixels);
    targets.canvas_panel.target.copy(&targets.player_panel.target, None, dst).unwrap();
}

fn render_menus(display: &mut Display, game: &mut Game) {
    let canvas_panel = &mut display.targets.canvas_panel;
    let display_state = &mut display.state;

    let menu_panel = &mut display.targets.menu_panel;
    let inventory_panel = &mut display.targets.inventory_panel;
    let panel = menu_panel.unit();

    let mut draw_menu: bool = true;
    canvas_panel.target.with_texture_canvas(&mut menu_panel.target, |canvas| {
        let mut panel = panel.with_target(canvas);

        if game.settings.state == GameState::Inventory {
            panel.target.copy(&inventory_panel.target, None, None).unwrap();
        } else if game.settings.state == GameState::SkillMenu {
            render_skill_menu(&mut panel, display_state, game);
        } else if game.settings.state == GameState::ClassMenu {
            render_class_menu(&mut panel, display_state, game);
        } else if game.settings.state == GameState::ConfirmQuit {
            render_confirm_quit(&mut panel, display_state, game);
        } else {
            draw_menu = false;
        }
    }).unwrap();

    if draw_menu {
        let dst = canvas_panel.get_rect_within(&display.targets.menu_area, menu_panel.num_pixels);
        canvas_panel.target.copy(&menu_panel.target, None, dst).unwrap();
    }
}

/// Draw an outline and title around an area of the screen
fn render_placard(panel: &mut Panel<&mut WindowCanvas>,
                  display_state: &mut DisplayState,
                  text: &str,
                  config: &Config) {
    let color = config.color_mint_green;
    
    // Draw a black background
    panel.target.set_draw_color(Sdl2Color::RGBA(0, 0, 0, 255));
    panel.target.clear();
    let (width, height) = panel.target.output_size().unwrap();
    let (cell_width, cell_height) = panel.cell_dims();

    panel.target.set_draw_color(sdl2_color(color));

    // Draw a thin line around the edges of the placard
    panel.target.draw_rect(Rect::new(cell_width as i32 / 2,
                                     cell_height as i32 / 2,
                                     width as u32 - (cell_width / 2),
                                     height as u32 - (cell_height / 2))).unwrap();

    // draw a rectangle around where the placard header text will be placed.
    let half_text = text.len() / 2;
    let text_offset = (width / 2) - (cell_width * half_text as u32);
    panel.target.fill_rect(Rect::new(text_offset as i32 - 3,
                                     0,
                                     (text.len() * cell_width as usize) as u32 + 2,
                                     cell_height as u32)).unwrap();

    // Draw header text
    let mid_char_offset = (width / cell_width) / 2;
    let text_start = (mid_char_offset - half_text as u32) as i32;

    let text_pos = Pos::new(text_start, 0);

    let sprite_key = display_state.lookup_spritekey("tiles");
    let tile_sprite = &mut display_state.sprites[&sprite_key];

    tile_sprite.draw_text(panel, &text, text_pos, config.color_dark_blue);
}

fn render_pips(panel: &mut Panel<&mut WindowCanvas>,
               _display_state: &mut DisplayState,
               num_pips: u32,
               y_pos: i32,
               color: Color) {
    if num_pips > 0 {
        let blend_mode = panel.target.blend_mode();
        panel.target.set_blend_mode(blend_mode);

        let color = sdl2_color(color);
        panel.target.set_draw_color(color);

        let cell_dims = panel.cell_dims();
        let x = cell_dims.0 * 2;
        let y = cell_dims.1 * y_pos as u32;

        let spacing = 4;
        let mut pips = Vec::new();
        for pip_index in 0..num_pips as u32 {
            let x_offset =  x + cell_dims.1 * pip_index + spacing as u32 * pip_index;
            let pip = Rect::new(x_offset as i32,
                                y as i32 + spacing as i32,
                                cell_dims.0,
                                cell_dims.1);
            pips.push(pip)
        }
        panel.target.fill_rects(&pips).unwrap();

        panel.target.set_blend_mode(BlendMode::None);
    }
}

fn render_bar(panel: &mut Panel<&mut WindowCanvas>,
              _display_state: &mut DisplayState,
              percent: f32,
              y_pos: i32,
              fg_color: Color,
              bg_color: Color) {
    let blend_mode = panel.target.blend_mode();

    panel.target.set_blend_mode(BlendMode::None);
    let color = sdl2_color(fg_color);
    panel.target.set_draw_color(color);

    let cell_dims = panel.cell_dims();

    let width = panel.num_pixels.0 as u32  - 3 * cell_dims.0;

    let x = cell_dims.0 as i32 * 2;

    let health_rect = Rect::new(x,
                                cell_dims.1 as i32 * y_pos,
                                (width as f32 * percent) as u32,
                                cell_dims.1);
    panel.target.fill_rect(health_rect).unwrap();

    let full_rect = Rect::new(x,
                              cell_dims.1 as i32 * y_pos,
                              width,
                              cell_dims.1);
    let color = sdl2_color(bg_color);
    panel.target.set_draw_color(color);
    panel.target.draw_rect(full_rect).unwrap();

    panel.target.set_blend_mode(blend_mode);
}


// TODO console
/*
fn render_console(display: &mut Display, game: &mut Game) {
    let color = game.config.color_console;
    let color = sdl2_color(color);
    display.canvas.set_draw_color(color);

    let console_rect =
        Rect::new(0, (SCREEN_HEIGHT - game.console.height) as i32, SCREEN_WIDTH, SCREEN_HEIGHT / 2);
    display.canvas.fill_rect(console_rect).unwrap();

    let color = Sdl2Color::RGBA(255, 255, 255, 255);
    display.canvas.set_draw_color(color);

    let line_width = 1;

    let y_offset = (SCREEN_HEIGHT - game.console.height) as i32;

    let top_line_rect =
        Rect::new(0, y_offset, SCREEN_WIDTH, line_width);
    display.canvas.fill_rect(top_line_rect).unwrap();

    let bottom_line_rect =
        Rect::new(0, SCREEN_HEIGHT as i32 - line_width as i32, SCREEN_WIDTH, line_width);
    display.canvas.fill_rect(bottom_line_rect).unwrap();

    let left_line_rect =
        Rect::new(0, y_offset, line_width, game.console.height);
    display.canvas.fill_rect(left_line_rect).unwrap();

    let right_line_rect =
        Rect::new(SCREEN_WIDTH as i32 - line_width as i32, y_offset, line_width, game.console.height);
    display.canvas.fill_rect(right_line_rect).unwrap();

    let console_area = 
        Area::new(0, y_offset, SCREEN_WIDTH as usize, y_offset as usize, FONT_WIDTH as usize, FONT_HEIGHT as usize);

    display.draw_char('>',
                            Pos::new(0, 0),
                            Color::white(),
                            &console_area);
    display.draw_text(&game.console.input.clone(),
                            Pos::new(1, 0),
                            Color::white(),
                            &console_area);

    let mut y_pos = 1;
    for output in game.console.output.iter() {
        display.draw_text(&output.clone(),
                                Pos::new(0, y_pos),
                                Color::white(),
                                &console_area);
        y_pos += 1;
    }
}
*/

fn render_player_info(panel: &mut Panel<&mut WindowCanvas>, display_state: &mut DisplayState, game: &mut Game) {
    render_placard(panel, display_state, "Player", &game.config);

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();

    let mut list: Vec<String> = Vec::new();

    let color = game.config.color_soft_green;

    if let Some(fighter) = game.data.entities.fighter.get(&player_id) {
        let hp = if fighter.hp > 0 {
            fighter.hp
        } else {
            0
        };
        let health_percent = hp as f32 / fighter.max_hp as f32;

        render_bar(panel, display_state, health_percent, 2, game.config.color_red, Color::white());
    }

    let energy = game.data.entities.energy[&player_id];
    render_pips(panel, display_state, energy, 3, game.config.color_light_green);

    list.push(format!("position:"));

    list.push(format!(" ({}, {})", 
                      game.data.entities.pos[&player_id].x,
                      game.data.entities.pos[&player_id].y));

    list.push(format!(""));

    let move_mode = game.data.entities.move_mode[&player_id];
    list.push(format!("{}", move_mode.to_string()));

    let stance = game.data.entities.stance[&player_id];
    list.push(format!("{:?}", stance));

    list.push(format!(""));
    list.push(format!("turn {}", game.settings.turn_count));

    let text_pos = Pos::new(1, 5);

    let sprite_key = display_state.lookup_spritekey("tiles");
    let tile_sprite = &mut display_state.sprites[&sprite_key];
    tile_sprite.draw_text_list(panel, &list, text_pos, color);
}

fn render_info(panel: &mut Panel<&mut WindowCanvas>,
               display_state: &mut DisplayState,
               game: &mut Game,
               mouse_xy: Option<Pos>) {
    render_placard(panel, display_state, "Info", &game.config);

    let sprite_key = display_state.lookup_spritekey("tiles");

    let info_pos = game.settings.cursor_pos;
    // NOTE this allows mouse support as well
    //    if let Some(mouse) = mouse_xy {
    //        mouse
    //    } else {
    //        game.settings.cursor_pos
    //    };


    let text_color = game.config.color_soft_green;

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();

    let object_ids =
        get_entity_at_pos(info_pos, &mut game.data);

    let mut y_pos = 1;

    let mut text_list = Vec::new();

    text_list.push(format!("({:>2},{:>2})", info_pos.x, info_pos.y));

    let text_pos = Pos::new(1, y_pos);

    {
        let tile_sprite = &mut display_state.sprites[&sprite_key];
        tile_sprite.draw_text_list(panel, &text_list, text_pos, text_color);
    }
    text_list.clear();

    y_pos += 1;

    let in_fov = game.settings.god_mode ||
                 game.data.is_in_fov(player_id, info_pos, &game.config);

    // only display first object
    if let Some(obj_id) = object_ids.first() {
        // only display things in the player's FOV
        if in_fov {
            if let Some(fighter) = game.data.entities.fighter.get(obj_id) {
                y_pos += 1;

                let health_percent = fighter.hp as f32 / fighter.max_hp as f32;

                render_bar(panel,
                           display_state,
                           health_percent,
                           y_pos,
                           game.config.color_red,
                           Color::white());

                y_pos += 2;
            }

            text_list.push(format!("{:?}", game.data.entities.name[obj_id]));

            text_list.push(format!(""));

            // show facing direction for player and monsters
            if game.data.entities.typ[obj_id] == EntityType::Player ||
               game.data.entities.typ[obj_id] == EntityType::Enemy {
                if let Some(direction) = game.data.entities.direction.get(obj_id) {
                    text_list.push(format!("Facing"));
                    text_list.push(format!("  {}", direction));
                }
            }

            if game.data.entities.fighter.get_mut(obj_id).map_or(false, |fighter| fighter.hp <= 0) {
                text_list.push(format!("  {}", "dead"));
            } else if let Some(behave) = game.data.entities.behavior.get(obj_id) {
                text_list.push(format!("{}", behave.description()));
            }
        }
    }

    let tile_sprite = &mut display_state.sprites[&sprite_key];
    let text_pos = Pos::new(1, y_pos);
    tile_sprite.draw_text_list(panel, &text_list, text_pos, text_color);
    text_list.push(format!(""));
    text_list.clear();

    y_pos = 11;
    if in_fov {
        let text_pos = Pos::new(1, y_pos);
        text_list.push(format!("Tile is"));
        if game.data.map[info_pos].tile_type == TileType::Water {
            text_list.push("water".to_string());
        } else {
            text_list.push(format!("{:?}",  game.data.map[info_pos].surface));
        }
        if game.data.map[info_pos].bottom_wall != Wall::Empty {
            text_list.push("Lower wall".to_string());
        }

        if game.data.map[info_pos].left_wall != Wall::Empty {
            text_list.push("Left wall".to_string());
        }

        if game.data.map[info_pos].block_move {
            text_list.push(format!("blocked"));
        }

        tile_sprite.draw_text_list(panel, &text_list, text_pos, text_color);
    }
}

fn render_skill_menu(panel: &mut Panel<&mut WindowCanvas>, display_state: &mut DisplayState, game: &mut Game) {
    let player_id = game.data.find_by_name(EntityName::Player).unwrap();

    // Render header
    render_placard(panel, display_state, "Skills", &game.config);

    let mut list = Vec::new();

    for (index, skill) in game.data.entities.skills[&player_id].iter().enumerate() {
        list.push(format!("{} {:?}", index, skill));
    }

    let y_pos = 2;
    let text_pos = Pos::new(2, y_pos);
    let color = game.config.color_light_grey;

    let sprite_key = display_state.lookup_spritekey("tiles");
    let tile_sprite = &mut display_state.sprites[&sprite_key];

    tile_sprite.draw_text_list(panel, &list, text_pos, color);
}

fn render_class_menu(panel: &mut Panel<&mut WindowCanvas>, display_state: &mut DisplayState, game: &mut Game) {
    // Render header
    render_placard(panel, display_state, "Choice Class", &game.config);

    let mut list = Vec::new();

    for (index, class) in EntityClass::classes().iter().enumerate() {
        list.push(format!("{} {:?}", index, class));
    }

    let y_pos = 2;
    let text_pos = Pos::new(2, y_pos);
    let color = game.config.color_light_grey;

    let sprite_key = display_state.lookup_spritekey("tiles");
    let tile_sprite = &mut display_state.sprites[&sprite_key];

    tile_sprite.draw_text_list(panel, &list, text_pos, color);
}

fn render_confirm_quit(panel: &mut Panel<&mut WindowCanvas>, display_state: &mut DisplayState, game: &mut Game) {
    // Render header
    render_placard(panel, display_state, "Quit?", &game.config);

    let mut list = Vec::new();

    list.push("q: quit game".to_string());
    list.push("".to_string());
    list.push("esc: continue".to_string());

    let y_pos = 2;
    let text_pos = Pos::new(2, y_pos);
    let color = game.config.color_light_grey;

    let sprite_key = display_state.lookup_spritekey("tiles");
    let tile_sprite = &mut display_state.sprites[&sprite_key];

    tile_sprite.draw_text_list(panel, &list, text_pos, color);
}

/// Render an inventory section within the given area
fn render_inventory(panel: &mut Panel<&mut WindowCanvas>, display_state: &mut DisplayState, game: &mut Game) {
    // Render header
    render_placard(panel, display_state, "Inventory", &game.config);

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();

    let sprite_key = display_state.lookup_spritekey("tiles");
    let tile_sprite = &mut display_state.sprites[&sprite_key];

    // Render each object's name in inventory
    let mut y_pos = 2;
    let mut item_index = 0;
    let item_ids = game.data.entities.inventory[&player_id].clone();

    for (index, obj_id) in item_ids.iter().enumerate() {
        let color;
        match &game.data.entities.item.get(obj_id) {
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
        tile_sprite.draw_char(panel,
                              ('0' as u8 + item_index) as char,
                              Pos::new(1, y_pos),
                              game.config.color_ice_blue);

        // place object name
        let text_pos = Pos::new(2, y_pos);
        let item_marker =
            if index == 0 && item_primary_at(player_id, &mut game.data.entities, 0) {
                "<"
            } else {
                ""
            };
        let item_text = format!(" {:?} {}", game.data.entities.name[obj_id], item_marker);
        tile_sprite.draw_text(panel, &item_text, text_pos, color);
        
        y_pos += 1;

        item_index += 1;
    }

    if game.data.entities.inventory[&player_id].len() == 0 {
        let text_pos = Pos::new(1, y_pos);
        tile_sprite.draw_text(panel, &format!("empty"), text_pos, game.config.color_ice_blue);
    }
}

/// render the background files, including water tiles
fn render_background(display: &mut Display, game: &mut Game) {
    if !display.targets.background_panel.dirty {
        return;
    }
    display.targets.background_panel.dirty = false;

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();

    let (map_width, map_height) = game.data.map.size();

    let sprite_key = display.state.lookup_spritekey("tiles");
    let sprite = &mut display.state.sprites[&sprite_key];

    let canvas = &mut display.targets.canvas_panel.target;

    let panel = display.targets.background_panel.unit();
    canvas.with_texture_canvas(&mut display.targets.background_panel.target, |canvas| {
        canvas.set_draw_color(Sdl2Color::RGB(0, 0, 0));
        canvas.clear();

        let mut panel = panel.with_target(canvas);
        for y in 0..map_height {
            for x in 0..map_width {
                let map_pos = Pos::new(x, y);

                let visible =
                    game.data.is_in_fov(player_id, map_pos, &game.config) ||
                    game.settings.god_mode;

                let tile = &game.data.map[(x, y)];
                if tile.tile_type != TileType::Water {
                    sprite.draw_char(&mut panel,
                                     MAP_EMPTY_CHAR as char,
                                     map_pos,
                                     empty_tile_color(&game.config, map_pos, visible));
                } else {
                    let color = tile_color(&game.config, x, y, tile, visible);
                    let chr = tile.chr;
                    sprite.draw_char(&mut panel, MAP_EMPTY_CHAR as char, map_pos, color);
                }
            }
        }
    }).unwrap();
}

/// Render the map, with environment and walls
fn render_map(panel: &mut Panel<&mut WindowCanvas>, display_state: &mut DisplayState, game: &mut Game) {
    let player_id = game.data.find_by_name(EntityName::Player).unwrap();

    let (map_width, map_height) = game.data.map.size();

    let sprite_key = display_state.lookup_spritekey("tiles");
    let sprite = &mut display_state.sprites[&sprite_key];

    for y in 0..map_height {
        for x in 0..map_width {
            let pos = Pos::new(x, y);

            // draw an outline around the tile
            {
                let mut outline_color = Color::white();
                outline_color.a /= 8;
                draw_outline_tile(panel, pos, outline_color);
            }

            // Render game stuff
            let visible =
                game.data.is_in_fov(player_id, pos, &game.config) ||
                game.settings.god_mode;

            let explored = game.data.map[pos].explored || visible;

            let tile = &game.data.map[pos];

            let wall_color =
                if explored {
                    game.config.color_light_brown
                } else {
                    game.config.color_dark_brown
                };

            let chr = tile.chr;

            // if the tile is not empty or water, draw it
            let color = tile_color(&game.config, x, y, tile, visible);
            if tile.tile_type == TileType::Water {
                sprite.draw_char(panel, ' ', pos, color);
            } else if chr != MAP_EMPTY_CHAR {
                sprite.draw_char(panel, chr as char, pos, color);
            }

            match tile.surface {
                Surface::Rubble => {
                    sprite.draw_char(panel, MAP_RUBBLE as char, pos, color);
                }

                Surface::Grass => {
                    sprite.draw_char(panel, MAP_RUBBLE as char, pos, game.config.color_light_green);
                }

                Surface::Floor => {
                    // Nothing to draw
                }
            }

            // finally, draw the between-tile walls appropriate to this tile
            if tile.bottom_wall == Wall::ShortWall {
                sprite.draw_char(panel, MAP_THIN_WALL_BOTTOM as char, pos, wall_color);
            } else if tile.bottom_wall == Wall::TallWall {
                sprite.draw_char(panel, MAP_THICK_WALL_BOTTOM as char, pos, wall_color);
            }

            if tile.left_wall == Wall::ShortWall {
                sprite.draw_char(panel, MAP_THIN_WALL_LEFT as char, pos, wall_color);

            } else if tile.left_wall == Wall::TallWall {
                sprite.draw_char(panel, MAP_THICK_WALL_LEFT as char, pos, wall_color);
            }

            if x + 1 < map_width {
                let right_pos = Pos::new(pos.x + 1, pos.y);
                let right_tile = &game.data.map[right_pos];
                if right_tile.left_wall == Wall::ShortWall {
                sprite.draw_char(panel, MAP_THIN_WALL_RIGHT as char, pos, wall_color);
                } else if right_tile.left_wall == Wall::TallWall {
                sprite.draw_char(panel, MAP_THICK_WALL_RIGHT as char, pos, wall_color);
                }
            }

            if y - 1 >= 0 {
                let up_pos = Pos::new(pos.x, pos.y - 1);
                let up_tile = &game.data.map[up_pos];
                if up_tile.bottom_wall == Wall::ShortWall {
                    sprite.draw_char(panel, MAP_THIN_WALL_TOP as char, pos, wall_color);
                } else if up_tile.bottom_wall == Wall::TallWall {
                    sprite.draw_char(panel, MAP_THICK_WALL_TOP as char, pos, wall_color);
                }
            }

            // apply a FoW darkening to cells
            if game.config.fog_of_war && !visible {
                let mut blackout_color = Color::black();
                if game.data.map[pos].explored {
                    blackout_color.a = game.config.explored_alpha
                }
                
                sprite.draw_char(panel, MAP_EMPTY_CHAR as char, pos, blackout_color);
            }
        }
    }
}

/// Render each effect currently playing in the game
/// The strategy here is to copy the effects vector, update all items,
/// and then remove finished effects from back to front. The
/// resulting vector of effects is then saved as the new effects vector.
fn render_effects(panel: &mut Panel<&mut WindowCanvas>,
                  display_state: &mut DisplayState,
                  game: &mut Game) {
    let mut remove_indices = Vec::new();

    let mut effects = display_state.effects.clone();

    let sprite_key = display_state.lookup_spritekey("tiles");
    let sprite = &mut display_state.sprites[&sprite_key];

    for (index, effect) in effects.iter_mut().enumerate() {
        match effect {
            Effect::HeardSomething(pos, created_turn) => {
                sprite.draw_char(panel,
                                 ENTITY_ELF as char,
                                 *pos,
                                 game.config.color_warm_grey);

                if *created_turn != game.settings.turn_count {
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

                    // NOTE(perf) with the new texture system, consider rendering a series of sound
                    //            tiles and simply pasting them, perhaps saving time from not
                    //            needing to blend.
                    for pos in dist_positions.iter() {
                        if !game.data.map[*pos].block_move {
                           draw_tile_highlight(panel, *pos, highlight_color);
                        }
                    }
                }

                if *sound_dt >= game.config.sound_timeout {
                    remove_indices.push(index);
                } else {
                    *sound_dt += 1.0 / game.config.frame_rate as f32;
                    if *sound_dt > game.config.sound_timeout {
                        *sound_dt = game.config.sound_timeout;
                    }
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

fn render_entity(panel: &mut Panel<&mut WindowCanvas>,
                 entity_id: EntityId,
                 display_state: &mut DisplayState,
                 game: &mut Game) -> Option<Sprite> {
    let mut animation_result = AnimationResult::new();

    let pos = game.data.entities.pos[&entity_id];
    let player_id = game.data.find_by_name(EntityName::Player).unwrap();

    // only draw if within the map (outside is (-1, -1) like if in inventory)
    // and not in limbo.
    if game.data.map.is_within_bounds(pos) &&
       game.data.entities.limbo.get(&entity_id).is_none() {
        let is_in_fov = 
           game.data.is_in_fov(player_id, pos, &game.config) ||
           game.settings.god_mode;

        if let Some(anim_key) = game.data.entities.animation[&entity_id].get(0) {
            animation_result = 
                render_animation(*anim_key,
                                 entity_id,
                                 is_in_fov,
                                 panel,
                                 display_state,
                                 &mut game.data,
                                 &game.settings,
                                 &game.config);

            if animation_result.done {
                game.data.entities.animation[&entity_id].pop_front();
            }
        } else {
            // NOTE the needs_removal can probably be removed
            let needs_removal = game.data.entities.needs_removal[&entity_id];
            if is_in_fov && !needs_removal {
                let color = game.data.entities.color[&entity_id];

                let chr = game.data.entities.chr[&entity_id];
                let sprite = Sprite::char(chr);
                display_state.draw_sprite(panel, sprite, pos, color);
                animation_result.sprite = Some(sprite);
            }
        }
    }

    return animation_result.sprite;
}

fn render_impressions(panel: &mut Panel<&mut WindowCanvas>, display_state: &mut DisplayState, game: &mut Game) {
    // check for entities that have left FOV and make an impression for them
    for impression in display_state.impressions.clone() {
        display_state.draw_sprite(panel, impression.sprite, impression.pos, game.config.color_light_grey);
    }
}

fn render_order(typ: EntityType) -> usize {
    let order = &[ EntityType::Other, EntityType::Trigger,
                   EntityType::Column, EntityType::Item, EntityType::Energy,
                   EntityType::Enemy, EntityType::Player];
    return order.iter().position(|t| *t == typ).expect(&format!("Entity type {:?} not expected!", typ));
}

/// Render each object in the game, filtering for objects not currently visible
fn render_entities(panel: &mut Panel<&mut WindowCanvas>, display_state: &mut DisplayState, game: &mut Game) {
    let player_id = game.data.find_by_name(EntityName::Player).unwrap();

    display_state.drawn_sprites.clear();

    let mut ids = game.data.entities.ids.clone();

    let compare_render_order = |id0: &EntityId, id1: &EntityId| -> Ordering {
        let order0 = render_order(game.data.entities.typ[id0]);
        //dbg!(game.data.entities.pos[id1]);
        //dbg!(game.data.entities.name[id1]);
        let order1 = render_order(game.data.entities.typ[id1]);
        return order0.cmp(&order1);
    };

    ids.sort_unstable_by(compare_render_order);

    for entity_id in ids.iter() {
        let maybe_sprite = render_entity(panel, *entity_id, display_state, game);

        if let Some(sprite) = maybe_sprite {
            display_state.drawn_sprites.insert(*entity_id, sprite);
        }
    }
}

fn render_animation(anim_key: AnimKey,
                    entity_id: EntityId,
                    is_in_fov: bool,
                    panel: &mut Panel<&mut WindowCanvas>,
                    display_state: &mut DisplayState,
                    data: &mut GameData,
                    settings: &GameSettings,
                    config: &Config) -> AnimationResult {

    let pos = data.entities.pos[&entity_id];
    let mut color = data.entities.color[&entity_id];

    // TODO should also freeze animation or leave at first element to indicate disarmed trap
    if data.entities.armed.get(&entity_id) == Some(&false) {
        color = config.color_warm_grey;
    }

    let mut animation_result: AnimationResult = AnimationResult::new();
    match display_state.animations[&anim_key].clone() {
        Animation::Between(ref mut sprite_anim, start, end, ref mut dist, blocks_per_sec) => {
           if settings.god_mode || is_in_fov {
               *dist = *dist + (blocks_per_sec / config.frame_rate as f32); 
               let num_blocks = *dist as usize;

               let draw_pos = move_towards(start, end, num_blocks);

               let sprite = sprite_anim.sprite();
               display_state.draw_sprite(panel,
                                         sprite,
                                         draw_pos,
                                         color);
               animation_result.sprite = Some(sprite);

               display_state.animations[&anim_key] =
                   Animation::Between(*sprite_anim, start, end, *dist, blocks_per_sec);

               animation_result.done = *dist >= distance(start, end) as f32;
           }
        }

        Animation::Loop(ref mut sprite_anim) => {
           if settings.god_mode || is_in_fov {
                let sprite = sprite_anim.sprite();
                display_state.draw_sprite(panel,
                                          sprite,
                                          pos,
                                          color);
                animation_result.sprite = Some(sprite);

                display_state.animations[&anim_key] =
                   Animation::Loop(*sprite_anim);

                // a looping animation never finishes
                animation_result.done = false;
            }
        }

        Animation::PlayEffect(effect) => {
            display_state.play_effect(effect);
            animation_result.done = true;

            // NOTE the sprite is not updated here- this may cause entity impressions to not work
            // in edge cases where an effect is playing.
        }

        Animation::Once(ref mut sprite_anim) => {
           if settings.god_mode || is_in_fov {
                if !sprite_anim.looped {
                    let sprite = sprite_anim.sprite();
                    display_state.draw_sprite(panel, sprite, pos, color);
                    animation_result.sprite = Some(sprite);

                    display_state.animations[&anim_key] =
                       Animation::Once(*sprite_anim);
                }

                animation_result.done = sprite_anim.looped;
            }
        }
    }

    return animation_result;
}

fn render_overlays(panel: &mut Panel<&mut WindowCanvas>,
                   display_state: &mut DisplayState,
                   game: &mut Game,
                   map_mouse_pos: Option<Pos>) {
    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    let player_pos = game.data.entities.pos[&player_id];

    let sprite_key = display_state.lookup_spritekey("tiles");

    // render a grid of numbers if enabled
    if game.config.overlay_directions {
        let tile_sprite = &mut display_state.sprites[&sprite_key];

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
                        tile_sprite.draw_char(panel, MAP_GROUND as char, pos, game.config.color_light_green);
                    } else {
                        tile_sprite.draw_char(panel, MAP_GROUND as char, pos, game.config.color_light_grey);
                    }
                    tile_sprite.draw_char(panel, ('0' as u8 + res.abs() as u8) as char, pos, game.config.color_red);
                }
            }
        }
    }

    // render cursor if enabled
    if game.config.use_cursor {
        // render cursor itself
        let cursor_pos = game.settings.cursor_pos;
        let tile_sprite = &mut display_state.sprites[&sprite_key];
        let mut color = game.config.color_mint_green;
        color.a = 230;
        tile_sprite.draw_char(panel, ENTITY_CURSOR as char, cursor_pos, color);

        // render player ghost
        if cursor_pos != player_pos && game.input.target == -1 {
            let alpha = game.data.entities.color[&player_id].a;
            game.data.entities.color[&player_id].a = 100;

            let dxy = sub_pos(cursor_pos, player_pos);
            let direction = Direction::from_dxy(dxy.x, dxy.y).unwrap();

            let mut reach = reach_by_mode(MoveMode::Sneak);
            if game.input.mode == ActionMode::Alternate {
                reach = reach_by_mode(MoveMode::Run);
            }

            if let Some(player_ghost_pos) = reach.furthest_in_direction(player_pos, direction) {
                game.data.entities.pos[&player_id] = player_ghost_pos;
                render_entity(panel, player_id, display_state, game);
                game.data.entities.color[&player_id].a = alpha;
                game.data.entities.pos[&player_id] = player_pos;
            }
        }
    }

    // render FOV if enabled
    if game.config.overlay_player_fov {
        let tile_sprite = &mut display_state.sprites[&sprite_key];

        let map_width = game.data.map.width();
        let map_height = game.data.map.height();
        for y in 0..map_height {
            for x in 0..map_width {
                let pos = Pos::new(x, y);

                let is_in_fov =
                    game.data.is_in_fov(player_id, pos, &game.config);
                if is_in_fov {
                    tile_sprite.draw_char(panel, MAP_GROUND as char, pos, game.config.color_light_green);
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

    {
        let tile_sprite = &mut display_state.sprites[&sprite_key];
        for entity_id in game.data.entities.ids.iter().map(|id| *id).collect::<Vec<EntityId>>().iter() {
            let pos = game.data.entities.pos[entity_id];

            if pos.x == -1 && pos.y == -1 {
                continue;
            }

            if game.data.is_in_fov(player_id, pos, &game.config) &&
               game.data.entities.status[entity_id].alive {
                if let Some(dir) = game.data.entities.direction.get(entity_id) {
                    // display.draw_tile_edge(pos, area, direction_color, dir);

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

                    tile_sprite.draw_sprite_at_cell(panel,
                                                    ARROW_RIGHT as usize,
                                                    pos,
                                                    direction_color,
                                                    rotation);
                }
            }
        }
    }

    // draw attack and fov position highlights
    if let Some(mouse_xy) = map_mouse_pos {
        // Draw monster attack overlay
        let object_ids = get_entity_at_pos(mouse_xy, &mut game.data);
        for entity_id in object_ids.iter() {
            let pos = game.data.entities.pos[entity_id];

            if game.data.is_in_fov(player_id, pos, &game.config) &&
               *entity_id != player_id &&
               game.data.entities.status[entity_id].alive {
               render_attack_overlay(panel, display_state, game, *entity_id);
               render_fov_overlay(panel, display_state, game, *entity_id);
               render_movement_overlay(panel, display_state, game, *entity_id);
            }
        }
    }

    // render attack overlay highlighting squares that an entity can attack
    if game.settings.overlay {
        let keys = game.data.entities.ids.iter().map(|id| *id).collect::<Vec<EntityId>>();
        for entity_id in keys {
            let pos = game.data.entities.pos[&entity_id];

            if entity_id != player_id &&
               game.data.map.is_within_bounds(pos) &&
               game.data.is_in_fov(player_id, pos, &game.config) &&
               game.data.entities.status[&entity_id].alive {
               render_attack_overlay(panel,
                                     display_state,
                                     game,
                                     entity_id);
            }
        }
    }

    // draw mouse path overlays
    if let Some(mouse_id) = game.data.find_by_name(EntityName::Mouse) {
        let tile_sprite = &mut display_state.sprites[&sprite_key];

        let mouse_pos = game.data.entities.pos[&mouse_id];
        let player_pos = game.data.entities.pos[&player_id];

        if game.config.draw_star_path {
            // get a path to the mouse path, regardless of distance
            let path = astar_path(&game.data.map, player_pos, mouse_pos, None, None);
            for pos in path {
                tile_sprite.draw_char(panel, MAP_EMPTY_CHAR as char, pos, highlight_color);
            }
        }

        if game.config.draw_mouse_line {
            // mouse pos at 0, 0 occurs when the mouse has not moved since startup.
            // this may cause a weirdness on the corner of the map
            if mouse_pos != Pos::new(0, 0) {
                let line = line(player_pos, mouse_pos).into_iter();
                for pos in line {
                    let pos = Pos::from(pos);
                    tile_sprite.draw_char(panel, MAP_EMPTY_CHAR as char, pos, highlight_color);
                }
            }
        }

        if game.settings.draw_selection_overlay {
            // mouse pos at 0, 0 occurs when the mouse has not moved since startup.
            // this may cause a weirdness on the corner of the map
            if mouse_pos != Pos::new(0, 0) {
                let selected_pos =
                    game.settings.selection.selected_pos(player_pos,
                                                         mouse_pos,
                                                         game.config.fov_radius_player,
                                                         &mut game.data, &game.config);

                if let Some(pos) = selected_pos {
                    tile_sprite.draw_char(panel, MAP_EMPTY_CHAR as char, pos, highlight_color);
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
                               game.data.entities.movement[&player_id],
                               player_id,
                               &mut game.data) {
                // draw a highlight on that square
                // don't draw overlay on top of character
                if movement.pos != game.data.entities.pos[&player_id] {
                    let dxy = sub_pos(movement.pos, player_pos);
                    let direction = Direction::from_dxy(dxy.x, dxy.y).unwrap();
                    let shadow_cursor_pos = direction.offset_pos(player_pos, 1);

                    render_entity_at(player_id, shadow_cursor_pos, game, panel, display_state);
                }
            }
        }
    }

    // Draw sound tiles overlay
    if game.settings.overlay {
        // NOTE this currently does not take into account FOV!
        for pos in display_state.sound_tiles.iter() {
            draw_tile_highlight(panel, *pos, highlight_color);
        }
    }

    // Outline tiles within FOV for clarity
    if game.settings.overlay {
        let mut highlight_color_fov = game.config.color_light_orange;
        highlight_color_fov.a = game.config.grid_alpha_visible;

        for y in 0..game.data.map.height() {
            for x in 0..game.data.map.width() {
                let pos = Pos::new(x, y);
                let in_fov = game.data.is_in_fov(player_id, pos, &game.config);
                if in_fov {
                    draw_outline_tile(panel, pos, highlight_color_fov);
                }
            }
        }
    }

    // NOTE floodfill ranges:
    // 4-5 is the end of a cooridor
    // < 14 is pretty enclosed
    // 14-24 is within the corner of a building or at an opening from an enclosed space
    // low 30s tend to be next to a shear wall
    // 40 are nearly fully open
    // 49 may be fully open
    if game.config.overlay_floodfill {
        let font_key = display_state.lookup_spritekey("font");

        let mut highlight_color = game.config.color_light_orange;
        highlight_color.a = 50;
        let fill_metric = map_fill_metric(&game.data.map);

        for (pos, near_count) in fill_metric {
            let amount = near_count as f32 / 50.0;
            let adj_color = lerp_color(game.config.color_ice_blue, game.config.color_red, amount);

            let tile_sprite = &mut display_state.sprites[&sprite_key];
            tile_sprite.draw_char(panel, MAP_EMPTY_CHAR as char, pos, adj_color);

            let font_sprite = &mut display_state.sprites[&font_key];
            font_sprite.draw_text(panel,
                                  &format!("{}", near_count),
                                  pos,
                                  highlight_color);
        }
    }
}

fn get_entity_at_pos(check_pos: Pos, data: &mut GameData) -> Vec<EntityId> {
    let mut object_ids: Vec<EntityId> = Vec::new();

    for key in data.entities.ids.iter() {
        let pos = data.entities.pos[key];
        let is_mouse = data.entities.name[key] == EntityName::Mouse;
        let removing = data.entities.needs_removal[key];

        if !removing && !is_mouse && check_pos == pos {
            object_ids.push(*key);
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

fn render_attack_overlay(panel: &mut Panel<&mut WindowCanvas>,
                         display_state: &mut DisplayState,
                         game: &mut Game,
                         entity_id: EntityId) {
    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    let player_pos = game.data.entities.pos[&player_id];

    let object_pos = game.data.entities.pos[&entity_id];

    let mut attack_highlight_color = game.config.color_red;
    attack_highlight_color.a = game.config.highlight_alpha_attack;

    let sprite_key = display_state.lookup_spritekey("tiles");
    let tile_sprite = &mut display_state.sprites[&sprite_key];

    if let Some(reach) = game.data.entities.attack.get(&entity_id) {
        let attack_positions = 
            reach.offsets()
                 .iter()
                 .map(|offset| Pos::new(object_pos.x as i32 + offset.x,
                                        object_pos.y as i32 + offset.y))
                 // filter out positions that are outside of the map, or with no clear
                 // path from the entity to the reached position
                 .filter(|pos| {
                     let in_bounds = game.data.map.is_within_bounds(*pos);
                     let traps_block = false;
                     let clear = game.data.clear_path(object_pos, *pos, traps_block);
                     let player_can_see = game.data.is_in_fov(player_id, *pos, &game.config);
                     // check for player position so it gets highligted, even
                     // though the player causes 'clear_path' to fail.
                     return player_can_see && in_bounds && (clear || *pos == player_pos);
                 })
                 .collect::<Vec<Pos>>();

        for position in attack_positions {
            tile_sprite.draw_char(panel, MAP_EMPTY_CHAR as char, position, attack_highlight_color);
        }
    }
}

fn render_fov_overlay(panel: &mut Panel<&mut WindowCanvas>,
                      _display_state: &mut DisplayState,
                      game: &mut Game,
                      entity_id: EntityId) {
    let player_id = game.data.find_by_name(EntityName::Player).unwrap();

    let mut highlight_color = game.config.color_light_grey;
    highlight_color.a = game.config.grid_alpha_overlay;

    for y in 0..game.data.map.height() {
        for x in 0..game.data.map.width() {
            let map_pos = Pos::new(x, y);

            let visible = game.data.is_in_fov(entity_id, map_pos, &game.config) &&
                          game.data.is_in_fov(player_id, map_pos, &game.config);


            if visible {
                draw_outline_tile(panel, map_pos, highlight_color);
            }
        }
    }
}

fn render_movement_overlay(panel: &mut Panel<&mut WindowCanvas>,
                           display_state: &mut DisplayState,
                           game: &mut Game,
                           entity_id: EntityId) {
    let player_id = game.data.find_by_name(EntityName::Player).unwrap();

    let entity_pos = game.data.entities.pos[&entity_id];

    let mut highlight_color = game.config.color_light_grey;
    highlight_color.a = game.config.grid_alpha_overlay;

    let sprite_key = display_state.lookup_spritekey("tiles");
    let tile_sprite = &mut display_state.sprites[&sprite_key];

    if let Some(reach) = game.data.entities.movement.get(&entity_id) {
        for move_pos in reach.reachables(entity_pos) {
            let visible = game.data.is_in_fov(player_id, move_pos, &game.config);
            if visible {
                let chr = game.data.entities.chr[&entity_id];

                tile_sprite.draw_char(panel, chr as char, move_pos, highlight_color);
            }
        }
    }
}

pub fn sdl2_color(color: Color) -> Sdl2Color {
    return Sdl2Color::RGBA(color.r, color.g, color.b, color.a);
}


pub fn render_entity_at(entity_id: EntityId, render_pos: Pos, game: &mut Game, panel: &mut Panel<&mut WindowCanvas>, display_state: &mut DisplayState) {
    let entity_pos = game.data.entities.pos[&entity_id];

    let alpha = game.data.entities.color[&entity_id].a;
    game.data.entities.color[&entity_id].a = 200;

    game.data.entities.pos[&entity_id] = render_pos;

    render_entity(panel, entity_id, display_state, game);

    game.data.entities.color[&entity_id].a = alpha;
    game.data.entities.pos[&entity_id] = entity_pos;
}

