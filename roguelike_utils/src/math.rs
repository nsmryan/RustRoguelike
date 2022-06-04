use std::str::FromStr;
use std::fmt;

use serde::{Serialize, Deserialize};


#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}

impl Pos {
    pub fn new(x: i32, y: i32) -> Pos {
        return Pos { x, y };
    }

    pub fn from(pair: (i32, i32)) -> Pos {
        return Pos::new(pair.0, pair.1);
    }

    pub fn to_tuple(&self) -> (i32, i32) {
        return (self.x, self.y);
    }
}


impl fmt::Display for Pos {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.x, self.y)
    }
}

impl FromStr for Pos {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        let mut args = s.split(" ");
        let x = args.next().ok_or("no arg")?.parse::<i32>().map_err(|err| format!("{}", err))?;
        let y = args.next().ok_or("no arg")?.parse::<i32>().map_err(|err| format!("{}", err))?;

        return Ok(Pos::new(x, y));
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn white() -> Color {
        return Color::new(255, 255, 255, 255);
    }

    pub fn black() -> Color {
        return Color::new(0, 0, 0, 255);
    }

    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Color {
        return Color {r,
                      g,
                      b,
                      a,
        };
    }
}

pub fn lerp(first: f32, second: f32, scale: f32) -> f32 {
    return first + ((second - first) * scale);
}

pub fn clamp<N: Ord>(val: N, min: N, max: N) -> N {
    if val < min {
        return min;
    } else if val > max {
        return max;
    } 

    return val;
}

pub fn clampf(val: f32, min: f32, max: f32) -> f32 {
    if val < min {
        return min;
    } else if val > max {
        return max;
    } 

    return val;
}

