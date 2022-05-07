//! 2D Platformer physics.
//!
//! I first thought about using the `heron` crate, but decided against it because I needed finer
//! control over collision shapes (For tilemaps), collision sides (For player states) and didn't
//! require many of the features it offered, such as dynamic rigidbodies or rotation.

use bevy::sprite::Rect;
use bevy::{core::FixedTimestep, prelude::*};
use bevy_ecs_tilemap::{Tile, TilePos};
use bitflags::bitflags;
use glam::{ivec2, vec2};

use crate::Player;
use crate::{
    world::{GameWorld, LevelId, LevelTile},
    LdtkProject,
};

#[derive(Component, Deref, DerefMut, Default)]
pub struct Velocity(Vec2);

#[derive(Component, Default)]
pub struct RectCollision {
    pub rect: Rect,
}

pub trait RectExtras: Sized {
    /// left-top corner plus a size (stretching right-down).
    fn from_min_size(min: Vec2, size: Vec2) -> Self;

    #[must_use]
    fn translate(self, amnt: Vec2) -> Self;

    fn intersects(self, other: Rect) -> bool;

    #[must_use]
    fn contains(&self, p: Vec2) -> bool;
}

impl RectExtras for Rect {
    #[must_use]
    #[inline(always)]
    fn from_min_size(min: Vec2, size: Vec2) -> Self {
        Rect {
            min,
            max: min + size,
        }
    }

    #[must_use]
    #[inline]
    fn translate(self, amnt: Vec2) -> Self {
        Self::from_min_size(self.min + amnt, self.size())
    }

    #[must_use]
    #[inline]
    fn intersects(self, other: Rect) -> bool {
        self.min.x <= other.max.x
            && other.min.x <= self.max.x
            && self.min.y <= other.max.y
            && other.min.y <= self.max.y
    }

    #[must_use]
    #[inline(always)]
    fn contains(&self, p: Vec2) -> bool {
        self.min.x <= p.x && p.x <= self.max.x && self.min.y <= p.y && p.y <= self.max.y
    }
}

// TODO: LevelCollision
#[derive(Component, Default)]
pub struct LevelCollision;

#[derive(Component, Default)]
pub struct KinematicBody;

#[derive(Component, Default)]
pub struct StaticBody;
// TODO: Merge KinematicBody & StaticBody into a RigidBody enum

#[derive(Component, Default)]
pub struct SensorBody;

bitflags! {
    #[derive(Default)]
    pub struct CollisionSide: u8 {
        const UP = 0b0000_0001;
        const DOWN = 0b0000_0010;
        const LEFT = 0b0000_0100;
        const RIGHT = 0b0000_1000;
    }
}

#[derive(Component, Default)]
pub struct KinematicCollisions {
    pub sides: CollisionSide,
}

/// Lists the bodies being touched by this entity. Added to entities with a valid [`SensorBody`]
/// configuration.
#[derive(Component, Default)]
pub struct SensedBodies {
    others: Vec<Entity>,
}

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app /*.init_resource::<Gravity>()*/
            .init_resource::<PhysicsWorld>()
            .add_system_set_to_stage(
                CoreStage::PreUpdate,
                SystemSet::new()
                    .with_run_criteria(FixedTimestep::step(1. / 60.))
                    .with_system(update_physics_world)
                    .with_system(move_bodies.after(update_physics_world))
                    .with_system(detect_bodies.after(update_physics_world)), //.with_system(gravity),
            );
    }
}
/*
/// Resource indicating the gravity (velocity per frame) to apply to kinematic bodies
#[derive(Deref)]
pub struct Gravity(Vec2);

impl Default for Gravity {
    fn default() -> Self {
        Self(vec2(0., -100.))
    }
}

pub fn gravity(
    time: Res<Time>,
    gravity: Res<Gravity>,
    mut query: Query<&mut Velocity, With<KinematicBody>>,
) {
    for mut velocity in query.iter_mut() {
        **velocity += **gravity * time.delta_seconds();
    }
}
*/

enum CollisionType {
    Rect(Rect),
    Level(LevelId),
}

#[derive(Default)]
struct PhysicsWorld {
    collisions: Vec<(Entity, CollisionType)>,
}

fn debug_level_collision(
    player: Query<(&GlobalTransform, &RectCollision), With<Player>>,
    mut tiles: Query<(&mut Tile, &TilePos)>,
) {
    if let Ok((transform, collision)) = player.get_single() {
        let rect = collision.rect.translate(transform.translation.truncate());
        // FIXME: This assumes tile size ~= collider size and only checks corners
        let x = rect.min.x;
        let y = rect.min.y;
        let w = rect.width();
        let h = rect.height();
        let tiles_to_check = [
            round(vec2(x, -y)),
            round(vec2(x + w, -y)),
            round(vec2(x, -y - h)),
            round(vec2(x + w, -y - h)),
        ];

        fn round(point: Vec2) -> IVec2 {
            ivec2((point.x - 0.5).floor() as i32, point.y.round() as i32)
        }

        for (mut tile, tile_pos) in tiles.iter_mut() {
            if tiles_to_check.contains(&ivec2(tile_pos.0 as i32, tile_pos.1 as i32)) {
                tile.visible = false;
            } else {
                tile.visible = true;
            }
        }
    }
}

impl PhysicsWorld {
    pub fn get_rect_collisions(
        &self,
        rect: Rect,
        this_entity: Entity,
        project: &LdtkProject,
    ) -> Option<Entity> {
        for (other, other_collision) in self.collisions.iter() {
            if *other == this_entity {
                continue;
            }
            match other_collision {
                CollisionType::Rect(other_rect) if other_rect.intersects(rect) => {
                    return Some(*other);
                }
                CollisionType::Level(_level_id) => {
                    // FIXME: This assumes tile size ~= collider size and only checks corners
                    let x = rect.min.x;
                    let y = rect.min.y;
                    let w = rect.width();
                    let h = rect.height();
                    let tiles_to_check = [
                        round(vec2(x, y)),
                        round(vec2(x + w, y)),
                        round(vec2(x, y + h)),
                        round(vec2(x + w, y + h)),
                    ];

                    fn round(point: Vec2) -> IVec2 {
                        ivec2((point.x - 0.5).floor() as i32, point.y.round() as i32)
                    }

                    if tiles_to_check.into_iter().any(|pos| {
                        matches!(
                            project.get_tile(pos.x as i64, pos.y as i64),
                            Some(LevelTile::Solid)
                        )
                    }) {
                        // FIXME: This return value is incorrect as the whole ldtk map is checked for collisions instead of this single level
                        return Some(*other);
                    }
                }

                _ => (),
            }
        }

        None
    }
}

fn update_physics_world(
    mut world: ResMut<PhysicsWorld>,
    rect_colliders: Query<(Entity, &RectCollision, &GlobalTransform), With<StaticBody>>,

    level_colliders: Query<(Entity, &LevelCollision, &LevelId), With<StaticBody>>,
) {
    world.collisions.clear();
    for (entity, collision, transform) in rect_colliders.iter() {
        let col_rect = collision.rect.translate(transform.translation.truncate());
        world
            .collisions
            .push((entity, CollisionType::Rect(col_rect)));
    }
    for (entity, _collision, level_id) in level_colliders.iter() {
        world
            .collisions
            .push((entity, CollisionType::Level(level_id.clone())));
    }
}

/// Detects bodies being touched by [`SensorBody`] and adds the [`SensedBodies`] component to them
pub fn detect_bodies(
    mut commands: Commands,
    world: Res<GameWorld>,
    map_assets: Res<Assets<LdtkProject>>,

    // Rect colliders
    rect_colliders: Query<(Entity, &RectCollision, &GlobalTransform), With<StaticBody>>,
    // Level colliders
    level_colliders: Query<(Entity, &LevelCollision, &LevelId, &GlobalTransform), With<StaticBody>>,
    // Bodies
    bodies: Query<(Entity, &GlobalTransform, &RectCollision, &SensorBody)>,
) {
    let project = if let Some(project) = map_assets.get(&world.ldtk) {
        project
    } else {
        return;
    };

    for (entity, transform, collision, _body) in bodies.iter() {
        let col_rect = collision.rect.translate(transform.translation.truncate());
        let mut bodies_sensed = Vec::new();

        for (other, other_collision, other_transform) in rect_colliders.iter() {
            if other == entity {
                continue;
            }
            if other_collision
                .rect
                .translate(other_transform.translation.truncate())
                .intersects(col_rect)
            {
                bodies_sensed.push(other);
            }
        }

        for (other, _other_collision, _level_id, _other_transform) in level_colliders.iter() {
            if other == entity {
                continue;
            }
            // FIXME: This assumes tile size ~= collider size and only checks corners
            let x = col_rect.min.x;
            let y = col_rect.min.y;
            let w = col_rect.width();
            let h = col_rect.height();
            let tiles_to_check = [
                round(vec2(x, y)),
                round(vec2(x + w, y)),
                round(vec2(x, y + h)),
                round(vec2(x + w, y + h)),
            ];

            fn round(point: Vec2) -> IVec2 {
                ivec2((point.x - 0.5).floor() as i32, point.y.round() as i32)
            }

            if tiles_to_check.into_iter().any(|pos| {
                matches!(
                    project.get_tile(pos.x as i64, pos.y as i64),
                    Some(LevelTile::Solid)
                )
            }) {
                bodies_sensed.push(other);
            }
        }
        commands.entity(entity).insert(SensedBodies {
            others: bodies_sensed,
        });
    }
}

// IMPORTANT: This must run one stage before systems that make use of collision data (e.g. Collisions)
// because commands are executed at the end of the stage
fn move_bodies(
    mut commands: Commands,
    time: Res<Time>,
    world: Res<GameWorld>,
    physics_world: Res<PhysicsWorld>,
    map_assets: Res<Assets<LdtkProject>>,
    mut bodies: Query<(
        Entity,
        &mut GlobalTransform,
        &mut Velocity,
        &RectCollision,
        &KinematicBody,
    )>,
) {
    let delta_time = time.delta_seconds();
    let project = if let Some(x) = map_assets.get(&world.ldtk) {
        x
    } else {
        return;
    };

    let is_colliding = |entity: Entity, transform: &GlobalTransform, collision: &RectCollision| {
        physics_world
            .get_rect_collisions(
                collision.rect.translate(transform.translation.truncate()),
                entity,
                project,
            )
            .is_some()
    };

    for (entity, mut transform, mut velocity, collision, _body) in bodies.iter_mut() {
        let mut to_move = (**velocity) * delta_time;

        if to_move.x == 0. && to_move.y == 0. {
            continue;
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
        let mut collisions = CollisionSide::empty();

        while to_move.length() >= step.length() {
            let last_position = transform.translation;
            transform.translation += step.extend(0.0);
            to_move -= step;

            if is_colliding(entity, &*transform, collision) {
                // Move one axis at a time to figure out where/how the collision happened
                transform.translation.x = last_position.x;
                if !is_colliding(entity, &*transform, collision) {
                    // Not colliding when moved back on the X axis, the body was blocked by a wall
                    collisions |= if velocity.x > 0. {
                        CollisionSide::RIGHT
                    } else {
                        CollisionSide::LEFT
                    };

                    velocity.x = 0.;
                    to_move.x = 0.;
                } else {
                    transform.translation.x += step.x;
                    transform.translation.y = last_position.y;
                    if !is_colliding(entity, &*transform, collision) {
                        // Not colliding when moved back on the Y axis, the body was blocked by
                        // the ground/ceiling
                        collisions |= if velocity.y > 0. {
                            CollisionSide::UP
                        } else {
                            CollisionSide::DOWN
                        };

                        velocity.y = 0.;
                        to_move.y = 0.;
                    } else {
                        // Colliding in both axes; Stop all movement
                        transform.translation = last_position;
                        collisions |= if velocity.x > 0. {
                            CollisionSide::RIGHT
                        } else {
                            CollisionSide::LEFT
                        };
                        collisions |= if velocity.y > 0. {
                            CollisionSide::UP
                        } else {
                            CollisionSide::DOWN
                        };

                        **velocity = Vec2::ZERO;
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

        commands
            .entity(entity)
            .insert(KinematicCollisions { sides: collisions });
    }
}
