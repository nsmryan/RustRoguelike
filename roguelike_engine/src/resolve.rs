#[allow(unused_imports)]
use std::mem;

use roguelike_utils::line::*;
use roguelike_utils::rng::*;
use roguelike_utils::comp::*;

use roguelike_map::*;

use roguelike_core::types::*;
use roguelike_core::ai::{Behavior, ai_move_to_attack_pos, ai_can_hit_target, ai_take_turn};
use roguelike_core::messaging::{MsgLog, Msg};
use roguelike_core::constants::*;
use roguelike_core::movement::{MoveMode, MoveType, Attack, Movement};
use roguelike_core::movement;
use roguelike_core::config::*;
use roguelike_core::utils::*;
use roguelike_core::level::*;

#[cfg(test)]
use crate::actions::InputAction;
#[cfg(test)]
use crate::generation::*;

use crate::generation::{make_energy, make_light, ensure_grass, make_smoke, make_magnifier};
use crate::game::{Game};
use crate::map_construct::map_construct;


pub fn resolve_messages(game: &mut Game) {
    let player_id = game.level.find_by_name(EntityName::Player).unwrap();

    /* Handle Message Log */
    while let Some(msg) = game.msg_log.pop() {
        match msg {
            Msg::Moved(entity_id, move_type, move_mode, pos) => {
               resolve_moved_message(entity_id, move_type, move_mode, pos, &mut game.level, &mut game.msg_log, &mut game.rng, &game.config);
            }

            Msg::Interact(entity_id, pos) => {
               resolve_interaction(entity_id, pos, &mut game.level, &mut game.msg_log, &game.config);
            }

            Msg::Crushed(entity_id, pos) => {
                resolve_crushed(entity_id, pos, &mut game.level, &mut game.msg_log, &game.config);
            }

            Msg::Sound(cause_id, source_pos, radius) => {
                let sound_aoe =
                    aoe_fill(&game.level.map, AoeEffect::Sound, source_pos, radius, &game.config);

                for sound_pos in sound_aoe.positions() {
                    game.msg_log.log_front(Msg::SoundHitTile(cause_id, source_pos, radius, sound_pos));
                }
            }

            Msg::SoundHitTile(cause_id, source_pos, _radius, tile_pos) => {
                resolve_sound_hit_tile(cause_id, source_pos, tile_pos, game);
            }

            Msg::ItemThrow(entity_id, item_id, start, end, hard) => {
                resolve_throw_item(entity_id, item_id, start, end, hard, &mut game.level, &mut game.rng, &mut game.msg_log, &game.config);
            }

            Msg::JumpWall(entity_id, _start, end) => {
                game.msg_log.log_front(Msg::Sound(entity_id, end, game.config.sound_radius_run));
            }

            Msg::Blink(entity_id) => {
                if try_use_energy(entity_id, Skill::Blink, &mut game.level, &mut game.msg_log) {
                    resolve_blink(entity_id, &mut game.level, &mut game.rng, &mut game.msg_log);
                }
            }

            Msg::Pushed(pusher, pushed, direction, push_amount, move_into) => {
                resolve_pushed_entity(pusher, pushed, direction, push_amount, move_into, &mut game.level, &game.config, &mut game.msg_log);
            }

            Msg::Yell(entity_id) => {
                resolve_yell(entity_id, &mut game.level, &mut game.msg_log, &game.config);
            }

            Msg::Remove(entity_id) => {
                remove_entity(entity_id, &mut game.level);
            }

            Msg::Killed(_attacker, attacked, _damage) => {
                resolve_killed_entity(attacked, &mut game.level, &mut game.msg_log, &game.config);
            }

            Msg::Attack(attacker, attacked, _damage) => {
                let pos = game.level.entities.pos[&attacked];
                game.msg_log.log_front(Msg::Sound(attacker, pos, game.config.sound_radius_attack)); 
            }

            Msg::Stabbed(_attacker_id, _attacked_id) => {
                // TODO this may be superceded by Hit, although perhaps Hit
                // should break out into finer grain attacks.
                //msg_log.log(Msg::Froze(attacked_id, game.config.dagger_stab_num_turns));
            }

            Msg::HammerRaise(entity_id, item_index, dir) => {
                let item_id = game.level.entities.inventory[&entity_id][item_index];
                game.level.entities.status[&entity_id].hammer_raised = Some((item_id, dir, 1));
                game.level.entities.took_turn[&entity_id] = true;
            }

            Msg::HammerSwing(entity_id, item_id, pos) => {
                hammer_swing(entity_id, item_id, pos, &mut game.level, &mut game.msg_log);
            }

            // TODO Consider making this a Push message, splitting out that code from Action as well
            Msg::HammerHitEntity(entity_id, hit_entity) => {
                hammer_hit_entity(entity_id, hit_entity, &mut game.level, &mut game.msg_log, &game.config);
            }

            Msg::HammerHitWall(entity_id, blocked) => {
                resolve_hammer_hit_wall(entity_id, blocked, &mut game.level, &mut game.msg_log, &game.config);
            }

            Msg::TryAttack(entity_id, attack_info, attack_pos) => {
                resolve_attack(entity_id, attack_info, attack_pos, &mut game.level, &mut game.msg_log, &game.config);
            }

            Msg::TryMove(entity_id, direction, amount, move_mode) => {
                resolve_try_move(entity_id, direction, amount, move_mode, &mut game.level, &mut game.msg_log);
            }

            Msg::PickUp(entity_id) => {
                resolve_pick_item_up(entity_id, &mut game.level, &mut game.msg_log);

                // this is necessary to re-emit entity information about the item being picked up
                game.settings.map_changed = true;
            }

            Msg::StateChange(entity_id, behavior) => {
                resolve_state_change(entity_id, behavior, &mut game.level, &mut game.msg_log, &game.config);
            }

            Msg::SpikeTrapTriggered(trap, entity_id) => {
                game.level.entities.take_damage(entity_id, SPIKE_DAMAGE);

                if game.level.entities.hp[&entity_id].hp <= 0 {
                    game.level.entities.status[&entity_id].alive = false;
                    game.level.entities.blocks[&entity_id] = false;

                    game.msg_log.log(Msg::Killed(trap, entity_id, SPIKE_DAMAGE));
                }
            }

            Msg::SoundTrapTriggered(trap, entity_id) => {
                let source_pos = game.level.entities.pos[&trap];

                // the triggering entity is considered the source of the sound
                game.msg_log.log(Msg::Sound(entity_id, source_pos, game.config.sound_radius_trap));
            }

            Msg::BlinkTrapTriggered(trap, entity_id) => {
                let source_pos = game.level.entities.pos[&trap];

                if let Some(blink_pos) = find_blink_pos(source_pos, &mut game.rng, &mut game.level) {
                    game.msg_log.log(Msg::Moved(entity_id, MoveType::Blink, MoveMode::Walk, blink_pos));
                }
            }

            Msg::Froze(entity_id, num_turns) => {
                if entity_id == player_id || game.level.entities.ai.get(&entity_id).is_some() {
                    game.level.entities.status[&entity_id].frozen = num_turns;

                    // If attacking, change to investigating the current target position.
                    if let Some(Behavior::Attacking(target_id)) = game.level.entities.behavior.get(&entity_id) {
                        let target_pos = game.level.entities.pos[&target_id];
                        game.msg_log.log(Msg::StateChange(entity_id, Behavior::Investigating(target_pos)));
                    }

                    game.msg_log.log(Msg::Frozen(entity_id, true));
                }
            }

            Msg::Thaw(entity_id, _amount) => {
                if let Some(mut status) = game.level.entities.status.get_mut(&entity_id) {
                    if status.frozen > 0 {
                        status.frozen -= 1;
                    }

                    if status.frozen == 0 {
                        game.msg_log.log(Msg::Frozen(entity_id, false));
                    }
                }
            }

            Msg::FreezeTrapTriggered(trap, cause_id) => {
                freeze_trap_triggered(trap, cause_id, &mut game.level, &mut game.msg_log, &game.config);
            }

            Msg::Untriggered(_trigger, _entity_id) => {
                // NOTE nothing untriggers yet
                //untriggered(trigger, level, &mut game.msg_log);
            }

            Msg::Triggered(trigger, entity_id) => {
                resolve_triggered(trigger, entity_id, &mut game.level, &mut game.msg_log);
            }

            Msg::AddClass(class) => {
                resolve_add_class(class, game);
            }

            Msg::MoveMode(entity_id, new_move_mode) => {
                game.level.entities.move_mode[&entity_id] = new_move_mode;

                // update entities movement reach with their new move mode
                game.level.entities.movement[&entity_id] = reach_by_mode(game.level.entities.move_mode[&entity_id]);
            }

            Msg::Hit(entity_id, pos, weapon_type, attack_style) => {
                resolve_hit(entity_id, pos, weapon_type, attack_style, &mut game.level, &mut game.msg_log, &game.config);
            }

            Msg::ChangeMoveMode(entity_id, increase) => {
                change_move_mode(entity_id, increase, &mut game.level, &mut game.msg_log);
            }

            Msg::DropItem(entity_id, item_index) => {
                inventory_drop_item(entity_id, item_index as usize, &mut game.level, &mut game.msg_log);
            }

            Msg::GrassWall(entity_id, direction) => {
                if try_use_energy(entity_id, Skill::GrassWall, &mut game.level, &mut game.msg_log) {
                    resolve_grass_wall(entity_id, direction, game);
                }
            }

            Msg::GrassThrow(entity_id, direction) => {
                if try_use_energy(entity_id, Skill::GrassThrow, &mut game.level, &mut game.msg_log) {
                    resolve_grass_throw(entity_id, direction, game);
                }
            }

            Msg::GrassShoes(entity_id, _action_mode) => {
                if try_use_energy(entity_id, Skill::GrassShoes, &mut game.level, &mut game.msg_log) {
                    game.level.entities.status[&entity_id].soft_steps = SKILL_GRASS_SHOES_TURNS;
                    game.level.entities.took_turn[&entity_id] = true;
                }
            }

            Msg::GrassCover(entity_id, _action_mode) => {
                if try_use_energy(entity_id, Skill::GrassCover, &mut game.level, &mut game.msg_log) {
                    resolve_grass_cover(entity_id, game);
                }
            }

            Msg::Illuminate(entity_id, pos, amount) => {
                if try_use_energy(entity_id, Skill::Illuminate, &mut game.level, &mut game.msg_log) {
                    resolve_illuminate(entity_id, pos, amount, game);
                }
            }

            Msg::HealSkill(entity_id, amount) => {
                if try_use_energy(entity_id, Skill::Heal, &mut game.level, &mut game.msg_log) {
                    resolve_heal(entity_id, amount, game);
                }
            }

            Msg::EatHerb(entity_id, item_id) => {
                resolve_eat_herb(entity_id, item_id, game);
            }

            Msg::FarSight(entity_id, amount) => {
                if try_use_energy(entity_id, Skill::FarSight, &mut game.level, &mut game.msg_log) {
                    game.level.entities.status[&entity_id].extra_fov += amount;
                    game.level.entities.took_turn[&entity_id] = true;
                }
            }

            Msg::Ping(entity_id, pos) => {
                if try_use_energy(entity_id, Skill::Ping, &mut game.level, &mut game.msg_log) {
                    game.msg_log.log_front(Msg::Sound(entity_id, pos, game.config.ping_sound_radius));
                    game.level.entities.took_turn[&entity_id] = true;
                }
            }

            Msg::Sprint(entity_id, direction, amount) => {
                if try_use_energy(entity_id, Skill::Sprint, &mut game.level, &mut game.msg_log) {
                    game.msg_log.log(Msg::TryMove(entity_id, direction, amount, MoveMode::Run));
                    game.level.entities.took_turn[&entity_id] = true;
                }
            }

            Msg::Roll(entity_id, direction, amount) => {
                if try_use_energy(entity_id, Skill::Roll, &mut game.level, &mut game.msg_log) {
                    game.msg_log.log(Msg::TryMove(entity_id, direction, amount, MoveMode::Sneak));
                    game.level.entities.took_turn[&entity_id] = true;
                }
            }

            Msg::Rubble(entity_id, rubble_pos) => {
                if try_use_energy(entity_id, Skill::Rubble, &mut game.level, &mut game.msg_log) {
                    resolve_rubble_skill(entity_id, rubble_pos, game);
                }
            }

            Msg::StoneThrow(entity_id, target_pos) => {
                resolve_stone_thrown(entity_id, target_pos, game);
            }

            Msg::GrassBlade(entity_id, action_mode, direction) => {
                if try_use_energy(entity_id, Skill::GrassBlade, &mut game.level, &mut game.msg_log) {
                    resolve_grass_blade(entity_id, action_mode, direction, game);
                }
            }

            Msg::Reform(entity_id, pos) => {
                if game.level.map[pos].surface == Surface::Rubble &&
                   game.level.has_blocking_entity(pos).is_none() {
                    if try_use_energy(entity_id, Skill::Reform, &mut game.level, &mut game.msg_log) {
                        game.level.map[pos].surface = Surface::Floor;
                        game.level.map[pos].block_move = true;
                        game.level.entities.took_turn[&entity_id] = true;
                    }
                }
            }

            Msg::StoneSkin(entity_id) => {
                game.level.entities.status[&entity_id].stone = SKILL_STONE_SKIN_TURNS;
                game.level.entities.took_turn[&entity_id] = true;
            }

            Msg::Swap(entity_id, target_id) => {
                if try_use_energy(entity_id, Skill::Swap, &mut game.level, &mut game.msg_log) {
                    resolve_swap(entity_id, target_id, game);
                }
            }

            Msg::PassWall(entity_id, pos) => {
                if try_use_energy(entity_id, Skill::PassWall, &mut game.level, &mut game.msg_log) {
                    //game.level.entities.set_pos(entity_id, pos);
                    game.msg_log.log(Msg::Moved(entity_id, MoveType::Misc, MoveMode::Walk, pos));

                    game.level.entities.took_turn[&entity_id] = true;
                }
            }

            Msg::ArmDisarmTrap(entity_id, trap_id) => {
                game.level.entities.armed[&trap_id] = !game.level.entities.armed[&trap_id];
                game.level.entities.took_turn[&entity_id] = true;
            }

            Msg::PlaceTrap(entity_id, place_pos, trap_id) => {
                game.level.entities.set_pos(trap_id, place_pos);
                game.level.entities.armed[&trap_id] = true;

                game.level.entities.remove_from_inventory(entity_id, trap_id);
                game.level.entities.took_turn[&entity_id] = true;
            }


            Msg::Push(entity_id, direction, amount) => {
                if try_use_energy(entity_id, Skill::Push, &mut game.level, &mut game.msg_log) {
                    resolve_push_skill(entity_id, direction, amount, &mut game.level, &mut game.msg_log);
                }
            }

            Msg::FaceTowards(entity_id, pos) => {
                game.level.entities.face(entity_id, pos);
                game.msg_log.log(Msg::Facing(entity_id, game.level.entities.direction[&entity_id]));
            }

            Msg::SetFacing(entity_id, direction) => {
                game.level.entities.direction[&entity_id] = direction;
                game.msg_log.log(Msg::Facing(entity_id, direction));
            }

            Msg::AiAttack(entity_id) => {
                if let Behavior::Attacking(target_id) = game.level.entities.behavior[&entity_id] {
                    resolve_ai_attack(entity_id, target_id, &mut game.level, &mut game.msg_log, &mut game.rng, &game.config);
                } else {
                    panic!("ai attacking but not in attack state!");
                }
            }

            Msg::StartUseItem(item_id) => {
            }

            Msg::Restart => {
                resolve_restart(game);
            }

            Msg::TryPassThrough(entity_id, direction) => {
                resolve_passthrough(entity_id, direction, game);
            }

            Msg::WhirlWind(entity_id, pos) => {
                resolve_whirlwind(entity_id, pos, game);
            }

            Msg::TrySwift(entity_id, direction) => {
                resolve_swift(entity_id, direction, game);
            }

            Msg::Forget(entity_id) => {
                game.msg_log.log(Msg::StateChange(entity_id, Behavior::Idle));
            }

            _ => {
            }
        }
    }

    /* Process Player Messages */
    game.level.entities.messages[&player_id].clear();
}

fn resolve_hit(entity_id: EntityId, hit_pos: Pos, weapon_type: WeaponType, attack_style: AttackStyle, level: &mut Level, msg_log: &mut MsgLog, config: &Config) {
    // Hitting always takes a turn currently.
    level.entities.took_turn[&entity_id] = true;

    let entity_pos = level.entities.pos[&entity_id];

    if let Some(hit_entity) = level.has_blocking_entity(hit_pos) {
        if level.entities.typ[&hit_entity] == EntityType::Column {
            // if we hit a column, and this is a strong, blunt hit, then
            // push the column over.
            if weapon_type == WeaponType::Blunt && attack_style == AttackStyle::Strong {
                let dir = Direction::from_positions(entity_pos, hit_pos).unwrap();
                msg_log.log(Msg::Pushed(entity_id, hit_entity, dir, 1, false));
            }
        } else {
            // if we hit an enemy, stun them and make a sound.
            if level.entities.typ[&hit_entity] == EntityType::Enemy {
                let mut hit_sound_radius = weapon_type.sound_radius(config);
                let mut stun_turns = weapon_type.stun_turns(config);

                // whet stone passive adds to sharp weapon stun turns
                if level.entities.passive[&entity_id].whet_stone && weapon_type.sharp() {
                       stun_turns += 1;
                }

                if attack_style == AttackStyle::Strong {
                    hit_sound_radius += config.sound_radius_extra;
                    stun_turns += config.stun_turns_extra;
                }

                msg_log.log(Msg::Froze(hit_entity, stun_turns));
                msg_log.log(Msg::Sound(entity_id, hit_pos, hit_sound_radius));
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
    //reduce_item_durability(level, entity_id, item_id);
}

fn resolve_attack(entity_id: EntityId,
                  attack_info: Attack,
                  attack_pos: Pos,
                  level: &mut Level,
                  msg_log: &mut MsgLog,
                  _config: &Config) {
    let entity_pos = level.entities.pos[&entity_id];

    // any time an entity attacks, they change to standing stance
    level.entities.stance[&entity_id] = Stance::Standing;
    msg_log.log(Msg::Stance(entity_id, level.entities.stance[&entity_id]));

    match attack_info {
        Attack::Attack(target_id) => {
            attack(entity_id, target_id, level, msg_log);
        }

        Attack::Stab(target_id, move_into) => {
            stab(entity_id, target_id, &mut level.entities, msg_log);

            if let Some(item_id) = level.using(entity_id, Item::Dagger) {
                level.used_up_item(entity_id, item_id);
            }

            if move_into && entity_pos != attack_pos {
                msg_log.log(Msg::Moved(entity_id, MoveType::Misc, MoveMode::Walk, attack_pos));
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
                    level: &mut Level,
                    msg_log: &mut MsgLog) {
    // blinking uses up movement
    if level.entities.status[&entity_id].blinked {
        return;
    }

    level.entities.move_mode[&entity_id] = move_mode;

    let reach = level.entities.movement[&entity_id];
    let reach = reach.with_dist(1);

    let maybe_movement = 
        if amount == 0 {
            Some(Movement::pass(level.entities.pos[&entity_id]))
        } else {
            movement::calculate_move(direction, reach, entity_id, level)
        };

    if let Some(movement) = maybe_movement {
        if let Some(attack) = movement.attack {
            // if the movement creates an attack, attempt to perform the attack
            msg_log.log(Msg::TryAttack(entity_id, attack, movement.pos));
        } else {
            // otherwise attempt to resolve a movement
            resolve_try_movement(entity_id, direction, amount, move_mode, movement, level, msg_log);
        }
    } else {
        // monsters that are not idle, but their movement does not change their
        // position will return to idle.
        if level.entities.behavior.get(&entity_id) != None &&
           level.entities.behavior.get(&entity_id) != Some(&Behavior::Idle) {
            // this takes up the monster's turn, as they already committed to this movement
            level.entities.took_turn[&entity_id] = true;
            msg_log.log(Msg::StateChange(entity_id, Behavior::Idle));
        }
    }
}

fn resolve_try_movement(entity_id: EntityId,
                        direction: Direction,
                        amount: usize,
                        move_mode: MoveMode,
                        movement: Movement,
                        level: &mut Level,
                        msg_log: &mut MsgLog) {
    let entity_pos = level.entities.pos[&entity_id];

    match movement.typ {
        MoveType::Collide => {
            level.entities.set_pos(entity_id, movement.pos);
            msg_log.log_front(Msg::Collided(entity_id, movement.pos));
            msg_log.log_front(Msg::FaceTowards(entity_id, movement.pos));
        }

        MoveType::Pass => {
            let move_mode = level.entities.move_mode[&entity_id];
            msg_log.log_front(Msg::Moved(entity_id, MoveType::Pass, move_mode, movement.pos));
        }

        MoveType::WallKick => {
            level.entities.set_pos(entity_id, movement.pos);

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
            if level.clear_path(entity_pos, movement.pos, traps_block) {
                if movement.typ == MoveType::Move {
                    msg_log.log(Msg::Moved(entity_id, movement.typ, move_mode, movement.pos));

                    if amount > 1 {
                        msg_log.log(Msg::TryMove(entity_id, direction, amount - 1, move_mode));
                    }
                } else {
                    msg_log.log(Msg::JumpWall(entity_id, entity_pos, movement.pos));
                    panic!("Can we even get here? No clear path, but didn't decide to jump a wall?");
                }
            } else if movement.typ == MoveType::JumpWall {
                if let Some(wall_pos) = movement.wall {
                    // If the entity is not next to the wall, move them next to the wall.
                    if entity_pos != wall_pos {
                        msg_log.log(Msg::Moved(entity_id, MoveType::Move, move_mode, wall_pos));
                    }

                    // Jump over the wall
                    let after_wall_pos = direction.offset_pos(wall_pos, 1);
                    msg_log.log(Msg::JumpWall(entity_id, wall_pos, after_wall_pos));
                    // Actually move over the wall with a JumpWall move_type.
                    msg_log.log(Msg::Moved(entity_id, movement.typ, move_mode, movement.pos));

                    // If the wall jump does not end next to the wall, emit another message
                    // indicating the move from the other end of the wall to the final location.
                    if after_wall_pos != movement.pos {
                        msg_log.log(Msg::Moved(entity_id, movement.typ, move_mode, movement.pos));
                    }
                } else {
                    panic!("Wall jump with no recorded wall position!");
                }
            } else {
                panic!("Why would we not have a clear path, but have received this movement?");
            }
        }

        MoveType::Blink => {
            if !level.pos_blocked(movement.pos) {
                msg_log.log_front(Msg::Moved(entity_id, movement.typ, MoveMode::Walk, movement.pos));
            }
        }

        MoveType::Misc => {
            msg_log.log(Msg::Moved(entity_id, movement.typ, MoveMode::Walk, movement.pos));
        }
    }

    // if entity is attacking, face their target after the move
    if let Some(Behavior::Attacking(target_id)) = level.entities.behavior.get(&entity_id) {
        let target_pos = level.entities.pos[target_id];
        msg_log.log(Msg::FaceTowards(entity_id, target_pos));
    }
}

fn resolve_push_skill(entity_id: EntityId,
                      direction: Direction,
                      amount: usize,
                      level: &mut Level,
                      msg_log: &mut MsgLog) {
    let pos = level.entities.pos[&entity_id];

    let push_pos = direction.offset_pos(pos, 1);
    for other_id in level.has_entities(push_pos) {
        if level.entities.typ[&other_id] == EntityType::Enemy {
            let dxy = sub_pos(push_pos, pos);
            let direction = Direction::from_dxy(dxy.x, dxy.y).unwrap();
            let move_into = false;
            msg_log.log(Msg::Pushed(entity_id, other_id, direction, amount, move_into));
            msg_log.log(Msg::Froze(other_id, SKILL_PUSH_STUN_TURNS));
        }
    }
    level.entities.took_turn[&entity_id] = true;
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

fn resolve_hammer_hit_wall(entity: EntityId, blocked: Blocked, level: &mut Level, msg_log: &mut MsgLog, config: &Config) {
    let entity_pos = level.entities.pos[&entity];
    let hit_pos = blocked.end_pos;

    // if hit water, do nothing
    if level.map[hit_pos].tile_type == TileType::Water {
        return;
    }

    if level.map[hit_pos].block_move {
        // hammer hit a full tile wall
        if level.map[hit_pos].surface == Surface::Floor {
            level.map[hit_pos].surface = Surface::Rubble;
        }

        level.map[hit_pos].block_move = false;
        level.map[hit_pos].block_sight = false;
        level.map[hit_pos].tile_type = TileType::Empty;

        let next_pos = next_from_to(entity_pos, hit_pos);
        msg_log.log_front(Msg::Crushed(entity, next_pos)); 
        msg_log.log_front(Msg::Sound(entity, blocked.end_pos, config.sound_radius_attack)); 
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
            level.map[wall_loc].left_wall = Wall::Empty;
        } else {
            level.map[wall_loc].bottom_wall = Wall::Empty;
        }

        msg_log.log_front(Msg::Crushed(entity, blocked.end_pos));
    }
}

fn resolve_pushed_entity(pusher: EntityId,
                         pushed: EntityId,
                         direction: Direction,
                         push_amount: usize,
                         move_into: bool,
                         level: &mut Level,
                         config: &Config,
                         msg_log: &mut MsgLog) {
    let pushed_pos = level.entities.pos[&pushed];
    let pusher_pos = level.entities.pos[&pusher];

    if level.entities.typ[&pushed] == EntityType::Column {
        let entity_diff = sub_pos(pushed_pos, pusher_pos);
        let next_pos = next_pos(pusher_pos, entity_diff);
        let blocked = level.map.path_blocked_move(pushed_pos, next_pos); 

        if blocked == None {
            level.entities.mark_for_removal(pushed);

            msg_log.log_front(Msg::Crushed(pusher, next_pos));

            if move_into && pos_on_map(level.entities.pos[&pusher]) {
                msg_log.log(Msg::Moved(pusher, MoveType::Misc, MoveMode::Walk, pushed_pos));
            }
        }
    } else if level.entities.status[&pushed].alive {
        let continue_push = 
            push_attack(pusher, pushed, direction, move_into, level, config, msg_log);

        if continue_push && push_amount > 1 {
            msg_log.log(Msg::Pushed(pusher, pushed, direction, push_amount - 1, move_into));
        }
    } else {
        panic!("Tried to push entity {:?}, alive = {}!",
               level.entities.typ[&pushed], level.entities.status[&pushed].alive);
    }
    level.entities.took_turn[&pusher] = true;
}

fn resolve_crushed(entity_id: EntityId, pos: Pos, level: &mut Level, msg_log: &mut MsgLog, config: &Config) {
    if level.map[pos].tile_type.is_wall() {
        level.map[pos] = Tile::empty();
    }
    level.map[pos].surface = Surface::Rubble;

    for crushed_id in level.has_entities(pos) {
        if crushed_id == entity_id {
            continue;
        }

        if level.entities.typ[&crushed_id] == EntityType::Column {
            let pos_diff = sub_pos(pos, level.entities.pos[&entity_id]);
            let next_pos = next_pos(level.entities.pos[&entity_id], pos_diff);

            msg_log.log_front(Msg::Crushed(crushed_id, next_pos));
        }

        if let Some(hp) = level.entities.hp.get(&crushed_id) {
            msg_log.log(Msg::Killed(entity_id, crushed_id, hp.hp));
        } else if level.entities.item.get(&crushed_id).is_none() &&
                  level.entities.name[&crushed_id] != EntityName::Mouse &&
                  level.entities.name[&crushed_id] != EntityName::Cursor {
            // the entity will be removed, such as an item.
            level.entities.mark_for_removal(crushed_id);
        }
    }

    msg_log.log_front(Msg::Sound(entity_id, pos, config.sound_radius_crushed));
}

fn resolve_throw_item(player_id: EntityId,
                      item_id: EntityId,
                      start_pos: Pos,
                      end_pos: Pos,
                      hard: bool,
                      level: &mut Level,
                      rng: &mut Rand32,
                      msg_log: &mut MsgLog,
                      config: &Config) {

    if start_pos == end_pos {
        panic!("Is it possible to throw an item and have it end where it started? Apparently yes")
    }

    let throw_line = line(start_pos, end_pos);

    // get target position in direction of player click
    let end_pos =
        Pos::from(throw_line.into_iter().take(PLAYER_THROW_DIST).last().unwrap());

    let hit_pos = level.throw_towards(start_pos, end_pos);

    if let Some(hit_entity) = level.has_blocking_entity(hit_pos) {
        if level.entities.typ[&hit_entity] == EntityType::Enemy {
            let mut stun_turns = level.entities.item[&item_id].throw_stun_turns(config);

            if level.entities.passive[&player_id].stone_thrower {
                stun_turns += 1;
            }

            if hard {
                stun_turns += 1;
            }

            if stun_turns > 0 {

                msg_log.log(Msg::Froze(hit_entity, stun_turns));
            }

            let player_pos = level.entities.pos[&player_id];
            level.entities.messages[&hit_entity].push(Message::Hit(player_pos));
        }
    }

    level.entities.set_pos(item_id, start_pos);

    //let movement = Movement::step_to(hit_pos);
    msg_log.log(Msg::Moved(item_id, MoveType::Misc, MoveMode::Walk, hit_pos));

    level.entities.remove_item(player_id, item_id);
    level.entities.took_turn[&player_id] = true;

    // NOTE the radius here is the stone radius, regardless of item type
    msg_log.log_front(Msg::Sound(player_id, hit_pos, config.sound_radius_stone));

    /* Resolve Specific Items */
    if level.entities.item[&item_id] == Item::SeedOfStone {
        level.map[hit_pos] = Tile::wall();
        // this is playing a little fast and lose- we assume that if
        // the seed of stone hits a tile, that any entity at that tile
        // is something we can destroy like a sword or grass entity.
        for entity_id in level.get_entities_at_pos(hit_pos) {
            remove_entity(entity_id, level);
        }
        remove_entity(item_id, level);
    } else if level.entities.item[&item_id] == Item::SeedCache {
        for seed_pos in floodfill(&level.map, hit_pos, SEED_CACHE_RADIUS) {
            if rng_trial(rng, 0.70) {
                ensure_grass(&mut level.entities, seed_pos, msg_log);
            }
        }
    } else if level.entities.item[&item_id] == Item::SmokeBomb {
        make_smoke(&mut level.entities, config, hit_pos, config.smoke_bomb_fov_block, msg_log);
        for smoke_pos in floodfill(&level.map, hit_pos, SMOKE_BOMB_RADIUS) {
            if smoke_pos != hit_pos {
                if rng_trial(rng, 0.30) {
                    make_smoke(&mut level.entities, config, smoke_pos, config.smoke_bomb_fov_block, msg_log);
                }
            }
        }
    } else if level.entities.item[&item_id] == Item::LookingGlass {
        make_magnifier(&mut level.entities, config, hit_pos, config.looking_glass_magnify_amount, msg_log);
    } else if level.entities.item[&item_id] == Item::GlassEye {
        for pos in level.map.pos_in_radius(hit_pos, GLASS_EYE_RADIUS) {
            for eyed_id in level.get_entities_at_pos(pos) {
                // check if outside FoV. Inside entities are already visible,
                // and entities on the edge should already have impressions, so
                // we don't need to make one here.
                if level.entities.typ[&eyed_id] == EntityType::Enemy &&
                   level.is_in_fov(player_id, eyed_id) == FovResult::Outside {
                    msg_log.log(Msg::Impression(pos));
                }
            }
        }
    } else if level.entities.item[&item_id] == Item::Teleporter {
        let end_x = rng_range_i32(rng, hit_pos.x - 1, hit_pos.x + 1);
        let end_y = rng_range_i32(rng, hit_pos.y - 1, hit_pos.y + 1);
        let mut end_pos = Pos::new(end_x, end_y);
        if !level.map.is_within_bounds(end_pos) {
            end_pos = hit_pos;
        }
        msg_log.log_front(Msg::Moved(player_id, MoveType::Blink, MoveMode::Walk, end_pos));
        remove_entity(item_id, level);
    }

    msg_log.log(Msg::ItemLanded(item_id, start_pos, hit_pos));
}

fn resolve_interaction(entity_id: EntityId,
                       interact_pos: Pos,
                       level: &mut Level, 
                       msg_log: &mut MsgLog,
                       _config: &Config) {
    let pos = level.entities.pos[&entity_id];

    if pos == interact_pos {
        if let Some(_item_id) = level.item_at_pos(pos) {
            msg_log.log(Msg::PickUp(entity_id));
        }
    } else {
        for other_id in level.has_entity(interact_pos) {
            if level.entities.trap.get(&other_id).is_some() {
                msg_log.log(Msg::ArmDisarmTrap(entity_id, other_id));
                break;
            }
        }
    }
}

fn resolve_moved_message(entity_id: EntityId,
                         move_type: MoveType,
                         move_mode: MoveMode,
                         pos: Pos,
                         level: &mut Level,
                         msg_log: &mut MsgLog,
                         rng: &mut Rand32,
                         config: &Config) {
    let player_id = level.find_by_name(EntityName::Player).unwrap();
    let original_pos = level.entities.pos[&entity_id];

    level.entities.set_pos(entity_id, pos);
    level.entities.took_turn[&entity_id] = true;

    if move_type != MoveType::Blink {
        if pos != original_pos && level.entities.typ[&entity_id] == EntityType::Enemy {
            msg_log.log_front(Msg::Sound(entity_id, original_pos, config.sound_radius_monster));
            msg_log.log_front(Msg::Sound(entity_id, pos, config.sound_radius_monster));
        } else if level.entities.typ[&entity_id] == EntityType::Item {
            // Dropping the item at your feet is silent. Other item movements make a sound.
            if pos != original_pos {
                msg_log.log_front(Msg::Sound(entity_id, pos, config.sound_radius_stone));
            }
        } else {
            // Only normal movements update the stance. Others like Blink leave it as-is.
            if move_type != MoveType::Blink && move_type != MoveType::Misc {
                if let Some(stance) = level.entities.stance.get(&entity_id) {
                    level.entities.stance[&entity_id] = update_stance(move_type, move_mode, *stance);
                    msg_log.log(Msg::Stance(entity_id, level.entities.stance[&entity_id]));
                }
            }

            // make a noise based on how fast the entity is moving and the terrain
            if pos != original_pos {
                make_move_sound(entity_id, original_pos, pos, move_mode, level, msg_log, config);
            }
        } // NOTE other entities do not make sounds on movement, such as items
    }

    // This is cleared in the start of the next turn when the game is stepped.
    level.entities.status[&entity_id].blinked = move_type == MoveType::Blink;

    // check if player walks on energy
    if entity_id == player_id {
        for other_id in level.entities.ids.clone().iter() {
            if level.entities.pos[other_id] == pos && 
            level.entities.typ[other_id] == EntityType::Energy {
                level.entities.energy[&player_id] += 1;
                level.entities.needs_removal[other_id] = true;
                msg_log.log(Msg::GainEnergy(player_id, 1));
            }
        }
    }

    if original_pos != pos {
        resolve_triggered_traps(entity_id, original_pos, level, rng, msg_log);
    }

    // check for passing turn while the hammer is raised
    if move_type == MoveType::Pass {
        if let Some((item_id, dir, turns)) = level.entities.status[&entity_id].hammer_raised {
            if turns == 0 {
                let hit_pos = dir.offset_pos(original_pos, 1);
                msg_log.log(Msg::HammerSwing(entity_id, item_id, hit_pos));
                level.entities.status[&entity_id].hammer_raised = None;
            }
        }
    }

    // Check if we trampled any grass.
    // This only happens for non-item moves that change position, and are not teleports.
    if level.entities.item.get(&entity_id).is_none() && original_pos != pos && move_type != MoveType::Blink {
        trample_grass_walls(level, original_pos, pos);
    }

    if level.map[pos].block_sight && level.map[pos].surface == Surface::Grass {
        level.map[pos].block_sight = false;
    }

    // if entity is a monster, which is also alert, and there is a path to the player,
    // then face the player
    if let Some(target_pos) = level.entities.target(entity_id) {
        if level.could_see(entity_id, target_pos, config) {
            msg_log.log_front(Msg::FaceTowards(entity_id, target_pos));
        }
    } else {
        let diff = sub_pos(pos, original_pos);
        if let Some(dir) = Direction::from_dxy(diff.x, diff.y) {
            msg_log.log_front(Msg::SetFacing(entity_id, dir));
        }
    }

    // For blinking movements, check if the entity disappears from the perspective of an entity.
    if move_type == MoveType::Blink {
        for behave_id in level.entities.behavior.ids.iter() {
            if level.entities.behavior[behave_id] == Behavior::Attacking(entity_id) {
                level.entities.messages[behave_id].push(Message::Disappeared(entity_id));
            }
        }
    }
}

fn resolve_triggered_traps(entity_id: EntityId,
                           original_pos: Pos,
                           level: &mut Level,
                           rng: &mut Rand32,
                           msg_log: &mut MsgLog) {
    // check for light touch first, in case it prevents a trap from triggering.
    if level.entities.passive.get(&entity_id).is_some() &&
       level.entities.passive[&entity_id].light_touch   &&
       rng_trial(rng, 0.5) {
        return;
    }

    // get a list of triggered traps
    let traps: Vec<EntityId> = level.entities.triggered_traps(level.entities.pos[&entity_id]);

    // Check if the entity hit a trap
    for trap in traps.iter() {
        match level.entities.trap[trap] {
            Trap::Spikes => {
                msg_log.log(Msg::SpikeTrapTriggered(*trap, entity_id));
                level.entities.mark_for_removal(*trap);
            }

            Trap::Sound => {
                msg_log.log(Msg::SoundTrapTriggered(*trap, entity_id));
                level.entities.needs_removal[trap] = true;
                level.entities.mark_for_removal(*trap);
            }

            Trap::Blink => {
                msg_log.log(Msg::BlinkTrapTriggered(*trap, entity_id));
                level.entities.mark_for_removal(*trap);
            }

            Trap::Freeze => {
                msg_log.log(Msg::FreezeTrapTriggered(*trap, entity_id));
                level.entities.mark_for_removal(*trap);
            }
        }
    }

    // Resolve triggers
    for key in level.entities.ids.iter() {
        // key is a trigger
        if level.entities.typ[key] == EntityType::Trigger {
            // stepped on trigger
           if level.entities.pos[key] == level.entities.pos[&entity_id] {
               msg_log.log_front(Msg::Triggered(*key, entity_id));
           }

            // stepped off of trigger
           if level.entities.pos[key] == original_pos &&
              level.entities.status[key].active {
               msg_log.log_front(Msg::Untriggered(*key, entity_id));
           }
        }
    }
}


fn resolve_ai_attack(entity_id: EntityId,
                     target_id: EntityId,
                     level: &mut Level,
                     msg_log: &mut MsgLog,
                     rng: &mut Rand32,
                     config: &Config) {
    let target_pos = level.entities.pos[&target_id];

    let attack_reach = level.entities.attack[&entity_id];
    let can_hit_target =
        ai_can_hit_target(level, entity_id, target_pos, &attack_reach, config);

    if level.entities.is_dead(target_id) {
        level.entities.took_turn[&entity_id] = true;
        msg_log.log(Msg::StateChange(entity_id, Behavior::Investigating(target_pos)));
    } else if let Some(_hit_pos) = can_hit_target {
        let mut can_attack = true;
        // quick reflexes
        if level.entities.passive.get(&target_id).is_some() && level.entities.passive[&target_id].quick_reflexes {
            if rng_trial(rng, SKILL_QUICK_REFLEXES_PERCENT) {
                can_attack = false;
                msg_log.log(Msg::Dodged(target_id));
            }
        }

        if can_attack {
            let attack_info = Attack::Attack(target_id);
            msg_log.log(Msg::TryAttack(entity_id, attack_info, target_pos));
        }
    } else if level.is_in_fov(entity_id, target_id) != FovResult::Inside {
        // If the target disappeared, change to idle- there is no need to
        // pursue their last position if we saw them blink away.
        if level.entities.target_disappeared(entity_id).is_some() {
            msg_log.log(Msg::StateChange(entity_id, Behavior::Idle));
        } else {
            // If we lose the target, end the turn and investigate their current position.
            // This allows the golem to 'see' a player move behind a wall and still investigate
            // them instead of losing track of their position.
            level.entities.took_turn[&entity_id] = true;
            let current_target_pos = level.entities.pos[&target_id];
            msg_log.log(Msg::StateChange(entity_id, Behavior::Investigating(current_target_pos)));
        }
    } else {
        // can see target, but can't hit them. try to move to a position where we can hit them
        let maybe_pos = ai_move_to_attack_pos(entity_id, target_id, level, config);

        if let Some(move_pos) = maybe_pos {
            // try to move in the given direction
            let entity_pos = level.entities.pos[&entity_id];
            let direction = Direction::from_positions(entity_pos, move_pos).unwrap();
            msg_log.log(Msg::TryMove(entity_id, direction, 1, MoveMode::Walk));
        } else {
            // if we can't move anywhere, we just end our turn
            level.entities.took_turn[&entity_id] = true;
        }
    }
}

fn resolve_yell(entity_id: EntityId, level: &mut Level, msg_log: &mut MsgLog, config: &Config) {
    let pos = level.entities.pos[&entity_id];
    msg_log.log_front(Msg::Sound(entity_id, pos, config.yell_radius));
    level.entities.took_turn[&entity_id] = true;
}

fn resolve_blink(entity_id: EntityId, level: &mut Level, rng: &mut Rand32, msg_log: &mut MsgLog) {
    let entity_pos = level.entities.pos[&entity_id];

    if let Some(blink_pos) = find_blink_pos(entity_pos, rng, level) {
        msg_log.log_front(Msg::Moved(entity_id, MoveType::Blink, MoveMode::Walk, blink_pos));
    } else {
        msg_log.log(Msg::FailedBlink(entity_id));
    }

    level.entities.took_turn[&entity_id] = true;
}

fn resolve_state_change(entity_id: EntityId, behavior: Behavior, level: &mut Level, msg_log: &mut MsgLog, config: &Config) {
    let original_behavior = level.entities.behavior[&entity_id];

    // If the entity hasn't completed a turn, the state change continues their turn.
    dbg!(behavior);
    if !level.entities.took_turn[&entity_id] &&
        level.entities.behavior[&entity_id] != original_behavior &&
        !matches!(behavior, Behavior::Investigating(_)) {
       ai_take_turn(entity_id, level, config, msg_log);
       dbg!("taking turn!");
    }
    dbg!(level.entities.took_turn[&entity_id]);

    level.entities.behavior[&entity_id] = behavior;

    if mem::discriminant(&behavior) != mem::discriminant(&original_behavior) {
        msg_log.log(Msg::BehaviorChanged(entity_id, behavior));
    }
}

fn resolve_add_class(class: EntityClass, game: &mut Game) {
    let player_id = game.level.find_by_name(EntityName::Player).unwrap();

    game.level.entities.skills[&player_id].clear();
    game.level.entities.class[&player_id] = class;

    fn add_skill(game: &mut Game, entity_id: EntityId, skill: Skill) {
        game.level.entities.add_skill(entity_id, skill);
        game.msg_log.log(Msg::AddSkill(skill));
    }

    match class {
        EntityClass::General => {
            add_skill(game, player_id, Skill::Blink);
            add_skill(game, player_id, Skill::Sprint);
            add_skill(game, player_id, Skill::Roll);
        }

        EntityClass::Monolith => {
            add_skill(game, player_id, Skill::PassWall);
            add_skill(game, player_id, Skill::Rubble);
            add_skill(game, player_id, Skill::StoneThrow);
            add_skill(game, player_id, Skill::Reform);
            add_skill(game, player_id, Skill::StoneSkin);
        }

        EntityClass::Grass => {
            add_skill(game, player_id, Skill::GrassWall);
            add_skill(game, player_id, Skill::GrassThrow);
            add_skill(game, player_id, Skill::GrassBlade);
            add_skill(game, player_id, Skill::GrassCover);
            add_skill(game, player_id, Skill::GrassShoes);
        }

        EntityClass::Clockwork => {
            add_skill(game, player_id, Skill::Push);
        }

        EntityClass::Hierophant => {
            add_skill(game, player_id, Skill::Illuminate);
            add_skill(game, player_id, Skill::Heal);
            add_skill(game, player_id, Skill::FarSight);
            add_skill(game, player_id, Skill::Ping);
        }

        EntityClass::Wind => {
            add_skill(game, player_id, Skill::PassThrough);
            add_skill(game, player_id, Skill::WhirlWind);
            add_skill(game, player_id, Skill::Swift);
        }
    }
}

fn resolve_grass_wall(entity_id: EntityId, direction: Direction, game: &mut Game) {
    let entity_pos = game.level.entities.pos[&entity_id];

    game.level.entities.took_turn[&entity_id] = true;

    match direction {
        Direction::Left | Direction::Right | Direction::Up | Direction::Down => {
            game.level.map.place_intertile_wall(entity_pos, Surface::Grass, direction);
            let next_to = direction.clockwise().clockwise().offset_pos(entity_pos, 1);
            game.level.map.place_intertile_wall(next_to, Surface::Grass, direction);
            let next_to = direction.counterclockwise().counterclockwise().offset_pos(entity_pos, 1);
            game.level.map.place_intertile_wall(next_to, Surface::Grass, direction);
        }

        Direction::DownLeft | Direction::DownRight | Direction::UpLeft | Direction::UpRight => {
            let next_to = direction.clockwise().offset_pos(entity_pos, 1);
            game.level.map.place_intertile_wall(next_to, Surface::Grass, direction.counterclockwise());
            let next_to = direction.counterclockwise().offset_pos(entity_pos, 1);
            game.level.map.place_intertile_wall(next_to, Surface::Grass, direction.clockwise());
        }
    }
}

fn resolve_grass_throw(entity_id: EntityId, direction: Direction, game: &mut Game) {
    let pos = game.level.entities.pos[&entity_id];

    for grass_pos in line_inclusive(pos, direction.offset_pos(pos, SKILL_GRASS_THROW_RADIUS as i32)) {

        // percent chance of not marking a tile
        if rng_trial(&mut game.rng, 0.75) {
            if game.level.map.is_within_bounds(grass_pos) && game.level.map[grass_pos].tile_type == TileType::Empty {
                game.level.map[grass_pos].surface = Surface::Grass;
                ensure_grass(&mut game.level.entities, grass_pos, &mut game.msg_log);
            }
        }

        // percent chance of marking a nearby tile
        if rng_trial(&mut game.rng, 0.35) {
            let other_pos;
            if rng_trial(&mut game.rng, 0.5) {
                other_pos = direction.clockwise().clockwise().offset_pos(grass_pos, 1);
            } else {
                other_pos = direction.counterclockwise().counterclockwise().offset_pos(grass_pos, 1);
            }

            if game.level.map.is_within_bounds(other_pos) && game.level.map[other_pos].tile_type == TileType::Empty {
                game.level.map[other_pos].surface = Surface::Grass;
                ensure_grass(&mut game.level.entities, other_pos, &mut game.msg_log);
            }
        }
    }
    // NOTE old cone style
    //for grass_pos in Cone::new(pos, direction, SKILL_GRASS_THROW_RADIUS as i32) {
    //    if game.level.map.is_within_bounds(grass_pos) && game.level.map[grass_pos].tile_type == TileType::Empty {
    //        game.level.map[grass_pos].surface = Surface::Grass;
    //        ensure_grass(&mut game.level.entities, grass_pos, &mut game.msg_log);
    //    }
    //}
    game.level.entities.took_turn[&entity_id] = true;
}

fn resolve_grass_cover(entity_id: EntityId, game: &mut Game) {
    let entity_pos = game.level.entities.pos[&entity_id];
    let facing = game.level.entities.direction[&entity_id];
    let next_pos = facing.offset_pos(entity_pos, 1);
    game.level.map[next_pos] = Tile::tall_grass();
    ensure_grass(&mut game.level.entities, next_pos, &mut game.msg_log);
    game.level.entities.took_turn[&entity_id] = true;
}

fn resolve_illuminate(entity_id: EntityId, pos: Pos, amount: usize, game: &mut Game) {
    let light = make_light(&mut game.level.entities, &game.config, pos, &mut game.msg_log);
    game.level.entities.illuminate[&light] = amount;

    game.level.entities.took_turn[&entity_id] = true;
}

fn resolve_heal(entity_id: EntityId, amount: usize, game: &mut Game) {
    let old_hp = game.level.entities.hp[&entity_id].hp;
    game.level.entities.hp[&entity_id].hp = 
        std::cmp::min(game.level.entities.hp[&entity_id].max_hp,
                      game.level.entities.hp[&entity_id].hp + amount as i32);

    game.level.entities.took_turn[&entity_id] = true;

    let amount = game.level.entities.hp[&entity_id].hp - old_hp;
    if amount > 0 {
        let max_hp = game.level.entities.hp[&entity_id].max_hp;
        game.msg_log.log(Msg::Healed(entity_id, amount, max_hp));
    }
}

fn resolve_eat_herb(entity_id: EntityId, item_id: EntityId, game: &mut Game) {
    let heal_amount = game.level.entities.hp[&entity_id].max_hp - game.level.entities.hp[&entity_id].hp;
    let max_hp = game.level.entities.hp[&entity_id].max_hp;
    game.msg_log.log(Msg::Healed(entity_id, heal_amount, max_hp));
   
    game.level.entities.hp[&entity_id].hp = game.level.entities.hp[&entity_id].max_hp;
    game.level.entities.remove_item(entity_id, item_id);
    game.msg_log.log(Msg::Remove(item_id));

    game.level.entities.took_turn[&entity_id] = true;
}

fn resolve_rubble_skill(entity_id: EntityId, rubble_pos: Pos, game: &mut Game) {
    let pos = game.level.entities.pos[&entity_id];
    let blocked = game.level.map.path_blocked_move(pos, rubble_pos);

    if let Some(blocked) = blocked {
        resolve_rubble(blocked, &mut game.level.map);
    } else if let Some(blocked_id) = game.level.has_blocking_entity(rubble_pos) {
        // if we hit a column, turn it into rubble
        if game.level.entities.typ[&blocked_id] == EntityType::Column {
            remove_entity(blocked_id, &mut game.level);
            game.level.map[rubble_pos].surface = Surface::Rubble;
        }
    }

    game.level.entities.took_turn[&entity_id] = true;
}

fn resolve_stone_thrown(entity_id: EntityId, target_pos: Pos, game: &mut Game) {
    let entity_pos = game.level.entities.pos[&entity_id];

    let mut rubble_pos = None;
    if game.level.map[entity_pos].surface == Surface::Rubble {
        rubble_pos = Some(entity_pos);
    }

    for pos in game.level.map.neighbors(entity_pos) {
        if game.level.map[pos].surface == Surface::Rubble {
            rubble_pos = Some(pos);
        }
    }

    if let Some(rubble_pos) = rubble_pos {
        for target_id in game.level.get_entities_at_pos(target_pos) {
            let target_pos = game.level.entities.pos[&target_id];
            let direction = Direction::from_positions(entity_pos, target_pos).expect("The player is on the same tile as a column?");

            if game.level.entities.typ[&target_id] == EntityType::Enemy {
                let move_into = false;
                push_attack(entity_id, target_id, direction, move_into, &mut game.level, &game.config, &mut game.msg_log);
            } else if game.level.entities.typ[&target_id] == EntityType::Column {
                game.msg_log.log(Msg::Pushed(entity_id, target_id, direction, 1, false));
            }
        }

        game.level.map[rubble_pos].surface = Surface::Floor;

        game.level.entities.took_turn[&entity_id] = true;
    }
}

fn resolve_grass_blade(entity_id: EntityId, action_mode: ActionMode, direction: Direction, game: &mut Game) {
    let pos = game.level.entities.pos[&entity_id];

    match action_mode {
        ActionMode::Primary => {
            // TODO anything?
        }

        ActionMode::Alternate => {
            // TODO anything?
        }
    }

    let attack_pos = direction.offset_pos(pos, 1);
    let targets = game.level.get_entities_at_pos(attack_pos);

    for target_id in targets {
        if game.level.entities.typ[&target_id] == EntityType::Enemy {
            let attack = Attack::Stab(target_id, false);
            resolve_attack(entity_id, attack, attack_pos, &mut game.level, &mut game.msg_log, &game.config);

            break;
        }
    }

    game.level.entities.took_turn[&entity_id] = true;
}

fn resolve_swap(entity_id: EntityId, target_id: EntityId, game: &mut Game) {
    let entity_dir = game.level.entities.direction[&entity_id];
    let target_dir = game.level.entities.direction[&target_id];

    let start_pos = game.level.entities.pos[&entity_id];
    let end_pos = game.level.entities.pos[&target_id];
    game.level.entities.set_pos(entity_id, end_pos);
    game.level.entities.set_pos(target_id, start_pos);

    game.msg_log.log(Msg::SetFacing(entity_id, target_dir));
    game.msg_log.log(Msg::SetFacing(target_id, entity_dir));

    game.level.entities.took_turn[&entity_id] = true;
}

fn resolve_restart(game: &mut Game) {
    map_construct(&game.settings.map_load_config.clone(), game);

    let player_id = game.level.find_by_name(EntityName::Player).unwrap();

    if game.level.entities.hp[&player_id].hp != game.level.entities.hp[&player_id].max_hp {
        let hp_diff = game.level.entities.hp[&player_id].max_hp - game.level.entities.hp[&player_id].hp;
        game.level.entities.hp[&player_id].hp = 
            game.level.entities.hp[&player_id].max_hp;
        game.msg_log.log(Msg::Healed(player_id, hp_diff, game.level.entities.hp[&player_id].max_hp));
    }

    if game.config.player_energy > game.level.entities.energy[&player_id] {
        let energy_diff = game.config.player_energy - game.level.entities.energy[&player_id];
        game.level.entities.energy[&player_id] = game.config.player_energy;
        game.msg_log.log(Msg::GainEnergy(player_id, energy_diff));
    }

    game.level.entities.skills[&player_id].clear();
    game.level.entities.inventory[&player_id].clear();
    game.level.entities.class[&player_id] = EntityClass::General;
    game.level.entities.status[&player_id] = StatusEffect::default();
    game.level.entities.status[&player_id].alive = true;

    game.settings.turn_count = 0;
    game.settings.cursor = None;
    game.settings.use_dir = None;
}

fn resolve_passthrough(entity_id: EntityId, direction: Direction, game: &mut Game) {
    let entity_pos = game.level.entities.pos[&entity_id];
    let dest = direction.offset_pos(entity_pos, 3);
    let next_pos = direction.offset_pos(entity_pos, 1);

    let pass_through_entity = game.level.has_blocking_entity(next_pos).is_some();
    let map_clear_path = game.level.map.path_blocked(entity_pos, dest, BlockedType::Move).is_none();
    let dest_clear_pos = !game.level.pos_blocked(dest);
    if map_clear_path && dest_clear_pos && pass_through_entity {
        game.msg_log.log(Msg::Moved(entity_id, MoveType::Misc, MoveMode::Walk, dest));

        for pos in line_inclusive(entity_pos, dest) {
            for other_id in game.level.get_entities_at_pos(pos) {
                if game.level.entities.typ[&other_id] == EntityType::Enemy {
                    game.msg_log.log(Msg::Forget(other_id));
                }
            }
        }
        game.msg_log.log(Msg::PassThrough(entity_id));
    }
}

fn resolve_whirlwind(entity_id: EntityId, pos: Pos, game: &mut Game) {
    let entity_pos = game.level.entities.pos[&entity_id];
    let mut near_walls = false;
    for dir in &Direction::directions() {
        let dir_pos = dir.offset_pos(pos, 1);

        if !game.level.map.is_within_bounds(dir_pos) {
            continue;
        }

        if game.level.map.move_blocked(pos, dir_pos, BlockedType::Move).is_some() {
            near_walls = true;
            break;
        }
    }

    let traps_block = false;
    if !near_walls && game.level.clear_path(entity_pos, pos, traps_block) {
        game.msg_log.log(Msg::Moved(entity_id, MoveType::Blink, MoveMode::Walk, pos));
    } // NOTE could create a failed whirlwind message, or generic failed skill message
}

fn resolve_swift(entity_id: EntityId, direction: Direction, game: &mut Game) {
    let entity_pos = game.level.entities.pos[&entity_id];
    let dest = direction.offset_pos(entity_pos, SKILL_SWIFT_DISTANCE as i32);

    if game.level.map.is_within_bounds(dest) {

        let mut near_walls = false;
        for dir in &Direction::directions() {
            if game.level.map.move_blocked(dest, dir.offset_pos(dest, 1), BlockedType::Move).is_some() {
                near_walls = true;
                break;
            }
        }

        let traps_block = false;
        if !near_walls && game.level.clear_path(entity_pos, dest, traps_block) {
            game.msg_log.log(Msg::Moved(entity_id, MoveType::Blink, MoveMode::Walk, dest));
            game.msg_log.log(Msg::Swift(entity_id, dest));
        }
    }
}

fn resolve_killed_entity(attacked: EntityId, level: &mut Level, msg_log: &mut MsgLog, config: &Config) {
    let attacked_pos = level.entities.pos[&attacked];

    // if the attacked entities position is not blocked
    if !level.map[attacked_pos].block_move {
        // all non-player entities leave rubble
        if level.entities.typ[&attacked] != EntityType::Player {
            level.map[attacked_pos].surface = Surface::Rubble;
        }

        // leave energy ball
        if level.entities.typ[&attacked] == EntityType::Enemy {
            make_energy(&mut level.entities, config, attacked_pos, msg_log);
        }
    }

    if let Some(hp) = level.entities.hp.get_mut(&attacked) {
        hp.hp = 0;
    }

    remove_entity(attacked, level);
}

fn resolve_pick_item_up(entity_id: EntityId, level: &mut Level, msg_log: &mut MsgLog) {
    let entity_pos = level.entities.pos[&entity_id];

    if let Some(item_id) = level.item_at_pos(entity_pos) {
        msg_log.log(Msg::PickedUp(entity_id, item_id));

        let to_drop_index = level.entities.pick_up_item(entity_id, item_id);

        if let Some(to_drop_index) = to_drop_index {
            msg_log.log(Msg::DropItem(entity_id, to_drop_index as u64));
        }
    }
}

fn resolve_triggered(trigger: EntityId, entity_id: EntityId, level: &mut Level, msg_log: &mut MsgLog) {
    if level.entities.name[&trigger] == EntityName::GateTrigger {
        let wall_pos = level.entities.gate_pos[&trigger];

        if level.entities.status[&trigger].active {
            // raise the gate
            level.entities.status[&trigger].active = false;

            // only raise if no entities are on the square.
            // otherwise wait for a move that leaves the trigger unblocked.
            if level.has_entity(wall_pos).is_none() {
                level.map[wall_pos] = Tile::wall();
            }
        } else {
            level.entities.status[&trigger].active = true;
            level.map[wall_pos] = Tile::empty();
        }

        msg_log.log(Msg::GateTriggered(trigger, entity_id));
    }
}

fn resolve_sound_hit_tile(cause_id: EntityId, source_pos: Pos, tile_pos: Pos, game: &mut Game) {
    for heard_id in game.level.get_entities_at_pos(tile_pos) {
        if heard_id != cause_id {
            // TODO replace with an Alerted message
            game.level.entities.messages[&heard_id].push(Message::Sound(source_pos));
        }
    }
}

#[test]
pub fn test_ai_start_investigating_doesnt_take_turn() {
    let mut config = Config::from_file("../config.yaml");
    config.map_load = MapLoadConfig::Empty;
    let mut game = Game::new(0, config.clone());
    map_construct(&config.map_load, &mut game);

    let player_id = game.level.find_by_name(EntityName::Player).unwrap();

    // Place a golem a few tiles to the right.
    let gol = make_gol(&mut game.level.entities, &game.config, Pos::new(3, 0), &mut game.msg_log);

    // Place walls to the right and down from the player.
    game.level.map[(1, 1)] = Tile::wall();
    game.level.map[(1, 2)] = Tile::wall();
    game.level.map[(1, 3)] = Tile::wall();

    game.step_game(InputAction::Pass);
    game.step_game(InputAction::Move(Direction::Down));
    assert_eq!(Pos::new(0, 1), game.level.entities.pos[&player_id]);

    // This fails because the golem does not get to take a turn once investigating.
    assert!(matches!(game.level.entities.behavior[&gol], Behavior::Investigating(_)));
    assert_eq!(Pos::new(2, 0), game.level.entities.pos[&gol]);
}
