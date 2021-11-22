use std::collections::VecDeque;
use std::collections::HashMap;
use std::collections::HashSet;

use bmp::Image;

use serde::{Serialize, Deserialize};

use sdl2::render::{Texture, WindowCanvas, TextureCreator};
use sdl2::video::WindowContext;
use sdl2::rect::{Rect};
use sdl2::pixels::{PixelFormatEnum};

use roguelike_core::types::*;
use roguelike_core::constants::*;
use roguelike_core::config::*;
use roguelike_core::messaging::*;
use roguelike_core::map::*;
use roguelike_core::utils::aoe_fill;
use roguelike_core::movement::{Direction};
use roguelike_core::rng::Rand32;

use roguelike_engine::game::Game;

use crate::animation::{Str, Sprite, Effect, SpriteKey, Animation, SpriteAnim, SpriteIndex};
use crate::drawcmd::*;


#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PanelName {
    Info,
    Map,
    Player,
    Inventory,
    Menu,
}

impl PanelName {
    pub fn names() -> [PanelName; 5] {
        return [PanelName::Info, PanelName::Map, PanelName::Player, PanelName::Inventory, PanelName::Menu];
    }
}

pub struct Display {
    pub state: DisplayState,

    pub canvas: WindowCanvas,

    pub texture_creator: TextureCreator<WindowContext>,

    pub textures: HashMap<PanelName, Texture>,
    pub panels: HashMap<PanelName, Panel>,

    // sprite state
    pub sprites: Vec<SpriteSheet>,
    pub next_sprite_key: SpriteKey,

    pub intern: HashMap<String, Str>,
    pub next_str: usize,

    // TODO this may not be necessary- the canvas is not like other panels
    // try to just store the area, or the pixel dimensions, instead of a panel.
    pub canvas_panel: Panel,
}

impl Display {
    pub fn new(canvas: WindowCanvas) -> Display {
        let mut texture_creator = canvas.texture_creator();
        let pixel_format = texture_creator.default_pixel_format();

        let canvas_cell_dims = (SCREEN_WIDTH / (FONT_WIDTH as u32 * 2), SCREEN_HEIGHT / (FONT_HEIGHT as u32 * 2));
        let canvas_panel = Panel::new((SCREEN_WIDTH, SCREEN_HEIGHT), canvas_cell_dims);
        let panels = create_panels(canvas_cell_dims.0, canvas_cell_dims.1);

        let mut textures = HashMap::new();

        for panel_name in panels.keys() {
            let texture = create_texture(&mut texture_creator, pixel_format, panels[panel_name].num_pixels);
            textures.insert(*panel_name, texture);
        }

        return Display { state: DisplayState::new(),
                         canvas,
                         texture_creator,
                         textures, 
                         sprites: Vec::new(),
                         next_sprite_key: 0,
                         panels,
                         intern: HashMap::new(),
                         next_str: 0,
                         canvas_panel};
    }

    pub fn add_string(&mut self, string: &str) -> usize {
        if let Some(key) = self.intern.get(string) {
            return *key;
        } else {
            let index = self.next_str;
            self.intern.insert(string.to_string(), index);
            self.next_str += 1;
            return index;
        }
    }

    pub fn process_draw_commands(&mut self) {
        let canvas = &mut self.canvas;

        for panel_name in PanelName::names().iter() {
            let clear = *panel_name != PanelName::Map;
            let panel = self.panels.get_mut(panel_name).unwrap();
            panel.process_cmds(clear,
                               self.textures.get_mut(panel_name).unwrap(),
                               canvas,
                               &mut self.sprites);
        }
    }

    pub fn update_display(&mut self) {
        self.canvas.present();
    }

    pub fn save_screenshot(&mut self, name: &str) {
        let format = PixelFormatEnum::RGB24;
        let (width, height) = self.canvas.output_size().unwrap();

        let pixels = self.canvas.read_pixels(None, format).unwrap();

        let mut shot = Image::new(width, height);

        for x in 0..width {
            for y in 0..height {
                let index = (x + y * width) as usize * 3;
                let pixel = bmp::Pixel::new(pixels[index + 0],
                                            pixels[index + 1],
                                            pixels[index + 2]);
                shot.set_pixel(x, y, pixel);
            }
        }

        shot.save(format!("{}.bmp", name)).unwrap();
    }

    pub fn add_spritesheet(&mut self, name: String, texture: Texture) {
        let sprite_key = self.next_sprite_key;
        let sprite_sheet = SpriteSheet::new(name, texture);
        self.next_sprite_key += 1;
        self.sprites.insert(sprite_key, sprite_sheet);
    }

    pub fn sprite_exists(&self, name: &str) -> bool {
        for sprite_sheet in self.sprites.iter() {
            if sprite_sheet.name == *name {
                return true;
            }
        }

        return false;
    }

    /// Create a sprite by looking up a texture and constructing the
    /// SpriteAnim structure.
    pub fn new_sprite(&mut self, name: &str, speed: f32) -> SpriteAnim {
        let sprite_key = lookup_spritekey(&self.sprites, name);
        let max_index = self.sprites[sprite_key].num_sprites;
        let name_str = self.add_string(name);
        return SpriteAnim::new(name_str, sprite_key, 0.0, max_index as f32, speed);
    }

    pub fn static_sprite(&mut self, sprite_sheet: &str, chr: char) -> SpriteAnim {
        let sprite_key = lookup_spritekey(&self.sprites, sprite_sheet);
        let name_str = self.add_string(&format!("{}", chr));
        return SpriteAnim::new(name_str,
                               sprite_key,
                               chr as i32 as SpriteIndex,
                               chr as i32 as SpriteIndex,
                               0.0);
    }

    pub fn random_sprite(&mut self, sprite_name: &str, speed: f32) -> Animation {
        let sprite_anim = self.new_sprite(sprite_name, speed);
        let anim = Animation::RandomLoop(sprite_anim);
        return anim;
    }

    pub fn between_sprite(&mut self, sprite_name: &str, start: Pos, end: Pos, speed: f32) -> Animation {
        let sprite_anim = self.new_sprite(sprite_name, speed);
        let anim = Animation::Between(sprite_anim, start, end, 0.0, speed);
        return anim;
    }

    /// Create and play a looping sprite
    pub fn loop_sprite(&mut self, sprite_name: &str, speed: f32) -> Animation {
        let sprite_anim = self.new_sprite(sprite_name, speed);
        let anim = Animation::Loop(sprite_anim);
        return anim;
    }

    pub fn play_idle_animation(&mut self, entity_id: EntityId, data: &mut Level, config: &Config) {
        if let Some(anim) = self.get_idle_animation(entity_id, data, config) {
            self.state.play_animation(entity_id, anim);
        }
    }

    pub fn get_idle_animation(&mut self, entity_id: EntityId, data: &mut Level, config: &Config) -> Option<Animation> {
        let name = data.entities.name[&entity_id];

        if name == EntityName::Player || name == EntityName::Gol || name == EntityName::Pawn || name == EntityName::Rook {
            let name = data.entities.name[&entity_id];
            let stance = data.entities.stance[&entity_id];
            let direction = data.entities.direction[&entity_id];

            let sheet_direction = sheet_direction(direction);
            let mut sheet_name = format!("{}_{}_{}", name, stance, sheet_direction);

            if !self.sprite_exists(&sheet_name) {
                sheet_name = format!("{}_{}_{}", name, Stance::Standing, sheet_direction);
            }

            let mut anim = self.loop_sprite(&sheet_name, config.idle_speed);
            anim.sprite_anim_mut().unwrap().flip_horiz = needs_flip_horiz(direction);

            return Some(anim);
        } else {
            if data.entities.name[&entity_id] == EntityName::Key {
                return Some(self.loop_sprite("key", config.idle_speed));
            } else if data.entities.name[&entity_id] == EntityName::SpikeTrap {
                return Some(self.loop_sprite("trap_damage", config.idle_speed));
            } else if data.entities.name[&entity_id] == EntityName::Armil {
                return Some(self.loop_sprite("armil_idle", config.idle_speed));
            } else if data.entities.name[&entity_id] == EntityName::Lantern {
                return Some(self.loop_sprite("lantern_idle", config.fire_speed));
            } else if data.entities.name[&entity_id] == EntityName::Grass {
                return Some(self.random_sprite("grassanim", config.grass_idle_speed));
            }
        }

        return None;
    }

    pub fn clear_level_state(&mut self) {
        self.state.impressions.clear();
        self.state.prev_turn_fov.clear();
        self.state.current_turn_fov.clear();
        self.state.sound_tiles.clear();
        self.state.effects.clear();
    }

    pub fn process_message(&mut self, msg: Msg, data: &mut Level, config: &Config) {
        match msg {
            Msg::StartTurn => {
                self.state.use_pos = None;
                self.state.use_positions.clear();
                self.state.use_dir = None;
                self.state.hit_positions.clear();
                self.state.sound_tiles.clear();
            }

            Msg::CursorState(state, pos) => {
                if !state {
                    let tiles = lookup_spritekey(&self.sprites, "tiles");
                    let cursor_sprite = Sprite::new(ENTITY_CURSOR as u32, tiles);
                    let color = config.color_mint_green;
                    let fade_effect = Effect::fade(cursor_sprite, color, config.cursor_alpha, 0, pos, config.cursor_fade_seconds);
                    self.state.play_effect(fade_effect);
                }
                self.state.time_of_cursor_toggle = self.state.time;
            }

            Msg::Sound(cause_id, source_pos, radius, should_animate) => {
                if should_animate {
                    // NOTE this is a duplicate computation, also done in logic message processing
                    let sound_aoe =
                        aoe_fill(&data.map, AoeEffect::Sound, source_pos, radius, config);

                    // Add to this turn's sound tiles list
                    self.state.sound_tiles.extend(sound_aoe.positions().iter());

                    let player_id = data.find_by_name(EntityName::Player).unwrap();
                    let player_pos = data.entities.pos[&player_id];

                    // only play the sound effect if the player position is included
                    let sound_hits_player = sound_aoe.positions().iter().any(|pos| *pos == player_pos);
                    let sound_from_monster = data.entities.typ.get(&cause_id) == Some(&EntityType::Enemy);
                    let player_can_see_source = data.is_in_fov(player_id, cause_id, config);
                    let visible_monster_sound = sound_from_monster && player_can_see_source;
                    if !visible_monster_sound && sound_hits_player {
                        let sound_effect = Effect::sound(sound_aoe);
                        self.state.play_effect(sound_effect);

                        let pos = data.entities.pos[&cause_id];
                        // NOTE it is slightly odd to look up this sprite sheet here...
                        let tiles = lookup_spritekey(&self.sprites, "tiles");
                        let impression_sprite = Sprite::new(ENTITY_UNKNOWN as u32, tiles);
                        self.state.impressions.push(Impression::new(impression_sprite, pos));
                    }
                }
            }

            Msg::ItemThrow(_thrower, item_id, start, _end) => {
                // this uses the entity's position instead of 'end' because we
                // want where it hit, not where it was thrown to.
                let end = data.entities.pos[&item_id];

                let sound_aoe = aoe_fill(&data.map, AoeEffect::Sound, end, config.sound_radius_stone, config);

                let chr = data.entities.chr[&item_id];
                let item_sprite = self.static_sprite("tiles", chr);

                let move_anim = Animation::Between(item_sprite, start, end, 0.0, config.item_throw_speed);
                let item_anim = Animation::PlayEffect(Effect::Sound(sound_aoe, 0.0));
                let loop_anim = Animation::Loop(item_sprite);

                self.state.play_animation(item_id, move_anim);
                self.state.append_animation(item_id, item_anim);
                self.state.append_animation(item_id, loop_anim);
            }

            Msg::PickedUp(entity_id, _item_id) => {
                self.play_idle_animation(entity_id, data, config);
            }

            Msg::Facing(entity_id, _pos) => {
                self.play_idle_animation(entity_id, data, config);
            }

            Msg::Killed(_attacker, attacked, _damage) => {
                if data.entities.typ[&attacked] != EntityType::Player {
                    self.state.clear_animations(attacked);

                    let sprite_name = format!("{:?}_death", data.entities.name[&attacked]);
                    if self.sprite_exists(&sprite_name) {
                        let sprite = self.new_sprite(&sprite_name, 1.0);
                        self.state.play_animation(attacked, Animation::Once(sprite));
                    }
                }
            }

            Msg::Blunt(from, to) | Msg::Pierce(from, to) | Msg::Slash(from, to) => {
                let sprite_name;
                if from == to || Direction::from_positions(from, to).unwrap().horiz() {
                    if matches!(msg, Msg::Blunt(_, _)) {
                        sprite_name = "player_blunt_cardinal";
                    } else if matches!(msg, Msg::Pierce(_, _)) {
                        sprite_name = "player_pierce_cardinal";
                    } else {
                        sprite_name = "player_slash_cardinal";
                    }
                } else {
                    if matches!(msg, Msg::Blunt(_, _)) {
                        sprite_name = "player_blunt_diagonal";
                    } else if matches!(msg, Msg::Pierce(_, _)) {
                        sprite_name = "player_pierce_diagonal";
                    } else {
                        sprite_name = "player_slash_diagonal";
                    }
                }
                let mut sprite_anim = self.new_sprite(sprite_name, config.attack_animation_speed);
                if let Some(dir) = Direction::from_positions(from, to) {
                    let turns;
                    if dir.horiz() {
                        turns = Direction::Up.turn_amount(dir);
                    } else {
                        turns = Direction::UpRight.turn_amount(dir);
                    }
                    sprite_anim.rotation = turns as f64 * 45.0;
                }

                let blunt_attack = Effect::attack(from, to, sprite_anim);
                self.state.play_effect(blunt_attack);
            }

            Msg::HammerSwing(entity_id, _item_id, _pos) => {
                if data.entities.typ[&entity_id] == EntityType::Player {
                    // TODO need hammer animation
                    //let attack_sprite =
                    //    self.new_sprite("player_attack_hammer".to_string(), config.player_attack_hammer_speed);
                    //let attack_anim = Animation::Once(attack_sprite);
                    //self.state.play_animation(entity_id, attack_anim);

                    //if let Some(idle_anim) = self.get_idle_animation(entity_id, data, config) {
                    //    self.state.append_animation(entity_id, idle_anim);
                    //}
                }
            }

            Msg::Stabbed(entity_id, _hit_entity) => {
                if data.entities.typ[&entity_id] == EntityType::Player {
                    // TODO need dagger animation
                    //let attack_sprite =
                    //    self.new_sprite("player_attack_dagger".to_string(), config.player_attack_speed);
                    //let attack_anim = Animation::Once(attack_sprite);
                    //self.state.play_animation(entity_id, attack_anim);

                    //if let Some(idle_anim) = self.get_idle_animation(entity_id, data, config) {
                    //    self.state.append_animation(entity_id, idle_anim);
                    //}
                }
            }

            Msg::Attack(attacker, attacked, _damage) => {
                if data.entities.typ[&attacker] == EntityType::Player {
                    // TODO need attack animation
                    //let attack_sprite =
                    //    self.new_sprite("player_attack".to_string(), config.player_attack_speed);
                    //let attack_anim = Animation::Once(attack_sprite);
                    //self.state.play_animation(attacker, attack_anim);

                    //if let Some(idle_anim) = self.get_idle_animation(attacker, data, config) {
                    //    self.state.play_animation(attacker, idle_anim);
                    //}
                } else {
                    let attacker_pos = data.entities.pos[&attacker];
                    let attacked_pos = data.entities.pos[&attacked];
                    let beam_effect = Effect::beam(config.beam_duration, attacker_pos, attacked_pos);
                    self.state.play_effect(beam_effect);
                }
            }

            Msg::JumpWall(jumper, start, end) => {
                let jump_anim = self.between_sprite("playerjump_right", start, end, config.idle_speed);
                self.state.play_animation(jumper, jump_anim);
            }

            Msg::SpawnedObject(entity_id, _typ, _pos, _name, _facing) => {
                if data.entities.ids.contains(&entity_id) {
                    self.play_idle_animation(entity_id, data, config);
                }
            }

            Msg::PlayerTurn => {
                let player_id = data.find_by_name(EntityName::Player).unwrap();

                self.state.prev_turn_fov.clear();
                self.state.prev_turn_fov.extend(self.state.current_turn_fov.iter());
                self.state.current_turn_fov.clear();

                for entity_id in data.entities.ids.clone() {
                    if entity_id != player_id && data.is_in_fov(player_id, entity_id, config) {
                        self.state.current_turn_fov.push(entity_id);
                    }
                }

                for entity_id in self.state.prev_turn_fov.iter() {
                    if data.entities.typ.get(entity_id) != Some(&EntityType::Enemy) {
                        continue;
                    }

                    if !data.is_in_fov(player_id, *entity_id, config) {
                        if let Some(sprite) = self.state.drawn_sprites.get(entity_id) {
                            let pos = data.entities.pos[entity_id];
                            self.state.impressions.push(Impression::new(*sprite, pos));
                        }
                    }
                }

                /* Remove impressions that are currently visible */
                let mut impressions_visible = Vec::new();
                for (index, impression) in self.state.impressions.iter().enumerate() {
                    data.entities.status[&player_id].extra_fov += 1;
                    let is_in_fov_ext = 
                       data.pos_in_fov(player_id, impression.pos, &config);
                    data.entities.status[&player_id].extra_fov -= 1;

                    if is_in_fov_ext {
                        impressions_visible.push(index);
                    }
                }
                impressions_visible.sort();
                impressions_visible.reverse();
                for index in impressions_visible.iter() {
                    self.state.impressions.swap_remove(*index);
                }
            }

            Msg::RemovedEntity(entity_id) => {
                self.state.animations.remove(&entity_id);
            }

            Msg::NewLevel => {
                self.clear_level_state();
                self.state.play_effect(Effect::particles(1.0));
            }

            Msg::TileFov(pos, fov_result) => {
                self.state.fov.insert(pos, fov_result);
            }

            Msg::UsePos(pos) => {
                self.state.use_pos = Some(pos);
                self.state.use_positions.insert(pos);
            }

            Msg::UseDir(dir) => {
                self.state.use_dir = Some(dir);
            }

            Msg::UseDirClear => {
                self.state.use_dir = None;
            }

            Msg::UseHitPos(pos) => {
                self.state.hit_positions.insert(pos);
            }

            _ => {
            }
        }
    }

    pub fn draw_all(&mut self, game: &mut Game) {
        self.process_draw_commands();
        self.copy_panels(game);
        self.state.update_animations(&mut game.rng, &game.config);
    }

    pub fn copy_panels(&mut self, game: &mut Game) {
        /* Split Screen Into Sections */
        let screen_area = self.canvas_panel.area();
        let (_map_area, _rest_area) = screen_area.split_top(self.panels[&PanelName::Map].cells.1 as usize);

        let map_cell_dims = self.panels[&PanelName::Map].cell_dims();
        let (map_width, map_height) = (map_cell_dims.0 * game.data.map.width() as u32, map_cell_dims.1 * game.data.map.height() as u32);
        let map_src = Rect::new(0, 0, map_width, map_height);
        let map_rect = Rect::new(0, 0, SCREEN_WIDTH, SCREEN_WIDTH);
        self.canvas.copy(&self.textures[&PanelName::Map], map_src, map_rect).unwrap();

        /* Draw Inventory Panel */
        let inventory_rect = Rect::new(0, SCREEN_WIDTH as i32, SCREEN_WIDTH / 3, SCREEN_HEIGHT - SCREEN_WIDTH);
        self.canvas.copy(&self.textures[&PanelName::Inventory], None, inventory_rect).unwrap();

        /* Draw Game Info Panel */
        let info_rect = Rect::new(SCREEN_WIDTH as i32 / 3, SCREEN_WIDTH as i32, SCREEN_WIDTH / 3, SCREEN_HEIGHT - SCREEN_WIDTH);
        self.canvas.copy(&self.textures[&PanelName::Info], None, info_rect).unwrap();

        /* Draw Player Info Panel */
        let player_rect = Rect::new(2 * SCREEN_WIDTH as i32 / 3, SCREEN_WIDTH as i32, SCREEN_WIDTH / 3, SCREEN_HEIGHT - SCREEN_WIDTH);
        self.canvas.copy(&self.textures[&PanelName::Player], None, player_rect).unwrap();

        if game.settings.state.is_menu() {
            let canvas_panel = &mut self.canvas_panel;
            let menu_panel = &self.panels[&PanelName::Menu];

            //let menu_area = self.panels[&PanelName::Menu].area();
            //let menu_area = map_area.centered(menu_area.width, menu_area.height);

            let menu_area = screen_area.centered(menu_panel.cells.0 as usize, menu_panel.cells.1 as usize);
            let dst = canvas_panel.get_rect_within(&menu_area, menu_panel.cells);
            self.canvas.copy(&self.textures[&PanelName::Menu], None, dst).unwrap();
        }
    }
}

pub type Panels = HashMap<PanelName, Panel>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisplayState {
    // currently active effects
    pub effects: Vec<Effect>,

    // animation information
    pub animations: Comp<VecDeque<Animation>>,
    pub next_anim_key: i64,

    // sprites drawn this frame
    pub drawn_sprites: Comp<Sprite>,

    // impressions left on map
    pub impressions: Vec<Impression>,

    // FOV information used when drawing
    pub prev_turn_fov: Vec<EntityId>,
    pub current_turn_fov: Vec<EntityId>,

    // tiles that heard a sound
    pub sound_tiles: Vec<Pos>,

    // fov information for this turn
    pub fov: HashMap<Pos, FovResult>,
    pub use_pos: Option<Pos>,
    pub use_positions: HashSet<Pos>,
    pub use_dir: Option<Direction>,
    pub hit_positions: HashSet<Pos>,

    pub dt: f32,
    pub time: f32,
    pub time_of_cursor_toggle: f32,
    pub last_cursor_pos: Pos,

    pub debug_entries: HashMap<String, String>,
}

impl DisplayState {
    pub fn new() -> DisplayState {
        return DisplayState {
            effects: Vec::new(),
            animations: Comp::<VecDeque<Animation>>::new(),
            next_anim_key: 0,
            drawn_sprites: Comp::new(),
            impressions: Vec::new(),
            prev_turn_fov: Vec::new(),
            current_turn_fov: Vec::new(),
            sound_tiles: Vec::new(),
            fov: HashMap::new(),
            use_pos: None,
            use_positions: HashSet::new(),
            use_dir: None,
            hit_positions: HashSet::new(),
            dt: 0.0,
            time: 0.0,
            time_of_cursor_toggle: 0.0,
            last_cursor_pos: Pos::new(0, 0),
            debug_entries: HashMap::<String, String>::new(),
        };
    }

    pub fn update_animations(&mut self, rng: &mut Rand32, config: &Config) {
        for anims in self.animations.store.iter_mut() {
            if let Some(anim) = anims.get_mut(0) {
                anim.step(self.dt, rng, config);
            }
        }
    }

    pub fn play_effect(&mut self, effect: Effect) {
        self.effects.push(effect);
    }

    pub fn play_animation(&mut self, entity_id: EntityId, animation: Animation) {
        // ensure that the entity has an animation vec. This is called on entity
        // spawn, so it is not necessary in the other animation functions.
        if self.animations.get(&entity_id).is_none() {
            self.animations.insert(entity_id, VecDeque::new());
        }

        self.clear_animations(entity_id);
        self.append_animation(entity_id, animation);
    }

    pub fn append_animation(&mut self, entity_id: EntityId, animation: Animation) {
        self.animations[&entity_id].push_back(animation);
    }

    pub fn clear_animations(&mut self, entity_id: EntityId) {
        self.animations[&entity_id].clear();
    }

    pub fn pop_animation(&mut self, entity_id: EntityId) {
        self.animations[&entity_id].pop_front();
    }

    pub fn show_debug(&mut self, name: &str, value: String) {
        self.debug_entries.insert(name.to_string(), value);
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Impression {
    pub sprite: Sprite,
    pub pos: Pos,
}

impl Impression {
    pub fn new(sprite: Sprite, pos: Pos) -> Impression {
        return Impression { sprite, pos };
    }
}


fn sheet_direction(direction: Direction) -> Direction {
    match direction {
        Direction::Up => return Direction::Up,
        Direction::Down => return Direction::Down,
        Direction::Left => return Direction::Right,
        Direction::Right => return Direction::Right,
        Direction::UpRight => return Direction::UpRight,
        Direction::UpLeft => return Direction::UpRight,
        Direction::DownRight => return Direction::DownRight,
        Direction::DownLeft => return Direction::DownRight,
    }
}

fn needs_flip_horiz(direction: Direction) -> bool {
    match direction {
        Direction::Up => return false,
        Direction::Down => return false,
        Direction::Left => return true,
        Direction::Right => return false,
        Direction::UpRight => return false,
        Direction::UpLeft => return true,
        Direction::DownRight => return false,
        Direction::DownLeft => return true,
    }
}

fn create_texture(texture_creator: &mut TextureCreator<WindowContext>, pixel_format: PixelFormatEnum, num_pixels: (u32, u32)) -> Texture {
    return texture_creator.create_texture_target(pixel_format, num_pixels.0, num_pixels.1).unwrap();
}

fn create_panels(width: u32, height: u32) -> HashMap<PanelName, Panel> {
    let mut panels = HashMap::new();

    let over_sample = 5;

    let map_pixels = (over_sample * MAP_WIDTH as u32 * FONT_WIDTH as u32, over_sample * MAP_HEIGHT as u32 * FONT_HEIGHT as u32);
    let map_dims = (MAP_WIDTH as u32, MAP_HEIGHT as u32);
    let map_panel = Panel::new(map_pixels, map_dims);
    panels.insert(PanelName::Map, map_panel);

    let info_width = width / 2;

    let over_sample = 10;

    let info_dims = (info_width, height - MAP_HEIGHT as u32);
    let info_pixels = (over_sample * info_dims.0 * FONT_WIDTH as u32, over_sample * info_dims.1 * FONT_HEIGHT as u32);
    panels.insert(PanelName::Info, Panel::new(info_pixels, info_dims));

    let inventory_dims = (info_width, height - MAP_HEIGHT as u32);
    let inventory_pixels = (over_sample * inventory_dims.0 * FONT_WIDTH as u32, over_sample * inventory_dims.1 * FONT_HEIGHT as u32);
    panels.insert(PanelName::Inventory, Panel::new(inventory_pixels, inventory_dims));

    let player_dims = (info_width, height - MAP_HEIGHT as u32);
    let player_pixels = (over_sample * player_dims.0 * FONT_WIDTH as u32, over_sample * player_dims.1 * FONT_HEIGHT as u32);
    panels.insert(PanelName::Player, Panel::new(player_pixels, player_dims));

    let menu_dims = (info_width, info_width);
    let menu_pixels = (over_sample * menu_dims.0 * FONT_WIDTH as u32, over_sample * menu_dims.1 * FONT_HEIGHT as u32);
    panels.insert(PanelName::Menu, Panel::new(menu_pixels, menu_dims));

    return panels;
}

