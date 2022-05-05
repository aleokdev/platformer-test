use std::time::Duration;

use crate::{
    input_binding::{self, InputBinder},
    physics::{
        KinematicBody, KinematicCollisions, RectCollision, RectExtras, SensorBody, Velocity,
    },
    World,
};
use bevy::{core::Stopwatch, prelude::*, sprite::Rect};
use ggez_egui::egui;
use glam::{ivec2, vec2, vec3, IVec2, Vec2};

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
            walljump_vertical_force: 20.,
            walljump_horizontal_force: 10.,
            dead_time_after_walljump: Duration::from_millis(200),
        }
    }
}

impl PlayerProperties {
    pub fn show_ui(&mut self, egui_ctx: &egui::CtxRef) {
        egui::CentralPanel::default().show(egui_ctx, |ui| {
            ui.add(egui::Slider::new(&mut self.max_run_speed, 0f32..=100.).text("Max run speed"));
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
                        egui::Slider::new(&mut self.air_direction_change_acceleration, 0f32..=200.)
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
                    ui.add(egui::Slider::new(&mut self.jump_force, 1f32..=100.).text("Jump Force"));
                    ui.add(
                        egui::Slider::new(&mut self.jump_gravity, 0f32..=100.).text("Jump Gravity"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.jumps_available, 0..=3).text("Jumps Available"),
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
                        let mut wallslide_max_v_speed = self.wallslide_max_v_speed.unwrap_or(15.);
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SlideSide {
    /// Sliding against a wall which is to the right of the player.
    Right,
    /// Sliding against a wall which is to the left of the player.
    Left,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum State {
    Grounded,
    Airborne,
    Sliding { side: SlideSide },
}

impl Default for State {
    fn default() -> Self {
        State::Airborne
    }
}

#[derive(Component, Default)]
pub struct Player {
    state: State,
    properties: PlayerProperties,

    pressed_jump: bool,
    can_jump: bool,
    jump_pressed_time: Duration,
    last_grounded_time: Duration,
    last_walljump_time: Duration,
    times_jumped_since_grounded: u32,
    left_side_sensor: Option<Entity>,
    right_side_sensor: Option<Entity>,
}

#[derive(Bundle)]
pub struct PlayerBundle {
    #[bundle]
    sprite: SpriteBundle,
    velocity: Velocity,
    body: KinematicBody,
    player: Player,
    collision: RectCollision,
}

impl Default for PlayerBundle {
    fn default() -> Self {
        Self {
            sprite: Default::default(),
            velocity: Default::default(),
            body: Default::default(),
            player: Default::default(),
            collision: RectCollision {
                rect: Rect::from_min_size(vec2(0., 0.), vec2(1., 1.)),
            },
        }
    }
}

#[derive(Bundle)]
struct PlayerSideCollisionCheckerBundle {
    global_transform: GlobalTransform,
    transform: Transform,
    collision: RectCollision,
    body: SensorBody,
}

impl Default for PlayerSideCollisionCheckerBundle {
    fn default() -> Self {
        Self {
            global_transform: Default::default(),
            transform: Default::default(),
            collision: RectCollision {
                rect: Rect::from_min_size(vec2(0., 0.), vec2(1., 1.)),
            },
            body: Default::default(),
        }
    }
}

impl PlayerSideCollisionCheckerBundle {
    pub fn left() -> Self {
        Self {
            transform: Transform::from_xyz(-0.1, 0., 0.),
            ..default()
        }
    }
    pub fn right() -> Self {
        Self {
            transform: Transform::from_xyz(0.1, 0., 0.),
            ..default()
        }
    }
}

pub fn spawn_player(commands: &mut Commands) {
    let mut left_id = None;
    let mut right_id = None;
    commands
        .spawn_bundle(PlayerBundle::default())
        .with_children(|children| {
            left_id = Some(
                children
                    .spawn_bundle(PlayerSideCollisionCheckerBundle::left())
                    .id(),
            );
            right_id = Some(
                children
                    .spawn_bundle(PlayerSideCollisionCheckerBundle::right())
                    .id(),
            );
        })
        .insert(Player {
            left_side_sensor: Some(left_id.unwrap()),
            right_side_sensor: Some(right_id.unwrap()),
            ..default()
        });
}

#[derive(Deref)]
pub struct GameplayTime(pub Stopwatch);

pub fn update_player(
    game_time: Res<Time>,
    gameplay_time: Res<GameplayTime>,
    input: Res<InputBinder>,
    mut player: Query<(&mut Transform, &mut Velocity, &mut Player)>,
) {
    let (transform, mut velocity, mut player) = player.single_mut();
    let unpaused_time = gameplay_time.elapsed();
    // Obtain frame & input data
    let x_input: f32 = input.axis_value(input_binding::Axis::Horizontal).value();
    if input.action_value(input_binding::Action::Jump) == input_binding::ActionState::JustPressed {
        player.pressed_jump = true;
        player.jump_pressed_time = unpaused_time;
    }
    let pressing_jump = input.action_value(input_binding::Action::Jump).is_pressed();
    let delta = game_time.delta_seconds();

    // Apply gravity
    velocity.y += if pressing_jump && velocity.y < 0. {
        player.properties.jump_gravity
    } else {
        player.properties.gravity
    } * delta;
    if f32::abs(velocity.y) > player.properties.terminal_speed {
        velocity.y = player.properties.terminal_speed * velocity.y.signum();
    }

    // Clamp velocity if sliding down wall
    if matches!(player.state, State::Sliding { .. }) {
        if let Some(wallslide_max_v_speed) = player.properties.wallslide_max_v_speed {
            velocity.y = velocity.y.clamp(-f32::INFINITY, wallslide_max_v_speed);
        }
    }

    if x_input == 0. {
        // Apply horizontal decceleration
        let decceleration = match player.state {
            State::Grounded => player.properties.ground_decceleration,
            State::Airborne => player.properties.air_decceleration,
            State::Sliding { .. } => 0.,
        } * delta;

        if f32::abs(velocity.x) > decceleration {
            velocity.x += if velocity.x > 0. {
                -decceleration
            } else {
                decceleration
            };
        } else {
            velocity.x = 0.;
        }
    } else if unpaused_time > player.last_walljump_time + player.properties.dead_time_after_walljump
    {
        // Apply horizontal acceleration
        let acceleration = if x_input.signum() != velocity.x.signum() {
            match player.state {
                State::Grounded => player.properties.ground_direction_change_acceleration,
                State::Airborne | State::Sliding { .. } => {
                    player.properties.air_direction_change_acceleration
                }
            }
        } else {
            match player.state {
                State::Grounded => player.properties.ground_acceleration,
                State::Airborne | State::Sliding { .. } => player.properties.air_acceleration,
            }
        } * delta;

        velocity.x += x_input * acceleration;

        if f32::abs(velocity.x) > player.properties.max_run_speed {
            velocity.x = player.properties.max_run_speed * velocity.x.signum();
        }
    }

    match player.state {
        State::Grounded => {
            if player.properties.jumps_available > 0 {
                player.can_jump = true;
            }
            player.times_jumped_since_grounded = 0;
            player.last_grounded_time = game_time.time_since_startup();
        }
        State::Airborne
            if player.times_jumped_since_grounded == 0
                && unpaused_time > player.last_grounded_time + player.properties.coyote_time =>
        {
            // If didn't jump after coyote time is over, mark it as one jump done
            if player.properties.jumps_available == 1 {
                player.can_jump = false;
            } else {
                player.times_jumped_since_grounded += 1;
            }
        }
        _ => (),
    }

    // Handle jumping/walljumping
    if player.pressed_jump {
        match (player.properties.can_walljump, player.state) {
            (true, State::Sliding { side }) => {
                player.pressed_jump = false;
                player.last_walljump_time = unpaused_time;

                velocity.y = -player.properties.walljump_vertical_force;
                velocity.x = match side {
                    SlideSide::Left => player.properties.walljump_horizontal_force,
                    SlideSide::Right => -player.properties.walljump_horizontal_force,
                };
            }
            _ if player.can_jump => {
                player.pressed_jump = false;

                velocity.y = -player.properties.jump_force
                    * player
                        .properties
                        .multijump_coefficient
                        .powi(player.times_jumped_since_grounded as i32);

                player.times_jumped_since_grounded += 1;
                if player.properties.jumps_available <= player.times_jumped_since_grounded {
                    player.can_jump = false;
                }
            }
            _ => (),
        }

        // Reset jump buffer if appropiate
        if unpaused_time > player.jump_pressed_time + player.properties.jump_buffer_time {
            player.pressed_jump = false;
        }
    }
}

pub fn set_player_state(mut query: Query<(&mut Player, &KinematicCollisions)>) {
    if let Ok((mut player, collisions)) = query.get_single_mut() {}
}

fn floor(point: Vec2) -> IVec2 {
    ivec2(point.x.floor() as i32, point.y.floor() as i32)
}
