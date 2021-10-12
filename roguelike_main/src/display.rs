use std::collections::VecDeque;
use std::collections::HashMap;

use bmp::Image;

use sdl2::render::{Texture, WindowCanvas, TextureCreator, BlendMode, RenderTarget, Canvas};
use sdl2::video::WindowContext;
use sdl2::rect::{Rect};
use sdl2::pixels::{PixelFormatEnum, Color as Sdl2Color};

use indexmap::map::IndexMap;

use roguelike_core::types::*;
use roguelike_core::constants::*;
use roguelike_core::config::*;
use roguelike_core::messaging::*;
use roguelike_core::map::*;
use roguelike_core::utils::aoe_fill;
use roguelike_core::movement::{Direction};

use roguelike_engine::game::Game;

use crate::animation::{Sprite, Effect, SpriteKey, Animation, SpriteAnim, SpriteIndex};


// consider
// highlight cell with color
// outline cell with color
// pixels vs cells
// black out cell
// NOTE use of String prevents Copy trait
#[derive(Clone, Debug, PartialEq)]
pub enum DrawCmd {
    Sprite(Sprite, Color, Pos),
    SpriteScaled(Sprite, f32, Option<Direction>, Color, Pos),
    SpriteAtPixel(Sprite, Color, Pos),
    HighlightTile(Color, Pos),
    OutlineTile(Color, Pos),
    Text(String, Color, Pos),
    Rect(Pos, (u32, u32), f32, bool, Color), // start cell, num cells width/height, offset percent into cell, color
    Fill(Pos, Color),
}

impl DrawCmd {
    pub fn aligned(&self) -> bool {
        return !matches!(self, DrawCmd::SpriteAtPixel(_, _, _));
    }

    pub fn pos(&self) -> Pos {
        match self {
            DrawCmd::Sprite(_, _, pos) => *pos,
            DrawCmd::SpriteScaled(_, _, _, _, pos) => *pos,
            DrawCmd::SpriteAtPixel(_, _, pos) => *pos,
            DrawCmd::HighlightTile(_, pos) => *pos,
            DrawCmd::OutlineTile(_, pos) => *pos,
            DrawCmd::Text(_, _, pos) => *pos,
            DrawCmd::Rect(pos, _, _, _, _) => *pos,
            DrawCmd::Fill(pos, _) => *pos,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PanelName {
    Info,
    Map,
    Player,
    Inventory,
    Background,
    Menu,
}

fn process_draw_cmd(panel: &Panel, canvas: &mut WindowCanvas, display_state: &mut DisplayState, cmd: &DrawCmd) {
    match cmd {
        DrawCmd::Sprite(sprite, color, pos) => {
            display_state.draw_sprite(panel, canvas, *sprite, *pos, *color);
        }

        DrawCmd::SpriteScaled(sprite, scale, direction, color, pos) => {
            let cell_dims = panel.cell_dims();
            let sprite_sheet = &mut display_state.sprites[&sprite.key];

            let src = sprite_sheet.sprite_src(sprite.index as usize);

            let (cell_width, cell_height) = cell_dims;
            let dst_width = (cell_width as f32 * scale) as u32;
            let dst_height = (cell_height as f32 * scale) as u32;

            let x_margin = ((cell_width - dst_width) / 2) as i32;
            let y_margin = ((cell_height - dst_height) / 2) as i32;

            let mut dst_x = pos.x * cell_width as i32;
            let mut dst_y = pos.y * cell_height as i32;
            match direction {
                None => {
                    dst_x += x_margin;
                    dst_y += y_margin;
                }
                
                Some(Direction::Left) => {
                    dst_y += y_margin;
                }

                Some(Direction::Right) => {
                    dst_x += cell_width as i32 - dst_width as i32;
                    dst_y += y_margin;
                }

                Some(Direction::Up) => {
                    dst_x += x_margin;
                }

                Some(Direction::Down) => {
                    dst_x += x_margin;
                    dst_y += cell_height as i32 - dst_height as i32;
                }

                Some(Direction::DownLeft) => {
                    dst_y += cell_height as i32 - dst_height as i32;
                }

                Some(Direction::DownRight) => {
                    dst_x += cell_width as i32 - dst_width as i32;
                    dst_y += cell_height as i32 - dst_height as i32;
                }

                Some(Direction::UpLeft) => {
                }

                Some(Direction::UpRight) => {
                    dst_x += cell_width as i32  - dst_width as i32;
                }
            }

            let dst = Rect::new(dst_x,
                                dst_y,
                                dst_width,
                                dst_height);

            canvas.set_blend_mode(BlendMode::Blend);
            sprite_sheet.texture.set_color_mod(color.r, color.g, color.b);
            sprite_sheet.texture.set_alpha_mod(color.a);

            canvas.copy_ex(&sprite_sheet.texture,
                                 Some(src),
                                 Some(dst),
                                 sprite.rotation,
                                 None,
                                 false,
                                 false).unwrap();
        }

        DrawCmd::SpriteAtPixel(sprite, color, pos) => {
            let (cell_width, cell_height) = panel.cell_dims();
            let pos = Pos::new(pos.x * cell_width as i32, pos.y * cell_height as i32);
            let sprite_sheet = &mut display_state.sprites[&sprite.key];
            sprite_sheet.draw_sprite_full(panel, canvas, sprite.index as usize, pos, *color, sprite.rotation, sprite.flip_horiz, sprite.flip_vert);
        }

        DrawCmd::OutlineTile(color, pos) => {
            draw_outline_tile(panel, canvas, *pos, *color);
        }

        DrawCmd::HighlightTile(color, pos) => {
            draw_tile_highlight(panel, canvas, *pos, *color);
        }

        DrawCmd::Text(string, color, start_pos) => {
            let sprite_key = display_state.lookup_spritekey("font");
            let sprite_sheet = &mut display_state.sprites[&sprite_key];
            let query = sprite_sheet.texture.query();

            let cell_dims = panel.cell_dims();
            let (cell_width, cell_height) = cell_dims;

            canvas.set_blend_mode(BlendMode::Blend);
            sprite_sheet.texture.set_color_mod(color.r, color.g, color.b);
            sprite_sheet.texture.set_alpha_mod(color.a);

            let mut pos = *start_pos;
            for chr in string.chars() {
                let chr_num = chr.to_lowercase().next().unwrap();
                let chr_index = chr_num as i32 - ASCII_START as i32;

                let ascii_width = ASCII_END - ASCII_START;
                let src = Rect::new((query.width as i32 / ascii_width as i32) * chr_index,
                                    0,
                                    query.width / ascii_width,
                                    query.height);

                let dst_pos = Pos::new(pos.x * cell_width as i32,
                                       pos.y * cell_height as i32);
                let dst = Rect::new(dst_pos.x as i32,
                                    dst_pos.y as i32,
                                    cell_width as u32,
                                    cell_height as u32);

                canvas.copy_ex(&sprite_sheet.texture,
                                     Some(src),
                                     Some(dst),
                                     0.0,
                                     None,
                                     false,
                                     false).unwrap();
                pos.x += 1;
            }
        }

        DrawCmd::Rect(pos, dims, offset, filled, color) => {
            assert!(*offset < 1.0, "offset >= 1 misaligns the starting cell!");

            // Draw a black background
            let (width, height) = canvas.output_size().unwrap();
            let (cell_width, cell_height) = panel.cell_dims();

            canvas.set_draw_color(sdl2_color(*color));

            let offset_x = (cell_width as f32 * offset) as i32;
            let x: i32 = cell_width as i32 * pos.x + offset_x as i32;

            let offset_y = (cell_height as f32 * offset) as i32;
            let y: i32 = cell_height as i32 * pos.y + offset_y as i32;

            let width = cell_width * dims.0 - (2 * offset_x as u32);
            let height = cell_height * dims.1 - (2 * offset_y as u32);

            if *filled {
                canvas.fill_rect(Rect::new(x, y, width, height));
            } else {
                canvas.draw_rect(Rect::new(x, y, width, height));
            }
        }

        DrawCmd::Fill(pos, color) => {
            let (cell_width, cell_height) = panel.cell_dims();
            canvas.set_draw_color(sdl2_color(*color));
            canvas.fill_rect(Rect::new(pos.x * cell_width as i32, pos.y * cell_height as i32, cell_width, cell_height));
        }
    }
}

pub struct Display {
    pub state: DisplayState,

    pub canvas_panel: Panel,
    pub canvas: WindowCanvas,

    pub background_panel: Panel,
    pub background: Texture,

    pub map_panel: Panel,
    pub map: Texture,

    pub player_panel: Panel,
    pub player: Texture,

    pub info_panel: Panel,
    pub info: Texture,

    pub inventory_panel: Panel,
    pub inventory: Texture,

    pub menu_panel: Panel,
    pub menu: Texture,

    pub texture_creator: TextureCreator<WindowContext>,
}

impl Display {
    pub fn new(canvas: WindowCanvas) -> Display {
        let texture_creator = canvas.texture_creator();
        let pixel_format = texture_creator.default_pixel_format();

        let over_sample = 5;

        let map_pixels = (over_sample * MAP_WIDTH as u32 * FONT_WIDTH as u32, over_sample * MAP_HEIGHT as u32 * FONT_HEIGHT as u32);
        let map_dims = (MAP_WIDTH as u32, MAP_HEIGHT as u32);
        let background_panel = Panel::new(map_pixels, map_dims);
        let background = texture_creator.create_texture_target(pixel_format, map_pixels.0, map_pixels.1).unwrap();

        let map_panel = Panel::new(map_pixels, map_dims);
        let map = texture_creator.create_texture_target(pixel_format, map_pixels.0, map_pixels.1).unwrap();

        let info_width = 14;

        let info_dims = (info_width, 15);
        let info_pixels = (info_dims.0 * FONT_WIDTH as u32, info_dims.1 * FONT_HEIGHT as u32);
        let info_panel = Panel::new(info_pixels, info_dims);
        let info = texture_creator.create_texture_target(pixel_format, info_pixels.0, info_pixels.1).unwrap();

        let inventory_dims = (info_width, 15);
        let inventory_pixels = (inventory_dims.0 * FONT_WIDTH as u32, inventory_dims.1 * FONT_HEIGHT as u32);
        let inventory_panel = Panel::new(inventory_pixels, inventory_dims);
        let inventory = texture_creator.create_texture_target(pixel_format, inventory_pixels.0, inventory_pixels.1).unwrap();

        let player_dims = (info_width, 20);
        let player_pixels = (player_dims.0 * FONT_WIDTH as u32, player_dims.1 * FONT_HEIGHT as u32);
        let player_panel = Panel::new(player_pixels, player_dims);
        let player = texture_creator.create_texture_target(pixel_format, player_pixels.0, player_pixels.1).unwrap();

        let menu_dims = (info_width, 15);
        let menu_pixels = (menu_dims.0 * FONT_WIDTH as u32, menu_dims.1 * FONT_HEIGHT as u32);
        let menu_panel = Panel::new(menu_pixels, menu_dims);
        let menu = texture_creator.create_texture_target(pixel_format, menu_pixels.0, menu_pixels.1).unwrap();

        let canvas_panel = Panel::new((SCREEN_WIDTH, SCREEN_HEIGHT), (SCREEN_WIDTH / FONT_WIDTH as u32, SCREEN_HEIGHT / FONT_HEIGHT as u32));

        return Display { state: DisplayState::new(),
                         canvas_panel,
                         canvas,
                         texture_creator,
                         background_panel,
                         background,
                         map_panel,
                         map,
                         player_panel,
                         player,
                         info_panel,
                         info,
                         menu_panel,
                         menu,
                         inventory_panel,
                         inventory, };
    }

    pub fn process_draw_commands(&mut self) {
        let canvas = &mut self.canvas;
        let display_state = &mut self.state;

        // copy background into the map before other draws.
        let background = &mut self.background;
        canvas.with_texture_canvas(&mut self.map, |canvas| {
            canvas.set_draw_color(Sdl2Color::RGB(0, 0, 0));
            canvas.clear();

            canvas.copy(&background, None, None).unwrap();
        }).unwrap();

        self.info_panel.process_cmds(true, &mut self.info, canvas, display_state);
        self.background_panel.process_cmds(true, &mut self.background, canvas, display_state);
        self.map_panel.process_cmds(false, &mut self.map, canvas, display_state);
        self.player_panel.process_cmds(true, &mut self.player, canvas, display_state);
        self.inventory_panel.process_cmds(true, &mut self.inventory, canvas, display_state);
        self.menu_panel.process_cmds(true, &mut self.menu, canvas, display_state);
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
        let sprite_key = self.state.next_sprite_key;
        let sprite_sheet = SpriteSheet::new(name, texture);
        self.state.next_sprite_key += 1;
        self.state.sprites.insert(sprite_key, sprite_sheet);
    }

    pub fn sprite_exists(&self, name: &str) -> bool {
        for (_key, sprite_sheet) in self.state.sprites.iter() {
            if sprite_sheet.name == *name {
                return true;
            }
        }

        return false;
    }

    pub fn static_sprite(&self, sprite_sheet: &str, chr: char) -> SpriteAnim {
        let sprite_key = self.state.lookup_spritekey(sprite_sheet);
        return SpriteAnim::new(format!("{}", chr),
                               sprite_key,
                               chr as i32 as SpriteIndex,
                               chr as i32 as SpriteIndex,
                               0.0);
    }

    /// Create and play a looping sprite
    pub fn loop_sprite(&mut self, sprite_name: &str, speed: f32) -> Animation {
        let sprite_anim = self.state.new_sprite(sprite_name, speed);
        
        let anim = Animation::Loop(sprite_anim);

        return anim;
    }

    pub fn play_idle_animation(&mut self, entity_id: EntityId, data: &mut GameData, config: &Config) {
        if let Some(anim) = self.get_idle_animation(entity_id, data, config) {
            self.state.play_animation(entity_id, anim);
        }
    }

    pub fn get_idle_animation(&mut self, entity_id: EntityId, data: &mut GameData, config: &Config) -> Option<Animation> {
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

    pub fn process_message(&mut self, msg: Msg, data: &mut GameData, config: &Config) {
        match msg {
            Msg::StartTurn => {
                self.state.sound_tiles.clear();
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
                        let tiles = self.state.lookup_spritekey("tiles");
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
                        let sprite = self.state.new_sprite(&sprite_name, 1.0);
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
                let mut sprite_anim = self.state.new_sprite(sprite_name, config.attack_animation_speed);
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

            Msg::JumpWall(_jumper, _start, _end) => {
                // This animation does not work
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

            _ => {
            }
        }
    }

    pub fn draw_all(&mut self, game: &mut Game) {
        self.process_draw_commands();
        self.copy_panels(game);
        self.state.update_animations(&game.config);
    }

    pub fn copy_panels(&mut self, game: &mut Game) {
        let map_size = game.data.map.size();

        /* Split Screen Into Sections */
        let screen_area = self.canvas_panel.area();
        let (map_area, rest_area) = screen_area.split_right(self.info_panel.cells.0 as usize);
        let (player_area, remaining_area) = rest_area.split_top(20);
        let (inventory_area, info_area) = remaining_area.split_top(15);

        let menu_area = self.menu_panel.area();
        let menu_area = map_area.centered(menu_area.width, menu_area.height);

        let map_rect = self.canvas_panel.get_rect_from_area(&map_area);

        // TODO just make the map panel the right size in the first place
        // and re-create it when the map changes.
        let src = self.map_panel.get_rect_up_left(map_size.0 as usize, map_size.1 as usize);
        self.canvas.copy(&self.background, src, map_rect).unwrap();
        self.canvas.copy(&self.map, src, map_rect).unwrap();

        /* Draw Inventory Panel */
        let dst = self.canvas_panel.get_rect_within(&inventory_area,
                                                       self.inventory_panel.num_pixels);
        self.canvas.copy(&self.inventory, None, dst).unwrap();

        /* Draw Game Info Panel */
        let dst = self.canvas_panel.get_rect_within(&info_area,
                                                       self.info_panel.num_pixels);
        self.canvas.copy(&self.info, None, dst).unwrap();

        /* Draw Player Info Panel */
        let dst = self.canvas_panel.get_rect_within(&player_area,
                                                       self.player_panel.num_pixels);
        self.canvas.copy(&self.player, None, dst).unwrap();

        // TODO perhaps this can be moved into draw command processing
        if game.settings.state.is_menu() {
            let canvas_panel = &mut self.canvas_panel;
            let dst = canvas_panel.get_rect_within(&menu_area, self.menu_panel.num_pixels);
            self.canvas.copy(&mut self.menu, None, dst).unwrap();
        }
    }
}

type TextureKey = u64;

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
pub struct Area {
    pub x_offset: usize,
    pub y_offset: usize,
    pub width: usize,
    pub height: usize,
}

impl Area {
    pub fn new(width: usize, height: usize) -> Area {
        return Area { x_offset: 0, y_offset: 0, width, height };
    }

    pub fn new_at(x_offset: usize, y_offset: usize, width: usize, height: usize) -> Area {
        return Area { x_offset, y_offset, width, height };
    }

    pub fn split_left(&self, left_width: usize) -> (Area, Area) {
        assert!(left_width <= self.width);

        let right_width = self.width - left_width;
        let left = Area::new_at(self.x_offset, self.y_offset, left_width, self.height);
        let right = Area::new_at(self.x_offset + left_width, self.y_offset, right_width, self.height);

        return (left, right);
    }

    pub fn split_right(&self, right_width: usize) -> (Area, Area) {
        assert!(right_width <= self.width);

        let left_width = self.width - right_width;
        let left = Area::new_at(self.x_offset, self.y_offset, left_width, self.height);
        let right = Area::new_at(self.x_offset + left_width, self.y_offset, right_width, self.height);

        return (left, right);
    }

    pub fn split_top(&self, top_height: usize) -> (Area, Area) {
        assert!(top_height <= self.height);

        let top = Area::new_at(self.x_offset, self.y_offset, self.width, top_height);
        let bottom = Area::new_at(self.x_offset, self.y_offset + top_height, self.width, self.height - top_height);

        return (top, bottom);
    }

    pub fn split_bottom(&self, bottom_height: usize) -> (Area, Area) {
        assert!(bottom_height <= self.height);

        let top_height = self.height - bottom_height;
        let top = Area::new_at(self.x_offset, self.y_offset, self.width, top_height);
        let bottom = Area::new_at(self.x_offset, self.y_offset + top_height, self.width, bottom_height);

        return (top, bottom);
    }

    pub fn centered(&self, width: usize, height: usize) -> Area {
        assert!(width <= self.width);
        assert!(height <= self.height);

        let x_offset = (self.width - width) / 2;
        let y_offset = (self.height - height) / 2;

        return Area::new_at(x_offset, y_offset, width, height);
    }

    pub fn cell_at_pixel(&self, pixel_pos: Pos) -> Option<(usize, usize)> {
        let cell_pos = Pos::new(pixel_pos.x / self.width as i32, pixel_pos.y / self.height as i32);

        return self.cell_at(cell_pos);
    }

    pub fn cell_at(&self, cell_pos: Pos) -> Option<(usize, usize)> {
        if cell_pos.x as usize >= self.x_offset && (cell_pos.x as usize) < self.x_offset + self.width &&
           cell_pos.y as usize >= self.y_offset && (cell_pos.y as usize) < self.y_offset + self.height {
               return Some((cell_pos.x as usize - self.x_offset, cell_pos.y as usize - self.y_offset));
        }

        return None;
    }
}

#[test]
pub fn test_area_splits_left() {
    let section = Area::new(100, 100);
    let (left, right) = section.split_left(20);

    assert_eq!(0, left.x_offset);
    assert_eq!(0, left.y_offset);
    assert_eq!(20, right.x_offset);
    assert_eq!(0, right.y_offset);

    assert_eq!(20, left.width);
    assert_eq!(80, right.width);
    assert_eq!(100, left.height);
    assert_eq!(100, right.height);
}

#[test]
pub fn test_area_splits_top() {
    let section = Area::new(100, 100);
    let (top, bottom) = section.split_top(20);

    assert_eq!(0, top.x_offset);
    assert_eq!(0, top.y_offset);
    assert_eq!(0, bottom.x_offset);
    assert_eq!(20, bottom.y_offset);

    assert_eq!(100, top.width);
    assert_eq!(100, bottom.width);
    assert_eq!(20, top.height);
    assert_eq!(80, bottom.height);
}

#[test]
pub fn test_area_splits_right() {
    let section = Area::new(100, 100);
    let (left, right) = section.split_right(20);

    assert_eq!(0, left.x_offset);
    assert_eq!(0, left.y_offset);
    assert_eq!(80, right.x_offset);
    assert_eq!(0, right.y_offset);

    assert_eq!(80, left.width);
    assert_eq!(20, right.width);
    assert_eq!(100, left.height);
    assert_eq!(100, right.height);
}

#[test]
pub fn test_area_splits_bottom() {
    let section = Area::new(100, 100);
    let (top, bottom) = section.split_bottom(20);
    assert_eq!(0, top.x_offset);
    assert_eq!(0, top.y_offset);
    assert_eq!(0, bottom.x_offset);
    assert_eq!(80, bottom.y_offset);

    assert_eq!(100, top.width);
    assert_eq!(100, bottom.width);
    assert_eq!(80, top.height);
    assert_eq!(20, bottom.height);
}

#[derive(Clone, Debug, PartialEq)]
pub struct Panel {
    pub cells: (u32, u32),
    pub num_pixels: (u32, u32),
    pub draw_cmds: Vec<DrawCmd>,
    pub old_draw_cmds: Vec<DrawCmd>,
}

impl Panel {
    pub fn new(num_pixels: (u32, u32), cells: (u32, u32)) -> Panel {
        return Panel { cells, num_pixels, draw_cmds: Vec::new(), old_draw_cmds: Vec::new(), };
    }

    pub fn cell_dims(&self) -> (u32, u32) {
        return (self.num_pixels.0 / self.cells.0, self.num_pixels.1 / self.cells.1);
    }

    pub fn area(&self) -> Area {
        return Area::new(self.cells.0 as usize, self.cells.1 as usize);
    }

    pub fn cell_from_pixel(&self, pixel: Pos) -> Pos {
        let dims = self.cell_dims();
        return Pos::new(pixel.x / dims.0 as i32, pixel.y / dims.1 as i32);
    }

    pub fn pixel_from_cell(&self, cell: Pos) -> Pos {
        let dims = self.cell_dims();
        return Pos::new(cell.x * dims.0 as i32, cell.y * dims.1 as i32);
    }

    pub fn get_rect_up_left(&self, width: usize, height: usize) -> Rect {
        assert!(width as u32 <= self.cells.0);
        assert!(height as u32 <= self.cells.1);

        let (cell_width, cell_height) = self.cell_dims();

        let pixel_width = width as u32 * cell_width;
        let pixel_height = height as u32 * cell_height;

        return Rect::new(0, 0, pixel_width, pixel_height);
    }

    pub fn get_rect_from_area(&self, area: &Area) -> Rect {
        let cell_dims = self.cell_dims();

        let x_offset = area.x_offset as f32 * cell_dims.0 as f32;
        let y_offset = area.y_offset as f32 * cell_dims.1 as f32;

        let width = (area.width as f32 * cell_dims.0 as f32) as u32;
        let height = (area.height as f32 * cell_dims.1 as f32) as u32;

        // don't draw off the screen
        assert!(x_offset as u32 + width <= self.num_pixels.0);
        assert!(y_offset as u32 + height <= self.num_pixels.1);

        return Rect::new(x_offset as i32, y_offset as i32, width, height);
    }

    pub fn get_rect_within(&self, area: &Area, target_dims: (u32, u32)) -> Rect {
        let (target_width, target_height) = target_dims;
        let base_rect = self.get_rect_from_area(area);

        let scale_x = base_rect.w as f32 / target_width as f32;
        let scale_y = base_rect.h as f32 / target_height as f32;

        let scaler;
        if scale_x * target_height as f32 > base_rect.h as f32 {
            scaler = scale_y;
        } else {
            scaler = scale_x;
        }

        let final_target_width = target_width as f32 * scaler;
        let final_target_height = target_height as f32 * scaler;

        let x_inner_offset = (base_rect.w as f32 - final_target_width) / 2.0;
        let y_inner_offset = (base_rect.h as f32 - final_target_height) / 2.0;
        let x_offset = base_rect.x + x_inner_offset as i32;
        let y_offset = base_rect.y + y_inner_offset as i32;

        // check that we don't reach past the destination rect we should be drawing within
        assert!((x_offset as f32 + final_target_width  as f32) <= base_rect.x as f32 + base_rect.w as f32);
        assert!((y_offset as f32 + final_target_height as f32) <= base_rect.y as f32 + base_rect.h as f32);

        return Rect::new(x_offset as i32,
                         y_offset as i32,
                         final_target_width as u32,
                         final_target_height as u32);
    }

    pub fn sprite_at_pixel_cmd(&mut self, sprite: Sprite, color: Color, pos: Pos) {
        let cmd = DrawCmd::SpriteAtPixel(sprite, color, pos);
        self.draw_cmd(cmd);
    }

    pub fn sprite_cmd(&mut self, sprite: Sprite, color: Color, pos: Pos) {
        let cmd = DrawCmd::Sprite(sprite, color, pos);
        self.draw_cmd(cmd);
    }

    pub fn sprite_scaled_cmd(&mut self, sprite: Sprite, scale: f32, direction: Option<Direction>, color: Color, pos: Pos) {
        let cmd = DrawCmd::SpriteScaled(sprite, scale, direction, color, pos);
        self.draw_cmd(cmd);
    }

    pub fn outline_cmd(&mut self, color: Color, pos: Pos) {
        let cmd = DrawCmd::OutlineTile(color, pos);
        self.draw_cmd(cmd);
    }

    pub fn highlight_cmd(&mut self, color: Color, pos: Pos) {
        let cmd = DrawCmd::HighlightTile(color, pos);
        self.draw_cmd(cmd);
    }

    pub fn text_cmd(&mut self, text: &str, color: Color, pos: Pos) {
        let string = text.to_string();
        let cmd = DrawCmd::Text(string, color, pos);
        self.draw_cmd(cmd);
    }

    pub fn text_list_cmd(&mut self, text_list: &Vec<String>, color: Color, cell: Pos) {
        for (index, text) in text_list.iter().enumerate() {
            let text_cell = Pos::new(cell.x, cell.y + index as i32);
            self.text_cmd(text, color, text_cell);
        }
    }

    pub fn rect_cmd(&mut self, pos: Pos, dims: (u32, u32), offset: f32, filled: bool, color: Color) {
        let cmd = DrawCmd::Rect(pos, dims, offset, filled, color);
        self.draw_cmd(cmd);
    }

    pub fn fill_cmd(&mut self, pos: Pos, color: Color) {
        let cmd = DrawCmd::Fill(pos, color);
        self.draw_cmd(cmd);
    }

    pub fn draw_cmd(&mut self, cmd: DrawCmd) {
        self.draw_cmds.push(cmd);
    }

    pub fn process_cmds(&mut self, clear: bool, texture: &mut Texture, canvas: &mut WindowCanvas, display_state: &mut DisplayState) {
        // As a simple optimization, only redraw if the commands haven't changed. This is common
        // for informational panels.
        if self.draw_cmds != self.old_draw_cmds {
            // Collect a map of positions which are going to be filled, to avoid drawing
            // aligned sprites below those tiles.
            let mut fill_map = HashMap::<Pos, u32>::new();
            for cmd in self.draw_cmds.iter() {
                if let DrawCmd::Fill(pos, color) = cmd {
                    if !fill_map.contains_key(pos) {
                        fill_map.insert(*pos, 0);
                    }
                    fill_map.get_mut(pos).map(|count| *count += 1);
                }
            }

            canvas.with_texture_canvas(texture, |canvas| {
                // we don't clear the map as the background was already drawn over it.
                // TODO consider removing background panel and just using map- it was an
                // optimization that we may be able to remove if other optimizations are good enough
                // TODO if the fill cmd optimization has enough of an effect, the background
                // panel optimization may be unnecessary
                if clear {
                  canvas.set_draw_color(Sdl2Color::RGBA(0, 0, 0, 255));
                  canvas.clear();
                }

                for cmd in self.draw_cmds.iter() {
                    // Check if there is going to be a fill in this position,
                    // in which case there is no need to draw an aligned command.
                    if cmd.aligned() {
                        let pos = cmd.pos();
                        if fill_map.contains_key(&pos) {
                            let is_fill =  matches!(cmd, DrawCmd::Fill(_, _));

                            if let Some(count) = fill_map.get_mut(&pos) {
                                if *count > 0 && is_fill {
                                    *count -= 1;
                                }

                                if *count > 0 {
                                    continue;
                                }
                            }
                        }
                    }
                    process_draw_cmd(self, canvas, display_state, cmd);
                }
            }).unwrap();

            self.old_draw_cmds.clear();
            std::mem::swap(&mut self.draw_cmds, &mut self.old_draw_cmds);
        }

        self.draw_cmds.clear();
    }
}


pub struct DisplayState {
    // sprite state
    pub sprites: IndexMap<SpriteKey, SpriteSheet>,
    pub next_sprite_key: i64,

    // currently active effects
    pub effects: Vec<Effect>,

    // animation information
    pub animations: IndexMap<EntityId, VecDeque<Animation>>,
    pub next_anim_key: i64,

    // sprites drawn this frame
    pub drawn_sprites: IndexMap<EntityId, Sprite>,

    // impressions left on map
    pub impressions: Vec<Impression>,

    // FOV information used when drawing
    pub prev_turn_fov: Vec<EntityId>,
    pub current_turn_fov: Vec<EntityId>,

    // tiles that heard a sound
    pub sound_tiles: Vec<Pos>,

    pub dt: f32,

    pub debug_entries: HashMap<String, String>,
}

impl DisplayState {
    pub fn new() -> DisplayState {

        return DisplayState {
            sprites: IndexMap::new(),
            next_sprite_key: 0,
            effects: Vec::new(),
            animations: IndexMap::<EntityId, VecDeque<Animation>>::new(),
            next_anim_key: 0,
            drawn_sprites: IndexMap::new(),
            impressions: Vec::new(),
            prev_turn_fov: Vec::new(),
            current_turn_fov: Vec::new(),
            sound_tiles: Vec::new(),
            dt: 0.0,
            debug_entries: HashMap::<String, String>::new(),
        };
    }
    pub fn lookup_spritekey(&self, name: &str) -> SpriteKey {
        for (key, sprite_sheet) in self.sprites.iter() {
            if sprite_sheet.name == *name {
                return *key;
            }
        }

        panic!(format!("Could not find sprite '{}'", name));
    }

    pub fn update_animations(&mut self, config: &Config) {
        for anims in self.animations.values_mut() {
            if let Some(anim) = anims.get_mut(0) {
                anim.step(self.dt, config);
                //if let Some(sprite_anim) = anim.sprite_anim_mut() {
                //    sprite_anim.step(self.dt);
                //}
            }
        }
    }

    /// Create a sprite by looking up a texture and constructing the
    /// SpriteAnim structure.
    pub fn new_sprite(&self, name: &str, speed: f32) -> SpriteAnim {
        let sprite_key = self.lookup_spritekey(name);
        let max_index = self.sprites[&sprite_key].num_sprites;
        return SpriteAnim::new(name.to_string(), sprite_key, 0.0, max_index as f32, speed);
    }

    pub fn draw_sprite(&mut self,
                       panel: &Panel,
                       canvas: &mut WindowCanvas,
                       sprite: Sprite,
                       pos: Pos,
                       color: Color) {
        let sprite_sheet = &mut self.sprites[&sprite.key];
        sprite_sheet.draw_sprite_at_cell(panel, canvas, sprite.index as usize, pos, color, sprite.rotation, sprite.flip_horiz, sprite.flip_vert);
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


#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Impression {
    pub sprite: Sprite,
    pub pos: Pos,
}

impl Impression {
    pub fn new(sprite: Sprite, pos: Pos) -> Impression {
        return Impression { sprite, pos };
    }
}


pub struct SpriteSheet {
    pub texture: Texture,
    pub name: String,
    pub num_sprites: usize,
    pub rows: usize,
    pub cols: usize,
    pub width: usize,
    pub height: usize,
}

impl SpriteSheet {
    pub fn new(name: String, texture: Texture) -> SpriteSheet {
        let tex_info = texture.query();
        let width = tex_info.width as usize;
        let height = tex_info.height as usize;

        let rows = height / FONT_HEIGHT as usize;
        let cols = width / FONT_WIDTH as usize;
        let num_sprites = cols * rows;

        return SpriteSheet {
            texture,
            name,
            num_sprites,
            rows,
            cols,
            width,
            height,
        };
    }

    pub fn num_cells(&self) -> (usize, usize) {
        return (self.cols, self.rows);
    }

    pub fn num_pixels(&self) -> (usize, usize) {
        return (self.width, self.height);
    }

    pub fn sprite_dims(&self) -> (usize, usize) {
        let (num_width, num_height) = self.num_cells();
        return (self.width / num_width, self.height / num_height);
    }

    pub fn draw_text_list<T>(&mut self,
                             panel: &Panel,
                             canvas: &mut Canvas<T>,
                             text_list: &Vec<String>,
                             cell: Pos,
                             color: Color) where T: RenderTarget {
        for (index, text) in text_list.iter().enumerate() {
            let text_cell = Pos::new(cell.x, cell.y + index as i32);
            self.draw_text(panel, canvas, text, text_cell, color);
        }
    }

    pub fn draw_texture<T>(&mut self,
                           panel: &Panel,
                           canvas: &mut Canvas<T>,
                           cell: Pos) where T: RenderTarget {
        let query = self.texture.query();

        let cell_dims = panel.cell_dims();
        let (cell_width, cell_height) = cell_dims;

        canvas.set_blend_mode(BlendMode::None);

        let pos = cell;

        let src = Rect::new(0,
                            0,
                            query.width,
                            query.height);

        let dst_pos = Pos::new(pos.x * cell_width as i32,
                               pos.y * cell_height as i32);
        let dst = Rect::new(dst_pos.x as i32,
                            dst_pos.y as i32,
                            query.width,
                            query.height);

        canvas.copy_ex(&self.texture,
                             Some(src),
                             Some(dst),
                             0.0,
                             None,
                             false,
                             false).unwrap();
    }

    pub fn draw_text<T>(&mut self,
                        panel: &Panel,
                        canvas: &mut Canvas<T>,
                        text: &str,
                        cell: Pos,
                        color: Color) where T: RenderTarget {
        let query = self.texture.query();

        let cell_dims = panel.cell_dims();
        let (cell_width, cell_height) = cell_dims;

        canvas.set_blend_mode(BlendMode::Blend);
        self.texture.set_color_mod(color.r, color.g, color.b);
        self.texture.set_alpha_mod(color.a);

        let mut pos = cell;
        for chr in text.chars() {
            let chr_num = chr.to_lowercase().next().unwrap();
            let chr_index = chr_num as i32 - ASCII_START as i32;

            let ascii_width = ASCII_END - ASCII_START;
            let src = Rect::new((query.width as i32 / ascii_width as i32) * chr_index,
                                0,
                                query.width / ascii_width,
                                query.height);

            let dst_pos = Pos::new(pos.x * cell_width as i32,
                                   pos.y * cell_height as i32);
            let dst = Rect::new(dst_pos.x as i32,
                                dst_pos.y as i32,
                                cell_width as u32,
                                cell_height as u32);

            canvas.copy_ex(&self.texture,
                                 Some(src),
                                 Some(dst),
                                 0.0,
                                 None,
                                 false,
                                 false).unwrap();
            pos.x += 1;
        }
    }

    pub fn draw_char<T>(&mut self,
                        panel: &Panel,
                        canvas: &mut Canvas<T>,
                        chr: char,
                        cell: Pos,
                        color: Color) where T: RenderTarget {
        self.draw_sprite_at_cell(panel, canvas, chr as usize, cell, color, 0.0, false, false);
    }

    pub fn draw_sprite_at_cell<T>(&mut self,
                                  panel: &Panel,
                                  canvas: &mut Canvas<T>,
                                  index: usize,
                                  cell: Pos,
                                  color: Color,
                                  rotation: f64,
                                  flip_horiz: bool,
                                  flip_vert: bool) where T: RenderTarget {
        let (cell_width, cell_height) = panel.cell_dims();

        let pos = Pos::new(cell.x * cell_width as i32, cell.y * cell_height as i32);

        self.draw_sprite_full(panel, canvas, index, pos, color, rotation, flip_horiz, flip_vert);
    }

    pub fn draw_sprite_full<T>(&mut self,
                            panel: &Panel,
                            canvas: &mut Canvas<T>,
                            index: usize,
                            pos: Pos,
                            color: Color,
                            rotation: f64,
                            flip_horizontal: bool,
                            flip_vertical: bool) where T: RenderTarget {
        let cell_dims = panel.cell_dims();

        let src = self.sprite_src(index);

        let (cell_width, cell_height) = cell_dims;

        let dst = Rect::new(pos.x as i32,
                            pos.y as i32,
                            cell_width as u32,
                            cell_height as u32);

        canvas.set_blend_mode(BlendMode::Blend);
        self.texture.set_color_mod(color.r, color.g, color.b);
        self.texture.set_alpha_mod(color.a);

        canvas.copy_ex(&self.texture,
                             Some(src),
                             Some(dst),
                             rotation,
                             None,
                             flip_horizontal,
                             flip_vertical).unwrap();
    }

    fn sprite_src(&mut self, index: usize) -> Rect {
        let (num_cells_x, _num_cells_y) = self.num_cells();
        let sprite_x = index % num_cells_x;
        let sprite_y = index / num_cells_x;

        let (sprite_width, sprite_height) = self.sprite_dims();
        let src = Rect::new((sprite_x * sprite_width) as i32,
                            (sprite_y * sprite_height) as i32,
                            sprite_width as u32,
                            sprite_height as u32);

        return src;
    }
}

pub fn engine_color(color: &Color) -> Sdl2Color {
    return Sdl2Color::RGBA(color.r, color.g, color.b, color.a);
}

pub fn draw_outline_tile<T>(panel: &Panel,
                            canvas: &mut Canvas<T>,
                            cell: Pos,
                            color: Color) where T: RenderTarget {
    let cell_dims = panel.cell_dims();

    canvas.set_blend_mode(BlendMode::Add);
    canvas.set_draw_color(Sdl2Color::RGBA(color.r, color.g, color.b, color.a));

    let rect = Rect::new(cell.x * cell_dims.0 as i32 + 1,
                         cell.y * cell_dims.1 as i32 + 1,
                         cell_dims.0,
                         cell_dims.1);

    canvas.draw_rect(rect).unwrap();
}

pub fn draw_tile_highlight<T>(panel: &Panel,
                              canvas: &mut Canvas<T>,
                              cell: Pos,
                              color: Color) where T: RenderTarget {
    let cell_dims = panel.cell_dims();

    canvas.set_blend_mode(BlendMode::Blend);
    canvas.set_draw_color(Sdl2Color::RGBA(color.r, color.g, color.b, color.a));

    let rect = Rect::new(cell.x * cell_dims.0 as i32,
                         cell.y * cell_dims.1 as i32,
                         cell_dims.0,
                         cell_dims.1);

    canvas.fill_rect(rect).unwrap();
}

pub fn cell_within_rect(rect: Rect, area_cell_dims: (i32, i32), pixel_pos: (i32, i32)) -> Option<(i32, i32)> {
    if pixel_pos.0 >= rect.x && pixel_pos.0 < rect.x + rect.w &&
       pixel_pos.1 >= rect.y && pixel_pos.1 < rect.y + rect.h {

       let cell_dims = (rect.w / area_cell_dims.0, rect.h / area_cell_dims.1);
       let x_cell = (pixel_pos.0 - rect.x) / cell_dims.0;
       let y_cell = (pixel_pos.1 - rect.y) / cell_dims.1;

       //assert!(x_cell * area_cell_dims.0 < rect.x + rect.w);
       //assert!(y_cell * area_cell_dims.1 < rect.y + rect.h);

       // NOTE hacky way to prevent this situation
       if x_cell >= area_cell_dims.0 || y_cell >= area_cell_dims.1 {
           return None;
       }

       return Some((x_cell, y_cell));
    }

    return None;
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

fn sdl2_color(color: Color) -> Sdl2Color {
    return Sdl2Color::RGBA(color.r, color.g, color.b, color.a);
}

