use std::fs;

use sdl2::image::LoadTexture;
use sdl2::render::{Texture, TextureCreator};
use sdl2::video::WindowContext;
use sdl2::ttf::Sdl2TtfContext;

use roguelike_core::constants::*;

use crate::display::*;


/// load a ttf font file and render all ascii characters onto a 16x16 grid.
/// the resulting texture is then used for rendering by copying character squares
/// for individual ascii characters.
pub fn load_font(ttf_context: &Sdl2TtfContext,
                 texture_creator: &TextureCreator<WindowContext>,
                 file_name: &str,
                 font_size: u16) -> Texture {
    let mut font = ttf_context.load_font(format!("resources/fonts/{}", file_name), font_size).expect("Could not load font file!");
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

    return font_texture;
}

