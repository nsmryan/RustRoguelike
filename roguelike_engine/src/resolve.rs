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
    while let Some(msg) = game.msg_log.messages.pop_front() {
        let msg_line = msg.msg_line(&game.data);
        if msg_line.len() > 0 {
            println!("msg: {}", msg_line);
        }

        match msg {
            Msg::Crushed(object_id, pos, _obj_type) => {
                game.data.map[pos].surface = Surface::Rubble;

                game.msg_log.log_front(Msg::Sound(object_id, pos, game.config.sound_radius_crushed, true));
            }

            Msg::Sound(cause_id, source_pos, radius, should_animate) => {
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

                if should_animate {
                    let sound_effect = Effect::Sound(sound_aoe, 0.0);
                    game.display_state.play_effect(sound_effect);
                }
            }

            Msg::ItemThrow(thrower, item_id, start, end) => {
                // NOTE the radius here is the stone radius, regardless of item type

                game.msg_log.log_front(Msg::Sound(thrower, end, SOUND_RADIUS_STONE, false));

                let sound_aoe = game.data.map.aoe_fill(AoeEffect::Sound, end, SOUND_RADIUS_STONE);

                let chr = game.data.objects[item_id].chr;
                let item_sprite =
                    game.display_state.font_sprite(chr)
                        .expect("Could not find item sprite!");

                let move_anim = Animation::Between(item_sprite, start, end, 0.0, game.config.item_throw_speed);
                let item_anim = Animation::PlayEffect(Effect::Sound(sound_aoe, 0.0));
                let loop_anim = Animation::Loop(item_sprite);

                let move_key = game.display_state.play_animation(move_anim);
                let item_key = game.display_state.play_animation(item_anim);
                let loop_key = game.display_state.play_animation(loop_anim);

                game.data.objects[item_id].animation.clear();
                game.data.objects[item_id].animation.push_back(move_key);
                game.data.objects[item_id].animation.push_back(item_key);
                game.data.objects[item_id].animation.push_back(loop_key);
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

                        let idle_sprite =
                            game.display_state.new_sprite("player_idle".to_string(), game.config.idle_speed)
                                              .unwrap();
                        let idle_anim = Animation::Loop(idle_sprite);
                        let idle_key = game.display_state.play_animation(idle_anim);

                        game.data.objects[player_handle].animation.clear();
                        game.data.objects[player_handle].animation.push_back(idle_key);

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
                    game.data.objects[attacked].animation.clear();

                    let sprite_name = format!("{}_die", game.data.objects[attacked].name);
                    let maybe_sprite = game.display_state.new_sprite(sprite_name, 1.0);
                    if let Some(sprite) = maybe_sprite {
                        let anim = game.display_state.play_animation(Animation::Once(sprite));
                        game.data.objects[attacked].animation.clear();
                        game.data.objects[attacked].animation.push_front(anim);
                    }

                    let pos = game.data.objects[attacked].pos();

                    game.data.map[pos].surface = Surface::Rubble;
                }

                game.data.objects[attacked].needs_removal = true;
            }

            Msg::Attack(attacker, attacked, _damage) => {
                if game.data.objects[attacker].typ == ObjType::Player {
                    let attack_sprite =
                        game.display_state.new_sprite("player_attack".to_string(), game.config.player_attack_speed)
                                          .unwrap();
                    let attack_anim = Animation::Once(attack_sprite);
                    let attack_key = game.display_state.play_animation(attack_anim);

                    let idle_sprite =
                        game.display_state.new_sprite("player_idle".to_string(), game.config.idle_speed)
                                          .unwrap();
                    let idle_anim = Animation::Loop(idle_sprite);
                    let idle_key = game.display_state.play_animation(idle_anim);

                    game.data.objects[attacker].animation.clear();
                    game.data.objects[attacker].animation.push_back(attack_key);
                    game.data.objects[attacker].animation.push_back(idle_key);
                }

                let pos = game.data.objects[attacked].pos();
                game.msg_log.log_front(Msg::Sound(attacker, pos, game.config.sound_radius_attack, true)); 
            }

            Msg::JumpWall(jumper, start, end) => {
                if game.data.objects[jumper].typ == ObjType::Player {
                    let jump_sprite =
                        game.display_state.new_sprite("player_vault".to_string(), game.config.player_vault_sprite_speed)
                                          .unwrap();
                    let jump_anim = Animation::Between(jump_sprite, start, end, 0.0, game.config.player_vault_move_speed);
                    let jump_key = game.display_state.play_animation(jump_anim);

                    let idle_sprite =
                        game.display_state.new_sprite("player_idle".to_string(), game.config.idle_speed)
                                          .unwrap();
                    let idle_anim = Animation::Loop(idle_sprite);
                    let idle_key = game.display_state.play_animation(idle_anim);

                    game.data.objects[jumper].animation.clear();
                    game.data.objects[jumper].animation.push_back(jump_key);
                    game.data.objects[jumper].animation.push_back(idle_key);
                }
            }

            Msg::SpikeTrapTriggered(_trap, _entity) => {
            }

            Msg::SpawnedObject(entity_id) => {
                let obj_id = game.data.find_entity(entity_id).unwrap();

                if game.data.objects[obj_id].typ == ObjType::Player {
                    let sprite = game.display_state.new_sprite("player_idle".to_string(), game.config.idle_speed)
                                                    .expect("Could not find sprite 'player_idle'");

                    let anim_key = game.display_state.play_animation(Animation::Loop(sprite));

                    game.data.objects[obj_id].animation.push_front(anim_key);
                } else if game.data.objects[obj_id].name == "key" {
                    let sprite = game.display_state.new_sprite("key".to_string(), game.config.key_speed)
                                                     .expect("Could not find sprite 'key'");

                    let anim_key = game.display_state.play_animation(Animation::Loop(sprite));

                    game.data.objects[obj_id].animation.push_front(anim_key);

                }
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

    /* Remove objects are awaiting removal */
    {
        let mut removals = Vec::new();
        for key in game.data.objects.keys() {
            if game.data.objects[key].needs_removal &&
               game.data.objects[key].animation.len() == 0 {
                   removals.push(key);
               }
        }

        for key in removals {
           game.data.objects.remove(key);
        }
    }
}

