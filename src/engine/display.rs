use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[allow(unused_imports)]use tcod::console::*;
#[allow(unused_imports)]use tcod::colors::*;
#[allow(unused_imports)]use tcod::input::{self, Event, Mouse};
#[allow(unused_imports)]use tcod::map::{Map as FovMap};

use crate::engine::types::*;
use crate::constants::*;
use crate::engine::map::*;


pub fn render_bar(panel: &mut Offscreen,
                  x: i32,
                  y: i32,
                  total_width: i32,
                  name: &str,
                  value: i32,
                  maximum: i32,
                  bar_color: Color,
                  back_color: Color) {
    let bar_width = (value as f32 / maximum as f32 * total_width as f32) as i32;

    panel.set_default_background(back_color);
    panel.rect(x, y, total_width, 1, false, BackgroundFlag::Screen);

    panel.set_default_background(bar_color);
    if bar_width > 0 {
        panel.rect(x, y, bar_width, 1, false, BackgroundFlag::Screen);
    }

    panel.set_default_foreground(WHITE);
    panel.print_ex(x + total_width / 2,
                   y,
                   BackgroundFlag::None,
                   TextAlignment::Center,
                   &format!("{}: {}/{}", name, value, maximum));
}

pub fn menu<T: AsRef<str>>(header: &str, options: &[T], width: i32, root: &mut Root) -> Option<usize> {
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

pub fn get_objects_under_mouse(mouse: Mouse, objects: &[Object], fov_map: &FovMap) -> Vec<ObjectId> {
    let (x, y) = (mouse.cx as i32, mouse.cy as i32);

    objects.iter()
           .enumerate()
           .filter(|(_, obj)| obj.pos() == (x, y) && fov_map.is_in_fov(obj.x, obj.y))
           .map(|(index, _)| index)
           .collect::<Vec<_>>()
}

pub fn get_names_under_mouse(mouse: Mouse, objects: &[Object], fov_map: &FovMap) -> String {
    let (x, y) = (mouse.cx as i32, mouse.cy as i32);

    let names = objects.iter()
                       .filter(|obj| { obj.pos() == (x, y) && fov_map.is_in_fov(obj.x, obj.y) })
                       .map(|obj| { format!("{}, Ai {:?}, Behavior {:?}", obj.name.clone(), obj.ai, obj.behavior) })
                       .collect::<Vec<_>>();

    names.join(", ")
}

// BLOCK3 solid block
// CHECKBOX_UNSET unfilled block
// CROSS cross
// DCROSS invert cross
// DHLINE invert horizontal line
// DTEEE,DTEEN,DTEES,DTEEW invert tees
// DNE,DNW,DSE,DSW corners
// DVLINE divider lines vertical
// TEEE,TEEN,TEES,TEEW tees
// RADIO_SET,RADIO_UNSET unfilled circle and circle with inner filled circle
// SUBP_DIAG,SUBP_E, SUBP_N,SUBP_NE,SUBP_NW,SUBP_SE,SUBP_SW half blocks, directional. maybe
// SW as well
// VLINE HLINE thin lines
// ARROW2_E,ARROW2_N,ARROW2_S,ARROW2_W solid arrow
// ARROW_E,ARROW_N,ARROW_S,ARROW_W  thin arrow
fn print_all_special_char(console: &mut Offscreen, mouse: Mouse) {
    use tcod::chars::*;
    let _ = vec!(ARROW2_E,ARROW2_N,ARROW2_S,ARROW2_W,ARROW_E,ARROW_N,ARROW_S,
                 ARROW_W,BLOCK1,BLOCK2,BLOCK3,BULLET,BULLET_INV,BULLET_SQUARE,
                 CENT,CHECKBOX_SET,CHECKBOX_UNSET,CLUB,COPYRIGHT,CROSS,CURRENCY,
                 DARROW_H,DARROW_V,DCROSS,DHLINE,DIAMOND,DIVISION,DNE,DNW,DSE,DSW,
                 DTEEE,DTEEN,DTEES,DTEEW,DVLINE,EXCLAM_DOUBLE,FEMALE,FUNCTION,
                 GRADE,HALF,HEART,HLINE,LIGHT,MALE,MULTIPLICATION,NE,NOTE,
                 NOTE_DOUBLE,NW,ONE_QUARTER,PILCROW,POUND,POW1,POW2,
                 POW3,RADIO_SET,RADIO_UNSET,RESERVED,SE,SECTION,
                 SMILIE,SMILIE_INV,SPADE,SUBP_DIAG,SUBP_E, SUBP_N,SUBP_NE,SUBP_NW,SUBP_SE,SUBP_SW
                 ,SW,TEEE,TEEN,TEES,TEEW,THREE_QUARTERS,UMLAUT,VLINE,YEN);
    let mut index = 0;

    for x in 0..MAP_WIDTH {
        for y in 0..MAP_HEIGHT {
            console.set_char_background(x, y, BLACK, BackgroundFlag::Set);
            console.put_char(x, y, ' ', BackgroundFlag::None);
        }
    }

    console.set_default_foreground(WHITE);
    for key in 0..256 {
        let index_x = 10 + (index % 32);
        let index_y = -10 + ((index / 32) * 2);

        let x = SCREEN_WIDTH/2 + index_x - 32 as i32;
        let y = SCREEN_HEIGHT/2 + index_y;

        console.put_char(x,
                              y,
                              key as u8 as char,
                              BackgroundFlag::None);
        if mouse.cx as i32 == x && mouse.cy as i32 == y {
            console.print_ex(x,
                                  y - 1,
                                  BackgroundFlag::None,
                                  TextAlignment::Left,
                                  format!("{}", key));
        }

        index += 1;
    }
}

pub fn rand_from_x_y(x: i32, y: i32) -> f32 {
    let mut hasher = DefaultHasher::new();

    (x as u32).hash(&mut hasher);
    (y as u32).hash(&mut hasher);
 
    let result: u64 = hasher.finish();

    return ((result & 0xFFFFFFFF) as f32) / 4294967295.0;
}

pub fn draw_movement_overlay(console: &mut Console,
                             map: &Map,
                             id: ObjectId,
                             objects: &[Object]) -> Vec<(i32, i32)> {
    let mut added_positions = Vec::new();

    if let Some(movement) = objects[id].movement {
        let offsets = movement.offsets();
        for offset in offsets {
            let x = objects[id].x as i32 + offset.0;
            let y = objects[id].y as i32 + offset.1;

            if map.clear_path((objects[id].x as i32, objects[id].y as i32), 
                              (x, y),
                              &objects) {
                console.put_char(x,
                                      y,
                                      '.',
                                      BackgroundFlag::None);

                added_positions.push((x, y));
            }
        }
    }

    return added_positions;
}

pub fn draw_attack_overlay(console: &mut dyn Console,
                           map: &Map,
                           id: ObjectId,
                           objects: &[Object]) -> Vec<(i32, i32)> {
    let mut added_positions = Vec::new();

    if let Some(attack) = objects[id].attack {
        let offsets = attack.offsets();
        for offset in offsets {
            let x = objects[id].x as i32 + offset.0;
            let y = objects[id].y as i32 + offset.1;

            if map.clear_path((objects[id].x as i32, objects[id].y as i32), 
                              (x, y),
                              &objects) {
                console.put_char(x,
                                      y,
                                      'x',
                                      BackgroundFlag::None);

                added_positions.push((x, y));
            }
        }
    }

    return added_positions;
}

pub fn render_map(console: &mut dyn Console,
                  fov: &FovMap,
                  map: &mut Map,
                  config: &Config) {
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let visible = fov.is_in_fov(x, y);

            // Color based on TileType and visibility
            let color = match (map.tiles[x as usize][y as usize].tile_type, visible) {
                (TileType::Wall, true) => config.color_light_brown.color(),
                (TileType::Wall, false) => config.color_dark_brown.color(),

                (TileType::Empty, true) => lerp(config.color_tile_blue_light.color(), config.color_tile_blue_dark.color(), rand_from_x_y(x, y)),
                (TileType::Empty, false) => config.color_very_dark_blue.color(),

                (TileType::Water, true) => config.color_blueish_grey.color(),
                (TileType::Water, false) => config.color_dark_brown.color(),

                (TileType::ShortWall, true) => config.color_light_brown.color(),
                (TileType::ShortWall, false) => config.color_dark_brown.color(),

                (TileType::Exit, true) => config.color_orange.color(),
                (TileType::Exit, false) => config.color_red.color(),
            };

            let mut explored = map.tiles[x as usize][y as usize].explored;
            if visible {
                explored = true;
            }

            if explored {
                let tile_type = map.tiles[x as usize][y as usize].tile_type;
                match tile_type {
                    TileType::Empty => {
                        let has_bottom_wall = map.tiles[x as usize][y as usize].bottom_wall != Wall::Empty;
                        let has_left_wall = map.tiles[x as usize][y as usize].left_wall != Wall::Empty;

                        let chr;
                        if  has_bottom_wall && has_left_wall {
                            // TODO this is a solid wall- there is no joint left/bottom wall tile
                            // yet
                            chr = '\u{DB}';
                        } else if has_left_wall {
                            chr = '\u{DD}';
                        } else if has_bottom_wall {
                            chr = '\u{DC}';
                        } else {
                            chr = ' ';
                        }

                        console.put_char(x, y, chr, BackgroundFlag::None);
                        console.set_char_background(x, y, color, BackgroundFlag::Set);
                    }

                    TileType::Water | TileType::Exit => {
                        console.put_char(x, y, ' ', BackgroundFlag::None);
                        console.set_char_background(x, y, color, BackgroundFlag::Set);
                    }

                    TileType::ShortWall | TileType::Wall => {
                        if visible {
                            console.set_char_background(x, y, config.color_tile_blue_light.color(), BackgroundFlag::Set);
                        } else {
                            console.set_char_background(x, y, config.color_very_dark_blue.color(), BackgroundFlag::Set);
                        }

                        let left = map[(x - 1, y)].tile_type == tile_type;
                        let right = map[(x + 1, y)].tile_type == tile_type;
                        let horiz = left || right;

                        let above = map[(x, y + 1)].tile_type == tile_type;
                        let below = map[(x, y - 1)].tile_type == tile_type;
                        let vert = above || below;

                        let chr;
                        if tile_type == TileType::Wall {
                            if horiz && vert {
                               chr = '\u{DC}';
                               console.set_char_background(x, y, color, BackgroundFlag::Set);
                            } else if horiz {
                               chr = '\u{EC}';
                            } else if vert {
                               chr = '\u{ED}';
                            } else {
                               chr = '\u{FE}';
                            }
                        } else {
                            if horiz && vert {
                               chr = tcod::chars::CROSS
                            } else if horiz {
                               chr = tcod::chars::HLINE;
                            } else if vert {
                               chr = tcod::chars::VLINE;
                            } else {
                               chr = tcod::chars::VLINE;
                            }
                        };

                        console.set_default_foreground(color);
                        console.put_char(x, y, chr, BackgroundFlag::None);
                    }
                }
            }

            map.tiles[x as usize][y as usize].explored = explored;
        }
    }
}

pub fn render_sound(console: &mut dyn Console,
                    animations: &Vec<Animation>,
                    map: &Map,
                    objects: &[Object]) {
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            // after animations play, draw sound for a frame
            if animations.len() == 0 {
               if let Some(sound_loc) = map[(x, y)].sound {
                   if map.clear_path_obstacles(sound_loc, (x, y), objects) {
                      console.put_char(x, y, '.', BackgroundFlag::None);
                   }
               }
            }
        }
    }
}

pub fn render_objects(console: &mut dyn Console, fov: &FovMap, objects: &[Object]) {
    let mut to_draw: Vec<_> =
        objects.iter().filter(|o| {
            fov.is_in_fov(o.x, o.y)
        }).collect();
    to_draw.sort_by(|o1, o2| { o1.blocks.cmp(&o2.blocks) });

    for object in &to_draw {
        object.draw(console);
    }
}

pub fn render_character_flags(console: &mut dyn Console) {
    for x in 0..10 {
        console.put_char(x, 0, '+', BackgroundFlag::None);
        console.put_char(x, 0, 'X', BackgroundFlag::None);

        console.put_char(x, 1, '+', BackgroundFlag::None);
        console.put_char(x, 1, 'X', BackgroundFlag::Set);

        console.put_char(x, 2, '+', BackgroundFlag::None);
        console.put_char(x, 2, 'X', BackgroundFlag::Lighten);

        console.put_char(x, 3, '+', BackgroundFlag::None);
        console.put_char(x, 3, 'X', BackgroundFlag::Darken);

        console.put_char(x, 4, '+', BackgroundFlag::None);
        console.put_char(x, 4, 'X', BackgroundFlag::Screen);

        console.put_char(x, 5, '+', BackgroundFlag::None);
        console.put_char(x, 5, 'X', BackgroundFlag::ColorDodge);

        console.put_char(x, 6, '+', BackgroundFlag::None);
        console.put_char(x, 6, 'X', BackgroundFlag::ColorBurn);

        console.put_char(x, 7, '+', BackgroundFlag::None);
        console.put_char(x, 7, 'X', BackgroundFlag::Add);

        console.put_char(x, 8, '+', BackgroundFlag::None);
        console.put_char(x, 8, 'X', BackgroundFlag::AddA);

        console.put_char(x, 9, '+', BackgroundFlag::None);
        console.put_char(x, 9, 'X', BackgroundFlag::Burn);

        console.put_char(x, 10, '+', BackgroundFlag::None);
        console.put_char(x, 10, 'X', BackgroundFlag::Overlay);

        console.put_char(x, 11, '+', BackgroundFlag::None);
        console.put_char(x, 11, 'X', BackgroundFlag::Alph);

        console.put_char(x, 12, '+', BackgroundFlag::None);
        console.put_char(x, 12, 'X', BackgroundFlag::Default);

        console.put_char(x, 13, '+', BackgroundFlag::None);
        console.put_char(x, 13, 'X', BackgroundFlag::Multiply);
    }
}

pub fn render_overlays(game: &mut Game, map: &Map, objects: &[Object]) {
    let ids = get_objects_under_mouse(game.mouse, objects, &game.fov);
    for id in ids {
        if !objects[id].alive {
            continue;
        }

        // TODO removed for ggez
        //  game.needs_clear.extend(draw_movement_overlay(&mut game.console, map, id, objects));
        //  game.needs_clear.extend(draw_attack_overlay(&mut game.console, map, id, objects));
    }

    for id in 0..objects.len() {
        let (x, y) = (objects[id].x, objects[id].y);
        if game.display_overlays && game.fov.is_in_fov(x, y) && objects[id].alive {
            // TODO removed for ggez
            // game.needs_clear.extend(draw_movement_overlay(&mut game.console, map, id, objects));
            // game.needs_clear.extend(draw_attack_overlay(&mut game.console, map, id, objects));
        }
    }
}

pub fn render_all(game: &mut Game,
                  objects: &[Object],
                  map: &mut Map,
                  messages: &mut Messages,
                  fov_recompute: bool,
                  config: &Config) {
    if fov_recompute {
        let player = &objects[PLAYER];
        let mut fov_distance = config.fov_distance;
        if game.god_mode {
            fov_distance = std::cmp::max(SCREEN_WIDTH, SCREEN_HEIGHT);
        }
        game.fov.compute_fov(player.x, player.y, fov_distance, FOV_LIGHT_WALLS, FOV_ALGO);
    }

    // TODO removed for ggez
    // render_map(&mut game.console, &game.fov, map, config);

    // TODO removed for ggez
    // render_sound(&mut game.console, &game.animations, map, objects);

    /* Draw objects */
    // TODO removed for ggez
    // render_objects(&mut game.console, &game.fov, objects);

    // Draw movement and attack overlays
    // TODO removed for ggez
    // render_overlays(game, map, objects);

    // display for checking out character flags
    //render_character_flags(&mut game.console);

    // TODO removed for ggez
    // game.panel.set_default_background(BLACK);
    // game.panel.clear();

    // Draw UI overlay
    let hp = objects[PLAYER].fighter.map_or(0, |f| f.hp);
    let max_hp = objects[PLAYER].fighter.map_or(0, |f| f.max_hp);
    // TODO removed for ggez
    // render_bar(&mut game.panel, 1, 1, BAR_WIDTH, "HP", hp, max_hp, LIGHT_RED, DARK_RED);

    let mut y = MSG_HEIGHT as i32;
    for &(ref msg, color) in messages.0.iter().rev() {
        // TODO removed for ggez
        // let msg_height = game.panel.get_height_rect(MSG_X, y, MSG_WIDTH, 0, msg);
        // y -= msg_height;
        // if y < 0 {
        //     break;
        // }
        // game.panel.set_default_foreground(color);
        // game.panel.print_rect(MSG_X, y, MSG_WIDTH, 0, msg);
    }

    // TODO removed for ggez
    //  game.panel.set_default_foreground(LIGHT_GREY);
    //  game.panel.print_ex(1, 2, BackgroundFlag::None, TextAlignment::Left, format!("Turn Count: {}", game.turn_count));
    //  game.panel.print_ex(1, 3, BackgroundFlag::None, TextAlignment::Left, format!("{:?}", (objects[PLAYER].momentum.unwrap().mx, objects[PLAYER].momentum.unwrap().my)));
    //  game.panel.print_ex(1, 3, BackgroundFlag::None, TextAlignment::Left, get_names_under_mouse(game.mouse, objects, &game.fov));

    //  game.console.set_default_background(LIGHT_GREY);
    //  game.console.set_default_foreground(RED);

    /* print all special characters */
    //print_all_special_char(&mut game.console, game.mouse);

    // replace screen with new console contents
    // TODO removed for ggez
    // blit(&mut game.console, (0, 0), (SCREEN_WIDTH, SCREEN_HEIGHT), &mut game.root, (0, 0),       1.0, 1.0);
    // blit(&mut game.panel,   (0, 0), (SCREEN_WIDTH, SCREEN_HEIGHT), &mut game.root, (0, PANEL_Y), 1.0, 1.0);
}

