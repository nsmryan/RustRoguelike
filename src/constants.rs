use tcod::map::{FovAlgorithm};


pub const CONFIG_FILE_NAME: &str = &"config.json";

pub const FONT_WIDTH: i32 = 16;
pub const FONT_HEIGHT: i32 = 16;

pub const SCREEN_WIDTH: i32 = 80;
pub const SCREEN_HEIGHT: i32 = 50;
pub const LIMIT_FPS: i32 = 20;

pub const MAP_WIDTH: i32 = 80;
pub const MAP_HEIGHT: i32 = 43;

pub const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;
pub const FOV_LIGHT_WALLS: bool = true;
pub const TORCH_RADIOUS: i32 = 10;
pub const HEAL_AMOUNT: i32 = 4;

pub const ROOM_MAX_SIZE: i32 = 10;
pub const ROOM_MIN_SIZE: i32 = 6;
pub const MAX_ROOMS: i32 = 30;
pub const MAX_ROOM_ITEMS: i32 = 2;

pub const MAX_ROOM_MONSTERS: i32 = 3;

pub const PLAYER: usize = 0;
pub const MAX_MOMENTUM: i32 = 2;

pub const BAR_WIDTH: i32 = 20;
pub const PANEL_HEIGHT: i32 = 7;
pub const PANEL_Y: i32 = SCREEN_HEIGHT - PANEL_HEIGHT;

pub const MSG_X: i32 = BAR_WIDTH + 2;
pub const MSG_WIDTH: i32 = SCREEN_WIDTH - BAR_WIDTH - 2;
pub const MSG_HEIGHT: usize = PANEL_HEIGHT as usize - 1;

pub const INVENTORY_WIDTH: i32 = 50;

pub const ISLAND_RADIUS: i32 = 15;
pub const ISLAND_NUM_OBSTICLES: i32 = 15;
pub const ISLAND_NUM_SUBTRACTIONS_ATTEMPTS: i32 = 50;
pub const ISLAND_NUM_ADDITION_ATTEMPTS: i32 = 50;

