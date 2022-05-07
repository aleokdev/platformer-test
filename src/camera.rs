use bevy::prelude::*;

#[derive(Component)]
pub struct SmoothFollow {
    pub target: Option<Entity>,
    pub multiplier: f32,
}

impl Default for SmoothFollow {
    fn default() -> Self {
        Self {
            target: None,
            multiplier: 0.95,
        }
    }
}

pub struct FollowPlugin;

impl Plugin for FollowPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(follow);
    }
}

pub fn follow(
    time: Res<Time>,
    mut query: Query<(&mut GlobalTransform, &SmoothFollow)>,
    target_q: Query<&GlobalTransform, Without<SmoothFollow>>,
) {
    let delta = time.delta_seconds();

    for (mut transform, follow) in query.iter_mut() {
        if let Some(target_transform) = follow.target.and_then(|t| target_q.get(t).ok()) {
            let current_translation = transform.translation;
            transform.translation +=
                (target_transform.translation - current_translation) * follow.multiplier * delta;
            transform.translation.z = current_translation.z;
        }
    }
}
