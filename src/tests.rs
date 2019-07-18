#[allow(unused_imports)]use tcod::map::{Map as FovMap};

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

    let mut animations = Vec::new();

    let hp = objects[PLAYER].hp;
    ai_take_turn(monster_id,
                 &map,
                 &mut objects,
                 &fov_map,
                 &mut animations);

    assert_eq!(objects[PLAYER].hp, hp);
    assert_eq!(objects[monster_id].pos(), (5, 5));
}

