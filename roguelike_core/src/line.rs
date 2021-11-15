use serde::{Serialize, Deserialize};

use crate::types::*;


#[derive(Clone, Copy, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct Line {
    step_x: i32,
    step_y: i32,
    e: i32,
    delta_x: i32,
    delta_y: i32,
    orig_x: i32,
    orig_y: i32,
    dest_x: i32,
    dest_y: i32,

    include_start: bool,
}

impl Line {
    pub fn new(start: Pos, end: Pos, include_start: bool) -> Line {
        let mut line: Line = Default::default();

        line.include_start = include_start;

        line.orig_x = start.x;
        line.orig_y = start.y;

        line.dest_x = end.x;
        line.dest_y = end.y;

        line.delta_x = end.x - start.x;
        line.delta_y = end.y - start.y;

        if line.delta_x > 0 {
            line.step_x = 1;
        } else if line.delta_x < 0 {
            line.step_x = -1;
        } else {
            line.step_x = 0;
        }

        if line.delta_y > 0 {
            line.step_y = 1;
        } else if line.delta_y < 0 {
            line.step_y = -1;
        } else {
            line.step_y = 0;
        }

        if line.step_x * line.delta_x > line.step_y * line.delta_y {
            line.e = line.step_x * line.delta_x;
            line.delta_x *= 2;
            line.delta_y *= 2;
        } else {
            line.e = line.step_y * line.delta_y;
            line.delta_x *= 2;
            line.delta_y *= 2;
        }

        return line;
    }

    pub fn step(&mut self) -> Option<Pos> {
        if self.include_start {
            self.include_start = false;
            return Some(Pos::new(self.orig_x, self.orig_y));
        }

        if self.step_x * self.delta_x > self.step_y * self.delta_y {
            if self.orig_x == self.dest_x {
                return None;
            }

            self.orig_x += self.step_x;

            self.e -= self.step_y * self.delta_y;
            if self.e < 0 {
                self.orig_y += self.step_y;
                self.e += self.step_x * self.delta_x;
            }
        } else {
            if self.orig_y == self.dest_y {
                return None;
            }

            self.orig_y += self.step_y;
            self.e -= self.step_x * self.delta_x;
            if self.e < 0 {
                self.orig_x += self.step_x;
                self.e += self.step_y * self.delta_y;
            }
        }

        let x: i32 = self.orig_x;
        let y: i32 = self.orig_y;

        return Some(Pos::new(x, y));
    }
}

impl Iterator for Line {
    type Item = Pos;

    fn next(&mut self) -> Option<Pos> {
        return self.step();
    }
}

// does not include start position
pub fn line(start: Pos, end: Pos) -> Vec<Pos> {
    let include_start = false;
    let line = Line::new(start, end, include_start);
    let points = line.collect::<Vec<Pos>>();

    return points;
}

// includes end position, even if line is 0 distance
pub fn line_inclusive(start: Pos, end: Pos) -> Vec<Pos> {

    let mut points = line(start, end);

    if start != end {
        points.push(end);
    }

    return points;
}

// includes start position
pub fn line_between(start: Pos, end: Pos) -> Vec<Pos> {
    let include_start = true;
    let line = Line::new(start, end, include_start);
    let points = line.collect::<Vec<Pos>>();

    return points;
}

#[test]
pub fn test_lines() {
    let dist: i32 = 10; 
    let offset: i32 = dist / 2;

    for x in 0..dist {
        for y in 0..dist {
            let x_offset = x - offset;
            let y_offset = y - offset;
            if x_offset == 0 && y_offset == 0 {
                continue;
            }

            let start = Pos::new(0, 0);
            let end = Pos::new(x_offset, y_offset);
            let path = line(start, end);

            assert!(path[0] != start);
            assert_eq!(path[path.len() - 1], end);
        }
    }
}

