//! 2D Platformer physics.
//!
//! I first thought about using the `heron` crate, but decided against it because I needed finer
//! control over collision shapes (For tilemaps), collision sides (For player states) and didn't
//! require many of the features it offered, such as dynamic rigidbodies or rotation.

use bevy::math::{ivec2, vec2};
use bevy::sprite::Rect;
use bevy::{core::FixedTimestep, prelude::*};
use bitflags::bitflags;

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

#[derive(Component)]
pub struct KinematicBody {
    mask: LevelTile,
}

impl Default for KinematicBody {
    fn default() -> Self {
        Self {
            mask: LevelTile::SOLID,
        }
    }
}
#[derive(Component, Default)]
pub struct StaticBody;

// TODO: Merge KinematicBody & StaticBody into a RigidBody enum

#[derive(Component)]
pub struct SensorBody {
    mask: LevelTile,
}

impl Default for SensorBody {
    fn default() -> Self {
        Self {
            mask: LevelTile::SOLID,
        }
    }
}

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
    pub others: Vec<Entity>,
    pub world: bool,
}

const PHYSICS_TIME_STEP: f64 = 1. / 60.;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app /*.init_resource::<Gravity>()*/
            .init_resource::<PhysicsWorld>()
            .add_system_set_to_stage(
                CoreStage::PreUpdate,
                SystemSet::new()
                    .with_run_criteria(FixedTimestep::step(PHYSICS_TIME_STEP))
                    .with_system(update_physics_world)
                    .with_system(move_bodies.after(update_physics_world))
                    .with_system(detect_bodies.after(update_physics_world)), //.with_system(gravity),
            );
    }
}

#[derive(Default)]
struct PhysicsWorld {
    collisions: Vec<(Entity, Rect)>,
}

impl PhysicsWorld {
    pub fn get_rect_collisions(&self, rect: Rect, this_entity: Entity) -> Option<Entity> {
        for (other, other_collision) in self.collisions.iter() {
            if *other == this_entity {
                continue;
            }
            if other_collision.intersects(rect) {
                return Some(*other);
            }
        }

        None
    }
}

fn is_colliding_with_world(rect: Rect, project: &LdtkProject, mask: LevelTile) -> bool {
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

    tiles_to_check
        .into_iter()
        .any(|pos| !(mask & project.get_tile(pos.x as i64, pos.y as i64)).is_empty())
}

fn update_physics_world(
    mut world: ResMut<PhysicsWorld>,
    rect_colliders: Query<(Entity, &RectCollision, &GlobalTransform), With<StaticBody>>,
) {
    world.collisions.clear();
    for (entity, collision, transform) in rect_colliders.iter() {
        let col_rect = collision.rect.translate(transform.translation.truncate());
        world.collisions.push((entity, col_rect));
    }
}

/// Detects bodies being touched by [`SensorBody`] and adds the [`SensedBodies`] component to them
pub fn detect_bodies(
    mut commands: Commands,
    world: Res<GameWorld>,
    map_assets: Res<Assets<LdtkProject>>,

    // Rect colliders
    rect_colliders: Query<(Entity, &RectCollision, &GlobalTransform), With<StaticBody>>,
    // Bodies
    bodies: Query<(Entity, &GlobalTransform, &RectCollision, &SensorBody)>,
) {
    let project = if let Some(project) = map_assets.get(&world.ldtk) {
        project
    } else {
        return;
    };

    for (entity, transform, collision, body) in bodies.iter() {
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

        commands.entity(entity).insert(SensedBodies {
            others: bodies_sensed,
            world: is_colliding_with_world(col_rect, project, body.mask),
        });
    }
}

// IMPORTANT: This must run one stage before systems that make use of collision data (e.g. Collisions)
// because commands are executed at the end of the stage
fn move_bodies(
    mut commands: Commands,
    world: Res<GameWorld>,
    physics_world: Res<PhysicsWorld>,
    map_assets: Res<Assets<LdtkProject>>,
    mut bodies: Query<(
        Entity,
        &mut Transform,
        &mut Velocity,
        &RectCollision,
        &KinematicBody,
    )>,
) {
    let delta_time = PHYSICS_TIME_STEP as f32;
    let project = if let Some(x) = map_assets.get(&world.ldtk) {
        x
    } else {
        return;
    };

    for (entity, mut transform, mut velocity, collision, body) in bodies.iter_mut() {
        let is_colliding = |position: Vec2| {
            let rect = collision.rect.translate(position);
            physics_world.get_rect_collisions(rect, entity).is_some()
                || is_colliding_with_world(rect, project, body.mask)
        };

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

            if is_colliding(transform.translation.truncate()) {
                // Move one axis at a time to figure out where/how the collision happened
                transform.translation.x = last_position.x;
                if !is_colliding(transform.translation.truncate()) {
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
                    if !is_colliding(transform.translation.truncate()) {
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
