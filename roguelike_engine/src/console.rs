use sdl2::keyboard::Keycode;


#[derive(Clone, Debug)]
pub struct Console {
    pub input: String,
    pub output: Vec<String>,
    pub history: Vec<String>,
    pub visible: bool,
    pub height: u32,
    pub time_at_open: f32,
}

impl Console {
    pub fn new() -> Console {
        return Console {
            input: String::new(),
            output: Vec::new(),
            history: Vec::new(),
            visible: false,
            height: 0,
            time_at_open: 0.0,
        };
    }

    pub fn key(&mut self, key: Keycode) {
        use Keycode::*;

        match key {
            A => println!("A"),
            B => println!("C"),
            _ => println!("{:?}", key),
        }
    }
}
