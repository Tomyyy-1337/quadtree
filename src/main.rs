use nannou::{color::{BLACK, WHITE}, event::Update, glam::Vec2, rand::random_range, App, Frame};
mod quadtree;
use quadtree::{Point, QuadTree};
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};

fn main() {
    rayon::ThreadPoolBuilder::new().num_threads(8).build_global().unwrap();

    nannou::app(Model::new)
        .update(Model::update)
        .run();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpawnerMode {
    Inactive,
    TopLeft,
}

struct Model {
    points: Vec<Point>,
    points_per_quad: usize,
    running: bool,
    show_quad_tree: bool,
    egui: nannou_egui::Egui,
    spawner_mode: SpawnerMode,
    mouse_radius: f32,
    minimum_size: f32,
    maximum_size: f32,
}

impl Model {
    fn new(app: &App) -> Model {
        let window_id = app.new_window()
            .size(800, 800)
            .view(Model::view)
            .raw_event(Model::raw_window_event)
            .build()
            .unwrap();

        let window = app.window(window_id).unwrap();

        Model {
            points: Vec::new(),
            points_per_quad: 16,
            running: true,
            show_quad_tree: true,
            egui: nannou_egui::Egui::from_window(&window),
            spawner_mode: SpawnerMode::TopLeft,
            mouse_radius: 16.0,
            minimum_size: 8.0,
            maximum_size: 16.0,
        }
    }

    fn update(app: &App, model: &mut Model, update: Update) {
        model.update_egui(update);
        
        match model.spawner_mode {
            SpawnerMode::Inactive => (),
            SpawnerMode::TopLeft => {
                let pos1 = Vec2::new(app.window_rect().left() + model.maximum_size, app.window_rect().top() - model.maximum_size);
                let pos2 = Vec2::new(app.window_rect().left() + model.maximum_size, app.window_rect().top() - model.maximum_size * 3.0);
                let pos3 = Vec2::new(app.window_rect().left() + model.maximum_size, app.window_rect().top() - model.maximum_size * 5.0);
                let pos4 = Vec2::new(app.window_rect().left() + model.maximum_size, app.window_rect().top() - model.maximum_size * 7.0);
                let arr = [pos1, pos2, pos3, pos4];
                arr.iter().for_each(|pos| {
                    if model.points.iter().find(|p| p.position.distance(*pos) < model.maximum_size * 2.0).is_none() {
                        model.spawn_point(*pos);
                    }
                });
            },
        }
        
        if !model.running || model.points.is_empty() {
            return;
        }
        
        let gravity = -2200.0;
        
        let substeps: i32 = 4;
        for _ in 0..substeps {            
            model.resolve_collisions(app);
            
            model.resolve_wall_collisions(gravity, app);
            
            model.resolve_mouse_collisions(app);
            
            model.resolve_collisions(app);
            
            let delta = 1.0 / (substeps as f32 * 90.0);
            for point in &mut model.points {
                let disp = point.position - point.prev_position;
                point.prev_position = point.position;
                point.acceleration *= delta * delta;
                point.position += disp + point.acceleration;
                point.acceleration = Vec2::ZERO;
            }
        }   
    }

    fn view(app: &App, model: &Model, frame: Frame) {
        let draw = app.draw();
        draw.background().color(BLACK);

        for point in &model.points {
            draw.ellipse()
                .xy(point.position)
                .radius(point.radius)
                .resolution(12.0)
                .color(point.color);
        }

        draw.ellipse()
            .xy(app.mouse.position())
            .radius(model.mouse_radius)
            .color(WHITE);

        let width = app.window_rect().w();
        let height = app.window_rect().h();

        let quad_tree = QuadTree::from_points(model.points.clone(), -width/2.0, -height/2.0, width, height, model.points_per_quad);
        
        let mouse_pos = app.mouse.position();

        let query = quad_tree.query_radius(mouse_pos.x, mouse_pos.y, model.mouse_radius + model.maximum_size);
        for p in query {
            draw.line()
                .start(mouse_pos)
                .end(p.position)
                .color(WHITE);
        }
        

        if model.show_quad_tree {
            quad_tree.draw_quad_tree_outlines(&draw);
        }

        draw.to_frame(app, &frame).unwrap();
        model.egui.draw_to_frame(&frame).unwrap();
    }

    fn raw_window_event(_app: &App, model: &mut Model, event: &nannou::winit::event::WindowEvent) {
        model.egui.handle_raw_event(event);
    }

    fn resolve_mouse_collisions(&mut self, app: &App) {
        let mouse_pos = app.mouse.position();
        self.points.iter_mut().for_each(|point| {
            let axis = point.position - mouse_pos;
            let dist = axis.length();
            if dist <= point.radius + self.mouse_radius {
                let delta = point.radius + self.mouse_radius - dist;
                let norm = axis.normalize() * delta * 0.5;
                point.position += norm;
            }
        });
    }

    fn update_egui(&mut self, update: Update) {
        let ctx = self.egui.begin_frame();
        nannou_egui::egui::Window::new("Quad Tree").show(&ctx, |ui| {
            ui.heading("Settings");
            if ui.button("Clear").clicked() {
                self.points.clear();
            }
            if ui.button("Toggle Simulation").clicked() {
                self.running = !self.running;
            }
            ui.checkbox(&mut self.show_quad_tree, "Show Quad Tree");
            ui.add(nannou_egui::egui::Slider::new(&mut self.points_per_quad, 1..=1000).logarithmic(true).text("Points per Quad"));
            ui.add(nannou_egui::egui::Slider::new(&mut self.mouse_radius, 1.0..=100.0).text("Mouse Radius"));
            ui.add(nannou_egui::egui::Slider::new(&mut self.minimum_size, 5.0..=self.maximum_size - 0.1).logarithmic(true).text("Minimum Size"));
            ui.add(nannou_egui::egui::Slider::new(&mut self.maximum_size, self.minimum_size+0.1..=100.0).logarithmic(true).text("Maximum Size"));
            ui.horizontal(|ui| {
                ui.label("Spawner Mode:");
                ui.radio_value(&mut self.spawner_mode, SpawnerMode::Inactive, "Inactive");
                ui.radio_value(&mut self.spawner_mode, SpawnerMode::TopLeft, "Top Left");
            });
            ui.label(format!("Frame time: {:.2} ms", update.since_last.as_secs_f64() * 1000.0));
        });
    }

    fn spawn_point(&mut self, position: Vec2) {
        let random_color = nannou::color::rgb(rand::random(), rand::random(), rand::random());
        let random_radius = random_range(self.minimum_size, self.maximum_size);
        self.points.push(Point::new(self.points.len(), position, position + Vec2::new(-2.0, 0.0), Vec2::ZERO, random_radius, random_color));
    }

    fn resolve_collisions(&mut self, app: &App) {
        let quadtree = QuadTree::from_points(self.points.clone(), app.window_rect().left(), app.window_rect().bottom(), app.window_rect().w(), app.window_rect().h(), self.points_per_quad);
        let max_radius = self.maximum_size;
        self.points.par_iter_mut().for_each(|point| {
            let const_point = point.clone();
            quadtree.query_radius(const_point.position.x, const_point.position.y, point.radius + max_radius)
                .into_iter()
                .for_each(|p| {
                    let axis = const_point.position - p.position;
                    let dist = axis.x * axis.x + axis.y * axis.y;
                    if p.id != const_point.id && dist <= (point.radius + p.radius) * (point.radius + p.radius) {
                        let delta = point.radius + p.radius - dist.sqrt();
                        let norm = axis.normalize() * delta * 0.5;
                        point.position += norm;
                    }
                });
            }); 
    }

    fn resolve_wall_collisions(&mut self, gravity: f32, app: &App) {
        let left = app.window_rect().left();
        let right = app.window_rect().right();
        let bottom = app.window_rect().bottom();
        self.points.iter_mut().for_each(|point| {
            point.acceleration = Vec2::new(0.0, gravity);
        
            if point.position.y - point.radius < bottom {
                let y_diff = point.position.y - point.prev_position.y;
                point.acceleration.y = 0.0;
                point.position.y = bottom + point.radius;
                point.prev_position.y = point.position.y + y_diff * 0.5;
            }
            if point.position.x - point.radius < left || point.position.x + point.radius > right {
                let x_diff = point.position.x - point.prev_position.x;
                if point.position.x - point.radius < left {
                    point.position.x = left + point.radius;
                } else {
                    point.position.x = right - point.radius;
                }
                point.prev_position.x = point.position.x + x_diff * 0.5;
            }
        });
    }
}
