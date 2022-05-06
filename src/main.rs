use bevy::{input::system::exit_on_esc_system, prelude::*, window::exit_on_window_close_system};

use platformer_test::{
    camera::FollowPlugin, input_binding::InputBindingPlugin, physics::PhysicsPlugin,
    player::PlayerPlugin, setup, world::WorldPlugin, AppState,
};

pub fn main() {
    App::new()
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
        .add_system(exit_on_window_close_system)
        .add_system(exit_on_esc_system)
        .run();
}
