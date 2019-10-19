use tcod::map::{Map as FovMap};
use tcod::line::*;

use crate::engine::ai::*;
use crate::engine::types::*;
use crate::engine::map::*;
use crate::engine::display::*;
use crate::engine::types::*;
use crate::input::*;
use crate::constants::*;
use crate::game::*;


#[test]
fn test_enemy_move() {
    let dims = (10, 10);
    let map = Map::from_dims(dims.0, dims.1);

    let player = make_player();
    let monster = make_orc(&Default::default(), 4, 5);
    let monster_id = 1;

    let mut objects = Vec::new();
    objects.push(player);
    objects.push(monster);

    let mut fov_map = FovMap::new(dims.0 as i32, dims.1 as i32);
    setup_fov(&mut fov_map, &map);

    let hp = objects[PLAYER].fighter.unwrap().hp;

    let turn = basic_ai_take_turn(monster_id, &map, &mut objects, &fov_map);

    let mut expected_turn = AiTurn::new();
    expected_turn.add(AiAction::Move((1, 0)));
    assert_eq!(turn, expected_turn);
}

#[test]
fn test_line_inclusive() {
    let mut move_line = Line::new((0, 0), (1, 1)).into_iter();

    assert_eq!(move_line.next(), Some((0, 0)));
    assert_eq!(move_line.next(), Some((1, 1)));
    assert_eq!(move_line.next(), None);
}
