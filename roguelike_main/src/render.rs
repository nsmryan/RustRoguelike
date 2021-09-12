use std::collections::HashSet;

use oorandom::Rand32;

use sdl2::render::{BlendMode, WindowCanvas};
use sdl2::rect::Rect;
use sdl2::pixels::{Color as Sdl2Color};

use roguelike_core::types::*;
use roguelike_core::map::*;
use roguelike_core::constants::*;
use roguelike_core::movement::*;
use roguelike_core::config::*;
use roguelike_core::utils::{item_primary_at, lerp_color, sub_pos, reach_by_mode, map_fill_metric};
use roguelike_core::perlin::Perlin;
use roguelike_core::line::line;
use roguelike_core::ai::*;

use roguelike_engine::game::*;

use crate::display::*;
use crate::animation::{Sprite, Effect, Animation, AnimationResult};


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


fn render_panels(display: &mut Display, game: &mut Game, _map_rect: Rect) {
    let mouse_map_pos = game.settings.cursor;

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

            render_entity_type(EntityType::Item, &mut panel, display_state, game);
            render_entity_type(EntityType::Trigger, &mut panel, display_state, game);
            render_map(&mut panel, display_state, game);
            render_entity_type(EntityType::Energy, &mut panel, display_state, game);
            render_entity_type(EntityType::Enemy, &mut panel, display_state, game);
            render_entity_type(EntityType::Column, &mut panel, display_state, game);
            render_entity_type(EntityType::Player, &mut panel, display_state, game);
            render_entity_type(EntityType::Other, &mut panel, display_state, game);
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

    list.push(format!(" ({}, {})", 
                      game.data.entities.pos[&player_id].x,
                      game.data.entities.pos[&player_id].y));

    list.push(format!(""));

    let move_mode = game.data.entities.move_mode[&player_id];
    list.push(format!("{}", move_mode.to_string()));

    let stance = game.data.entities.stance[&player_id];
    list.push(format!("{}", stance));

    list.push(format!(""));
    if let Some(class) = game.data.entities.class.get(&player_id) {
        list.push(format!("{}", class));
    } else {
        list.push(format!(""));
    }

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
               _mouse_xy: Option<Pos>) {
    render_placard(panel, display_state, "Info", &game.config);

    if let Some(info_pos) = game.settings.cursor {
        let text_color = game.config.color_soft_green;

        let player_id = game.data.find_by_name(EntityName::Player).unwrap();

        let object_ids = game.data.get_entities_at_pos(info_pos);

        let mut y_pos = 1;

        let mut text_list = Vec::new();

        text_list.push(format!("({:>2},{:>2})", info_pos.x, info_pos.y));

        let text_pos = Pos::new(1, y_pos);

        let sprite_key = display_state.lookup_spritekey("tiles");

        {
            let tile_sprite = &mut display_state.sprites[&sprite_key];
            tile_sprite.draw_text_list(panel, &text_list, text_pos, text_color);
        }
        text_list.clear();

        y_pos += 1;

        // only display first object
        if let Some(obj_id) = object_ids.first() {
            let entity_in_fov = game.settings.god_mode ||
                                game.data.is_in_fov(player_id, *obj_id, &game.config);

            // only display things in the player's FOV
            if entity_in_fov {
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

        let tile_in_fov = game.data.pos_in_fov(player_id, info_pos, &game.config);
        if tile_in_fov {
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
    render_placard(panel, display_state, "Choose Class", &game.config);

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

    let mut index = 0;
    while index < game.data.entities.inventory[&player_id].len() {
        let obj_id = game.data.entities.inventory[&player_id][index];
        index += 1;

        let color;
        match &game.data.entities.item.get(&obj_id) {
            Some(Item::Stone) => {
                color = game.config.color_light_grey;
            }

            Some(Item::Key) => {
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
        let item_text = format!(" {:?} {}", game.data.entities.name[&obj_id], item_marker);
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

                //let visible =
                //    game.data.pos_in_fov(player_id, map_pos, &game.config) ||
                //    game.settings.god_mode;

                let tile = &game.data.map[(x, y)];
                if tile.tile_type != TileType::Water {
                    sprite.draw_char(&mut panel,
                                     MAP_EMPTY_CHAR as char,
                                     map_pos,
                                     Color::white());
                } else {
                    sprite.draw_char(&mut panel, MAP_EMPTY_CHAR as char, map_pos, Color::white());
                }
            }
        }
    }).unwrap();
}

fn render_surface(panel: &mut Panel<&mut WindowCanvas>, sprite: &mut SpriteSheet, surface: Surface, pos: Pos) {
    match surface {
        Surface::Rubble => {
            sprite.draw_char(panel, MAP_RUBBLE as char, pos, Color::white());
        }

        Surface::Grass => {
            sprite.draw_char(panel, MAP_GRASS as char, pos, Color::white()); //game.config.color_light_green);
        }

        Surface::Floor => {
            // Nothing to draw
        }
    }
}

/// Render Wall Shadows (full tile and intertile walls, left and down)
fn render_wall_shadow(pos: Pos, panel: &mut Panel<&mut WindowCanvas>, display_state: &mut DisplayState, game: &mut Game) {
    let shadow_sprite_key = display_state.lookup_spritekey("shadows");

    let tile = game.data.map[pos];

    let (_map_width, map_height) = game.data.map.size();
    let (x, y) = pos.to_tuple();

    let left_valid = x - 1 > 0;
    let down_valid = y + 1 < map_height;
    let down_left_valid = left_valid && down_valid;
    let left_wall = left_valid && game.data.map[(x - 1, y)].tile_type == TileType::Wall;
    let down_wall = down_valid && game.data.map[(x, y + 1)].tile_type == TileType::Wall;
    let down_left_wall = down_left_valid && game.data.map[(x - 1, y + 1)].tile_type == TileType::Wall;

    /* render full tile wall shadows */
    if tile.tile_type == TileType::Wall {
        if left_valid && !left_wall {
            // left
            let shadow_pos = Pos::new(x - 1, y);
            let shadow_left_upper = Sprite::new(SHADOW_FULLTILE_LEFT as u32, shadow_sprite_key);
            display_state.draw_sprite(panel, shadow_left_upper, shadow_pos, game.config.color_shadow);
        }

        if down_left_valid && !down_left_wall {
            let shadow_pos = Pos::new(x - 1, y + 1);
            let shadow_left_lower = Sprite::new(SHADOW_FULLTILE_LEFT_DOWN as u32, shadow_sprite_key);
            display_state.draw_sprite(panel, shadow_left_lower, shadow_pos, game.config.color_shadow);
        }

        if down_valid && !down_wall {
            // lower
            let shadow_lower_right = Sprite::new(SHADOW_FULLTILE_DOWN as u32, shadow_sprite_key);
            let shadow_pos = Pos::new(x, y + 1);
            display_state.draw_sprite(panel, shadow_lower_right, shadow_pos, game.config.color_shadow);
        }

        if down_left_valid && !down_left_wall {
            let shadow_lower_left = Sprite::new(SHADOW_FULLTILE_DOWN_LEFT as u32, shadow_sprite_key);
            let shadow_pos = Pos::new(x - 1, y + 1);
            display_state.draw_sprite(panel, shadow_lower_left, shadow_pos, game.config.color_shadow);
        }
    } else if tile.left_wall == Wall::ShortWall {
        // left
        if left_valid {
            let shadow_pos = Pos::new(x - 1, y);
            let shadow_left_upper = Sprite::new(SHADOW_INTERTILE_LEFT as u32, shadow_sprite_key);
            display_state.draw_sprite(panel, shadow_left_upper, shadow_pos, game.config.color_shadow);
        }

        // left down
        if down_left_valid {
            let shadow_pos = Pos::new(x - 1, y + 1);
            let shadow_left_lower = Sprite::new(SHADOW_INTERTILE_LEFT_DOWN as u32, shadow_sprite_key);
            display_state.draw_sprite(panel, shadow_left_lower, shadow_pos, game.config.color_shadow);
        }
    } else if tile.bottom_wall == Wall::ShortWall {
        if down_valid {
            // lower
            if down_valid {
                let shadow_lower_right = Sprite::new(SHADOW_INTERTILE_DOWN as u32, shadow_sprite_key);
                let shadow_pos = Pos::new(x, y + 1);
                display_state.draw_sprite(panel, shadow_lower_right, shadow_pos, game.config.color_shadow);
            }

            // left down
            if down_left_valid {
                let shadow_lower_left = Sprite::new(SHADOW_INTERTILE_DOWN_LEFT as u32, shadow_sprite_key);
                let shadow_pos = Pos::new(x - 1, y + 1);
                display_state.draw_sprite(panel, shadow_lower_left, shadow_pos, game.config.color_shadow);
            }
        }
    }
}

/// Render the map, with environment and walls
fn render_map(panel: &mut Panel<&mut WindowCanvas>, display_state: &mut DisplayState, game: &mut Game) {
    let player_id = game.data.find_by_name(EntityName::Player).unwrap();

    let (map_width, map_height) = game.data.map.size();

    let sprite_key = display_state.lookup_spritekey("tiles");
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
                game.data.pos_in_fov(player_id, pos, &game.config) ||
                game.settings.god_mode;

            let tile = game.data.map[pos];

            let chr = tile.chr;

            // if the tile is not empty or water, draw it
            if tile.tile_type == TileType::Water {
                let sprite = &mut display_state.sprites[&sprite_key];
                sprite.draw_char(panel, MAP_WATER as char, pos, Color::white());
            } else if chr != MAP_EMPTY_CHAR {
                let sprite = &mut display_state.sprites[&sprite_key];
                sprite.draw_char(panel, chr as char, pos, Color::white());
            }

            {
                let sprite = &mut display_state.sprites[&sprite_key];
                render_surface(panel, sprite, tile.surface, pos);
            }

            render_wall_shadow(pos, panel, display_state, game);

            /* draw the between-tile walls appropriate to this tile */
            {
                let sprite = &mut display_state.sprites[&sprite_key];
                render_itertile_walls(panel, &mut game.data.map, sprite, pos, &game.config);
            }

            // apply a FoW darkening to cells
            if game.config.fog_of_war && !visible {
                game.data.entities.status[&player_id].extra_fov += 1;
                let is_in_fov_ext = 
                   game.data.pos_in_fov(player_id, pos, &game.config);
                game.data.entities.status[&player_id].extra_fov -= 1;

                let mut blackout_color = Color::black();
                if is_in_fov_ext {
                    blackout_color.a = game.config.fov_edge_alpha
                } else if game.data.map[pos].explored {
                    blackout_color.a = game.config.explored_alpha
                }
                
                let sprite = &mut display_state.sprites[&sprite_key];
                sprite.draw_char(panel, MAP_EMPTY_CHAR as char, pos, blackout_color);
            }
        }
    }
}

fn render_itertile_walls(panel: &mut Panel<&mut WindowCanvas>,
                         map: &Map,
                         sprite: &mut SpriteSheet,
                         pos: Pos,
                         _config: &Config) {
    let (x, y) = pos.to_tuple();
    let tile = map[pos];
    let wall_color = Color::white();

    // Lower walls
    if tile.bottom_wall == Wall::ShortWall {
        sprite.draw_char(panel, MAP_THIN_WALL_BOTTOM as char, pos, wall_color);
    } else if tile.bottom_wall == Wall::TallWall {
        sprite.draw_char(panel, MAP_THICK_WALL_BOTTOM as char, pos, wall_color);
    }

    // Left walls
    if tile.left_wall == Wall::ShortWall {
        sprite.draw_char(panel, MAP_THIN_WALL_LEFT as char, pos, wall_color);
    } else if tile.left_wall == Wall::TallWall {
        sprite.draw_char(panel, MAP_THICK_WALL_LEFT as char, pos, wall_color);
    }

    // Right walls
    if x + 1 < map.width() {
        let right_pos = Pos::new(pos.x + 1, pos.y);
        let right_tile = &map[right_pos];
        if right_tile.left_wall == Wall::ShortWall {
            sprite.draw_char(panel, MAP_THIN_WALL_RIGHT as char, pos, wall_color);
        } else if right_tile.left_wall == Wall::TallWall {
            sprite.draw_char(panel, MAP_THICK_WALL_RIGHT as char, pos, wall_color);
        }
    }

    // Upper walls
    if y - 1 >= 0 {
        let up_pos = Pos::new(pos.x, pos.y - 1);
        let up_tile = &map[up_pos];
        if up_tile.bottom_wall == Wall::ShortWall {
            sprite.draw_char(panel, MAP_THIN_WALL_TOP as char, pos, wall_color);
        } else if up_tile.bottom_wall == Wall::TallWall {
            sprite.draw_char(panel, MAP_THICK_WALL_TOP as char, pos, wall_color);
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
    let player_id = game.data.find_by_name(EntityName::Player).unwrap();

    let mut index = 0;
    while index < display_state.effects.len() {
        let mut effect_complete = false;

        let mut effect = display_state.effects[index].clone();
        match &mut effect {
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
                        if !game.data.map[*pos].block_move &&
                           game.data.pos_in_fov(player_id, *pos, &game.config) {
                           draw_tile_highlight(panel, *pos, highlight_color);
                        }
                    }
                }

                if *sound_dt >= game.config.sound_timeout {
                    effect_complete = true;
                } else {
                    *sound_dt += 1.0 / game.config.frame_rate as f32;
                    if *sound_dt > game.config.sound_timeout {
                        *sound_dt = game.config.sound_timeout;
                    }
                }
            }

            Effect::Beam(remaining, start, end) => {
                let sprite_key = display_state.lookup_spritekey("tiles");
                let tile_sprite = &mut display_state.sprites[&sprite_key];

                let dxy = sub_pos(*end, *start);
                let dir = Direction::from_dxy(dxy.x, dxy.y).unwrap();
                let rotation;
                let sprite_index;
                match dir {
                    Direction::Right | Direction::Left => {
                        sprite_index = GOLEM_ATTACK_HORIZ;
                        rotation = 0.0;
                    }

                    Direction::Up | Direction::Down => {
                        sprite_index = GOLEM_ATTACK_HORIZ;
                        rotation = 90.0;
                    }

                    Direction::UpRight | Direction::DownLeft => {
                        rotation = 0.0;
                        sprite_index = GOLEM_ATTACK_DIAG;
                    }

                    Direction::DownRight | Direction::UpLeft => {
                        rotation = 90.0;
                        sprite_index = GOLEM_ATTACK_DIAG;
                    }
                };

                for pos in line(*start, *end) {
                    tile_sprite.draw_sprite_at_cell(panel,
                                                    sprite_index as usize,
                                                    pos,
                                                    Color::white(),
                                                    rotation,
                                                    false,
                                                    false);
                }

                if *remaining == 0 {
                    effect_complete = true;
                } else {
                    *remaining -= 1;
                }
            }
        }
        display_state.effects[index] = effect;

        if effect_complete {
            display_state.effects.swap_remove(index);
        } else {
            index += 1;
        }
    }
}

fn render_entity(panel: &mut Panel<&mut WindowCanvas>,
                 entity_id: EntityId,
                 display_state: &mut DisplayState,
                 game: &mut Game) -> Option<Sprite> {
    let mut animation_result = AnimationResult::new();

    let pos = game.data.entities.pos[&entity_id];
    animation_result.pos = pos;

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();

    // only draw if within the map (outside is (-1, -1) like if in inventory)
    // and not about to be removed.
    if !game.data.map.is_within_bounds(pos) ||
       game.data.entities.needs_removal[&entity_id] {
           return None;
    }

    let is_in_fov = 
       game.data.is_in_fov(player_id, entity_id, &game.config) ||
       game.settings.god_mode;

    if is_in_fov {
        if let Some(mut anims) = display_state.animations.swap_remove(&entity_id) {
            if let Some(mut anim) = anims.pop_front() {
                animation_result = anim.step(pos, game.settings.dt, &game.config);

                if let Animation::PlayEffect(effect) = anim {
                    display_state.play_effect(effect);
                } else {
                    if let Some(sprite) = animation_result.sprite {
                        let mut color = game.data.entities.color[&entity_id];

                        // unarmed traps are grayed out
                        if game.data.entities.armed.get(&entity_id) == Some(&false) {
                            color = game.config.color_warm_grey;
                        }

                        display_state.draw_sprite(panel, sprite, animation_result.pos, color);
                    }

                    // for animations other then effects, keep playing by pushing to front of
                    // animation sequence.
                    if !animation_result.done {
                        anims.push_front(anim);
                    }
                }

                display_state.animations.insert(entity_id, anims);
            }
        } else {
            let color = game.data.entities.color[&entity_id];

            let tiles = display_state.lookup_spritekey("tiles");
            let chr = game.data.entities.chr[&entity_id];
            let sprite = Sprite::new(chr as u32, tiles);
            display_state.draw_sprite(panel, sprite, pos, color);
            animation_result.sprite = Some(sprite);
        }
    } else {
        // if not in FoV, see if we need to add an impression for a golem
        if game.data.entities.typ[&entity_id] == EntityType::Enemy {
            game.data.entities.status[&player_id].extra_fov += 1;
            let is_in_fov_ext = 
               game.data.is_in_fov(player_id, entity_id, &game.config);
            game.data.entities.status[&player_id].extra_fov -= 1;

            if is_in_fov_ext {
                if display_state.impressions.iter().all(|impresssion| impresssion.pos != pos) {
                    let tiles = display_state.lookup_spritekey("tiles");
                    let impression_sprite = Sprite::new(ENTITY_UNKNOWN as u32, tiles);
                    display_state.impressions.push(Impression::new(impression_sprite, pos));
                }
            }
        }
    }

    return animation_result.sprite;
}

fn render_impressions(panel: &mut Panel<&mut WindowCanvas>, display_state: &mut DisplayState, game: &mut Game) {
    // check for entities that have left FOV and make an impression for them
    let mut index = 0;
    while index < display_state.impressions.len() {
        let impression = display_state.impressions[index];
        index += 1;
        display_state.draw_sprite(panel, impression.sprite, impression.pos, game.config.color_light_grey);
    }
}

fn render_entity_type(typ: EntityType, panel: &mut Panel<&mut WindowCanvas>, display_state: &mut DisplayState, game: &mut Game) {
    if typ == EntityType::Player && game.settings.state == GameState::Use && game.settings.use_dir.is_some() {
        // For the player in use-mode, while holding down a direction, we
        // need special rendering. Otherwise the player is rendered as normal.
        let player_id = game.data.find_by_name(EntityName::Player).unwrap();
        let use_dir = game.settings.use_dir.unwrap(); // already checked for is_some
        let use_result = game.data.calculate_use_move(player_id,
                                                      game.settings.use_index as usize,
                                                      use_dir,
                                                      game.settings.use_move_mode);
        if let Some(pos) = use_result.pos {
            let player_pos = game.data.entities.pos[&player_id];
            render_entity_ghost(player_id, player_pos, game, panel, display_state);
            game.data.entities.pos[&player_id] = pos;
            render_entity(panel, player_id, display_state, game);
            game.data.entities.pos[&player_id] = player_pos;
        }
    } else {
        let mut index = 0;
        while index < game.data.entities.ids.len() {
            let entity_id = game.data.entities.ids[index];
            index += 1;

            if !game.data.entities.needs_removal[&entity_id] && game.data.entities.typ[&entity_id] == typ {
                let maybe_sprite = render_entity(panel, entity_id, display_state, game);

                if let Some(sprite) = maybe_sprite {
                    display_state.drawn_sprites.insert(entity_id, sprite);
                }
            }
        }
    }
}

fn render_overlays(panel: &mut Panel<&mut WindowCanvas>,
                   display_state: &mut DisplayState,
                   game: &mut Game,
                   map_mouse_pos: Option<Pos>) {
    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    let player_pos = game.data.entities.pos[&player_id];

    let sprite_key = display_state.lookup_spritekey("tiles");

    // Draw use-mode overlay
    if game.settings.state == GameState::Use {
        let tile_sprite = &mut display_state.sprites[&sprite_key];

        let mut highlight_color = game.config.color_light_grey;
        highlight_color.a = game.config.grid_alpha_overlay;

        let direction_color = Color::white();

        let mut attack_highlight_color = game.config.color_red;
        attack_highlight_color.a = game.config.grid_alpha_overlay;

        if let Some(use_dir) = game.settings.use_dir {
            let use_result = game.data.calculate_use_move(player_id,
                                                          game.settings.use_index as usize,
                                                          use_dir,
                                                          game.settings.use_move_mode);
            if let Some(pos) = use_result.pos {
                draw_tile_highlight(panel, pos, highlight_color);
                render_arrow(panel, tile_sprite, use_dir, pos, direction_color);

                for hit_pos in use_result.hit_positions {
                    draw_tile_highlight(panel, hit_pos, attack_highlight_color);
                }
            }
        } else {
            let mut hit_positions: HashSet<Pos> = HashSet::new();
            let mut move_positions: HashSet<Pos> = HashSet::new();
            for dir in Direction::move_actions().iter() {
                let use_result = game.data.calculate_use_move(player_id,
                                                             game.settings.use_index as usize,
                                                             *dir,
                                                             game.settings.use_move_mode);
                if let Some(pos) = use_result.pos {
                    if !move_positions.contains(&pos) {
                        draw_tile_highlight(panel, pos, highlight_color);
                    }
                    move_positions.insert(pos);
                    render_arrow(panel, tile_sprite, *dir, pos, direction_color);
                    hit_positions.extend(use_result.hit_positions.iter());
                }
            }

            for hit_pos in hit_positions {
                draw_tile_highlight(panel, hit_pos, attack_highlight_color);
            }
        }
    }

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
        if let Some(cursor_pos) = game.settings.cursor {
            let tile_sprite = &mut display_state.sprites[&sprite_key];
            let mut color = game.config.color_mint_green;
            color.a = 230;
            tile_sprite.draw_char(panel, ENTITY_CURSOR as char, cursor_pos, color);

            // render player ghost
            if cursor_pos != player_pos && game.input.target == None {
                let alpha = game.data.entities.color[&player_id].a;
                game.data.entities.color[&player_id].a = 100;

                let maybe_next_pos = astar_next_pos(&game.data.map, player_pos, cursor_pos, None, None);
                if let Some(next_pos) = maybe_next_pos {
                    let dxy = sub_pos(next_pos, player_pos);
                    let direction = Direction::from_dxy(dxy.x, dxy.y).unwrap();

                    let mut reach = reach_by_mode(MoveMode::Sneak);
                    if !game.input.cursor && game.input.shift {
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
                    game.data.pos_in_fov(player_id, pos, &game.config);
                if is_in_fov {
                    tile_sprite.draw_char(panel, MAP_GROUND as char, pos, game.config.color_light_green);
                }
            }
        }
    }

    let mut highlight_color: Color = game.config.color_warm_grey;
    highlight_color.a = game.config.highlight_player_move;

    // draw direction overlays
    if game.config.draw_directional_arrow {
        let direction_color = Color::white();
        let tile_sprite = &mut display_state.sprites[&sprite_key];
        let mut index = 0;
        while index < game.data.entities.ids.len() {
            let entity_id = game.data.entities.ids[index];
            index += 1;

            let pos = game.data.entities.pos[&entity_id];

            if pos.x == -1 && pos.y == -1 {
                continue;
            }

            if game.data.is_in_fov(player_id, entity_id, &game.config) &&
               game.data.entities.status[&entity_id].alive {
                if let Some(dir) = game.data.entities.direction.get(&entity_id) {
                    render_arrow(panel, tile_sprite, *dir, pos, direction_color);
                }
            }
        }
    }

    // draw attack and fov position highlights
    if let Some(mouse_xy) = map_mouse_pos {
        // Draw monster attack overlay
        let object_ids = game.data.get_entities_at_pos(mouse_xy);
        for entity_id in object_ids.iter() {
            let pos = game.data.entities.pos[entity_id];

            if game.data.pos_in_fov(player_id, pos, &game.config) &&
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
               game.data.pos_in_fov(player_id, pos, &game.config) &&
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
    }

    // Draw player movement overlay
    if game.settings.overlay {
        for dir in Direction::move_actions().iter() {
            // for all movements except staying still
            // calculate the move that would occur
            if let Some(movement) =
                calculate_move(*dir,
                               game.data.entities.movement[&player_id],
                               player_id,
                               &mut game.data) {
                // draw a highlight on that square
                // don't draw overlay on top of character
                if movement.pos != game.data.entities.pos[&player_id] {
                    let dxy = sub_pos(movement.pos, player_pos);
                    let direction = Direction::from_dxy(dxy.x, dxy.y).unwrap();
                    let shadow_cursor_pos = direction.offset_pos(player_pos, 1);

                    render_entity_ghost(player_id, shadow_cursor_pos, game, panel, display_state);
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
                let in_fov = game.data.pos_in_fov(player_id, pos, &game.config);
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

    // draw alertness overlays
    {
        let alertness_color = game.config.color_pink;
        let scale = 0.5;
        let tile_sprite = &mut display_state.sprites[&sprite_key];
        for entity_id in game.data.entities.ids.iter() {
            let pos = game.data.entities.pos[entity_id];

            if pos.x == -1 && pos.y == -1 {
                continue;
            }

            if game.data.is_in_fov(player_id, *entity_id, &game.config) {
                let mut status_drawn: bool = false;
                if let Some(status) = game.data.entities.status.get(entity_id) {
                    if status.frozen > 0 {
                        status_drawn = true;
                        tile_sprite.draw_sprite_direction(panel,
                                                          ASTERISK as usize,
                                                          Some(Direction::UpRight),
                                                          pos,
                                                          scale,
                                                          alertness_color,
                                                          0.0);
                    }
                }

                if status_drawn {
                    if let Some(behavior) = game.data.entities.behavior.get(entity_id) {
                        match behavior {
                            Behavior::Idle => {
                            }

                            Behavior::Investigating(_) => {
                                tile_sprite.draw_sprite_direction(panel,
                                                                  QUESTION_MARK as usize,
                                                                  Some(Direction::UpRight),
                                                                  pos,
                                                                  scale,
                                                                  alertness_color,
                                                                  0.0);
                            }

                            Behavior::Attacking(_) => {
                                tile_sprite.draw_sprite_direction(panel,
                                                                  EXCLAMATION_POINT as usize,
                                                                  Some(Direction::UpRight),
                                                                  pos,
                                                                  scale,
                                                                  alertness_color,
                                                                  0.0);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn empty_tile_color(config: &Config, pos: Pos, visible: bool, rng: &mut Rand32) -> Color {
    let low_color;
    let high_color;
    if visible {
        low_color = config.color_medium_grey;
        high_color = config.color_light_grey;
    } else {
        low_color = config.color_medium_grey;
        high_color = config.color_light_grey;
    }

    let simplex = Perlin::new(rng);
    let color =
        lerp_color(low_color,
                   high_color,
                   simplex.noise2d(pos.x as f64 / config.tile_noise_scaler,
                                   pos.y as f64 / config.tile_noise_scaler) as f32);

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
                     let player_can_see = in_bounds && game.data.pos_in_fov(player_id, *pos, &game.config);
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

            let visible = game.data.pos_in_fov(entity_id, map_pos, &game.config) &&
                          game.data.pos_in_fov(player_id, map_pos, &game.config);


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
            let visible = game.data.pos_in_fov(player_id, move_pos, &game.config);
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


pub fn render_entity_ghost(entity_id: EntityId, render_pos: Pos, game: &mut Game, panel: &mut Panel<&mut WindowCanvas>, display_state: &mut DisplayState) {
    let entity_pos = game.data.entities.pos[&entity_id];

    let alpha = game.data.entities.color[&entity_id].a;
    game.data.entities.color[&entity_id].a = 200;

    game.data.entities.pos[&entity_id] = render_pos;

    render_entity(panel, entity_id, display_state, game);

    game.data.entities.color[&entity_id].a = alpha;
    game.data.entities.pos[&entity_id] = entity_pos;
}

fn render_arrow(panel: &mut Panel<&mut WindowCanvas>, tile_sprite: &mut SpriteSheet, dir: Direction, pos: Pos, direction_color: Color) {
    let sprite_index;
    let rotation: f64;
    match dir {
        Direction::Up => {
            rotation = -90.0;
            sprite_index = ARROW_HORIZ;
        }
        Direction::Down => {
            rotation = 90.0;
            sprite_index = ARROW_HORIZ;
        }
        Direction::Right => {
            rotation = 0.0;
            sprite_index = ARROW_HORIZ;
        }
        Direction::Left => {
            rotation = 180.0;
            sprite_index = ARROW_HORIZ;
        }
        Direction::DownLeft => {
            rotation = -180.0;
            sprite_index = ARROW_DIAG;
        }
        Direction::DownRight => {
            rotation = 90.0;
            sprite_index = ARROW_DIAG;
        }
        Direction::UpLeft => {
            rotation = -90.0;
            sprite_index = ARROW_DIAG;
        }
        Direction::UpRight => {
            rotation = 0.0;
            sprite_index = ARROW_DIAG;
        }
    };

    tile_sprite.draw_sprite_at_cell(panel, sprite_index as usize, pos, direction_color, rotation, false, false);
}
