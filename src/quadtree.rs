use nannou::{color::{rgb, WHITE}, glam::Vec2};

#[derive(Clone, Copy, Debug, PartialEq)]    
pub struct Point {
    pub id : usize,
    pub position: Vec2,
    pub prev_position: Vec2,
    pub acceleration: Vec2,
    pub radius: f32,
    pub color: rgb::Rgb<nannou::color::encoding::Srgb, f64>,
}

impl Point {
    pub fn new(id: usize, position: Vec2, prev_position: Vec2, acceleration: Vec2, radius: f32, color: rgb::Rgb<nannou::color::encoding::Srgb, f64>) -> Point {
        Point { id, position, prev_position, acceleration, radius, color }
    }
}

#[derive(Debug)]
pub struct QuadTree {
    left_x: f32,
    bottom_y: f32,
    width: f32,
    height: f32,
    root: Node,
}

impl QuadTree {
    pub fn new(left_x: f32, bottom_y: f32, width: f32, height: f32) -> QuadTree {
        QuadTree {
            left_x: left_x,
            bottom_y: bottom_y,
            width: width,
            height: height,
            root: Node::Leaf{ value: Vec::new() },
        }
    }

    pub fn from_points(points: Vec<Point>, left_x: f32, bottom_y: f32, width: f32, height: f32, points_per_quad: usize) -> QuadTree {
        let mut tree = QuadTree::new(left_x, bottom_y, width, height);
        for point in points.into_iter().filter(|p| p.position.x >= left_x && p.position.x <= left_x + width && p.position.y >= bottom_y && p.position.y <= bottom_y + height) {
            tree.insert(point, points_per_quad);
        }
        tree
    }

    pub fn query_radius(&self, x: f32, y: f32, radius: f32) -> Vec<&Point> {
        let mut result = Vec::new();
        // QuadTree::query_radius_rec(&self.root, x, y, self.width, self.height, self.left_x, self.bottom_y, radius, &mut result);

        let mut stack = vec![(&self.root, self.left_x, self.bottom_y, self.width, self.height)];
        while let Some((node, quad_x, quad_y, width, height)) = stack.pop() {
            if x + radius < quad_x || x - radius > quad_x + width || y + radius < quad_y || y - radius > quad_y + height {
                continue;
            }
            match node {
                Node::Leaf{ value } => {
                    result.extend(value);
                },
                Node::Branch{ nw, ne, sw, se } => {
                    let width = width / 2.0;
                    let height: f32 = height / 2.0;
                    let x_mid = quad_x + width;
                    let y_mid = quad_y + height;
                    stack.push((nw, quad_x, y_mid, width, height));
                    stack.push((ne, x_mid, y_mid, width, height));
                    stack.push((sw, quad_x, quad_y, width, height));
                    stack.push((se, x_mid, quad_y, width, height));
                }
            }
        }

        result
    }

    fn query_radius_rec<'a>(node: &'a Node, x: f32, y: f32, width: f32, height: f32, quad_x: f32, quad_y: f32, radius: f32, result: &mut Vec<&'a Point>) {
        let x_right = quad_x + width;
        let y_top = quad_y + height;
        if x + radius < quad_x || x - radius > x_right || y + radius < quad_y || y - radius > y_top {
            return;
        }
        match node {
            Node::Leaf{ value } => {
                result.extend(value);
            },
            Node::Branch{ nw, ne, sw, se } => {                
                let width = width / 2.0;
                let height: f32 = height / 2.0;
                let x_mid = quad_x + width;
                let y_mid = quad_y + height;
                QuadTree::query_radius_rec(nw, x, y, width, height, quad_x, y_mid, radius, result);
                QuadTree::query_radius_rec(ne, x, y, width, height, x_mid, y_mid, radius, result);
                QuadTree::query_radius_rec(sw, x, y, width, height, quad_x, quad_y, radius, result);
                QuadTree::query_radius_rec(se, x, y, width, height, x_mid, quad_y, radius, result);
            }
        }
    }

    fn insert(&mut self, ball: Point, points_per_quad: usize) {
        let mut node = &mut self.root;
        let mut width = self.width;
        let mut height = self.height;
        let mut x = self.left_x;
        let mut y = self.bottom_y;
        loop {
            match node {
                Node::Leaf{ value } => {
                    value.push(ball);
                    if value.len() > points_per_quad {
                        width /= 2.0;
                        height /= 2.0;
                        let x_mid = x + width;
                        let y_mid = y + height;
                        let (mut north, mut south): (Vec<Point>, Vec<Point>) = value.drain(..).partition(|b| b.position.y >= y_mid);
                        let (nw, ne): (Vec<Point>, Vec<Point>) = north.drain(..).partition(|b| b.position.x < x_mid);
                        let (sw, se): (Vec<Point>, Vec<Point>) = south.drain(..).partition(|b| b.position.x < x_mid);

                        *node = Node::Branch{ 
                            nw: Box::new(Node::Leaf{ value: nw }),
                            ne: Box::new(Node::Leaf{ value: ne }),
                            sw: Box::new(Node::Leaf{ value: sw }),
                            se: Box::new(Node::Leaf{ value: se }),
                        };
                    }
                    return;
                },
                Node::Branch{ nw, ne, sw, se } => {
                    width /= 2.0;
                    height /= 2.0;
                    let x_mid = x + width;
                    let y_mid = y + height;
                    if ball.position.x < x_mid {
                        if ball.position.y > y_mid {
                            node = nw;
                            y = y_mid;
                        } else {
                            node = sw;
                        }
                    } else {
                        if ball.position.y > y_mid {
                            node = ne;
                            x = x_mid;
                            y = y_mid;
                        } else {
                            node = se;
                            x = x_mid;
                        }
                    }
                }
            }
        }

    }
    
    pub fn draw_quad_tree_outlines(&self, draw: &nannou::draw::Draw) {
        QuadTree::draw_quad_tree_outlines_rec(draw, &self.root, self.left_x, self.bottom_y, self.width, self.height);
    }

    fn draw_quad_tree_outlines_rec(draw: &nannou::draw::Draw, node: &Node, x: f32, y: f32, width: f32, height: f32) {
        match node {
            Node::Leaf{ .. } => {
                draw.rect()
                    .x_y(x + width / 2.0, y + height / 2.0)
                    .w_h(width, height)
                    .stroke(WHITE)
                    .stroke_weight(1.0)
                    .z(50.0)
                    .no_fill();
            },
            Node::Branch{ nw, ne, sw, se } => {
                let x_mid = x + width / 2.0;
                let y_mid = y + height / 2.0;
                QuadTree::draw_quad_tree_outlines_rec(draw, nw, x, y_mid, width / 2.0, height / 2.0);
                QuadTree::draw_quad_tree_outlines_rec(draw, ne, x_mid, y_mid, width / 2.0, height / 2.0);
                QuadTree::draw_quad_tree_outlines_rec(draw, sw, x, y, width / 2.0, height / 2.0);
                QuadTree::draw_quad_tree_outlines_rec(draw, se, x_mid, y, width / 2.0, height / 2.0);
            }
        }
    }
}

#[derive(Debug)]
enum Node {
    Leaf{
        value: Vec<Point>,
    },
    Branch{
        nw: Box<Node>,
        ne: Box<Node>,
        sw: Box<Node>,
        se: Box<Node>,
    }
}