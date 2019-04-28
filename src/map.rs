use rand::Rng;
use std::cmp;
use std::ops::{Index, IndexMut};

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
        if self.0[x as usize][y as usize].blocked {
            return true;
        }

        objects.iter().any(|object| {
            object.blocks && object.pos() == (x, y)
        })
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

impl IndexMut<(i32, i32)> for Map {
    fn index_mut(&mut self, index: (i32, i32)) -> &mut Tile {
        &mut self.0[index.0 as usize][index.1 as usize]
    }
}


pub fn make_map(objects: &mut Vec<Object>) -> (Map, Position) {
    let mut map = Map::with_vec(vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize]);

    /* Generate rooms, from tutorial */
    //let starting_position = make_rooms(&mut map, objects);

    let starting_position = make_island(&mut map, objects);

    (map, starting_position)
}

pub fn random_position() -> Position {
    Position(rand::thread_rng().gen_range(-ISLAND_RADIUS, ISLAND_RADIUS),
             rand::thread_rng().gen_range(-ISLAND_RADIUS, ISLAND_RADIUS))
}

pub fn make_island(map: &mut Map, objects: &mut Vec<Object>) -> Position {
    let center = Position(MAP_WIDTH/2, MAP_HEIGHT/2);

    for x in 0..MAP_WIDTH {
        for y in 0..MAP_HEIGHT {
            let pos = Position(x, y);
            if pos.distance(&center) <= ISLAND_RADIUS {
                map.0[x as usize][y as usize] = Tile::empty();
            } else {
                map.0[x as usize][y as usize] = Tile::water();
            }
        }
    }

    let obstacles = Obstacle::all_obstacles();
    
    for _ in 0..ISLAND_NUM_OBSTICLES {
        let rand_pos = random_position();
        let pos = Position(center.0 + rand_pos.0, center.1 + rand_pos.1);

        let obstacle = *rand::thread_rng().choose(&obstacles).unwrap();

        // Buildings are generated separately, so don't add them in random generation
        if obstacle != Obstacle::Building {
            add_obstacle(map, &pos, obstacle);
        }
    }

    for _ in 0..rand::thread_rng().gen_range(3, 5) {
        let rand_pos = random_position();
        let pos = Position(center.0 + rand_pos.0, center.1 + rand_pos.1);
        add_obstacle(map, &pos, Obstacle::Building);
    }


    // random subtraction
    for _ in 0..ISLAND_NUM_SUBTRACTIONS_ATTEMPTS {
        let pos = Position(center.0 + rand::thread_rng().gen_range(-ISLAND_RADIUS, ISLAND_RADIUS),
                           center.1 + rand::thread_rng().gen_range(-ISLAND_RADIUS, ISLAND_RADIUS));

        if map.0[pos.0 as usize][pos.1 as usize].tile_type == TileType::Wall {
            map.0[pos.0 as usize][pos.1 as usize] = Tile::empty();
        }
    }

    // random additions
    for _ in 0..ISLAND_NUM_ADDITION_ATTEMPTS {
        let pos = Position(center.0 + rand::thread_rng().gen_range(-ISLAND_RADIUS, ISLAND_RADIUS),
                           center.1 + rand::thread_rng().gen_range(-ISLAND_RADIUS, ISLAND_RADIUS));
        let obstacle = *rand::thread_rng().choose(&obstacles).unwrap();

        if map.0[pos.0 as usize][pos.1 as usize].tile_type == TileType::Wall {
            add_obstacle(map, &pos, obstacle);
        }
    }

    // add monsters
    loop {
        let x = rand::thread_rng().gen_range(0, MAP_WIDTH);
        let y = rand::thread_rng().gen_range(0, MAP_HEIGHT);

        if !map.is_blocked(x, y, objects) {
            let mut monster = if rand::random::<f32>() < 0.8 {
                let mut orc = Object::new(x, y, 'o', "orc", DESATURATED_GREEN, true);
                orc.fighter = Some(Fighter{max_hp: 10, hp: 10, defense: 0, power: 3, on_death: DeathCallback::Monster });
                orc.ai = Some(Ai::BasicEnemy);
                orc.behavior = Some(Behavior::Idle);
                orc
            } else {
                let mut troll = Object::new(x, y, 'T', "troll", DARKER_GREEN, true);
                troll.fighter = Some(Fighter{max_hp: 16, hp: 16, defense: 1, power: 4, on_death: DeathCallback::Monster });
                troll.ai = Some(Ai::BasicEnemy);
                troll.behavior = Some(Behavior::Idle);
                troll
            };

            monster.alive = true;

            objects.push(monster);

            break;
        }
    }

    center
}

pub fn place_line(map: &mut Map, start: &Position, end: &Position, tile: Tile) -> Vec<Position> {
    let mut positions = Vec::new();
    let mut line = Line::new(start.pair(), end.pair());

    println!("{:?}, {:?}", start, end);

    while let Some(pos) = line.step() {
        println!("{:?}", pos);
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
           for x in 0..2 {
               for y in 0..2 {
                 map.0[pos.0 as usize + x][pos.1 as usize + y] = Tile::wall();
               }
           }
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

pub fn make_rooms(map: &mut Map, objects: &mut Vec<Object>) -> Position {
    let mut starting_position = (0, 0);

    let mut rooms = vec![];

    for _ in 0..MAX_ROOMS {
        let w = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        let h = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);

        let x = rand::thread_rng().gen_range(0, MAP_WIDTH - w);
        let y = rand::thread_rng().gen_range(0, MAP_HEIGHT - h);

        let new_room = Rect::new(x, y, w, h);

        let failed = rooms.iter().any(|other_room| new_room.intersects_with(other_room));

        if !failed {
            create_room(new_room, map);
            place_objects(new_room, map, objects);

            let (new_x, new_y) = new_room.center();
            if rooms.is_empty() {
                starting_position = (new_x, new_y)
            } else {
                let (prev_x, prev_y) = rooms[rooms.len()-1].center();

                if rand::random() {
                    create_h_tunnel(prev_x, new_x, prev_y, map);
                    create_v_tunnel(prev_y, new_y, prev_x, map);
                } else {
                    create_v_tunnel(prev_y, new_y, prev_x, map);
                    create_h_tunnel(prev_x, new_x, prev_y, map);
                }
            }

            rooms.push(new_room)
        }
    }

    Position(starting_position.0, starting_position.1)
}

pub fn create_h_tunnel(x1: i32, x2: i32, y: i32, map: &mut Map) {
    for x in cmp::min(x1, x2)..(cmp::max(x1, x2)+1) {
        map.0[x as usize][y as usize] = Tile::empty();
    }
}

pub fn create_v_tunnel(y1: i32, y2: i32, x: i32, map: &mut Map) {
    for y in cmp::min(y1, y2)..(cmp::max(y1, y2)+1) {
        map.0[x as usize][y as usize] = Tile::empty();
    }
}

pub fn place_objects(room: Rect, map: &Map, objects: &mut Vec<Object>) {
    let num_monsters = rand::thread_rng().gen_range(0, MAX_ROOM_MONSTERS + 1);

    for _ in 0..num_monsters {
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        if !map.is_blocked(x, y, objects) {
            let mut monster = if rand::random::<f32>() < 0.8 {
                let mut orc = Object::new(x, y, 'o', "orc", DESATURATED_GREEN, true);
                orc.fighter = Some(Fighter{max_hp: 10, hp: 10, defense: 0, power: 3, on_death: DeathCallback::Monster });
                orc.ai = Some(Ai::BasicEnemy);
                orc.behavior = Some(Behavior::Idle);
                orc
            } else {
                let mut troll = Object::new(x, y, 'T', "troll", DARKER_GREEN, true);
                troll.fighter = Some(Fighter{max_hp: 16, hp: 16, defense: 1, power: 4, on_death: DeathCallback::Monster });
                troll.ai = Some(Ai::BasicEnemy);
                troll.behavior = Some(Behavior::Idle);
                troll
            };

            monster.alive = true;

            objects.push(monster);
        }
    }

    let num_items = rand::thread_rng().gen_range(0, MAX_ROOM_ITEMS + 1);

    for _ in 0..num_items {
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        if !map.is_blocked(x, y, objects) {
            let mut object = Object::new(x, y, '!', "healing potion", VIOLET, false);
            object.item = Some(Item::Heal);
            objects.push(object);
        }
    }
}

fn create_room(room: Rect, map: &mut Map) {
    for x in (room.x1 + 1)..room.x2 {
        for y in (room.y1 + 1)..room.y2 {
            map.0[x as usize][y as usize] = Tile::empty();
        }
    }

    for x in room.x1..room.x2 {
        map.0[x as usize][room.y1 as usize] = Tile::wall();
        map.0[x as usize][room.y2 as usize] = Tile::wall();
    }

    for y in room.y1..room.y2 {
        map.0[room.x1 as usize][y as usize] = Tile::wall();
        map.0[room.x2 as usize][y as usize] = Tile::wall();
    }

    map.0[room.x2 as usize][room.y2 as usize] = Tile::wall();
}

