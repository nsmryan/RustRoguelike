use std::collections::VecDeque;

use crate::types::*;
use crate::movement::{Movement, MoveType, MoveMode};
use crate::ai::Behavior;


pub struct MsgLog {
    pub messages: VecDeque<Msg>,
    pub turn_messages: VecDeque<Msg>,
}

impl MsgLog {
    pub fn new() -> MsgLog {
        return MsgLog {
            messages: VecDeque::new(),
        };
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

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Msg {
    Pass(),
    Crushed(ObjectId, Pos, ObjType), // object that did the crushing, position, type that was crushed
    Sound(ObjectId, Pos, usize, bool), // object causing sound, location, radius, whether animation will play
    SoundTrapTriggered(ObjectId, ObjectId), // trap, entity
    SpikeTrapTriggered(ObjectId, ObjectId), // trap, entity
    PlayerDeath,
    PickedUp(ObjectId, ObjectId), // entity, item id
    ItemThrow(ObjectId, ObjectId, Pos, Pos), // thrower, stone id, start, end
    Attack(ObjectId, ObjectId, Hp), // attacker, attacked, hp lost
    Killed(ObjectId, ObjectId, Hp), // attacker, attacked, hp lost
    Moved(ObjectId, Movement, Pos),
    JumpWall(ObjectId, Pos, Pos), // current pos, new pos
    WallKick(ObjectId, Pos),
    StateChange(ObjectId, Behavior),
    Collided(ObjectId, Pos),
    Yell(Pos),
    GameState(GameState),
    MoveMode(MoveMode),
    TriedRunWithShield,
    SpawnedObject(EntityId),
}

impl Msg {
    pub fn msg_line(&self, game_data: &GameData) -> String {
        match self {
            Msg::Crushed(_obj_id, _pos, _obj_type) => {
                return "An object has been crushed".to_string();
            }

            Msg::Sound(_obj_id, _pos, _radius, _animate) => {
                return "".to_string();
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
                return format!("{} picked up a {}",
                               game_data.objects[*entity].name,
                               game_data.objects[*item].name.clone());
            }

            Msg::ItemThrow(_thrower, _item, _start, _end) => {
                return "Item throw".to_string();
            }

            Msg::Attack(attacker, attacked, damage) => {
                return format!("{} attacked {} for {} damage",
                               game_data.objects[*attacker].name,
                               game_data.objects[*attacked].name,
                               damage);
            }

            Msg::Killed(_attacker, _attacked, _damage) => {
                return "Killed".to_string();
            }

            Msg::Moved(object_id, movement, _pos) => {
                if let MoveType::Pass = movement.typ {
                    return format!("{} passed their turn", game_data.objects[*object_id].name);
                } else {
                    return format!("{} moved", game_data.objects[*object_id].name);
                }
            }

            Msg::JumpWall(_object_id, _start, _end) => {
                return "Jumped a wall".to_string();
            }

            Msg::WallKick(_object_id, _pos) => {
                return "Did a wallkick".to_string();
            }

            Msg::StateChange(_object_id, behavior) => {
                return format!("Changed state to {:?}", *behavior);
            }

            Msg::Yell(_pos) => {
                return format!("Yelled");
            }

            Msg::Collided(_object_id, _pos) => {
                return "Collided".to_string();
            }

            Msg::GameState(game_state) => {
                match game_state {
                    GameState::Inventory => {
                        return "Opened Inventory".to_string();
                    }

                    GameState::Playing => {
                        return "Closed Inventory".to_string();
                    }

                    GameState::Throwing => {
                        return "Throwing item".to_string();
                    }

                    _ => {
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

            Msg::TriedRunWithShield => {
                return "Can't run with shield!".to_string();
            }

            Msg::SpawnedObject(entity_id) => {
                return "".to_string();
            }
        }
    }
}

