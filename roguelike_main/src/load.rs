use sdl2::image::LoadTexture;
use sdl2::render::{WindowCanvas, Texture, TextureCreator};
use sdl2::video::WindowContext;
use sdl2::ttf::Sdl2TtfContext;

use walkdir::WalkDir;

use roguelike_core::constants::*;

use crate::display::*;


fn load_sprites(texture_creator: &TextureCreator<WindowContext>, display: &mut Display) {
    load_sprite(texture_creator, display, "animations/player/Player_Idle.png", "player_idle", 1);
    load_sprite(texture_creator, display, "animations/player/Player_Crouch.png", "player_crouching", 1);
    load_sprite(texture_creator, display, "animations/player/Player_Idle_Dagger_Crouch.png", "player_crouch_dagger", 1);
    load_sprite(texture_creator, display, "animations/player/player_attack.png", "player_attack", 1);
    load_sprite(texture_creator, display, "animations/player/player_attack_Hammer.png", "player_attack_hammer", 1);
    load_sprite(texture_creator, display, "animations/player/player_attack_Dagger.png", "player_attack_dagger", 1);
    load_sprite(texture_creator, display, "animations/player/Player_Idle_Dagger.png", "player_idle_dagger", 1);
    load_sprite(texture_creator, display, "animations/player/Player_Idle_Hammer.png", "player_idle_hammer", 1);
    load_sprite(texture_creator, display, "animations/player/Player_Idle_Shield.png", "player_idle_shield", 1);
    load_sprite(texture_creator, display, "animations/player/player_vault.png", "player_vault", 1);
    load_sprite(texture_creator, display, "animations/player/player_wallkick.png", "player_wall_kick", 1);
    load_sprite(texture_creator, display, "animations/monster1/Gol_Idle.png", "gol_idle", 1);
    load_sprite(texture_creator, display, "animations/monster1/Gol_Die.png", "gol_die", 1);
    load_sprite(texture_creator, display, "animations/monster3/Pawn_Idle.png", "elf_idle", 1);
    load_sprite(texture_creator, display, "animations/traps/DamageTrap.png", "spikes", 1);
    load_sprite(texture_creator, display, "resources/rexpaint16x16.png", "font", 16);
    load_sprite(texture_creator, display, "animations/traps/McMuffin.png", "key", 1);

    for entry in WalkDir::new("animations/autoload/") {
        let entry = entry.unwrap();
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_file() && file_name.ends_with("png") {
                load_sprite(texture_creator, display, path.to_str().unwrap(), &file_name, 1);
            }
        }
    }
}

fn load_sprite(texture_creator: &TextureCreator<WindowContext>,
               display: &mut Display,
               path: &str,
               sprite_name: &str,
               rows: usize) {
    let texture = texture_creator.load_texture(path).expect("Could not load texture!");
    display.add_spritesheet(sprite_name.to_string(), texture, rows);
}

fn load_font(ttf_context: &Sdl2TtfContext,
             texture_creator: &TextureCreator<WindowContext>,
             canvas: &mut WindowCanvas,
             file_name: String,
             font_size: u16) -> Texture {
    let font = ttf_context.load_font(format!("resources/{}", file_name), font_size).expect("Could not load font file!");

    let pixel_format = texture_creator.default_pixel_format();

    // assumes monospace font- otherwise none of this works
    let (char_width, char_height) = font.size_of_char('a').unwrap();

    let mut font_texture =
        texture_creator.create_texture_target(pixel_format,
                                              char_width as u32 * FONT_WIDTH as u32,
                                              char_height as u32 * FONT_HEIGHT as u32).unwrap();

    canvas.with_texture_canvas(&mut font_texture, |canvas| {
        canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 0));
        canvas.clear();
        for chr_ix in 1..=255u8 {
            let chr_surface = font.render_latin1(&[chr_ix]).solid(sdl2::pixels::Color::WHITE).unwrap();
            let char_texture = chr_surface.as_texture(&texture_creator).unwrap();

            let char_rect = sdl2::rect::Rect::new(chr_ix as i32 % 16, chr_ix as i32 / 16, char_width, char_height);

            canvas.copy(&char_texture, None, char_rect).unwrap();
        }
    }).unwrap();

    return font_texture;
}

