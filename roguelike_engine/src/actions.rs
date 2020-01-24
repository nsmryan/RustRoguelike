use roguelike_core::types::*;
use roguelike_core::movement::*;

use crate::input::*;


pub fn player_apply_action(action: Action, game_data: &mut GameData) {
    let player_handle = game_data.find_player().unwrap();
    let player_pos = game_data.objects[player_handle].pos();

    match action {
        Action::Move(movement) => {
            movement::player_move_or_attack(movement, game_data);
        }

        Action::StateChange(behavior) => {
            panic!("Player tried to change behavior?");
        }

        Action::Pickup(item_id) => {
            pick_item_up(player_handle, item_id, &mut game_data.objects);
        }

        Action::ThrowStone(throw_pos, stone_handle) => {
            throw_stone(player_pos, throw_pos, stone_handle, game_data);
        }

        Action::NoAction => {
        }
    }
}

