pub const CONFIG_FILE_NAME: &str = &"config.json";

/* Screen and font */
pub const FONT_WIDTH: i32 = 16;
pub const FONT_HEIGHT: i32 = 16;

pub const SCREEN_WIDTH: u32 = 800;
pub const SCREEN_HEIGHT: u32 = 960;

pub const INVENTORY_WIDTH: usize = 300;
pub const INVENTORY_HEIGHT: usize = 500;

pub const ASCII_START: u32 = 32;
pub const ASCII_END: u32 = 127;

/* General Settings */
pub const INITIAL_OBJECT_CAPACITY: usize = 1000;
pub const BLINK_RADIUS: usize = 4;
pub const TILE_FILL_METRIC_DIST: usize = 3;
pub const ILLUMINATE_FOV_RADIUS: i32 = 1000;
pub const STAB_STUN_TURNS: usize = 3;
pub const SPEAR_STUN_TURNS: usize = 3;
pub const STONE_STUN_TURNS: usize = 1;
pub const DEFAULT_ITEM_STUN_TURNS: usize = 1;
pub const ITEM_DURABILITY: usize = 5;

pub const GOLEM_ATTACK_DIAG: u8 = 180;
pub const GOLEM_ATTACK_HORIZ: u8 = 181;

/* Monster Settings */
pub const MAX_MONSTERS: i32 = 15;

pub const GOL_ATTACK_DISTANCE: usize = 5;
pub const GOL_MOVE_DISTANCE: usize = 1;

pub const SPIRE_ATTACK_DISTANCE: usize = 5;
pub const SPIRE_MOVE_DISTANCE: usize = 1;

pub const PAWN_MOVE_DISTANCE: usize = 1;
pub const PAWN_ATTACK_DISTANCE: usize = 1;

pub const ROOK_MOVE_DISTANCE: usize = 1;
pub const ROOK_ATTACK_DISTANCE: usize = 1;

pub const ARMIL_MOVE_DISTANCE: usize = 1;

pub const KEY_ATTACK_DISTANCE: usize = 3;
pub const KEY_MOVE_DISTANCE: usize = 2;

pub const MAX_MOMENTUM: i32 = 2;

pub const SPIKE_DAMAGE: i32 = 20;
pub const HAMMER_DAMAGE: i32 = 40;
pub const SWORD_DAMAGE: i32 = 20;

pub const TRIGGER_WALL_DAMAGE: i32 = 20;

pub const NOT_IN_FOV_COST: usize = 5;

/* Skill Settings */
pub const SKILL_GRASS_SHOES_TURNS: usize = 6;
pub const SKILL_GRASS_THROW_RADIUS: usize = 4;
pub const SKILL_ILLUMINATE_RADIUS: usize = LANTERN_ILLUMINATE_RADIUS;
pub const SKILL_HEAL_AMOUNT: usize = LANTERN_ILLUMINATE_RADIUS;
pub const SKILL_FARSIGHT_FOV_AMOUNT: usize = 1;
pub const SKILL_PUSH_STUN_TURNS: usize = 3;
pub const SKILL_SPRINT_AMOUNT: usize = 3;
pub const SKILL_STONE_SKIN_TURNS: usize = 4;
pub const SKILL_SWIFT_DISTANCE: usize = 4;
pub const SKILL_QUICK_REFLEXES_PERCENT: f32 = 0.5;

/* Item Settings */
pub const LANTERN_ILLUMINATE_RADIUS: usize = 2;
pub const ILLUMINATE_AMOUNT: usize = LANTERN_ILLUMINATE_RADIUS;

/* Trap Settings */
pub const FREEZE_TRAP_NUM_TURNS: usize = 5;

/* Player */
pub const PLAYER_THROW_DIST: usize = 5;

pub const PLAYER_MAX_ITEMS: usize = 3;

/* UI */
pub const BAR_WIDTH: u32 = 20;
pub const PANEL_HEIGHT: u32 = 7;
pub const PANEL_Y: u32 = SCREEN_HEIGHT - PANEL_HEIGHT;

pub const MSG_X: u32 = BAR_WIDTH + 2;
pub const MSG_WIDTH: u32 = SCREEN_WIDTH - BAR_WIDTH - 2;
pub const MSG_HEIGHT: u32 = PANEL_HEIGHT - 1;

/* Map and View */
pub const MAP_WIDTH: i32 = 20;
pub const MAP_HEIGHT: i32 = 20;

pub const ISLAND_DISTANCE: i32 = 7;

pub const ISLAND_RADIUS: i32 = 15;
pub const ISLAND_NUM_OBSTACLES: i32 = 15;
pub const ISLAND_NUM_SUBTRACTIONS_ATTEMPTS: i32 = 50;
pub const ISLAND_NUM_ADDITION_ATTEMPTS: i32 = 50;

pub const MAP_LAYER_GROUND: usize = 0;
pub const MAP_LAYER_ENVIRONMENT: usize = 1;
pub const MAP_LAYER_ENTITIES: usize = 2;

pub const MAP_GROUND: u8 = 154;
pub const MAP_WATER_XP: u8 = 171;
pub const MAP_WATER: u8 = 171;
pub const MAP_EMPTY: u8 = 32;
pub const MAP_EMPTY_CHAR: u8 = 219; // all white square

pub const MAP_STATUE_1: u8 = 148;
pub const MAP_STATUE_2: u8 = 149;
pub const MAP_STATUE_3: u8 = 140;
pub const MAP_STATUE_4: u8 = 133;
pub const MAP_STATUE_5: u8 = 132;
pub const MAP_STATUE_6: u8 = 151;

pub const MAP_COLUMN: u8 = 42;
pub const MAP_RUBBLE: u8 = 233;
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
pub const MAP_FULL: u8 = 13;
pub const MAP_STAR: u8 = 15;
pub const MAP_ORB: u8 = 135;
pub const MAP_WALL: u8 = 150;

pub const MAP_ROOK: u8 = 130;

/* Items */
pub const ENTITY_POLAXE: u8 = 0x8C;
pub const ENTITY_CASTLE: u8 = 0x85;
pub const ENTITY_ORB: u8 = 135;
pub const ENTITY_KEY: u8 = 235;
pub const ENTITY_STONE: u8 = 244;
pub const ENTITY_DAGGER: u8 = 247;
pub const ENTITY_SWORD: u8 = 248;
pub const ENTITY_SHIELD: u8 = 255;
pub const ENTITY_HAMMER: u8 = 251;
pub const ENTITY_SPEAR: u8 = 250;
pub const ENTITY_GREATSWORD: u8 = 254;
pub const ENTITY_LANTERN: u8 = 134;

/* Entities */
pub const ENTITY_GOL: u8 = 152;
pub const ENTITY_PLAYER: u8 = 139;
pub const ENTITY_CLOAK_GUY: u8 = 141;
pub const ENTITY_ELF: u8 = 165;
pub const ENTITY_UNKNOWN: u8 = 239;
pub const ENTITY_ENERGY: u8 = 6;

/* Map Objects */
pub const ENTITY_EXIT: u8 = 135;
pub const ENTITY_TRIGGER_1: u8 = 224;
pub const ENTITY_TRIGGER_2: u8 = 225;
pub const ENTITY_TRIGGER_3: u8 = 226;
pub const ENTITY_TRIGGER_4: u8 = 227;
pub const ENTITY_TRIGGER_5: u8 = 228;
pub const ENTITY_CURSOR: u8 = 231;
pub const ENTITY_UNKNOWN_GOLEM_MARKER: u8 = 239;

pub const SHADOW_FULLTILE_LEFT: u8 = 2;
pub const SHADOW_FULLTILE_LEFT_DOWN: u8 = 6;
pub const SHADOW_FULLTILE_DOWN: u8 = 1;
pub const SHADOW_FULLTILE_DOWN_LEFT: u8 = 0;

pub const SHADOW_INTERTILE_LEFT: u8 = 3;
pub const SHADOW_INTERTILE_LEFT_DOWN: u8 = 7;
pub const SHADOW_INTERTILE_DOWN: u8 = 5;
pub const SHADOW_INTERTILE_DOWN_LEFT: u8 = 4;

/* Triggers */
pub const ENTITY_GATE_TRIGGER: u8 = ENTITY_TRIGGER_2;

/* Traps */
pub const ENTITY_SPIKE_TRAP: u8 = 136;
pub const ENTITY_BLINK_TRAP: u8 = ENTITY_TRIGGER_1;
pub const ENTITY_TRAP_SOUND: u8 = ENTITY_TRIGGER_5;
pub const ENTITY_FREEZE_TRAP: u8 = ENTITY_TRIGGER_4;

/* Arrows */
pub const ARROW_HORIZ: u8 = 16;
pub const ARROW_DIAG: u8 = 17;

/* Characters */
pub const EXCLAMATION_POINT: u8 = 33 as u8;
pub const QUESTION_MARK: u8 = 63 as u8;
pub const ASTERISK: u8 = 239; // NOTE there is no asterisk currently, so this is a marker

