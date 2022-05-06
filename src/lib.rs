pub mod camera;
pub mod input_mapper;
pub mod physics;
pub mod player;
pub mod util;
pub mod world;

use bevy_ecs_tilemap::Map;
use input_mapper::InputMapper;
use player::spawn_player;
pub use player::{Player, PlayerProperties};
use world::GameWorld;
pub use world::LdtkProject;

use bevy::{asset::AssetServerSettings, prelude::*, render::camera::ScalingMode};

use crate::{
    camera::SmoothFollow,
    input_mapper::InputMappings,
    world::{LevelBundle, LevelId},
};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum AppState {
    Loading,
    Playing,
}

pub fn camera_follow_player(
    mut commands: Commands,
    camera_query: Query<(Entity, &Camera), Without<SmoothFollow>>,
    player_query: Query<Entity, With<Player>>,
) {
    if let Ok(player) = player_query.get_single() {
        for (entity, _) in camera_query.iter() {
            commands.entity(entity).insert(SmoothFollow {
                target: Some(player),
                ..default()
            });
        }
    }
}

pub fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut input_mapper: ResMut<InputMapper>,
) {
    commands.spawn_bundle(OrthographicCameraBundle {
        orthographic_projection: OrthographicProjection {
            scale: 10.,
            scaling_mode: ScalingMode::FixedVertical,
            ..default()
        },
        ..OrthographicCameraBundle::new_2d()
    });

    let ldtk: Handle<LdtkProject> = asset_server.load("world.ldtk");
    commands.insert_resource(GameWorld { ldtk });

    let map_entity = commands.spawn().id();

    commands.entity(map_entity).insert_bundle(LevelBundle {
        map: Map::new(0u16, map_entity),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        level_id: LevelId("Level_0".to_owned()),
        ..Default::default()
    });

    input_mapper.mappings = asset_server.load::<InputMappings, _>("input.ron");
}
