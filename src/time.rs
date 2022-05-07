use bevy::{core::Stopwatch, prelude::*};

use crate::AppState;

/// Records unpaused time, used for gameplay logic.
#[derive(Deref, DerefMut, Default)]
pub struct GameplayTime(pub Stopwatch);

pub struct TimePlugin;

impl Plugin for TimePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameplayTime>()
            .add_system_set(SystemSet::on_enter(AppState::Paused).with_system(pause_gameplay_time))
            .add_system_set(
                SystemSet::on_enter(AppState::Playing).with_system(unpause_gameplay_time),
            )
            .add_system_set(
                SystemSet::on_update(AppState::Playing).with_system(tick_gameplay_time),
            );
    }
}

fn pause_gameplay_time(mut time: ResMut<GameplayTime>) {
    time.pause();
}

fn unpause_gameplay_time(mut time: ResMut<GameplayTime>) {
    time.unpause();
}

fn tick_gameplay_time(rtime: Res<Time>, mut time: ResMut<GameplayTime>) {
    time.tick(rtime.delta());
}
