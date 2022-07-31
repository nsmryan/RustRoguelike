use std::collections::HashMap;

use sdl2::render::{Texture, WindowCanvas};
use sdl2::rect::Rect;
use sdl2::pixels::{Color as Sdl2Color};

use roguelike_utils::math::*;

use roguelike_map::{PlayerDirection};

use crate::animation::{Sprite};
use crate::area::{Area};
use crate::drawcmd::{SpriteSheet, process_draw_cmd, DrawCmd, Justify};


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

    pub fn sprite_float_scaled_cmd(&mut self, sprite: Sprite, color: Color, x: f32, y: f32, x_scale: f32, y_scale: f32) {
        let cmd = DrawCmd::SpriteFloat(sprite, color, x, y, x_scale, y_scale);
        self.draw_cmd(cmd);
    }

    pub fn sprite_float_cmd(&mut self, sprite: Sprite, color: Color, x: f32, y: f32) {
        let cmd = DrawCmd::SpriteFloat(sprite, color, x, y, 1.0, 1.0);
        self.draw_cmd(cmd);
    }

    pub fn sprite_cmd(&mut self, sprite: Sprite, color: Color, pos: Pos) {
        let cmd = DrawCmd::Sprite(sprite, color, pos);
        self.draw_cmd(cmd);
    }

    pub fn sprite_scaled_cmd(&mut self, sprite: Sprite, scale: f32, direction: PlayerDirection, color: Color, pos: Pos) {
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

    pub fn justify_cmd(&mut self, text: &str, justify: Justify, fg_color: Color, bg_color: Color, text_pos: Pos, width: u32, scale: f32) {
        let string = text.to_string();
        let cmd = DrawCmd::TextJustify(string, justify, fg_color, bg_color, text_pos, width, scale);
        self.draw_cmd(cmd);
    }

    pub fn text_cmd(&mut self, text: &str, color: Color, pos: Pos, scale: f32) {
        let string = text.to_string();
        let cmd = DrawCmd::Text(string, color, pos, scale);
        self.draw_cmd(cmd);
    }

    // NOTE This command uses f32 positions where 0.0 is the first cell, 1.0 is the second cell,
    // and 0.5 is the pixel in the middle of the cell, etc.
    pub fn text_float_cmd(&mut self, text: &str, color: Color, x: f32, y: f32, scale: f32) {
        let string = text.to_string();
        let cmd = DrawCmd::TextFloat(string, color, x, y, scale);
        self.draw_cmd(cmd);
    }

    pub fn text_list_cmd(&mut self, text_list: &Vec<String>, color: Color, cell: Pos, scale: f32) {
        for (index, text) in text_list.iter().enumerate() {
            let text_cell = Pos::new(cell.x, cell.y + index as i32);
            self.text_cmd(text, color, text_cell, scale);
        }
    }

    pub fn colored_text_list_cmd(&mut self, text_list: &Vec<(Color, String)>, cell: Pos, scale: f32) {
        for (index, (color, text)) in text_list.iter().enumerate() {
            let text_cell = Pos::new(cell.x, cell.y + index as i32);
            self.text_cmd(text, *color, text_cell, scale);
        }
    }

    pub fn rect_cmd(&mut self, pos: Pos, dims: (u32, u32), offset: f32, filled: bool, color: Color) {
        let cmd = DrawCmd::Rect(pos, dims.0, dims.1, offset, filled, color);
        self.draw_cmd(cmd);
    }

    pub fn rect_float_cmd(&mut self, x: f32, y: f32, dims: (f32, f32), filled: bool, color: Color) {
        let cmd = DrawCmd::RectFloat(x, y, dims.0, dims.1, filled, color);
        self.draw_cmd(cmd);
    }

    pub fn fill_cmd(&mut self, pos: Pos, color: Color) {
        let cmd = DrawCmd::Fill(pos, color);
        self.draw_cmd(cmd);
    }

    pub fn draw_cmd(&mut self, cmd: DrawCmd) {
        self.draw_cmds.push(cmd);
    }

    pub fn process_cmds_if_new(&mut self,
                               clear: bool,
                               texture: &mut Texture,
                               canvas: &mut WindowCanvas,
                               sprite_texture: &mut Texture,
                               sprites: &mut Vec<SpriteSheet>,
                               font_texture: &mut Texture) {
        // If there are no commands, just clear the panel with black.
        if self.draw_cmds.len() == 0 {
            canvas.with_texture_canvas(texture, |canvas| {
                  canvas.set_draw_color(Sdl2Color::RGBA(0, 0, 0, 255));
                  canvas.clear();
            }).unwrap();
        } else if self.draw_cmds != self.old_draw_cmds {
            // As a simple optimization, only redraw if the commands haven't changed. This is common
            // for informational panels.
            self.process_cmds(clear,
                              texture,
                              canvas,
                              sprite_texture,
                              sprites,
                              font_texture);

            self.old_draw_cmds.clear();
            std::mem::swap(&mut self.draw_cmds, &mut self.old_draw_cmds);
        }
    }

    pub fn process_cmds(&mut self,
                        clear: bool,
                        texture: &mut Texture,
                        canvas: &mut WindowCanvas,
                        sprite_texture: &mut Texture,
                        sprites: &mut Vec<SpriteSheet>,
                        font_texture: &mut Texture) {
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
                        let is_fill = matches!(cmd, DrawCmd::Fill(_, _));

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
                process_draw_cmd(self, canvas, sprite_texture, sprites, font_texture, cmd);
            }
        }).unwrap();

        self.draw_cmds.clear();
    }

    pub fn outline_area(&mut self, area: &Area, cell_offset: f32) {
        let filled = false;
        let ui_color = Color::new(0xcd, 0xb4, 0x96, 255);
        self.rect_cmd(Pos::new(area.x_offset as i32, area.y_offset as i32),
                      (area.width as u32, area.height as u32),
                      cell_offset,
                      filled,
                      ui_color);
    }
}

