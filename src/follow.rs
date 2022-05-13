use bevy::math::vec2;
use bevy::prelude::*;
use bevy::sprite::Rect;

#[derive(Component)]
pub struct CameraFollow {
    pub target: Option<Entity>,
    pub bounds: Rect,
}

impl Default for CameraFollow {
    fn default() -> Self {
        Self {
            target: None,
            bounds: Rect {
                min: vec2(f32::NEG_INFINITY, f32::NEG_INFINITY),
                max: vec2(f32::INFINITY, f32::INFINITY),
            },
        }
    }
}

pub struct FollowPlugin;

impl Plugin for FollowPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_to_stage(CoreStage::PostUpdate, follow);
    }
}

fn follow(
    mut query: Query<(&mut Transform, &OrthographicProjection, &CameraFollow)>,
    target_q: Query<&Transform, Without<CameraFollow>>,
) {
    for (mut transform, projection, follow) in query.iter_mut() {
        if let Some(target_transform) = follow.target.and_then(|t| target_q.get(t).ok()) {
            let min = follow.bounds.min.x - projection.left;
            let max = follow.bounds.max.x - projection.right;
            transform.translation.x = if max <= min {
                (follow.bounds.min.x + follow.bounds.max.x) / 2.
            } else {
                target_transform.translation.x.clamp(min, max)
            };
            let min = follow.bounds.min.y - projection.bottom;
            let max = follow.bounds.max.y - projection.top;
            transform.translation.y = if max <= min {
                (follow.bounds.min.y + follow.bounds.max.y) / 2.
            } else {
                target_transform.translation.y.clamp(min, max)
            };
        }
    }
}
