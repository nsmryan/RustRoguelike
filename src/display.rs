#[allow(unused_imports)]use tcod::console::*;
#[allow(unused_imports)]use tcod::colors::*;
#[allow(unused_imports)]use tcod::input::{self, Event, Mouse};
#[allow(unused_imports)]use tcod::map::{Map as FovMap};

use crate::types::*;
use crate::constants::*;
use crate::map::*;


pub fn render_bar(game: &mut Game,
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
           .filter(|(index, obj)| { obj.pos() == (x, y) && fov_map.is_in_fov(obj.x, obj.y) })
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
fn print_all_special_char(game: &mut Game) {
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
            game.console.set_char_background(x, y, BLACK, BackgroundFlag::Set);
            game.console.put_char(x, y, ' ', BackgroundFlag::None);
        }
    }

    game.console.set_default_foreground(WHITE);
    for key in 0..256 {
        let index_x = 10 + (index % 32);
        let index_y = -10 + ((index / 32) * 2);

        let x = SCREEN_WIDTH/2 + index_x - 32 as i32;
        let y = SCREEN_HEIGHT/2 + index_y;

        game.console.put_char(x,
                              y,
                              key as u8 as char,
                              BackgroundFlag::None);
        if game.mouse.cx as i32 == x && game.mouse.cy as i32 == y {
            game.console.print_ex(x,
                                  y - 1,
                                  BackgroundFlag::None,
                                  TextAlignment::Left,
                                  format!("{}", key));
        }

        index += 1;
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
        game.fov.compute_fov(player.x, player.y, TORCH_RADIOUS, FOV_LIGHT_WALLS, FOV_ALGO);
    }

    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let visible = game.fov.is_in_fov(x, y);

            // Color based on TileType and visibility
            let color = match (map.0[x as usize][y as usize].tile_type, visible) {
                (TileType::Wall, true) => config.color_light_wall,
                (TileType::Wall, false) => config.color_dark_wall,

                (TileType::Empty, true) => config.color_light_ground,
                (TileType::Empty, false) => config.color_dark_ground,

                (TileType::Water, true) => config.color_light_water,
                (TileType::Water, false) => config.color_dark_water,

                (TileType::ShortWall, true) => config.color_light_wall,
                (TileType::ShortWall, false) => config.color_dark_wall,

                (TileType::Exit, true) => config.color_light_exit,
                (TileType::Exit, false) => config.color_dark_exit,
            };

            let mut explored = map.0[x as usize][y as usize].explored;
            if visible {
                explored = true;
            }

            if explored {
                let tile_type = map.0[x as usize][y as usize].tile_type;
                match tile_type {
                    TileType::Empty | TileType::Water | TileType::Exit => {
                        game.console.set_char_background(x, y, color.color(), BackgroundFlag::Set);
                    }

                    TileType::ShortWall | TileType::Wall => {
                        if visible {
                            game.console.set_char_background(x, y, config.color_light_ground.color(), BackgroundFlag::Set);
                        } else {
                            game.console.set_char_background(x, y, config.color_dark_ground.color(), BackgroundFlag::Set);
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
                               game.console.set_char_background(x, y, color.color(), BackgroundFlag::Set);
                            } else if horiz {
                               chr = '\u{DF}';
                            } else if vert {
                               chr = '\u{DD}';
                            } else {
                               chr = '\u{DC}';
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

                        game.console.set_default_foreground(color.color());
                        game.console.put_char(x, y, chr, BackgroundFlag::None);
                    }
                }
            }
            map.0[x as usize][y as usize].explored = explored;
        }
    }

    /* Draw objects */
    let mut to_draw: Vec<_> =
        objects.iter().filter(|o| {
            game.fov.is_in_fov(o.x, o.y)
        }).collect();
    to_draw.sort_by(|o1, o2| { o1.blocks.cmp(&o2.blocks) });

    for object in &to_draw {
        object.draw(&mut game.console);
    }

    let ids = get_objects_under_mouse(game.mouse, objects, &game.fov);
    for id in ids {
        if let Some(movement) = objects[id].movement {
            let offsets = movement.offsets();
            for offset in offsets {
                let x = game.mouse.cx as i32 + offset.0;
                let y = game.mouse.cy as i32 + offset.1;
                game.console.put_char(x,
                                      y,
                                      '.',
                                      BackgroundFlag::None);

                game.needs_clear.push((x, y));
            }
        }
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
    game.panel.print_ex(1, 2, BackgroundFlag::None, TextAlignment::Left, format!("Turn Count: {}", game.turn_count));
    game.panel.print_ex(1, 3, BackgroundFlag::None, TextAlignment::Left, format!("{:?}", (objects[PLAYER].momentum.unwrap().0, objects[PLAYER].momentum.unwrap().1)));
    game.panel.print_ex(1, 3, BackgroundFlag::None, TextAlignment::Left, get_names_under_mouse(game.mouse, objects, &game.fov));

    game.console.set_default_background(LIGHT_GREY);
    game.console.set_default_foreground(RED);

    /* print all special characters */
    //print_all_special_char(game);

    blit(&mut game.console, (0, 0), (SCREEN_WIDTH, SCREEN_HEIGHT), &mut game.root, (0, 0),       1.0, 1.0);
    blit(&mut game.panel,   (0, 0), (SCREEN_WIDTH, SCREEN_HEIGHT), &mut game.root, (0, PANEL_Y), 1.0, 1.0);
}

