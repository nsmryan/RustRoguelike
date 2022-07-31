use roguelike_utils::math::*;

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
pub struct Area {
    pub x_offset: usize,
    pub y_offset: usize,
    pub width: usize,
    pub height: usize,
}

impl Area {
    pub fn new(width: usize, height: usize) -> Area {
        return Area { x_offset: 0, y_offset: 0, width, height };
    }

    pub fn new_at(x_offset: usize, y_offset: usize, width: usize, height: usize) -> Area {
        return Area { x_offset, y_offset, width, height };
    }

    pub fn dims(&self) -> (usize, usize) {
        return (self.width, self.height);
    }

    pub fn split_left(&self, left_width: usize) -> (Area, Area) {
        assert!(left_width <= self.width);

        let right_width = self.width - left_width;
        let left = Area::new_at(self.x_offset, self.y_offset, left_width, self.height);
        let right = Area::new_at(self.x_offset + left_width, self.y_offset, right_width, self.height);

        return (left, right);
    }

    pub fn split_right(&self, right_width: usize) -> (Area, Area) {
        assert!(right_width <= self.width);

        let left_width = self.width - right_width;
        let left = Area::new_at(self.x_offset, self.y_offset, left_width, self.height);
        let right = Area::new_at(self.x_offset + left_width, self.y_offset, right_width, self.height);

        return (left, right);
    }

    pub fn split_top(&self, top_height: usize) -> (Area, Area) {
        assert!(top_height <= self.height);

        let top = Area::new_at(self.x_offset, self.y_offset, self.width, top_height);
        let bottom = Area::new_at(self.x_offset, self.y_offset + top_height, self.width, self.height - top_height);

        return (top, bottom);
    }

    pub fn split_bottom(&self, bottom_height: usize) -> (Area, Area) {
        assert!(bottom_height <= self.height);

        let top_height = self.height - bottom_height;
        let top = Area::new_at(self.x_offset, self.y_offset, self.width, top_height);
        let bottom = Area::new_at(self.x_offset, self.y_offset + top_height, self.width, bottom_height);

        return (top, bottom);
    }

    pub fn centered(&self, width: usize, height: usize) -> Area {
        assert!(width <= self.width);
        assert!(height <= self.height);

        let x_offset = (self.width - width) / 2;
        let y_offset = (self.height - height) / 2;

        return Area::new_at(x_offset, y_offset, width, height);
    }

    pub fn cell_at_pixel(&self, pixel_pos: Pos) -> Option<(usize, usize)> {
        let cell_pos = Pos::new(pixel_pos.x / self.width as i32, pixel_pos.y / self.height as i32);

        return self.cell_at(cell_pos);
    }

    pub fn cell_at(&self, cell_pos: Pos) -> Option<(usize, usize)> {
        if cell_pos.x as usize >= self.x_offset && (cell_pos.x as usize) < self.x_offset + self.width &&
           cell_pos.y as usize >= self.y_offset && (cell_pos.y as usize) < self.y_offset + self.height {
               return Some((cell_pos.x as usize - self.x_offset, cell_pos.y as usize - self.y_offset));
        }

        return None;
    }
}

#[test]
pub fn test_area_splits_left() {
    let section = Area::new(100, 100);
    let (left, right) = section.split_left(20);

    assert_eq!(0, left.x_offset);
    assert_eq!(0, left.y_offset);
    assert_eq!(20, right.x_offset);
    assert_eq!(0, right.y_offset);

    assert_eq!(20, left.width);
    assert_eq!(80, right.width);
    assert_eq!(100, left.height);
    assert_eq!(100, right.height);
}

#[test]
pub fn test_area_splits_top() {
    let section = Area::new(100, 100);
    let (top, bottom) = section.split_top(20);

    assert_eq!(0, top.x_offset);
    assert_eq!(0, top.y_offset);
    assert_eq!(0, bottom.x_offset);
    assert_eq!(20, bottom.y_offset);

    assert_eq!(100, top.width);
    assert_eq!(100, bottom.width);
    assert_eq!(20, top.height);
    assert_eq!(80, bottom.height);
}

#[test]
pub fn test_area_splits_right() {
    let section = Area::new(100, 100);
    let (left, right) = section.split_right(20);

    assert_eq!(0, left.x_offset);
    assert_eq!(0, left.y_offset);
    assert_eq!(80, right.x_offset);
    assert_eq!(0, right.y_offset);

    assert_eq!(80, left.width);
    assert_eq!(20, right.width);
    assert_eq!(100, left.height);
    assert_eq!(100, right.height);
}

#[test]
pub fn test_area_splits_bottom() {
    let section = Area::new(100, 100);
    let (top, bottom) = section.split_bottom(20);
    assert_eq!(0, top.x_offset);
    assert_eq!(0, top.y_offset);
    assert_eq!(0, bottom.x_offset);
    assert_eq!(80, bottom.y_offset);

    assert_eq!(100, top.width);
    assert_eq!(100, bottom.width);
    assert_eq!(80, top.height);
    assert_eq!(20, bottom.height);
}

