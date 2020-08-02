use std::collections::VecDeque;

use serde::{Serialize, Deserialize};

use crate::types::*;
use crate::map::*;
use crate::movement::{Movement, MoveType, MoveMode, Action};
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
    PlayerDeath,
    PickedUp(EntityId, EntityId), // entity, item id
    ItemThrow(EntityId, EntityId, Pos, Pos), // thrower, stone id, start, end
    Attack(EntityId, EntityId, Hp), // attacker, attacked, hp lost
    Killed(EntityId, EntityId, Hp), // attacker, attacked, hp lost
    Pushed(EntityId, EntityId, Pos), // attacker, attacked, change in position
    Moved(EntityId, Movement, Pos),
    JumpWall(EntityId, Pos, Pos), // current pos, new pos
    WallKick(EntityId, Pos),
    StateChange(EntityId, Behavior),
    Collided(EntityId, Pos),
    Yell(EntityId, Pos),
    GameState(GameState),
    MoveMode(MoveMode),
    TriedRunWithHeavyEquipment,
    SpawnedObject(EntityId, EntityType, Pos, EntityName),
    SwordSwing(EntityId, Pos), // entity, position swung at
    HammerSwing(EntityId, Pos), // entity, position swung at
    HammerHitEntity(EntityId, EntityId), // entity, hit entity
    HammerHitWall(EntityId, Blocked),
    ChangeLevel(),
    Action(EntityId, Action),
    FailedBlink(EntityId),
    NotEnoughEnergy(EntityId),
    DropFailed(EntityId),
    PlayerTurn(),
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

            Msg::SoundTrapTriggered(_trap, _entity) => {
                return "Sound trap triggered".to_string();
            }

            Msg::SpikeTrapTriggered(_trap, _entity) => {
                return "Spike trap triggered".to_string();
            }

            Msg::PlayerDeath => {
                return "Player died!".to_string();
            }

            Msg::PickedUp(entity, item) => {
                return format!("{:?} picked up a {:?}",
                               data.entities.name[entity].clone(),
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

            Msg::Pushed(attacker, attacked, _delta_pos) => {
                return format!("{:?} pushed {:?}", data.entities.name[attacker], data.entities.name[attacked]);
            }

            Msg::Moved(entity, movement, pos) => {
                if let MoveType::Pass = movement.typ {
                    return format!("{:?} passed their turn", data.entities.name[entity]);
                } else {
                    return format!("{:?} moved to {}", data.entities.name[entity], pos);
                }
            }

            Msg::JumpWall(_entity, _start, _end) => {
                return "Jumped a wall".to_string();
            }

            Msg::WallKick(_entity, _pos) => {
                return "Did a wallkick".to_string();
            }

            Msg::StateChange(_entity, behavior) => {
                return format!("Changed state to {:?}", *behavior);
            }

            Msg::Yell(entity, _pos) => {
                return format!("{:?} yelled", data.entities.name[entity]);
            }

            Msg::Collided(_entity, _pos) => {
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

                    _ => {
                        dbg!(game_state);
                        panic!();
                    }
                }
            }

            Msg::MoveMode(move_mode) => {
                match move_mode {
                    MoveMode::Sneak => {
                        return "Sneaking".to_string();
                    }

                    MoveMode::Walk => {
                        return "Walking".to_string();
                    }

                    MoveMode::Run => {
                        return "Running".to_string();
                    }
                }
            }

            Msg::TriedRunWithHeavyEquipment => {
                return "Your equipment is too heavy to run!".to_string();
            }

            Msg::SwordSwing(entity, _pos) => {
                return format!("{:?} swung their sword", data.entities.name[entity]);
            }

            Msg::HammerSwing(entity, _pos) => {
                return format!("{:?} swung their hammer", data.entities.name[entity]);
            }

            Msg::HammerHitEntity(entity, hit_entity) => {
                let entity_name = &data.entities.name[entity];
                let hit_entity_name = &data.entities.name[hit_entity];
                return format!("{:?} hit {:?} with their hammer", entity_name, hit_entity_name);
            }

            Msg::HammerHitWall(entity, _blocked) => {
                return format!("{:?} hit a wall with their hammer", data.entities.name[entity]);
            }

            Msg::FailedBlink(entity) => {
                return format!("{:?} failed to blink!", data.entities.name[entity]);
            }

            Msg::NotEnoughEnergy(entity) => {
                return format!("{:?} does not have enough energy for that", data.entities.name[entity]);
            }

            Msg::DropFailed(entity) => {
                return format!("{:?} tried to drop an item, but its too crowded!", data.entities.name[entity]);
            }

            Msg::PlayerTurn() => {
                return "".to_string();
            }

            _ => {
                return "".to_string();
            }
        }
    }
}

