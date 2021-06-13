#[allow(unused_imports)]
use log::{trace, error};

use oorandom::Rand32;

use roguelike_core::types::*;
use roguelike_core::ai::{Behavior, ai_move_to_attack_pos, ai_can_hit_target, ai_take_turn, ai_is_in_fov};
use roguelike_core::map::{Surface, AoeEffect};
use roguelike_core::messaging::{MsgLog, Msg};
use roguelike_core::constants::*;
use roguelike_core::movement::{MoveMode, MoveType, Attack, Movement, Direction, Reach};
use roguelike_core::movement;
use roguelike_core::config::*;
use roguelike_core::utils::*;
use roguelike_core::map::*;
use roguelike_core::line::line;

use crate::generation::{make_energy, make_dagger, make_sword, make_light};


pub fn resolve_messages(data: &mut GameData,
                        msg_log: &mut MsgLog,
                        rng: &mut Rand32,
                        config: &Config) {
    let player_id = data.find_by_name(EntityName::Player).unwrap();

    /* Handle Message Log */
    while let Some(msg) = msg_log.pop() {
        match msg {
            Msg::Moved(entity_id, move_type, pos) => {
               process_moved_message(entity_id, move_type, pos, data, msg_log, config);
            }

            Msg::Interact(entity_id, pos) => {
               process_interaction(entity_id, pos, data, msg_log, config);
            }

            Msg::Crushed(entity_id, pos) => {
                crushed(entity_id, pos, data, msg_log, config);
            }

            Msg::Sound(cause_id, source_pos, radius, _should_animate) => {
                let sound_aoe =
                    aoe_fill(&data.map, AoeEffect::Sound, source_pos, radius, config);

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
                msg_log.log_front(Msg::Sound(entity_id, end, config.sound_radius_stone, false));
            }

            Msg::JumpWall(entity_id, _start, end) => {
                msg_log.log_front(Msg::Sound(entity_id, end, config.sound_radius_run, true));
            }


            Msg::Blink(entity_id) => {
                if use_energy(entity_id, data) {
                    resolve_blink(entity_id, data, rng, msg_log);
                }
            }

            Msg::Pushed(pusher, pushed, direction, push_amount, move_into) => {
                pushed_entity(pusher, pushed, direction, push_amount, move_into, data, config, msg_log);
            }

            Msg::Yell(entity_id) => {
                let pos = data.entities.pos[&entity_id];
                msg_log.log_front(Msg::Sound(entity_id, pos, config.yell_radius, true));
                data.entities.took_turn[&entity_id] = true;
            }

            Msg::Killed(_attacker, attacked, _damage) => {
                killed_entity(attacked, data, msg_log, config);
            }

            Msg::Attack(attacker, attacked, _damage) => {
                // TODO move attack function here, and remove push Msg::Attack in attack function
                let pos = data.entities.pos[&attacked];
                msg_log.log_front(Msg::Sound(attacker, pos, config.sound_radius_attack, true)); 
            }

            Msg::SwordSwing(entity_id, pos) => {
                sword_swing(entity_id, pos, data, msg_log);
            }

            Msg::HammerSwing(entity_id, pos) => {
                hammer_swing(entity_id, pos, data, msg_log);
            }

            // TODO Consider making this a Push message, splitting out that code from Action as well
            Msg::HammerHitEntity(entity_id, hit_entity) => {
                hammer_hit_entity(entity_id, hit_entity, data, msg_log, config);
            }

            Msg::HammerHitWall(entity_id, blocked) => {
                hammer_hit_wall(entity_id, blocked, data, msg_log, config);
            }

            Msg::TryAttack(entity_id, attack_info, attack_pos) => {
                resolve_attack(entity_id, attack_info, attack_pos, data, msg_log, config);
            }

            Msg::TryMove(entity_id, direction, amount, move_mode) => {
                resolve_try_move(entity_id, direction, amount, move_mode, data, msg_log);
            }

            Msg::PickUp(entity_id) => {
                pick_item_up(entity_id, data, msg_log);
            }

            Msg::StateChange(entity_id, behavior) => {
                data.entities.behavior[&entity_id] = behavior;

                // if the entity hasn't completed a turn, the state change continues their turn.
                // NOTE this might be better off as a message! emit it every time a state change
                // occurs?
                if !data.entities.took_turn[&entity_id] {
                   ai_take_turn(entity_id, data, config, msg_log);
                }
            }

            Msg::SpikeTrapTriggered(trap, entity_id) => {
                data.entities.take_damage(entity_id, SPIKE_DAMAGE);

                if data.entities.fighter[&entity_id].hp <= 0 {
                    data.entities.status[&entity_id].alive = false;
                    data.entities.blocks[&entity_id] = false;

                    msg_log.log(Msg::Killed(trap, entity_id, SPIKE_DAMAGE));
                }
            }

            Msg::SoundTrapTriggered(trap, entity_id) => {
                let source_pos = data.entities.pos[&trap];

                // the triggering entity is considered the source of the sound
                msg_log.log(Msg::Sound(entity_id, source_pos, config.sound_radius_trap, true));
            }

            Msg::BlinkTrapTriggered(trap, entity_id) => {
                let source_pos = data.entities.pos[&trap];

                if let Some(blink_pos) = find_blink_pos(source_pos, rng, data) {
                    data.entities.move_to(entity_id, blink_pos);
                }
            }

            Msg::Froze(entity_id, num_turns) => {
                if entity_id == player_id || data.entities.ai.get(&entity_id).is_some() {
                    data.entities.status[&entity_id].frozen = num_turns;
                }
            }

            Msg::FreezeTrapTriggered(trap, cause_id) => {
                freeze_trap_triggered(trap, cause_id, data, msg_log, config);
            }

            Msg::Untriggered(trigger, _entity_id) => {
                untriggered(trigger, data, msg_log);
            }

            Msg::Triggered(trigger, _entity_id) => {
                triggered(trigger, data, msg_log);
            }

            Msg::AddClass(class) => {
                data.entities.skills[&player_id].clear();

                match class {
                    EntityClass::General => {
                        data.entities.class[&player_id] = class;
                        data.entities.add_skill(player_id, Skill::Blink);
                        data.entities.add_skill(player_id, Skill::Sprint);
                    }

                    EntityClass::Monolith => {
                        data.entities.class[&player_id] = class;
                        data.entities.add_skill(player_id, Skill::PassWall);
                        data.entities.add_skill(player_id, Skill::Rubble);
                        data.entities.add_skill(player_id, Skill::Reform);
                    }

                    EntityClass::Grass => {
                        data.entities.class[&player_id] = class;
                        data.entities.add_skill(player_id, Skill::GrassThrow);
                        data.entities.add_skill(player_id, Skill::GrassBlade);
                        data.entities.add_skill(player_id, Skill::GrassShoes);
                    }

                    EntityClass::Clockwork => {
                        data.entities.class[&player_id] = class;
                        data.entities.add_skill(player_id, Skill::Push);
                    }

                    EntityClass::Hierophant => {
                        data.entities.class[&player_id] = class;
                        data.entities.add_skill(player_id, Skill::Illuminate);
                        data.entities.add_skill(player_id, Skill::Heal);
                        data.entities.add_skill(player_id, Skill::FarSight);
                    }
                }
            }

            Msg::SwapPrimaryItem => {
                // TODO make a message for this, and allow resolve to do this.
                if item_primary_at(player_id, &mut data.entities, 0) &&
                   item_primary_at(player_id, &mut data.entities, 1) {
                       let temp_id = data.entities.inventory[&player_id][0];

                       data.entities.inventory[&player_id][0] = 
                           data.entities.inventory[&player_id][1];

                       data.entities.inventory[&player_id][1] = temp_id;
               }
            }

            Msg::MoveMode(entity_id, new_move_mode) => {
                data.entities.move_mode[&entity_id] = new_move_mode;

                // update entities movement reach with their new move mode
                data.entities.movement[&entity_id] = reach_by_mode(data.entities.move_mode[&entity_id]);
            }

            Msg::ChangeMoveMode(entity_id, increase) => {
                change_move_mode(entity_id, increase, data, msg_log);
            }

            Msg::DropItem(entity_id, item_index) => {
                inventory_drop_item(entity_id, item_index as usize, data, msg_log);
            }

            Msg::GrassThrow(entity_id, direction) => {
                if use_energy(entity_id, data) {

                    let pos = data.entities.pos[&entity_id];

                    let grass_pos = direction.offset_pos(pos, 1);
                    if data.map[grass_pos].tile_type == TileType::Empty {
                        data.map[grass_pos].surface = Surface::Grass;
                    }
                    data.entities.took_turn[&entity_id] = true;
                }
            }

            Msg::GrassShoes(entity_id, _action_mode) => {
                if use_energy(entity_id, data) {
                    data.entities.status[&entity_id].soft_steps = SKILL_GRASS_SHOES_TURNS;
                    data.entities.took_turn[&entity_id] = true;
                }
            }

            Msg::Illuminate(entity_id, pos, amount) => {
                if use_energy(entity_id, data) {
                    let entity_pos = data.entities.pos[&entity_id];
                    let light = make_light(&mut data.entities, config, pos, msg_log);
                    data.entities.status[&light].illuminate = amount;

                    data.entities.took_turn[&entity_id] = true;
                }
            }

            Msg::Heal(entity_id, amount) => {
                if use_energy(entity_id, data) {
                    data.entities.fighter[&entity_id].hp = 
                        std::cmp::min(data.entities.fighter[&entity_id].max_hp,
                                      data.entities.fighter[&entity_id].hp + amount as i32);

                    data.entities.took_turn[&entity_id] = true;
                }
            }

            Msg::FarSight(entity_id, amount) => {
                if use_energy(entity_id, data) {
                    data.entities.status[&entity_id].extra_fov += amount;
                    data.entities.took_turn[&entity_id] = true;
                }
            }

            Msg::Sprint(entity_id, direction, amount) => {
                if use_energy(entity_id, data) {
                    msg_log.log(Msg::TryMove(entity_id, direction, amount, MoveMode::Run));
                    data.entities.took_turn[&entity_id] = true;
                }
            }

            Msg::Rubble(entity_id, rubble_pos) => {
                let pos = data.entities.pos[&entity_id];
                let blocked = data.map.path_blocked_move(pos, rubble_pos);

                if let Some(blocked) = blocked {
                    if data.has_blocking_entity(rubble_pos).is_none() {
                        if use_energy(entity_id, data) {
                            resolve_rubble(entity_id, blocked, data, msg_log);
                        }
                    }
                }
            }

            Msg::GrassBlade(entity_id, action_mode) => {
                if use_energy(entity_id, data) {

                    let pos = data.entities.pos[&entity_id];

                    let item_id;
                    match action_mode {
                        ActionMode::Primary => {
                            item_id = make_sword(&mut data.entities, config, pos, msg_log);
                        }

                        ActionMode::Alternate => {
                            item_id = make_dagger(&mut data.entities, config, pos, msg_log);
                        }
                    }

                    data.entities.pick_up_item(entity_id, item_id);
                    data.entities.took_turn[&entity_id] = true;
                }
            }

            Msg::Reform(entity_id, pos) => {
                if data.map[pos].surface == Surface::Rubble &&
                   data.has_blocking_entity(pos).is_none() {
                    if use_energy(entity_id, data) {
                        data.map[pos].surface = Surface::Floor;
                        data.map[pos].block_move = true;
                        data.map[pos].chr = MAP_WALL;
                        data.entities.took_turn[&entity_id] = true;
                    }
                }
            }

            Msg::Swap(entity_id, target_id) => {
                if use_energy(entity_id, data) {

                    let start_pos = data.entities.pos[&entity_id];
                    let end_pos = data.entities.pos[&target_id];
                    data.entities.move_to(entity_id, end_pos);
                    data.entities.move_to(target_id, start_pos);

                    data.entities.took_turn[&entity_id] = true;
                }
            }

            Msg::PassWall(entity_id, pos) => {
                if use_energy(entity_id, data) {
                    data.entities.move_to(entity_id, pos);

                    data.entities.took_turn[&entity_id] = true;
                }
            }

            Msg::UseItem(entity_id, pos, item_id) => {
                use_item(entity_id, pos, item_id, data, msg_log);
            }

            Msg::ArmDisarmTrap(entity_id, trap_id) => {
                data.entities.armed[&trap_id] = !data.entities.armed[&trap_id];
                data.entities.took_turn[&entity_id] = true;
            }

            Msg::PlaceTrap(entity_id, place_pos, trap_id) => {
                place_trap(trap_id, place_pos, data);
                data.entities.took_turn[&entity_id] = true;
            }


            Msg::Push(entity_id, direction, amount) => {
                if use_energy(entity_id, data) {
                    resolve_push_skill(entity_id, direction, amount, data, msg_log);
                }
            }

            Msg::FaceTowards(entity_id, pos) => {
                data.entities.face(entity_id, pos);
                msg_log.log(Msg::Facing(entity_id, data.entities.direction[&entity_id]));
            }

            Msg::AiAttack(entity_id) => {
                if let Behavior::Attacking(target_id) = data.entities.behavior[&entity_id] {
                    resolve_ai_attack(entity_id, target_id, data, msg_log, config);
                } else {
                    panic!("ai attacking but not in attack state!");
                }
            }

            _ => {
            }
        }
    }

    /* Process Player Messages */
    for message in data.entities.messages[&player_id].iter() {
        if let Message::Sound(obj_id, _pos) = message {
            if *obj_id == player_id {
                panic!("Player sent themselves a message?")
            }

            let _player_pos = data.entities.pos[&player_id];
            //if !data.map.is_in_fov(player_pos, *pos, config.fov_radius_player) {
                //let heard = Effect::HeardSomething(*pos, settings.turn_count);
                // TODO move to somewhere else?
                //display_state.effects.push(heard);
            //}
        }
    }
    data.entities.messages[&player_id].clear();
}

fn hammer_swing(entity_id: EntityId, pos: Pos, data: &mut GameData, msg_log: &mut MsgLog) {
    let entity_pos = data.entities.pos[&entity_id];

    if let Some(blocked) = data.map.path_blocked_move(entity_pos, pos) {
        msg_log.log_front(Msg::HammerHitWall(entity_id, blocked));
        data.used_up_item(entity_id);
    } else if let Some(hit_entity) = data.has_blocking_entity(pos) {
        // we hit another entity!
        msg_log.log_front(Msg::HammerHitEntity(entity_id, hit_entity));
        data.used_up_item(entity_id);
    }

    data.entities.took_turn[&entity_id] = true;
}

fn hammer_hit_entity(entity_id: EntityId, hit_entity: EntityId, data: &mut GameData, msg_log: &mut MsgLog, config: &Config) {
    let first = data.entities.pos[&entity_id];
    let second = data.entities.pos[&hit_entity];

    let dxy = sub_pos(second, first);
    let direction = Direction::from_dxy(dxy.x, dxy.y).unwrap();
    let amount = 1;
    msg_log.log(Msg::Pushed(entity_id, hit_entity, direction, amount, false));
    msg_log.log_front(Msg::Sound(entity_id, second, config.sound_radius_hammer, true));

    if let Some(fighter) = data.entities.fighter.get(&hit_entity) {
        let damage = fighter.hp;

        msg_log.log(Msg::Killed(entity_id, hit_entity, damage));
    }
}

fn sword_swing(entity_id: EntityId, pos: Pos, data: &mut GameData, msg_log: &mut MsgLog) {
    let mut any_hit_entity = false;

    let adj_locs = Reach::single(1).reachables(pos);
    for loc in adj_locs {
        if let Some(target_id) = data.has_blocking_entity(loc) {
            if data.entities.status[&target_id].alive {
                attack(entity_id, target_id, data, msg_log);
                any_hit_entity = true;
            }
        }
    }

    if any_hit_entity {
        data.used_up_item(entity_id);
    }

    data.entities.took_turn[&entity_id] = true;
}

fn freeze_trap_triggered(trap: EntityId, cause_id: EntityId, data: &mut GameData, msg_log: &mut MsgLog, config: &Config) {
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

fn triggered(trigger: EntityId, data: &mut GameData, _msg_log: &mut MsgLog) {
    if data.entities.name[&trigger] == EntityName::GateTrigger {
        if !data.entities.status[&trigger].active {
            let trigger_pos = data.entities.pos[&trigger];

            // any wall nearby is a potential target
            for neighbor in data.map.cardinal_neighbors(trigger_pos) {
                if data.map[neighbor].tile_type == TileType::Wall {
                    data.entities.status[&trigger].active = true;

                    data.map[neighbor] = Tile::empty();

                    data.entities.gate_pos[&trigger] = Some(neighbor);
                    break;
                }
            }
        }
    }
}

fn resolve_attack(entity_id: EntityId,
                  attack_info: Attack,
                  attack_pos: Pos,
                  data: &mut GameData,
                  msg_log: &mut MsgLog,
                  _config: &Config) {
    let entity_pos = data.entities.pos[&entity_id];

    // any time an entity attacks, they change to standing stance
    data.entities.stance[&entity_id] = Stance::Standing;

    // we already checked that this unwrap is safe before calling this function
    match attack_info {
        Attack::Attack(target_id) => {
            attack(entity_id, target_id, data, msg_log);
        }

        Attack::Stab(target_id) => {
            stab(entity_id, target_id, &mut data.entities, msg_log);

            if data.using(entity_id, Item::Dagger) {
                data.used_up_item(entity_id);
            }

            if entity_pos != attack_pos {
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
                    data: &mut GameData,
                    msg_log: &mut MsgLog) {
    if amount == 0 {
        panic!("Why try to move with amount == 0?");
    }

    data.entities.move_mode[&entity_id] = move_mode;

    let reach = data.entities.movement[&entity_id];
    let reach = reach.with_dist(1);

    let maybe_movement = 
        movement::calculate_move(direction, reach, entity_id, data);

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
                        data: &mut GameData,
                        msg_log: &mut MsgLog) {
    let entity_pos = data.entities.pos[&entity_id];

    match movement.typ {
        MoveType::Collide => {
            data.entities.move_to(entity_id, movement.pos);

            msg_log.log(Msg::Collided(entity_id, movement.pos));
        }

        MoveType::Pass => {
            msg_log.log(Msg::Moved(entity_id, MoveType::Pass, movement.pos));
        }

        MoveType::WallKick => {
            data.entities.move_to(entity_id, movement.pos);

            // TODO could check for enemy and attack
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
                    msg_log.log(Msg::Moved(entity_id, movement.typ, movement.pos));

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
                msg_log.log(Msg::Moved(entity_id, movement.typ, movement.pos));
            } else {
                panic!("Why would we not have a clear path, but have received this movement?");
                // TODO move towards position, perhaps emitting a Collide
                // message. This is likely causing the jump wall issue!
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
                      data: &mut GameData,
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

fn resolve_blink(entity_id: EntityId, data: &mut GameData, rng: &mut Rand32, msg_log: &mut MsgLog) {
    let entity_pos = data.entities.pos[&entity_id];

    if let Some(blink_pos) = find_blink_pos(entity_pos, rng, data) {
        data.entities.move_to(entity_id, blink_pos);
    } else {
        msg_log.log(Msg::FailedBlink(entity_id));
    }

    data.entities.took_turn[&entity_id] = true;
}

fn resolve_rubble(entity_id: EntityId, blocked: Blocked, data: &mut GameData, _msg_log: &mut MsgLog) {
    let entity_pos = data.entities.pos[&entity_id];

    data.map[blocked.end_pos].surface = Surface::Rubble;
    data.map[blocked.end_pos].block_move = false;
    data.map[blocked.end_pos].chr = ' ' as u8;

    if blocked.wall_type != Wall::Empty {
        let dxy = sub_pos(blocked.end_pos, entity_pos);
        match Direction::from_dxy(dxy.x, dxy.y).unwrap() {
            Direction::Up => {
                data.map[blocked.end_pos].bottom_wall = Wall::Empty;
            }

            Direction::Down => {
                data.map[entity_pos].bottom_wall = Wall::Empty;
            }

            Direction::Left => {
                data.map[entity_pos].left_wall = Wall::Empty;
            }
            
            Direction::Right => {
                data.map[blocked.end_pos].left_wall = Wall::Empty;
            }

            _ => {
                panic!("Rubble skill doesn't work on diagonals!");
            }
        }
    }

    data.entities.took_turn[&entity_id] = true;
}

fn untriggered(trigger: EntityId, data: &mut GameData, msg_log: &mut MsgLog) {
    if data.entities.name[&trigger] == EntityName::GateTrigger {
        // if the gate is currently active, raise the wall
        if data.entities.status[&trigger].active {
            // is the trigger free of other entities?
            let stepped_on =
                data.entities
                    .ids.iter()
                    .any(|key| data.entities.typ[key] != EntityType::Trigger &&
                               data.entities.pos[&trigger] == data.entities.pos[key]);

            let mut maybe_wall_pos = None;
            if !stepped_on {
                // raise the gate
                data.entities.status[&trigger].active = false;
                if let Some(wall_pos) = data.entities.gate_pos[&trigger] {
                    data.map[wall_pos] = Tile::wall();
                    data.entities.gate_pos[&trigger] = None;
                    maybe_wall_pos = Some(wall_pos);
                }
            }

            // if the gate was raised, kill entities in that spot
            if let Some(wall_pos) = maybe_wall_pos {
                for key in data.entities.ids.iter() {
                    if data.entities.pos[key] == wall_pos &&
                       data.entities.status[key].alive {
                        msg_log.log(Msg::Killed(trigger, *key, TRIGGER_WALL_DAMAGE));
                    }
                }
            }
        }
    }
}

fn hammer_hit_wall(entity: EntityId, blocked: Blocked, data: &mut GameData, msg_log: &mut MsgLog, config: &Config) {
    let entity_pos = data.entities.pos[&entity];
    let hit_pos = blocked.end_pos;
    if data.map[hit_pos].block_move {
        if data.map[hit_pos].surface == Surface::Floor {
            data.map[hit_pos].surface = Surface::Rubble;
        }

        data.map[hit_pos].block_move = false;
        data.map[hit_pos].chr = ' ' as u8;

        let next_pos = next_from_to(entity_pos, hit_pos);
        msg_log.log_front(Msg::Crushed(entity, next_pos)); 
        msg_log.log_front(Msg::Sound(entity, blocked.end_pos, config.sound_radius_attack, true)); 
    } else {
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
            panic!(format!("Hammer direction was not up/down/left/right ({:?})!", blocked));
         }

        if left_wall {
            data.map[wall_loc].left_wall = Wall::Empty;
        } else {
            data.map[wall_loc].bottom_wall = Wall::Empty;
        }

        msg_log.log_front(Msg::Crushed(entity, blocked.end_pos));
    }
}

fn killed_entity(attacked: EntityId, data: &mut GameData, msg_log: &mut MsgLog, config: &Config) {
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

    if let Some(fighter) = data.entities.fighter.get_mut(&attacked) {
        fighter.hp = 0;
    }
    data.entities.status[&attacked].alive = false;

    data.entities.blocks[&attacked] = false;

    data.entities.limbo.insert(attacked, ());

    data.entities.count_down.insert(attacked, 1);
}

fn pushed_entity(pusher: EntityId,
                 pushed: EntityId,
                 direction: Direction,
                 push_amount: usize,
                 move_into: bool,
                 data: &mut GameData,
                 config: &Config,
                 msg_log: &mut MsgLog) {
    let pushed_pos = data.entities.pos[&pushed];
    let pusher_pos = data.entities.pos[&pusher];

    if data.entities.typ[&pushed] == EntityType::Column {
        let entity_diff = sub_pos(pushed_pos, pusher_pos);
        let next_pos = next_pos(pusher_pos, entity_diff);
        let blocked = data.map.path_blocked_move(pushed_pos, next_pos); 

        if blocked == None {
            data.entities.count_down.insert(pushed, 1);

            msg_log.log_front(Msg::Crushed(pusher, next_pos));

            if pos_on_map(data.entities.pos[&pusher]) {
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

fn crushed(entity_id: EntityId, pos: Pos, data: &mut GameData, msg_log: &mut MsgLog, config: &Config) {
    data.map[pos].surface = Surface::Rubble;

    for crushed_id in data.has_entities(pos) {
        if crushed_id == entity_id {
            continue;
        }
        if let Some(fighter) = data.entities.fighter.get(&crushed_id) {
            msg_log.log(Msg::Killed(entity_id, crushed_id, fighter.hp));
        } else if data.entities.item.get(&crushed_id).is_none() &&
                  data.entities.name[&crushed_id] != EntityName::Mouse &&
                  data.entities.name[&crushed_id] != EntityName::Cursor {
            // the entity will be removed
            data.entities.count_down.insert(crushed_id, 1);
        }
    }

    msg_log.log_front(Msg::Sound(entity_id, pos, config.sound_radius_crushed, true));
}

fn use_energy(entity_id: EntityId, data: &mut GameData) -> bool {
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
    }

    return enough_energy;
}

fn pick_item_up(entity_id: EntityId, data: &mut GameData, msg_log: &mut MsgLog) {
    let entity_pos = data.entities.pos[&entity_id];

    if let Some(item_id) = data.item_at_pos(entity_pos) {
        data.entities.pick_up_item(entity_id, item_id);
        msg_log.log(Msg::PickedUp(entity_id, item_id));
    }
}

fn place_trap(trap_id: EntityId, place_pos: Pos, data: &mut GameData) {
    data.entities.set_pos(trap_id, place_pos);
    data.entities.armed[&trap_id] = true;
}

fn throw_item(player_id: EntityId,
              item_id: EntityId,
              start_pos: Pos,
              end_pos: Pos,
              data: &mut GameData,
              msg_log: &mut MsgLog) {
    let throw_line = line(start_pos, end_pos);

    // get target position in direction of player click
    let mut end_pos =
        Pos::from(throw_line.into_iter().take(PLAYER_THROW_DIST).last().unwrap());

    if let Some(blocked) = data.map.path_blocked_move(start_pos, end_pos) {
        // the start pos of the blocked struct is the last reached position
        end_pos = blocked.start_pos;
    }

    data.entities.set_pos(item_id, start_pos);

    let movement = Movement::step_to(end_pos);
    msg_log.log(Msg::Moved(item_id, movement.typ, end_pos));

    data.entities.remove_item(player_id, item_id);
    data.entities.took_turn[&player_id] = true;
}

fn find_blink_pos(pos: Pos, rng: &mut Rand32, data: &mut GameData) -> Option<Pos> {
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
                    data: &mut GameData,
                    msg_log: &mut MsgLog) {
    if increase {
        let holding_shield = data.using(entity_id, Item::Shield);
        let holding_hammer = data.using(entity_id, Item::Hammer);

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
                       data: &mut GameData,
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
                       data: &mut GameData, 
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
            data: &mut GameData,
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
            panic!("Dagger is used by moving right now. UseItem might be better");
            //msg_log.log(Msg::TryAttack(entity_id, Attack::Stab(target_id), pos))
        }

        Item::Shield => {
        }

        Item::Hammer => {
            msg_log.log(Msg::HammerSwing(entity_id, pos));
        }

        Item::Sword => {
            msg_log.log(Msg::SwordSwing(entity_id, pos));
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

fn process_moved_message(entity_id: EntityId,
                         move_type: MoveType,
                         pos: Pos,
                         data: &mut GameData,
                         msg_log: &mut MsgLog,
                         config: &Config) {
    let player_id = data.find_by_name(EntityName::Player).unwrap();
    let original_pos = data.entities.pos[&entity_id];

    data.entities.move_to(entity_id, pos);
    data.entities.took_turn[&entity_id] = true;

    if let Some(move_mode) = data.entities.move_mode.get(&entity_id) {
        if let Some(stance) = data.entities.stance.get(&entity_id) {
            if move_type == MoveType::Pass {
                data.entities.stance[&entity_id] = stance.waited(*move_mode);
            } else if *move_mode == MoveMode::Run {
                data.entities.stance[&entity_id] = Stance::Running;
            } else if *move_mode == MoveMode::Sneak {
                data.entities.stance[&entity_id] = Stance::Crouching;
            } else if *move_mode == MoveMode::Walk && *stance == Stance::Crouching {
                data.entities.stance[&entity_id] = Stance::Standing;
            }
        }

        // make a noise based on how fast the entity is moving and the terrain
        if pos != original_pos {
            let mut sound_radius;

            match move_mode {
                MoveMode::Sneak => sound_radius = config.sound_radius_sneak,
                MoveMode::Walk => sound_radius = config.sound_radius_walk,
                MoveMode::Run => sound_radius = config.sound_radius_run,
            }

            if data.map[pos].surface == Surface::Rubble {
                sound_radius += config.sound_rubble_radius;
            } else if data.map[pos].surface == Surface::Grass {
                sound_radius -= config.sound_grass_radius;
            }

            if data.entities.status[&entity_id].soft_steps > 0 {
                sound_radius -= 1;
            }

            msg_log.log_front(Msg::Sound(entity_id, pos, sound_radius, true));
        }
    } else if pos != original_pos {
        msg_log.log_front(Msg::Sound(entity_id, pos, config.sound_radius_monster, true));
    }

    // get a list of triggered traps
    let mut traps: Vec<EntityId> = Vec::new();
    for key in data.entities.ids.iter() {
        if data.entities.trap.get(key).is_some()           && // key is a trap
           data.entities.armed.get(key) == Some(&true)     && // trap is armed
           data.entities.status[&entity_id].alive          && // entity is alive
           data.entities.fighter.get(&entity_id).is_some() && // entity is a fighter
           data.entities.pos[key] == data.entities.pos[&entity_id] {
            traps.push(*key);
        }
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

            Trap::Blink => {
                msg_log.log(Msg::BlinkTrapTriggered(*trap, entity_id));
                data.entities.needs_removal[trap] = true;
            }

            Trap::Freeze => {
                msg_log.log(Msg::FreezeTrapTriggered(*trap, entity_id));
                data.entities.needs_removal[trap] = true;
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

    // if entity is a monster, which is also alert, and there is a path to the player,
    // then face the player
    if let Some(target_pos) = data.entities.target(entity_id) {
        if data.could_see(entity_id, target_pos, config) {
            msg_log.log_front(Msg::FaceTowards(entity_id, target_pos));
            //data.entities.face(entity_id, target_pos);
        }
    }
}

fn resolve_ai_attack(entity_id: EntityId,
                     target_id: EntityId,
                     data: &mut GameData,
                     msg_log: &mut MsgLog,
                     config: &Config) {
    let target_pos = data.entities.pos[&target_id];

    let attack_reach = data.entities.attack[&entity_id];
    let can_hit_target =
        ai_can_hit_target(data, entity_id, target_pos, &attack_reach, config);

    if data.entities.is_dead(target_id) {
        data.entities.took_turn[&entity_id] = true;
        msg_log.log(Msg::StateChange(entity_id, Behavior::Investigating(target_pos)));
    } else if let Some(_hit_pos) = can_hit_target {
        let attack_info = Attack::Attack(target_id);
        msg_log.log(Msg::TryAttack(entity_id, attack_info, target_pos));
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

