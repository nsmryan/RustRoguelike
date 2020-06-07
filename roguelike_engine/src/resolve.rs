use log::trace;

use roguelike_core::types::*;
use roguelike_core::map::{Surface, AoeEffect};
use roguelike_core::messaging::{MsgLog, Msg};
use roguelike_core::constants::*;
use roguelike_core::movement::{MoveMode, MoveType};
use roguelike_core::config::*;
use roguelike_core::utils::*;

use crate::game::*;


pub fn resolve_messages(data: &mut GameData, msg_log: &mut MsgLog, _settings: &mut GameSettings, config: &Config) {
    /* Handle Message Log */
    while let Some(msg) = msg_log.pop() {
        let msg_line = msg.msg_line(data);
        if msg_line.len() > 0 {
            println!("msg: {}", msg_line);
        }

        match msg {
            Msg::Crushed(object_id, pos, _obj_type) => {
                data.map[pos].surface = Surface::Rubble;

                msg_log.log_front(Msg::Sound(object_id, pos, config.sound_radius_crushed, true));
            }

            Msg::Sound(cause_id, source_pos, radius, _should_animate) => {
                trace!("sound {} {}", cause_id, source_pos);
                let sound_aoe =
                    data.map.aoe_fill(AoeEffect::Sound, source_pos, radius);

                let who_heard =
                    data.within_aoe(&sound_aoe);

                for obj_id in who_heard {
                    if obj_id != cause_id {
                        // TODO replace with an Alerted message
                        data.entities.messages[&obj_id].push(Message::Sound(cause_id, source_pos));
                    }
                }
            }

            Msg::ItemThrow(thrower, _item_id, _start, end) => {
                // NOTE the radius here is the stone radius, regardless of item type

                msg_log.log_front(Msg::Sound(thrower, end, SOUND_RADIUS_STONE, false));
            }

            Msg::Moved(object_id, movement, pos) => {
                let player_handle = data.find_player().unwrap();
                if object_id == player_handle {

                    if matches!(movement.typ, MoveType::Pass) {
                        if data.entities.move_mode[&player_handle] ==
                           MoveMode::Run {
                            data.entities.move_mode[&player_handle].decrease();
                        }
                        // this is just to pattern match on movement
                    } else { // monster moved
                        let mut sound_radius;
                        match data.entities.move_mode[&player_handle] {
                            MoveMode::Sneak => sound_radius = SOUND_RADIUS_SNEAK,
                            MoveMode::Walk => sound_radius = SOUND_RADIUS_WALK,
                            MoveMode::Run => sound_radius = SOUND_RADIUS_RUN,
                        }

                        if data.map[pos].surface == Surface::Rubble {
                            sound_radius += config.sound_rubble_radius;
                        } else if data.map[pos].surface == Surface::Grass {
                            sound_radius -= config.sound_grass_radius;
                        }

                        msg_log.log_front(Msg::Sound(object_id, pos, sound_radius, true));
                    }
                } else {
                    msg_log.log_front(Msg::Sound(object_id, pos, SOUND_RADIUS_MONSTER_MOVE, true));
                }
            }

            Msg::Yell(pos) => {
                // NOTE this assumes that only the player yells
                let player_handle = data.find_player().unwrap();

                msg_log.log_front(Msg::Sound(player_handle, pos, config.player_yell_radius, true));
            }

            Msg::Killed(_attacker, attacked, _damage) => {
                if data.entities.typ[&attacked] != ObjType::Player {
                    let pos = data.entities.pos[&attacked];

                    data.map[pos].surface = Surface::Rubble;
                }

                data.entities.needs_removal[&attacked] = true;
            }

            Msg::Attack(attacker, attacked, _damage) => {
                let pos = data.entities.pos[&attacked];
                msg_log.log_front(Msg::Sound(attacker, pos, config.sound_radius_attack, true)); 
            }

            Msg::HammerSwing(entity, pos) => {
                let entity_pos = data.entities.pos[&entity];
                let pos_diff = sub_pos(pos, entity_pos);

                // can't swing at yourself, and can't swing diagonally
                if is_ordinal(pos_diff) {
                    if let Some(hit_entity) = data.has_blocking_entity(pos) {
                        // we hit another entity!
                        msg_log.log_front(Msg::HammerHitEntity(entity, hit_entity));
                    } else if let Some(blocked) = data.map.is_blocked_by_wall(entity_pos, pos_diff.x, pos_diff.y) {
                        msg_log.log_front(Msg::HammerHitWall(entity, blocked));
                    }
                }
            }

            Msg::HammerHitEntity(entity, hit_entity) => {
                let first = data.entities.pos[&entity];
                let second = data.entities.pos[&hit_entity];
                push_attack(entity, hit_entity, sub_pos(first, second), false, data, msg_log);

                // TODO this is repeated in push_attack, and likely elsewhere
                data.entities.alive[&hit_entity] = false;
                data.entities.blocks[&hit_entity] = false;
                let damage = data.entities.fighter[&hit_entity].hp;

                msg_log.log(Msg::Killed(entity, hit_entity, damage));
            }

            Msg::HammerHitWall(entity, blocked) => {
                let hit_pos = blocked.end_pos;
                if data.map[hit_pos].blocked {
                    if data.map[hit_pos].surface == Surface::Floor {
                        data.map[hit_pos].surface = Surface::Rubble;
                    }

                    data.map[hit_pos].blocked = false;
                    data.map[hit_pos].chr = ' ' as u8;

                    msg_log.log_front(Msg::Sound(entity, blocked.end_pos, config.sound_radius_attack, true)); 
                }

                //NOTE ignore between-wall tiles
            }

            _ => {
            }
        }
    }

    /* Process Player Messages */
    let player_handle = data.find_player().unwrap();
    for message in data.entities.messages[&player_handle].iter() {
        if let Message::Sound(obj_id, pos) = message {
            if *obj_id == player_handle {
                panic!("Player sent themselves a message?")
            }

            let player_pos = data.entities.pos[&player_handle];
            if !data.map.is_in_fov(player_pos, *pos, config.fov_radius_player) {
                //let heard = Effect::HeardSomething(*pos, settings.turn_count);
                // TODO move to somewhere else?
                //display_state.effects.push(heard);
            }
        }
    }
    data.entities.messages[&player_handle].clear();
}

