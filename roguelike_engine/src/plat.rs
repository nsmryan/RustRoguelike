use std::boxed::Box;


pub type Percent = f32;

#[derive(Clone)]
pub struct Zone {
    name: Option<String>,
}

impl Zone {
    pub fn empty() -> Zone {
        Zone { name: None }
    }

    pub fn new(name: String) -> Zone {
        Zone { name: Some(name) }
    }
}

#[derive(Clone)]
pub enum Split {
    Horiz,
    Vert,
}

#[derive(Clone)]
pub struct Node {
    name: Option<String>,
    split: Split,
    percent: Percent,
}

impl Node {
    pub fn horiz(name: Option<String>, percent: Percent) -> Node {
        Node { name, percent, split: Split::Horiz }
    }

    pub fn vert(name: Option<String>, percent: Percent) -> Node {
        Node { name, percent, split: Split::Vert }
    }
}

#[derive(Clone)]
pub enum Plan {
    Zone(Zone),
    Node(Node, Box<Plan>, Box<Plan>),
}

impl Plan {
    pub fn empty() -> Plan {
        Plan::Zone(Zone::empty())
    }

    pub fn zone<S>(name: S) -> Plan
        where S: Into<String> {
        Plan::Zone(Zone::new(name.into()))
    }

    pub fn node(node: Node, left: Plan, right: Plan) -> Plan {
        Plan::Node(node, Box::new(left), Box::new(right))
    }

    pub fn vert<S>(name: S, percent: Percent, left: Plan, right: Plan) -> Plan
        where S: Into<String> {
        Plan::node(Node::vert(Some(name.into()), percent), left, right)
    }

    pub fn horiz<S>(name: S, percent: Percent, left: Plan, right: Plan) -> Plan 
        where S: Into<String> {
        Plan::node(Node::horiz(Some(name.into()), percent), left, right)
    }

    pub fn split_vert(percent: Percent, left: Plan, right: Plan) -> Plan {
        Plan::node(Node::vert(None, percent), left, right)
    }

    pub fn split_horiz(percent: Percent, left: Plan, right: Plan) -> Plan {
        Plan::node(Node::horiz(None, percent), left, right)
    }

    pub fn plot(&self, x: usize, y: usize, width: usize, height: usize) -> impl Iterator<Item=Plot> {
        let mut plots = Vec::new();

        Plan::plot_helper(Box::new(self.clone()), x, y, width, height, &mut plots);

        return plots.into_iter();
    }

    fn plot_helper(plan: Box<Plan>, x: usize, y: usize, width: usize, height: usize, plots: &mut Vec<Plot>) {
        match *plan {
            Plan::Zone(zone) => {
                if let Some(name) = zone.name {
                    plots.push(Plot { name: name,
                                      x,
                                      y,
                                      width,
                                      height,
                    });
                }
            }

            Plan::Node(node, left, right) => {
                if let Some(name) = node.name {
                    plots.push(Plot { name: name,
                                      x,
                                      y,
                                      width,
                                      height,
                    });
                }

                match node.split {
                    Split::Horiz => {
                        let new_height = (height as f32 * node.percent) as usize;
                        Plan::plot_helper(left,  x, y,              width, new_height,          plots);
                        Plan::plot_helper(right, x, y + new_height, width, height - new_height, plots);
                    }

                    Split::Vert => {
                        let new_width = (width as f32 * node.percent) as usize;
                        Plan::plot_helper(left,  x,             y, new_width,         height, plots);
                        Plan::plot_helper(right, x + new_width, y, width - new_width, height, plots);
                    }
                }
            }
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Plot {
    pub name: String,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

impl Plot {
    pub fn offset(&self, x: usize, y: usize, width: usize, height: usize) -> (usize, usize) {
        let scale = self.scale(width, height);
        return (self.x + (scale.0 * x as f32) as usize, self.y + (scale.1 * y as f32) as usize);
    }

    pub fn scale(&self, width: usize, height: usize) -> (f32, f32) {
        return (self.width as f32 / width as f32, self.height as f32 / height as f32);
    }

    pub fn fit(&self, width: usize, height: usize) -> ((f32, f32), f32) {
        let scale_x = self.width as f32 / width as f32;
        let scale_y = self.height as f32 / height as f32;

        let scaler;
        if scale_x * self.height as f32 > height as f32 {
            scaler = scale_y;
        } else {
            scaler = scale_x;
        }

        let x_offset = (self.width  as f32 - (width as f32 * scaler)) / 2.0;
        let y_offset = (self.height as f32 - (height as f32 * scaler)) / 2.0;

        return ((x_offset, y_offset), scaler);
    }

    pub fn dims(&self) -> (usize, usize) {
        return (self.width, self.height);
    }

    pub fn pos(&self) -> (usize, usize) {
        return (self.x, self.y);
    }

    pub fn contains(&self, x: usize, y: usize) -> bool {
        return self.x > x && self.y > y && x < self.x + self.width && y < self.y + self.height;
    }

    pub fn within(&self, x: usize, y: usize) -> (usize, usize) {
        return (x - self.x, y - self.y);
    }
}

