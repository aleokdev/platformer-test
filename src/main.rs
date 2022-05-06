use std::path::Path;

use bevy::{asset::AssetServerSettings, prelude::*, window::exit_on_window_close_system};
use bevy_ecs_tilemap::TilemapPlugin;
use platformer_test::{
    camera::FollowPlugin,
    input_binding::{InputBinder, InputBindingPlugin},
    physics::PhysicsPlugin,
    player::{PlayerBundle, PlayerPlugin},
    setup,
    world::WorldPlugin,
    AppState,
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
        .run();

    /*
    let cb = ggez::ContextBuilder::new("Platformer Template", "aleok")
        .window_setup(conf::WindowSetup::default().title("Platformer test"))
        .window_mode(conf::WindowMode::default().dimensions(40. * 18., 25. * 18.))
        .add_resource_path(
            std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default())
                .join("assets"),
        );
    let (mut ctx, event_loop) = cb.build()?;
    let state = MainState::new(&mut ctx)?;
    event::run(ctx, event_loop, state)
    */
}
