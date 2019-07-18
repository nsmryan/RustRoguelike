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
    let monster = make_orc(&Default::default(), 5, 5);
    let mut objects = Vec::new();
    objects.push(player);
    objects.push(monster);

    let mut fov_map = FovMap::new(dims.0 as i32, dims.1 as i32);

    setup_fov(&mut fov_map, &map);

    let mut animations = Vec::new();

    ai_take_turn(1,
                 &map,
                 &mut objects,
                 &fov_map,
                 &mut animations);
}

