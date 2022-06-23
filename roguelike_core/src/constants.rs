pub const CONFIG_FILE_NAME: &str = &"config.json";

/* Screen */
pub const UI_CELLS_TOP: u32 = 3;
pub const UI_CELLS_BOTTOM: u32 = 12;

pub const CELL_MULTIPLIER: u32 = 3;

pub const FONT_LENGTH: u32 = 16;

pub const MAP_CELLS_TO_PIXELS: u32 = CELL_MULTIPLIER * FONT_LENGTH;
pub const UI_CELLS_TO_PIXELS: u32 = FONT_LENGTH;

pub const SCREEN_WIDTH: u32 = MAP_WIDTH as u32 * MAP_CELLS_TO_PIXELS;
pub const SCREEN_HEIGHT: u32 = (MAP_HEIGHT as u32 * MAP_CELLS_TO_PIXELS) + (UI_CELLS_TOP + UI_CELLS_BOTTOM) * UI_CELLS_TO_PIXELS;

pub const UI_PIXELS_TOP: u32 = UI_CELLS_TOP * UI_CELLS_TO_PIXELS;
pub const UI_PIXELS_BOTTOM: u32 = UI_CELLS_BOTTOM * UI_CELLS_TO_PIXELS;

/* General Settings */
pub const BLINK_RADIUS: usize = 4;
pub const TILE_FILL_METRIC_DIST: usize = 3;
pub const ILLUMINATE_FOV_RADIUS: i32 = 1000;
pub const STAB_STUN_TURNS: usize = 3;
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
pub const ROOK_ATTACK_DISTANCE: usize = 5;

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
pub const SKILL_ROLL_AMOUNT: usize = 2;
pub const SKILL_STONE_SKIN_TURNS: usize = 4;
pub const SKILL_SWIFT_DISTANCE: usize = 4;
pub const SKILL_QUICK_REFLEXES_PERCENT: f32 = 0.5;

/* Item Settings */
pub const LANTERN_ILLUMINATE_RADIUS: usize = 2;
pub const ILLUMINATE_AMOUNT: usize = LANTERN_ILLUMINATE_RADIUS;
pub const SEED_CACHE_RADIUS: usize = 1;
pub const SMOKE_BOMB_RADIUS: usize = 1;
pub const GLASS_EYE_RADIUS: i32 = 4;
pub const SOUND_RADIUS_THUMPER: usize = 3;

/* Trap Settings */
pub const FREEZE_TRAP_NUM_TURNS: usize = 5;

/* Player */
pub const PLAYER_THROW_DIST: usize = 4;
pub const SLING_THROW_DIST: usize = 7;

pub const PLAYER_MAX_ITEMS: usize = 3;

/* Map and View */
pub const MAP_WIDTH: i32 = 22;
pub const MAP_HEIGHT: i32 = 15;

pub const ISLAND_DISTANCE: i32 = 7;

pub const ISLAND_RADIUS: i32 = 15;
pub const ISLAND_NUM_OBSTACLES: i32 = 15;
pub const ISLAND_NUM_SUBTRACTIONS_ATTEMPTS: i32 = 50;
pub const ISLAND_NUM_ADDITION_ATTEMPTS: i32 = 50;

pub const MAP_LAYER_GROUND: usize = 0;
pub const MAP_LAYER_ENVIRONMENT: usize = 1;
pub const MAP_LAYER_ENTITIES: usize = 2;

pub const MAP_GROUND: u8 = 154;
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
pub const ENTITY_SLING: u8 = 143;
pub const ENTITY_GLASS_EYE: u8 = 127;
pub const ENTITY_SEED_CACHE: u8 = 109;
pub const ENTITY_HERB: u8 = 111;
pub const ENTITY_SEED_OF_STONE: u8 = 122;
pub const ENTITY_TELEPORTER: u8 = 110;
pub const ENTITY_ENERGY_BALL: u8 = 123;

/* Entities */
pub const ENTITY_GOL: u8 = 152;
pub const ENTITY_PLAYER: u8 = 139;
pub const ENTITY_CLOAK_GUY: u8 = 141;
pub const ENTITY_ELF: u8 = 165;
pub const ENTITY_UNKNOWN: u8 = 239;
pub const ENTITY_ENERGY: u8 = 6;
pub const ENTITY_SMOKE: u8 = 124;
pub const ENTITY_KHOPESH: u8 = 123;
pub const ENTITY_MAGNIFIER: u8 = 127;

/* Map Objects */
pub const ENTITY_EXIT: u8 = 135;
pub const ENTITY_TALL_GRASS: u8 = 223;
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

pub const GRASS_INTERTILE_UP: u8 = 105;
pub const GRASS_INTERTILE_DOWN: u8 = 106;
pub const GRASS_INTERTILE_LEFT: u8 = 107;
pub const GRASS_INTERTILE_RIGHT: u8 = 108;

/* Triggers */
pub const ENTITY_GATE_TRIGGER: u8 = ENTITY_TRIGGER_2;

/* Traps */
pub const ENTITY_SPIKE_TRAP: u8 = 119;
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
pub const AT_SYMBOL: u8 = 64;
pub const ARMED_SYMBOL: u8 = 50; // NOTE armed is just a sprite that looks related to explosions

/* Golems */
pub const AI_EXPLODE_RADIUS: usize = 2;
pub const ARMIL_TURNS_ARMED: usize = 1;
