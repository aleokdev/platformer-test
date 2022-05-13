#![allow(clippy::too_many_arguments)]

pub mod debug;
pub mod follow;
pub mod input_mapper;
pub mod physics;
pub mod player;
pub mod time;
pub mod util;
pub mod world;

use debug::DebugMode;
use input_mapper::InputMapper;

pub use player::{Player, PlayerProperties};
use world::GameWorld;
pub use world::LdtkProject;

use bevy::{prelude::*, render::camera::ScalingMode};

use crate::{follow::CameraFollow, input_mapper::InputMappings};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum AppState {
    Loading,
    Playing,
    Paused,
}

pub fn camera_follow_player(
    mut commands: Commands,
    camera_query: Query<(Entity, &Camera), Without<CameraFollow>>,
    player_query: Query<Entity, With<Player>>,
) {
    if let Ok(player) = player_query.get_single() {
        for (entity, _) in camera_query.iter() {
            commands.entity(entity).insert(CameraFollow {
                target: Some(player),
                ..default()
            });
        }
    }
}

pub fn show_fps(debug: Res<DebugMode>, mut egui: ResMut<bevy_egui::EguiContext>, time: Res<Time>) {
    if !debug.active {
        return;
    }

    use bevy_egui::egui;
    egui::Window::new("Frame info [debug]").show(egui.ctx_mut(), |ui| {
        let delta = time.delta_seconds();
        ui.label(format!("Frame time: {:.3}", delta));
        ui.label(format!("FPS: {:.0}", 1. / delta));
    });
}

pub fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut input_mapper: ResMut<InputMapper>,
) {
    commands.spawn_bundle(OrthographicCameraBundle {
        orthographic_projection: OrthographicProjection {
            top: 8.,
            bottom: -8.,
            left: -8.,
            right: 8.,
            scaling_mode: ScalingMode::None,
            ..default()
        },
        ..OrthographicCameraBundle::new_2d()
    });

    let ldtk: Handle<LdtkProject> = asset_server.load("world.ldtk");
    commands.insert_resource(GameWorld { ldtk });

    input_mapper.mappings = asset_server.load::<InputMappings, _>("input.ron");
}
