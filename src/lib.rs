pub mod camera;
pub mod input_mapper;
pub mod physics;
pub mod player;
pub mod time;
pub mod util;
pub mod world;

use input_mapper::InputMapper;

pub use player::{Player, PlayerProperties};
use world::GameWorld;
pub use world::LdtkProject;

use bevy::{prelude::*, render::camera::ScalingMode};

use crate::{camera::SmoothFollow, input_mapper::InputMappings};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum AppState {
    Loading,
    Playing,
    Paused,
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

    input_mapper.mappings = asset_server.load::<InputMappings, _>("input.ron");
}
