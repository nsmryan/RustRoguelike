use slotmap::dense::*;

use sdl2::render::{Texture, WindowCanvas, TextureCreator};
use sdl2::video::WindowContext;
use sdl2::rect::Rect;
use sdl2::pixels::{Color as Sdl2Color};

use roguelike_core::types::*;
use roguelike_core::constants::*;
use roguelike_core::animation::{AnimKey, Effect, SpriteKey, Animation, Sprite, SpriteIndex};

use crate::plat::*;


pub struct DisplayState {
    pub font_image: Texture,
    pub sprites: DenseSlotMap<SpriteKey, SpriteSheet>,
    pub display_overlays: bool,
    pub screen_sections: Plan,
    pub zones: Vec<Plot>,
    pub canvas: WindowCanvas,
    pub effects: Vec<Effect>,
    pub animations: DenseSlotMap<AnimKey, Animation>,
    pub background: Option<Texture>,
    pub texture_creator: TextureCreator<WindowContext>,
}

impl DisplayState {
    pub fn new(screen_sections: Plan,
               font_image: Texture,
               sprites: DenseSlotMap<SpriteKey, SpriteSheet>,
               canvas: WindowCanvas) -> DisplayState {

        let texture_creator = canvas.texture_creator();
        return DisplayState {
            font_image,
            sprites,
            display_overlays: false,
            screen_sections,
            canvas,
            zones: Vec::new(),
            effects: Vec::new(),
            animations: DenseSlotMap::new(),
            background: None,
            texture_creator,
        };
    }

    pub fn lookup_spritekey(&self, name: &String) -> Option<SpriteKey> {
        for (key, sprite_sheet) in self.sprites.iter() {
            if sprite_sheet.name == *name {
                return Some(key);
            }
        }

        return None;
    }

    pub fn new_sprite(&self, name: String, speed: f32) -> Option<Sprite> {
        if let Some(sprite_key) = self.lookup_spritekey(&name) {
            let max_index = self.sprites[sprite_key].num_sprites;
            return Some(Sprite::make_sprite(name, sprite_key, max_index as f32, speed));
        }

        return None;
    }

    pub fn font_sprite(&self, chr: char) -> Option<Sprite> {
        if let Some(sprite_key) = self.lookup_spritekey(&"font".to_string()) {
            return Some(Sprite::new(format!("{}", chr),
                                    sprite_key,
                                    chr as i32 as SpriteIndex,
                                    chr as i32 as SpriteIndex,
                                    0.0));
        }

        return None;
    }

    pub fn draw_text(&mut self,
                     text: String,
                     pos: Pos,
                     color: Color,
                     area: &Area) {
        for (index, chr) in text.chars().enumerate() {
            let chr_pos = Pos::new(pos.x + index as i32, pos.y);
            self.draw_char(chr, chr_pos, color, area);
        }
    }

    pub fn draw_text_list(&mut self,
                          text_list: Vec<String>,
                          pos: Pos,
                          color: Color,
                          area: &Area) {
        let mut y_pos = 0;
        for text in text_list {
            self.draw_text(text, Pos::new(pos.x, pos.y + y_pos), color, area);
            y_pos += 1;
        }
    }

    pub fn draw_sprite(&mut self,
                       sprite: &Sprite,
                       pos: Pos,
                       color: Color,
                       area: &Area) {
        let sprite_sheet = &mut self.sprites[sprite.sprite_key];

        let sprites_per_row = sprite_sheet.sprites_per_row();
        let sprite_x = (sprite.index as usize) % sprites_per_row;
        let sprite_y = (sprite.index as usize) / sprites_per_row;

        let src = Rect::new(sprite_x as i32 * FONT_WIDTH,
                            sprite_y as i32 * FONT_HEIGHT,
                            FONT_WIDTH as u32,
                            FONT_HEIGHT as u32);

        let dst = area.char_rect(pos.x, pos.y);

        sprite_sheet.texture.set_color_mod(color.r, color.g, color.b);
        sprite_sheet.texture.set_alpha_mod(color.a);

        self.canvas.copy_ex(&sprite_sheet.texture,
                            Some(src),
                            Some(dst),
                            0.0,
                            None,
                            false,
                            false).unwrap();
    }

    pub fn draw_char(&mut self,
                     chr: char,
                     pos: Pos,
                     color: Color,
                     area: &Area) {
        let chr_x = (chr as i32) % FONT_WIDTH;
        let chr_y = (chr as i32) / FONT_HEIGHT;

        let src = Rect::new((chr_x * FONT_WIDTH) as i32,
                            (chr_y * FONT_HEIGHT) as i32,
                            FONT_WIDTH as u32,
                            FONT_HEIGHT as u32);

        let dst = area.char_rect(pos.x, pos.y);

        self.font_image.set_color_mod(color.r, color.g, color.b);
        self.font_image.set_alpha_mod(color.a);

        self.canvas.copy_ex(&self.font_image,
                            Some(src),
                            Some(dst),
                            0.0,
                            None,
                            false,
                            false).unwrap();
    }

    pub fn draw_tile_outline(&mut self, pos: Pos, area: &Area, color: Color) {
        let tile_rect = area.char_rect(pos.x, pos.y);
        self.canvas.set_draw_color(Sdl2Color::RGBA(color.r, color.g, color.b, color.a));
        self.canvas.draw_rect(tile_rect).unwrap();
    }

    pub fn play_animation(&mut self, animation: Animation) -> AnimKey {
        return self.animations.insert(animation);
    }

    pub fn play_effect(&mut self, effect: Effect) {
        self.effects.push(effect);
    }
}


pub struct SpriteSheet {
    pub texture: Texture,
    pub name: String,
    pub num_sprites: usize,
    pub rows: usize,
}

impl SpriteSheet {
    pub fn new(name: String, texture: Texture, rows: usize) -> SpriteSheet {
        let num_sprites_per_row = texture.query().width as usize / FONT_WIDTH as usize;
        let num_sprites = num_sprites_per_row * rows;

        return SpriteSheet {
            texture,
            name,
            num_sprites,
            rows,
        };
    }

    pub fn sprites_per_row(&self) -> usize {
         return self.num_sprites / self.rows;
    }
}


#[derive(PartialEq, Debug, Copy, Clone)]
pub struct Area {
    pub x_offset: i32,
    pub y_offset: i32,
    pub width: usize,
    pub height: usize,
    pub font_width: usize,
    pub font_height: usize,
}

impl Area {
    pub fn new(x_offset: i32,
               y_offset: i32,
               width: usize,
               height: usize,
               font_width: usize,
               font_height: usize) -> Area {
        Area { x_offset,
               y_offset,
               width,
               height,
               font_width,
               font_height,
        }
    }

    pub fn char_rect(&self, x: i32, y: i32) -> Rect {
        return Rect::new(self.x_offset + x * self.font_width as i32,
                         self.y_offset + y * self.font_height as i32,
                         self.font_width as u32,
                         self.font_height as u32);
    }

    pub fn get_rect(&self) -> Rect {
        return Rect::new(self.x_offset,
                         self.y_offset,
                         self.width as u32,
                         self.height as u32);
    }
}


pub fn engine_color(color: &Color) -> Sdl2Color {
    return Sdl2Color::RGBA(color.r, color.g, color.b, color.a);
}

pub fn draw_char(canvas: &mut WindowCanvas,
                 font_image: &mut Texture,
                 chr: char,
                 pos: Pos,
                 color: Color,
                 area: &Area) {
    let chr_x = (chr as i32) % FONT_WIDTH;
    let chr_y = (chr as i32) / FONT_HEIGHT;

    let src = Rect::new((chr_x * FONT_WIDTH) as i32,
                        (chr_y * FONT_HEIGHT) as i32,
                        FONT_WIDTH as u32,
                        FONT_HEIGHT as u32);

    let dst = area.char_rect(pos.x, pos.y);

    font_image.set_color_mod(color.r, color.g, color.b);
    font_image.set_alpha_mod(color.a);

    canvas.copy_ex(font_image,
                   Some(src),
                   Some(dst),
                   0.0,
                   None,
                   false,
                   false).unwrap();
}
