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

pub fn get_names_under_mouse(mouse: Mouse, objects: &[Object], fov_map: &FovMap) -> String {
    let (x, y) = (mouse.cx as i32, mouse.cy as i32);

    let names = objects.iter()
                       .filter(|obj| { obj.pos() == (x, y) && fov_map.is_in_fov(obj.x, obj.y)})
                       .map(|obj| { format!("{}, Ai {:?}, Behavior {:?}", obj.name.clone(), obj.ai, obj.behavior) })
                       .collect::<Vec<_>>();

    names.join(", ")
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
            };

            let mut explored = map.0[x as usize][y as usize].explored;
            if visible {
                explored = true;
            }

            if explored {
                let tile_type = map.0[x as usize][y as usize].tile_type;
                match tile_type {
                    TileType::Empty | TileType::Water => {
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
                               chr = tcod::chars::SUBP_N;
                               game.console.set_char_background(x, y, color.color(), BackgroundFlag::Set);
                            } else if horiz {
                               chr = tcod::chars::SUBP_N;
                            } else if vert {
                               chr = tcod::chars::SUBP_E;
                            } else {
                               chr = tcod::chars::SUBP_E;
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
    let mut to_draw: Vec<_> = objects.iter().filter(|o| game.fov.is_in_fov(o.x, o.y)).collect();
    to_draw.sort_by(|o1, o2| { o1.blocks.cmp(&o2.blocks) });

    for object in &to_draw {
        object.draw(&mut game.console);
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

    blit(&mut game.console, (0, 0), (SCREEN_WIDTH, SCREEN_HEIGHT), &mut game.root, (0, 0), 1.0, 1.0);

    blit(&mut game.panel, (0, 0), (SCREEN_WIDTH, SCREEN_HEIGHT), &mut game.root, (0, PANEL_Y), 1.0, 1.0);

}

