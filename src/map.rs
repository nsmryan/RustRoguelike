use rand::Rng;
use std::cmp;
use std::ops::{Index, IndexMut};
use std::print;

use tcod::colors::*;
use tcod::line::*;

use crate::constants::*;
use crate::types::*;


pub struct Map(pub Vec<Vec<Tile>>);

impl Map {
    pub fn with_vec(map: Vec<Vec<Tile>>) -> Map {
        Map(map)
    }

    pub fn is_blocked(&self, x: i32, y: i32, objects: &[Object]) -> bool {
        if self[(x, y)].blocked {
            return true;
        }

        let mut is_blocked = false;
        for object in objects.iter() {
            if object.blocks && object.pos() == (x, y) {
                is_blocked = true;
                break;
            }
        }
        return is_blocked;
    }

    pub fn is_empty(&self, x: i32, y: i32, _objects: &[Object]) -> bool {
        self[(x, y)].tile_type == TileType::Empty
    }

    pub fn size(&self) -> (i32, i32) {
        (self.0.len() as i32, self.0[0].len() as i32)
    }

    pub fn make_tcod_map(&self) -> tcod::map::Map {
        let (map_width, map_height) = self.size();
        let mut map_copy = tcod::map::Map::new(map_width, map_height);
        for x in 0..map_width {
            for y in 0..map_height {
                let transparent = !self.0[x as usize][y as usize].block_sight;
                let walkable = !self.0[x as usize][y as usize].blocked;
                map_copy.set(x, y, transparent, walkable);
            }
        }

        map_copy
    }
}

impl Index<(i32, i32)> for Map {
    type Output = Tile;

    fn index(&self, index: (i32, i32)) -> &Tile {
        &self.0[index.0 as usize][index.1 as usize]
    }
}

impl Index<Position> for Map {
    type Output = Tile;

    fn index(&self, position: Position) -> &Tile {
        &self.0[position.0 as usize][position.1 as usize]
    }
}

impl IndexMut<(i32, i32)> for Map {
    fn index_mut(&mut self, index: (i32, i32)) -> &mut Tile {
        &mut self.0[index.0 as usize][index.1 as usize]
    }
}

pub fn near_tile_type(map: &Map, position: Position, tile_type: TileType) -> bool {
    let neighbor_offsets: Vec<(i32, i32)>
        = vec!((1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1));

    /*
     neighbor_offsets.iter()
                     .map(|offset| position.add(Position(offset.0, offset.1)))
                     .any(|pos| map[pos].tile_type == tile_type)
    */

    let mut near_given_tile = false;

    for offset in neighbor_offsets {
        let neighbor_position = position.add(Position(offset.0, offset.1));

        if map[neighbor_position].tile_type == tile_type {
            near_given_tile = true;
            break;
        }
    }

    return near_given_tile;
}

pub fn make_map(objects: &mut Vec<Object>, config: &Config) -> (Map, Position) {
    let mut map = Map::with_vec(vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize]);

    let starting_position = make_island(&mut map, objects, config);

    (map, starting_position)
}

pub fn random_offset() -> Position {
    Position(rand::thread_rng().gen_range(-ISLAND_RADIUS, ISLAND_RADIUS),
             rand::thread_rng().gen_range(-ISLAND_RADIUS, ISLAND_RADIUS))
}

pub fn random_position() -> Position {
    Position(rand::thread_rng().gen_range(0, MAP_WIDTH), rand::thread_rng().gen_range(0, MAP_HEIGHT))
}

pub fn pos_in_radius(pos: Position, radius: i32) -> Position {
    return Position(pos.0 + rand::thread_rng().gen_range(-radius, radius),
                    pos.1 + rand::thread_rng().gen_range(-radius, radius));
}

pub fn make_island(map: &mut Map, objects: &mut Vec<Object>, config: &Config) -> Position {
    let center = Position(MAP_WIDTH/2, MAP_HEIGHT/2);

    let mut water_tile_positions = Vec::new();

    /* Create Island */
    // the center has land, the remaining square are filled with water
    for x in 0..MAP_WIDTH {
        for y in 0..MAP_HEIGHT {
            let pos = Position(x, y);
            if pos.distance(&center) <= ISLAND_RADIUS {
                map.0[x as usize][y as usize] = Tile::empty();
            } else {
                map.0[x as usize][y as usize] = Tile::water();
                water_tile_positions.push((x, y));
            }
        }
    }

    /* add obstacles */
    let obstacles = Obstacle::all_obstacles();

    for _ in 0..ISLAND_NUM_OBSTACLES {
        let rand_pos = random_offset();
        let pos = Position(center.0 + rand_pos.0, center.1 + rand_pos.1);

        let obstacle = *rand::thread_rng().choose(&obstacles).unwrap();

        // Buildings are generated separately, so don't add them in random generation
        if obstacle != Obstacle::Building {
            add_obstacle(map, &pos, obstacle);
        }
    }

    /* add buildings */
    for _ in 0..rand::thread_rng().gen_range(3, 5) {
        let rand_pos = random_offset();
        let pos = Position(center.0 + rand_pos.0, center.1 + rand_pos.1);
        add_obstacle(map, &pos, Obstacle::Building);
    }

    /* random subtraction */
    for _ in 0..ISLAND_NUM_SUBTRACTIONS_ATTEMPTS {
        let pos = pos_in_radius(center, ISLAND_RADIUS);

        if map.0[pos.0 as usize][pos.1 as usize].tile_type == TileType::Wall {
            map.0[pos.0 as usize][pos.1 as usize] = Tile::empty();
        }
    }

    /* random additions */
    for _ in 0..ISLAND_NUM_ADDITION_ATTEMPTS {
        let pos = pos_in_radius(center, ISLAND_RADIUS);
        let obstacle = *rand::thread_rng().choose(&obstacles).unwrap();

        if map.0[pos.0 as usize][pos.1 as usize].tile_type == TileType::Wall {
            add_obstacle(map, &pos, obstacle);
        }
    }

    /* random stones */
    let mut rng = rand::thread_rng();
    for _ in 0..10 {
        let pos = pos_in_radius(center, ISLAND_RADIUS);

        if map.is_empty(pos.0, pos.1, &objects) {
            let mut stone = Object::make_stone(pos.0, pos.1);
            stone.item = Some(Item::Stone);
            objects.push(stone);
        }
    }

    /* add monsters */
    loop {
        let (x, y) = pos_in_radius(center, ISLAND_RADIUS).pair();

        if !map.is_blocked(x, y, objects) {
            let mut monster = if rand::random::<f32>() < 1.0 {
                let mut orc = Object::new(x, y, 'o', "orc", DESATURATED_GREEN, true);
                orc.fighter = Some( Fighter { max_hp: 10, hp: 10, defense: 0, power: 3, on_death: DeathCallback::Monster } );
                orc.ai = Some(Ai::Basic);
                orc.behavior = Some(Behavior::Idle);
                orc.color = config.color_orc.color();
                orc.movement = Some(Reach::Single);
                orc.attack = Some(Reach::Diag);
                orc
            } else {
                let mut troll = Object::new(x, y, 'T', "troll", DARKER_GREEN, true);
                troll.fighter = Some( Fighter { max_hp: 16, hp: 16, defense: 1, power: 4, on_death: DeathCallback::Monster } );
                troll.ai = Some(Ai::Basic);
                troll.behavior = Some(Behavior::Idle);
                troll.color = config.color_troll.color();
                troll.movement = Some(Reach::Single);
                troll.attack = Some(Reach::Diag);
                troll
            };

let num_items = rand::thread_rng().gen_range(0, MAX_ROOM_ITEMS + 1);

    for _ in 0..num_items {
        let x = rand::thread_rng().gen_range(0, MAP_WIDTH);
        let y = rand::thread_rng().gen_range(0, MAP_HEIGHT);

        if !map.is_blocked(x, y, objects) {
            let mut object = Object::new(x, y, '!', "healing potion", VIOLET, false);
            object.item = Some(Item::Heal);
            objects.push(object);
        }
    }
        let x = rand::thread_rng().gen_range(0, MAP_WIDTH);
        let y = rand::thread_rng().gen_range(0, MAP_HEIGHT);
            
        if !map.is_blocked(x, y, objects) {
            let mut object = Object::new(x,y, '\u{FD}', "goal", RED, false);
            object.item = Some(Item::Goal);
            objects.push(object);
        }


            monster.alive = true;

            objects.push(monster);

            break;
        }
    }

    /* add goal object */
    let (mut x, mut y) = pos_in_radius(center, ISLAND_RADIUS).pair();
    while !map.is_empty(x, y, &objects) {
        let pos = pos_in_radius(center, ISLAND_RADIUS).pair();
        x = pos.0;
        y = pos.1;
    }
    let mut object = Object::new(x, y, '\u{FD}', "goal", RED, false);
    object.item = Some(Item::Goal);
    objects.push(object);

    /* add exit */
    // find edge of island
    let map_size = map.size();
    let mut edge_positions = Vec::new();
    for x in 0..map_size.0 {
        for y in 0..map_size.1 {
            let pos = Position::from_pair(&(x, y));
            if !(map[(x, y)].tile_type == TileType::Water) && near_tile_type(&map, pos, TileType::Water) {
                edge_positions.push(pos);
            }
        }
    }
    // choose a random edge position
    let edge_pos = edge_positions[rand::thread_rng().gen_range(0, edge_positions.len())];

    // make the random edge position the exit
    map.0[edge_pos.0 as usize][edge_pos.1 as usize] = Tile::exit();

    /* Ensure that objects placed outside of the island are removed */
    for pos in water_tile_positions {
        map[pos].tile_type = TileType::Water;
    }

    center
}

pub fn place_block(map: &mut Map, start: &Position, width: i32, tile: Tile) -> Vec<Position> {
    let mut positions = Vec::new();

    for x in 0..width {
        for y in 0..width {
            let pos = (start.0 + x, start.1 + y);
            map[pos] = tile;
            positions.push(Position::new(pos.0, pos.1));
        }
    }

    positions
}

pub fn place_line(map: &mut Map, start: &Position, end: &Position, tile: Tile) -> Vec<Position> {
    let mut positions = Vec::new();
    let mut line = Line::new(start.pair(), end.pair());

    while let Some(pos) = line.step() {
        map[pos] = tile;
        positions.push(Position::new(pos.0, pos.1));
    }

    positions
}

pub fn add_obstacle(map: &mut Map, pos: &Position, obstacle: Obstacle) {
    match obstacle {
        Obstacle::Block => {
            map.0[pos.0 as usize][pos.1 as usize] = Tile::wall();
        }

        Obstacle::Wall => {
            let end_pos = if rand::thread_rng().next_f64() < 0.5 {
                pos.move_x(3)
            } else {
                pos.move_y(3)
            };
            place_line(map, pos, &end_pos, Tile::wall());
        }

        Obstacle::ShortWall => {
            let end_pos = if rand::thread_rng().next_f64() < 0.5 {
                pos.move_x(3)
            } else {
                pos.move_y(3)
            };
            place_line(map, pos, &end_pos, Tile::short_wall());
        }

        Obstacle::Square => {
            place_block(map, pos, 2, Tile::wall());
        }

        Obstacle::LShape => {
            let dir = rand::thread_rng().choose(&[1, -1]).unwrap();

            if rand::thread_rng().next_f64() < 0.5 {
                for x in 0..3 {
                    map.0[pos.0 as usize + x][pos.1 as usize] = Tile::wall();
                }
                map.0[pos.0 as usize][(pos.1 + dir) as usize] = Tile::wall();
            } else {
                for y in 0..3 {
                    map.0[pos.0 as usize][pos.1 as usize + y] = Tile::wall();
                }
                map.0[(pos.0 + dir) as usize][pos.1 as usize] = Tile::wall();
            }
        }

        Obstacle::Building => {
            let size = 2;

            let mut positions = vec!();
            positions.append(&mut place_line(map, &pos.move_by(-size, size), &pos.move_by(size, size), Tile::wall()));
            positions.append(&mut place_line(map, &pos.move_by(-size, size), &pos.move_by(-size, -size), Tile::wall()));
            positions.append(&mut place_line(map, &pos.move_by(-size, -size), &pos.move_by(size, -size), Tile::wall()));
            positions.append(&mut place_line(map, &pos.move_by(size, -size), &pos.move_by(size, size), Tile::wall()));

            for _ in 0..rand::thread_rng().gen_range(0, 10) {
                positions.swap_remove(rand::thread_rng().gen_range(0, positions.len()));
            }
        }
    }
}

