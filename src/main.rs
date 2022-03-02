use std::path::PathBuf;
use std::time::Duration;

use ggez::event::{self, MouseButton};
use ggez::graphics::{self, Color};
use ggez::input;
use ggez::timer;
use ggez::{Context, GameResult};
use glam::*;

use ggez_egui::*;

struct PlayerProperties {
    pub max_run_speed: f32,
    pub terminal_speed: f32,
    pub ground_acceleration: f32,
    pub ground_decceleration: f32,
    pub ground_direction_change_acceleration: f32,
    pub air_acceleration: f32,
    pub air_decceleration: f32,
    pub air_direction_change_acceleration: f32,
    pub gravity: f32,
    pub jump_force: f32,
    pub jump_gravity: f32,
    pub coyote_time: Duration,
}

impl Default for PlayerProperties {
    fn default() -> Self {
        Self {
            max_run_speed: 7.,
            terminal_speed: 17.,
            ground_acceleration: 85.,
            ground_decceleration: 40.,
            ground_direction_change_acceleration: 85. + 40.,
            air_acceleration: 50.,
            air_decceleration: 20.,
            air_direction_change_acceleration: 50. + 20.,
            gravity: 70.,
            jump_force: 5.,
            jump_gravity: 2.,
            coyote_time: Duration::from_millis(83),
        }
    }
}

struct Player {
    position: Vec2,
    velocity: Vec2,
    pub properties: PlayerProperties,
    image: graphics::Image,

    grounded: bool,
    hugging_wall: bool,
}

impl Player {
    pub fn new(ctx: &mut Context, position: Vec2) -> GameResult<Self> {
        Ok(Self {
            position,
            velocity: vec2(0., 0.),
            properties: Default::default(),
            image: graphics::Image::solid(ctx, 1, graphics::Color::WHITE)?,
            grounded: false,
            hugging_wall: false,
        })
    }

    pub fn update(&mut self, ctx: &Context, level: &Level) {
        let x_input: f32 = if input::keyboard::is_key_pressed(ctx, input::keyboard::KeyCode::A) {
            -1.
        } else if input::keyboard::is_key_pressed(ctx, input::keyboard::KeyCode::D) {
            1.
        } else {
            0.
        };

        let delta = timer::delta(ctx).as_secs_f32();

        // Apply gravity
        self.velocity.y += self.properties.gravity * delta;
        if f32::abs(self.velocity.y) > self.properties.terminal_speed {
            self.velocity.y = self.properties.terminal_speed * self.velocity.y.signum();
        }

        if x_input == 0. {
            // Apply decceleration
            let decceleration = if self.grounded {
                self.properties.ground_decceleration
            } else {
                self.properties.air_decceleration
            } * delta;

            if f32::abs(self.velocity.x) > decceleration {
                self.velocity.x += if self.velocity.x > 0. {
                    -decceleration
                } else {
                    decceleration
                };
            } else {
                self.velocity.x = 0.;
            }
        } else {
            // Apply acceleration
            let acceleration = if x_input.signum() != self.velocity.x.signum() {
                if self.grounded {
                    self.properties.ground_direction_change_acceleration
                } else {
                    self.properties.air_direction_change_acceleration
                }
            } else {
                if self.grounded {
                    self.properties.ground_acceleration
                } else {
                    self.properties.air_acceleration
                }
            } * delta;

            self.velocity.x += x_input * acceleration;

            if f32::abs(self.velocity.x) > self.properties.max_run_speed {
                self.velocity.x = self.properties.max_run_speed * self.velocity.x.signum();
            }
        }

        self.reset_frame_state();

        self.try_move(self.velocity * delta, level);
    }

    fn reset_frame_state(&mut self) {
        self.grounded = false;
        self.hugging_wall = false;
    }

    fn try_move(&mut self, mut to_move: Vec2, level: &Level) {
        if to_move.x == 0. && to_move.y == 0. {
            return;
        }

        fn calculate_delta_step(delta: Vec2) -> Vec2 {
            const MAX_STEP_LENGTH: f32 = 0.1;
            let delta_len = delta.length();
            if delta_len <= MAX_STEP_LENGTH {
                delta
            } else {
                delta / delta.length() * MAX_STEP_LENGTH
            }
        }

        let mut step = calculate_delta_step(to_move);

        while to_move.length() >= step.length() {
            let last_position = self.position;
            self.position += step;
            to_move -= step;

            if self.is_colliding(level) {
                // Move one axis at a time to figure out where/how the collision happened
                self.position.x = last_position.x;
                if !self.is_colliding(level) {
                    // Not colliding when moved back on the X axis, the player was blocked by a wall
                    self.velocity.x = 0.;
                    self.hugging_wall = true;
                    to_move.x = 0.;
                } else {
                    self.position.x += step.x;
                    self.position.y = last_position.y;
                    if !self.is_colliding(level) {
                        // Not colliding when moved back on the Y axis, the player was blocked by
                        // the ground/ceiling
                        self.velocity.y = 0.;
                        self.grounded = true;
                        to_move.y = 0.;
                    } else {
                        // Colliding in both axes; Stop all movement
                        self.position = last_position;
                        self.grounded = true;
                        self.hugging_wall = true;
                        return;
                    }
                }

                if to_move == Vec2::ZERO {
                    break;
                } else {
                    step = calculate_delta_step(to_move);
                }
            }
        }
    }

    fn is_colliding(&self, level: &Level) -> bool {
        let col = self.collision_rect();
        fn floor(point: Vec2) -> IVec2 {
            ivec2(point.x.floor() as i32, point.y.floor() as i32)
        }
        let tiles_to_check = [
            floor(col.point().into()),
            floor(vec2(col.x + col.w, col.y)),
            floor(vec2(col.x, col.y + col.h)),
            floor(vec2(col.x + col.w, col.y + col.h)),
        ];

        tiles_to_check
            .into_iter()
            .any(|pos| matches!(level.get_tile(pos.x, pos.y), Some(LevelTile::Solid)))
    }

    pub fn collision_rect(&self) -> graphics::Rect {
        graphics::Rect {
            x: self.position.x,
            y: self.position.y,
            w: 1.,
            h: 1.,
        }
    }

    pub fn draw(
        &self,
        ctx: &mut Context,
        draw_param: graphics::DrawParam,
        level: &Level,
    ) -> GameResult {
        graphics::draw(
            ctx,
            &self.image,
            draw_param
                .dest(self.position * 18.)
                .color(if self.grounded {
                    Color::RED
                } else {
                    Color::WHITE
                }),
        )?;
        graphics::draw(
            ctx,
            &graphics::Text::new(format!("pos: {}, vel: {}", self.position, self.velocity)),
            graphics::DrawParam::default(),
        )?;
        Ok(())
    }

    pub fn teleport_to(&mut self, pos: Vec2) {
        self.position = pos;
        self.velocity = vec2(0., 0.);
    }
}

enum LevelTile {
    Solid,
}

struct Level {
    tiles: Vec<Option<LevelTile>>,
    width: u32,
    height: u32,
    tile_batch: graphics::spritebatch::SpriteBatch,
    pub spawn_point: Vec2,
}

impl Level {
    pub fn new(map: tiled::Map, ctx: &mut Context) -> GameResult<Self> {
        let mut image_path = PathBuf::from("/");
        image_path.push(&map.tilesets()[0].image.as_ref().unwrap().source);
        let image = graphics::Image::new(ctx, image_path)?;
        let tile_batch = Self::generate_tile_batch(image, &map)?;
        let tiles = if let tiled::LayerType::TileLayer(tiled::TileLayer::Finite(layer)) =
            map.get_layer(0).unwrap().layer_type()
        {
            (0..layer.height() as i32)
                .flat_map(|y| (0..layer.width() as i32).map(move |x| (x, y)))
                .map(|(x, y)| layer.get_tile(x, y).map(|_| LevelTile::Solid))
                .collect()
        } else {
            panic!()
        };

        let spawn_point = map
            .layers()
            .find_map(|layer| match layer.layer_type() {
                tiled::LayerType::ObjectLayer(layer) => Some(layer),
                _ => None,
            })
            .and_then(|layer| layer.objects().nth(0))
            .map(|obj| {
                vec2(
                    obj.x() / map.tile_width as f32,
                    obj.y() / map.tile_height as f32,
                )
            })
            .unwrap();

        Ok(Level {
            tile_batch,
            tiles,
            width: map.width,
            height: map.height,
            spawn_point,
        })
    }

    pub fn get_tile(&self, x: i32, y: i32) -> Option<&LevelTile> {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            self.tiles[(x + y * self.width as i32) as usize].as_ref()
        } else {
            None
        }
    }

    fn generate_tile_batch(
        image: graphics::Image,
        map: &tiled::Map,
    ) -> GameResult<graphics::spritebatch::SpriteBatch> {
        let (width, height) = (image.width(), image.height());
        let mut batch = graphics::spritebatch::SpriteBatch::new(image);
        for layer in map.layers() {
            if let tiled::LayerType::TileLayer(tiled::TileLayer::Finite(layer)) = layer.layer_type()
            {
                for x in 0..layer.width() as i32 {
                    for y in 0..layer.height() as i32 {
                        if let Some(tile) = layer.get_tile(x, y) {
                            batch.add(
                                graphics::DrawParam::default()
                                    .src(get_tile_rect(
                                        map.tilesets()[0].as_ref(),
                                        tile.id(),
                                        width,
                                        height,
                                    ))
                                    .dest(vec2(x as f32, y as f32))
                                    .scale(vec2(1. / 18., 1. / 18.)),
                            );
                        }
                    }
                }
            }
        }

        Ok(batch)
    }

    pub fn draw(&self, ctx: &mut Context, draw_param: graphics::DrawParam) -> GameResult {
        graphics::draw(ctx, &self.tile_batch, draw_param)?;
        Ok(())
    }
}

fn get_tile_rect(
    tileset: &tiled::Tileset,
    id: u32,
    ts_img_width: u16,
    ts_img_height: u16,
) -> graphics::Rect {
    let ts_x = id % tileset.columns;
    let ts_y = id / tileset.columns;

    let x = (tileset.margin + (tileset.tile_width + tileset.spacing) * ts_x) as f32;
    let y = (tileset.margin + (tileset.tile_height + tileset.spacing) * ts_y) as f32;

    let ts_img_width = ts_img_width as f32;
    let ts_img_height = ts_img_height as f32;

    graphics::Rect {
        x: x / ts_img_width,
        y: y / ts_img_height,
        w: tileset.tile_width as f32 / ts_img_width,
        h: tileset.tile_height as f32 / ts_img_height,
    }
}

struct MainState {
    level: Level,
    player: Player,
    egui_backend: EguiBackend,
}

impl MainState {
    fn new(ctx: &mut Context) -> GameResult<MainState> {
        let level = Level::new(
            tiled::Map::parse_file("assets/map.tmx", &mut tiled::FilesystemResourceCache::new())
                .unwrap(),
            ctx,
        )?;
        let s = MainState {
            player: Player::new(ctx, level.spawn_point)?,
            level,
            egui_backend: EguiBackend::default(),
        };
        Ok(s)
    }
}

impl event::EventHandler<ggez::GameError> for MainState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        self.player.update(ctx, &self.level);

        let egui_ctx = self.egui_backend.ctx();

        egui::Window::new("Property editor").show(&egui_ctx, |ui| {
            let prop = &mut self.player.properties;
            ui.add(egui::Slider::new(&mut prop.max_run_speed, 0f32..=100.).text("Max run speed"));
            ui.collapsing("Grounded properties", |ui| {
                ui.add(
                    egui::Slider::new(&mut prop.ground_decceleration, 0f32..=100.)
                        .text("Ground decceleration"),
                );
                ui.add(
                    egui::Slider::new(&mut prop.ground_acceleration, 0f32..=100.)
                        .text("Ground acceleration"),
                );
            });
            ui.collapsing("Airborne properties", |ui| {
                ui.add(
                    egui::Slider::new(&mut prop.air_acceleration, 0f32..=100.)
                        .text("Air acceleration"),
                );
                ui.add(
                    egui::Slider::new(&mut prop.air_decceleration, 0f32..=100.)
                        .text("Air decceleration"),
                );
                ui.add(
                    egui::Slider::new(&mut prop.air_direction_change_acceleration, 0f32..=100.)
                        .text("Air direction change acceleration"),
                );
                ui.add(egui::Slider::new(&mut prop.gravity, 0f32..=100.).text("Gravity"));
                ui.add(
                    egui::Slider::new(&mut prop.terminal_speed, 0f32..=100.).text("Terminal speed"),
                );
            });
        });

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        graphics::clear(ctx, [0.1, 0.2, 0.3, 1.0].into());

        self.level
            .draw(ctx, graphics::DrawParam::default().scale([18., 18.]))?;
        self.player.draw(
            ctx,
            graphics::DrawParam::default().scale([18., 18.]),
            &self.level,
        )?;

        graphics::draw(ctx, &self.egui_backend, ([0.0, 0.0],))?;

        graphics::present(ctx)?;
        Ok(())
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        button: MouseButton,
        _x: f32,
        _y: f32,
    ) {
        self.egui_backend.input.mouse_button_down_event(button);
    }

    fn mouse_button_up_event(&mut self, _ctx: &mut Context, button: MouseButton, _x: f32, _y: f32) {
        self.egui_backend.input.mouse_button_up_event(button);
    }

    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32, _dx: f32, _dy: f32) {
        self.egui_backend.input.mouse_motion_event(x, y);
    }

    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        keycode: event::KeyCode,
        _keymods: event::KeyMods,
        _repeat: bool,
    ) {
        if keycode == event::KeyCode::R {
            self.player.teleport_to(self.level.spawn_point);
        }
    }
}

pub fn main() -> GameResult {
    let cb = ggez::ContextBuilder::new("super_simple", "aleok")
        .add_resource_path(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let (mut ctx, event_loop) = cb.build()?;
    let state = MainState::new(&mut ctx)?;
    event::run(ctx, event_loop, state)
}
