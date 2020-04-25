use roguelike_core::types::*;
use roguelike_core::map::{Surface, AoeEffect};
use roguelike_core::messaging::Msg;
use roguelike_core::constants::*;
use roguelike_core::animation::{Effect, Animation};
use roguelike_core::movement::{MoveMode, MoveType};
use roguelike_core::config::*;

use crate::game::*;
use crate::read_map::read_map_xp;


pub fn resolve_messages(game: &mut Game) {
    /* Handle Message Log */
    while let Some(msg) = game.msg_log.pop() {
        let msg_line = msg.msg_line(&game.data);
        if msg_line.len() > 0 {
            println!("msg: {}", msg_line);
        }

        match msg {
            Msg::Crushed(object_id, pos, _obj_type) => {
                game.data.map[pos].surface = Surface::Rubble;

                game.msg_log.log_front(Msg::Sound(object_id, pos, game.config.sound_radius_crushed, true));
            }

            Msg::Sound(cause_id, source_pos, radius, _should_animate) => {
                let sound_aoe =
                    game.data.map.aoe_fill(AoeEffect::Sound, source_pos, radius);

                let who_heard =
                    game.data.within_aoe(&sound_aoe);

                for obj_id in who_heard {
                    if obj_id != cause_id {
                        // TODO replace with an Alerted message
                        game.data.objects[obj_id].messages.push(Message::Sound(cause_id, source_pos));
                    }
                }
            }

            Msg::ItemThrow(thrower, item_id, start, end) => {
                // NOTE the radius here is the stone radius, regardless of item type

                game.msg_log.log_front(Msg::Sound(thrower, end, SOUND_RADIUS_STONE, false));
            }

            Msg::Moved(object_id, movement, pos) => {
                let player_handle = game.data.find_player().unwrap();
                if object_id == player_handle {

                    if matches!(movement.typ, MoveType::Pass) {
                        if game.data.objects[player_handle].move_mode.unwrap() ==
                           MoveMode::Run {
                            let player = &mut game.data.objects[player_handle];
                            player.move_mode = player.move_mode.map(|mode| mode.decrease());
                        }
                        // this is just to pattern match on movement
                    } else { // monster moved
                        let mut sound_radius;
                        match game.data.objects[player_handle].move_mode.unwrap() {
                            MoveMode::Sneak => sound_radius = SOUND_RADIUS_SNEAK,
                            MoveMode::Walk => sound_radius = SOUND_RADIUS_WALK,
                            MoveMode::Run => sound_radius = SOUND_RADIUS_RUN,
                        }

                        if game.data.map[pos].surface == Surface::Rubble {
                            sound_radius += game.config.sound_rubble_radius;
                        } else if game.data.map[pos].surface == Surface::Grass {
                            sound_radius -= game.config.sound_grass_radius;
                        }

                        game.msg_log.log_front(Msg::Sound(object_id, pos, sound_radius, true));
                    }
                } else {
                    game.msg_log.log_front(Msg::Sound(object_id, pos, SOUND_RADIUS_MONSTER_MOVE, true));
                }
            }

            Msg::Yell(pos) => {
                // NOTE this assumes that only the player yells
                let player_handle = game.data.find_player().unwrap();

                game.msg_log.log_front(Msg::Sound(player_handle, pos, game.config.player_yell_radius, true));
            }

            Msg::Killed(_attacker, attacked, _damage) => {
                if game.data.objects[attacked].typ != ObjType::Player {
                    let pos = game.data.objects[attacked].pos();

                    game.data.map[pos].surface = Surface::Rubble;
                }

                game.data.objects[attacked].needs_removal = true;
            }

            Msg::Attack(attacker, attacked, _damage) => {
                let pos = game.data.objects[attacked].pos();
                game.msg_log.log_front(Msg::Sound(attacker, pos, game.config.sound_radius_attack, true)); 
            }

            _ => {
            }
        }
    }

    /* Process Player Messages */
    let player_handle = game.data.find_player().unwrap();
    for message in game.data.objects[player_handle].messages.iter() {
        if let Message::Sound(obj_id, pos) = message {
            if *obj_id == player_handle {
                panic!("Player sent themselves a message?")
            }

            let player_pos = game.data.objects[player_handle].pos();
            if !game.data.map.is_in_fov(player_pos, *pos, game.config.fov_radius_player) {
                let heard = Effect::HeardSomething(*pos, game.settings.turn_count);
                game.display_state.effects.push(heard);
            }
        }
    }
    game.data.objects[player_handle].messages.clear();
}

