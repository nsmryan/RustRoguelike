#![allow(dead_code)]

extern crate num;
extern crate rand;
extern crate roguelike_core;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;

use std::env;
use std::fs::File;
use std::io::Read;

use roguelike_core::config::Config;

fn main() {
    let args = env::args().collect::<Vec<String>>();

    let config: Config;
    {
        let mut file =
            File::open("config.json").expect("Could not open/parse config file config.json");
        let mut config_string = String::new();
        file.read_to_string(&mut config_string)
            .expect("Could not read contents of config.json");
        config = serde_json::from_str(&config_string).expect("Could not parse config.json file!");
    }

    roguelike_engine::run(&args, config).unwrap();
}
