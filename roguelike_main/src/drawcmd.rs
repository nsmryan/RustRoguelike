use std::collections::HashMap;

use sdl2::render::{Texture, RenderTarget, WindowCanvas, BlendMode, Canvas};
use sdl2::rect::{Rect};
use sdl2::pixels::{Color as Sdl2Color};

use roguelike_core::movement::{Direction};
use roguelike_core::types::*;
use roguelike_core::constants::*;

use crate::animation::{Sprite, SpriteKey};


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Justify {
    Right,
    Center,
    Left,
}

// NOTE use of String prevents Copy trait
#[derive(Clone, Debug, PartialEq)]
pub enum DrawCmd {
    Sprite(Sprite, Color, Pos),
    SpriteScaled(Sprite, f32, Option<Direction>, Color, Pos),
    SpriteAtPixel(Sprite, Color, Pos),
    HighlightTile(Color, Pos),
    OutlineTile(Color, Pos),
    Text(String, Color, Pos),
    TextJustify(String, Justify, Color, Color, Pos, u32), // text, justify, fg color, bg color, pos, width in cells
    Rect(Pos, (u32, u32), f32, bool, Color), // start cell, num cells width/height, offset percent into cell, color
    Fill(Pos, Color),
}

impl DrawCmd {
    pub fn aligned(&self) -> bool {
        return !matches!(self, DrawCmd::SpriteAtPixel(_, _, _));
    }

    pub fn pos(&self) -> Pos {
        match self {
            DrawCmd::Sprite(_, _, pos) => *pos,
            DrawCmd::SpriteScaled(_, _, _, _, pos) => *pos,
            DrawCmd::SpriteAtPixel(_, _, pos) => *pos,
            DrawCmd::HighlightTile(_, pos) => *pos,
            DrawCmd::OutlineTile(_, pos) => *pos,
            DrawCmd::Text(_, _, pos) => *pos,
            DrawCmd::TextJustify(_, _, _, _, pos, _) => *pos,
            DrawCmd::Rect(pos, _, _, _, _) => *pos,
            DrawCmd::Fill(pos, _) => *pos,
        }
    }
}

fn process_draw_cmd(panel: &Panel, canvas: &mut WindowCanvas, sprites: &mut Vec<SpriteSheet>,  cmd: &DrawCmd) {
    match cmd {
        DrawCmd::Sprite(sprite, color, pos) => {
            let sprite_sheet = &mut sprites[sprite.key];
            let (cell_width, cell_height) = panel.cell_dims();

            let pos = Pos::new(pos.x * cell_width as i32, pos.y * cell_height as i32);

            let cell_dims = panel.cell_dims();

            let src = sprite_sheet.sprite_src(sprite.index as usize);

            let (cell_width, cell_height) = cell_dims;

            let dst = Rect::new(pos.x as i32,
                                pos.y as i32,
                                cell_width as u32,
                                cell_height as u32);

            canvas.set_blend_mode(BlendMode::Blend);
            sprite_sheet.texture.set_color_mod(color.r, color.g, color.b);
            sprite_sheet.texture.set_alpha_mod(color.a);

            canvas.copy_ex(&sprite_sheet.texture,
                           Some(src),
                           Some(dst),
                           sprite.rotation,
                           None,
                           sprite.flip_horiz,
                           sprite.flip_vert).unwrap();
        }

        DrawCmd::SpriteScaled(sprite, scale, direction, color, pos) => {
            let cell_dims = panel.cell_dims();
            let sprite_sheet = &mut sprites[sprite.key];

            let src = sprite_sheet.sprite_src(sprite.index as usize);

            let (cell_width, cell_height) = cell_dims;
            let dst_width = (cell_width as f32 * scale) as u32;
            let dst_height = (cell_height as f32 * scale) as u32;

            let x_margin = ((cell_width - dst_width) / 2) as i32;
            let y_margin = ((cell_height - dst_height) / 2) as i32;

            let mut dst_x = pos.x * cell_width as i32;
            let mut dst_y = pos.y * cell_height as i32;
            match direction {
                None => {
                    dst_x += x_margin;
                    dst_y += y_margin;
                }
                
                Some(Direction::Left) => {
                    dst_y += y_margin;
                }

                Some(Direction::Right) => {
                    dst_x += cell_width as i32 - dst_width as i32;
                    dst_y += y_margin;
                }

                Some(Direction::Up) => {
                    dst_x += x_margin;
                }

                Some(Direction::Down) => {
                    dst_x += x_margin;
                    dst_y += cell_height as i32 - dst_height as i32;
                }

                Some(Direction::DownLeft) => {
                    dst_y += cell_height as i32 - dst_height as i32;
                }

                Some(Direction::DownRight) => {
                    dst_x += cell_width as i32 - dst_width as i32;
                    dst_y += cell_height as i32 - dst_height as i32;
                }

                Some(Direction::UpLeft) => {
                }

                Some(Direction::UpRight) => {
                    dst_x += cell_width as i32  - dst_width as i32;
                }
            }

            let dst = Rect::new(dst_x,
                                dst_y,
                                dst_width,
                                dst_height);

            canvas.set_blend_mode(BlendMode::Blend);
            sprite_sheet.texture.set_color_mod(color.r, color.g, color.b);
            sprite_sheet.texture.set_alpha_mod(color.a);

            canvas.copy_ex(&sprite_sheet.texture,
                           Some(src),
                           Some(dst),
                           sprite.rotation,
                           None,
                           false,
                           false).unwrap();
        }

        DrawCmd::SpriteAtPixel(sprite, color, pos) => {
            let sprite_sheet = &mut sprites[sprite.key];

            let pos = Pos::new(pos.x, pos.y);

            let cell_dims = panel.cell_dims();

            let src = sprite_sheet.sprite_src(sprite.index as usize);

            let (cell_width, cell_height) = cell_dims;

            let dst = Rect::new(pos.x as i32,
                                pos.y as i32,
                                cell_width as u32,
                                cell_height as u32);

            canvas.set_blend_mode(BlendMode::Blend);
            sprite_sheet.texture.set_color_mod(color.r, color.g, color.b);
            sprite_sheet.texture.set_alpha_mod(color.a);

            canvas.copy_ex(&sprite_sheet.texture,
                           Some(src),
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

        DrawCmd::TextJustify(string, justify, fg_color, bg_color, start_pos, width) => {
            let ascii_width = ASCII_END - ASCII_START;

            let sprite_key = lookup_spritekey(sprites, "font");
            let sprite_sheet = &mut sprites[sprite_key];
            let query = sprite_sheet.texture.query();

            let cell_dims = panel.cell_dims();
            let (cell_width, cell_height) = cell_dims;

            let font_width = query.width / ascii_width;
            let font_height = query.height;

            let char_height = cell_height;
            let char_width = ((cell_width * font_width) / font_height) - 16;


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

            sprite_sheet.texture.set_color_mod(fg_color.r, fg_color.g, fg_color.b);
            sprite_sheet.texture.set_alpha_mod(fg_color.a);

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

                canvas.copy_ex(&sprite_sheet.texture,
                                     Some(src),
                                     Some(dst),
                                     0.0,
                                     None,
                                     false,
                                     false).unwrap();
            }
        }

        DrawCmd::Text(string, color, start_pos) => {
            let ascii_width = ASCII_END - ASCII_START;

            let sprite_key = lookup_spritekey(sprites, "font");
            let sprite_sheet = &mut sprites[sprite_key];
            let query = sprite_sheet.texture.query();

            let cell_dims = panel.cell_dims();
            let (_cell_width, cell_height) = cell_dims;

            let font_width = query.width / ascii_width;
            let font_height = query.height;

            let char_height = cell_height;
            let char_width = ((cell_height * font_width) / font_height) - 16;

            canvas.set_blend_mode(BlendMode::Blend);
            sprite_sheet.texture.set_color_mod(color.r, color.g, color.b);
            sprite_sheet.texture.set_alpha_mod(color.a);

            let mut pos = *start_pos;
            for chr in string.chars() {
                let chr_num = chr.to_lowercase().next().unwrap();
                let chr_index = chr_num as i32 - ASCII_START as i32;

                let src = Rect::new(font_width as i32 * chr_index,
                                    0,
                                    font_width,
                                    font_height);

                let dst_pos = Pos::new(pos.x * char_width as i32,
                                       pos.y * cell_height as i32);
                let dst = Rect::new(dst_pos.x as i32,
                                    dst_pos.y as i32,
                                    char_width as u32,
                                    char_height as u32);

                canvas.copy_ex(&sprite_sheet.texture,
                                     Some(src),
                                     Some(dst),
                                     0.0,
                                     None,
                                     false,
                                     false).unwrap();
                pos.x += 1;
            }
        }

        DrawCmd::Rect(pos, dims, offset, filled, color) => {
            assert!(*offset < 1.0, "offset >= 1 misaligns the starting cell!");

            let (cell_width, cell_height) = panel.cell_dims();

            canvas.set_draw_color(sdl2_color(*color));

            let offset_x = (cell_width as f32 * offset) as i32;
            let x: i32 = cell_width as i32 * pos.x + offset_x as i32;

            let offset_y = (cell_height as f32 * offset) as i32;
            let y: i32 = cell_height as i32 * pos.y + offset_y as i32;

            let width = cell_width * dims.0 - (2 * offset_x as u32);
            let height = cell_height * dims.1 - (2 * offset_y as u32);

            if *filled {
                canvas.fill_rect(Rect::new(x, y, width, height)).unwrap();
            } else {
                canvas.fill_rect(Rect::new(x, y, 3, height)).unwrap();
                canvas.fill_rect(Rect::new(x, y, width, 3)).unwrap();
                canvas.fill_rect(Rect::new(x + width as i32, y, 3, height)).unwrap();
                canvas.fill_rect(Rect::new(x, y + height as i32, width, 3)).unwrap();
            }
        }

        DrawCmd::Fill(pos, color) => {
            let (cell_width, cell_height) = panel.cell_dims();
            canvas.set_draw_color(sdl2_color(*color));
            canvas.fill_rect(Rect::new(pos.x * cell_width as i32, pos.y * cell_height as i32, cell_width, cell_height)).unwrap();
        }
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
pub struct Area {
    pub x_offset: usize,
    pub y_offset: usize,
    pub width: usize,
    pub height: usize,
}

impl Area {
    pub fn new(width: usize, height: usize) -> Area {
        return Area { x_offset: 0, y_offset: 0, width, height };
    }

    pub fn new_at(x_offset: usize, y_offset: usize, width: usize, height: usize) -> Area {
        return Area { x_offset, y_offset, width, height };
    }

    pub fn split_left(&self, left_width: usize) -> (Area, Area) {
        assert!(left_width <= self.width);

        let right_width = self.width - left_width;
        let left = Area::new_at(self.x_offset, self.y_offset, left_width, self.height);
        let right = Area::new_at(self.x_offset + left_width, self.y_offset, right_width, self.height);

        return (left, right);
    }

    pub fn split_right(&self, right_width: usize) -> (Area, Area) {
        assert!(right_width <= self.width);

        let left_width = self.width - right_width;
        let left = Area::new_at(self.x_offset, self.y_offset, left_width, self.height);
        let right = Area::new_at(self.x_offset + left_width, self.y_offset, right_width, self.height);

        return (left, right);
    }

    pub fn split_top(&self, top_height: usize) -> (Area, Area) {
        assert!(top_height <= self.height);

        let top = Area::new_at(self.x_offset, self.y_offset, self.width, top_height);
        let bottom = Area::new_at(self.x_offset, self.y_offset + top_height, self.width, self.height - top_height);

        return (top, bottom);
    }

    pub fn split_bottom(&self, bottom_height: usize) -> (Area, Area) {
        assert!(bottom_height <= self.height);

        let top_height = self.height - bottom_height;
        let top = Area::new_at(self.x_offset, self.y_offset, self.width, top_height);
        let bottom = Area::new_at(self.x_offset, self.y_offset + top_height, self.width, bottom_height);

        return (top, bottom);
    }

    pub fn centered(&self, width: usize, height: usize) -> Area {
        assert!(width <= self.width);
        assert!(height <= self.height);

        let x_offset = (self.width - width) / 2;
        let y_offset = (self.height - height) / 2;

        return Area::new_at(x_offset, y_offset, width, height);
    }

    pub fn cell_at_pixel(&self, pixel_pos: Pos) -> Option<(usize, usize)> {
        let cell_pos = Pos::new(pixel_pos.x / self.width as i32, pixel_pos.y / self.height as i32);

        return self.cell_at(cell_pos);
    }

    pub fn cell_at(&self, cell_pos: Pos) -> Option<(usize, usize)> {
        if cell_pos.x as usize >= self.x_offset && (cell_pos.x as usize) < self.x_offset + self.width &&
           cell_pos.y as usize >= self.y_offset && (cell_pos.y as usize) < self.y_offset + self.height {
               return Some((cell_pos.x as usize - self.x_offset, cell_pos.y as usize - self.y_offset));
        }

        return None;
    }
}

#[test]
pub fn test_area_splits_left() {
    let section = Area::new(100, 100);
    let (left, right) = section.split_left(20);

    assert_eq!(0, left.x_offset);
    assert_eq!(0, left.y_offset);
    assert_eq!(20, right.x_offset);
    assert_eq!(0, right.y_offset);

    assert_eq!(20, left.width);
    assert_eq!(80, right.width);
    assert_eq!(100, left.height);
    assert_eq!(100, right.height);
}

#[test]
pub fn test_area_splits_top() {
    let section = Area::new(100, 100);
    let (top, bottom) = section.split_top(20);

    assert_eq!(0, top.x_offset);
    assert_eq!(0, top.y_offset);
    assert_eq!(0, bottom.x_offset);
    assert_eq!(20, bottom.y_offset);

    assert_eq!(100, top.width);
    assert_eq!(100, bottom.width);
    assert_eq!(20, top.height);
    assert_eq!(80, bottom.height);
}

#[test]
pub fn test_area_splits_right() {
    let section = Area::new(100, 100);
    let (left, right) = section.split_right(20);

    assert_eq!(0, left.x_offset);
    assert_eq!(0, left.y_offset);
    assert_eq!(80, right.x_offset);
    assert_eq!(0, right.y_offset);

    assert_eq!(80, left.width);
    assert_eq!(20, right.width);
    assert_eq!(100, left.height);
    assert_eq!(100, right.height);
}

#[test]
pub fn test_area_splits_bottom() {
    let section = Area::new(100, 100);
    let (top, bottom) = section.split_bottom(20);
    assert_eq!(0, top.x_offset);
    assert_eq!(0, top.y_offset);
    assert_eq!(0, bottom.x_offset);
    assert_eq!(80, bottom.y_offset);

    assert_eq!(100, top.width);
    assert_eq!(100, bottom.width);
    assert_eq!(80, top.height);
    assert_eq!(20, bottom.height);
}

#[derive(Clone, Debug, PartialEq)]
pub struct Panel {
    pub cells: (u32, u32),
    pub num_pixels: (u32, u32),
    pub draw_cmds: Vec<DrawCmd>,
    pub old_draw_cmds: Vec<DrawCmd>,
}

impl Panel {
    pub fn new(num_pixels: (u32, u32), cells: (u32, u32)) -> Panel {
        return Panel { cells, num_pixels, draw_cmds: Vec::new(), old_draw_cmds: Vec::new(), };
    }

    pub fn cell_dims(&self) -> (u32, u32) {
        return (self.num_pixels.0 / self.cells.0, self.num_pixels.1 / self.cells.1);
    }

    pub fn area(&self) -> Area {
        return Area::new(self.cells.0 as usize, self.cells.1 as usize);
    }

    pub fn cell_from_pixel(&self, pixel: Pos) -> Pos {
        let dims = self.cell_dims();
        return Pos::new(pixel.x / dims.0 as i32, pixel.y / dims.1 as i32);
    }

    pub fn pixel_from_cell(&self, cell: Pos) -> Pos {
        let dims = self.cell_dims();
        return Pos::new(cell.x * dims.0 as i32, cell.y * dims.1 as i32);
    }

    pub fn get_rect_full(&self) -> Rect {
        return Rect::new(0, 0, self.num_pixels.0, self.num_pixels.1);
    }

    pub fn get_rect_up_left(&self, width: usize, height: usize) -> Rect {
        assert!(width as u32 <= self.cells.0);
        assert!(height as u32 <= self.cells.1);

        let (cell_width, cell_height) = self.cell_dims();

        let pixel_width = width as u32 * cell_width;
        let pixel_height = height as u32 * cell_height;

        return Rect::new(0, 0, pixel_width, pixel_height);
    }

    pub fn get_rect_from_area(&self, area: &Area) -> Rect {
        let cell_dims = self.cell_dims();

        let x_offset = area.x_offset as f32 * cell_dims.0 as f32;
        let y_offset = area.y_offset as f32 * cell_dims.1 as f32;

        let width = (area.width as f32 * cell_dims.0 as f32) as u32;
        let height = (area.height as f32 * cell_dims.1 as f32) as u32;

        // don't draw off the screen
        assert!(x_offset as u32 + width <= self.num_pixels.0);
        assert!(y_offset as u32 + height <= self.num_pixels.1);

        return Rect::new(x_offset as i32, y_offset as i32, width, height);
    }

    pub fn get_rect_within(&self, area: &Area, target_dims: (u32, u32)) -> Rect {
        let (target_width, target_height) = target_dims;
        let base_rect = self.get_rect_from_area(area);

        let scale_x = base_rect.w as f32 / target_width as f32;
        let scale_y = base_rect.h as f32 / target_height as f32;

        let scaler;
        if scale_x * target_height as f32 > base_rect.h as f32 {
            scaler = scale_y;
        } else {
            scaler = scale_x;
        }

        let final_target_width = target_width as f32 * scaler;
        let final_target_height = target_height as f32 * scaler;

        let x_inner_offset = (base_rect.w as f32 - final_target_width) / 2.0;
        let y_inner_offset = (base_rect.h as f32 - final_target_height) / 2.0;
        let x_offset = base_rect.x + x_inner_offset as i32;
        let y_offset = base_rect.y + y_inner_offset as i32;

        // check that we don't reach past the destination rect we should be drawing within
        assert!((x_offset as f32 + final_target_width  as f32) <= base_rect.x as f32 + base_rect.w as f32);
        assert!((y_offset as f32 + final_target_height as f32) <= base_rect.y as f32 + base_rect.h as f32);

        return Rect::new(x_offset as i32,
                         y_offset as i32,
                         final_target_width as u32,
                         final_target_height as u32);
    }

    pub fn sprite_at_pixel_cmd(&mut self, sprite: Sprite, color: Color, pos: Pos) {
        let cmd = DrawCmd::SpriteAtPixel(sprite, color, pos);
        self.draw_cmd(cmd);
    }

    pub fn sprite_cmd(&mut self, sprite: Sprite, color: Color, pos: Pos) {
        let cmd = DrawCmd::Sprite(sprite, color, pos);
        self.draw_cmd(cmd);
    }

    pub fn sprite_scaled_cmd(&mut self, sprite: Sprite, scale: f32, direction: Option<Direction>, color: Color, pos: Pos) {
        let cmd = DrawCmd::SpriteScaled(sprite, scale, direction, color, pos);
        self.draw_cmd(cmd);
    }

    pub fn outline_cmd(&mut self, color: Color, pos: Pos) {
        let cmd = DrawCmd::OutlineTile(color, pos);
        self.draw_cmd(cmd);
    }

    pub fn highlight_cmd(&mut self, color: Color, pos: Pos) {
        let cmd = DrawCmd::HighlightTile(color, pos);
        self.draw_cmd(cmd);
    }

    pub fn justify_cmd(&mut self, text: &str, justify: Justify, fg_color: Color, bg_color: Color, text_pos: Pos, width: u32) {
        let string = text.to_string();
        let cmd = DrawCmd::TextJustify(string, justify, fg_color, bg_color, text_pos, width);
        self.draw_cmd(cmd);
    }

    pub fn text_cmd(&mut self, text: &str, color: Color, pos: Pos) {
        let string = text.to_string();
        let cmd = DrawCmd::Text(string, color, pos);
        self.draw_cmd(cmd);
    }

    pub fn text_list_cmd(&mut self, text_list: &Vec<String>, color: Color, cell: Pos) {
        for (index, text) in text_list.iter().enumerate() {
            let text_cell = Pos::new(cell.x, cell.y + index as i32);
            self.text_cmd(text, color, text_cell);
        }
    }

    pub fn rect_cmd(&mut self, pos: Pos, dims: (u32, u32), offset: f32, filled: bool, color: Color) {
        let cmd = DrawCmd::Rect(pos, dims, offset, filled, color);
        self.draw_cmd(cmd);
    }

    pub fn fill_cmd(&mut self, pos: Pos, color: Color) {
        let cmd = DrawCmd::Fill(pos, color);
        self.draw_cmd(cmd);
    }

    pub fn draw_cmd(&mut self, cmd: DrawCmd) {
        self.draw_cmds.push(cmd);
    }

    pub fn process_cmds(&mut self, clear: bool, texture: &mut Texture, canvas: &mut WindowCanvas, sprites: &mut Vec<SpriteSheet>) {
        // As a simple optimization, only redraw if the commands haven't changed. This is common
        // for informational panels.
        if self.draw_cmds != self.old_draw_cmds {
            // Collect a map of positions which are going to be filled, to avoid drawing
            // aligned sprites below those tiles.
            let mut fill_map = HashMap::<Pos, u32>::new();
            for cmd in self.draw_cmds.iter() {
                if let DrawCmd::Fill(pos, _color) = cmd {
                    if !fill_map.contains_key(pos) {
                        fill_map.insert(*pos, 0);
                    }
                    fill_map.get_mut(pos).map(|count| *count += 1);
                }
            }

            canvas.with_texture_canvas(texture, |canvas| {
                // we don't clear the map as the background was already drawn over it.
                // TODO consider removing background panel and just using map- it was an
                // optimization that we may be able to remove if other optimizations are good enough
                // TODO if the fill cmd optimization has enough of an effect, the background
                // panel optimization may be unnecessary
                if clear {
                  canvas.set_draw_color(Sdl2Color::RGBA(0, 0, 0, 255));
                  canvas.clear();
                }

                for cmd in self.draw_cmds.iter() {
                    // Check if there is going to be a fill in this position,
                    // in which case there is no need to draw an aligned command.
                    if cmd.aligned() {
                        let pos = cmd.pos();
                        if fill_map.contains_key(&pos) {
                            let is_fill =  matches!(cmd, DrawCmd::Fill(_, _));

                            if let Some(count) = fill_map.get_mut(&pos) {
                                if *count > 0 && is_fill {
                                    *count -= 1;
                                }

                                if *count > 0 {
                                    continue;
                                }
                            }
                        }
                    }
                    process_draw_cmd(self, canvas, sprites, cmd);
                }
            }).unwrap();

            self.old_draw_cmds.clear();
            std::mem::swap(&mut self.draw_cmds, &mut self.old_draw_cmds);
        }

        self.draw_cmds.clear();
    }
}

pub struct SpriteSheet {
    pub texture: Texture,
    pub name: String,
    pub num_sprites: usize,
    pub rows: usize,
    pub cols: usize,
    pub width: usize,
    pub height: usize,
}

impl SpriteSheet {
    pub fn new(name: String, texture: Texture) -> SpriteSheet {
        let tex_info = texture.query();
        let width = tex_info.width as usize;
        let height = tex_info.height as usize;

        let rows = height / FONT_HEIGHT as usize;
        let cols = width / FONT_WIDTH as usize;
        let num_sprites = cols * rows;

        return SpriteSheet {
            texture,
            name,
            num_sprites,
            rows,
            cols,
            width,
            height,
        };
    }

    pub fn num_cells(&self) -> (usize, usize) {
        return (self.cols, self.rows);
    }

    pub fn num_pixels(&self) -> (usize, usize) {
        return (self.width, self.height);
    }

    pub fn sprite_dims(&self) -> (usize, usize) {
        let (num_width, num_height) = self.num_cells();
        return (self.width / num_width, self.height / num_height);
    }

    fn sprite_src(&mut self, index: usize) -> Rect {
        let (num_cells_x, _num_cells_y) = self.num_cells();
        let sprite_x = index % num_cells_x;
        let sprite_y = index / num_cells_x;

        let (sprite_width, sprite_height) = self.sprite_dims();
        let src = Rect::new((sprite_x * sprite_width) as i32,
                            (sprite_y * sprite_height) as i32,
                            sprite_width as u32,
                            sprite_height as u32);

        return src;
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

pub fn lookup_spritekey(sprites: &Vec<SpriteSheet>, name: &str) -> SpriteKey {
    for (key, sprite_sheet) in sprites.iter().enumerate() {
        if sprite_sheet.name == *name {
            return key;
        }
    }

    panic!(format!("Could not find sprite '{}'", name));
}

fn sdl2_color(color: Color) -> Sdl2Color {
    return Sdl2Color::RGBA(color.r, color.g, color.b, color.a);
}

