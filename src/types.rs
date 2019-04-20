use tcod::console::*;
use tcod::map::{Map as FovMap};
use tcod::input::Mouse;
use tcod::colors::*;

use crate::constants::*;



pub type Messages = Vec<(String, Color)>;

pub struct Game {
    pub root: Root,
    pub console: Offscreen,
    pub fov: FovMap,
    pub mouse: Mouse,
    pub panel: Offscreen,
}

impl Game {
    pub fn with_root(root: Root) -> Game {
        Game {
            root: root,
            console: Offscreen::new(SCREEN_WIDTH, SCREEN_HEIGHT),
            fov: FovMap::new(MAP_WIDTH, MAP_HEIGHT),
            mouse: Default::default(),
            panel: Offscreen::new(SCREEN_WIDTH, PANEL_HEIGHT),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Tile {
    pub blocked: bool,
    pub block_sight: bool,
    pub explored: bool,
}

impl Tile {
    pub fn empty() -> Self {
        Tile { blocked: false, block_sight: false, explored: false, }
    }

    pub fn wall() -> Self {
        Tile { blocked: true, block_sight: true, explored: false, }
    }
}

