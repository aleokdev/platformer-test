use bevy::{
    asset::AssetServerSettings,
    input::system::exit_on_esc_system,
    prelude::*,
    window::{exit_on_window_close_system, PresentMode},
};

use platformer_test::{
    camera_follow_player,
    debug::DebugPlugin,
    follow::FollowPlugin,
    input_mapper::InputBindingPlugin,
    physics::PhysicsPlugin,
    player::{spawn_player, PlayerPlugin},
    setup, show_fps,
    time::TimePlugin,
    world::{change_to_playing_state_on_level_load, WorldPlugin},
    AppState,
};

pub fn main() {
    let mut app = App::new();

    app.insert_resource(AssetServerSettings {
        watch_for_changes: true,
        ..default()
    })
    .add_plugins(DefaultPlugins)
    .add_plugin(bevy_framepace::FramepacePlugin::default())
    .add_state(AppState::Loading)
    .add_plugin(bevy_egui::EguiPlugin)
    .add_plugin(InputBindingPlugin)
    .add_plugin(WorldPlugin)
    .add_plugin(PlayerPlugin)
    .add_plugin(PhysicsPlugin)
    .add_plugin(FollowPlugin)
    .add_plugin(TimePlugin)
    .insert_resource(ClearColor(Color::hex("34202b").unwrap()))
    .insert_resource(WindowDescriptor {
        title: "Platform Template".to_owned(),
        present_mode: PresentMode::Mailbox,
        ..default()
    })
    .add_startup_system(setup)
    .add_system(camera_follow_player)
    .add_system(show_fps)
    .add_system_set(
        SystemSet::on_update(AppState::Loading).with_system(change_to_playing_state_on_level_load),
    )
    .add_system_set(SystemSet::on_enter(AppState::Playing).with_system(spawn_player))
    .add_system(exit_on_window_close_system)
    .add_system(exit_on_esc_system);

    #[cfg(debug_assertions)]
    app.add_plugin(DebugPlugin);

    app.run();
}
