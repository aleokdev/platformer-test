use std::path::Path;

use bevy::{asset::AssetServerSettings, prelude::*};
use bevy_ecs_tilemap::TilemapPlugin;
use platformer_test::{
    input_binding::{InputBinder, InputBindingPlugin},
    physics::PhysicsPlugin,
    player::PlayerBundle,
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
        .add_plugin(PhysicsPlugin)
        .add_startup_system(setup)
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
