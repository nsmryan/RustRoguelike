use crate::types::*;


pub struct MsgLog {
    pub messages: Vec<Msg>,
}

impl MsgLog {
    pub fn new() -> MsgLog {
        return MsgLog {
            messages: Vec::new(),
        };
    }

    pub fn log(&mut self, msg: Msg) {
        self.messages.push(msg);
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Msg {
    SpikeTrapTriggered(ObjectId, ObjectId), // trap, entity
    PlayerDeath,
    PickedUp(ObjectId, ObjectId), // entity, item id
    StoneThrow(ObjectId, ObjectId, Pos, Pos), // thrower, stone id, start, end
    Attack(ObjectId, ObjectId, Hp), // attacker, attacked, hp lost
    Killed(ObjectId, ObjectId, Hp), // attacker, attacked, hp lost
    Moved(ObjectId, Pos),
    JumpWall(ObjectId, Pos),
    WallKick(ObjectId, Pos),
    StateChange(ObjectId, Behavior),
    Collided(ObjectId, Pos),
}

impl Msg {
    pub fn msg_line(&self, game_data: &GameData) -> String {
        match self {
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

            Msg::StoneThrow(thrower, stone, start, end) => {
                return "Stone throw".to_string();
            }

            Msg::Attack(attacker, attacked, damage) => {
                return format!("{} attacked {} for {} damage",
                               game_data.objects[*attacker].name,
                               game_data.objects[*attacked].name,
                               damage);
            }

            Msg::Killed(attacker, attacked, damage) => {
                return "Killed".to_string();
            }

            Msg::Moved(object_id, pos) => {
                return format!("{} moved", game_data.objects[*object_id].name);
            }

            Msg::JumpWall(object_id, pos) => {
                return "Jumped a wall".to_string();
            }

            Msg::WallKick(object_id, pos) => {
                return "Did a wallkick".to_string();
            }

            Msg::StateChange(object_id, behavior) => {
                return format!("Changed state to {:?}", *behavior);
            }

            Msg::Collided(object_id, pos) => {
                return "Collided".to_string();
            }
        }
    }
}
