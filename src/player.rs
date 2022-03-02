use std::time::{Duration, Instant};

use ggez::*;
use glam::{ivec2, vec2, IVec2, Vec2};

use crate::{Level, LevelTile};

pub struct PlayerProperties {
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
    pub jump_buffer_time: Duration,
}

impl Default for PlayerProperties {
    fn default() -> Self {
        Self {
            max_run_speed: 7.,
            ground_acceleration: 85.,
            ground_decceleration: 40.,
            ground_direction_change_acceleration: 85. + 40.,
            air_acceleration: 50.,
            air_decceleration: 20.,
            air_direction_change_acceleration: 50. + 20.,
            gravity: 100.,
            terminal_speed: 45.,
            jump_force: 23.,
            jump_gravity: 57.,
            coyote_time: Duration::from_millis(83),
            jump_buffer_time: Duration::from_millis(25),
        }
    }
}

pub struct Player {
    position: Vec2,
    velocity: Vec2,
    pub properties: PlayerProperties,
    image: graphics::Image,

    grounded: bool,
    hugging_wall: bool,
    pressed_jump: bool,
    jump_pressed_time: Instant,
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
            pressed_jump: false,
            jump_pressed_time: Instant::now(),
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
        if Instant::now() > self.jump_pressed_time + self.properties.jump_buffer_time {
            self.pressed_jump = false;
        }
        let pressing_jump = input::keyboard::is_key_pressed(ctx, input::keyboard::KeyCode::Space);
        if pressing_jump {
            self.pressed_jump = true;
            self.jump_pressed_time = Instant::now();
        }

        let delta = timer::delta(ctx).as_secs_f32();

        // Apply gravity
        self.velocity.y += if pressing_jump {
            self.properties.jump_gravity
        } else {
            self.properties.gravity
        } * delta;
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

        if self.grounded && self.pressed_jump {
            self.pressed_jump = false;
            self.velocity.y = -self.properties.jump_force;
        }

        self.reset_frame_state();

        self.try_move(self.velocity * delta, level);
    }

    pub fn collision_rect(&self) -> graphics::Rect {
        graphics::Rect {
            x: self.position.x,
            y: self.position.y,
            w: 1.,
            h: 1.,
        }
    }

    pub fn draw(&self, ctx: &mut Context, draw_param: graphics::DrawParam) -> GameResult {
        graphics::draw(
            ctx,
            &self.image,
            draw_param
                .dest(self.position * 18.)
                .color(if self.grounded {
                    graphics::Color::RED
                } else {
                    graphics::Color::WHITE
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
}
