use std::fs;

use sdl2::image::LoadTexture;
use sdl2::render::{Texture, TextureCreator};
use sdl2::video::WindowContext;
use sdl2::ttf::Sdl2TtfContext;

use roguelike_core::constants::*;

use crate::display::*;


/// load all png images in the resources/animations directory as sprites.
/// This assumes that all sprites have the same width and height. Otherwise we
/// will need a configuration file of some kind to specify the dimensions of the sprites.
pub fn load_sprites(texture_creator: &TextureCreator<WindowContext>, display: &mut Display) {
    load_sprites_from("resources/animations".to_string(), texture_creator, display);
}

pub fn load_sprites_from(path: String, texture_creator: &TextureCreator<WindowContext>, display: &mut Display) {
    for entry in fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();

        let file_name = path.as_path().to_str().unwrap();
        let sprite_name = path.as_path().file_stem().unwrap().to_str().unwrap();
        if let Ok(metadata) = path.metadata() {
            if metadata.is_file() && file_name.ends_with("png") {
                load_sprite(texture_creator, display, file_name, &sprite_name);
            } else if metadata.is_dir() {
                // NOTE likely unnecessary String
                load_sprites_from(path.to_str().unwrap().to_string(), texture_creator, display);
            }
        }
    }
}

pub fn load_sprite(texture_creator: &TextureCreator<WindowContext>,
                   display: &mut Display,
                   path: &str,
                   sprite_name: &str) {
    let texture = texture_creator.load_texture(path).expect("Could not load texture!");
    display.add_spritesheet(sprite_name.to_string(), texture);
}

/// load a ttf font file and render all ascii characters onto a 16x16 grid.
/// the resulting texture is then used for rendering by copying character squares
/// for individual ascii characters.
pub fn load_font(ttf_context: &Sdl2TtfContext,
                 texture_creator: &TextureCreator<WindowContext>,
                 file_name: String,
                 font_size: u16) -> Texture {
    let mut font = ttf_context.load_font(format!("resources/{}", file_name), font_size).expect("Could not load font file!");
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
        .expect(&format!("Could not load font {}", file_name));

    /*
    // assumes monospace font- otherwise none of this works
    let (char_width, char_height) = font.size_of_char('a').unwrap();

    let mut font_texture =
        texture_creator.create_texture_target(pixel_format,
                                              char_width as u32 * 16,
                                              char_height as u32 * 16).unwrap();

    canvas.with_texture_canvas(&mut font_texture, |canvas| {
        canvas.set_draw_color(sdl2::pixels::Color::RGB(255, 0, 255));
        canvas.clear();
        for chr_ix in 1..=255u8 {
            let chr_surface = font.render_latin1(&[chr_ix])
                                  .blended(sdl2::pixels::Color::RGB(0, 0, 0))
                                  .unwrap();
            let char_texture = chr_surface.as_texture(&texture_creator).unwrap();

            let char_rect = sdl2::rect::Rect::new(chr_ix as i32 % 16, chr_ix as i32 / 16, char_width, char_height);

            canvas.copy(&char_texture, None, char_rect).unwrap();
        }
    }).unwrap();
    */

    return font_texture;
}

