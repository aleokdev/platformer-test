use bevy::{
    asset::AssetServerSettings, input::system::exit_on_esc_system, prelude::*,
    window::exit_on_window_close_system,
};

use platformer_test::{
    camera::FollowPlugin,
    camera_follow_player,
    input_mapper::InputBindingPlugin,
    physics::PhysicsPlugin,
    player::{spawn_player, PlayerPlugin},
    setup,
    world::{change_to_playing_state_on_level_load, WorldPlugin},
    AppState,
};

pub fn main() {
    App::new()
        .insert_resource(AssetServerSettings {
            watch_for_changes: true,
            ..default()
        })
        .add_state(AppState::Loading)
        .add_plugins(DefaultPlugins)
        .add_plugin(bevy_egui::EguiPlugin)
        .add_plugin(InputBindingPlugin)
        .add_plugin(WorldPlugin)
        .add_plugin(PlayerPlugin)
        .add_plugin(PhysicsPlugin)
        .add_plugin(FollowPlugin)
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(WindowDescriptor {
            title: "Platform Template".to_owned(),
            ..default()
        })
        .add_startup_system(setup)
        .add_system(camera_follow_player)
        .add_system_set(
            SystemSet::on_update(AppState::Loading)
                .with_system(change_to_playing_state_on_level_load),
        )
        .add_system_set(SystemSet::on_enter(AppState::Playing).with_system(spawn_player))
        .add_system(exit_on_window_close_system)
        .add_system(exit_on_esc_system)
        .run();
}
