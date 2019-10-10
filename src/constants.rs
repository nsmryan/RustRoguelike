use tcod::map::{FovAlgorithm};


pub const CONFIG_FILE_NAME: &str = &"config.json";

pub const FONT_WIDTH: i32 = 16;
pub const FONT_HEIGHT: i32 = 16;

pub const SCREEN_WIDTH: i32 = 80;
pub const SCREEN_HEIGHT: i32 = 50;
pub const LIMIT_FPS: i32 = 20;

pub const TIME_BETWEEN_FRAMES_MS: i64 = 25;

pub const MAP_WIDTH: i32 = 80;
pub const MAP_HEIGHT: i32 = 60;

pub const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;
pub const FOV_LIGHT_WALLS: bool = true;
pub const TORCH_RADIOUS: i32 = 10;
pub const HEAL_AMOUNT: i32 = 4;

pub const ROOM_MAX_SIZE: i32 = 10;
pub const ROOM_MIN_SIZE: i32 = 6;
pub const MAX_ROOMS: i32 = 30;
pub const MAX_ROOM_ITEMS: i32 = 12;

pub const MAX_MONSTERS: i32 = 15;

pub const MONSTER_VIEW_DIST: i32 = 6;


pub const PLAYER: usize = 0;
pub const MAX_MOMENTUM: i32 = 2;

pub const PLAYER_THROW_DIST: usize = 5;

pub const BAR_WIDTH: i32 = 20;
pub const PANEL_HEIGHT: i32 = 7;
pub const PANEL_Y: i32 = SCREEN_HEIGHT - PANEL_HEIGHT;

pub const MSG_X: i32 = BAR_WIDTH + 2;
pub const MSG_WIDTH: i32 = SCREEN_WIDTH - BAR_WIDTH - 2;
pub const MSG_HEIGHT: usize = PANEL_HEIGHT as usize - 1;

pub const INVENTORY_WIDTH: i32 = 50;

pub const STONE_SOUND_RADIUS: i32 = 5;

pub const ISLAND_RADIUS: i32 = 15;
pub const ISLAND_NUM_OBSTACLES: i32 = 15;
pub const ISLAND_NUM_SUBTRACTIONS_ATTEMPTS: i32 = 50;
pub const ISLAND_NUM_ADDITION_ATTEMPTS: i32 = 50;

pub const MAP_LAYER_GROUND: usize = 0;
pub const MAP_LAYER_ENVIRONMENT: usize = 1;
pub const MAP_LAYER_ENTITIES: usize = 2;

pub const MAP_GROUND: u8 = 154;
pub const MAP_WATER: u8 = 224;
pub const MAP_EMPTY: u8 = 32;

pub const MAP_STATUE_1: u8 = 148;
pub const MAP_STATUE_2: u8 = 149;
pub const MAP_STATUE_3: u8 = 140;
pub const MAP_STATUE_4: u8 = 133;
pub const MAP_STATUE_5: u8 = 132;
pub const MAP_STATUE_6: u8 = 151;

pub const MAP_TALL_SPIKES: u8 = 136;
pub const MAP_WIDE_SPIKES: u8 = 137;

pub const MAP_THIN_WALL_TOP: u8 = 0x9c;
pub const MAP_THIN_WALL_BOTTOM: u8 = 0x9D;
pub const MAP_THIN_WALL_LEFT: u8 = 0x9E;
pub const MAP_THIN_WALL_RIGHT: u8 = 0x9F;

pub const MAP_THIN_WALL_TOP_LEFT: u8 = 24;
pub const MAP_THIN_WALL_BOTTOM_LEFT: u8 = 25;
pub const MAP_THIN_WALL_TOP_RIGHT: u8 = 26;
pub const MAP_THIN_WALL_BOTTOM_RIGHT: u8 = 27;

pub const MAP_THICK_WALL_RIGHT: u8 = 175;
pub const MAP_THICK_WALL_LEFT: u8 = 174;
pub const MAP_THICK_WALL_TOP: u8 = 172;
pub const MAP_THICK_WALL_BOTTOM: u8 =  173;

pub const MAP_THICK_WALL_TOP_LEFT: u8 = 240;
pub const MAP_THICK_WALL_BOTTOM_LEFT: u8 = 241;
pub const MAP_THICK_WALL_TOP_RIGHT: u8 = 242;
pub const MAP_THICK_WALL_BOTTOM_RIGHT: u8 = 243;

pub const MAP_DOT_TOP_LEFT: u8 = 145;
pub const MAP_DOT_TOP_RIGHT: u8 = 146;
pub const MAP_DOT_BOTTOM_LEFT: u8 = 161;
pub const MAP_DOT_BOTTOM_RIGHT: u8 = 162;

pub const MAP_DOT_MIDDLE: u8 = 249;
pub const MAP_SMALL_DOT_MIDDLE: u8 = 250;
pub const MAP_STAR: u8 = 15;
pub const MAP_ORB: u8 = 135;

pub const MAP_ROOK: u8 = 130;

pub const ENTITY_ORC: u8 = 152;
pub const ENTITY_POLAXE: u8 = 0x8C;
pub const ENTITY_CASTLE: u8 = 0x85;
pub const ENTITY_PLAYER: u8 = 0x40;
pub const ENTITY_SWIRL_CIRCLE: u8 = 0x15;
