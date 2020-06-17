use log::{trace, error};

use roguelike_core::types::*;
use roguelike_core::ai::{Behavior};
use roguelike_core::map::{Surface, AoeEffect};
use roguelike_core::messaging::{MsgLog, Msg};
use roguelike_core::constants::*;
use roguelike_core::movement::{MoveMode, MoveType, Action, Attack};
use roguelike_core::config::*;
use roguelike_core::utils::*;

use crate::game::*;
use crate::actions::{throw_item, pick_item_up};


pub fn resolve_messages(data: &mut GameData, msg_log: &mut MsgLog, _settings: &mut GameSettings, config: &Config) {
    /* Handle Message Log */
    while let Some(msg) = msg_log.pop() {
        let msg_line = msg.msg_line(data);
        if msg_line.len() > 0 {
            println!("msg: {}", msg_line);
        }

        match msg {
            Msg::Crushed(entity_id, pos, _obj_type) => {
                data.map[pos].surface = Surface::Rubble;

                msg_log.log_front(Msg::Sound(entity_id, pos, config.sound_radius_crushed, true));
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

            Msg::ItemThrow(entity_id, item_id, start, end) => {
                throw_item(entity_id, item_id, start, end, data, msg_log);

                // NOTE the radius here is the stone radius, regardless of item type
                msg_log.log_front(Msg::Sound(entity_id, end, SOUND_RADIUS_STONE, false));
            }

            Msg::JumpWall(entity_id, _start, end) => {
                    msg_log.log_front(Msg::Sound(entity_id, end, SOUND_RADIUS_RUN, true));
            }

            Msg::Moved(entity_id, movement, pos) => {
                // if running, but didn't move any squares, then decrease speed
                if matches!(movement.typ, MoveType::Pass) {
                    if matches!(data.entities.move_mode.get(&entity_id), Some(MoveMode::Run)) {
                        data.entities.move_mode[&entity_id].decrease();
                    }
                }

                // make a noise based on how fast the entity is moving and the terrain
                if let Some(move_mode) = data.entities.move_mode.get(&entity_id) {
                    let mut sound_radius;

                    match move_mode {
                        MoveMode::Sneak => sound_radius = SOUND_RADIUS_SNEAK,
                        MoveMode::Walk => sound_radius = SOUND_RADIUS_WALK,
                        MoveMode::Run => sound_radius = SOUND_RADIUS_RUN,
                    }

                    if data.map[pos].surface == Surface::Rubble {
                        sound_radius += config.sound_rubble_radius;
                    } else if data.map[pos].surface == Surface::Grass {
                        sound_radius -= config.sound_grass_radius;
                    }

                    msg_log.log_front(Msg::Sound(entity_id, pos, sound_radius, true));
                } else {
                    msg_log.log_front(Msg::Sound(entity_id, pos, SOUND_RADIUS_MONSTER_MOVE, true));
                }

                // get a list of triggered traps
                let mut traps: Vec<EntityId> = Vec::new();
                for key in data.entities.ids.iter() {
                    if data.entities.trap.get(key).is_some()          && // key is a trap
                       data.entities.alive[&entity_id]                 && // entity is alive
                       data.entities.fighter.get(&entity_id).is_some() && // entity is a fighter
                       data.entities.pos[key] == data.entities.pos[&entity_id] {
                        traps.push(*key);
                    }
                }

                // Check if the entity hit a trap
                for trap in traps.iter() {
                    match data.entities.trap[trap] {
                        Trap::Spikes => {
                            msg_log.log(Msg::SpikeTrapTriggered(*trap, entity_id));
                            data.entities.needs_removal[trap] = true;
                        }

                        Trap::Sound => {
                            msg_log.log(Msg::SoundTrapTriggered(*trap, entity_id));
                            data.entities.needs_removal[trap] = true;
                        }
                    }
                }
            }

            Msg::Yell(entity_id, pos) => {
                msg_log.log_front(Msg::Sound(entity_id, pos, config.yell_radius, true));
            }

            Msg::Killed(_attacker, attacked, _damage) => {
                if data.entities.typ[&attacked] != EntityType::Player {
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

                if let Some(hit_entity) = data.has_blocking_entity(pos) {
                    // we hit another entity!
                    msg_log.log_front(Msg::HammerHitEntity(entity, hit_entity));
                } else if let Some(blocked) = data.map.is_blocked_by_wall(entity_pos, pos_diff.x, pos_diff.y) {
                    msg_log.log_front(Msg::HammerHitWall(entity, blocked));
                }
            }

            // TODO Consider making this a Push message, splitting out that code from Action as well
            Msg::HammerHitEntity(entity, hit_entity) => {
                dbg!(entity, hit_entity);
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

            Msg::Action(entity_id, action) => {
                let entity_pos = data.entities.pos[&entity_id];

                // TODO add remaining variants, and move AI state change to here.
                // likely need to split this into a separate function, as it is getting long
                if let Action::Move(movement) = action {
                    if let Some(attack_field) = movement.attack {
                        match attack_field {
                            Attack::Attack(target_id) => {
                                attack(entity_id, target_id, data, msg_log);
                            }

                            Attack::Stab(target_id) => {
                                stab(entity_id, target_id, &mut data.entities, msg_log);

                                if data.using(entity_id, Item::Dagger) {
                                    data.used_up_item(entity_id);
                                }

                                if entity_pos != movement.pos {
                                    data.entities.move_to(entity_id, movement.pos);
                                    msg_log.log(Msg::Moved(entity_id, movement, movement.pos));
                                }
                            }

                            Attack::Push(target_id, delta_pos) => {
                                if data.entities.typ[&target_id] == EntityType::Column {
                                    let pos = data.entities.pos[&entity_id];
                                        let next_pos = next_pos(pos, sub_pos(movement.pos, pos));

                                    // if there is a path to the next tile, move it.
                                    let diff = sub_pos(movement.pos, pos);
                                    let blocked =
                                        data.map.is_blocked_by_wall(movement.pos, diff.x, diff.y); 

                                    if blocked == None {
                                        data.entities.move_to(entity_id, movement.pos);

                                        data.remove_entity(target_id);

                                        if let Some(hit_entity) = data.has_blocking_entity(next_pos) {
                                            crush(target_id, hit_entity, &mut data.entities, msg_log);
                                        }

                                        msg_log.log(Msg::Crushed(entity_id, next_pos, EntityType::Column));
                                    }
                                } else if data.entities.alive[&target_id] {
                                    push_attack(entity_id, target_id, delta_pos, true, data, msg_log);
                                } else {
                                    error!("Tried to push entity {:?}, which was not valid!", data.entities.typ[&target_id]);
                                    panic!("What did you push? Check the game log!");
                                }
                            }
                        }

                        // TODO this is causing monsters to move into the player
                        // set pos in case we moved in order to attack
                        //data.entities.set_pos(entity_id, movement.pos);
                    } else if movement.attack.is_none() {
                        match movement.typ {
                            MoveType::Collide => {
                                data.entities.move_to(entity_id, movement.pos);

                                msg_log.log(Msg::Collided(entity_id, movement.pos));
                            }

                            MoveType::Pass => {
                                msg_log.log(Msg::Moved(entity_id, movement, movement.pos));
                            }

                            MoveType::WallKick(_dir_x, _dir_y) => {
                                data.entities.move_to(entity_id, movement.pos);

                                // TODO could check for enemy and attack
                                msg_log.log(Msg::WallKick(entity_id, movement.pos));
                            }

                            MoveType::Move | MoveType::JumpWall => {
                                // TODO what about if the entity is moved (say, pushed)? shouldn't
                                // do the move at all, likely
                                if entity_pos != movement.pos {
                                    if data.clear_path(entity_pos, movement.pos) {
                                        data.entities.move_to(entity_id, movement.pos);

                                        if movement.typ == MoveType::Move {
                                            msg_log.log(Msg::Moved(entity_id, movement, movement.pos));
                                        } else {
                                            msg_log.log(Msg::JumpWall(entity_id, entity_pos, movement.pos));
                                        }
                                    } else {
                                        // TODO move towards position, perhaps emitting a Collide
                                        // message.
                                    }
                                }
                                // else the movement does not change our position, so do nothing
                            }
                        }

                        // if entity is attacking, face their target after the move
                        if let Some(Behavior::Attacking(target_id)) = data.entities.behavior.get(&entity_id) {
                            let target_pos = data.entities.pos[target_id];
                            data.entities.face(entity_id, target_pos);
                        }
                    }
                } else if let Action::StateChange(behavior) = action {
                    msg_log.log(Msg::StateChange(entity_id, behavior));
                } else if let Action::Yell = action {
                    msg_log.log(Msg::Yell(entity_id, entity_pos));
                } else if let Action::Pass = action {
                    msg_log.log(Msg::Pass());
                } else if let Action::ThrowItem(throw_pos, item_id) = action {
                    msg_log.log(Msg::ItemThrow(entity_id, item_id, entity_pos, throw_pos));
                } else if let Action::Pickup(item_id) = action {
                    msg_log.log(Msg::PickedUp(entity_id, item_id));
                } else if let Action::UseItem(pos) = action {
                    let holding_hammer = data.using(entity_id, Item::Hammer);

                    if holding_hammer {
                        msg_log.log(Msg::HammerSwing(entity_id, pos));
                    }
                }
            }

            Msg::PickedUp(entity_id, item_id) => {
                pick_item_up(entity_id, item_id, &mut data.entities);
            }

            Msg::StateChange(entity_id, behavior) => {
                data.entities.behavior[&entity_id] = behavior;
            }

            Msg::SpikeTrapTriggered(trap, entity_id) => {
                data.entities.take_damage(entity_id, SPIKE_DAMAGE);

                if data.entities.fighter[&entity_id].hp <= 0 {
                    data.entities.alive[&entity_id] = false;
                    data.entities.blocks[&entity_id] = false;

                    msg_log.log(Msg::Killed(trap, entity_id, SPIKE_DAMAGE));
                }
            }

            Msg::SoundTrapTriggered(trap, entity) => {
                let source_pos = data.entities.pos[&trap];

                // the triggering entity is considered the source of the sound
                Msg::Sound(entity, source_pos, config.sound_radius_trap, true);
            }

            _ => {
            }
        }
    }

    /* Process Player Messages */
    let player_id = data.find_player().unwrap();
    for message in data.entities.messages[&player_id].iter() {
        if let Message::Sound(obj_id, pos) = message {
            if *obj_id == player_id {
                panic!("Player sent themselves a message?")
            }

            let player_pos = data.entities.pos[&player_id];
            if !data.map.is_in_fov(player_pos, *pos, config.fov_radius_player) {
                //let heard = Effect::HeardSomething(*pos, settings.turn_count);
                // TODO move to somewhere else?
                //display_state.effects.push(heard);
            }
        }
    }
    data.entities.messages[&player_id].clear();
}

