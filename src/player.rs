use std::time::{Duration, Instant};

use ggez::*;
use ggez_egui::egui;
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
    pub jumps_available: u32,
    pub multijump_coefficient: f32,
    pub wallslide_max_v_speed: Option<f32>,
    pub can_walljump: bool,
    pub walljump_vertical_force: f32,
    pub walljump_horizontal_force: f32,
    pub dead_time_after_walljump: Duration,
}

impl Default for PlayerProperties {
    fn default() -> Self {
        Self {
            max_run_speed: 12.,
            ground_acceleration: 85.,
            ground_decceleration: 65.,
            ground_direction_change_acceleration: 85. + 40.,
            air_acceleration: 50.,
            air_decceleration: 20.,
            air_direction_change_acceleration: 100.,
            gravity: 100.,
            terminal_speed: 45.,
            jump_force: 22.,
            jump_gravity: 57.,
            coyote_time: Duration::from_millis(100),
            jump_buffer_time: Duration::from_millis(150),
            jumps_available: 2,
            multijump_coefficient: 0.8,
            wallslide_max_v_speed: Some(15.),
            can_walljump: true,
            walljump_vertical_force: 22.,
            walljump_horizontal_force: 10.,
            dead_time_after_walljump: Duration::from_millis(200),
        }
    }
}

impl PlayerProperties {
    pub fn show_ui(&mut self, egui_ctx: &egui::CtxRef) {
        egui::Window::new("Property editor")
            .anchor(egui::Align2::RIGHT_BOTTOM, (0., 0.))
            .show(egui_ctx, |ui| {
                ui.add(
                    egui::Slider::new(&mut self.max_run_speed, 0f32..=100.).text("Max run speed"),
                );
                egui::CollapsingHeader::new("Grounded properties")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.add(
                            egui::Slider::new(&mut self.ground_decceleration, 0f32..=100.)
                                .text("Ground decceleration"),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.ground_acceleration, 0f32..=100.)
                                .text("Ground acceleration"),
                        );
                        ui.add(
                            egui::Slider::new(
                                &mut self.ground_direction_change_acceleration,
                                0f32..=200.,
                            )
                            .text("Ground direction change acceleration"),
                        );
                    });
                egui::CollapsingHeader::new("Airborne properties")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.add(
                            egui::Slider::new(&mut self.air_acceleration, 0f32..=100.)
                                .text("Air acceleration"),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.air_decceleration, 0f32..=100.)
                                .text("Air decceleration"),
                        );
                        ui.add(
                            egui::Slider::new(
                                &mut self.air_direction_change_acceleration,
                                0f32..=200.,
                            )
                            .text("Air direction change acceleration"),
                        );
                        ui.add(egui::Slider::new(&mut self.gravity, 0f32..=200.).text("Gravity"));

                        ui.add(
                            egui::Slider::new(&mut self.terminal_speed, 0f32..=100.)
                                .text("Terminal speed"),
                        );
                    });
                egui::CollapsingHeader::new("Jump properties")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.add(
                            egui::Slider::new(&mut self.jump_force, 1f32..=100.).text("Jump Force"),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.jump_gravity, 0f32..=100.)
                                .text("Jump Gravity"),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.jumps_available, 0..=3)
                                .text("Jumps Available"),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.multijump_coefficient, 0f32..=1.)
                                .text("Multijump coefficient"),
                        );
                        let mut allow_wallsliding = self.wallslide_max_v_speed.is_some();
                        ui.add(egui::Checkbox::new(
                            &mut allow_wallsliding,
                            "Allow wallsliding",
                        ));
                        if !allow_wallsliding {
                            self.wallslide_max_v_speed = None;
                        } else {
                            let mut wallslide_max_v_speed =
                                self.wallslide_max_v_speed.unwrap_or(15.);
                            ui.add(
                                egui::Slider::new(&mut wallslide_max_v_speed, 0f32..=100.)
                                    .text("Wallslide max vertical speed"),
                            );
                            self.wallslide_max_v_speed = Some(wallslide_max_v_speed);
                        }
                        ui.add(egui::Checkbox::new(
                            &mut self.can_walljump,
                            "Allow walljumps",
                        ));
                        if self.can_walljump {
                            ui.add(
                                egui::Slider::new(&mut self.walljump_horizontal_force, 0f32..=100.)
                                    .text("Walljump horizontal force"),
                            );
                            ui.add(
                                egui::Slider::new(&mut self.walljump_vertical_force, 0f32..=100.)
                                    .text("Walljump vertical force"),
                            );
                        }
                    });
            });
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WallslideState {
    /// Collided with a wall while moving on the -X direction, now the player is sliding down it.
    HuggingLeftWall,
    /// Collided with a wall while moving on the +X direction, now the player is sliding down it.
    HuggingRightWall,
    /// Not collided with any walls horizontally while moving.
    HuggingNoWall,
}

pub struct Player {
    position: Vec2,
    velocity: Vec2,
    pub properties: PlayerProperties,
    image: graphics::Image,

    grounded: bool,
    wallslide_state: WallslideState,
    pressed_jump: bool,
    can_jump: bool,
    jump_pressed_time: Instant,
    last_grounded_time: Instant,
    last_walljump_time: Instant,
    times_jumped_since_grounded: u32,
}

impl Player {
    pub fn new(ctx: &mut Context, position: Vec2) -> GameResult<Self> {
        Ok(Self {
            position,
            velocity: vec2(0., 0.),
            properties: Default::default(),
            image: graphics::Image::solid(ctx, 1, graphics::Color::WHITE)?,
            grounded: false,
            wallslide_state: WallslideState::HuggingNoWall,
            pressed_jump: false,
            jump_pressed_time: Instant::now(),
            last_grounded_time: Instant::now(),
            last_walljump_time: Instant::now(),
            can_jump: false,
            times_jumped_since_grounded: 0,
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

        let delta = timer::delta(ctx).as_secs_f32();

        // Apply gravity
        self.velocity.y += if pressing_jump && self.velocity.y < 0. {
            self.properties.jump_gravity
        } else {
            self.properties.gravity
        } * delta;
        if f32::abs(self.velocity.y) > self.properties.terminal_speed {
            self.velocity.y = self.properties.terminal_speed * self.velocity.y.signum();
        }

        if self.wallslide_state != WallslideState::HuggingNoWall {
            if let Some(wallslide_max_v_speed) = self.properties.wallslide_max_v_speed {
                self.velocity.y = self.velocity.y.clamp(-f32::INFINITY, wallslide_max_v_speed);
            }
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
        } else if Instant::now()
            > self.last_walljump_time + self.properties.dead_time_after_walljump
        {
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

        if self.grounded {
            if self.properties.jumps_available > 0 {
                self.can_jump = true;
            }
            self.times_jumped_since_grounded = 0;
            self.last_grounded_time = Instant::now();
        } else if self.times_jumped_since_grounded == 0
            && Instant::now() > self.last_grounded_time + self.properties.coyote_time
        {
            // If didn't jump after coyote time is over, mark it as one jump done
            if self.properties.jumps_available == 1 {
                self.can_jump = false;
            } else {
                self.times_jumped_since_grounded += 1;
            }
        }

        if self.pressed_jump {
            if self.properties.can_walljump
                && self.wallslide_state != WallslideState::HuggingNoWall
                && !self.grounded
            {
                self.last_walljump_time = Instant::now();

                self.velocity.y = -self.properties.walljump_vertical_force;
                self.velocity.x = match self.wallslide_state {
                    WallslideState::HuggingLeftWall => self.properties.walljump_horizontal_force,
                    WallslideState::HuggingRightWall => -self.properties.walljump_horizontal_force,
                    WallslideState::HuggingNoWall => unreachable!(),
                }
            } else if self.can_jump {
                self.pressed_jump = false;

                self.velocity.y = -self.properties.jump_force
                    * self
                        .properties
                        .multijump_coefficient
                        .powi(self.times_jumped_since_grounded as i32);

                self.times_jumped_since_grounded += 1;
                if self.properties.jumps_available <= self.times_jumped_since_grounded {
                    self.can_jump = false;
                }
            }
        }

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

    pub fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        keycode: event::KeyCode,
        _keymods: event::KeyMods,
        repeat: bool,
    ) {
        if keycode == event::KeyCode::Space && !repeat {
            self.pressed_jump = true;
            self.jump_pressed_time = Instant::now();
        }
    }

    pub fn teleport_to(&mut self, pos: Vec2) {
        self.position = pos;
        self.velocity = vec2(0., 0.);
    }

    fn try_move(&mut self, mut to_move: Vec2, level: &Level) {
        self.grounded = false;

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
                    self.wallslide_state = if self.velocity.x > 0. {
                        WallslideState::HuggingRightWall
                    } else {
                        WallslideState::HuggingLeftWall
                    };
                    self.velocity.x = 0.;
                    to_move.x = 0.;
                } else {
                    self.position.x += step.x;
                    self.position.y = last_position.y;
                    if !self.is_colliding(level) {
                        // Not colliding when moved back on the Y axis, the player was blocked by
                        // the ground/ceiling
                        if self.velocity.y > 0. {
                            self.grounded = true;
                        }
                        self.velocity.y = 0.;
                        to_move.y = 0.;
                    } else {
                        // Colliding in both axes; Stop all movement
                        self.position = last_position;
                        if self.velocity.y > 0. {
                            self.grounded = true;
                        }
                        self.wallslide_state = if self.velocity.x > 0. {
                            WallslideState::HuggingRightWall
                        } else {
                            WallslideState::HuggingLeftWall
                        };
                        self.velocity = Vec2::ZERO;
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

        match (self.wallslide_state, self.velocity.x) {
            (WallslideState::HuggingLeftWall, vx) if vx > 0. => {
                self.wallslide_state = WallslideState::HuggingNoWall
            }
            (WallslideState::HuggingRightWall, vx) if vx < 0. => {
                self.wallslide_state = WallslideState::HuggingNoWall
            }
            _ => (),
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
