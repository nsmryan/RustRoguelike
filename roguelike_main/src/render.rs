use std::collections::HashSet;

use oorandom::Rand32;

use logging_timer::timer;

use roguelike_core::types::*;
use roguelike_core::map::*;
use roguelike_core::constants::*;
use roguelike_core::movement::*;
use roguelike_core::config::*;
use roguelike_core::utils::*;
use roguelike_core::perlin::Perlin;
use roguelike_core::line::line;
use roguelike_core::ai::*;

use roguelike_engine::game::*;

use crate::display::*;
use crate::animation::{Sprite, SpriteKey, Effect, Animation, AnimationResult, Particle};


pub fn render_all(panels: &mut Panels, display_state: &mut DisplayState, game: &mut Game, dt: f32) -> Result<(), String> {
    display_state.dt = dt;
    display_state.time += dt;

    /* Draw Background */
    render_background(panels.get_mut(&PanelName::Map).unwrap(), display_state, game);

    /* Draw Map */
    render_panels(panels, display_state, game);

    /* Draw Debug Overlay */
    if game.settings.debug_enabled {
        render_debug(panels.get_mut(&PanelName::Map).unwrap(), display_state);
    }

    let menu_panel = panels.get_mut(&PanelName::Menu).unwrap();

    if game.settings.state == GameState::Inventory {
        render_inventory(menu_panel, display_state, game);
    } else if game.settings.state == GameState::SkillMenu {
        render_skill_menu(menu_panel, display_state, game);
    } else if game.settings.state == GameState::ClassMenu {
        render_class_menu(menu_panel, display_state);
    } else if game.settings.state == GameState::ConfirmQuit {
        render_confirm_quit(menu_panel, display_state);
    }

    Ok(())
}


fn render_panels(panels: &mut Panels, display_state: &mut DisplayState, game: &mut Game) {
    if game.settings.render_map {
        let panel = &mut panels.get_mut(&PanelName::Map).unwrap();

        {
            let _map = timer!("MAP");
            render_map(panel, display_state, game);
        }

        {
            let _mid = timer!("MID");
            render_entity_type(panel, EntityType::Trigger, display_state, game);
            render_entity_type(panel, EntityType::Item, display_state, game);

            render_map_middle(panel, display_state, game);
        }

        {
            let _above = timer!("ABOVE");
            render_entity_type(panel, EntityType::Energy, display_state, game);
            render_entity_type(panel, EntityType::Enemy, display_state, game);
            render_entity_type(panel, EntityType::Column, display_state, game);
            render_entity_type(panel, EntityType::Player, display_state, game);
            render_entity_type(panel, EntityType::Other, display_state, game);
        }

        {
            let _overlays_game = timer!("OVERLAYSGAME");
            render_game_overlays(panel, display_state, game, game.settings.cursor);
        }

        {
            let _mapabove = timer!("MAPABOVE");
            render_map_above(panel, display_state, game);
        }

        {
            let _extra = timer!("EXTRA");
            render_impressions(panel, display_state, game);
            render_effects(panel, display_state, game);
            render_overlays(panel, display_state, game, game.settings.cursor);
        }
    }

    /* Draw Player Info */
    {
        let player_panel = &mut panels.get_mut(&PanelName::Player).unwrap();
        render_player_info(player_panel, display_state, game);
    }

    /* Draw Inventory */
    {
        let inventory_panel = &mut panels.get_mut(&PanelName::Inventory).unwrap();
        render_inventory(inventory_panel, display_state, game);
    }

    /* Draw Game Info */
    {
        let info_panel = &mut panels.get_mut(&PanelName::Info).unwrap();
        render_info(info_panel, display_state, game);
    }
}


fn render_debug(panel: &mut Panel, display_state: &mut DisplayState) {
    let mut text_list = Vec::new();
    for (key, value) in display_state.debug_entries.iter() {
        text_list.push(format!("{}: {}", key, value));
    }

    let text_pos = Pos::new(1, 1);
    let text_color = Color::new(0xcd, 0xb4, 0x96, 255);
    panel.text_list_cmd(&text_list, text_color, text_pos);
}

/// Draw an outline and title around an area of the screen
fn render_placard(panel: &mut Panel,
                  display_state: &mut DisplayState,
                  text: &str) {
    // TODO delete commented out code when draw commands are complete
    // TODO this color comes from the UI mockups as a light brown
    let color = Color::new(0xcd, 0xb4, 0x96, 255);

    // Draw a black background
    let (cell_width, cell_height) = panel.cell_dims();

    let filled = false;
    panel.rect_cmd(Pos::new(0, 0), panel.cells, 0.5, filled, color);

    // draw a rectangle around where the placard header text will be placed.
    let half_text = text.len() / 2;
    assert!(panel.cells.0 / 2 >= half_text as u32, "text too long to center!");
    let text_offset = (panel.cells.0 / 2) - half_text as u32;

    let filled = true;
    let text_dims = (text.len() as u32, 1);
    panel.rect_cmd(Pos::new(text_offset as i32, 0), text_dims, 0.0, filled, color);

    // Draw header text
    let text_pos = Pos::new(text_offset as i32, 0);
    let text_color = Color::new(0, 0, 0, 255);

    panel.justify_cmd(text,
                      Justify::Center,
                      text_color,
                      Pos::new(0, 0),
                      panel.cells.0);
}

fn render_pips(panel: &mut Panel,
               display_state: &mut DisplayState,
               num_pips: u32,
               pos: Pos,
               color: Color) {
    if num_pips > 0 {
        for pip_index in 0..num_pips as i32 {
            let filled = true;
            let pip_pos = move_x(pos, pip_index);
            panel.rect_cmd(pip_pos, (1, 1), 0.12, filled, color);
        }
    }
}

// TODO remove commented out code when draw cmds are done
fn render_bar(panel: &mut Panel,
              display_state: &mut DisplayState,
              full: i32,
              current: i32,
              pos: Pos,
              fg_color: Color,
              bg_color: Color,
              draw_outline: bool) {
    let filled = true;
    panel.rect_cmd(pos, (current as u32, 1), 0.0, filled, fg_color);

    if draw_outline {
        let filled = false;
        panel.rect_cmd(pos, (full as u32, 1), 0.0, filled, bg_color);
    }
}

fn render_player_info(panel: &mut Panel, display_state: &mut DisplayState, game: &mut Game) {
    render_placard(panel, display_state, "Player");

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();

    let mut list: Vec<String> = Vec::new();

    // TODO this color comes from the UI mockups as a light brown
    let color = Color::new(0xcd, 0xb4, 0x96, 255);

    if let Some(fighter) = game.data.entities.fighter.get(&player_id) {
        let hp = if fighter.hp > 0 {
            fighter.hp
        } else {
            0
        };
        let health_percent = hp as f32 / fighter.max_hp as f32;

        // TODO this color red comes from the UI mockups
        let health_color = Color::new(0x96, 0x54, 0x56, 255);
        let bar_pos = Pos::new(2, 2);
        render_bar(panel, display_state, fighter.max_hp, hp, bar_pos, health_color, Color::white(), false);
    }

    let energy = game.data.entities.energy[&player_id];
    // TODO this color orange comes from the UI mockups
    let energy_color = Color::new(0xaf, 0x83, 0x56, 255);
    render_pips(panel, display_state, energy, Pos::new(2, 3), energy_color);

    list.push(format!(""));


    let stance = game.data.entities.stance[&player_id];
    list.push(format!("{}", stance));
    list.push("next move".to_string());
    let stance = game.settings.move_mode;
    list.push(format!("{}", stance));

    list.push(format!(""));
    list.push(format!(""));
    list.push(format!(""));
    list.push(format!(""));
    list.push(format!(""));
    list.push(format!(""));
    list.push(format!(""));
    list.push(format!(""));
    list.push(format!(""));
    list.push(format!("turn {}", game.settings.turn_count));

    let text_pos = Pos::new(1, 5);

    panel.text_list_cmd(&list, color, text_pos);
}

fn render_info(panel: &mut Panel,
               display_state: &mut DisplayState,
               game: &mut Game) {
    render_placard(panel, display_state, "Info");

    if let Some(info_pos) = game.settings.cursor {
        let text_color = Color::new(0xcd, 0xb4, 0x96, 255);

        let player_id = game.data.find_by_name(EntityName::Player).unwrap();

        let object_ids = game.data.get_entities_at_pos(info_pos);

        let mut y_pos = 1;

        let mut text_list = Vec::new();

        text_list.push(format!("({:>2},{:>2})", info_pos.x, info_pos.y));

        let text_pos = Pos::new(1, y_pos);

        panel.text_list_cmd(&text_list, text_color, text_pos);

        text_list.clear();

        y_pos += 1;

        let mut drawn_info = false;

        // only display first object
        if let Some(obj_id) = object_ids.first() {
            let entity_in_fov = game.settings.god_mode ||
                                game.data.is_in_fov(player_id, *obj_id, &game.config);

            // only display things in the player's FOV
            if entity_in_fov {
                drawn_info = true;

                if let Some(fighter) = game.data.entities.fighter.get(obj_id) {
                    y_pos += 1;

                    let health_percent = fighter.hp as f32 / fighter.max_hp as f32;

                    let health_color = Color::new(0x96, 0x54, 0x56, 255);
                    render_bar(panel,
                               display_state,
                               fighter.max_hp,
                               fighter.hp,
                               Pos::new(2, y_pos),
                               health_color,
                               Color::white(),
                               false);

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

        // if there was nothing else to draw, check for an impression
        if !drawn_info {
            for impr in display_state.impressions.iter() {
                if impr.pos == info_pos {
                    text_list.push("Golem".to_string());
                    break;
                }
            }
        }

        let text_pos = Pos::new(1, y_pos);
        panel.text_list_cmd(&text_list, text_color, text_pos);
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

            panel.text_list_cmd(&text_list, text_color, text_pos);
        }
    }
}

fn render_skill_menu(panel: &mut Panel, display_state: &mut DisplayState, game: &mut Game) {
    let player_id = game.data.find_by_name(EntityName::Player).unwrap();

    // Render header
    render_placard(panel, display_state, "Skills");

    let mut list = Vec::new();

    for (index, skill) in game.data.entities.skills[&player_id].iter().enumerate() {
        list.push(format!("{} {:?}", index, skill));
    }

    let y_pos = 2;
    let text_pos = Pos::new(2, y_pos);

    // TODO this color comes from the ui mockups as a light brown
    let color = Color::new(0xcd, 0xb4, 0x96, 255);

    panel.text_list_cmd(&list, color, text_pos);
}

fn render_class_menu(panel: &mut Panel, display_state: &mut DisplayState) {
    // Render header
    render_placard(panel, display_state, "Choose Class");

    let mut list = Vec::new();

    for (index, class) in EntityClass::classes().iter().enumerate() {
        list.push(format!("{} {:?}", index, class));
    }

    let y_pos = 2;
    let text_pos = Pos::new(2, y_pos);

    // TODO this color comes from the ui mockups as a light brown
    let color = Color::new(0xcd, 0xb4, 0x96, 255);

    let sprite_key = display_state.lookup_spritekey("font");
    let tile_sprite = &mut display_state.sprites[sprite_key];

    panel.text_list_cmd(&list, color, text_pos);
}

fn render_confirm_quit(panel: &mut Panel, display_state: &mut DisplayState) {
    // Render header
    render_placard(panel, display_state, "Quit?");

    let mut list = Vec::new();

    list.push("q: quit game".to_string());
    list.push("".to_string());
    list.push("esc: continue".to_string());

    let y_pos = 2;
    let text_pos = Pos::new(2, y_pos);

    // TODO this color comes from the UI mockups as a light brown
    let color = Color::new(0xcd, 0xb4, 0x96, 255);

    panel.text_list_cmd(&list, color, text_pos);
}

/// Render an inventory section within the given area
fn render_inventory(panel: &mut Panel, display_state: &mut DisplayState, game: &mut Game) {
    // Render header
    render_placard(panel, display_state, "Inventory");

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();

    // TODO this color comes from the UI mockups as a light brown
    let ui_color = Color::new(0xcd, 0xb4, 0x96, 255);

    let sprite_key = display_state.lookup_spritekey("font");

    // Render each object's name in inventory
    let mut y_pos = 2;

    // Draw Primary Items
    panel.text_cmd("z", ui_color, Pos::new(1, y_pos));

    let mut index = 0;
    while index < game.data.entities.inventory[&player_id].len() {
        let item_id = game.data.entities.inventory[&player_id][index];

        if game.data.entities.item[&item_id].class() == ItemClass::Primary {
            let item_text = format!("{:?}", game.data.entities.name[&item_id]);
            let text_pos = Pos::new(3, y_pos);
            panel.text_cmd(&item_text, ui_color, text_pos);
            break;
        }

        index += 1;
    }
    y_pos += 1;

    // Draw Consumable Items
    let tile_sprite = &mut display_state.sprites[sprite_key];
    panel.text_cmd(&"x", ui_color, Pos::new(1, y_pos));

    let mut index = 0;
    while index < game.data.entities.inventory[&player_id].len() {
        let item_id = game.data.entities.inventory[&player_id][index];

        if game.data.entities.item[&item_id].class() == ItemClass::Consumable {
            let item_text = format!("{:?}", game.data.entities.name[&item_id]);
            let text_pos = Pos::new(3, y_pos);
            panel.text_cmd(&item_text, ui_color, text_pos);
            break;
        }

        index += 1;
    }
    y_pos += 1;

    // Draw Stones Items
    panel.text_cmd(&"c", ui_color, Pos::new(1, y_pos));

    let mut num_stones = 0;
    let mut index = 0;
    while index < game.data.entities.inventory[&player_id].len() {
        let item_id= game.data.entities.inventory[&player_id][index];

        if game.data.entities.item[&item_id] == Item::Stone {
            num_stones += 1;
        }

        index += 1;
    }

    if num_stones > 0 {
        panel.text_cmd(&"c", ui_color, Pos::new(1, y_pos));

        let text_pos = Pos::new(3, y_pos);
        panel.text_cmd(&"stone", ui_color, text_pos);

        let num_text = format!("({})", num_stones);
        panel.text_cmd(&num_text, ui_color, Pos::new(9, y_pos));
    }

    y_pos += 1;
    y_pos += 1;

    // Draw Remaining Items
    let mut index = 0;
    while index < game.data.entities.inventory[&player_id].len() {
        let item_id= game.data.entities.inventory[&player_id][index];

        if game.data.entities.item[&item_id].class() == ItemClass::Misc &&
           game.data.entities.item[&item_id] != Item::Stone {
            let item_text = format!("{:?}", game.data.entities.name[&item_id]);
            let text_pos = Pos::new(3, y_pos);
            panel.text_cmd(&item_text, ui_color, text_pos);
            y_pos += 1;
        }

        index += 1;
    }
}

/// render the background files, including water tiles
fn render_background(panel: &mut Panel, display_state: &mut DisplayState, game: &mut Game) {
    let (map_width, map_height) = game.data.map.size();

    let sprite_key = display_state.lookup_spritekey("tiles");
    let sprite = &mut display_state.sprites[sprite_key];

    for y in 0..map_height {
        for x in 0..map_width {
            let map_pos = Pos::new(x, y);

            let tile = &game.data.map[(x, y)];
            // TODO why are these branches identical?
            if tile.tile_type != TileType::Water {
                let sprite = Sprite::new(MAP_EMPTY_CHAR as u32, sprite_key);
                panel.sprite_cmd(sprite, Color::white(), map_pos);
            } else {
                let sprite = Sprite::new(MAP_EMPTY_CHAR as u32, sprite_key);
                panel.sprite_cmd(sprite, Color::white(), map_pos);
            }
        }
    }
}

fn surface_chr(surface: Surface) -> Option<u8> {
    match surface {
        Surface::Rubble => {
            return Some(MAP_RUBBLE);
        }

        Surface::Grass => {
            return Some(MAP_GRASS);
        }

        Surface::Floor => {
            // Nothing to draw
            return None;
        }
    }
}

/// Render Wall Shadows (full tile and intertile walls, left and down)
fn render_wall_shadow(panel: &mut Panel, pos: Pos, display_state: &mut DisplayState, game: &mut Game) {
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

    let shadow_color = game.config.color_shadow;

    /* render full tile wall shadows */
    if tile.tile_type == TileType::Wall {
        if left_valid && !left_wall {
            // left
            let shadow_pos = Pos::new(x - 1, y);
            let shadow_left_upper = Sprite::new(SHADOW_FULLTILE_LEFT as u32, shadow_sprite_key);
            panel.sprite_cmd(shadow_left_upper, shadow_color, shadow_pos);
        }

        if down_left_valid && !down_left_wall {
            let shadow_pos = Pos::new(x - 1, y + 1);
            let shadow_left_lower = Sprite::new(SHADOW_FULLTILE_LEFT_DOWN as u32, shadow_sprite_key);
            panel.sprite_cmd(shadow_left_lower, shadow_color, shadow_pos);
        }

        if down_valid && !down_wall {
            // lower
            let shadow_lower_right = Sprite::new(SHADOW_FULLTILE_DOWN as u32, shadow_sprite_key);
            let shadow_pos = Pos::new(x, y + 1);
            panel.sprite_cmd(shadow_lower_right, shadow_color, shadow_pos);
        }

        if down_left_valid && !down_left_wall {
            let shadow_lower_left = Sprite::new(SHADOW_FULLTILE_DOWN_LEFT as u32, shadow_sprite_key);
            let shadow_pos = Pos::new(x - 1, y + 1);
            panel.sprite_cmd(shadow_lower_left, shadow_color, shadow_pos);
        }
    } else if tile.left_wall == Wall::ShortWall {
        // left
        if left_valid {
            let shadow_pos = Pos::new(x - 1, y);
            let shadow_left_upper = Sprite::new(SHADOW_INTERTILE_LEFT as u32, shadow_sprite_key);
            panel.sprite_cmd(shadow_left_upper, shadow_color, shadow_pos);
        }

        // left down
        if down_left_valid {
            let shadow_pos = Pos::new(x - 1, y + 1);
            let shadow_left_lower = Sprite::new(SHADOW_INTERTILE_LEFT_DOWN as u32, shadow_sprite_key);
            panel.sprite_cmd(shadow_left_lower, shadow_color, shadow_pos);
        }
    } else if tile.bottom_wall == Wall::ShortWall {
        if down_valid {
            // lower
            if down_valid {
                let shadow_lower_right = Sprite::new(SHADOW_INTERTILE_DOWN as u32, shadow_sprite_key);
                let shadow_pos = Pos::new(x, y + 1);
                panel.sprite_cmd(shadow_lower_right, shadow_color, shadow_pos);
            }

            // left down
            if down_left_valid {
                let shadow_lower_left = Sprite::new(SHADOW_INTERTILE_DOWN_LEFT as u32, shadow_sprite_key);
                let shadow_pos = Pos::new(x - 1, y + 1);
                panel.sprite_cmd(shadow_lower_left, shadow_color, shadow_pos);
            }
        }
    }
}

fn render_map_above(panel: &mut Panel, display_state: &mut DisplayState, game: &mut Game) {
    let player_id = game.data.find_by_name(EntityName::Player).unwrap();

    let (map_width, map_height) = game.data.map.size();

    let sprite_key = display_state.lookup_spritekey("tiles");
    for y in 0..map_height {
        for x in 0..map_width {
            let pos = Pos::new(x, y);
            /* draw the between-tile walls appropriate to this tile */
            {
                let tile = game.data.map[pos];
                let wall_color = Color::white();

                // Lower walls
                if tile.bottom_wall == Wall::ShortWall {
                    let sprite = Sprite::new(MAP_THIN_WALL_BOTTOM as u32, sprite_key);
                    panel.sprite_cmd(sprite, wall_color, pos);
                } else if tile.bottom_wall == Wall::TallWall {
                    let sprite = Sprite::new(MAP_THICK_WALL_BOTTOM as u32, sprite_key);
                    panel.sprite_cmd(sprite, wall_color, pos);
                }
            }

            let mut fov_result;
            {
                let _vis = timer!("VIS");
                fov_result = game.data.pos_in_fov_edge(player_id, pos, &game.config);
            }
            if game.settings.god_mode {
                fov_result = FovResult::Inside;
            }

            // apply a FoW darkening to cells
            if game.config.fog_of_war && fov_result != FovResult::Inside {
                let is_in_fov_ext = fov_result == FovResult::Edge;

                let mut blackout_color = Color::black();
                let sprite = Sprite::new(MAP_EMPTY_CHAR as u32, sprite_key);
                if is_in_fov_ext {
                    blackout_color.a = game.config.fov_edge_alpha;
                    panel.sprite_cmd(sprite, blackout_color, pos);
                } else if game.data.map[pos].explored {
                    blackout_color.a = game.config.explored_alpha;
                    panel.sprite_cmd(sprite, blackout_color, pos);
                } else {
                    panel.fill_cmd(pos, blackout_color);
                }
            }
        }
    }
}

fn render_map_middle(panel: &mut Panel, display_state: &mut DisplayState, game: &mut Game) {
    let (map_width, map_height) = game.data.map.size();

    let sprite_key = display_state.lookup_spritekey("tiles");
    for y in 0..map_height {
        for x in 0..map_width {
            let pos = Pos::new(x, y);

            render_wall_shadow(panel, pos, display_state, game);

            /* draw the between-tile walls appropriate to this tile */
            render_intertile_walls_below(panel, display_state, &mut game.data.map, sprite_key, pos);
        }
    }
}

/// Render the map, with environment and walls
fn render_map(panel: &mut Panel, display_state: &mut DisplayState, game: &mut Game) {
    let (map_width, map_height) = game.data.map.size();

    let sprite_key = display_state.lookup_spritekey("tiles");
    for y in 0..map_height {
        for x in 0..map_width {
            let pos = Pos::new(x, y);

            // draw an outline around the tile
            {
                let mut outline_color = Color::white();
                outline_color.a /= 8;
                panel.outline_cmd(outline_color, pos);
            }

            // Render game stuff
            let tile = game.data.map[pos];

            let chr = tile.chr;

            // if the tile is not empty or water, draw it
            if tile.tile_type == TileType::Water {
                let sprite = Sprite::new(MAP_WATER as u32, sprite_key);
                panel.sprite_cmd(sprite, Color::white(), pos);
            } else if chr != MAP_EMPTY_CHAR {
                let sprite = Sprite::new(chr as u32, sprite_key);
                panel.sprite_cmd(sprite, Color::white(), pos);
            }

            if let Some(chr) = surface_chr(tile.surface) {
                let sprite = Sprite::new(chr as u32, sprite_key);
                panel.sprite_cmd(sprite, Color::white(), pos);
            }
        }
    }
}

fn render_intertile_walls_below(panel: &mut Panel,
                                   display_state: &mut DisplayState,
                                   map: &Map,
                                   sprite_key: SpriteKey,
                                   pos: Pos) {
    let (x, y) = pos.to_tuple();
    let tile = map[pos];
    let wall_color = Color::white();

    // Left walls
    if tile.left_wall == Wall::ShortWall {
        let sprite = Sprite::new(MAP_THIN_WALL_LEFT as u32, sprite_key);
        panel.sprite_cmd(sprite, wall_color, pos);
    } else if tile.left_wall == Wall::TallWall {
        let sprite = Sprite::new(MAP_THIN_WALL_LEFT as u32, sprite_key);
        panel.sprite_cmd(sprite, wall_color, pos);
    }

    // Right walls
    if x + 1 < map.width() {
        let right_pos = Pos::new(pos.x + 1, pos.y);
        let right_tile = &map[right_pos];
        if right_tile.left_wall == Wall::ShortWall {
            let sprite = Sprite::new(MAP_THIN_WALL_RIGHT as u32, sprite_key);
            panel.sprite_cmd(sprite, wall_color, pos);
        } else if right_tile.left_wall == Wall::TallWall {
            let sprite = Sprite::new(MAP_THICK_WALL_RIGHT as u32, sprite_key);
            panel.sprite_cmd(sprite, wall_color, pos);
        }
    }

    // Upper walls
    if y - 1 >= 0 {
        let up_pos = Pos::new(pos.x, pos.y - 1);
        let up_tile = &map[up_pos];
        if up_tile.bottom_wall == Wall::ShortWall {
            let sprite = Sprite::new(MAP_THIN_WALL_TOP as u32, sprite_key);
            panel.sprite_cmd(sprite, wall_color, pos);
        } else if up_tile.bottom_wall == Wall::TallWall {
            let sprite = Sprite::new(MAP_THICK_WALL_TOP as u32, sprite_key);
            panel.sprite_cmd(sprite, wall_color, pos);
        }
    }
}

/// Render each effect currently playing in the game
/// The strategy here is to copy the effects vector, update all items,
/// and then remove finished effects from back to front. The
/// resulting vector of effects is then saved as the new effects vector.
fn render_effects(panel: &mut Panel,
                  display_state: &mut DisplayState,
                  game: &mut Game) {
    let player_id = game.data.find_by_name(EntityName::Player).unwrap();

    let mut index = 0;
    while index < display_state.effects.len() {
        let mut effect_complete = false;

        // NOTE(perf) cloning the effect vector each iteration!
        let mut effect = display_state.effects[index].clone();
        match &mut effect {
            Effect::Particles(rate, particles) => {
                if particles.len() < game.config.max_particles && rng_trial(&mut game.rng, *rate) {
                    let size = (panel.num_pixels.0 as i32, panel.num_pixels.1 as i32);
                    let pos = rng_pos(&mut game.rng, size);
                    particles.push(Particle::new(game.config.particle_duration, pos));
                }

                let sprite_key = display_state.lookup_spritekey("particle_speck");

                let dims = panel.cell_dims();
                let mut index = 0;
                while index < particles.len() {
                    let cell = panel.cell_from_pixel(particles[index].pos);
                    particles[index].duration -= display_state.dt;

                    // if the particle is finished, or has left the map, remove it.
                    if particles[index].duration < 0.0 || !game.data.map.is_within_bounds(cell) {
                        particles.swap_remove(index);
                    } else {
                        // offset the particle according to how long it has been running.
                        let x_offset = (dims.0 as f32 * (game.config.particle_duration - particles[index].duration)) as i32;
                        let draw_pos = move_x(particles[index].pos, x_offset);
                        let draw_cell = panel.cell_from_pixel(draw_pos);

                        if game.data.map.is_within_bounds(draw_cell) && game.data.pos_in_fov(player_id, draw_cell, &game.config) {
                            let mut color = Color::white();
                            // fade the particle out according to how long it has been running.
                            color.a = (255.0 * (particles[index].duration / game.config.particle_duration)) as u8;
                            //speck_sprite.draw_sprite_full(panel, 0, draw_pos, color, 0.0, false, false);
                            let sprite = Sprite::new(0, sprite_key);
                            panel.sprite_at_pixel_cmd(sprite, color, draw_pos);
                        }
                        index += 1;
                    }
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
                        if !game.data.map[*pos].block_move &&
                           game.data.pos_in_fov(player_id, *pos, &game.config) {
                           panel.highlight_cmd(highlight_color, *pos);
                           panel.outline_cmd(highlight_color, *pos);
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
                    let mut sprite = Sprite::new(sprite_index as u32, sprite_key);
                    sprite.rotation = rotation;
                    panel.sprite_cmd(sprite, Color::white(), pos);
                }

                if *remaining == 0 {
                    effect_complete = true;
                } else {
                    *remaining -= 1;
                }
            }

            Effect::Attack(from, to, sprite_anim) => {
                let sprite = sprite_anim.sprite();

                let pixel_from = panel.pixel_from_cell(move_next_to(*from, *to));
                let pixel_to = panel.pixel_from_cell(*to);
                let pixel_pos = Pos::new((pixel_from.x + pixel_to.x) / 2,
                                         (pixel_from.y + pixel_to.y) / 2);
                panel.sprite_at_pixel_cmd(sprite, Color::white(), pixel_pos);

                // if the sprite animation looped back to the beginning, end the effect
                if sprite_anim.looped {
                    effect_complete = true;
                }
            }

            Effect::Fade(sprite, color, start, end, pos, seconds, time_taken) => {
                let mut color = color;
                let percent = *time_taken / *seconds;
                let new_alpha = (*end as f32 - *start as f32) * percent;
                color.a = (*start as f32 + new_alpha) as u8;
                panel.sprite_cmd(*sprite, *color, *pos);

                *time_taken += display_state.dt;
                effect_complete = time_taken >= seconds;
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

fn render_entity(panel: &mut Panel,
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
            if let Some(anim) = anims.pop_front() {
                animation_result = anim.status(pos);

                if let Animation::PlayEffect(effect) = anim {
                    display_state.play_effect(effect);
                } else {
                    if let Some(sprite) = animation_result.sprite {
                        let mut color = game.data.entities.color[&entity_id];

                        // unarmed traps are grayed out
                        if game.data.entities.armed.get(&entity_id) == Some(&false) {
                            color = game.config.color_warm_grey;
                        }

                        panel.sprite_cmd(sprite, color, animation_result.pos);
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

            panel.sprite_cmd(sprite, color, pos);
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

fn render_impressions(panel: &mut Panel, display_state: &mut DisplayState, game: &mut Game) {
    // check for entities that have left FOV and make an impression for them
    let mut index = 0;
    while index < display_state.impressions.len() {
        let impression = display_state.impressions[index];
        index += 1;
        panel.sprite_cmd(impression.sprite,
                                 game.config.color_light_grey,
                                 impression.pos);
    }
}

fn render_entity_type(panel: &mut Panel, typ: EntityType, display_state: &mut DisplayState, game: &mut Game) {
    if typ == EntityType::Player && game.settings.state == GameState::Use && game.settings.use_dir.is_some() {
        // For the player in use-mode, while holding down a direction, we
        // need special rendering. Otherwise the player is rendered as normal.

        let player_id = game.data.find_by_name(EntityName::Player).unwrap();
        let player_pos = game.data.entities.pos[&player_id];
        let use_dir = game.settings.use_dir.unwrap(); // already checked for is_some

        let mut use_pos = None;
        if let UseAction::Item(item_class) = game.settings.use_action {

            if let Some(item_index) = game.data.entities.item_by_class(player_id, item_class) {
                let use_result = game.data.calculate_use_move(player_id,
                                                              item_index,
                                                              use_dir,
                                                              game.settings.move_mode);
                use_pos = use_result.pos;
            }
        } else {
            // if interacting, draw player ghost only if the tile is clear
            let target_pos = use_dir.offset_pos(player_pos, 1);
            if game.data.clear_path(player_pos, target_pos, false) {
                use_pos = Some(target_pos);
            }
        }

        if let Some(pos) = use_pos {
            render_entity_ghost(panel, player_id, player_pos, game, display_state);
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

fn render_overlay_use_item(panel: &mut Panel,
                              item_class: ItemClass,
                              display_state: &mut DisplayState,
                              game: &mut Game) {
    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    let player_pos = game.data.entities.pos[&player_id];

    let direction_color = Color::white();
    let mut attack_highlight_color = game.config.color_red;
    attack_highlight_color.a = game.config.grid_alpha_overlay;
    let mut highlight_color = game.config.color_light_grey;
    highlight_color.a = game.config.grid_alpha_overlay;

    let sprite_key = display_state.lookup_spritekey("tiles");

    if let Some(item_index) = game.data.entities.item_by_class(player_id, item_class) {
        if let Some(use_dir) = game.settings.use_dir {
            let use_result = game.data.calculate_use_move(player_id,
                                                          item_index,
                                                          use_dir,
                                                          game.settings.move_mode);
            if let Some(_pos) = use_result.pos {
                let arrow_pos = use_dir.offset_pos(player_pos, 1);
                render_arrow(panel, sprite_key, use_dir, arrow_pos, direction_color);

                for hit_pos in use_result.hit_positions {
                   panel.highlight_cmd(attack_highlight_color, hit_pos);
                }
            }
        } else {
            // try each direction, keeping track of all hit positions to draw a highlight
            // on those tiles, and keeping track of all move positions to avoid drawing
            // multiple highlights on movements tiles that are re-used between directions.
            let mut hit_positions: HashSet<Pos> = HashSet::new();
            let mut move_positions: HashSet<Pos> = HashSet::new();
            for dir in Direction::move_actions().iter() {
                let use_result = game.data.calculate_use_move(player_id,
                                                             item_index,
                                                             *dir,
                                                             game.settings.move_mode);
                if let Some(pos) = use_result.pos {
                    if !move_positions.contains(&pos) {
                       panel.highlight_cmd(highlight_color, pos);
                    }
                    move_positions.insert(pos);
                    let arrow_pos = dir.offset_pos(player_pos, 1);
                    render_arrow(panel, sprite_key, *dir, arrow_pos, direction_color);
                    hit_positions.extend(use_result.hit_positions.iter());
                }
            }

            for hit_pos in hit_positions {
               panel.highlight_cmd(attack_highlight_color, hit_pos);
            }
        }
    }
}

fn render_sound_overlay(panel: &mut Panel,
                           display_state: &mut DisplayState,
                           game: &mut Game) {
    let mut highlight_sound: Color = game.config.color_warm_grey;
    highlight_sound.a = game.config.sound_alpha;
    // NOTE(perf) this clone is only necessary because draw commands
    // mut borrow the entire display state, instead of only the draw_cmd hashmap or
    // even just the vec of commands.
    for pos in display_state.sound_tiles.clone().iter() {
        // NOTE this currently does not take into account FOV!
        // if game.data.pos_in_fov(player_id, pos, &game.config) {
        panel.highlight_cmd(highlight_sound, *pos);
        //}
    }
}

fn render_game_overlays(panel: &mut Panel,
                   display_state: &mut DisplayState,
                   game: &mut Game,
                   cursor_pos: Option<Pos>) {
    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    let player_pos = game.data.entities.pos[&player_id];

    let tiles_key = display_state.lookup_spritekey("tiles");

    if game.config.use_cursor {
        if let Some(cursor_pos) = game.settings.cursor {
            // render trigger plate wall highlight if selected
            let entities = game.data.get_entities_at_pos(cursor_pos);
            for entity in entities {
                if game.data.entities.name[&entity] == EntityName::GateTrigger {
                    let gate_pos = game.data.entities.gate_pos[&entity];
                    let mut highlight_color: Color = game.config.color_red;
                    highlight_color.a = 100;
                    panel.highlight_cmd(highlight_color, gate_pos);
                } else if game.data.entities.name[&entity] == EntityName::FreezeTrap {
                    let trap_pos = game.data.entities.pos[&entity];
                    let freeze_aoe =
                        aoe_fill(&game.data.map, AoeEffect::Freeze, trap_pos, game.config.freeze_trap_radius, &game.config);
                    for pos in freeze_aoe.positions() {
                        let mut highlight_color: Color = game.config.color_blueish_grey;
                        highlight_color.a = 100;
                        panel.highlight_cmd(highlight_color, pos);
                    }
                }
            }

            // render some extra player information if cursor is over player's tile
            if cursor_pos == player_pos {
                // Draw sound tiles overlay
                render_sound_overlay(panel, display_state, game);
            }
        }
    }

    // Draw use-mode overlay
    if game.settings.state == GameState::Use {

        let mut highlight_color = game.config.color_light_grey;
        highlight_color.a = game.config.grid_alpha_overlay;

        let direction_color = Color::white();

        let mut attack_highlight_color = game.config.color_red;
        attack_highlight_color.a = game.config.grid_alpha_overlay;

        if UseAction::Interact == game.settings.use_action {
            for dir in Direction::move_actions().iter() {
                let target_pos = dir.offset_pos(player_pos, 1);
                if game.data.clear_path(player_pos, target_pos, false) {
                    panel.highlight_cmd(highlight_color, target_pos);

                    render_arrow(panel, tiles_key, *dir, target_pos, direction_color);
                }
            }
        } else if let UseAction::Item(item_class) = game.settings.use_action {
            render_overlay_use_item(panel, item_class, display_state, game);
        }
    }

    // draw direction overlays
    if game.config.draw_directional_arrow {
        let direction_color = Color::white();
        let mut index = 0;
        while index < game.data.entities.ids.len() {
            let entity_id = game.data.entities.ids[index];
            index += 1;

            let pos = game.data.entities.pos[&entity_id];

            if pos.x == -1 && pos.y == -1 {
                continue;
            }

            if game.data.entities.status[&entity_id].alive {
                if let Some(dir) = game.data.entities.direction.get(&entity_id) {
                    render_arrow(panel, tiles_key, *dir, pos, direction_color);
                }
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
               game.data.entities.status[&entity_id].alive {
               render_attack_overlay(panel,
                                     display_state,
                                     game,
                                     entity_id);
            }
        }
    }

    render_overlay_alertness(panel, tiles_key, game);
}

fn render_overlays(panel: &mut Panel,
                   display_state: &mut DisplayState,
                   game: &mut Game,
                   cursor_pos: Option<Pos>) {
    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    let player_pos = game.data.entities.pos[&player_id];

    let tiles_key = display_state.lookup_spritekey("tiles");

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
                        let sprite = Sprite::new(MAP_GROUND as u32, tiles_key);
                        panel.sprite_cmd(sprite, game.config.color_light_green, pos);
                    } else {
                        let sprite = Sprite::new(MAP_GROUND as u32, tiles_key);
                        panel.sprite_cmd(sprite, game.config.color_light_grey, pos);
                    }
                    let sprite = Sprite::new(('0' as u8 + res.abs() as u8) as u32, tiles_key);
                    panel.sprite_cmd(sprite, game.config.color_red, pos);
                }
            }
        }
    }

    // render cursor if enabled
    if game.config.use_cursor {
        // render cursor itself
        if let Some(cursor_pos) = game.settings.cursor {
            let time_since_toggle = display_state.time - display_state.time_of_cursor_toggle;
            let time_since_toggle = clampf(time_since_toggle, 0.0, game.config.cursor_fade_seconds);

            let mut color = game.config.color_mint_green;
            let percent = time_since_toggle / game.config.cursor_fade_seconds;
            color.a = (game.config.cursor_alpha as f32 * percent) as u8;

            let sprite = Sprite::new(ENTITY_CURSOR as u32, tiles_key);
            panel.sprite_cmd(sprite, color, cursor_pos);

            // render player ghost
            if cursor_pos != player_pos && game.input.target == None {

                let maybe_next_pos = astar_next_pos(&game.data.map, player_pos, cursor_pos, None, None);
                if let Some(next_pos) = maybe_next_pos {
                    let dxy = sub_pos(next_pos, player_pos);
                    let direction = Direction::from_dxy(dxy.x, dxy.y).unwrap();

                    let mut reach = reach_by_mode(MoveMode::Sneak);
                    if !game.input.cursor && game.input.shift {
                        reach = reach_by_mode(MoveMode::Run);
                    }

                    if let Some(player_ghost_pos) = reach.furthest_in_direction(player_pos, direction) {
                        render_entity_ghost(panel, player_id, player_ghost_pos, game, display_state);
                    }
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

                let is_in_fov =
                    game.data.pos_in_fov(player_id, pos, &game.config);
                if is_in_fov {
                    //tile_sprite.draw_char(panel, MAP_GROUND as char, pos, game.config.color_light_green);
                    let sprite = Sprite::new(MAP_GROUND as u32, tiles_key);
                    panel.sprite_cmd(sprite, game.config.color_light_green, pos);
                }
            }
        }
    }

    // draw attack and fov position highlights
    if let Some(mouse_xy) = cursor_pos {
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

    let mut highlight_color: Color = game.config.color_warm_grey;
    highlight_color.a = game.config.highlight_player_move;

    // draw mouse path overlays
    if let Some(mouse_id) = game.data.find_by_name(EntityName::Mouse) {
        let mouse_pos = game.data.entities.pos[&mouse_id];
        let player_pos = game.data.entities.pos[&player_id];

        if game.config.draw_star_path {
            // get a path to the mouse path, regardless of distance
            let path = astar_path(&game.data.map, player_pos, mouse_pos, None, None);
            for pos in path {
                //tile_sprite.draw_char(panel, MAP_EMPTY_CHAR as char, pos, highlight_color);
                let sprite = Sprite::new(MAP_EMPTY_CHAR as u32, tiles_key);
                panel.sprite_cmd(sprite, highlight_color, pos);
            }
        }

        if game.config.draw_mouse_line {
            // mouse pos at 0, 0 occurs when the mouse has not moved since startup.
            // this may cause a weirdness on the corner of the map
            if mouse_pos != Pos::new(0, 0) {
                let line = line(player_pos, mouse_pos).into_iter();
                for pos in line {
                    let pos = Pos::from(pos);
                    //tile_sprite.draw_char(panel, MAP_EMPTY_CHAR as char, pos, highlight_color);
                    let sprite = Sprite::new(MAP_EMPTY_CHAR as u32, tiles_key);
                    panel.sprite_cmd(sprite, highlight_color, pos);
                }
            }
        }
    }

    // Draw overlays if enabled
    if game.settings.overlay {
        // Draw player movement overlay
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

                    render_entity_ghost(panel, player_id, shadow_cursor_pos, game, display_state);
                }
            }
        }

        // Draw sound tiles overlay
        render_sound_overlay(panel, display_state, game);

        // Outline tiles within FOV for clarity
        render_fov_overlay(panel, display_state, game, player_id);
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

            //tile_sprite.draw_char(panel, , pos, adj_color);
            let sprite = Sprite::new(MAP_EMPTY_CHAR as u32, tiles_key);
            panel.sprite_cmd(sprite, adj_color, pos);

            panel.text_cmd(&format!("{}", near_count), highlight_color, pos);
        }
    }
}

fn render_overlay_alertness(panel: &mut Panel, sprite_key: SpriteKey, game: &mut Game) {
    let alertness_color = game.config.color_pink;
    let scale = 0.5;
    for entity_id in game.data.entities.ids.iter() {
        let pos = game.data.entities.pos[entity_id];

        if pos.x == -1 && pos.y == -1 {
            continue;
        }

        let mut status_drawn: bool = false;
        if let Some(status) = game.data.entities.status.get(entity_id) {
            if status.frozen > 0 {
                status_drawn = true;
                let sprite = Sprite::new(ASTERISK as u32, sprite_key);
                panel.sprite_scaled_cmd(sprite, scale,
                                        Some(Direction::UpRight),
                                        alertness_color,
                                        pos);
            }
        }

        if !status_drawn {
            if let Some(behavior) = game.data.entities.behavior.get(entity_id) {
                match behavior {
                    Behavior::Idle => {
                    }

                    Behavior::Investigating(_) => {
                        let sprite = Sprite::new(QUESTION_MARK as u32, sprite_key);
                        panel.sprite_scaled_cmd(sprite, scale,
                                                Some(Direction::UpRight),
                                                alertness_color,
                                                pos);
                    }

                    Behavior::Attacking(_) => {
                        let sprite = Sprite::new(EXCLAMATION_POINT as u32, sprite_key);
                        panel.sprite_scaled_cmd(sprite, scale,
                                                Some(Direction::UpRight),
                                                alertness_color,
                                                pos);
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
        (TileType::Wall, true) => config.color_light_brown,
        (TileType::Wall, false) => config.color_dark_brown,

        (TileType::Empty, true) => config.color_light_brown,

        (TileType::Empty, false) => config.color_dark_brown,

        (TileType::Water, true) => config.color_blueish_grey,
        (TileType::Water, false) => config.color_blueish_grey,

        (TileType::ShortWall, true) => config.color_light_brown,
        (TileType::ShortWall, false) => config.color_dark_brown,

        (TileType::Exit, true) => config.color_orange,
        (TileType::Exit, false) => config.color_red,
    };

    return color;
}

fn render_attack_overlay(panel: &mut Panel,
                         display_state: &mut DisplayState,
                         game: &mut Game,
                         entity_id: EntityId) {
    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    let player_pos = game.data.entities.pos[&player_id];

    let object_pos = game.data.entities.pos[&entity_id];

    let mut attack_highlight_color = game.config.color_red;
    attack_highlight_color.a = game.config.highlight_alpha_attack;

    let tiles_key = display_state.lookup_spritekey("tiles");

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
            let sprite = Sprite::new(MAP_EMPTY_CHAR as u32, tiles_key);
            panel.sprite_cmd(sprite, attack_highlight_color, position);
        }
    }
}

fn render_fov_overlay(panel: &mut Panel,
                         display_state: &mut DisplayState,
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
                panel.outline_cmd(highlight_color, map_pos);
            }
        }
    }
}

fn render_movement_overlay(panel: &mut Panel,
                           display_state: &mut DisplayState,
                           game: &mut Game,
                           entity_id: EntityId) {
    let player_id = game.data.find_by_name(EntityName::Player).unwrap();

    let entity_pos = game.data.entities.pos[&entity_id];

    let mut highlight_color = game.config.color_light_grey;
    highlight_color.a = game.config.grid_alpha_overlay;

    let tiles_key = display_state.lookup_spritekey("tiles");

    if let Some(reach) = game.data.entities.movement.get(&entity_id) {
        for move_pos in reach.reachables(entity_pos) {
            let visible = game.data.pos_in_fov(player_id, move_pos, &game.config);
            if visible {
                let chr = game.data.entities.chr[&entity_id];

                let sprite = Sprite::new(chr as u32, tiles_key);
                panel.sprite_cmd(sprite, highlight_color, move_pos);
            }
        }
    }
}

fn render_entity_ghost(panel: &mut Panel,
                       entity_id: EntityId,
                       render_pos: Pos,
                       game: &mut Game,
                       display_state: &mut DisplayState) {
    let entity_pos = game.data.entities.pos[&entity_id];

    let alpha = game.data.entities.color[&entity_id].a;
    game.data.entities.color[&entity_id].a = game.config.ghost_alpha;

    game.data.entities.pos[&entity_id] = render_pos;

    // a little ugly, but set the delta time to 0 so the render_entity function does not
    // step the animation forward when rendering a ghost.
    let dt = display_state.dt;
    display_state.dt = 0.0;
    render_entity(panel, entity_id, display_state, game);
    display_state.dt = dt;

    game.data.entities.color[&entity_id].a = alpha;
    game.data.entities.pos[&entity_id] = entity_pos;
}

fn render_arrow(panel: &mut Panel,
                sprite_key: SpriteKey,
                dir: Direction,
                pos: Pos,
                direction_color: Color) {
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

    let mut sprite = Sprite::new(sprite_index as u32, sprite_key);
    sprite.rotation = rotation;
    panel.sprite_cmd(sprite, direction_color, pos);
}

