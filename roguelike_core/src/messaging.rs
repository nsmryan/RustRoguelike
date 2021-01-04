use std::collections::VecDeque;

use serde::{Serialize, Deserialize};

use crate::types::*;
use crate::map::*;
use crate::movement::{Movement, MoveType, MoveMode, Action, Direction};
use crate::ai::Behavior;


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
        return self.messages.pop_front();
    }

    pub fn log(&mut self, msg: Msg) {
        self.messages.push_back(msg);
        self.turn_messages.push_back(msg);
    }

    pub fn log_front(&mut self, msg: Msg) {
        self.messages.push_front(msg);
        self.turn_messages.push_front(msg);
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.turn_messages.clear();
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Deserialize, Serialize)]
pub enum Msg {
    Pass(),
    Crushed(EntityId, Pos), // object that did the crushing, position
    Sound(EntityId, Pos, usize, bool), // object causing sound, location, radius, whether animation will play
    SoundTrapTriggered(EntityId, EntityId), // trap, entity
    SpikeTrapTriggered(EntityId, EntityId), // trap, entity
    BlinkTrapTriggered(EntityId, EntityId), // trap, entity
    FreezeTrapTriggered(EntityId, EntityId), // trap, entity
    GateTriggered(EntityId, EntityId), // trap, entity
    Froze(EntityId, usize), // entity, num turns
    PlayerDeath,
    PickedUp(EntityId, EntityId), // entity, item id
    ItemThrow(EntityId, EntityId, Pos, Pos), // thrower, stone id, start, end
    Attack(EntityId, EntityId, Hp), // attacker, attacked, hp lost
    Killed(EntityId, EntityId, Hp), // attacker, attacked, hp lost
    Pushed(EntityId, EntityId, Direction, usize, bool), // attacker, attacked, direction, amount, move into pushed square
    TryMove(EntityId, Direction, usize, MoveMode),
    Moved(EntityId, MoveType, Pos),
    JumpWall(EntityId, Pos, Pos), // current pos, new pos
    WallKick(EntityId, Pos),
    StateChange(EntityId, Behavior),
    Collided(EntityId, Pos),
    Yell(EntityId, Pos),
    GameState(GameState),
    MoveMode(EntityId, MoveMode),
    TriedRunWithHeavyEquipment,
    SpawnedObject(EntityId, EntityType, Pos, EntityName),
    SwordSwing(EntityId, Pos), // entity, position swung at
    HammerSwing(EntityId, Pos), // entity, position swung at
    HammerHitEntity(EntityId, EntityId), // entity, hit entity
    HammerHitWall(EntityId, Blocked),
    Stabbed(EntityId, EntityId), // entity, hit entity
    Action(EntityId, Action),
    FailedBlink(EntityId),
    NotEnoughEnergy(EntityId),
    DropFailed(EntityId),
    DroppedItem(EntityId, EntityId),
    PlayerTurn(),
    Triggered(EntityId, EntityId), // trap, entity
    Untriggered(EntityId, EntityId), // trap, entity
    AddClass(EntityClass),
    SwapPrimaryItem,
    DropItem(EntityId, EntityId), // entity, item
}

impl Msg {
    pub fn msg_line(&self, data: &GameData) -> String {
        match self {
            Msg::Crushed(_obj_id, _pos) => {
                return "An object has been crushed".to_string();
            }

            Msg::Pass() => {
                return "Player passed their turn".to_string();
            }

            Msg::BlinkTrapTriggered(_trap, _entity_id) => {
                return "Blink trap triggered".to_string();
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

            Msg::PickedUp(entity_id, item) => {
                return format!("{:?} picked up a {:?}",
                               data.entities.name[entity_id].clone(),
                               data.entities.name[item].clone());
            }

            Msg::ItemThrow(_thrower, _item, _start, _end) => {
                return "Item throw".to_string();
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

            Msg::Pushed(attacker, attacked, _direction, _amount, _move_into) => {
                return format!("{:?} pushed {:?}", data.entities.name[attacker], data.entities.name[attacked]);
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

            Msg::Yell(entity_id, _pos) => {
                return format!("{:?} yelled", data.entities.name[entity_id]);
            }

            Msg::Collided(_entity_id, _pos) => {
                return "Collided".to_string();
            }

            Msg::GameState(game_state) => {
                match game_state {
                    GameState::Inventory => {
                        return "Opened Inventory".to_string();
                    }

                    GameState::Playing => {
                        return "".to_string();
                    }

                    GameState::Selection => {
                        return "Selecting a location".to_string();
                    }

                    GameState::SkillMenu => {
                        return "Selecting a skill".to_string();
                    }

                    GameState::ClassMenu => {
                        return "Selecting a class".to_string();
                    }

                    GameState::ConfirmQuit => {
                        return "Confirm quit".to_string();
                    }

                    _ => {
                        dbg!(game_state);
                        panic!();
                    }
                }
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

            Msg::SwordSwing(entity_id, _pos) => {
                return format!("{:?} swung their sword", data.entities.name[entity_id]);
            }

            Msg::HammerSwing(entity_id, _pos) => {
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

            Msg::PlayerTurn() => {
                return "".to_string();
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
            
            Msg::SwapPrimaryItem => {
                return format!("Player swapped primary and secondary item");
            }

            Msg::DropItem(entity_id, item_index) => {
                let item_id = data.entities.inventory[entity_id][*item_index as usize];
                return format!("{:?} dropped a {:?}!", data.entities.name[entity_id], item_id);
            }

            _ => {
                return "".to_string();
            }
        }
    }
}

