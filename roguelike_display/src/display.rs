use std::collections::VecDeque;
use std::collections::HashMap;
use std::collections::HashSet;
use std::str::FromStr;
use std::io::BufRead;

use bmp::Image;

use serde::{Serialize, Deserialize};

use sdl2::render::{Texture, WindowCanvas, TextureCreator};
use sdl2::video::WindowContext;
use sdl2::rect::{Rect};
use sdl2::pixels::{PixelFormatEnum};
use sdl2::image::LoadTexture;

use roguelike_utils::math::*;
use roguelike_utils::rng::Rand32;
use roguelike_utils::comp::*;

use roguelike_map::*;

use roguelike_core::types::*;
use roguelike_core::constants::*;
use roguelike_core::ai::*;
use roguelike_core::config::*;
use roguelike_core::messaging::*;
use roguelike_core::utils::aoe_fill;
use roguelike_core::movement::{MoveMode};

use roguelike_draw::animation::{Str, Sprite, Effect, Animation, SpriteAnim, SpriteIndex};
use roguelike_draw::drawcmd::*;


#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PanelName {
    Info,
    Map,
    Player,
    Inventory,
    Menu,
    Pip,
    Help,
}

impl PanelName {
    pub fn names() -> [PanelName; 7] {
        return [PanelName::Info, PanelName::Map, PanelName::Player, PanelName::Inventory, PanelName::Menu, PanelName::Pip, PanelName::Help];
    }
}

pub struct Display {
    pub state: DisplayState,

    pub canvas: WindowCanvas,

    pub texture_creator: TextureCreator<WindowContext>,

    pub textures: HashMap<PanelName, Texture>,
    pub panels: HashMap<PanelName, Panel>,
    pub screen_areas: HashMap<PanelName, Area>,

    pub sprites: Vec<SpriteSheet>,
    pub screen_texture: Texture,
    pub atlas_texture: Texture,
    pub font: SpriteSheet,
    pub font_texture: Texture,

    pub intern: HashMap<String, Str>,
    pub next_str: usize,

    pub canvas_panel: Panel,

    pub rng: Rand32,
}

impl Display {
    pub fn new(canvas: WindowCanvas) -> Display {
        let mut texture_creator = canvas.texture_creator();
        let pixel_format = texture_creator.default_pixel_format();

        let canvas_cell_dims = (SCREEN_WIDTH / (FONT_WIDTH as u32 * 1), SCREEN_HEIGHT / (FONT_HEIGHT as u32 * 1));
        let canvas_panel = Panel::new((SCREEN_WIDTH, SCREEN_HEIGHT), canvas_cell_dims);
        
        /* Lay out screen areas */
        let screen_area = canvas_panel.area();
        let (top_area, rest_area) = screen_area.split_top(canvas_panel.cells.0 as usize - 2);
        let (pip_area, map_area) = top_area.split_top(PIP_HEIGHT as usize);
        let (player_area, rest_area) = rest_area.split_left(canvas_panel.cells.0 as usize / 6);
        let (inventory_area, rest_area) = rest_area.split_left(canvas_panel.cells.0 as usize / 2);
        let info_area = rest_area;
        let menu_area = screen_area.centered((info_area.width as f32 * 1.5) as usize, (info_area.height as f32 * 1.5) as usize);
        let help_area = screen_area.centered((screen_area.width as f32 * 0.8) as usize, (screen_area.height as f32 * 0.9) as usize);

        let mut screen_areas = HashMap::new();
        screen_areas.insert(PanelName::Map, map_area);
        screen_areas.insert(PanelName::Pip, pip_area);
        screen_areas.insert(PanelName::Info, info_area);
        screen_areas.insert(PanelName::Player, player_area);
        screen_areas.insert(PanelName::Inventory, inventory_area);
        screen_areas.insert(PanelName::Menu, menu_area);
        screen_areas.insert(PanelName::Help, help_area);

        let panels = create_panels(&screen_areas);

        let mut textures = HashMap::new();

        for panel_name in panels.keys() {
            let texture = create_texture(&mut texture_creator, pixel_format, panels[panel_name].num_pixels);
            textures.insert(*panel_name, texture);
        }

        let screen_texture = create_texture(&mut texture_creator, pixel_format, (SCREEN_WIDTH, SCREEN_HEIGHT));

        let sprites = parse_atlas_file("resources/spriteAtlas.txt");
        let atlas_texture = texture_creator.load_texture("resources/spriteAtlas.png").expect("Could not load sprite atlas!");

        let ttf_context = sdl2::ttf::init().expect("Could not init SDL2 TTF!");
        let font_name = "Inconsolata-Bold.ttf";
        let font_size = 24;
        let mut font = ttf_context.load_font(format!("resources/fonts/{}", font_name), font_size).expect("Could not load font file!");
        font.set_style(sdl2::ttf::FontStyle::BOLD);

        let mut chrs: [u8; 256] = [0; 256];
        for chr_ix in 0..256 {
            chrs[chr_ix] = chr_ix as u8;
        }

        let text_surface = font.render_latin1(&chrs[ASCII_START as usize .. ASCII_END as usize])
                               .blended(sdl2::pixels::Color::RGB(255, 255, 255))
                               .unwrap();

        let font_texture = texture_creator
            .create_texture_from_surface(&text_surface)
            .expect(&format!("Could not load font {}", font_name));

        let font_query = font_texture.query();
        let num_chars = (ASCII_END - ASCII_START + 1) as usize;
        let font = SpriteSheet::new("font".to_string(), num_chars, 1, num_chars, font_query.width as usize, font_query.height as usize, 0, 0);

        return Display { state: DisplayState::new(),
                         canvas,
                         texture_creator,
                         textures, 
                         sprites,
                         atlas_texture,
                         font_texture,
                         font,
                         panels,
                         screen_areas,
                         screen_texture,
                         intern: HashMap::new(),
                         next_str: 0,
                         canvas_panel,
                         rng: Rand32::new(0),
        };
    }

    pub fn load_atlas(&mut self) {
        self.sprites = parse_atlas_file("resources/spriteAtlas.txt");
        self.atlas_texture = self.texture_creator.load_texture("resources/spriteAtlas.png").expect("Could not load sprite atlas!");
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

            panel.process_cmds_if_new(clear,
                                      self.textures.get_mut(panel_name).unwrap(),
                                      canvas,
                                      &mut self.atlas_texture,
                                      &mut self.sprites,
                                      &mut self.font_texture,
                                      &mut self.font);
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

    pub fn add_sprite(&mut self, name: String, texture: Texture) {
        let sprite_sheet = SpriteSheet::single(name, texture);
        self.sprites.push(sprite_sheet);
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

    pub fn play_idle_animation(&mut self, entity_id: EntityId, config: &Config) {
        if let Some(anim) = self.get_idle_animation(entity_id, config) {
            self.state.play_animation(entity_id, anim);
        }
    }

    pub fn get_idle_animation(&mut self, entity_id: EntityId, config: &Config) -> Option<Animation> {
        let name = self.state.name[&entity_id];

        if name == EntityName::Player || name == EntityName::Gol || name == EntityName::Pawn || name == EntityName::Rook {
            // we check for stance first in case the entity has been created but no stance is yet
            // available.
            if self.state.stance.get(&entity_id).is_some() {
                let name = self.state.name[&entity_id];
                let stance = self.state.stance[&entity_id];
                let direction = self.state.direction[&entity_id];

                let sheet_direction = sheet_direction(direction);
                let mut sheet_name = format!("{}_{}_{}", name, stance, sheet_direction);

                if !self.sprite_exists(&sheet_name) {
                    sheet_name = format!("{}_{}_{}", name, Stance::Standing, sheet_direction);
                }

                let mut anim = self.loop_sprite(&sheet_name, config.idle_speed);
                anim.sprite_anim_mut().unwrap().flip_horiz = needs_flip_horiz(direction);

                return Some(anim);
            }
        } else {
            if name == EntityName::Key {
                return Some(self.loop_sprite("key", config.idle_speed));
            } else if name == EntityName::SpikeTrap {
                let sprite = self.static_sprite("rustrogueliketiles", ENTITY_SPIKE_TRAP as char);
                return Some(Animation::Loop(sprite));
            } else if name == EntityName::Armil {
                return Some(self.loop_sprite("armil_idle", config.idle_speed));
            } else if name == EntityName::Lantern {
                return Some(self.loop_sprite("Lantern_Idle", config.fire_speed));
            } else if name == EntityName::Smoke {
                let sprite = self.static_sprite("rustrogueliketiles", ENTITY_SMOKE as char);
                return Some(Animation::Loop(sprite));
            } else if name == EntityName::Khopesh {
                let sprite = self.static_sprite("rustrogueliketiles", ENTITY_KHOPESH as char);
                return Some(Animation::Loop(sprite));
            } else if name == EntityName::Magnifier {
                let sprite = self.static_sprite("rustrogueliketiles", ENTITY_MAGNIFIER as char);
                return Some(Animation::Loop(sprite));
            } else if name == EntityName::Sling {
                let sprite = self.static_sprite("rustrogueliketiles", ENTITY_SLING as char);
                return Some(Animation::Loop(sprite));
            } else if name == EntityName::GlassEye {
                let sprite = self.static_sprite("rustrogueliketiles", ENTITY_GLASS_EYE as char);
                return Some(Animation::Loop(sprite));
            } else if name == EntityName::Herb {
                let sprite = self.static_sprite("rustrogueliketiles", ENTITY_HERB as char);
                return Some(Animation::Loop(sprite));
            } else if name == EntityName::SeedOfStone {
                let sprite = self.static_sprite("rustrogueliketiles", ENTITY_SEED_OF_STONE as char);
                return Some(Animation::Loop(sprite));
            } else if name == EntityName::SeedCache {
                let sprite = self.static_sprite("rustrogueliketiles", ENTITY_SEED_CACHE as char);
                return Some(Animation::Loop(sprite));
            } else if name == EntityName::Teleporter {
                let sprite = self.static_sprite("rustrogueliketiles", ENTITY_TELEPORTER as char);
                return Some(Animation::Loop(sprite));
            } else if name == EntityName::Grass {
                return Some(self.random_sprite("GrassAnim", config.grass_idle_speed));
            } else if name == EntityName::Statue {
                let statues = vec!(MAP_STATUE_1, MAP_STATUE_2, MAP_STATUE_3, MAP_STATUE_4, MAP_STATUE_5, MAP_STATUE_6);
                let index = roguelike_utils::rng::choose(&mut self.rng, &statues).unwrap();
                let sprite = self.static_sprite("rustrogueliketiles", index as char);
                return Some(Animation::Loop(sprite));
            }
        }

        return None;
    }

    pub fn clear_level_state(&mut self) {
        self.state.impressions.clear();
        self.state.prev_turn_fov.clear();
        self.state.sound_tiles.clear();
        self.state.effects.clear();
        self.state.gate_pos.clear();
        self.state.frozen.clear();
        self.state.cursor_pos = None;

        self.clear_turn_state();
    }

    pub fn clear_turn_state(&mut self) {
        self.state.use_pos = None;
        self.state.use_dirs.clear();
        self.state.use_dir = None;
        self.state.hit_positions.clear();
        self.state.entities_at_cursor.clear();
        self.state.entity_movements.clear();
        self.state.entity_attacks.clear();
        self.state.entity_fov.clear();
        self.state.sound_tiles.clear();
        self.state.fov.clear();
        self.state.entities_in_fov.clear();
        self.state.inventory.clear();
        self.state.player_ghost = None;
    }

    pub fn map_message(&mut self, map_str: &str) {
        parse_map(map_str, &mut self.state.map);
    }

    pub fn console_message(&mut self, msg_line: String, config: &Config) {
        if msg_line.len() > 0 {
            self.state.msg_lines.push_back((self.state.turn_count, msg_line));
            if self.state.msg_lines.len() > config.display_console_lines {
                self.state.msg_lines.pop_front();
            }
        }
    }

    pub fn clear_console_messages(&mut self) {
        self.state.msg_lines.clear();
    }

    pub fn process_message(&mut self, msg: Msg, map: &Map, config: &Config) {
        match msg {
            Msg::StartTurn => {
                self.clear_turn_state();
                self.state.turn_count += 1;
            }

            Msg::PlayerAction => {
                // inventory is re-emitted after every action, so clear it first
                self.state.inventory.clear();
            }

            Msg::GameState(new_state) => {
                self.state.state = new_state;
            }

            Msg::CursorMove(pos) => {
                // clear entities at cursor at move- positions will come in separate messages
                self.state.entities_at_cursor.clear();
                self.state.cursor_pos = Some(pos);
            }

            Msg::CursorState(state, pos) => {
                if state {
                    self.state.cursor_pos = Some(pos);
                } else {
                    self.state.cursor_pos = None;

                    let tiles = lookup_spritekey(&self.sprites, "rustrogueliketiles");
                    let cursor_sprite = Sprite::new(ENTITY_CURSOR as u32, tiles);
                    let color = config.color_mint_green;
                    let fade_effect = Effect::fade(cursor_sprite, color, config.cursor_alpha, 0, pos, config.cursor_fade_seconds);
                    self.state.play_effect(fade_effect);
                }

                self.state.time_of_cursor_toggle = self.state.time;
            }

            Msg::StateChange(entity_id, behavior) => {
                if self.state.behavior.get(&entity_id).is_none() {
                    self.state.behavior.insert(entity_id, behavior);
                } else {
                    self.state.behavior[&entity_id] = behavior;
                }
            }

            Msg::SoundHitTile(cause_id, source_pos, radius, hit_pos) => {
                // Add to this turn's sound tiles list
                self.state.sound_tiles.push(hit_pos);

                let player_id = self.state.player_id();
                let player_pos = self.state.pos[&player_id];

                // only play the sound effect if the player position is included
                let sound_hits_player = hit_pos == player_pos;
                let sound_from_monster = self.state.typ.get(&cause_id) == Some(&EntityType::Enemy);

                let player_can_see_source = 
                    self.state.entity_is_in_fov(cause_id) == FovResult::Inside;

                let visible_monster_sound = sound_from_monster && player_can_see_source;
                if !visible_monster_sound && sound_hits_player {
                    let sound_aoe =
                        aoe_fill(map, AoeEffect::Sound, source_pos, radius, config);

                    let sound_effect = Effect::sound(sound_aoe);
                    self.state.play_effect(sound_effect);

                    if cause_id != player_id {
                        let pos = self.state.pos[&cause_id];
                        // NOTE it is slightly odd to look up this sprite sheet here and not in
                        // render.rs.
                        let tiles = lookup_spritekey(&self.sprites, "rustrogueliketiles");
                        let impression_sprite = Sprite::new(ENTITY_UNKNOWN as u32, tiles);
                        self.state.impressions.push(Impression::new(impression_sprite, pos));
                    }
                }
            }

            Msg::Restart => {
                self.state.skills.clear();
                self.state.turn_count = 0;
                self.clear_level_state();
                self.clear_console_messages();
            }

            Msg::Impression(pos) => {
                let tiles = lookup_spritekey(&self.sprites, "rustrogueliketiles");
                let impression_sprite = Sprite::new(ENTITY_UNKNOWN as u32, tiles);
                self.state.impressions.push(Impression::new(impression_sprite, pos));
            }

            Msg::ItemLanded(item_id, start, end) => {
                let sound_aoe = aoe_fill(map, AoeEffect::Sound, end, config.sound_radius_stone, config);

                let chr = self.state.chr[&item_id];
                let item_sprite = self.static_sprite("rustrogueliketiles", chr);

                let move_anim = Animation::Between(item_sprite, start, end, 0.0, config.item_throw_speed);
                let item_anim = Animation::PlayEffect(Effect::Sound(sound_aoe, 0.0));
                let loop_anim = Animation::Loop(item_sprite);

                self.state.play_animation(item_id, move_anim);
                self.state.append_animation(item_id, item_anim);
                self.state.append_animation(item_id, loop_anim);
            }

            Msg::PickedUp(entity_id, item_id) => {
                self.play_idle_animation(entity_id, config);

                // Remove the item from FOV, as picking it up will not
                // take a turn, and therefore not re-emit all entity 
                // FOV information.
                self.state.entities_in_fov.remove(&item_id);
            }

            Msg::Healed(entity_id, amount, max_hp) => {
                if self.state.hp.get(&entity_id).is_none() {
                    self.state.hp.insert(entity_id, 0);
                }
                self.state.hp[&entity_id] += amount;

                if self.state.max_hp.get(&entity_id).is_none() {
                    self.state.max_hp.insert(entity_id, 0);
                }
                self.state.max_hp[&entity_id] = max_hp;

                let entity_pos = self.state.pos[&entity_id];
                let heal_num = Effect::number_change(amount, entity_pos, config.color_mint_green);
                self.state.play_effect(heal_num);
            }

            Msg::Stance(entity_id, stance) => {
                self.state.stance.insert(entity_id, stance);
                self.play_idle_animation(entity_id, config);
            }

            Msg::GainEnergy(entity_id, amount) => {
                if self.state.energy.get(&entity_id).is_none() {
                    self.state.energy.insert(entity_id, 0);
                }
                self.state.energy[&entity_id] += amount;

                let entity_pos = self.state.pos[&entity_id];
                let energy_color = Color::new(176, 132, 87, 255);
                let effect = Effect::number_change(amount as i32, entity_pos, energy_color);
                self.state.play_effect(effect);
            }

            Msg::UsedEnergy(entity_id) => {
                if !self.state.test_mode {
                    self.state.energy[&entity_id] -= 1;

                    let entity_pos = self.state.pos[&entity_id];
                    let energy_color = Color::new(176, 132, 87, 255);
                    let effect = Effect::number_change(-1, entity_pos, energy_color);
                    self.state.play_effect(effect);
                }
            }

            Msg::GainStamina(entity_id, amount) => {
                if self.state.stamina.get(&entity_id).is_none() {
                    self.state.stamina.insert(entity_id, 0);
                }
                self.state.stamina[&entity_id] += amount;
                //let entity_pos = self.state.pos[&entity_id];
                //let effect = Effect::number_change(amount as i32, entity_pos, config.color_mint_green);
                //self.state.play_effect(effect);
            }

            Msg::UsedStamina(entity_id, amount) => {
                if !self.state.test_mode {
                    self.state.stamina[&entity_id] -= amount;

                    //let entity_pos = self.state.pos[&entity_id];
                    //let effect = Effect::number_change(-(amount as i32), entity_pos, config.color_mint_green);
                    //self.state.play_effect(effect);
                }
            }

            Msg::TestMode(state) => {
                self.state.test_mode = state;
            }

            Msg::Facing(entity_id, direction) => {
                self.state.direction[&entity_id] = direction;
                self.play_idle_animation(entity_id, config);
            }

            Msg::Killed(_attacker, attacked, _damage) => {
                if self.state.typ[&attacked] != EntityType::Player {
                    self.state.clear_animations(attacked);

                    let sprite_name = format!("{:?}_death", self.state.name[&attacked]);
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

                let attack_effect = Effect::attack(from, to, sprite_anim);
                self.state.play_effect(attack_effect);
            }

            Msg::HammerSwing(entity_id, _item_id, _pos) => {
                if self.state.typ[&entity_id] == EntityType::Player {
                    // TODO need hammer animation
                    //let attack_sprite =
                    //    self.new_sprite("player_attack_hammer".to_string(), config.player_attack_hammer_speed);
                    //let attack_anim = Animation::Once(attack_sprite);
                    //self.state.play_animation(entity_id, attack_anim);

                    //if let Some(idle_anim) = self.get_idle_animation(entity_id, config) {
                    //    self.state.append_animation(entity_id, idle_anim);
                    //}
                }
            }

            Msg::Stabbed(entity_id, _hit_entity) => {
                if self.state.typ[&entity_id] == EntityType::Player {
                    // TODO need dagger animation
                    //let attack_sprite =
                    //    self.new_sprite("player_attack_dagger".to_string(), config.player_attack_speed);
                    //let attack_anim = Animation::Once(attack_sprite);
                    //self.state.play_animation(entity_id, attack_anim);

                    //if let Some(idle_anim) = self.get_idle_animation(entity_id, config) {
                    //    self.state.append_animation(entity_id, idle_anim);
                    //}
                }
            }

            Msg::Attack(attacker, attacked, damage) => {
                let attacked_pos = self.state.pos[&attacked];
                let hit_nums = Effect::number_change(-damage, attacked_pos, config.color_light_red);
                self.state.play_effect(hit_nums);

                if self.state.typ[&attacker] == EntityType::Player {
                    // TODO need attack animation
                    //let attack_sprite =
                    //    self.new_sprite("player_attack".to_string(), config.player_attack_speed);
                    //let attack_anim = Animation::Once(attack_sprite);
                    //self.state.play_animation(attacker, attack_anim);

                    //if let Some(idle_anim) = self.get_idle_animation(attacker, config) {
                    //    self.state.play_animation(attacker, idle_anim);
                    //}
                } else {
                    let attacker_pos = self.state.pos[&attacker];
                    let beam_effect = Effect::beam(config.beam_duration, attacker_pos, attacked_pos);
                    self.state.play_effect(beam_effect);
                }

                if !self.state.test_mode {
                    self.state.hp[&attacked] -= damage;
                }
            }

            Msg::JumpWall(jumper, start, end) => {
                let jump_anim = self.between_sprite("playerjump_right", start, end, config.idle_speed);
                self.state.play_animation(jumper, jump_anim);
            }

            Msg::SpawnedObject(entity_id, typ, pos, name, facing) => {
                let chr = entity_name_to_chr(name);
                self.state.chr.insert(entity_id, chr as char);
                self.state.pos.insert(entity_id, pos);
                self.state.typ.insert(entity_id, typ);
                self.state.name.insert(entity_id, name);
                self.state.direction.insert(entity_id, facing);

                if let Some(ix_pos) = self.state.ids.iter().position(|val| *val == entity_id) {
                    eprintln!("entity id {} already at position {}", entity_id, ix_pos);
                    panic!();
                }

                self.state.ids.push(entity_id);

                self.play_idle_animation(entity_id, config);
            }

            Msg::PlayerTurn => {
                self.state.prev_turn_fov.clear();
                for (entity_id, fov_result) in self.state.entities_in_fov.iter() {
                    if *fov_result == FovResult::Inside {
                        self.state.prev_turn_fov.push(*entity_id);
                    }
                }

                // Update impressions based on previous FoV and current FoV.
                for entity_id in self.state.prev_turn_fov.iter() {
                    // Impressions are only of golems.
                    if self.state.typ.get(entity_id) != Some(&EntityType::Enemy) {
                        continue;
                    }

                    if self.state.entity_is_in_fov(*entity_id) != FovResult::Inside {
                        if let Some(sprite) = self.state.drawn_sprites.get(entity_id) {
                            let pos = self.state.pos[entity_id];
                            self.state.impressions.push(Impression::new(*sprite, pos));
                        }
                    }
                }

                /* Remove impressions that are currently visible */
                let mut impressions_visible = Vec::new();
                for (index, impression) in self.state.impressions.iter().enumerate() {
                    if self.state.pos_is_in_fov(impression.pos) == FovResult::Inside {
                        impressions_visible.push(index);
                    }
                }
                // We need to sort and reverse in order to swap_remove from the end each time,
                // avoiding the need to move sections of the original vector with 'remove'.
                impressions_visible.sort();
                impressions_visible.reverse();
                for index in impressions_visible.iter() {
                    self.state.impressions.swap_remove(*index);
                }
            }

            Msg::RemovedEntity(entity_id) => {
                self.state.animations.remove(&entity_id);
                self.state.chr.remove(&entity_id);
                self.state.pos.remove(&entity_id);
                self.state.typ.remove(&entity_id);
                self.state.name.remove(&entity_id);
                self.state.direction.remove(&entity_id);
                self.state.stance.remove(&entity_id);
                self.state.energy.remove(&entity_id);
                self.state.behavior.remove(&entity_id);
                self.state.hp.remove(&entity_id);
                self.state.max_hp.remove(&entity_id);
                self.state.gate_pos.remove(&entity_id);
                self.state.frozen.remove(&entity_id);

                if let Some(ix_pos) = self.state.ids.iter().position(|val| *val == entity_id) {
                    self.state.ids.remove(ix_pos);
                }
            }

            Msg::NewLevel => {
                self.clear_level_state();
                self.state.play_effect(Effect::particles(1.0));
                self.clear_console_messages();
            }

            Msg::Moved(entity_id, _move_type, _move_mode, pos) => {
                self.state.pos[&entity_id] = pos;
            }

            Msg::SetPos(entity_id, pos) => {
                self.state.pos[&entity_id] = pos;
            }

            Msg::TileFov(pos, fov_result) => {
                self.state.fov.insert(pos, fov_result);
            }

            Msg::EntityInFov(entity_id, in_fov) => {
                self.state.entities_in_fov.insert(entity_id, in_fov);
            }

            Msg::UsePos(pos) => {
                self.state.use_pos = Some(pos);
            }

            Msg::UseDir(dir) => {
                self.state.use_dir = Some(dir);
            }

            Msg::UseOption(pos, dir) => {
                self.state.use_dirs.insert((pos, dir));
            }

            Msg::UseDirClear => {
                self.state.use_dir = None;
            }

            Msg::UseHitPosClear => {
                self.state.hit_positions.clear();
            }

            Msg::UseHitPos(pos) => {
                self.state.hit_positions.insert(pos);
            }

            Msg::EntityAtCursor(entity_id) => {
                self.state.entities_at_cursor.push(entity_id);
            }

            Msg::EntityMovement(entity_id, pos) => {
                if self.state.entity_movements.get(&entity_id).is_none() {
                    self.state.entity_movements.insert(entity_id, Vec::new());
                }
                self.state.entity_movements.get_mut(&entity_id).unwrap().push(pos);
            }

            Msg::EntityAttack(entity_id, pos) => {
                if self.state.entity_attacks.get(&entity_id).is_none() {
                    self.state.entity_attacks.insert(entity_id, Vec::new());
                }
                self.state.entity_attacks.get_mut(&entity_id).unwrap().push(pos);
            }

            Msg::EntityFov(entity_id, pos) => {
                if self.state.entity_fov.get(&entity_id).is_none() {
                    self.state.entity_fov.insert(entity_id, Vec::new());
                }
                self.state.entity_fov.get_mut(&entity_id).unwrap().push(pos);
            }

            Msg::InventoryItem(item, item_class) => {
                self.state.inventory.push((item, item_class));
            }

            Msg::AddClass(_class) => {
                self.state.skills.clear();
            }

            Msg::AddSkill(skill) => {
                self.state.skills.push(skill);
            }

            Msg::GatePos(entity_id, pos) => {
                self.state.gate_pos.insert(entity_id, pos);
            }

            Msg::Froze(entity_id, num_turns) => {
                self.state.frozen.insert(entity_id, num_turns);
            }

            Msg::Thaw(entity_id, num_turns) => {
                self.state.frozen[&entity_id] -= num_turns;
            }

            Msg::PlayerGhost(player_ghost) => {
                self.state.player_ghost = Some(player_ghost);
            }

            Msg::NextMoveMode(move_mode) => {
                self.state.move_mode = move_mode;
            }

            Msg::Overlay(state) => {
                self.state.overlay = state;
            }

            Msg::DebugEnabled(state) => {
                self.state.debug_enabled = state;
            }

            Msg::UseAction(use_action) => {
                self.state.use_action = use_action;
            }

            Msg::SpikeTrapTriggered(trap, _entity_id) |
            Msg::SoundTrapTriggered(trap, _entity_id) |
            Msg::BlinkTrapTriggered(trap, _entity_id) |
            Msg::FreezeTrapTriggered(trap, _entity_id) => {
                self.state.pos[&trap] = Pos::new(-1, -1);
            }

            _ => {
            }
        }
    }

    pub fn draw_all(&mut self, dims: (i32, i32), rng: &mut Rand32, config: &Config) {
        self.process_draw_commands();
        self.copy_panels(dims);
        self.state.update_animations(rng, config);
    }

    pub fn copy_panels(&mut self, dims: (i32, i32)) {
        let canvas_panel = &mut self.canvas_panel;
        let panels = &mut self.panels;
        let textures = &mut self.textures;
        let state = self.state.state;
        let screen_areas = &self.screen_areas;

        let section_name_scale = 1.2;

        self.canvas.with_texture_canvas(&mut self.screen_texture, |canvas| {
            let dims = (dims.0 as u32, dims.1 as u32);

            /* Draw Screen in Sections */
            let pip_cell_dims = panels[&PanelName::Pip].cell_dims();
            let (pip_width, pip_height) = (pip_cell_dims.0 * dims.0, pip_cell_dims.1 * dims.1);
            let pip_src = Rect::new(0, 0, pip_width, pip_height);
            let pip_rect = canvas_panel.get_rect_from_area(&screen_areas[&PanelName::Pip]);
            canvas.copy(&textures[&PanelName::Pip], pip_src, pip_rect).unwrap();

            //let map_area = &screen_areas[&PanelName::Map];
            //canvas_panel.outline_area(map_area, 0.05);
            let map_cell_dims = panels[&PanelName::Map].cell_dims();
            let (map_width, map_height) = (map_cell_dims.0 * dims.0, map_cell_dims.1 * dims.1);
            let map_src = Rect::new(0, 0, map_width, map_height);
            let map_rect = canvas_panel.get_rect_from_area(&screen_areas[&PanelName::Map]);
            canvas.copy(&textures[&PanelName::Map], map_src, map_rect).unwrap();

            let player_area = screen_areas[&PanelName::Player];
            let map_area = screen_areas[&PanelName::Map];
            let filled = false;
            let cell_offset = 0.5;
            let ui_color = Color::new(0xcd, 0xb4, 0x96, 255);
            canvas_panel.rect_cmd(Pos::new(player_area.x_offset as i32, player_area.y_offset as i32),
                                  (map_area.width as u32, player_area.height as u32),
                                  cell_offset,
                                  filled,
                                  ui_color);

            /* Draw Inventory Panel */
            let inventory_rect = canvas_panel.get_rect_from_area(&screen_areas[&PanelName::Inventory]);
            canvas.copy(&textures[&PanelName::Inventory], None, inventory_rect).unwrap();

            let inventory_area = &screen_areas[&PanelName::Inventory];
            let text_color = Color::new(0, 0, 0, 255);
            let highlight_color = Color::new(0xcd, 0xb4, 0x96, 255);
            canvas_panel.justify_cmd("Inventory",
                                     Justify::Center,
                                     text_color,
                                     highlight_color,
                                     Pos::new(inventory_area.x_offset as i32, inventory_area.y_offset as i32),
                                     inventory_area.width as u32, 
                                     section_name_scale);

            /* Draw Game Info Panel */
            let info_rect = canvas_panel.get_rect_from_area(&screen_areas[&PanelName::Info]);
            canvas.copy(&textures[&PanelName::Info], None, info_rect).unwrap();

            let info_area = &screen_areas[&PanelName::Info];
            canvas_panel.outline_area(info_area, 0.5);
            canvas_panel.justify_cmd("Info",
                                     Justify::Center,
                                     text_color,
                                     highlight_color,
                                     Pos::new(2 * info_area.x_offset as i32, info_area.y_offset as i32),
                                     info_area.width as u32, 
                                     section_name_scale);

            /* Draw Player Info Panel */
            let player_rect = canvas_panel.get_rect_from_area(&screen_areas[&PanelName::Player]);
            canvas.copy(&textures[&PanelName::Player], None, player_rect).unwrap();

            let player_area = &screen_areas[&PanelName::Player];
            canvas_panel.outline_area(player_area, 0.5);
            canvas_panel.justify_cmd("Player",
                                     Justify::Center,
                                     text_color,
                                     highlight_color,
                                     Pos::new(2 * player_area.x_offset as i32, player_area.y_offset as i32),
                                     player_area.width as u32, 
                                     section_name_scale);
        }).unwrap();

        // Execute draw commands for the main canvas.
        let clear = false;
        canvas_panel.process_cmds(clear,
                                  &mut self.screen_texture,
                                  &mut self.canvas,
                                  &mut self.atlas_texture,
                                  &mut self.sprites,
                                  &mut self.font_texture,
                                  &mut self.font);

        // Render the menus last to ensure that they display on top of everything.
        self.canvas.with_texture_canvas(&mut self.screen_texture, |canvas| {
            if state.is_menu() {
                if state ==  GameState::HelpMenu {
                    let help_rect = canvas_panel.get_rect_from_area(&screen_areas[&PanelName::Help]);
                    canvas.copy(&textures[&PanelName::Help], None, help_rect).unwrap();
                } else {
                    let menu_rect = canvas_panel.get_rect_from_area(&screen_areas[&PanelName::Menu]);
                    canvas.copy(&textures[&PanelName::Menu], None, menu_rect).unwrap();
                }
            }
        }).unwrap();

        // Finally, copy the main canvas to the screen.
        self.canvas.copy(&self.screen_texture, None, None).unwrap();
    }
}

pub fn parse_atlas_file(atlas_file: &str) -> Vec<SpriteSheet> {
    let file =
        std::fs::File::open(&atlas_file).expect(&format!("Could not open atlas file '{}'", atlas_file));

    let mut sheets: Vec<SpriteSheet> = Vec::new();

    for line in std::io::BufReader::new(file).lines() {
        let line = line.unwrap();
        let line = line.to_string();

        if let Ok(sheet) = SpriteSheet::from_str(&line) { 
            sheets.push(sheet);
        }
    }

    return sheets;
}

pub type Panels = HashMap<PanelName, Panel>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisplayState {
    // currently active effects
    pub effects: Vec<Effect>,

    pub ids: Vec<EntityId>,

    // animation information
    pub animations: Comp<VecDeque<Animation>>,
    pub next_anim_key: i64,

    // sprites drawn this frame
    pub drawn_sprites: Comp<Sprite>,

    // entity information
    pub chr: Comp<char>,
    pub pos: Comp<Pos>,
    pub typ: Comp<EntityType>,
    pub name: Comp<EntityName>,
    pub direction: Comp<Direction>,
    pub stance: Comp<Stance>,
    pub energy: Comp<u32>,
    pub stamina: Comp<u32>,
    pub hp: Comp<i32>,
    pub max_hp: Comp<i32>,
    pub behavior: Comp<Behavior>,
    pub inventory: Vec<(Item, ItemClass)>,
    pub skills: Vec<Skill>,
    pub gate_pos: Comp<Pos>,
    pub frozen: Comp<usize>,
    pub player_ghost: Option<Pos>,

    pub map: Map,

    // settings
    pub debug_enabled: bool,
    pub overlay: bool,
    pub move_mode: MoveMode,
    pub use_action: UseAction,

    pub turn_count: usize,

    // game state
    pub state: GameState,

    // impressions left on map
    pub impressions: Vec<Impression>,

    // FOV information used when drawing
    pub prev_turn_fov: Vec<EntityId>,

    // tiles that heard a sound
    pub sound_tiles: Vec<Pos>,

    // Action log with turn count.
    pub msg_lines: VecDeque<(usize, String)>,

    // turn data from messages
    // Player FoV information. Missing tiles are Fov::Outside.
    pub fov: HashMap<Pos, FovResult>,
    pub entities_in_fov: HashMap<EntityId, FovResult>,
    pub use_pos: Option<Pos>,
    pub use_dirs: HashSet<(Pos, Direction)>,
    pub use_dir: Option<Direction>,
    pub hit_positions: HashSet<Pos>,
    pub entities_at_cursor: Vec<EntityId>,
    pub entity_movements: HashMap<EntityId, Vec<Pos>>,
    pub entity_attacks: HashMap<EntityId, Vec<Pos>>,
    pub entity_fov: HashMap<EntityId, Vec<Pos>>,

    // cursor visual effect state
    pub dt: f32,
    pub time: f32,
    pub time_of_cursor_toggle: f32,
    pub cursor_pos: Option<Pos>,

    pub test_mode: bool,

    pub debug_entries: HashMap<String, String>,
    pub rng: Rand32,
}

impl DisplayState {
    pub fn new() -> DisplayState {
        return DisplayState {
            effects: Vec::new(),
            ids: Vec::new(),
            animations: Comp::<VecDeque<Animation>>::new(),
            next_anim_key: 0,
            drawn_sprites: Comp::new(),
            chr: Comp::new(),
            pos: Comp::new(),
            typ: Comp::new(),
            name: Comp::new(),
            direction: Comp::new(),
            stance: Comp::new(),
            energy: Comp::new(),
            stamina: Comp::new(),
            hp: Comp::new(),
            max_hp: Comp::new(),
            behavior: Comp::new(),
            inventory: Vec::new(),
            skills: Vec::new(),
            gate_pos: Comp::new(),
            frozen: Comp::new(),
            player_ghost: None,
            map: Map::from_dims(1, 1),
            debug_enabled: false,
            overlay: false,
            move_mode: MoveMode::Walk,
            use_action: UseAction::Interact,
            turn_count: 0,
            state: GameState::Playing,
            impressions: Vec::new(),
            prev_turn_fov: Vec::new(),
            sound_tiles: Vec::new(),
            msg_lines: VecDeque::new(),
            fov: HashMap::new(),
            entities_in_fov: HashMap::new(),
            use_pos: None,
            use_dirs: HashSet::new(),
            use_dir: None,
            hit_positions: HashSet::new(),
            entities_at_cursor: Vec::new(),
            entity_movements: HashMap::new(),
            entity_attacks: HashMap::new(),
            entity_fov: HashMap::new(),
            dt: 0.0,
            time: 0.0,
            time_of_cursor_toggle: 0.0,
            cursor_pos: None,
            test_mode: false,
            debug_entries: HashMap::<String, String>::new(),
            rng: Rand32::new(0),
        };
    }

    pub fn update_animations(&mut self, rng: &mut Rand32, config: &Config) {
        for anims in self.animations.store.iter_mut() {
            if let Some(anim) = anims.get_mut(0) {
                anim.step(self.dt, rng, config);
            }
        }

        for effect in self.effects.iter_mut() {
            if let Effect::Attack(_from, _to, sprite_anim) = effect {
                sprite_anim.step(self.dt);
            }
        }
    }

    pub fn player_id(&self) -> EntityId {
        let mut player_id = None;
        for (key, nam) in self.name.iter() {
            if *nam == EntityName::Player {
                player_id = Some(key);
            }
        }
        return player_id.unwrap();
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

    pub fn entity_is_in_fov(&self, entity_id: EntityId) -> FovResult {
        if let Some(fov_result) = self.entities_in_fov.get(&entity_id) {
            return *fov_result;
        }

        return FovResult::Outside;
    }

    pub fn pos_is_in_fov(&self, pos: Pos) -> FovResult {
        if let Some(fov_result) = self.fov.get(&pos) {
            return *fov_result;
        }

        return FovResult::Outside;
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

fn entity_name_to_chr(name: EntityName) -> char {
    let mut chr = ' ' as u8;
    match name {
        EntityName::Player => chr = ENTITY_PLAYER,
        EntityName::Gol => chr = '\u{98}' as u8,
        EntityName::Pawn => chr = '\u{A5}' as u8,
        EntityName::Rook => chr = '\u{A5}' as u8,
        EntityName::Column => chr = MAP_COLUMN,
        EntityName::Key => chr = ENTITY_KEY,
        EntityName::Exit => chr = ENTITY_EXIT,
        EntityName::Dagger => chr = ENTITY_DAGGER,
        EntityName::Hammer => chr = ENTITY_HAMMER,
        EntityName::Spear => chr = ENTITY_SPEAR,
        EntityName::GreatSword => chr = ENTITY_GREATSWORD,
        EntityName::Sword => chr = ENTITY_SWORD,
        EntityName::Shield => chr = ENTITY_SHIELD,
        EntityName::Lantern => chr = ENTITY_LANTERN,
        EntityName::SeedOfStone => chr = ENTITY_SEED_OF_STONE,
        EntityName::GlassEye => chr = ENTITY_GLASS_EYE,
        EntityName::Teleporter => chr = ENTITY_TELEPORTER,
        EntityName::Spire => chr = '\u{15}' as u8,
        EntityName::Armil => chr = '\u{98}' as u8,
        EntityName::SpikeTrap => chr = MAP_TALL_SPIKES,
        EntityName::BlinkTrap => chr = ENTITY_BLINK_TRAP,
        EntityName::FreezeTrap => chr = ENTITY_FREEZE_TRAP,
        EntityName::SoundTrap => chr = ENTITY_TRAP_SOUND,
        EntityName::GateTrigger => chr = ENTITY_GATE_TRIGGER,
        EntityName::Stone => chr = ENTITY_STONE,
        EntityName::Energy => chr = ENTITY_ENERGY,
        EntityName::Herb => chr = ENTITY_HERB,
        EntityName::Statue => chr = MAP_STATUE_1,
        //Mouse, Cursor, Grass, Other
        _ => {},
    }
    return chr as char;
}

fn create_texture(texture_creator: &mut TextureCreator<WindowContext>, pixel_format: PixelFormatEnum, num_pixels: (u32, u32)) -> Texture {
    return texture_creator.create_texture_target(pixel_format, num_pixels.0, num_pixels.1).unwrap();
}

fn create_panels(screen_areas: &HashMap<PanelName, Area>) -> HashMap<PanelName, Panel> {
    let mut panels = HashMap::new();

    let over_sample = 16;

    let pip_pixels = (over_sample * PIP_WIDTH as u32 * FONT_WIDTH as u32, over_sample * PIP_HEIGHT as u32 * FONT_HEIGHT as u32);
    let pip_dims = screen_areas[&PanelName::Pip].dims();
    let pip_dims = (pip_dims.0 as u32, pip_dims.1 as u32);
    let pip_panel = Panel::new(pip_pixels, pip_dims);
    panels.insert(PanelName::Pip, pip_panel);

    let map_pixels = (over_sample * MAP_WIDTH as u32 * FONT_WIDTH as u32, over_sample * MAP_HEIGHT as u32 * FONT_HEIGHT as u32);
    let map_dims = screen_areas[&PanelName::Map].dims();
    let map_dims = (map_dims.0 as u32, map_dims.1 as u32);
    let map_panel = Panel::new(map_pixels, map_dims);
    panels.insert(PanelName::Map, map_panel);

    let over_sample = 10;
    let multiplier = 1.2;

    let info_dims = screen_areas[&PanelName::Info].dims();
    let info_dims = ((info_dims.0 as f32 / multiplier) as u32, (info_dims.1 as f32 / multiplier) as u32);
    let info_pixels = (over_sample * info_dims.0 * FONT_WIDTH as u32, over_sample * info_dims.1 * FONT_HEIGHT as u32);
    panels.insert(PanelName::Info, Panel::new(info_pixels, info_dims));

    let inventory_dims = screen_areas[&PanelName::Inventory].dims();
    let inventory_dims = ((inventory_dims.0 as f32 / multiplier) as u32, (inventory_dims.1 as f32 / multiplier) as u32);
    let inventory_pixels = (over_sample * inventory_dims.0 * FONT_WIDTH as u32, over_sample * inventory_dims.1 * FONT_HEIGHT as u32);
    panels.insert(PanelName::Inventory, Panel::new(inventory_pixels, inventory_dims));

    let player_dims = screen_areas[&PanelName::Player].dims();
    let player_dims = ((player_dims.0 as f32 / multiplier) as u32, (player_dims.1 as f32 / multiplier) as u32);
    let player_pixels = (over_sample * player_dims.0 * FONT_WIDTH as u32, over_sample * player_dims.1 * FONT_HEIGHT as u32);
    panels.insert(PanelName::Player, Panel::new(player_pixels, player_dims));

    //let menu_dims = info_dims;
    let menu_dims = screen_areas[&PanelName::Menu].dims();
    let menu_dims = ((menu_dims.0 as f32 / multiplier) as u32, (menu_dims.1 as f32 / multiplier) as u32);
    let menu_pixels = (over_sample * menu_dims.0 * FONT_WIDTH as u32, over_sample * menu_dims.1 * FONT_HEIGHT as u32);
    panels.insert(PanelName::Menu, Panel::new(menu_pixels, menu_dims));

    let help_dims = screen_areas[&PanelName::Help].dims();
    let help_dims = ((help_dims.0 as f32 / multiplier) as u32, (help_dims.1 as f32 / multiplier) as u32);
    let help_pixels = (over_sample * help_dims.0 * FONT_WIDTH as u32, over_sample * help_dims.1 * FONT_HEIGHT as u32);
    panels.insert(PanelName::Help, Panel::new(help_pixels, help_dims));

    return panels;
}

fn parse_map(map_str: &str, map: &mut Map) {
    let mut parts = map_str.split("!");
    let width = parts.next().unwrap().parse::<i32>().unwrap();
    let height = parts.next().unwrap().parse::<i32>().unwrap();

    if map.width() != width || map.height() != height {
        *map = Map::from_dims(width as u32, height as u32);
    }

    let summary = parts.next().unwrap();
    let chrs = parts.next().unwrap();

    let mut summary_tiles = Vec::new();
    for index in 0..(summary.len() / 8) {
        let tile_index = index * 8;
        let summary = &summary[tile_index..(tile_index + 8)];
        let tile = chrs_tile(summary);
        summary_tiles.push(tile);
    }

    let chrs = chrs.chars().collect::<Vec<char>>();
    let mut index = 0;
    let mut map_index = 0;
    while index < chrs.len() {
        let chr = chrs[index];

        if chr >= '0' && chr <= '9' {
            let chr_index = chr as u8 - '0' as u8;

            let times;
            if chr_index == 0 {
                times = 10;
            } else if chr_index == 1 {
                times = 11;
            } else {
                times = chr_index;
            }

            index += 1;
            let tile_chr = chrs[index];
            index += 1;

            let chr_index;
            let explored;
            if tile_chr.is_ascii_lowercase() {
                chr_index = tile_chr as u8 - 'a' as u8;
                explored = false;
            } else {
                chr_index = tile_chr as u8 - 'A' as u8;
                explored = true;
            }
            let mut tile = summary_tiles[chr_index as usize];
            tile.explored = explored;

            for _ in 0..times {
                let (x, y) = coord_from_index(map_index, width);
                map[(x, y)] = tile;

                map_index += 1;
            }
        } else {
            let chr_index;
            let explored;
            if chrs[index].is_ascii_lowercase() {
                chr_index = chr as u8 - 'a' as u8;
                explored = false;
            } else {
                chr_index = chr as u8 - 'A' as u8;
                explored = true;
            }
            let mut tile = summary_tiles[chr_index as usize];
            tile.explored = explored;

            let tile = summary_tiles[chr_index as usize];
            let (x, y) = coord_from_index(map_index, width);
            map[(x, y)] = tile;
            map_index += 1;

            index += 1;
        }
    }
}

fn coord_from_index(index: i32, width: i32) -> (i32, i32) {
    return (index % width, index / width);
}


fn chrs_tile(summary: &str) -> Tile {
    let chrs = summary.chars().collect::<Vec<char>>();

    let mut tile_index = 0;

    let mut tile = Tile::empty();
    tile.block_move = chr_bool(chrs[tile_index]);
    tile_index += 1;

    tile.block_sight = chr_bool(chrs[tile_index]);
    tile_index += 1;

    tile.tile_type = chr_tile_type(chrs[tile_index]);
    tile_index += 1;

    tile.bottom_wall = chr_wall(chrs[tile_index]);
    tile_index += 1;

    tile.bottom_material = chr_surface(chrs[tile_index]);
    tile_index += 1;

    tile.left_wall = chr_wall(chrs[tile_index]);
    tile_index += 1;

    tile.left_material = chr_surface(chrs[tile_index]);
    tile_index += 1;

    tile.surface = chr_surface(chrs[tile_index]);

    return tile;
}

fn chr_tile_type(chr: char) -> TileType {
    match chr {
        'e' => TileType::Empty,
        's' => TileType::ShortWall,
        'w' => TileType::Wall,
        'a' => TileType::Water,
        'x' => TileType::Exit,
        _ => panic!("unexpected tile_type char!"),
    }
}

fn chr_wall(chr: char) -> Wall {
    match chr {
        'e' => Wall::Empty,
        's' => Wall::ShortWall,
        't' => Wall::TallWall,
        _ => panic!("unexpected wall char!"),
    }
}

fn chr_surface(chr: char) -> Surface {
    match chr {
        'f' => Surface::Floor,
        'r' => Surface::Rubble,
        'g' => Surface::Grass,
        _ => panic!("unexpected surface char!"),
    }
}

fn chr_bool(chr: char) -> bool {
    match chr {
        '0' => false,
        '1' => true,
        _ => panic!("unexpected bool char!"),
    }
}
