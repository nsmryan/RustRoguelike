pub const CONFIG_FILE_NAME: &str = &"config.json";

/* Screen and font */
pub const FONT_WIDTH: i32 = 16;
pub const FONT_HEIGHT: i32 = 16;

pub const SCREEN_WIDTH: u32 = 1000;
pub const SCREEN_HEIGHT: u32 = 800;

pub const INVENTORY_WIDTH: usize = 300;
pub const INVENTORY_HEIGHT: usize = 500;

/* General Settings */
pub const INITIAL_OBJECT_CAPACITY: usize = 1000;
pub const BLINK_RADIUS: usize = 4;

/* Monster Settings */
pub const MAX_MONSTERS: i32 = 15;

pub const GOL_ATTACK_DISTANCE: usize = 5;
pub const GOL_MOVE_DISTANCE: usize = 1;

pub const SPIRE_ATTACK_DISTANCE: usize = 5;
pub const SPIRE_MOVE_DISTANCE: usize = 1;

pub const PAWN_MOVE_DISTANCE: usize = 1;
pub const PAWN_ATTACK_DISTANCE: usize = 1;

pub const KEY_ATTACK_DISTANCE: usize = 3;
pub const KEY_MOVE_DISTANCE: usize = 2;

pub const MAX_MOMENTUM: i32 = 2;

pub const SPIKE_DAMAGE: i32 = 20;
pub const HAMMER_DAMAGE: i32 = 40;

pub const NOT_IN_FOV_COST: usize = 5;

/* Player */
pub const PLAYER_THROW_DIST: usize = 5;

/* UI */
pub const BAR_WIDTH: u32 = 20;
pub const PANEL_HEIGHT: u32 = 7;
pub const PANEL_Y: u32 = SCREEN_HEIGHT - PANEL_HEIGHT;

pub const MSG_X: u32 = BAR_WIDTH + 2;
pub const MSG_WIDTH: u32 = SCREEN_WIDTH - BAR_WIDTH - 2;
pub const MSG_HEIGHT: u32 = PANEL_HEIGHT - 1;

/* Map and View */
pub const MAP_WIDTH: i32 = 30;
pub const MAP_HEIGHT: i32 = 30;

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
pub const MAP_EMPTY_CHAR: u8 = 219; // all white square

pub const MAP_STATUE_1: u8 = 148;
pub const MAP_STATUE_2: u8 = 149;
pub const MAP_STATUE_3: u8 = 140;
pub const MAP_STATUE_4: u8 = 133;
pub const MAP_STATUE_5: u8 = 132;
pub const MAP_STATUE_6: u8 = 151;

pub const MAP_COLUMN: u8 = 42;
pub const MAP_RUBBLE: u8 = 15;
pub const MAP_GRASS: u8 = 234;

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

pub const MAP_SMALL_DOT_MIDDLE: u8 = 250;
pub const MAP_STAR: u8 = 15;
pub const MAP_ORB: u8 = 135;
pub const MAP_WALL: u8 = 150;

pub const MAP_ROOK: u8 = 130;

pub const ENTITY_GOL: u8 = 152;
pub const ENTITY_POLAXE: u8 = 0x8C;
pub const ENTITY_CASTLE: u8 = 0x85;
pub const ENTITY_PLAYER: u8 = 139;
pub const ENTITY_EXIT: u8 = 135;
pub const ENTITY_CLOAK_GUY: u8 = 141;
pub const ENTITY_ELF: u8 = 165;
pub const ENTITY_ORB: u8 = 135;
pub const ENTITY_KEY: u8 = 235;
pub const ENTITY_STONE: u8 = 244;
pub const ENTITY_DAGGER: u8 = 247;
pub const ENTITY_SWORD: u8 = 248;
pub const ENTITY_SHIELD: u8 = 255;
pub const ENTITY_HAMMER: u8 = 251;
pub const ENTITY_TRAP_SOUND: u8 = 83; // char S
pub const ENTITY_SPIKE_TRAP: u8 = 136;
pub const ENTITY_BLINK_TRAP: u8 = 6;

pub const ARROW_RIGHT: u8 = 16;
pub const ARROW_LEFT: u8 = 17;
pub const ARROW_UP: u8 = 30;
pub const ARROW_DOWN: u8 = 31;

