use std::str::FromStr;

use serde::{Serialize, Deserialize};

use roguelike_core::constants::*;
use roguelike_core::messaging::*;
use roguelike_core::map::*;
use roguelike_core::types::*;
use roguelike_core::config::*;

use crate::generation::*;


#[derive(Copy, Clone, PartialOrd, PartialEq, Debug, Serialize, Deserialize)]
pub enum VaultTag {
    Medium,
    Rare,
    NoRotate,
    NoMirror,
    NoReplace,
    Common,
}

impl FromStr for VaultTag {
    type Err = String;

    fn from_str(original_str: &str) -> Result<Self, Self::Err> {

        let s: &mut str = &mut original_str.to_string();
        s.make_ascii_lowercase();

        if s == "medium" {
            return Ok(VaultTag::Medium);
        } else if s == "rare" {
            return Ok(VaultTag::Rare);
        } else if s == "norot" {
            return Ok(VaultTag::NoRotate);
        } else if s == "nomirror" {
            return Ok(VaultTag::NoMirror);
        } else if s == "noreplace" {
            return Ok(VaultTag::NoReplace);
        } else if s == "common" {
            return Ok(VaultTag::Common);
        }

        return Err(format!("Could not decode vault tag '{}'", original_str));
    }
}


#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Vault {
    pub data: Level,
    pub tags: Vec<VaultTag>,
}

impl Vault {
    pub fn new(tiles: Vec<Vec<Tile>>, tags: Vec<VaultTag>) -> Vault {
        let map = Map::with_vec(tiles);
        return Vault { data: Level::new(map, Entities::new()), tags };
    }

    pub fn empty() -> Vault {
        return Vault { data: Level::empty(0, 0), tags: Vec::new() };
    }
}


fn remove_commas(s: String) -> String {
    let s = s.chars().collect::<Vec<char>>();
    let mut new_s = String::new();
    let mut index = 0;
    if s[0] == ',' {
        new_s.push(' ');
    }

    while index < s.len() {
        if s[index] == ',' {
           if index + 1 < s.len() && s[index + 1] == ',' {
                new_s.push(' ');
           }
        } else {
            new_s.push(s[index]);
        }
        index += 1;
    }

    if s[s.len() - 1] == ',' {
        new_s.push(' ');
    }

    return new_s;
}

#[test]
fn test_remove_commas() {
    assert_eq!("   ".to_string(), remove_commas(",,".to_string()));
    assert_eq!("   ".to_string(), remove_commas(", ,".to_string()));
    assert_eq!("   ".to_string(), remove_commas(" , , ".to_string()));
    assert_eq!("  9".to_string(), remove_commas(" , ,9".to_string()));
    assert_eq!("0 9".to_string(), remove_commas("0, ,9".to_string()));
    assert_eq!("% %".to_string(), remove_commas("%,,%".to_string()));
}

/// Read Vault file into Vault structure
pub fn parse_vault(file_name: &str, config: &Config) -> Vault {
    //eprintln!("{}", file_name);

    let file_contents =
        std::fs::read_to_string(file_name).expect(&format!("Could not read {}", file_name));

    let mut lines = Vec::new();
    let mut tags: Vec<VaultTag> = Vec::new();

    for line in file_contents.lines() {
        let cleaned_line = remove_commas(line.to_string());

        if cleaned_line.starts_with("::") {
            for tag_str in cleaned_line.split_at(2).1.trim().split(" ") {
                if tag_str.starts_with("::") {
                    break;
                }
                tags.push(VaultTag::from_str(tag_str).unwrap());
            }
            break;
        }

        let char_line = cleaned_line.chars().collect::<Vec<char>>();
        lines.push(char_line);
    }

    let vault = parse_ascii_chars(lines, config);

    return vault;
}

fn parse_ascii_chars(lines: Vec<Vec<char>>, config: &Config) -> Vault {
    let height = lines.len() / 2;
    let width = (lines[0].len() - 1) / 2;

    let tile_map = vec![vec![Tile::empty(); height]; width];
    let mut vault = Vault::new(tile_map, Vec::new());

    for y in 0..height {
        for x in 0..width {
            let tile_chr = lines[y * 2][x * 2 + 1];
            let left_wall = lines[y * 2][x * 2];
            let bottom_wall = lines[y * 2 + 1][x * 2 + 1];
            let tile = tile_from_ascii(tile_chr, left_wall, bottom_wall, Pos::new(x as i32, y as i32), &mut vault, config);
            vault.data.map[(x as i32, y as i32)] = tile;
        }
    }

    return vault;
}

fn tile_from_ascii(tile_chr: char, left_wall: char, bottom_wall: char, pos: Pos, vault: &mut Vault, config: &Config) -> Tile {
    let mut tile;
    match tile_chr {
        ' ' | '\t' | '.' => {
            tile = Tile::empty();
        }

        ';' => {
            tile = Tile::rubble();
        }

        '%' => {
            tile = Tile::water();
        }
        
        '#' => {
            tile = Tile::wall_with(MAP_WALL as char);
        }

        '"' | '`' => {
            tile = Tile::grass();
        }

        'I' => {
            tile = Tile::empty();
            let mut msg_log = MsgLog::new();
            make_column(&mut vault.data.entities, config, pos, &mut msg_log);
        }

        'p' => {
            tile = Tile::empty();
            let mut msg_log = MsgLog::new();
            make_pawn(&mut vault.data.entities, config, pos, &mut msg_log);
        }

        'g' => {
            tile = Tile::empty();
            let mut msg_log = MsgLog::new();
            make_gol(&mut vault.data.entities, config, pos, &mut msg_log);
        }

        'o' => {
            tile = Tile::empty();
            let mut msg_log = MsgLog::new();
            make_stone(&mut vault.data.entities, config, pos, &mut msg_log);
        }

        '*' => {
            tile = Tile::empty();
            // TODO trap
        }

        'S' => {
            tile = Tile::empty();
            // TODO Statue - choose from list of statues
        }

        'v' => {
            tile = Tile::empty();
            let mut msg_log = MsgLog::new();
            make_dagger(&mut vault.data.entities, config, pos, &mut msg_log);
        }

        _ => {
            tile = Tile::empty();
            eprintln!("Unexpected char '{}' in {}", tile_chr, pos);
        }
    }

    if left_wall == '|' || left_wall == '\u{c780}' || left_wall as u16 == 8212 {
        tile.left_wall = Wall::ShortWall;
    }

    if bottom_wall == '_' || bottom_wall == '\u{2014}' || bottom_wall as u16 == 124 {
        tile.bottom_wall = Wall::ShortWall;
    }

    return tile;
}

