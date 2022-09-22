use std::str::FromStr;
use std::io::BufRead;

use sdl2::render::{Texture};
use sdl2::rect::Rect;

use crate::animation::{Sprite, SpriteKey};
use crate::panel::{Panel};


pub const FONT_WIDTH: i32 = 16;
pub const FONT_HEIGHT: i32 = 16;


pub struct SpriteSheet {
    pub name: String,
    pub num_sprites: usize,
    pub rows: usize,
    pub cols: usize,
    pub width: usize,
    pub height: usize,
    pub x_offset: u32,
    pub y_offset: u32,
}

impl SpriteSheet {
    pub fn new(name: String, num_sprites: usize, rows: usize, cols: usize, width: usize, height: usize, x_offset: u32, y_offset: u32) -> SpriteSheet {
        return SpriteSheet { name, num_sprites, rows, cols, width, height, x_offset, y_offset };
    }

    pub fn with_offset(name: String, x_offset: u32, y_offset: u32, width: usize, height: usize) -> SpriteSheet {
        let rows = height / FONT_HEIGHT as usize;
        let cols = width / FONT_WIDTH as usize;
        let num_sprites = cols * rows;

        return SpriteSheet {
            name,
            num_sprites,
            rows,
            cols,
            width,
            height,
            x_offset,
            y_offset,

        };
    }

    pub fn single(name: String, texture: Texture) -> SpriteSheet {
        let tex_info = texture.query();
        let width = tex_info.width as usize;
        let height = tex_info.height as usize;
        let num_sprites = 1;
        let rows = 1;
        let cols = 1;
        let x_offset = 0;
        let y_offset = 0;

        return SpriteSheet {
            name,
            num_sprites,
            rows,
            cols,
            width,
            height,
            x_offset,
            y_offset,
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

    // Get the source rectangle for a particular sprite
    // given by its index into the sprite sheet.
    pub fn sprite_src(&mut self, index: u32) -> Rect {
        let sprite_x;
        let sprite_y;
        let sprite_width;
        let sprite_height;

        let (num_cells_x, _num_cells_y) = self.num_cells();
        let index = index as usize;
        sprite_x = index % num_cells_x;
        sprite_y = index / num_cells_x;

        let dims = self.sprite_dims();
        sprite_width = dims.0;
        sprite_height = dims.1;

        let src = Rect::new(self.x_offset as i32 + (sprite_x * sprite_width) as i32,
                            self.y_offset as i32 + (sprite_y * sprite_height) as i32,
                            sprite_width as u32,
                            sprite_height as u32);

        return src;
    }
}

impl FromStr for SpriteSheet {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();

        let mut args = s.split(" ");

        let name = args.next().unwrap().to_string();
        let x = args.next().ok_or("no arg")?.parse::<u32>().map_err(|err| format!("{}", err))?;
        let y = args.next().ok_or("no arg")?.parse::<u32>().map_err(|err| format!("{}", err))?;
        let width = args.next().ok_or("no arg")?.parse::<usize>().map_err(|err| format!("{}", err))?;
        let height = args.next().ok_or("no arg")?.parse::<usize>().map_err(|err| format!("{}", err))?;

        let is_button = name.contains("Button");
        let mut sheet = SpriteSheet::with_offset(name, x, y, width, height);
        if is_button {
            sheet.rows = 1;
            sheet.cols = 1;
            sheet.num_sprites = 1;
        }

        return Ok(sheet);
    }
}

pub fn parse_atlas_file(atlas_file: &str) -> Vec<SpriteSheet> {
    let file =
        std::fs::File::open(&atlas_file).expect(&format!("Could not open atlas file '{}'", atlas_file));

    let mut sheets: Vec<SpriteSheet> = Vec::new();

    for line in std::io::BufReader::new(file).lines() {
        let line = line.unwrap();
        let line = line.to_string();

        if let Ok(sheet) = SpriteSheet::from_str(&line) { 
            sheets.push(sheet);
        }
    }

    return sheets;
}

pub fn lookup_spritekey(sprites: &Vec<SpriteSheet>, name: &str) -> SpriteKey {
    for (key, sprite_sheet) in sprites.iter().enumerate() {
        if sprite_sheet.name == *name {
            return key;
        }
    }

    dbg!(name);
    panic!("Could not find sprite");
}

