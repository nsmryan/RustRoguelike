use logging_timer::timer;

use roguelike_utils::line::line;
use roguelike_utils::rng::*;
use roguelike_utils::comp::*;

use roguelike_core::types::*;
use roguelike_core::map::*;
use roguelike_core::constants::*;
use roguelike_core::movement::*;
use roguelike_core::config::*;
use roguelike_core::utils::*;
use roguelike_core::ai::*;

use roguelike_engine::game::*;

use crate::display::*;
use crate::drawcmd::*;
use crate::animation::{Sprite, SpriteKey, Effect, Animation, AnimationResult, Particle};


pub fn render_all(panels: &mut Panels, display_state: &mut DisplayState, sprites: &Vec<SpriteSheet>, game: &Game, dt: f32) -> Result<(), String> {
    display_state.show_debug("ids", format!("{}", display_state.ids.len()));

    display_state.dt = dt;
    display_state.time += dt;

    /* Draw Background */
    render_background(panels.get_mut(&PanelName::Map).unwrap(), &game.level.map, sprites);

    /* Draw Map */
    render_panels(panels, display_state, game, sprites);

    /* Draw Debug Overlay */
    if game.settings.debug_enabled {
        render_debug(panels.get_mut(&PanelName::Map).unwrap(), display_state);
    }

    let menu_panel = panels.get_mut(&PanelName::Menu).unwrap();

    if display_state.state == GameState::Inventory {
        render_inventory(menu_panel, display_state);
    } else if display_state.state == GameState::SkillMenu {
        render_skill_menu(menu_panel, display_state);
    } else if display_state.state == GameState::ClassMenu {
        render_class_menu(menu_panel);
    } else if display_state.state == GameState::ConfirmQuit {
        render_confirm_quit(menu_panel);
    }

    Ok(())
}


fn render_panels(panels: &mut Panels, display_state: &mut DisplayState, game: &Game, sprites: &Vec<SpriteSheet>) {
    let panel = &mut panels.get_mut(&PanelName::Map).unwrap();

    {
        let _map = timer!("MAP");
        render_map(panel, &game.level.map, sprites);
    }

    {
        let _mid = timer!("MID");
        render_entity_type(panel, EntityType::Environment, display_state, &game.config, sprites);
        render_entity_type(panel, EntityType::Trigger, display_state, &game.config, sprites);
        render_entity_type(panel, EntityType::Item, display_state, &game.config, sprites);

        render_map_middle(panel, &game.level.map, &game.config, sprites);
    }

    {
        let _above = timer!("ABOVE");
        render_entity_type(panel, EntityType::Energy, display_state, &game.config, sprites);
        render_entity_type(panel, EntityType::Enemy, display_state, &game.config, sprites);
        render_entity_type(panel, EntityType::Column, display_state, &game.config, sprites);
        render_entity_type(panel, EntityType::Player, display_state, &game.config, sprites);
        render_entity_type(panel, EntityType::Other, display_state, &game.config, sprites);
    }

    {
        let _overlays_game = timer!("OVERLAYSGAME");
        render_game_overlays(panel, display_state, &game.level, &game.settings, &game.config, sprites);
    }

    {
        let _mapabove = timer!("MAPABOVE");
        render_map_above(panel, display_state, &game.level.map, &game.config, sprites);
    }

    {
        let _extra = timer!("EXTRA");
        render_impressions(panel, display_state, &game.config);
        render_effects(panel, display_state, &game.level, &game.config, sprites);
        render_overlays(panel, display_state, game, sprites);
    }

    /* Draw Player Info */
    {
        let player_panel = &mut panels.get_mut(&PanelName::Player).unwrap();
        render_player_info(player_panel, display_state, &game.settings);
    }

    /* Draw Inventory */
    {
        let inventory_panel = &mut panels.get_mut(&PanelName::Inventory).unwrap();
        render_inventory(inventory_panel, display_state);
    }

    /* Draw Game Info */
    {
        let info_panel = &mut panels.get_mut(&PanelName::Info).unwrap();
        render_info(info_panel, display_state, &game.level);
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
fn render_placard(panel: &mut Panel, text: &str) {
    // Draw header text
    let text_color = Color::new(0, 0, 0, 255);
    let highlight_color = Color::new(0xcd, 0xb4, 0x96, 255);

    panel.rect_cmd(Pos::new(0, 0), panel.cells, 0.5, false, highlight_color);
    panel.justify_cmd(text,
                      Justify::Center,
                      text_color,
                      highlight_color,
                      Pos::new(0, 0),
                      panel.cells.0);
}

fn render_pips(panel: &mut Panel,
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

fn render_bar(panel: &mut Panel,
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

fn render_player_info(panel: &mut Panel, display_state: &DisplayState, settings: &GameSettings) {
    render_placard(panel, "Player");

    let player_id = display_state.player_id();

    let mut list: Vec<String> = Vec::new();

    // TODO this color comes from the UI mockups as a light brown
    let color = Color::new(0xcd, 0xb4, 0x96, 255);

    let x_offset = 3;

    if let Some(hp) = display_state.hp.get(&player_id) {
        let max_hp = display_state.max_hp[&player_id];
        let current_hp = if *hp > 0 {
            *hp
        } else {
            0
        };
        // TODO this color red comes from the UI mockups
        let health_color = Color::new(0x96, 0x54, 0x56, 255);
        let bar_pos = Pos::new(1, 2);
        render_bar(panel, max_hp, current_hp, bar_pos, health_color, Color::white(), false);
    }

    let energy = display_state.energy[&player_id];
    // TODO this color orange comes from the UI mockups
    let energy_color = Color::new(0xaf, 0x83, 0x56, 255);
    render_pips(panel, energy, Pos::new(1, 3), energy_color);

    list.push(format!(""));


    let stance = display_state.stance[&player_id];
    list.push(format!("{}", stance));
    list.push("next move".to_string());
    let stance = settings.move_mode;
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
    list.push(format!("turn {}", settings.turn_count));

    let text_pos = Pos::new(x_offset, 5);

    panel.text_list_cmd(&list, color, text_pos);
}

fn render_info(panel: &mut Panel, display_state: &mut DisplayState, level: &Level) {
    render_placard(panel, "Info");

    if let Some(info_pos) = display_state.cursor_pos {
        let x_offset = 5;

        let text_color = Color::new(0xcd, 0xb4, 0x96, 255);

        let object_ids = display_state.entities_at_cursor.clone();

        let mut y_pos = 1;

        let mut text_list = Vec::new();

        text_list.push(format!("({:>2},{:>2})", info_pos.x, info_pos.y));

        let text_pos = Pos::new(x_offset, y_pos);

        panel.text_list_cmd(&text_list, text_color, text_pos);

        text_list.clear();

        y_pos += 1;

        let mut drawn_info = false;

        // only display first object
        if let Some(obj_id) = object_ids.first() {
            let entity_in_fov = display_state.entity_is_in_fov(*obj_id) == FovResult::Inside;

            // only display things in the player's FOV
            if entity_in_fov {
                drawn_info = true;

                if let Some(hp) = display_state.hp.get(obj_id) {
                    y_pos += 1;

                    let health_color = Color::new(0x96, 0x54, 0x56, 255);
                    let max_hp = display_state.max_hp[obj_id];
                    render_bar(panel,
                               max_hp,
                               *hp,
                               Pos::new(1, y_pos),
                               health_color,
                               Color::white(),
                               false);

                    y_pos += 1;
                }

                text_list.push(format!("{:?}", display_state.name[obj_id]));

                // show facing direction for player and monsters
                if display_state.typ[obj_id] == EntityType::Player ||
                   display_state.typ[obj_id] == EntityType::Enemy {
                    if let Some(direction) = display_state.direction.get(obj_id) {
                        text_list.push(format!("Facing {}", direction));
                    }
                }

                if matches!(display_state.hp.get(obj_id), Some(0)) {
                    text_list.push(format!("  {}", "dead"));
                } else if let Some(behave) = display_state.behavior.get(obj_id) {
                    text_list.push(format!("currently {}", behave));
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

        let text_pos = Pos::new(x_offset, y_pos);
        panel.text_list_cmd(&text_list, text_color, text_pos);

        let tile_in_fov = display_state.fov[&info_pos];
        if tile_in_fov == FovResult::Inside {
            if level.map[info_pos].tile_type == TileType::Water {
                text_list.push("Tile is water".to_string());
            } else {
                text_list.push(format!("Tile is {:?}",  level.map[info_pos].surface));
            }

            if level.map[info_pos].bottom_wall != Wall::Empty {
                text_list.push("Lower wall".to_string());
            }

            if level.map.is_within_bounds(move_x(info_pos, 1)) &&
               level.map[move_x(info_pos, 1)].left_wall != Wall::Empty {
                text_list.push("Right wall".to_string());
            }

            if level.map.is_within_bounds(move_y(info_pos, -1)) &&
               level.map[move_y(info_pos, -1)].bottom_wall != Wall::Empty {
                text_list.push("Top wall".to_string());
            }

            if level.map[info_pos].left_wall != Wall::Empty {
                text_list.push("Left wall".to_string());
            }

            if level.map[info_pos].block_move {
                text_list.push(format!("blocked"));
            }
        }

        panel.text_list_cmd(&text_list, text_color, text_pos);
    }
}

fn render_skill_menu(panel: &mut Panel, display_state: &DisplayState) {
    // Render header
    render_placard(panel, "Skills");

    let mut list = Vec::new();

    for (index, skill) in display_state.skills.iter().enumerate() {
        list.push(format!("{} {:?}", index, skill));
    }

    let y_pos = 2;
    let text_pos = Pos::new(2, y_pos);

    // TODO this color comes from the ui mockups as a light brown
    let color = Color::new(0xcd, 0xb4, 0x96, 255);

    panel.text_list_cmd(&list, color, text_pos);
}

fn render_class_menu(panel: &mut Panel) {
    // Render header
    render_placard(panel, "Choose Class");

    let mut list = Vec::new();

    for (index, class) in EntityClass::classes().iter().enumerate() {
        list.push(format!("{} {:?}", index, class));
    }

    let y_pos = 2;
    let text_pos = Pos::new(2, y_pos);

    // TODO this color comes from the ui mockups as a light brown
    let color = Color::new(0xcd, 0xb4, 0x96, 255);

    panel.text_list_cmd(&list, color, text_pos);
}

fn render_confirm_quit(panel: &mut Panel) {
    // Render header
    render_placard(panel, "Quit?");

    let mut list = Vec::new();

    list.push("q: quit game".to_string());
    list.push("".to_string());
    list.push("esc: continue".to_string());
    list.push("".to_string());
    list.push("r: restart".to_string());

    let y_pos = 2;
    let text_pos = Pos::new(5, y_pos);

    // TODO this color comes from the UI mockups as a light brown
    let color = Color::new(0xcd, 0xb4, 0x96, 255);

    panel.text_list_cmd(&list, color, text_pos);
}

/// Render an inventory section within the given area
fn render_inventory(panel: &mut Panel, display_state: &DisplayState) {
    // Render header
    render_placard(panel, "Inventory");

    // TODO this color comes from the UI mockups as a light brown
    let ui_color = Color::new(0xcd, 0xb4, 0x96, 255);

    // Render each object's name in inventory
    let mut y_pos = 2;
    let x_offset = 5;

    // Draw Primary Items
    panel.text_cmd("z", ui_color, Pos::new(x_offset, y_pos));

    for (item, item_class) in display_state.inventory.iter() {
        if *item_class == ItemClass::Primary {
            let item_text = format!("{:?}", item);
            let text_pos = Pos::new(x_offset + 2, y_pos);
            panel.text_cmd(&item_text, ui_color, text_pos);
            break;
        }
    }
    y_pos += 1;

    // Draw Consumable Items
    panel.text_cmd(&"x", ui_color, Pos::new(x_offset, y_pos));

    for (item, item_class) in display_state.inventory.iter() {
        if *item_class == ItemClass::Consumable {
            let item_text = format!("{:?}", item);
            let text_pos = Pos::new(x_offset + 2, y_pos);
            panel.text_cmd(&item_text, ui_color, text_pos);
            break;
        }
    }
    y_pos += 1;

    // Draw Stones Items
    panel.text_cmd(&"c", ui_color, Pos::new(x_offset, y_pos));

    let mut num_stones = 0;
    for (item, _item_class) in display_state.inventory.iter() {
        if *item == Item::Stone {
            num_stones += 1;
        }
    }

    if num_stones > 0 {
        panel.text_cmd(&"c", ui_color, Pos::new(x_offset, y_pos));

        let text_pos = Pos::new(x_offset + 2, y_pos);
        panel.text_cmd(&"stone", ui_color, text_pos);

        let num_text = format!("({})", num_stones);
        panel.text_cmd(&num_text, ui_color, Pos::new(x_offset + 8, y_pos));
    }

    y_pos += 1;
    y_pos += 1;

    // Draw Remaining Items
    for (item, item_class) in display_state.inventory.iter() {
        if *item_class == ItemClass::Misc && *item != Item::Stone {
            let item_text = format!("{:?}", item);
            let text_pos = Pos::new(x_offset + 2, y_pos);
            panel.text_cmd(&item_text, ui_color, text_pos);
            y_pos += 1;
        }
    }
}

/// render the background files, including water tiles
fn render_background(panel: &mut Panel, map: &Map, sprites: &Vec<SpriteSheet>) {
    let (map_width, map_height) = map.size();

    let sprite_key = lookup_spritekey(sprites, "tiles");

    for y in 0..map_height {
        for x in 0..map_width {
            let map_pos = Pos::new(x, y);

            let tile = &map[(x, y)];
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
            // Grass is animated now
            //return Some(MAP_GRASS);
            return None;
        }

        Surface::Floor => {
            // Nothing to draw
            return None;
        }
    }
}

/// Render Wall Shadows (full tile and intertile walls, left and down)
fn render_wall_shadow(panel: &mut Panel, pos: Pos, map: &Map, sprites: &Vec<SpriteSheet>, shadow_color: Color) {
    let shadow_sprite_key = lookup_spritekey(sprites, "shadows");

    let tile = map[pos];

    let (_map_width, map_height) = map.size();
    let (x, y) = pos.to_tuple();

    let left_valid = x - 1 > 0;
    let down_valid = y + 1 < map_height;
    let down_left_valid = left_valid && down_valid;
    let left_wall = left_valid && map[(x - 1, y)].tile_type == TileType::Wall;
    let down_wall = down_valid && map[(x, y + 1)].tile_type == TileType::Wall;
    let down_left_wall = down_left_valid && map[(x - 1, y + 1)].tile_type == TileType::Wall;

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

fn render_map_above(panel: &mut Panel, display_state: &DisplayState, map: &Map, config: &Config, sprites: &Vec<SpriteSheet>) {
    let (map_width, map_height) = map.size();

    let sprite_key = lookup_spritekey(sprites, "tiles");
    for y in 0..map_height {
        for x in 0..map_width {
            let pos = Pos::new(x, y);
            /* draw the between-tile walls appropriate to this tile */
            {
                let tile = map[pos];
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

            let fov_result = display_state.pos_is_in_fov(pos);

            // apply a FoW darkening to cells
            if config.fog_of_war && fov_result != FovResult::Inside {
                let is_in_fov_ext = fov_result == FovResult::Edge;

                let mut blackout_color = Color::black();
                let sprite = Sprite::new(MAP_EMPTY_CHAR as u32, sprite_key);
                if is_in_fov_ext {
                    blackout_color.a = config.fov_edge_alpha;
                    panel.sprite_cmd(sprite, blackout_color, pos);
                } else if map[pos].explored {
                    blackout_color.a = config.explored_alpha;
                    panel.sprite_cmd(sprite, blackout_color, pos);
                } else {
                    panel.fill_cmd(pos, blackout_color);
                }
            }
        }
    }
}

fn render_map_middle(panel: &mut Panel, map: &Map, config: &Config, sprites: &Vec<SpriteSheet>) {
    let (map_width, map_height) = map.size();

    let sprite_key = lookup_spritekey(sprites, "tiles");
    for y in 0..map_height {
        for x in 0..map_width {
            let pos = Pos::new(x, y);

            let shadow_color = config.color_shadow;
            render_wall_shadow(panel, pos, map, sprites, shadow_color);

            /* draw the between-tile walls appropriate to this tile */
            render_intertile_walls_below(panel, map, sprite_key, pos);
        }
    }
}

/// Render the map, with environment and walls
fn render_map(panel: &mut Panel, map: &Map, sprites: &Vec<SpriteSheet>) {
    let (map_width, map_height) = map.size();

    let sprite_key = lookup_spritekey(sprites, "tiles");
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
            let tile = map[pos];

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
                  level: &Level,
                  config: &Config,
                  sprites: &Vec<SpriteSheet>) {
    let player_id = level.find_by_name(EntityName::Player).unwrap();

    let mut index = 0;
    while index < display_state.effects.len() {
        let mut effect_complete = false;

        // NOTE(perf) cloning the effect vector each iteration!
        let mut effect = display_state.effects[index].clone();
        match &mut effect {
            Effect::Particles(rate, particles) => {
                if particles.len() < config.max_particles && rng_trial(&mut display_state.rng, *rate) {
                    let size = (panel.num_pixels.0 as i32, panel.num_pixels.1 as i32);
                    let pos = rng_pos(&mut display_state.rng, size);
                    particles.push(Particle::new(config.particle_duration, pos));
                }

                let sprite_key = lookup_spritekey(sprites, "particle_speck");

                let dims = panel.cell_dims();
                let mut index = 0;
                while index < particles.len() {
                    let cell = panel.cell_from_pixel(particles[index].pos);
                    particles[index].duration -= display_state.dt;

                    // if the particle is finished, or has left the map, remove it.
                    if particles[index].duration < 0.0 || !level.map.is_within_bounds(cell) {
                        particles.swap_remove(index);
                    } else {
                        // offset the particle according to how long it has been running.
                        let x_offset = (dims.0 as f32 * (config.particle_duration - particles[index].duration)) as i32;
                        let draw_pos = move_x(particles[index].pos, x_offset);
                        let draw_cell = panel.cell_from_pixel(draw_pos);

                        if level.map.is_within_bounds(draw_cell) && 
                           level.pos_in_fov(player_id, draw_cell, config) {
                            let mut color = Color::white();
                            // fade the particle out according to how long it has been running.
                            color.a = (255.0 * (particles[index].duration / config.particle_duration)) as u8;
                            let sprite = Sprite::new(0, sprite_key);
                            panel.sprite_at_pixel_cmd(sprite, color, draw_pos);
                        }
                        index += 1;
                    }
                }
            }

            Effect::Sound(sound_aoe, sound_dt) => {
                let mut highlight_color = config.color_warm_grey;

                let radius = sound_aoe.positions.len();
                let sound_interval = config.sound_timeout / radius as f32;
                let cur_dist = *sound_dt / sound_interval;
                for (dist, dist_positions) in sound_aoe.positions.iter().enumerate() {
                    highlight_color.a =
                        config.sound_alpha / ((dist as i16 - cur_dist as i16).abs() as u8 + 1);

                    for pos in dist_positions.iter() {
                        if !level.map[*pos].block_move &&
                           level.pos_in_fov(player_id, *pos, config) {
                           panel.highlight_cmd(highlight_color, *pos);
                           panel.outline_cmd(highlight_color, *pos);
                        }
                    }
                }

                if *sound_dt >= config.sound_timeout {
                    effect_complete = true;
                } else {
                    *sound_dt += 1.0 / config.frame_rate as f32;
                    if *sound_dt > config.sound_timeout {
                        *sound_dt = config.sound_timeout;
                    }
                }
            }

            Effect::Beam(remaining, start, end) => {
                let sprite_key = lookup_spritekey(sprites, "tiles");

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
                 color: Option<Color>,
                 sprites: &Vec<SpriteSheet>) -> Option<Sprite> {
    let mut animation_result = AnimationResult::new();

    let pos = display_state.pos[&entity_id];
    animation_result.pos = pos;

    // only draw if within the map (outside is (-1, -1) like if in inventory)
    // and not about to be removed.
    if pos.x < 0 || pos.y < 0 {
        return None;
    }

    let color = color.unwrap_or(Color::new(255, 255, 255, 255));

    let is_in_fov =
       display_state.entity_is_in_fov(entity_id) == FovResult::Inside;

    if is_in_fov {
        if let Some(anims) = display_state.animations.get_mut(&entity_id) {
            if let Some(anim) = anims.pop_front() {
                animation_result = anim.status(pos);

                if let Animation::PlayEffect(effect) = anim {
                    display_state.play_effect(effect);
                } else {
                    if let Some(sprite) = animation_result.sprite {
                        panel.sprite_cmd(sprite, color, animation_result.pos);
                    }

                    // for animations other then effects, keep playing by pushing to front of
                    // animation sequence.
                    if !animation_result.done {
                        anims.push_front(anim);
                    }
                }
            }
        } else {
            let tiles = lookup_spritekey(sprites, "tiles");
            let chr = display_state.chr[&entity_id];
            let sprite = Sprite::new(chr as u32, tiles);

            panel.sprite_cmd(sprite, color, pos);
            animation_result.sprite = Some(sprite);
        }
    } else {
        // if not in FoV, see if we need to add an impression for a golem
        if display_state.typ[&entity_id] == EntityType::Enemy {
            if display_state.entity_is_in_fov(entity_id) == FovResult::Edge {
                if display_state.impressions.iter().all(|impresssion| impresssion.pos != pos) {
                    let tiles = lookup_spritekey(sprites, "tiles");
                    let impression_sprite = Sprite::new(ENTITY_UNKNOWN as u32, tiles);
                    display_state.impressions.push(Impression::new(impression_sprite, pos));
                }
            }
        }
    }

    return animation_result.sprite;
}

fn render_impressions(panel: &mut Panel, display_state: &mut DisplayState, config: &Config) {
    // check for entities that have left FOV and make an impression for them
    let mut index = 0;
    while index < display_state.impressions.len() {
        let impression = display_state.impressions[index];
        index += 1;
        panel.sprite_cmd(impression.sprite,
                                 config.color_light_grey,
                                 impression.pos);
    }
}

fn render_entity_type(panel: &mut Panel, typ: EntityType, display_state: &mut DisplayState, config: &Config, sprites: &Vec<SpriteSheet>) {
    if typ == EntityType::Player && display_state.state == GameState::Use && display_state.use_dir.is_some() {
        // For the player in use-mode, while holding down a direction, we
        // need special rendering. Otherwise the player is rendered as normal.

        let player_id = display_state.player_id();
        let player_pos = display_state.pos[&player_id];

        let use_pos;
        if let Some(use_dir) = display_state.use_dir {
            let target_pos = use_dir.offset_pos(player_pos, 1);
            use_pos = Some(target_pos);
        } else {
            use_pos = display_state.use_pos;
        }

        if let Some(pos) = use_pos {
            render_entity_ghost(panel, player_id, player_pos, config, display_state, sprites);
            display_state.pos[&player_id] = pos;
            render_entity(panel, player_id, display_state, None, sprites);
            display_state.pos[&player_id] = player_pos;
        }
    } else {
        let mut index = 0;
        while index < display_state.ids.len() {
            let entity_id = display_state.ids[index];
            index += 1;

            if display_state.typ[&entity_id] == typ {
                let maybe_sprite = render_entity(panel, entity_id, display_state, None, sprites);

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
                           config: &Config,
                           sprites: &Vec<SpriteSheet>) {
    let player_id = display_state.player_id();
    let player_pos = display_state.pos[&player_id];

    let direction_color = Color::white();

    let mut attack_highlight_color = config.color_red;
    attack_highlight_color.a = config.grid_alpha_overlay;

    let mut highlight_color = config.color_light_grey;
    highlight_color.a = config.grid_alpha_overlay;

    let sprite_key = lookup_spritekey(sprites, "tiles");

    if display_state.inventory.iter().position(|(_item, class)| *class == item_class).is_some() {
        if let Some(use_dir) = display_state.use_dir {
            if let Some(_use_pos) = display_state.use_pos {
                let arrow_pos = use_dir.offset_pos(player_pos, 1);
                render_arrow(panel, sprite_key, use_dir, arrow_pos, direction_color);

                for hit_pos in display_state.hit_positions.iter() {
                   panel.highlight_cmd(attack_highlight_color, *hit_pos);
                }
            }
        } else {
            for (use_pos, use_dir) in display_state.use_dirs.iter() {
                // skip player positions for highlights
                if *use_pos != player_pos {
                    panel.highlight_cmd(highlight_color, *use_pos);
                }

                let arrow_pos = use_dir.offset_pos(player_pos, 1);
                render_arrow(panel, sprite_key, *use_dir, arrow_pos, direction_color);
            }

            for hit_pos in display_state.hit_positions.iter() {
                // skip the player's position for attack highlights.
                if *hit_pos != player_pos {
                   panel.highlight_cmd(attack_highlight_color, *hit_pos);
                }
            }
        }
    }
}

fn render_sound_overlay(panel: &mut Panel,
                        display_state: &mut DisplayState,
                        config: &Config) {
    let mut highlight_sound: Color = config.color_warm_grey;
    highlight_sound.a = config.sound_alpha;
    // NOTE(perf) this clone is only necessary because draw commands
    // mut borrow the entire display state, instead of only the draw_cmd hashmap or
    // even just the vec of commands.
    for pos in display_state.sound_tiles.clone().iter() {
        // NOTE this currently does not take into account FOV!
        panel.highlight_cmd(highlight_sound, *pos);
    }
}

fn render_game_overlays(panel: &mut Panel,
                        display_state: &mut DisplayState,
                        level: &Level,
                        settings: &GameSettings,
                        config: &Config,
                        sprites: &Vec<SpriteSheet>) {
    let player_id = display_state.player_id();
    let player_pos = display_state.pos[&player_id];

    let tiles_key = lookup_spritekey(sprites, "tiles");

    if config.use_cursor {
        if let Some(cursor_pos) = display_state.cursor_pos {
            // render trigger plate wall highlight if selected
            for entity in display_state.entities_at_cursor.iter() {
                if display_state.name[&entity] == EntityName::GateTrigger {
                    let gate_pos = level.entities.gate_pos[&entity];
                    let mut highlight_color: Color = config.color_red;
                    highlight_color.a = 100;
                    panel.highlight_cmd(highlight_color, gate_pos);
                } else if display_state.name[&entity] == EntityName::FreezeTrap {
                    let trap_pos = display_state.pos[&entity];
                    let freeze_aoe =
                        aoe_fill(&level.map, AoeEffect::Freeze, trap_pos, config.freeze_trap_radius, config);
                    for pos in freeze_aoe.positions() {
                        let mut highlight_color: Color = config.color_blueish_grey;
                        highlight_color.a = 100;
                        panel.highlight_cmd(highlight_color, pos);
                    }
                }
            }

            // TODO track cursor pos and state with message and use here instead
            // render some extra player information if cursor is over player's tile
            if cursor_pos == player_pos {
                // Draw sound tiles overlay
                render_sound_overlay(panel, display_state, config);
            }
        }
    }

    // Draw use-mode overlay
    if display_state.state == GameState::Use {

        let mut highlight_color = config.color_light_grey;
        highlight_color.a = config.grid_alpha_overlay;

        let direction_color = Color::white();

        let mut attack_highlight_color = config.color_red;
        attack_highlight_color.a = config.grid_alpha_overlay;

        if UseAction::Interact == settings.use_action {
            for use_pos in display_state.use_pos.iter() {
                panel.highlight_cmd(highlight_color, *use_pos);

                let dir = Direction::from_positions(player_pos, *use_pos).unwrap();
                render_arrow(panel, tiles_key, dir, *use_pos, direction_color);
            }
        } else if let UseAction::Item(item_class) = settings.use_action {
            render_overlay_use_item(panel, item_class, display_state, &config, sprites);
        }
    }

    // draw direction overlays
    if config.draw_directional_arrow {
        let direction_color = Color::white();
        let mut index = 0;
        while index < display_state.ids.len() {
            let entity_id = display_state.ids[index];
            index += 1;

            let pos = display_state.pos[&entity_id];

            if pos.x == -1 && pos.y == -1 {
                continue;
            }

            if let Some(dir) = display_state.direction.get(&entity_id) {
                render_arrow(panel, tiles_key, *dir, pos, direction_color);
            }
        }
    }

    // render attack overlay highlighting squares that an entity can attack
    if settings.overlay {
        let keys = display_state.ids.iter().map(|id| *id).collect::<Vec<EntityId>>();
        for entity_id in keys {
            let pos = display_state.pos[&entity_id];

            if pos.x >= 0 && pos.y >= 0 && entity_id != player_id {
               render_attack_overlay(panel,
                                     config,
                                     display_state,
                                     entity_id,
                                     sprites);
            }
        }
    }

    render_overlay_alertness(panel, display_state, tiles_key, &level.entities, config);
}

fn render_overlay_direction(panel: &mut Panel,
                            display_state: &mut DisplayState,
                            map: &Map,
                            config: &Config,
                            tiles_key: SpriteKey) {
    let map_width = map.width();
    let map_height = map.height();

    let player_id = display_state.player_id();
    let player_pos = display_state.pos[&player_id];

    for y in 0..map_height {
        for x in 0..map_width {
            let pos = Pos::new(x, y);
            let x_diff = x - player_pos.x;
            let y_diff = y - player_pos.y;

            if x_diff.abs() < 5 && y_diff.abs() < 5 {
                let res: i8 = x_diff as i8 - y_diff as i8;
                if res <= 0 {
                    let sprite = Sprite::new(MAP_GROUND as u32, tiles_key);
                    panel.sprite_cmd(sprite, config.color_light_green, pos);
                } else {
                    let sprite = Sprite::new(MAP_GROUND as u32, tiles_key);
                    panel.sprite_cmd(sprite, config.color_light_grey, pos);
                }
                let sprite = Sprite::new(('0' as u8 + res.abs() as u8) as u32, tiles_key);
                panel.sprite_cmd(sprite, config.color_red, pos);
            }
        }
    }
}

fn render_overlay_cursor(panel: &mut Panel, display_state: &mut DisplayState, game: &Game, sprites: &Vec<SpriteSheet>) {
    let player_id = display_state.player_id();
    let player_pos = display_state.pos[&player_id];

    let tiles_key = lookup_spritekey(sprites, "tiles");

    // render cursor itself
    if let Some(cursor_pos) = display_state.cursor_pos {
        let time_since_toggle = display_state.time - display_state.time_of_cursor_toggle;
        let time_since_toggle = clampf(time_since_toggle, 0.0, game.config.cursor_fade_seconds);

        let mut color = game.config.color_mint_green;
        let percent = time_since_toggle / game.config.cursor_fade_seconds;
        color.a = (game.config.cursor_alpha as f32 * percent) as u8;

        let sprite = Sprite::new(ENTITY_CURSOR as u32, tiles_key);
        panel.sprite_cmd(sprite, color, cursor_pos);

        // render player ghost
        if cursor_pos != player_pos && game.input.target == None {

            let maybe_next_pos = astar_next_pos(&game.level.map, player_pos, cursor_pos, None, None);
            if let Some(next_pos) = maybe_next_pos {
                let dxy = sub_pos(next_pos, player_pos);
                let direction = Direction::from_dxy(dxy.x, dxy.y).unwrap();

                let mut reach = reach_by_mode(MoveMode::Sneak);
                if !game.input.cursor && game.input.shift {
                    reach = reach_by_mode(MoveMode::Run);
                }

                if let Some(player_ghost_pos) = reach.furthest_in_direction(player_pos, direction) {
                    render_entity_ghost(panel, player_id, player_ghost_pos, &game.config, display_state, sprites);
                }
            }
        }
    }
}

fn render_overlay_fov(panel: &mut Panel,
                      display_state: &mut DisplayState,
                      map: &Map,
                      config: &Config,
                      tiles_key: SpriteKey) {
    let map_width = map.width();
    let map_height = map.height();
    for y in 0..map_height {
        for x in 0..map_width {
            let pos = Pos::new(x, y);

            if display_state.pos_is_in_fov(pos) == FovResult::Inside {
                let sprite = Sprite::new(MAP_GROUND as u32, tiles_key);
                panel.sprite_cmd(sprite, config.color_light_green, pos);
            }
        }
    }
}

fn render_overlay_attack(panel: &mut Panel,
                         display_state: &mut DisplayState,
                         level: &Level,
                         config: &Config,
                         sprites: &Vec<SpriteSheet>) {
    let player_id = display_state.player_id();

    // Draw monster attack overlay
    for entity_id in display_state.entities_at_cursor.clone() {
        let pos = display_state.pos[&entity_id];

        if display_state.pos_is_in_fov(pos) == FovResult::Inside &&
           entity_id != player_id &&
           display_state.typ[&entity_id] == EntityType::Enemy {
           render_attack_overlay(panel, config, display_state, entity_id, sprites);
           render_fov_overlay(panel, display_state, level, config, entity_id);
           render_movement_overlay(panel, config, display_state, entity_id, sprites);
        }
    }
}

fn render_overlay_floodfill(panel: &mut Panel,
                            level: &Level,
                            config: &Config,
                            tiles_key: SpriteKey) {
    let mut highlight_color = config.color_light_orange;

    highlight_color.a = 50;
    let fill_metric = map_fill_metric(&level.map);

    for (pos, near_count) in fill_metric {
        let amount = near_count as f32 / 50.0;
        let adj_color = lerp_color(config.color_ice_blue, config.color_red, amount);

        //tile_sprite.draw_char(panel, , pos, adj_color);
        let sprite = Sprite::new(MAP_EMPTY_CHAR as u32, tiles_key);
        panel.sprite_cmd(sprite, adj_color, pos);

        panel.text_cmd(&format!("{}", near_count), highlight_color, pos);
    }
}

fn render_overlays(panel: &mut Panel,
                   display_state: &mut DisplayState,
                   game: &Game,
                   sprites: &Vec<SpriteSheet>) {
    let player_id = display_state.player_id();

    let tiles_key = lookup_spritekey(sprites, "tiles");

    // render a grid of numbers if enabled
    if game.config.overlay_directions {
        render_overlay_direction(panel, display_state, &game.level.map, &game.config, tiles_key);
    }

    // render cursor if enabled
    if game.config.use_cursor {
        render_overlay_cursor(panel, display_state, game, sprites);
    }

    // render FOV if enabled
    if game.config.overlay_player_fov {
        render_overlay_fov(panel, display_state, &game.level.map, &game.config, tiles_key);
    }

    // draw attack and fov position highlights
    if let Some(_cursor_pos) = display_state.cursor_pos {
        render_overlay_attack(panel, display_state, &game.level, &game.config, sprites);
    }

    let mut highlight_color: Color = game.config.color_warm_grey;
    highlight_color.a = game.config.highlight_player_move;

    // Draw overlays if enabled
    if game.settings.overlay {
        // Draw player movement overlay
        let player_pos = display_state.pos[&player_id];
        for move_pos in display_state.entity_movements[&player_id].clone() {
            if move_pos != player_pos {
                let dxy = sub_pos(move_pos, player_pos);
                let direction = Direction::from_dxy(dxy.x, dxy.y).unwrap();
                let shadow_cursor_pos = direction.offset_pos(player_pos, 1);

                render_entity_ghost(panel, player_id, shadow_cursor_pos, &game.config, display_state, sprites);
            }
        }

        // Draw sound tiles overlay
        render_sound_overlay(panel, display_state, &game.config);

        // Outline tiles within FOV for clarity
        render_fov_overlay(panel, display_state, &game.level, &game.config, player_id);
    }

    // NOTE floodfill ranges:
    // 4-5 is the end of a cooridor
    // < 14 is pretty enclosed
    // 14-24 is within the corner of a building or at an opening from an enclosed space
    // low 30s tend to be next to a shear wall
    // 40 are nearly fully open
    // 49 may be fully open
    if game.config.overlay_floodfill {
        render_overlay_floodfill(panel, &game.level, &game.config, tiles_key);
    }
}

fn render_overlay_alertness(panel: &mut Panel,
                            display_state: &mut DisplayState,
                            sprite_key: SpriteKey,
                            entities: &Entities,
                            config: &Config) {
    let alertness_color = config.color_pink;
    let scale = 0.5;
    for entity_id in display_state.ids.iter() {
        if display_state.entity_is_in_fov(*entity_id) != FovResult::Inside {
            continue;
        }

        let pos = display_state.pos[entity_id];

        let mut status_drawn: bool = false;
        if let Some(status) = entities.status.get(entity_id) {
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
            if let Some(behavior) = display_state.behavior.get(entity_id) {
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

/*
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
*/

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
                         config: &Config,
                         display_state: &mut DisplayState,
                         entity_id: EntityId,
                         sprites: &Vec<SpriteSheet>) {
    let mut attack_highlight_color = config.color_red;
    attack_highlight_color.a = config.highlight_alpha_attack;

    let tiles_key = lookup_spritekey(sprites, "tiles");

    if let Some(attack_positions) = display_state.entity_attacks.get(&entity_id) {
        for position in attack_positions.iter() {
            let sprite = Sprite::new(MAP_EMPTY_CHAR as u32, tiles_key);
            panel.sprite_cmd(sprite, attack_highlight_color, *position);
        }
    }
}

fn render_fov_overlay(panel: &mut Panel,
                      display_state: &DisplayState,
                      level: &Level,
                      config: &Config,
                      entity_id: EntityId) {
    let player_id = display_state.player_id();

    let mut highlight_color = config.color_light_grey;
    highlight_color.a = config.grid_alpha_overlay;

    for y in 0..level.map.height() {
        for x in 0..level.map.width() {
            let map_pos = Pos::new(x, y);

            let visible = level.pos_in_fov(entity_id, map_pos, config) &&
                          level.pos_in_fov(player_id, map_pos, config);


            if visible {
                panel.outline_cmd(highlight_color, map_pos);
            }
        }
    }
}

fn render_movement_overlay(panel: &mut Panel,
                           config: &Config,
                           display_state: &mut DisplayState,
                           entity_id: EntityId,
                           sprites: &Vec<SpriteSheet>) {
    let mut highlight_color = config.color_light_grey;
    highlight_color.a = config.grid_alpha_overlay;

    let tiles_key = lookup_spritekey(sprites, "tiles");

    if let Some(move_positions) = display_state.entity_movements.get(&entity_id) {
        let chr = display_state.chr[&entity_id];
        let sprite = Sprite::new(chr as u32, tiles_key);

        for move_pos in move_positions.iter() {
            panel.sprite_cmd(sprite, highlight_color, *move_pos);
        }
    }
}

fn render_entity_ghost(panel: &mut Panel,
                       entity_id: EntityId,
                       render_pos: Pos,
                       config: &Config,
                       display_state: &mut DisplayState,
                       sprites: &Vec<SpriteSheet>) {
    let entity_pos = display_state.pos[&entity_id];

    display_state.pos[&entity_id] = render_pos;

    // a little ugly, but set the delta time to 0 so the render_entity function does not
    // step the animation forward when rendering a ghost.
    let dt = display_state.dt;
    display_state.dt = 0.0;
    let ghost_color = Color::new(255, 255, 255, config.ghost_alpha);
    render_entity(panel, entity_id, display_state, Some(ghost_color), sprites);
    display_state.dt = dt;

    display_state.pos[&entity_id] = entity_pos;
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

