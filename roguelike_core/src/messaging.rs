use std::fmt;
use std::collections::VecDeque;

use serde::{Serialize, Deserialize};

use crate::types::*;
use crate::map::*;
use crate::movement::{MoveType, MoveMode, Direction};
use crate::ai::Behavior;
use crate::movement::Attack;


#[derive(Copy, Clone, PartialEq, Debug, Deserialize, Serialize)]
pub enum Msg {
    StartTurn,
    Pass,
    Crushed(EntityId, Pos), // object that did the crushing, position
    Sound(EntityId, Pos, usize, bool), // object causing sound, location, radius, whether animation will play
    SoundTrapTriggered(EntityId, EntityId), // trap, entity
    SpikeTrapTriggered(EntityId, EntityId), // trap, entity
    BlinkTrapTriggered(EntityId, EntityId), // trap, entity
    Blink(EntityId),
    FreezeTrapTriggered(EntityId, EntityId), // trap, entity
    GateTriggered(EntityId, EntityId), // trap, entity
    Froze(EntityId, usize), // entity, num turns
    PlayerDeath,
    PickedUp(EntityId, EntityId), // entity, item
    PickUp(EntityId), // entity trying to pick up an item
    ItemThrow(EntityId, EntityId, Pos, Pos), // thrower, stone id, start, end
    TryAttack(EntityId, Attack, Pos), // attacker, attack description, attack pos
    Attack(EntityId, EntityId, i32), // attacker, attacked, hp lost
    Blunt(Pos, Pos), // attacker position, attacked position
    Pierce(Pos, Pos), // attacker position, attacked position
    Slash(Pos, Pos), // attacker position, attacked position
    Killed(EntityId, EntityId, i32), // attacker, attacked, hp lost
    Remove(EntityId), // entity_id
    Push(EntityId, Direction, usize), // attacker, direction, amount
    Pushed(EntityId, EntityId, Direction, usize, bool), // attacker, attacked, direction, amount, move into pushed square
    TryMove(EntityId, Direction, usize, MoveMode),
    Moved(EntityId, MoveType, Pos),
    Interact(EntityId, Pos),
    JumpWall(EntityId, Pos, Pos), // current pos, new pos
    WallKick(EntityId, Pos),
    StateChange(EntityId, Behavior),
    Collided(EntityId, Pos),
    Yell(EntityId),
    ChangeMoveMode(EntityId, bool), // true = increase, false = decrease
    MoveMode(EntityId, MoveMode),
    TriedRunWithHeavyEquipment,
    Hit(EntityId, Pos, WeaponType, AttackStyle),
    HammerRaise(EntityId, usize, Direction), // entity, item index, direction moved
    HammerSwing(EntityId, EntityId, Pos), // entity, item, position swung at
    HammerHitEntity(EntityId, EntityId), // entity, hit entity
    HammerHitWall(EntityId, Blocked),
    Stabbed(EntityId, EntityId), // entity, hit entity
    FailedBlink(EntityId),
    NotEnoughEnergy(EntityId),
    DropFailed(EntityId),
    DroppedItem(EntityId, EntityId),
    PlayerTurn,
    Triggered(EntityId, EntityId), // trap, entity
    Untriggered(EntityId, EntityId), // trap, entity
    AddClass(EntityClass),
    DropItem(EntityId, EntityId), // entity, item
    GrassThrow(EntityId, Direction),
    GrassShoes(EntityId, ActionMode),
    GrassBlade(EntityId, ActionMode, Direction),
    Illuminate(EntityId, Pos, usize), // entity, position, amount
    Heal(EntityId, usize), // entity, amount
    FarSight(EntityId, usize), // entity, amount
    Ping(EntityId, Pos),
    Sprint(EntityId, Direction, usize), // entity, direction, amount
    Rubble(EntityId, Pos),
    Reform(EntityId, Pos),
    StoneSkin(EntityId),
    Swap(EntityId, EntityId), // casting entity, entity to swap with
    PassWall(EntityId, Pos),
    UseItem(EntityId, Pos, EntityId), // holding entity, position, item id
    ArmDisarmTrap(EntityId, EntityId), // acting entity, trap id
    PlaceTrap(EntityId, Pos, EntityId), // placing entity, position, trap id
    SpawnedObject(EntityId, EntityType, Pos, EntityName, Direction),
    FaceTowards(EntityId, Pos),
    SetFacing(EntityId, Direction),
    Facing(EntityId, Direction),
    AiAttack(EntityId),
    RemovedEntity(EntityId),
    StartUseItem(EntityId),
    StartUseInteract,
    NewLevel,
    CursorToggle(bool, Pos),
}

impl fmt::Display for Msg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Msg::StartTurn => write!(f, "startturn"),
            Msg::Pass => write!(f, "pass"),
            Msg::Crushed(entity_id, pos) => write!(f, "crushed {} {} {}", entity_id, pos.x, pos.y),
            Msg::Sound(entity_id, pos, radius, animate) => write!(f, "sound {} {} {} {} {}", entity_id, pos.x, pos.y, radius, animate),
            Msg::SoundTrapTriggered(trap_id, entity_id) => write!(f, "sound_trap_triggered {} {}", trap_id, entity_id),
            Msg::SpikeTrapTriggered(trap_id, entity_id) => write!(f, "spike_trap_triggered {} {}", trap_id, entity_id),
            Msg::BlinkTrapTriggered(trap_id, entity_id) => write!(f, "blink_trap_triggered {} {}", trap_id, entity_id),
            Msg::Blink(entity_id) => write!(f, "blink {}", entity_id),
            Msg::FreezeTrapTriggered(trap_id, entity_id) => write!(f, "freeze_trap_triggered {} {}", trap_id, entity_id),
            Msg::GateTriggered(trap_id, entity_id) => write!(f, "gate_triggered {} {}", trap_id, entity_id),
            Msg::Froze(entity_id, turns) => write!(f, "froze {} {}", entity_id, turns),
            Msg::PlayerDeath => write!(f, "player_death"),
            Msg::PickedUp(entity_id, item_id) => write!(f, "picked_up {} {}", entity_id, item_id),
            Msg::PickUp(entity_id) => write!(f, "pickup {}", entity_id),
            Msg::ItemThrow(entity_id, item_id, start, end) => write!(f, "item_throw {} {} {} {} {} {}", entity_id, item_id, start.x, start.y, end.x, end.y),
            Msg::TryAttack(entity_id, attack, pos) => {
                match attack {
                    Attack::Attack(target_id) => write!(f, "try_attack {} {} {} {}", entity_id, target_id, pos.x, pos.y),
                    Attack::Push(target_id, direction, amount) => write!(f, "try_push {} {} {} {} {} {}", entity_id, target_id, pos.x, pos.y, direction, amount),
                    Attack::Stab(target_id, move_into) => write!(f, "try_stab {} {} {} {} {}", entity_id, target_id, pos.x, pos.y, move_into),
                }
            }
            Msg::Attack(entity_id, target_id, hp) => write!(f, "attack {} {} {}", entity_id, target_id, hp),
            Msg::Blunt(attacker, attacked) => write!(f, "blunt {} {} {} {}", attacker.x, attacker.y, attacked.x, attacked.y),
            Msg::Pierce(attacker, attacked) => write!(f, "pierce {} {} {} {}", attacker.x, attacker.y, attacked.x, attacked.y),
            Msg::Slash(attacker, attacked) => write!(f, "slash {} {} {} {}", attacker.x, attacker.y, attacked.x, attacked.y),
            Msg::Killed(entity_id, target_id, hp) => write!(f, "killed {} {} {}", entity_id, target_id, hp),
            Msg::Remove(entity_id) => write!(f, "remove {}", entity_id),
            Msg::Push(entity_id, direction, amount) => write!(f, "pushed {} {} {}", entity_id, direction, amount),
            Msg::Pushed(entity_id, target_id, direction, amount, follow) => write!(f, "pushed {} {} {} {} {}", entity_id, target_id, direction, amount, follow),
            Msg::TryMove(entity_id, direction, amount, move_mode) => write!(f, "try_move {} {} {} {}", entity_id, direction, amount, move_mode),
            Msg::Moved(entity_id, move_type, pos) => write!(f, "moved {} {} {} {}", entity_id, move_type, pos.x, pos.y),
            Msg::Interact(entity_id, pos) => write!(f, "interact {} {} {}", entity_id, pos.x, pos.y),
            Msg::JumpWall(entity_id, pos, new_pos) => write!(f, "jump_wall {} {} {} {} {}", entity_id, pos.x, pos.y, new_pos.x, new_pos.y),
            Msg::WallKick(entity_id, pos) => write!(f, "wall_kick {} {} {}", entity_id, pos.y, pos.y),
            Msg::StateChange(entity_id, behavior) => {
                match behavior {
                    Behavior::Idle => write!(f, "state_change_idle {}", entity_id),
                    Behavior::Investigating(pos) => write!(f, "state_change_investigating {} {} {}", entity_id, pos.x, pos.y),
                    Behavior::Attacking(target_id) => write!(f, "state_change_attacking {} {}", entity_id, target_id),
                }
            }
            Msg::Collided(entity_id, pos) => write!(f, "collided {} {} {}", entity_id, pos.x, pos.y),
            Msg::Yell(entity_id) => write!(f, "yell {}", entity_id),
            Msg::ChangeMoveMode(entity_id, upwards) => write!(f, "chage_move_mode {} {}", entity_id, upwards),
            Msg::MoveMode(entity_id, move_mode) => write!(f, "move_mode {} {}", entity_id, move_mode),
            Msg::TriedRunWithHeavyEquipment => write!(f, "tried_run_with_heavy_equipment"),
            Msg::Hit(entity_id, pos, weapon_type, attack_style) => write!(f, "hit {} {} {} {} {}", entity_id, pos.x, pos.y, weapon_type, attack_style),
            Msg::HammerRaise(entity_id, item_index, dir) => write!(f, "hammer_raise {} {} {}", entity_id, item_index, dir),
            Msg::HammerSwing(entity_id, item_id, pos) => write!(f, "hammer_swing {} {} {} {}", entity_id, item_id, pos.x, pos.y),
            Msg::HammerHitEntity(entity_id, target_id) => write!(f, "hammer_hit_entity {} {}", entity_id, target_id),
            Msg::HammerHitWall(entity_id, blocked) => write!(f, "hammer_hit_wall {} {} {} {} {} {}", entity_id, blocked.start_pos, blocked.end_pos, blocked.direction, blocked.blocked_tile, blocked.wall_type),
            Msg::Stabbed(entity_id, target_id) => write!(f, "stabbed {} {}", entity_id, target_id),
            Msg::FailedBlink(entity_id) => write!(f, "failed_blink {}", entity_id),
            Msg::NotEnoughEnergy(entity_id) => write!(f, "not_enough_energy {}", entity_id),
            Msg::DropFailed(entity_id) => write!(f, "drop_failed {}", entity_id),
            Msg::DroppedItem(entity_id, trap_id) => write!(f, "dropped_item {} {}", entity_id, trap_id),
            Msg::PlayerTurn => write!(f, "player_turn"),
            Msg::Triggered(trap_id, entity_id) => write!(f, "triggered {} {}", trap_id, entity_id),
            Msg::Untriggered(trap_id, entity_id) => write!(f, "untriggered {} {}", trap_id, entity_id),
            Msg::AddClass(entity_class) => write!(f, "add_class {}", entity_class),
            Msg::DropItem(entity_id, item_id) => write!(f, "drop_item {} {}", entity_id, item_id),
            Msg::GrassThrow(entity_id, direction) => write!(f, "grass_throw {} {}", entity_id, direction),
            Msg::GrassShoes(entity_id, action_mode) => write!(f, "grass_shoes {} {}", entity_id, action_mode),
            Msg::GrassBlade(entity_id, action_mode, direction) => write!(f, "grass_blade {} {} {}", entity_id, action_mode, direction),
            Msg::Illuminate(entity_id, pos, amount) => write!(f, "illuminate {} {} {} {}", entity_id, pos.x, pos.y, amount),
            Msg::Heal(entity_id, amount) => write!(f, "heal {} {}", entity_id, amount),
            Msg::FarSight(entity_id, amount) => write!(f, "farsight {} {}", entity_id, amount),
            Msg::Ping(entity_id, pos) => write!(f, "ping {} {} {}", entity_id, pos.x, pos.y),
            Msg::Sprint(entity_id, direction, amount) => write!(f, "sprint {} {} {}", entity_id, direction, amount),
            Msg::Rubble(entity_id, pos) => write!(f, "rubble {} {} {}", entity_id, pos.x, pos.y),
            Msg::Reform(entity_id, pos) => write!(f, "reform {} {} {}", entity_id, pos.x, pos.y),
            Msg::StoneSkin(entity_id) => write!(f, "reform {}", entity_id),
            Msg::Swap(entity_id, target_id) => write!(f, "swap {} {}", entity_id, target_id),
            Msg::PassWall(entity_id, pos) => write!(f, "pass_wall {} {} {}", entity_id, pos.x, pos.y),
            Msg::UseItem(entity_id, pos, item_id) => write!(f, "use_item {} {} {} {}", entity_id, pos.x, pos.y, item_id),
            Msg::ArmDisarmTrap(entity_id, trap_id) => write!(f, "arm_disarm_trap {} {}", entity_id, trap_id),
            Msg::PlaceTrap(entity_id, pos, trap_id) => write!(f, "place_trap {} {} {} {}", entity_id, pos.x, pos.y, trap_id),
            Msg::SpawnedObject(entity_id, entity_type, pos, entity_name, facing) => write!(f, "spawned {} {} {} {} {} {}", entity_id, entity_type, pos.x, pos.y, entity_name, facing),
            Msg::FaceTowards(entity_id, pos) => write!(f, "face_towards {} {} {}", entity_id, pos.x, pos.y),
            Msg::SetFacing(entity_id, direction) => write!(f, "set_facing {} {}", entity_id, direction),
            Msg::Facing(entity_id, direction) => write!(f, "facing {} {}", entity_id, direction),
            Msg::AiAttack(entity_id) => write!(f, "ai_attack {}", entity_id),
            Msg::RemovedEntity(entity_id) => write!(f, "removed {}", entity_id),
            Msg::StartUseItem(entity_id) => write!(f, "startuseitem {}", entity_id),
            Msg::StartUseInteract => write!(f, "startuseinteract"),
            Msg::NewLevel => write!(f, "newlevel"),
            Msg::CursorToggle(state, pos) => write!(f, "cursortoggle {} {} {}", state, pos.x, pos.y),
        }
    }
}


impl Msg {
    pub fn msg_line(&self, data: &Level) -> String {
        match self {
            Msg::StartTurn => {
                return "Starting turn".to_string();
            }

            Msg::Crushed(_obj_id, _pos) => {
                return "An object has been crushed".to_string();
            }

            Msg::Pass => {
                return "Player passed their turn".to_string();
            }

            Msg::BlinkTrapTriggered(_trap, _entity_id) => {
                return "Blink trap triggered".to_string();
            }

            Msg::Blink(entity_id) => {
                return format!("{:?} blinked",
                               data.entities.name[entity_id].clone());
            }

            Msg::SoundTrapTriggered(_trap, _entity_id) => {
                return "Sound trap triggered".to_string();
            }

            Msg::SpikeTrapTriggered(_trap, _entity_id) => {
                return "Spike trap triggered".to_string();
            }

            Msg::FreezeTrapTriggered(_trap, _entity_id) => {
                return "Freeze trap triggered".to_string();
            }

            Msg::GateTriggered(_trap, _entity_id) => {
                return "Gate activated".to_string();
            }

            Msg::PlayerDeath => {
                return "Player died!".to_string();
            }

            Msg::PickedUp(entity_id, item_id) => {
                return format!("{:?} picked up a {:?}",
                               data.entities.name[entity_id].clone(),
                               data.entities.name[item_id].clone());
            }

            Msg::ItemThrow(_thrower, _item, _start, _end) => {
                return "Item throw".to_string();
            }

            Msg::TryAttack(_entity_id, _attack_info, _attack_pos) => {
                return "".to_string();
            }

            Msg::Attack(attacker, attacked, damage) => {
                return format!("{:?} attacked {:?} for {} damage",
                               data.entities.name[attacker],
                               data.entities.name[attacked],
                               damage);
            }

            Msg::Killed(attacker, attacked, _damage) => {
                return format!("{:?} killed {:?}", data.entities.name[attacker], data.entities.name[attacked]);
            }

            Msg::Push(_attacker, _direction, _amount) => {
                return "".to_string();
            }

            Msg::Pushed(attacker, attacked, _direction, _amount, _move_into) => {
                return format!("{:?} pushed {:?}", data.entities.name[attacker], data.entities.name[attacked]);
            }

            Msg::Interact(_entity_id, _pos) => {
                return "".to_string();
            }

            Msg::Moved(entity_id, move_type, pos) => {
                if let MoveType::Pass = move_type {
                    return format!("{:?} passed their turn", data.entities.name[entity_id]);
                } else {
                    return format!("{:?} moved to {}", data.entities.name[entity_id], pos);
                }
            }

            Msg::TryMove(_entity_id, _direction, _amount, _move_mode) => {
                return "".to_string();
            }

            Msg::JumpWall(_entity_id, _start, _end) => {
                return "Jumped a wall".to_string();
            }

            Msg::WallKick(_entity_id, _pos) => {
                return "Did a wallkick".to_string();
            }

            Msg::StateChange(_entity_id, behavior) => {
                return format!("Changed state to {:?}", *behavior);
            }

            Msg::Yell(entity_id) => {
                return format!("{:?} yelled", data.entities.name[entity_id]);
            }

            Msg::Collided(_entity_id, _pos) => {
                return "Collided".to_string();
            }

            Msg::ChangeMoveMode(_entity_id, _increase) => {
                return "".to_string();
            }

            Msg::MoveMode(entity_id, move_mode) => {
                match move_mode {
                    MoveMode::Sneak => {
                        return format!("{:?} is now sneaking", data.entities.name[entity_id]);
                    }

                    MoveMode::Walk => {
                        return format!("{:?} is now walking", data.entities.name[entity_id]);
                    }

                    MoveMode::Run => {
                        return format!("{:?} is now running", data.entities.name[entity_id]);
                    }
                }
            }

            Msg::TriedRunWithHeavyEquipment => {
                return "Your equipment is too heavy to run!".to_string();
            }

            Msg::HammerRaise(entity_id, _item_index, _dir) => {
                return format!("{:?} raises their hammer", data.entities.name[entity_id]);
            }

            Msg::HammerSwing(entity_id, _item_id, _pos) => {
                return format!("{:?} swung their hammer", data.entities.name[entity_id]);
            }

            Msg::HammerHitEntity(entity_id, hit_entity) => {
                let entity_name = &data.entities.name[entity_id];
                let hit_entity_name = &data.entities.name[hit_entity];
                return format!("{:?} hit {:?} with their hammer", entity_name, hit_entity_name);
            }

            Msg::HammerHitWall(entity_id, _blocked) => {
                return format!("{:?} hit a wall with their hammer", data.entities.name[entity_id]);
            }

            Msg::Stabbed(attacker, attacked) => {
                let entity_name = &data.entities.name[attacker];
                let hit_entity_name = &data.entities.name[attacked];
                return format!("{:?} stabbed {:?} with a dagger!", entity_name, hit_entity_name);
            }

            Msg::FailedBlink(entity_id) => {
                return format!("{:?} failed to blink!", data.entities.name[entity_id]);
            }

            Msg::NotEnoughEnergy(entity_id) => {
                return format!("{:?} does not have enough energy for that", data.entities.name[entity_id]);
            }

            Msg::DropFailed(entity_id) => {
                return format!("{:?} tried to drop an item, but its too crowded!", data.entities.name[entity_id]);
            }

            Msg::Froze(entity_id, _num_turns) => {
                return format!("{:?} was frozen!", data.entities.name[entity_id]);
            }

            Msg::Triggered(_trap, entity_id) => {
                return format!("{:?} triggered something!", data.entities.name[entity_id]);
            }

            Msg::Untriggered(_trap, entity_id) => {
                return format!("{:?} stepped off something!", data.entities.name[entity_id]);
            }

            Msg::AddClass(class) => {
                return format!("Player chose class {:?}!", class);
            }
            
            Msg::DropItem(entity_id, item_index) => {
                if let Some(item_id) = data.entities.inventory[entity_id].get(*item_index as usize) {
                    return format!("{:?} dropped a {:?}!", data.entities.name[entity_id], item_id);
                } else {
                    return "".to_string();
                }
            }

            Msg::GrassThrow(entity_id, direction) => {
                return format!("{:?} threw grass {}", data.entities.name[entity_id], direction);
            }

            Msg::GrassShoes(entity_id, _action_mode) => {
                return format!("{:?} used grass shoes", data.entities.name[entity_id]);
            }

            Msg::GrassBlade(entity_id, _action_mode, _direction) => {
                return format!("{:?} used grass blade", data.entities.name[entity_id]);
            }

            Msg::Illuminate(entity_id, _pos, _amount) => {
                return format!("{:?} illuminated their surroundings", data.entities.name[entity_id]);
            }

            Msg::Heal(entity_id, amount) => {
                return format!("{:?} healed by {}", data.entities.name[entity_id], amount);
            }

            Msg::FarSight(entity_id, amount) => {
                return format!("{:?} has farsight ({})", data.entities.name[entity_id], amount);
            }

            Msg::Ping(entity_id, pos) => {
                return format!("{:?} has pinged ({})", data.entities.name[entity_id], pos);
            }

            Msg::Sprint(entity_id, _direction, _amount) => {
                return format!("{:?} has sprinted!", data.entities.name[entity_id]);
            }

            Msg::Rubble(entity_id, pos) => {
                return format!("{:?} turned a wall to rubble at {}", data.entities.name[entity_id], pos);
            }

            Msg::Reform(entity_id, _pos) => {
                return format!("{:?} turns rubble into wall", data.entities.name[entity_id]);
            }

            Msg::StoneSkin(entity_id) => {
                return format!("{:?} turns into stone!", data.entities.name[entity_id]);
            }

            Msg::Swap(entity_id, other_id) => {
                return format!("{:?} swaps with {:?}", data.entities.name[entity_id], data.entities.name[other_id]);
            }
            
           Msg::PassWall(entity_id, pos) => { 
                return format!("{:?} passes through {}", data.entities.name[entity_id], pos);
            }

            Msg::UseItem(entity_id, pos, item_id) => {
                return format!("{:?} used {:?} on {}", data.entities.name[entity_id], data.entities.name[item_id], pos);
            }

            Msg::ArmDisarmTrap(entity_id, trap_id) => {
                return format!("{:?} fiddles with {:?}", data.entities.name[entity_id], data.entities.name[trap_id]);
            }

            Msg::PlaceTrap(entity_id, pos, trap_id) => {
                return format!("{:?} place {:?} at {}", data.entities.name[entity_id], data.entities.name[trap_id], pos);
            }

            _ => {
                return "".to_string();
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MsgLog {
    pub messages: VecDeque<Msg>,
    pub turn_messages: VecDeque<Msg>,
}

impl MsgLog {
    pub fn new() -> MsgLog {
        return MsgLog {
            messages: VecDeque::new(),
            turn_messages: VecDeque::new(),
        };
    }

    pub fn pop(&mut self) -> Option<Msg> {
        let msg = self.messages.pop_front();
        if let Some(msg) = msg {
            self.turn_messages.push_back(msg);
        }
        return msg;
    }

    pub fn log(&mut self, msg: Msg) {
        self.messages.push_back(msg);
    }

    pub fn log_front(&mut self, msg: Msg) {
        self.messages.push_front(msg);
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.turn_messages.clear();
    }
}

