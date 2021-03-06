use std::time::Duration;

use crate::{
    debug::DebugMode,
    follow::CameraFollow,
    input_mapper::{self, Input},
    physics::{
        CollisionSide, KinematicBody, KinematicCollisions, RectCollision, RectExtras, SensedBodies,
        SensorBody, Velocity,
    },
    time::GameplayTime,
    world::{GameWorld, TILE_SIZE},
    AppState, LdtkProject,
};
use bevy::math::vec2;
use bevy::{prelude::*, sprite::Rect};
use bevy_egui::egui;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            SystemSet::on_update(AppState::Playing)
                .with_system(set_player_state.before(update_player))
                .with_system(update_jump.before(update_player))
                .with_system(update_player)
                .with_system(update_camera_bounds),
        )
        .add_system(update_current_room)
        .add_system(update_room_pos)
        .add_system(debug_player_state);
    }
}

#[derive(Debug)]
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
    pub fn show_ui(&mut self, egui_ctx: &egui::Context) {
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SlideSide {
    /// Sliding against a wall which is to the right of the player.
    Right,
    /// Sliding against a wall which is to the left of the player.
    Left,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

#[derive(Component, Default, PartialEq, Eq, Clone, Debug)]
pub struct RoomPos {
    pub x: i64,
    pub y: i64,
}

#[derive(Component)]
pub struct CurrentRoom {
    id: String,
}

#[derive(Component, Default, Debug)]
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
    room_pos: RoomPos,
    collision: RectCollision,
}

impl Default for PlayerBundle {
    fn default() -> Self {
        Self {
            sprite: SpriteBundle {
                transform: Transform::from_xyz(0., 0., 10.),
                ..default()
            },
            velocity: default(),
            body: default(),
            player: default(),
            collision: RectCollision {
                rect: Rect::from_min_size(vec2(0., 0.), vec2(1., 1.)),
            },
            room_pos: default(),
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
            global_transform: default(),
            transform: default(),
            collision: RectCollision {
                rect: Rect::from_min_size(vec2(0., 0.), vec2(1., 1.)),
            },
            body: default(),
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

pub fn spawn_player(
    mut commands: Commands,
    world: Res<GameWorld>,
    player_query: Query<(), With<Player>>,
    ldtk_maps: Res<Assets<LdtkProject>>,
) {
    if !player_query.is_empty() {
        return;
    }

    let map = ldtk_maps
        .get(&world.ldtk)
        .expect("Player was spawned before project was loaded in");

    let start_point_def = map
        .project
        .defs
        .entities
        .iter()
        .find(|def| def.identifier == "Start_Point")
        .expect("Could not find Start_Point entity definition");

    let (start_level, start_point) = map
        .project
        .levels
        .iter()
        .find_map(|level| {
            level
                .layer_instances
                .iter()
                .flatten()
                .flat_map(|layer| layer.entity_instances.iter())
                .find(|&entity| entity.def_uid == start_point_def.uid)
                .map(|entity| (level, entity))
        })
        .expect("Could not find world start point");

    let px = (start_point.px[0] + start_level.world_x) as f32 / 16.;
    let py = -(start_point.px[1] + start_level.world_y) as f32 / 16.;

    let transform = Transform::from_xyz(px, py, 10.);

    info!("Spawning player @ {:?}", transform.translation);

    let mut left_id = None;
    let mut right_id = None;
    commands
        .spawn_bundle(PlayerBundle {
            sprite: SpriteBundle {
                transform,
                ..default()
            },
            ..default()
        })
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
            left_side_sensor: left_id,
            right_side_sensor: right_id,
            ..default()
        });
}

fn update_camera_bounds(
    world: Res<GameWorld>,
    maps: Res<Assets<LdtkProject>>,
    current_room: Query<&CurrentRoom, With<Player>>,
    mut camera: Query<&mut CameraFollow, With<Camera>>,
) {
    let map = if let Some(map) = maps.get(&world.ldtk) {
        map
    } else {
        return;
    };

    const TILE_SIZE: f32 = crate::world::TILE_SIZE as f32;

    if let Ok(current_room) = current_room.get_single() {
        let level = map
            .project
            .levels
            .iter()
            .find(|level| level.identifier == current_room.id)
            .unwrap();

        if let Ok(mut follow) = camera.get_single_mut() {
            follow.bounds = Rect::from_min_size(
                vec2(
                    level.world_x as f32 / TILE_SIZE,
                    -(level.world_y + level.px_hei) as f32 / TILE_SIZE,
                ),
                vec2(
                    level.px_wid as f32 / TILE_SIZE,
                    level.px_hei as f32 / TILE_SIZE,
                ),
            );
        }
    }
}

fn update_current_room(
    mut commands: Commands,
    world: Res<GameWorld>,
    maps: Res<Assets<LdtkProject>>,
    mut query: Query<(Entity, &GlobalTransform, Option<&CurrentRoom>), Changed<RoomPos>>,
) {
    let map = if let Some(map) = maps.get(&world.ldtk) {
        map
    } else {
        return;
    };

    const TILE_SIZE: f32 = crate::world::TILE_SIZE as f32;

    for (entity, transform, current_room) in query.iter_mut() {
        for level in map.project.levels.iter().filter(|level| {
            current_room
                .map(|room| level.identifier != room.id)
                .unwrap_or(true)
        }) {
            let level_rect = Rect::from_min_size(
                vec2(
                    level.world_x as f32 / TILE_SIZE - 0.5,
                    -(level.world_y + level.px_hei) as f32 / TILE_SIZE - 0.5,
                ),
                vec2(
                    level.px_wid as f32 / TILE_SIZE + 1.,
                    level.px_hei as f32 / TILE_SIZE + 1.,
                ),
            );
            if level_rect.contains(transform.translation.truncate()) {
                commands.entity(entity).insert(CurrentRoom {
                    id: level.identifier.clone(),
                });
                break;
            }
        }
    }
}

fn update_room_pos(
    world: Res<GameWorld>,
    maps: Res<Assets<LdtkProject>>,
    mut query: Query<(&mut RoomPos, &GlobalTransform), Changed<GlobalTransform>>,
) {
    let map = if let Some(map) = maps.get(&world.ldtk) {
        map
    } else {
        return;
    };

    for (mut room_pos, transform) in query.iter_mut() {
        let get_pos = |point: Vec2| -> RoomPos {
            RoomPos {
                x: ((point.x - 0.5).floor() * TILE_SIZE as f32
                    / map.project.world_grid_width.unwrap() as f32)
                    .floor() as i64,
                y: (point.y.round() * TILE_SIZE as f32
                    / map.project.world_grid_height.unwrap() as f32)
                    .floor() as i64,
            }
        };

        let according_pos = get_pos(transform.translation.truncate());
        if according_pos != *room_pos {
            *room_pos = according_pos;
        }
    }
}

fn update_jump(
    gameplay_time: Res<GameplayTime>,
    input: Res<Input>,
    mut player: Query<&mut Player>,
) {
    let mut player = if let Ok(player) = player.get_single_mut() {
        player
    } else {
        return;
    };

    if !input.actions[input_mapper::Action::Down].is_pressed()
        && input.actions[input_mapper::Action::Jump] == input_mapper::ActionState::JustPressed
    {
        player.pressed_jump = true;
        player.jump_pressed_time = gameplay_time.elapsed();
    }
}

fn update_player(
    time: Res<Time>,
    gameplay_time: Res<GameplayTime>,
    input: Res<Input>,
    mut player: Query<(&mut Velocity, &mut Player, &mut KinematicBody)>,
) {
    let (mut velocity, mut player, mut body) = if let Ok(player) = player.get_single_mut() {
        player
    } else {
        return;
    };
    let delta = time.delta_seconds();
    let unpaused_time = gameplay_time.elapsed();

    // Obtain frame & input data
    let x_input: f32 = input.axes[input_mapper::Axis::Horizontal].value();
    let pressing_down = input.actions[input_mapper::Action::Down].is_pressed();
    let pressing_jump = input.actions[input_mapper::Action::Jump].is_pressed();
    body.pass_through_platforms = pressing_down && pressing_jump;

    let pressing_jump = !pressing_down && pressing_jump;

    // Apply gravity
    velocity.y -= if pressing_jump && velocity.y > 0. {
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
            velocity.y = velocity
                .y
                .clamp(-wallslide_max_v_speed, wallslide_max_v_speed);
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
            player.last_grounded_time = time.time_since_startup();
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

                velocity.y = player.properties.walljump_vertical_force;
                velocity.x = match side {
                    SlideSide::Left => player.properties.walljump_horizontal_force,
                    SlideSide::Right => -player.properties.walljump_horizontal_force,
                };
            }
            _ if player.can_jump => {
                player.pressed_jump = false;

                velocity.y = player.properties.jump_force
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

fn set_player_state(
    mut query: Query<(&mut Player, &KinematicCollisions)>,
    sensors: Query<&SensedBodies, With<SensorBody>>,
) {
    if let Ok((mut player, collisions)) = query.get_single_mut() {
        if collisions.sides.contains(CollisionSide::DOWN) {
            player.state = State::Grounded;
        } else if sensors.get(player.left_side_sensor.unwrap()).unwrap().world {
            player.state = State::Sliding {
                side: SlideSide::Left,
            };
        } else if sensors
            .get(player.right_side_sensor.unwrap())
            .unwrap()
            .world
        {
            player.state = State::Sliding {
                side: SlideSide::Right,
            };
        } else {
            player.state = State::Airborne;
        }
    }
}

// Debug systems

fn noclip_player_movement(
    time: Res<Time>,
    input: Res<Input>,
    mut player: Query<&mut Transform, With<Player>>,
) {
    let horizontal = input.axes[crate::input_mapper::Axis::Horizontal];

    if let Ok(mut transform) = player.get_single_mut() {
        transform.translation.x += horizontal.value() * 10. * time.delta_seconds();
    }
}

fn debug_player_state(
    debug: Res<DebugMode>,
    mut egui: ResMut<bevy_egui::EguiContext>,
    query: Query<(&Player, &Velocity)>,
) {
    if !debug.active {
        return;
    }

    if let Ok((player, velocity)) = query.get_single() {
        egui::Window::new("Player state [debug]").show(egui.ctx_mut(), |ui| {
            ui.label(format!("Component: {:#?}", player));
            ui.label(format!("Velocity: {:?}", **velocity));
        });
    }
}
