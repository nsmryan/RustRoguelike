use std::str::FromStr;
use std::io::BufRead;

use sdl2::render::{Texture, TextureCreator, RenderTarget, WindowCanvas, BlendMode, Canvas};
use sdl2::rect::Rect;
use sdl2::video::WindowContext;
use sdl2::pixels::{PixelFormatEnum, Color as Sdl2Color};
use sdl2::ttf::Sdl2TtfContext;

use parse_display::{Display, FromStr};

use roguelike_utils::math::*;

use roguelike_map::{PlayerDirection};

use crate::animation::{Sprite, SpriteKey};
use crate::panel::{Panel};
use crate::spritesheet::{SpriteSheet, lookup_spritekey, parse_atlas_file};


pub const ASCII_START: u32 = 32;
pub const ASCII_END: u32 = 127;


#[derive(Clone, Copy, Debug, PartialEq, Display, FromStr)]
#[display(style = "snake_case")]
pub enum Justify {
    Right,
    Center,
    Left,
}

// NOTE use of String prevents Copy trait
#[derive(Clone, Display, FromStr, Debug, PartialEq)]
#[display(style = "snake_case")]
pub enum DrawCmd {
    #[display("sprite {0} {1} {2}")]
    Sprite(Sprite, Color, Pos),
    #[display("sprite_scaled {0} {1} {2} {3} {4}")]
    SpriteScaled(Sprite, f32, PlayerDirection, Color, Pos),
    #[display("sprite_float {0} {1} {2} {3} {4} {5}")]
    SpriteFloat(Sprite, Color, f32, f32, f32, f32), // sprite, color, x, y, x scale, y scale
    #[display("highlight_tile {0} {1}")]
    HighlightTile(Color, Pos),
    #[display("outline_tile {0} {1}")]
    OutlineTile(Color, Pos),
    #[display("text {0} {1} {2} {3}")]
    Text(String, Color, Pos, f32), // text, color, tile position, scale
    #[display("text_float {0} {1} {2} {3} {4}")]
    TextFloat(String, Color, f32, f32, f32), // text, color, x, y, scale
    #[display("text_justify {0} {1} {2} {3} {4} {5} {6}")]
    TextJustify(String, Justify, Color, Color, Pos, u32, f32), // text, justify, fg color, bg color, tile pos, width in cells, scale
    #[display("rect {0} {1} {2} {3} {4} {5}")]
    Rect(Pos, u32, u32, f32, bool, Color), // start cell, num cells width/height, offset percent into cell, color
    #[display("rect_float {0} {1} {2} {3} {4} {5}")]
    RectFloat(f32, f32, f32, f32, bool, Color), // x, y, width/height, color
    #[display("fill {0} {1}")]
    Fill(Pos, Color),
}

impl DrawCmd {
    pub fn aligned(&self) -> bool {
        let float_sprite = matches!(self, DrawCmd::SpriteFloat(_, _, _, _, _, _));
        let float_text = matches!(self, DrawCmd::TextFloat(_, _, _, _, _));
        return !(float_sprite || float_text);
    }

    pub fn pos(&self) -> Pos {
        match self {
            DrawCmd::Sprite(_, _, pos) => *pos,
            DrawCmd::SpriteScaled(_, _, _, _, pos) => *pos,
            DrawCmd::SpriteFloat(_, _, x, y, _, _) => Pos::new(*x as i32, *y as i32),
            DrawCmd::HighlightTile(_, pos) => *pos,
            DrawCmd::OutlineTile(_, pos) => *pos,
            DrawCmd::Text(_, _, pos, _) => *pos,
            DrawCmd::TextFloat(_, _, x, y, _) => Pos::new(*x as i32, *y as i32),
            DrawCmd::TextJustify(_, _, _, _, pos, _, _) => *pos,
            DrawCmd::Rect(pos, _, _, _, _, _) => *pos,
            DrawCmd::RectFloat(x, y, _, _, _, _) => Pos::new(*x as i32, *y as i32),
            DrawCmd::Fill(pos, _) => *pos,
        }
    }
}

pub fn process_draw_cmd(panel: &Panel,
                        canvas: &mut WindowCanvas,
                        sprite_texture: &mut Texture,
                        sprites: &mut Vec<SpriteSheet>,
                        font_texture: &mut Texture,
                        cmd: &DrawCmd) {
    match cmd {
        DrawCmd::Sprite(sprite, color, pos) => {
            let sprite_sheet = &mut sprites[sprite.key];
            let (cell_width, cell_height) = panel.cell_dims();

            let pos = Pos::new(pos.x * cell_width as i32, pos.y * cell_height as i32);

            let dst_rect = Rect::new(pos.x as i32,
                                     pos.y as i32,
                                     cell_width as u32,
                                     cell_height as u32);

            canvas.set_blend_mode(BlendMode::Blend);


            let src_rect = sprite_sheet.sprite_src(sprite.index);
            sprite_texture.set_color_mod(color.r, color.g, color.b);
            sprite_texture.set_alpha_mod(color.a);

            canvas.copy_ex(sprite_texture,
                           src_rect,
                           Some(dst_rect),
                           sprite.rotation,
                           None,
                           sprite.flip_horiz,
                           sprite.flip_vert).unwrap();
        }

        DrawCmd::SpriteScaled(sprite, scale, direction, color, pos) => {
            let cell_dims = panel.cell_dims();
            let sprite_sheet = &mut sprites[sprite.key];

            let src_rect = sprite_sheet.sprite_src(sprite.index);

            let (cell_width, cell_height) = cell_dims;
            let dst_width = (cell_width as f32 * scale) as u32;
            let dst_height = (cell_height as f32 * scale) as u32;

            let x_margin = ((cell_width - dst_width) / 2) as i32;
            let y_margin = ((cell_height - dst_height) / 2) as i32;

            let mut dst_x = pos.x * cell_width as i32;
            let mut dst_y = pos.y * cell_height as i32;
            match direction {
                PlayerDirection::Center => {
                    dst_x += x_margin;
                    dst_y += y_margin;
                }
                
                PlayerDirection::Left => {
                    dst_y += y_margin;
                }

                PlayerDirection::Right => {
                    dst_x += cell_width as i32 - dst_width as i32;
                    dst_y += y_margin;
                }

                PlayerDirection::Up => {
                    dst_x += x_margin;
                }

                PlayerDirection::Down => {
                    dst_x += x_margin;
                    dst_y += cell_height as i32 - dst_height as i32;
                }

                PlayerDirection::DownLeft => {
                    dst_y += cell_height as i32 - dst_height as i32;
                }

                PlayerDirection::DownRight => {
                    dst_x += cell_width as i32 - dst_width as i32;
                    dst_y += cell_height as i32 - dst_height as i32;
                }

                PlayerDirection::UpLeft => {
                    // Already in the upper left corner by default.
                }

                PlayerDirection::UpRight => {
                    dst_x += cell_width as i32  - dst_width as i32;
                }
            }

            let dst = Rect::new(dst_x,
                                dst_y,
                                dst_width,
                                dst_height);

            canvas.set_blend_mode(BlendMode::Blend);
            sprite_texture.set_color_mod(color.r, color.g, color.b);
            sprite_texture.set_alpha_mod(color.a);

            canvas.copy_ex(sprite_texture,
                           src_rect,
                           Some(dst),
                           sprite.rotation,
                           None,
                           false,
                           false).unwrap();
        }

        DrawCmd::SpriteFloat(sprite, color, x, y, x_scale, y_scale) => {
            let sprite_sheet = &mut sprites[sprite.key];

            let cell_dims = panel.cell_dims();

            let src_rect = sprite_sheet.sprite_src(sprite.index);

            let (cell_width, cell_height) = cell_dims;

            let x_offset = (*x * cell_width as f32) as i32;
            let y_offset = (*y * cell_height as f32) as i32;

            let dst = Rect::new(x_offset,
                                y_offset,
                                (cell_width as f32 * x_scale) as u32,
                                (cell_height as f32 * y_scale) as u32);

            canvas.set_blend_mode(BlendMode::Blend);

            sprite_texture.set_color_mod(color.r, color.g, color.b);
            sprite_texture.set_alpha_mod(color.a);

            canvas.copy_ex(sprite_texture,
                           src_rect,
                           Some(dst),
                           sprite.rotation,
                           None,
                           sprite.flip_horiz,
                           sprite.flip_vert).unwrap();
        }

        DrawCmd::OutlineTile(color, pos) => {
            draw_outline_tile(panel, canvas, *pos, *color);
        }

        DrawCmd::HighlightTile(color, pos) => {
            draw_tile_highlight(panel, canvas, *pos, *color);
        }

        DrawCmd::TextJustify(string, justify, fg_color, bg_color, start_pos, width, scale) => {
            let ascii_width = ASCII_END - ASCII_START;

            let query = font_texture.query();

            let cell_dims = panel.cell_dims();
            let (cell_width, cell_height) = cell_dims;

            let font_width = query.width / ascii_width;
            let font_height = query.height;

            let char_height = (cell_height as f32 * scale) as u32;
            let char_width = (cell_height * font_width) / font_height;
            let char_width = (char_width as f32 * scale) as u32;

            let pixel_width = (*width * cell_width) as i32;
            let mut x_offset;
            match justify {
                Justify::Right => {
                    x_offset = (start_pos.x * cell_width as i32) + pixel_width - char_width as i32 * string.len() as i32;
                }

                Justify::Center => {
                    x_offset = (((start_pos.x * cell_width as i32) + pixel_width) / 2) - ((char_width as i32 * string.len() as i32) / 2);
                }

                Justify::Left => {
                    x_offset = start_pos.x * cell_width as i32;
                }
            }

            let y_offset = start_pos.y * cell_height as i32;

            canvas.set_blend_mode(BlendMode::Blend);
            canvas.set_draw_color(sdl2::pixels::Color::BLACK);
            canvas.fill_rect(Rect::new(x_offset, y_offset, string.len() as u32 * char_width, char_height)).unwrap();

            canvas.set_blend_mode(BlendMode::Blend);
            canvas.set_draw_color(sdl2_color(*bg_color));
            canvas.fill_rect(Rect::new(x_offset, y_offset, string.len() as u32 * char_width, char_height)).unwrap();

            font_texture.set_color_mod(fg_color.r, fg_color.g, fg_color.b);
            font_texture.set_alpha_mod(fg_color.a);

            for chr in string.chars() {
                let chr_num = chr.to_lowercase().next().unwrap();
                let chr_index = chr_num as i32 - ASCII_START as i32;

                let src = Rect::new((query.width as i32 / ascii_width as i32) * chr_index,
                                    0,
                                    font_width,
                                    font_height);

                let dst_pos = Pos::new(x_offset, y_offset);
                x_offset += char_width as i32;
                let dst = Rect::new(dst_pos.x as i32,
                                    dst_pos.y as i32,
                                    char_width as u32,
                                    char_height as u32);

                canvas.copy_ex(font_texture,
                               Some(src),
                               Some(dst),
                               0.0,
                               None,
                               false,
                               false).unwrap();
            }
        }

        DrawCmd::TextFloat(string, color, x, y, scale) => {
            let ascii_width = ASCII_END - ASCII_START;

            let query = font_texture.query();

            let cell_dims = panel.cell_dims();
            let (cell_width, cell_height) = cell_dims;

            let font_width = query.width / ascii_width;
            let font_height = query.height;

            let char_height = (cell_height as f32 * scale) as u32;
            let char_width = (cell_height * font_width) / font_height;
            let char_width = (char_width as f32 * scale) as u32;

            canvas.set_blend_mode(BlendMode::Blend);
            font_texture.set_color_mod(color.r, color.g, color.b);
            font_texture.set_alpha_mod(color.a);

            let text_pixel_width = string.len() as i32 * char_width as i32;

            let mut x_offset = (*x * cell_width as f32) as i32 - (text_pixel_width / 2);
            let y_offset = (*y * cell_height as f32) as i32;
            for chr in string.chars() {
                let chr_num = chr.to_lowercase().next().unwrap();
                let chr_index = chr_num as i32 - ASCII_START as i32;

                let src = Rect::new(font_width as i32 * chr_index,
                                    0,
                                    font_width,
                                    font_height);

                let dst_pos = Pos::new(x_offset, y_offset);
                let dst = Rect::new(dst_pos.x as i32,
                                    dst_pos.y as i32,
                                    char_width as u32,
                                    char_height as u32);

                canvas.copy_ex(font_texture,
                               Some(src),
                               Some(dst),
                               0.0,
                               None,
                               false,
                               false).unwrap();
                x_offset += char_width as i32;
            }
        }

        DrawCmd::Text(string, color, start_pos, scale) => {
            let ascii_width = ASCII_END - ASCII_START;

            let query = font_texture.query();

            let cell_dims = panel.cell_dims();
            let (cell_width, cell_height) = cell_dims;

            let font_width = query.width / ascii_width;
            let font_height = query.height;

            let char_height = (cell_height as f32 * scale) as u32;
            let char_width = (cell_height * font_width) / font_height;
            let char_width = (char_width as f32 * scale) as u32;

            canvas.set_blend_mode(BlendMode::Blend);
            font_texture.set_color_mod(color.r, color.g, color.b);
            font_texture.set_alpha_mod(color.a);

            let y_offset = start_pos.y * cell_height as i32;
            let mut x_offset = start_pos.x * cell_width as i32;
            for chr in string.chars() {
                let chr_num = chr.to_lowercase().next().unwrap();
                let chr_index = chr_num as i32 - ASCII_START as i32;

                let src = Rect::new(font_width as i32 * chr_index,
                                    0,
                                    font_width,
                                    font_height);

                let dst_pos = Pos::new(x_offset, y_offset);
                let dst = Rect::new(dst_pos.x as i32,
                                    dst_pos.y as i32,
                                    char_width as u32,
                                    char_height as u32);

                canvas.copy_ex(font_texture,
                               Some(src),
                               Some(dst),
                               0.0,
                               None,
                               false,
                               false).unwrap();
                x_offset += char_width as i32;
            }
        }

        DrawCmd::Rect(pos, width, height, offset, filled, color) => {
            assert!(*offset < 1.0, "offset >= 1 misaligns the starting cell!");

            let (cell_width, cell_height) = panel.cell_dims();

            canvas.set_draw_color(sdl2_color(*color));

            let offset_x = (cell_width as f32 * offset) as i32;
            let x: i32 = cell_width as i32 * pos.x + offset_x as i32;

            let offset_y = (cell_height as f32 * offset) as i32;
            let y: i32 = cell_height as i32 * pos.y + offset_y as i32;

            let width = cell_width * width - (2 * offset_x as u32);
            let height = cell_height * height - (2 * offset_y as u32);

            let size = (panel.num_pixels.0 / panel.cells.0) / 10;
            if *filled {
                canvas.fill_rect(Rect::new(x, y, width, height)).unwrap();
            } else {
                canvas.fill_rect(Rect::new(x, y, size, height)).unwrap();
                canvas.fill_rect(Rect::new(x, y, width, size)).unwrap();
                canvas.fill_rect(Rect::new(x + width as i32, y, size, height + size)).unwrap();
                canvas.fill_rect(Rect::new(x, y + height as i32, width + size, size)).unwrap();
            }
        }

        DrawCmd::RectFloat(x, y, width, height, filled, color) => {
            let (cell_width, cell_height) = panel.cell_dims();

            canvas.set_draw_color(sdl2_color(*color));

            let x_offset = (*x * cell_width as f32) as i32;
            let y_offset = (*y * cell_height as f32) as i32;

            let width = (width * cell_width as f32) as u32;
            let height = (height * cell_height as f32) as u32;

            let size = (panel.num_pixels.0 / panel.cells.0) / 5;
            if *filled {
                canvas.fill_rect(Rect::new(x_offset, y_offset, width, height)).unwrap();
            } else {
                // left down
                canvas.fill_rect(Rect::new(x_offset, y_offset, size, height)).unwrap();
                // top
                canvas.fill_rect(Rect::new(x_offset, y_offset, width + size, size)).unwrap();
                // right down
                canvas.fill_rect(Rect::new(x_offset + width as i32, y_offset, size, height)).unwrap();
                // bottom
                canvas.fill_rect(Rect::new(x_offset, y_offset + height as i32 - size as i32, width + size, size)).unwrap();
            }
        }

        DrawCmd::Fill(pos, color) => {
            let (cell_width, cell_height) = panel.cell_dims();
            canvas.set_draw_color(sdl2_color(*color));
            canvas.fill_rect(Rect::new(pos.x * cell_width as i32, pos.y * cell_height as i32, cell_width, cell_height)).unwrap();
        }
    }
}

fn draw_outline_tile<T>(panel: &Panel,
                        canvas: &mut Canvas<T>,
                        cell: Pos,
                        color: Color) where T: RenderTarget {
    let cell_dims = panel.cell_dims();

    canvas.set_blend_mode(BlendMode::Add);
    canvas.set_draw_color(Sdl2Color::RGBA(color.r, color.g, color.b, color.a));

    let rect = Rect::new(cell.x * cell_dims.0 as i32 + 1,
                         cell.y * cell_dims.1 as i32 + 1,
                         cell_dims.0,
                         cell_dims.1);

    canvas.draw_rect(rect).unwrap();
}

fn draw_tile_highlight<T>(panel: &Panel,
                          canvas: &mut Canvas<T>,
                          cell: Pos,
                          color: Color) where T: RenderTarget {
    let cell_dims = panel.cell_dims();

    canvas.set_blend_mode(BlendMode::Blend);
    canvas.set_draw_color(Sdl2Color::RGBA(color.r, color.g, color.b, color.a));

    let rect = Rect::new(cell.x * cell_dims.0 as i32,
                         cell.y * cell_dims.1 as i32,
                         cell_dims.0,
                         cell_dims.1);

    canvas.fill_rect(rect).unwrap();
}

fn sdl2_color(color: Color) -> Sdl2Color {
    return Sdl2Color::RGBA(color.r, color.g, color.b, color.a);
}

pub fn load_font(font_name: &str, font_size: u16, texture_creator: &mut TextureCreator<WindowContext>, ttf_context: &mut Sdl2TtfContext) -> Texture {
    let mut font = ttf_context.load_font(format!("resources/fonts/{}", font_name), font_size).expect("Could not load font file!");
    font.set_style(sdl2::ttf::FontStyle::BOLD);

    let mut chrs: [u8; 256] = [0; 256];
    for chr_ix in 0..256 {
        chrs[chr_ix] = chr_ix as u8;
    }

    let text_surface = font.render_latin1(&chrs[ASCII_START as usize .. ASCII_END as usize])
                           .blended(sdl2::pixels::Color::RGB(255, 255, 255))
                           .unwrap();

    let font_texture = texture_creator
        .create_texture_from_surface(&text_surface)
        .expect(&format!("Could not load font {}", font_name));

    return font_texture;
}

pub fn create_texture(texture_creator: &mut TextureCreator<WindowContext>, pixel_format: PixelFormatEnum, num_pixels: (u32, u32)) -> Texture {
    return texture_creator.create_texture_target(pixel_format, num_pixels.0, num_pixels.1).unwrap();
}

