use std::io::BufRead;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use sdl2::image::LoadTexture;

use roguelike_draw::drawcmd::*;


pub fn main() {
    let io_recv = spawn_input_reader();

    let width = 640;
    let height = 480;

    /* SDL2 */
    let sdl_context = sdl2::init().unwrap();
    let video = sdl_context.video().unwrap();
    let window = video.window("Rust Roguelike", width, height)
                      .position_centered().build().map_err(|e| e.to_string()).unwrap();

    let mut canvas = window.into_canvas()
                           .accelerated()
                           .build()
                           .map_err(|e| e.to_string()).unwrap();
    let mut texture_creator = canvas.texture_creator();
    let pixel_format = texture_creator.default_pixel_format();

    let pixels = (width, height);
    let dims = (20, 20);
    let mut panel = Panel::new(pixels, dims);

    let mut sprites = parse_atlas_file("resources/spriteAtlas.txt");
    let mut sprite_texture = texture_creator.load_texture("resources/spriteAtlas.png").expect("Could not load sprite atlas!");

    let mut ttf_context = sdl2::ttf::init().expect("Could not init SDL2 TTF!");
    let mut font_texture = load_font("Inconsolata-Bold.ttf", 34, &mut texture_creator, &mut ttf_context);

    let mut screen_texture = create_texture(&mut texture_creator, pixel_format, (width, height));

    loop {
        if let Ok(msg) = io_recv.recv_timeout(Duration::from_millis(100)) {
            if let Ok(_cmd) = msg.parse::<DrawCmd>() {
                panel.process_cmds(false, 
                                   &mut screen_texture,
                                   &mut canvas,
                                   &mut sprite_texture,
                                   &mut sprites,
                                   &mut font_texture);
            }
        }
    }
}

fn spawn_input_reader() -> Receiver<String> {
    let (io_send, io_recv) = mpsc::channel();

    thread::spawn(move || {
        let stdin = std::io::stdin();
        let stdin = stdin.lock().lines();

        for line in stdin {
            let text = line.unwrap();
            if !text.is_empty() {
                io_send.send(text).unwrap();
            }
        }
    });

    return io_recv;
}
