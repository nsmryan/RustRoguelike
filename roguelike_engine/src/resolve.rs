#[allow(unused_imports)]

use roguelike_core::types::*;
use roguelike_core::ai::{Behavior, ai_move_to_attack_pos, ai_can_hit_target, ai_take_turn, ai_is_in_fov};
use roguelike_core::map::{Surface, AoeEffect};
use roguelike_core::messaging::{MsgLog, Msg};
use roguelike_core::constants::*;
use roguelike_core::movement::{MoveMode, MoveType, Attack, Movement, Direction};
use roguelike_core::movement;
use roguelike_core::config::*;
use roguelike_core::utils::*;
use roguelike_core::map::*;
use roguelike_core::line::*;
use roguelike_core::rng::Rand32;

use crate::generation::{make_energy, make_light, ensure_grass};
use crate::game::Game;
use crate::make_map::{make_map};


pub fn resolve_messages(game: &mut Game) {
    let player_id = game.data.find_by_name(EntityName::Player).unwrap();

    /* Handle Message Log */
    while let Some(msg) = game.msg_log.pop() {
        match msg {
            Msg::Moved(entity_id, move_type, pos) => {
               process_moved_message(entity_id, move_type, pos, &mut game.data, &mut game.msg_log, &mut game.rng, &game.config);
            }

            Msg::Interact(entity_id, pos) => {
               process_interaction(entity_id, pos, &mut game.data, &mut game.msg_log, &game.config);
            }

            Msg::Crushed(entity_id, pos) => {
                crushed(entity_id, pos, &mut game.data, &mut game.msg_log, &game.config);
            }

            Msg::Sound(cause_id, source_pos, radius, _should_animate) => {
                let sound_aoe =
                    aoe_fill(&game.data.map, AoeEffect::Sound, source_pos, radius, &game.config);

                let who_heard =
                    game.data.within_aoe(&sound_aoe);

                for obj_id in who_heard {
                    if obj_id != cause_id {
                        // TODO replace with an Alerted message
                        game.data.entities.messages[&obj_id].push(Message::Sound(cause_id, source_pos));
                    }
                }
            }

            Msg::ItemThrow(entity_id, item_id, start, end) => {
                if start == end {
                    // TODO make this drop an item
                    //inventory_drop_item(entity_id, item_index as usize, data, &mut game.msg_log);
                } else {
                    throw_item(entity_id, item_id, start, end, &mut game.data, &mut game.msg_log, &game.config);
                }
            }

            Msg::JumpWall(entity_id, _start, end) => {
                game.msg_log.log_front(Msg::Sound(entity_id, end, game.config.sound_radius_run, true));
            }


            Msg::Blink(entity_id) => {
                if use_energy(entity_id, &mut game.data, &mut game.msg_log) {
                    resolve_blink(entity_id, &mut game.data, &mut game.rng, &mut game.msg_log);
                }
            }

            Msg::Pushed(pusher, pushed, direction, push_amount, move_into) => {
                pushed_entity(pusher, pushed, direction, push_amount, move_into, &mut game.data, &game.config, &mut game.msg_log);
            }

            Msg::Yell(entity_id) => {
                let pos = game.data.entities.pos[&entity_id];
                game.msg_log.log_front(Msg::Sound(entity_id, pos, game.config.yell_radius, true));
                game.data.entities.took_turn[&entity_id] = true;
            }

            Msg::Remove(entity_id) => {
                remove_entity(entity_id, &mut game.data);
            }

            Msg::Killed(_attacker, attacked, _damage) => {
                killed_entity(attacked, &mut game.data, &mut game.msg_log, &game.config);
            }

            Msg::Attack(attacker, attacked, _damage) => {
                // TODO move attack function here, and remove push Msg::Attack in attack function
                let pos = game.data.entities.pos[&attacked];
                game.msg_log.log_front(Msg::Sound(attacker, pos, game.config.sound_radius_attack, true)); 
            }

            Msg::Stabbed(_attacker_id, _attacked_id) => {
                // TODO this may be superceded by Hit, although perhaps Hit
                // should break out into finer grain attacks.
                //msg_log.log(Msg::Froze(attacked_id, game.config.dagger_stab_num_turns));
            }

            Msg::HammerRaise(entity_id, item_index, dir) => {
                let item_id = game.data.entities.inventory[&entity_id][item_index];
                game.data.entities.status[&entity_id].hammer_raised = Some((item_id, dir, 1));
                game.data.entities.took_turn[&entity_id] = true;
            }

            Msg::HammerSwing(entity_id, item_id, pos) => {
                hammer_swing(entity_id, item_id, pos, &mut game.data, &mut game.msg_log);
            }

            // TODO Consider making this a Push message, splitting out that code from Action as well
            Msg::HammerHitEntity(entity_id, hit_entity) => {
                hammer_hit_entity(entity_id, hit_entity, &mut game.data, &mut game.msg_log, &game.config);
            }

            Msg::HammerHitWall(entity_id, blocked) => {
                hammer_hit_wall(entity_id, blocked, &mut game.data, &mut game.msg_log, &game.config);
            }

            Msg::TryAttack(entity_id, attack_info, attack_pos) => {
                resolve_attack(entity_id, attack_info, attack_pos, &mut game.data, &mut game.msg_log, &game.config);
            }

            Msg::TryMove(entity_id, direction, amount, move_mode) => {
                resolve_try_move(entity_id, direction, amount, move_mode, &mut game.data, &mut game.msg_log);
            }

            Msg::PickUp(entity_id) => {
                pick_item_up(entity_id, &mut game.data, &mut game.msg_log);
            }

            Msg::StateChange(entity_id, behavior) => {
                game.data.entities.behavior[&entity_id] = behavior;

                // if the entity hasn't completed a turn, the state change continues their turn.
                // NOTE this might be better off as a message! emit it every time a state change
                // occurs?
                if !game.data.entities.took_turn[&entity_id] {
                   ai_take_turn(entity_id, &mut game.data, &game.config, &mut game.msg_log);
                }
            }

            Msg::SpikeTrapTriggered(trap, entity_id) => {
                game.data.entities.take_damage(entity_id, SPIKE_DAMAGE);

                if game.data.entities.hp[&entity_id].hp <= 0 {
                    game.data.entities.status[&entity_id].alive = false;
                    game.data.entities.blocks[&entity_id] = false;

                    game.msg_log.log(Msg::Killed(trap, entity_id, SPIKE_DAMAGE));
                }
            }

            Msg::SoundTrapTriggered(trap, entity_id) => {
                let source_pos = game.data.entities.pos[&trap];

                // the triggering entity is considered the source of the sound
                game.msg_log.log(Msg::Sound(entity_id, source_pos, game.config.sound_radius_trap, true));
            }

            Msg::BlinkTrapTriggered(trap, entity_id) => {
                let source_pos = game.data.entities.pos[&trap];

                if let Some(blink_pos) = find_blink_pos(source_pos, &mut game.rng, &mut game.data) {
                    game.data.entities.set_pos(entity_id, blink_pos);
                    game.data.entities.status[&entity_id].blinked = true;
                }
            }

            Msg::Froze(entity_id, num_turns) => {
                if entity_id == player_id || game.data.entities.ai.get(&entity_id).is_some() {
                    game.data.entities.status[&entity_id].frozen = num_turns;
                    game.data.entities.behavior[&entity_id] = Behavior::Idle;
                }
            }

            Msg::FreezeTrapTriggered(trap, cause_id) => {
                freeze_trap_triggered(trap, cause_id, &mut game.data, &mut game.msg_log, &game.config);
            }

            Msg::Untriggered(_trigger, _entity_id) => {
                // NOTE nothing untriggers yet
                //untriggered(trigger, data, &mut game.msg_log);
            }

            Msg::Triggered(trigger, _entity_id) => {
                triggered(trigger, &mut game.data);
            }

            Msg::AddClass(class) => {
                game.data.entities.skills[&player_id].clear();

                match class {
                    EntityClass::General => {
                        game.data.entities.class[&player_id] = class;
                        game.data.entities.add_skill(player_id, Skill::Blink);
                        game.data.entities.add_skill(player_id, Skill::Sprint);
                    }

                    EntityClass::Monolith => {
                        game.data.entities.class[&player_id] = class;
                        game.data.entities.add_skill(player_id, Skill::PassWall);
                        game.data.entities.add_skill(player_id, Skill::Rubble);
                        game.data.entities.add_skill(player_id, Skill::StoneThrow);
                        game.data.entities.add_skill(player_id, Skill::Reform);
                        game.data.entities.add_skill(player_id, Skill::StoneSkin);
                    }

                    EntityClass::Grass => {
                        game.data.entities.class[&player_id] = class;
                        game.data.entities.add_skill(player_id, Skill::GrassThrow);
                        game.data.entities.add_skill(player_id, Skill::GrassBlade);
                        game.data.entities.add_skill(player_id, Skill::GrassShoes);
                    }

                    EntityClass::Clockwork => {
                        game.data.entities.class[&player_id] = class;
                        game.data.entities.add_skill(player_id, Skill::Push);
                    }

                    EntityClass::Hierophant => {
                        game.data.entities.class[&player_id] = class;
                        game.data.entities.add_skill(player_id, Skill::Illuminate);
                        game.data.entities.add_skill(player_id, Skill::Heal);
                        game.data.entities.add_skill(player_id, Skill::FarSight);
                        game.data.entities.add_skill(player_id, Skill::Ping);
                    }

                    EntityClass::Wind => {
                        game.data.entities.add_skill(player_id, Skill::PassThrough);
                        game.data.entities.add_skill(player_id, Skill::WhirlWind);
                        game.data.entities.add_skill(player_id, Skill::Swift);
                    }
                }
            }

            Msg::MoveMode(entity_id, new_move_mode) => {
                game.data.entities.move_mode[&entity_id] = new_move_mode;

                // update entities movement reach with their new move mode
                game.data.entities.movement[&entity_id] = reach_by_mode(game.data.entities.move_mode[&entity_id]);
            }

            Msg::Hit(entity_id, pos, weapon_type, attack_style) => {
                process_hit(entity_id, pos, weapon_type, attack_style, &mut game.data, &mut game.msg_log, &game.config);
            }

            Msg::ChangeMoveMode(entity_id, increase) => {
                change_move_mode(entity_id, increase, &mut game.data, &mut game.msg_log);
            }

            Msg::DropItem(entity_id, item_index) => {
                inventory_drop_item(entity_id, item_index as usize, &mut game.data, &mut game.msg_log);
            }

            Msg::GrassThrow(entity_id, direction) => {
                if use_energy(entity_id, &mut game.data, &mut game.msg_log) {
                    let pos = game.data.entities.pos[&entity_id];

                    for grass_pos in line_inclusive(pos, direction.offset_pos(pos, SKILL_GRASS_THROW_RADIUS as i32)) {

                        // NOTE percent chance of not marking a tile
                        // NOTE percent chance of marking a nearby tile
                        if rng_trial(&mut game.rng, 0.75) {
                            if game.data.map.is_within_bounds(grass_pos) && game.data.map[grass_pos].tile_type == TileType::Empty {
                                game.data.map[grass_pos].surface = Surface::Grass;
                                ensure_grass(&mut game.data.entities, grass_pos, &mut game.msg_log);
                            }
                        }

                        if rng_trial(&mut game.rng, 0.35) {
                            let other_pos;
                            if rng_trial(&mut game.rng, 0.5) {
                                other_pos = direction.clockwise().clockwise().offset_pos(grass_pos, 1);
                            } else {
                                other_pos = direction.counterclockwise().counterclockwise().offset_pos(grass_pos, 1);
                            }

                            if game.data.map.is_within_bounds(other_pos) && game.data.map[other_pos].tile_type == TileType::Empty {
                                game.data.map[other_pos].surface = Surface::Grass;
                                ensure_grass(&mut game.data.entities, other_pos, &mut game.msg_log);
                            }
                        }
                    }
                    // NOTE old cone style
                    //for grass_pos in Cone::new(pos, direction, SKILL_GRASS_THROW_RADIUS as i32) {
                    //    if game.data.map.is_within_bounds(grass_pos) && game.data.map[grass_pos].tile_type == TileType::Empty {
                    //        game.data.map[grass_pos].surface = Surface::Grass;
                    //        ensure_grass(&mut game.data.entities, grass_pos, &mut game.msg_log);
                    //    }
                    //}
                    game.data.entities.took_turn[&entity_id] = true;
                }
            }

            Msg::GrassShoes(entity_id, _action_mode) => {
                if use_energy(entity_id, &mut game.data, &mut game.msg_log) {
                    game.data.entities.status[&entity_id].soft_steps = SKILL_GRASS_SHOES_TURNS;
                    game.data.entities.took_turn[&entity_id] = true;
                }
            }

            Msg::Illuminate(entity_id, pos, amount) => {
                if use_energy(entity_id, &mut game.data, &mut game.msg_log) {
                    let light = make_light(&mut game.data.entities, &game.config, pos, &mut game.msg_log);
                    game.data.entities.illuminate[&light] = amount;

                    game.data.entities.took_turn[&entity_id] = true;
                }
            }

            Msg::Heal(entity_id, amount) => {
                if use_energy(entity_id, &mut game.data, &mut game.msg_log) {
                    game.data.entities.hp[&entity_id].hp = 
                        std::cmp::min(game.data.entities.hp[&entity_id].max_hp,
                                      game.data.entities.hp[&entity_id].hp + amount as i32);

                    game.data.entities.took_turn[&entity_id] = true;
                }
            }

            Msg::FarSight(entity_id, amount) => {
                if use_energy(entity_id, &mut game.data, &mut game.msg_log) {
                    game.data.entities.status[&entity_id].extra_fov += amount;
                    game.data.entities.took_turn[&entity_id] = true;
                }
            }

            Msg::Ping(entity_id, pos) => {
                if use_energy(entity_id, &mut game.data, &mut game.msg_log) {
                    game.msg_log.log_front(Msg::Sound(entity_id, pos, game.config.ping_sound_radius, true));
                }
            }

            Msg::Sprint(entity_id, direction, amount) => {
                if use_energy(entity_id, &mut game.data, &mut game.msg_log) {
                    game.msg_log.log(Msg::TryMove(entity_id, direction, amount, MoveMode::Run));
                    game.data.entities.took_turn[&entity_id] = true;
                }
            }

            Msg::Rubble(entity_id, rubble_pos) => {
                if use_energy(entity_id, &mut game.data, &mut game.msg_log) {
                    let pos = game.data.entities.pos[&entity_id];
                    let blocked = game.data.map.path_blocked_move(pos, rubble_pos);

                    if let Some(blocked) = blocked {
                        // if we hit a column, turn it into rubble
                        if let Some(blocked_id) = game.data.has_blocking_entity(blocked.end_pos) {
                            if game.data.entities.name[&blocked_id] == EntityName::Column {
                                remove_entity(blocked_id, &mut game.data);
                                game.data.map[blocked.end_pos].surface = Surface::Rubble;
                            }
                        } else {
                            resolve_rubble(blocked, &mut game.data.map);
                            game.data.entities.took_turn[&entity_id] = true;
                        }
                    }
                }
            }

            Msg::StoneThrow(entity_id, target_pos) => {
                let entity_pos = game.data.entities.pos[&entity_id];

                let mut rubble_pos = None;
                if game.data.map[entity_pos].surface == Surface::Rubble {
                    rubble_pos = Some(entity_pos);
                }

                for pos in game.data.map.neighbors(entity_pos) {
                    if game.data.map[pos].surface == Surface::Rubble {
                        rubble_pos = Some(pos);
                    }
                }

                if let Some(rubble_pos) = rubble_pos {
                    for target_id in game.data.get_entities_at_pos(target_pos) {
                        let target_pos = game.data.entities.pos[&target_id];
                        let direction = Direction::from_positions(entity_pos, target_pos).expect("The player is on the same tile as a column?");

                        if game.data.entities.typ[&target_id] == EntityType::Enemy {
                            let move_into = false;
                            push_attack(entity_id, target_id, direction, move_into, &mut game.data, &game.config, &mut game.msg_log);
                        } else if game.data.entities.typ[&target_id] == EntityType::Column {
                            game.msg_log.log(Msg::Pushed(entity_id, target_id, direction, 1, false));
                        }
                    }

                    game.data.map[rubble_pos].surface = Surface::Floor;
                }
            }

            Msg::GrassBlade(entity_id, action_mode, direction) => {
                if use_energy(entity_id, &mut game.data, &mut game.msg_log) {
                    let pos = game.data.entities.pos[&entity_id];

                    match action_mode {
                        ActionMode::Primary => {
                            // TODO anything?
                        }

                        ActionMode::Alternate => {
                            // TODO anything?
                        }
                    }

                    let attack_pos = direction.offset_pos(pos, 1);
                    let targets = game.data.get_entities_at_pos(attack_pos);

                    for target_id in targets {
                        if game.data.entities.typ[&target_id] == EntityType::Enemy {
                            let attack = Attack::Stab(target_id, false);
                            resolve_attack(entity_id, attack, attack_pos, &mut game.data, &mut game.msg_log, &game.config);

                            game.data.entities.took_turn[&entity_id] = true;
                            break;
                        }
                    }
                }
            }

            Msg::Reform(entity_id, pos) => {
                if game.data.map[pos].surface == Surface::Rubble &&
                   game.data.has_blocking_entity(pos).is_none() {
                    if use_energy(entity_id, &mut game.data, &mut game.msg_log) {
                        game.data.map[pos].surface = Surface::Floor;
                        game.data.map[pos].block_move = true;
                        game.data.map[pos].chr = MAP_WALL;
                        game.data.entities.took_turn[&entity_id] = true;
                    }
                }
            }

            Msg::StoneSkin(entity_id) => {
                game.data.entities.status[&entity_id].stone = SKILL_STONE_SKIN_TURNS;
            }

            Msg::Swap(entity_id, target_id) => {
                if use_energy(entity_id, &mut game.data, &mut game.msg_log) {

                    let entity_dir = game.data.entities.direction[&entity_id];
                    let target_dir = game.data.entities.direction[&target_id];

                    let start_pos = game.data.entities.pos[&entity_id];
                    let end_pos = game.data.entities.pos[&target_id];
                    game.data.entities.set_pos(entity_id, end_pos);
                    game.data.entities.set_pos(target_id, start_pos);

                    game.msg_log.log(Msg::SetFacing(entity_id, target_dir));
                    game.msg_log.log(Msg::SetFacing(target_id, entity_dir));

                    game.data.entities.took_turn[&entity_id] = true;
                }
            }

            Msg::PassWall(entity_id, pos) => {
                if use_energy(entity_id, &mut game.data, &mut game.msg_log) {
                    game.data.entities.set_pos(entity_id, pos);
                    game.msg_log.log(Msg::MoveMode(entity_id, MoveMode::Walk));
                    game.msg_log.log(Msg::Moved(entity_id, MoveType::Move, pos));

                    game.data.entities.took_turn[&entity_id] = true;
                }
            }

            // TODO this message isn't used anymore. remove the message and remove the
            // use_item function
            Msg::UseItem(entity_id, pos, item_id) => {
                use_item(entity_id, pos, item_id, &mut game.data, &mut game.msg_log);
            }

            Msg::ArmDisarmTrap(entity_id, trap_id) => {
                game.data.entities.armed[&trap_id] = !game.data.entities.armed[&trap_id];
                game.data.entities.took_turn[&entity_id] = true;
            }

            Msg::PlaceTrap(entity_id, place_pos, trap_id) => {
                place_trap(trap_id, place_pos, &mut game.data);
                game.data.entities.remove_from_inventory(entity_id, trap_id);
                game.data.entities.took_turn[&entity_id] = true;
            }


            Msg::Push(entity_id, direction, amount) => {
                if use_energy(entity_id, &mut game.data, &mut game.msg_log) {
                    resolve_push_skill(entity_id, direction, amount, &mut game.data, &mut game.msg_log);
                }
            }

            Msg::FaceTowards(entity_id, pos) => {
                game.data.entities.face(entity_id, pos);
                game.msg_log.log(Msg::Facing(entity_id, game.data.entities.direction[&entity_id]));
            }

            Msg::SetFacing(entity_id, direction) => {
                game.data.entities.direction[&entity_id] = direction;
                game.msg_log.log(Msg::Facing(entity_id, direction));
            }

            Msg::AiAttack(entity_id) => {
                if let Behavior::Attacking(target_id) = game.data.entities.behavior[&entity_id] {
                    resolve_ai_attack(entity_id, target_id, &mut game.data, &mut game.msg_log, &mut game.rng, &game.config);
                } else {
                    panic!("ai attacking but not in attack state!");
                }
            }

            Msg::StartUseItem(item_id) => {
                match game.data.entities.item[&item_id] {
                    Item::Dagger => {
                    }

                    Item::Shield => {
                    }

                    Item::Hammer => {
                    }

                    Item::Sword => {
                        //let mut offsets = SmallVec::new();
                        //let dir = game.data.entities.direction[&player_id] as usize;
                        //offsets[dir] = Some(ItemUse(Item::Sword, 1));
                        //data.entities.
                    }

                    _ => {
                    }
                }
            }

            Msg::Restart => {
                make_map(&game.settings.map_load_config.clone(), game);
            }

            Msg::PassThrough(entity_id, direction) => {
                let entity_pos = game.data.entities.pos[&entity_id];
                let dest = direction.offset_pos(entity_pos, 3);
                let clear_path = game.data.map.path_blocked(entity_pos, dest, BlockedType::Move).is_none();
                let blocked_pos = game.data.pos_blocked(dest);
                if  clear_path && !blocked_pos {
                    game.msg_log.log(Msg::Moved(entity_id, MoveType::Blink, dest));

                    for pos in line_inclusive(entity_pos, dest) {
                        for other_id in game.data.get_entities_at_pos(pos) {
                            if game.data.entities.typ[&other_id] == EntityType::Enemy {
                                game.msg_log.log(Msg::Forget(other_id));
                            }
                        }
                    }
                }
            }

            Msg::WhirlWind(entity_id, pos) => {
                let entity_pos = game.data.entities.pos[&entity_id];
                let mut near_walls = false;
                for dir in Direction::directions() {
                    let dir_pos = dir.offset_pos(pos, 1);

                    if !game.data.map.is_within_bounds(dir_pos) {
                        continue;
                    }

                    if game.data.map.move_blocked(pos, dir_pos, BlockedType::Move).is_some() {
                        near_walls = true;
                        break;
                    }
                }

                let traps_block = false;
                if !near_walls && game.data.clear_path(entity_pos, pos, traps_block) {
                    game.msg_log.log(Msg::Moved(entity_id, MoveType::Blink, pos));
                } // NOTE could create a failed whirlwind message, or generic failed skill message
            }

            Msg::Swift(entity_id, direction) => {
                let entity_pos = game.data.entities.pos[&entity_id];
                let dest = direction.offset_pos(entity_pos, SKILL_SWIFT_DISTANCE as i32);

                if game.data.map.is_within_bounds(dest) {

                    let mut near_walls = false;
                    for dir in Direction::directions() {
                        if game.data.map.move_blocked(dest, dir.offset_pos(dest, 1), BlockedType::Move).is_some() {
                            near_walls = true;
                            break;
                        }
                    }

                    let traps_block = false;
                    if !near_walls && game.data.clear_path(entity_pos, dest, traps_block) {
                        game.msg_log.log(Msg::Moved(entity_id, MoveType::Blink, dest));
                    }
                }
            }

            Msg::Forget(entity_id) => {
                game.msg_log.log(Msg::StateChange(entity_id, Behavior::Idle));
            }

            _ => {
            }
        }
    }

    /* Process Player Messages */
    for message in game.data.entities.messages[&player_id].iter() {
        if let Message::Sound(obj_id, _pos) = message {
            if *obj_id == player_id {
                panic!("Player sent themselves a message?")
            }

            let _player_pos = game.data.entities.pos[&player_id];

            // TODO need to add impression if not in FOV (#274)
        }
    }
    game.data.entities.messages[&player_id].clear();
}

fn hammer_swing(entity_id: EntityId, item_id: EntityId, pos: Pos, data: &mut Level, msg_log: &mut MsgLog) {
    let entity_pos = data.entities.pos[&entity_id];

    msg_log.log_front(Msg::Blunt(entity_pos, pos));

    if let Some(blocked) = data.map.path_blocked_move(entity_pos, pos) {
        msg_log.log_front(Msg::HammerHitWall(entity_id, blocked));
        data.used_up_item(entity_id, item_id);
    } else if let Some(hit_entity) = data.has_blocking_entity(pos) {
        // we hit another entity!
        msg_log.log_front(Msg::HammerHitEntity(entity_id, hit_entity));
        data.used_up_item(entity_id, item_id);
    }

    data.entities.took_turn[&entity_id] = true;
}

fn hammer_hit_entity(entity_id: EntityId, hit_entity: EntityId, data: &mut Level, msg_log: &mut MsgLog, config: &Config) {
    let first = data.entities.pos[&entity_id];
    let second = data.entities.pos[&hit_entity];

    let dxy = sub_pos(second, first);
    let direction = Direction::from_dxy(dxy.x, dxy.y).unwrap();
    let amount = 1;
    msg_log.log(Msg::Pushed(entity_id, hit_entity, direction, amount, false));
    msg_log.log_front(Msg::Sound(entity_id, second, config.sound_radius_hammer, true));

    if let Some(hp) = data.entities.hp.get(&hit_entity) {
        let damage = hp.hp;

        msg_log.log(Msg::Killed(entity_id, hit_entity, damage));
        msg_log.log(Msg::Sound(entity_id, second, config.sound_radius_blunt, true));
    }
}

fn process_hit(entity_id: EntityId, hit_pos: Pos, weapon_type: WeaponType, attack_style: AttackStyle, data: &mut Level, msg_log: &mut MsgLog, config: &Config) {
    let entity_pos = data.entities.pos[&entity_id];

    if let Some(hit_entity) = data.has_blocking_entity(hit_pos) {
        if data.entities.typ[&hit_entity] == EntityType::Column {
            // if we hit a column, and this is a strong, blunt hit, then
            // push the column over.
            if weapon_type == WeaponType::Blunt && attack_style == AttackStyle::Strong {
                let dir = Direction::from_positions(entity_pos, hit_pos).unwrap();
                msg_log.log(Msg::Pushed(entity_id, hit_entity, dir, 1, false));
            }
        } else {
            // if we hit an enemy, stun them and make a sound.
            if data.entities.typ[&hit_entity] == EntityType::Enemy {
                let mut hit_sound_radius;
                let mut stun_turns;
                match weapon_type {
                    WeaponType::Blunt => {
                        hit_sound_radius = config.sound_radius_blunt;
                        stun_turns = config.stun_turns_blunt;
                    },

                    WeaponType::Pierce => {
                        hit_sound_radius = config.sound_radius_pierce;
                        stun_turns = config.stun_turns_pierce;
                    },

                    WeaponType::Slash => {
                        hit_sound_radius = config.sound_radius_slash;
                        stun_turns = config.stun_turns_slash;
                    },
                }

                // whet stone passive adds to sharp weapon stun turns
                if data.entities.passive[&entity_id].whet_stone && weapon_type.sharp() {
                       stun_turns += 1;
                }

                if attack_style == AttackStyle::Strong {
                    hit_sound_radius += config.sound_radius_extra;
                    stun_turns += config.stun_turns_extra;
                }

                msg_log.log(Msg::Froze(hit_entity, stun_turns));
                msg_log.log(Msg::Sound(entity_id, hit_pos, hit_sound_radius, true));
            }
        }
    } else {
        // no entity- check for a wall. if blunt and strong, crush the wall.
        // TODO message for hitting a wall, use for hammer as well
    }

    match weapon_type {
        WeaponType::Blunt => {
            msg_log.log(Msg::Blunt(entity_pos, hit_pos));
        },

        WeaponType::Pierce => {
            msg_log.log(Msg::Pierce(entity_pos, hit_pos));
        },

        WeaponType::Slash => {
            msg_log.log(Msg::Slash(entity_pos, hit_pos));
        },
    }

    // TODO maybe a UsedItem message for this, although only one per use.
    //reduce_item_durability(data, entity_id, item_id);
}

fn freeze_trap_triggered(trap: EntityId, cause_id: EntityId, data: &mut Level, msg_log: &mut MsgLog, config: &Config) {
    let source_pos = data.entities.pos[&trap];

    let freeze_aoe =
        aoe_fill(&data.map, AoeEffect::Freeze, source_pos, config.freeze_trap_radius, config);

    let who_hit =
        data.within_aoe(&freeze_aoe);

    for obj_id in who_hit {
        // TODO probably need to filter out a bit more
        if obj_id != cause_id && data.entities.status[&obj_id].alive {
            msg_log.log(Msg::Froze(obj_id, FREEZE_TRAP_NUM_TURNS));
        }
    }
}

fn triggered(trigger: EntityId, data: &mut Level) {
    if data.entities.name[&trigger] == EntityName::GateTrigger {
        let wall_pos = data.entities.gate_pos[&trigger];

        if data.entities.status[&trigger].active {
            // raise the gate
            data.entities.status[&trigger].active = false;

            // only raise if no entities are on the square.
            // otherwise wait for a move that leaves the trigger unblocked.
            if data.has_entity(wall_pos).is_none() {
                data.map[wall_pos] = Tile::wall();
            }
        } else {
            data.entities.status[&trigger].active = true;
            data.map[wall_pos] = Tile::empty();
        }
    }
}

fn resolve_attack(entity_id: EntityId,
                  attack_info: Attack,
                  attack_pos: Pos,
                  data: &mut Level,
                  msg_log: &mut MsgLog,
                  _config: &Config) {
    let entity_pos = data.entities.pos[&entity_id];

    // any time an entity attacks, they change to standing stance
    data.entities.stance[&entity_id] = Stance::Standing;

    match attack_info {
        Attack::Attack(target_id) => {
            attack(entity_id, target_id, data, msg_log);
        }

        Attack::Stab(target_id, move_into) => {
            stab(entity_id, target_id, &mut data.entities, msg_log);

            if let Some(item_id) = data.using(entity_id, Item::Dagger) {
                data.used_up_item(entity_id, item_id);
            }

            if move_into && entity_pos != attack_pos {
                msg_log.log(Msg::Moved(entity_id, MoveType::Move, attack_pos));
            }

            // this is done after the Moved msg to ensure that the attack
            // animation plays instead of an idle animation
            msg_log.log(Msg::Stabbed(entity_id, target_id));
        }

        Attack::Push(target_id, direction, amount) => {
            msg_log.log(Msg::Pushed(entity_id, target_id, direction, amount, true));
        }
    }
}

fn resolve_try_move(entity_id: EntityId,
                    direction: Direction,
                    amount: usize,
                    move_mode: MoveMode,
                    data: &mut Level,
                    msg_log: &mut MsgLog) {
    // blinking uses up movement
    if data.entities.status[&entity_id].blinked {
        return;
    }

    data.entities.move_mode[&entity_id] = move_mode;

    let reach = data.entities.movement[&entity_id];
    let reach = reach.with_dist(1);

    let maybe_movement = 
        if amount == 0 {
            Some(Movement::pass(data.entities.pos[&entity_id]))
        } else {
            movement::calculate_move(direction, reach, entity_id, data)
        };

    if let Some(movement) = maybe_movement {
        if let Some(attack) = movement.attack {
            // if the movement creates an attack, attempt to perform the attack
            msg_log.log(Msg::TryAttack(entity_id, attack, movement.pos));
        } else {
            // otherwise attempt to resolve a movement
            resolve_try_movement(entity_id, direction, amount, move_mode, movement, data, msg_log);
        }
    } else {
        // monsters that are not idle, but their movement does not change their
        // position will return to idle.
        if data.entities.behavior.get(&entity_id) != None &&
           data.entities.behavior.get(&entity_id) != Some(&Behavior::Idle) {
            // this takes up the monster's turn, as they already committed to this movement
            data.entities.took_turn[&entity_id] = true;
            msg_log.log(Msg::StateChange(entity_id, Behavior::Idle));
        }
    }
}

fn resolve_try_movement(entity_id: EntityId,
                        direction: Direction,
                        amount: usize,
                        move_mode: MoveMode,
                        movement: Movement,
                        data: &mut Level,
                        msg_log: &mut MsgLog) {
    let entity_pos = data.entities.pos[&entity_id];

    match movement.typ {
        MoveType::Collide => {
            data.entities.set_pos(entity_id, movement.pos);
            msg_log.log_front(Msg::Collided(entity_id, movement.pos));
            msg_log.log_front(Msg::FaceTowards(entity_id, movement.pos));
        }

        MoveType::Pass => {
            msg_log.log_front(Msg::Moved(entity_id, MoveType::Pass, movement.pos));
        }

        MoveType::WallKick => {
            data.entities.set_pos(entity_id, movement.pos);

            // NOTE may need to set facing
            // NOTE could check for enemy and attack
            msg_log.log(Msg::WallKick(entity_id, movement.pos));
        }

        MoveType::Move | MoveType::JumpWall => {
            if entity_pos == movement.pos {
                panic!("Why did you move without changing location?");
            }

            // TODO if monster, and would hit trap, don't move
            // TODO what about if the entity is moved (say, pushed)?
            // should check for this, and no do the move at all, likely

            let traps_block = false;
            if data.clear_path(entity_pos, movement.pos, traps_block) {
                if movement.typ == MoveType::Move {
                    msg_log.log_front(Msg::Moved(entity_id, movement.typ, movement.pos));

                    if amount > 1 {
                        msg_log.log(Msg::TryMove(entity_id, direction, amount - 1, move_mode));
                    }
                } else {
                    msg_log.log(Msg::JumpWall(entity_id, entity_pos, movement.pos));
                    panic!("Can we even get here? No clear path, but didn't decide to jump a wall?");
                }
            } else if movement.typ == MoveType::JumpWall {
                // no clear path to moved position
                msg_log.log(Msg::JumpWall(entity_id, entity_pos, movement.pos));
                msg_log.log_front(Msg::Moved(entity_id, movement.typ, movement.pos));
            } else {
                panic!("Why would we not have a clear path, but have received this movement?");
                // TODO move towards position, perhaps emitting a Collide
                // message. This is likely causing the jump wall issue!
            }
        }

        MoveType::Blink => {
            if !data.pos_blocked(movement.pos) {
                msg_log.log_front(Msg::Moved(entity_id, movement.typ, movement.pos));
            }
        }
    }

    // if entity is attacking, face their target after the move
    if let Some(Behavior::Attacking(target_id)) = data.entities.behavior.get(&entity_id) {
        let target_pos = data.entities.pos[target_id];
        msg_log.log(Msg::FaceTowards(entity_id, target_pos));
    }
}

fn resolve_push_skill(entity_id: EntityId,
                      direction: Direction,
                      amount: usize,
                      data: &mut Level,
                      msg_log: &mut MsgLog) {
    let pos = data.entities.pos[&entity_id];

    let push_pos = direction.offset_pos(pos, 1);
    for other_id in data.has_entities(push_pos) {
        if data.entities.typ[&other_id] == EntityType::Enemy {
            let dxy = sub_pos(push_pos, pos);
            let direction = Direction::from_dxy(dxy.x, dxy.y).unwrap();
            let move_into = false;
            msg_log.log(Msg::Pushed(entity_id, other_id, direction, amount, move_into));
            msg_log.log(Msg::Froze(other_id, SKILL_PUSH_STUN_TURNS));
        }
    }
    data.entities.took_turn[&entity_id] = true;
}

fn resolve_blink(entity_id: EntityId, data: &mut Level, rng: &mut Rand32, msg_log: &mut MsgLog) {
    let entity_pos = data.entities.pos[&entity_id];

    if let Some(blink_pos) = find_blink_pos(entity_pos, rng, data) {
        data.entities.set_pos(entity_id, blink_pos);
    } else {
        msg_log.log(Msg::FailedBlink(entity_id));
    }

    data.entities.took_turn[&entity_id] = true;
}

fn place_rubble(pos: Pos, map: &mut Map) {
    map[pos].surface = Surface::Rubble;
    map[pos].block_move = false;
    map[pos].chr = ' ' as u8;
    map[pos].tile_type = TileType::Empty;
}

fn resolve_rubble(blocked: Blocked, map: &mut Map) {
    if map[blocked.end_pos].tile_type == TileType::Wall || 
       map[blocked.end_pos].tile_type == TileType::ShortWall {
           place_rubble(blocked.end_pos, map);
    } 

    if blocked.wall_type != Wall::Empty {
        let rev_dir = blocked.direction.reverse();
        let prev_pos = rev_dir.offset_pos(blocked.end_pos, 1);
        match blocked.direction {
            Direction::Up => {
                map[blocked.end_pos].bottom_wall = Wall::Empty;
                place_rubble(blocked.end_pos, map);
            }

            Direction::Down => {
                map[prev_pos].bottom_wall = Wall::Empty;
                place_rubble(blocked.end_pos, map);
            }

            Direction::Left => {
                map[prev_pos].left_wall = Wall::Empty;
                place_rubble(blocked.end_pos, map);
            }
            
            Direction::Right => {
                map[blocked.end_pos].left_wall = Wall::Empty;
                place_rubble(blocked.end_pos, map);
            }

            _ => {
            }

            /* TODO needs review
            Direction::UpRight => {
                if map[blocked.end_pos].bottom_wall != Wall::Empty {
                    map[blocked.end_pos].bottom_wall = Wall::Empty;
                    place_rubble(blocked.end_pos, map);
                }

                if map[blocked.end_pos].left_wall != Wall::Empty {
                    map[blocked.end_pos].left_wall = Wall::Empty;
                    place_rubble(blocked.end_pos, map);
                }
            }

            Direction::UpLeft => {
                map[blocked.end_pos].bottom_wall = Wall::Empty;
                map[move_y(prev_pos, -1)].left_wall = Wall::Empty;
            }

            Direction::DownRight => {
                map[move_x(prev_pos, 1)].bottom_wall = Wall::Empty;
                map[blocked.end_pos].left_wall = Wall::Empty;
            }

            Direction::DownLeft => {
                map[move_x(prev_pos, -1)].bottom_wall = Wall::Empty;
                map[move_y(prev_pos, 1)].left_wall = Wall::Empty;
            }
            */
        }
    }
}

fn hammer_hit_wall(entity: EntityId, blocked: Blocked, data: &mut Level, msg_log: &mut MsgLog, config: &Config) {
    let entity_pos = data.entities.pos[&entity];
    let hit_pos = blocked.end_pos;

    // if hit water, do nothing
    if data.map[hit_pos].tile_type == TileType::Water {
        return;
    }

    if data.map[hit_pos].block_move {
        // hammer hit a full tile wall
        if data.map[hit_pos].surface == Surface::Floor {
            data.map[hit_pos].surface = Surface::Rubble;
        }

        data.map[hit_pos].block_move = false;
        data.map[hit_pos].block_sight = false;
        data.map[hit_pos].tile_type = TileType::Empty;
        data.map[hit_pos].chr = ' ' as u8;

        let next_pos = next_from_to(entity_pos, hit_pos);
        msg_log.log_front(Msg::Crushed(entity, next_pos)); 
        msg_log.log_front(Msg::Sound(entity, blocked.end_pos, config.sound_radius_attack, true)); 
    } else {
        // hammer hit an inter-tile wall
        let wall_loc: Pos;
        let left_wall: bool;
        if blocked.direction == Direction::Left {
             wall_loc = blocked.start_pos;
             left_wall = true;
        } else if blocked.direction == Direction::Right {
             wall_loc = blocked.end_pos;
             left_wall = true;
         } else if blocked.direction == Direction::Down {
             wall_loc = blocked.start_pos;
             left_wall = false;
         } else if blocked.direction == Direction::Up {
             wall_loc = blocked.end_pos;
             left_wall = false;
         } else {
            dbg!(blocked);
            panic!("Hammer direction was not up/down/left/right");
         }

        if left_wall {
            data.map[wall_loc].left_wall = Wall::Empty;
        } else {
            data.map[wall_loc].bottom_wall = Wall::Empty;
        }

        msg_log.log_front(Msg::Crushed(entity, blocked.end_pos));
    }
}

fn killed_entity(attacked: EntityId, data: &mut Level, msg_log: &mut MsgLog, config: &Config) {
    let attacked_pos = data.entities.pos[&attacked];

    // if the attacked entities position is not blocked
    if !data.map[attacked_pos].block_move {
        // all non-player entities leave rubble
        if data.entities.typ[&attacked] != EntityType::Player {
            data.map[attacked_pos].surface = Surface::Rubble;
        }

        // leave energy ball
        if data.entities.typ[&attacked] == EntityType::Enemy {
            make_energy(&mut data.entities, config, attacked_pos, msg_log);
        }
    }

    if let Some(hp) = data.entities.hp.get_mut(&attacked) {
        hp.hp = 0;
    }

    remove_entity(attacked, data);
}

fn remove_entity(entity_id: EntityId, data: &mut Level) {
    data.entities.status[&entity_id].alive = false;

    data.entities.blocks[&entity_id] = false;

    data.entities.mark_for_removal(entity_id);
}

fn pushed_entity(pusher: EntityId,
                 pushed: EntityId,
                 direction: Direction,
                 push_amount: usize,
                 move_into: bool,
                 data: &mut Level,
                 config: &Config,
                 msg_log: &mut MsgLog) {
    let pushed_pos = data.entities.pos[&pushed];
    let pusher_pos = data.entities.pos[&pusher];

    if data.entities.typ[&pushed] == EntityType::Column {
        let entity_diff = sub_pos(pushed_pos, pusher_pos);
        let next_pos = next_pos(pusher_pos, entity_diff);
        let blocked = data.map.path_blocked_move(pushed_pos, next_pos); 

        if blocked == None {
            data.entities.mark_for_removal(pushed);

            msg_log.log_front(Msg::Crushed(pusher, next_pos));

            if move_into && pos_on_map(data.entities.pos[&pusher]) {
                let movement = Movement::step_to(pushed_pos);
                msg_log.log(Msg::Moved(pusher, movement.typ, pushed_pos));
            }
        }
    } else if data.entities.status[&pushed].alive {
        let continue_push = 
            push_attack(pusher, pushed, direction, move_into, data, config, msg_log);

        if continue_push && push_amount > 1 {
            msg_log.log(Msg::Pushed(pusher, pushed, direction, push_amount - 1, move_into));
        }
    } else {
        panic!("Tried to push entity {:?}, alive = {}!",
               data.entities.typ[&pushed], data.entities.status[&pushed].alive);
    }
    data.entities.took_turn[&pusher] = true;
}

fn crushed(entity_id: EntityId, pos: Pos, data: &mut Level, msg_log: &mut MsgLog, config: &Config) {

    if data.map[pos].tile_type.is_wall() {
        data.map[pos] = Tile::empty();
    }
    data.map[pos].surface = Surface::Rubble;

    for crushed_id in data.has_entities(pos) {
        if crushed_id == entity_id {
            continue;
        }

        if data.entities.name[&crushed_id] == EntityName::Column {
            let pos_diff = sub_pos(pos, data.entities.pos[&entity_id]);
            let next_pos = next_pos(data.entities.pos[&entity_id], pos_diff);

            msg_log.log_front(Msg::Crushed(crushed_id, next_pos));
        }

        if let Some(hp) = data.entities.hp.get(&crushed_id) {
            msg_log.log(Msg::Killed(entity_id, crushed_id, hp.hp));
        } else if data.entities.item.get(&crushed_id).is_none() &&
                  data.entities.name[&crushed_id] != EntityName::Mouse &&
                  data.entities.name[&crushed_id] != EntityName::Cursor {
            // the entity will be removed, such as an item.
            data.entities.mark_for_removal(crushed_id);
        }
    }

    msg_log.log_front(Msg::Sound(entity_id, pos, config.sound_radius_crushed, true));
}

fn use_energy(entity_id: EntityId, data: &mut Level, msg_log: &mut MsgLog) -> bool {
    let pos = data.entities.pos[&entity_id];

    let class = data.entities.class[&entity_id];

    // NOTE this uses the entity's class, not the skill's class
    let has_energy = data.entities.energy[&entity_id] > 0;
    let mut enough_energy: bool = false;
    match class {
        EntityClass::General => {
            if data.entities.energy[&entity_id] > 0 {
                enough_energy = true;
                data.entities.energy[&entity_id] -= 1;
            }
        }

        EntityClass::Grass => {
            let free_energy = data.map[pos].surface == Surface::Grass;
            if free_energy || has_energy {
                if !free_energy && has_energy {
                    data.entities.energy[&entity_id] -= 1;
                }

                enough_energy = true;
                data.map[pos].surface = Surface::Floor;

                if let Some(grass_id) = data.entities.get_names_at_pos(pos, EntityName::Grass).get(0) {
                    msg_log.log(Msg::Remove(*grass_id));
                }
            }
        }

        EntityClass::Monolith => {
            let free_energy = data.map[pos].surface == Surface::Rubble;
            if free_energy || has_energy {
                if !free_energy && has_energy {
                    data.entities.energy[&entity_id] -= 1;
                }

                enough_energy = true;
                data.map[pos].surface = Surface::Floor;
            }
        }

        EntityClass::Clockwork => {
            if data.entities.energy[&entity_id] > 0 {
                enough_energy = true;
                data.entities.energy[&entity_id] -= 1;
            }
        }

        EntityClass::Hierophant => {
            if data.entities.energy[&entity_id] > 0 {
                enough_energy = true;
                data.entities.energy[&entity_id] -= 1;
            }
        }

        EntityClass::Wind => {
            // The wind class does not use energy.
            enough_energy = true;
        }
    }

    return enough_energy;
}

fn pick_item_up(entity_id: EntityId, data: &mut Level, msg_log: &mut MsgLog) {
    let entity_pos = data.entities.pos[&entity_id];

    if let Some(item_id) = data.item_at_pos(entity_pos) {
        msg_log.log(Msg::PickedUp(entity_id, item_id));

        let to_drop_index = data.entities.pick_up_item(entity_id, item_id);

        if let Some(to_drop_index) = to_drop_index {
            msg_log.log(Msg::DropItem(entity_id, to_drop_index as u64));
        }
    }
}

fn place_trap(trap_id: EntityId, place_pos: Pos, data: &mut Level) {
    data.entities.set_pos(trap_id, place_pos);
    data.entities.armed[&trap_id] = true;
}

fn throw_item(player_id: EntityId,
              item_id: EntityId,
              start_pos: Pos,
              end_pos: Pos,
              data: &mut Level,
              msg_log: &mut MsgLog,
              config: &Config) {
    let throw_line = line(start_pos, end_pos);

    // get target position in direction of player click
    let end_pos =
        Pos::from(throw_line.into_iter().take(PLAYER_THROW_DIST).last().unwrap());

    let hit_pos = data.throw_towards(start_pos, end_pos);

    if let Some(hit_entity) = data.has_blocking_entity(hit_pos) {
        if data.entities.typ[&hit_entity] == EntityType::Enemy {
            let mut stun_turns = data.entities.item[&item_id].throw_stun_turns();

            if data.entities.passive[&player_id].stone_thrower {
                stun_turns += 1;
            }

            msg_log.log(Msg::Froze(hit_entity, stun_turns));
        }
    }

    data.entities.set_pos(item_id, start_pos);

    let movement = Movement::step_to(hit_pos);
    msg_log.log(Msg::Moved(item_id, movement.typ, hit_pos));

    data.entities.remove_item(player_id, item_id);
    data.entities.took_turn[&player_id] = true;

    // NOTE the radius here is the stone radius, regardless of item type
    msg_log.log_front(Msg::Sound(player_id, hit_pos, config.sound_radius_stone, true));
}

fn find_blink_pos(pos: Pos, rng: &mut Rand32, data: &mut Level) -> Option<Pos> {
    let mut potential_positions = floodfill(&data.map, pos, BLINK_RADIUS);
    while potential_positions.len() > 0 {
        let ix = rng_range_u32(rng, 0, potential_positions.len() as u32) as usize;
        let rand_pos = potential_positions[ix];

        if data.has_blocking_entity(rand_pos).is_none() &&
           data.map.path_blocked_move(pos, rand_pos).is_none() {
               return Some(rand_pos);
        }

        potential_positions.swap_remove(ix);
    }
    
    return None;
}

fn change_move_mode(entity_id: EntityId,
                    increase: bool,
                    data: &mut Level,
                    msg_log: &mut MsgLog) {
    if increase {
        let holding_shield = data.using(entity_id, Item::Shield).is_some();
        let holding_hammer = data.using(entity_id, Item::Hammer).is_some();

        let move_mode = data.entities 
                            .move_mode
                            .get(&entity_id)
                            .expect("Entity should have had a move mode!");
        let new_move_mode = move_mode.increase();

        if new_move_mode == movement::MoveMode::Run && (holding_shield || holding_hammer) {
            msg_log.log(Msg::TriedRunWithHeavyEquipment);
        } else {
            msg_log.log(Msg::MoveMode(entity_id, new_move_mode));
        }
    } else {
        let new_move_mode = data.entities.move_mode[&entity_id].decrease();
        msg_log.log(Msg::MoveMode(entity_id, new_move_mode));
    }
}

fn inventory_drop_item(entity_id: EntityId,
                       item_index: usize,
                       data: &mut Level,
                       msg_log: &mut MsgLog) {
    let player_pos = data.entities.pos[&entity_id];

    if let Some(item_id) = data.entities.inventory[&entity_id].get(item_index).map(|v| *v) {
        // Find a place to drop the item, without placing it on the same tile
        // as another item.
        let mut found_tile = false;
        let mut dist = 1;
        while !found_tile && dist < 10 {
            let positions = floodfill(&data.map, player_pos, dist);

            for pos in positions {
                if data.item_at_pos(pos).is_none() {
                    data.entities.remove_item(entity_id, item_id);
                    data.entities.set_pos(item_id, pos);

                    found_tile = true;
                    break;
                }
            }

            dist += 1;
        }

        if found_tile {
            data.entities.took_turn[&entity_id] = true;
        } else {
            msg_log.log(Msg::DropFailed(entity_id));
        }
    }
}

fn process_interaction(entity_id: EntityId,
                       interact_pos: Pos,
                       data: &mut Level, 
                       msg_log: &mut MsgLog,
                       _config: &Config) {
    let pos = data.entities.pos[&entity_id];

    if pos == interact_pos {
        if let Some(_item_id) = data.item_at_pos(pos) {
            msg_log.log(Msg::PickUp(entity_id));
        }
    } else {
        for other_id in data.has_entity(interact_pos) {
            if data.entities.trap.get(&other_id).is_some() {
                msg_log.log(Msg::ArmDisarmTrap(entity_id, other_id));
                break;
            }
        }
    }
}

fn use_item(entity_id: EntityId,
            pos: Pos,
            item_id: EntityId,
            data: &mut Level,
            msg_log: &mut MsgLog) {
    let item = data.entities.item[&item_id];

    match item {
        Item::Stone => {
            let start = data.entities.pos[&entity_id];
            let direction = Direction::from_positions(start, pos).unwrap();
            let max_end = direction.offset_pos(start, PLAYER_THROW_DIST as i32);
            let end = data.map.path_blocked_move(start, max_end)
                               .map_or(max_end, |b| b.end_pos);
            msg_log.log(Msg::ItemThrow(entity_id, item_id, start, end));
        }

        Item::Key => {
        }

        Item::Dagger => {
            //panic!("Dagger is used by moving right now. UseItem might be better");
            //msg_log.log(Msg::TryAttack(entity_id, Attack::Stab(target_id), pos))
        }

        Item::Shield => {
        }

        Item::Hammer => {
            //msg_log.log(Msg::HammerSwing(entity_id, item_id, pos));
        }

        Item::Spear => {
            // this does nothing, as spear use will go through the new use-mode system
        }

        Item::GreatSword => {
            // this does nothing, as the great sword use will go through the new use-mode system
        }

        Item::Sword => {
            //msg_log.log(Msg::SwordSwing(entity_id, item_id, pos));
            //panic!("Swords are used through use-mode how. How did you get here?");
        }

        Item::Lantern => {
            data.entities.pos[&item_id] = pos;
        }

        Item::SpikeTrap => {
            place_trap(item_id, pos, data);
            data.entities.took_turn[&entity_id] = true;
        }

        Item::SoundTrap => {
            place_trap(item_id, pos, data);
            data.entities.took_turn[&entity_id] = true;
        }

        Item::BlinkTrap => {
            place_trap(item_id, pos, data);
            data.entities.took_turn[&entity_id] = true;
        }

        Item::FreezeTrap => {
            place_trap(item_id, pos, data);
            data.entities.took_turn[&entity_id] = true;
        }
    }
}

fn make_move_sound(entity_id: EntityId,
                   original_pos: Pos,
                   pos: Pos,
                   move_mode: MoveMode,
                   data: &mut Level,
                   msg_log: &mut MsgLog,
                   config: &Config) {
    let mut sound_radius;

    match move_mode {
        MoveMode::Sneak => sound_radius = config.sound_radius_sneak,
        MoveMode::Walk => sound_radius = config.sound_radius_walk,
        MoveMode::Run => sound_radius = config.sound_radius_run,
    }

    if data.map[pos].surface == Surface::Rubble {
        // If the entity has no passives, or they do but are not sure footed.
        if data.entities.passive.get(&entity_id).is_none() || !data.entities.passive[&entity_id].sure_footed {
            sound_radius += config.sound_rubble_radius;
        }
    } else if data.map[pos].surface == Surface::Grass {
        sound_radius -= config.sound_grass_radius;
    }

    if sound_radius > 0 && data.entities.status[&entity_id].soft_steps > 0 {
        sound_radius -= 1;
    }

    if sound_radius > 0 && data.entities.passive[&entity_id].soft_shoes {
        sound_radius -= 1;
    }

    msg_log.log_front(Msg::Sound(entity_id, pos, sound_radius, true));
    msg_log.log_front(Msg::Sound(entity_id, original_pos, sound_radius, true));
}

fn process_moved_message(entity_id: EntityId,
                         move_type: MoveType,
                         pos: Pos,
                         data: &mut Level,
                         msg_log: &mut MsgLog,
                         rng: &mut Rand32,
                         config: &Config) {
    let player_id = data.find_by_name(EntityName::Player).unwrap();
    let original_pos = data.entities.pos[&entity_id];

    data.entities.set_pos(entity_id, pos);
    data.entities.took_turn[&entity_id] = true;

    if move_type != MoveType::Blink {
        if let Some(move_mode) = data.entities.move_mode.get(&entity_id) {
            if let Some(stance) = data.entities.stance.get(&entity_id) {
                data.entities.stance[&entity_id] = update_stance(move_type, *move_mode, *stance);
            }

            // make a noise based on how fast the entity is moving and the terrain
            if pos != original_pos {
                make_move_sound(entity_id, original_pos, pos, *move_mode, data, msg_log, config);
            }
        } else if pos != original_pos && data.entities.typ[&entity_id] == EntityType::Enemy {
            msg_log.log_front(Msg::Sound(entity_id, original_pos, config.sound_radius_monster, true));
            msg_log.log_front(Msg::Sound(entity_id, pos, config.sound_radius_monster, true));
        } // NOTE other entities do not make sounds on movement, such as items
    }

    // check if player walks on energy
    if entity_id == player_id {
        for other_id in data.entities.ids.clone().iter() {
            if data.entities.pos[other_id] == pos && 
            data.entities.typ[other_id] == EntityType::Energy {
                data.entities.energy[&player_id] += 1;
                data.entities.needs_removal[other_id] = true;
            }
        }
    }

    if original_pos != pos {
        resolve_triggered_traps(entity_id, original_pos, data, rng, msg_log);
    }

    // check for passing turn while the hammer is raised
    if move_type == MoveType::Pass {
        if let Some((item_id, dir, turns)) = data.entities.status[&entity_id].hammer_raised {
            if turns == 0 {
                let hit_pos = dir.offset_pos(original_pos, 1);
                msg_log.log(Msg::HammerSwing(entity_id, item_id, hit_pos));
                data.entities.status[&entity_id].hammer_raised = None;
            }
        }
    }

    // if entity is a monster, which is also alert, and there is a path to the player,
    // then face the player
    if let Some(target_pos) = data.entities.target(entity_id) {
        if data.could_see(entity_id, target_pos, config) {
            msg_log.log_front(Msg::FaceTowards(entity_id, target_pos));
        }
    } else {
        let diff = sub_pos(pos, original_pos);
        if let Some(dir) = Direction::from_dxy(diff.x, diff.y) {
            msg_log.log_front(Msg::SetFacing(entity_id, dir));
        }
    }
}

fn resolve_triggered_traps(entity_id: EntityId,
                           original_pos: Pos,
                           data: &mut Level,
                           rng: &mut Rand32,
                           msg_log: &mut MsgLog) {
    if data.entities.passive[&entity_id].light_touch && rng_trial(rng, 0.5) {
        return;
    }

    // get a list of triggered traps
    let traps: Vec<EntityId> = data.entities.triggered_traps(data.entities.pos[&entity_id]);

    // Check if the entity hit a trap
    for trap in traps.iter() {
        match data.entities.trap[trap] {
            Trap::Spikes => {
                msg_log.log(Msg::SpikeTrapTriggered(*trap, entity_id));
                data.entities.mark_for_removal(*trap);
            }

            Trap::Sound => {
                msg_log.log(Msg::SoundTrapTriggered(*trap, entity_id));
                data.entities.needs_removal[trap] = true;
                data.entities.mark_for_removal(*trap);
            }

            Trap::Blink => {
                data.entities.status[&entity_id].blinked = true;
                msg_log.log(Msg::BlinkTrapTriggered(*trap, entity_id));
                data.entities.mark_for_removal(*trap);
            }

            Trap::Freeze => {
                msg_log.log(Msg::FreezeTrapTriggered(*trap, entity_id));
                data.entities.mark_for_removal(*trap);
            }
        }
    }

    // Resolve triggers
    for key in data.entities.ids.iter() {
        // key is a trigger
        if data.entities.typ[key] == EntityType::Trigger {
            // stepped on trigger
           if data.entities.pos[key] == data.entities.pos[&entity_id] {
               msg_log.log_front(Msg::Triggered(*key, entity_id));
           }

            // stepped off of trigger
           if data.entities.pos[key] == original_pos &&
              data.entities.status[key].active {
               msg_log.log_front(Msg::Untriggered(*key, entity_id));
           }
        }
    }
}


fn resolve_ai_attack(entity_id: EntityId,
                     target_id: EntityId,
                     data: &mut Level,
                     msg_log: &mut MsgLog,
                     rng: &mut Rand32,
                     config: &Config) {
    let target_pos = data.entities.pos[&target_id];

    let attack_reach = data.entities.attack[&entity_id];
    let can_hit_target =
        ai_can_hit_target(data, entity_id, target_pos, &attack_reach, config);

    if data.entities.is_dead(target_id) {
        data.entities.took_turn[&entity_id] = true;
        msg_log.log(Msg::StateChange(entity_id, Behavior::Investigating(target_pos)));
    } else if let Some(_hit_pos) = can_hit_target {
        let mut can_attack = true;
        // quick reflexes
        if data.entities.passive.get(&target_id).is_some() && data.entities.passive[&target_id].quick_reflexes {
            if rng_trial(rng, SKILL_QUICK_REFLEXES_PERCENT) {
                can_attack = false;
                msg_log.log(Msg::Dodged(target_id));
            }
        }

        if can_attack {
            let attack_info = Attack::Attack(target_id);
            msg_log.log(Msg::TryAttack(entity_id, attack_info, target_pos));
        }
    } else if !ai_is_in_fov(entity_id, target_id, data, config) {
        // if we lose the target, end the turn
        data.entities.took_turn[&entity_id] = true;
        msg_log.log(Msg::StateChange(entity_id, Behavior::Investigating(target_pos)));
    } else {
        // can see target, but can't hit them. try to move to a position where we can hit them
        let maybe_pos = ai_move_to_attack_pos(entity_id, target_id, data, config);

        if let Some(move_pos) = maybe_pos {
            // try to move in the given direction
            let entity_pos = data.entities.pos[&entity_id];
            let direction = Direction::from_positions(entity_pos, move_pos).unwrap();
            msg_log.log(Msg::TryMove(entity_id, direction, 1, MoveMode::Walk));
        } else {
            // if we can't move anywhere, we just end our turn
            data.entities.took_turn[&entity_id] = true;
        }
    }
}

