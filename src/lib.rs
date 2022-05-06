pub mod camera;
pub mod input_binding;
pub mod physics;
pub mod player;
pub mod util;
pub mod world;

use bevy_ecs_tilemap::Map;
use input_binding::InputBinder;
use player::spawn_player;
pub use player::{Player, PlayerProperties};
use world::GameWorld;
pub use world::LdtkProject;

use bevy::{asset::AssetServerSettings, prelude::*, render::camera::ScalingMode};

use crate::{
    camera::SmoothFollow,
    world::{LevelBundle, LevelId},
};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum AppState {
    Loading,
    Playing,
}

pub fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut input_bindings: ResMut<InputBinder>,
) {
    let player = spawn_player(&mut commands);
    commands
        .spawn_bundle(OrthographicCameraBundle {
            orthographic_projection: OrthographicProjection {
                scale: 10.,
                scaling_mode: ScalingMode::FixedVertical,
                ..default()
            },
            ..OrthographicCameraBundle::new_2d()
        })
        .insert(SmoothFollow {
            target: Some(player),
            ..default()
        });

    commands.insert_resource(AssetServerSettings {
        watch_for_changes: true,
        ..default()
    });
    info!("Starting to load world file");
    let ldtk: Handle<LdtkProject> = asset_server.load("world.ldtk");
    commands.insert_resource(GameWorld { ldtk });

    let map_entity = commands.spawn().id();

    info!("Inserted level");
    commands.entity(map_entity).insert_bundle(LevelBundle {
        map: Map::new(0u16, map_entity),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        level_id: LevelId("Level_0".to_owned()),
        ..Default::default()
    });

    // TODO: use asset loading for bindings
    input_bindings.load_from_str(include_str!("../assets/input.ron"));
}
