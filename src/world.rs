use std::{collections::HashMap, path::Path};

use crate::physics::LevelCollision;
use crate::physics::RectExtras;
use crate::physics::StaticBody;
use crate::AppState;
use bevy::asset::{AssetPath, LoadedAsset};
use bevy::ecs::world;
use bevy::reflect::TypeUuid;
use bevy::render::render_resource::TextureUsages;
use bevy::sprite::Rect;
use bevy::{asset::AssetLoader, prelude::*};
use bevy_ecs_tilemap::{
    Chunk, ChunkPos, ChunkSize, Layer, LayerBuilder, LayerBundle, LayerSettings, Map, MapSize,
    TextureSize, TileBundle, TileBundleTrait, TilePos, TileSize, TilemapPlugin,
};
use glam::{ivec2, vec2, vec3, IVec2, Vec2};
use path_clean::PathClean;
use serde::Deserialize;

#[derive(TypeUuid)]
#[uuid = "ace787fd-c5d2-4651-a42b-f08fa985676c"]
pub struct LdtkProject {
    pub project: ldtk_rust::Project,
    pub tilesets: HashMap<i64, Handle<Image>>,
}

pub struct LdtkLoader;

impl AssetLoader for LdtkLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::asset::BoxedFuture<'a, anyhow::Result<(), anyhow::Error>> {
        Box::pin(async move {
            let project: ldtk_rust::Project = serde_json::from_slice(bytes)?;
            let dependencies: Vec<(i64, AssetPath)> = project
                .defs
                .tilesets
                .iter()
                .filter_map(|tileset| {
                    tileset.rel_path.as_ref().map(|path| {
                        (
                            tileset.uid,
                            load_context
                                .path()
                                .parent()
                                .unwrap()
                                .join(path.clone())
                                .into(),
                        )
                    })
                })
                .collect();
            let loaded_asset = LoadedAsset::new(LdtkProject {
                project,
                tilesets: dependencies
                    .iter()
                    .map(|dep| (dep.0, load_context.get_handle(dep.1.clone())))
                    .collect(),
            });
            load_context.set_default_asset(
                loaded_asset.with_dependencies(dependencies.iter().map(|x| x.1.clone()).collect()),
            );

            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["ldtk"]
    }
}

pub struct GameWorld {
    pub ldtk: Handle<LdtkProject>,
}

pub enum LevelTile {
    Solid,
}

impl LdtkProject {
    pub fn get_tile(&self, x: i64, y: i64) -> Option<LevelTile> {
        const TILE_SIZE: u32 = 16;

        self.project
            .levels
            .iter()
            .find(|level| {
                Rect::from_min_size(
                    vec2(
                        level.world_x as f32 / TILE_SIZE as f32,
                        level.world_y as f32 / TILE_SIZE as f32,
                    ),
                    vec2(
                        level.px_wid as f32 / TILE_SIZE as f32,
                        level.px_hei as f32 / TILE_SIZE as f32,
                    ),
                )
                .contains(vec2(x as f32, y as f32))
            })
            .and_then(|level| {
                let local_pos = (
                    x - level.world_x / TILE_SIZE as i64,
                    y - level.world_y / TILE_SIZE as i64,
                );

                level
                    .layer_instances
                    .iter()
                    .flatten()
                    .find(|layer| layer.identifier == "Collision")
                    .and_then(|layer| {
                        let idx = local_pos.0 + local_pos.1 * layer.c_wid;
                        if layer.int_grid_csv[idx as usize] != 0 {
                            Some(LevelTile::Solid)
                        } else {
                            None
                        }
                    })
            })
    }
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(TilemapPlugin)
            .add_asset::<LdtkProject>()
            .add_asset_loader(LdtkLoader)
            .add_system(process_loaded_tile_maps)
            .add_system(set_texture_usages);
    }
}

#[derive(Component, Deref, DerefMut, Default)]
pub struct LevelId(pub String);

#[derive(Bundle, Default)]
pub struct LevelBundle {
    pub map: Map,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub level_id: LevelId,
    pub collision: LevelCollision,
    pub body: StaticBody,
}

pub fn set_texture_usages(
    mut texture_events: EventReader<AssetEvent<Image>>,
    mut textures: ResMut<Assets<Image>>,
) {
    // quick and dirty, run this for all textures anytime a texture is created.
    for event in texture_events.iter() {
        match event {
            AssetEvent::Created { handle } => {
                if let Some(mut texture) = textures.get_mut(handle) {
                    texture.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
                        | TextureUsages::COPY_SRC
                        | TextureUsages::COPY_DST;
                }
            }
            _ => (),
        }
    }
}

pub fn process_loaded_tile_maps(
    mut commands: Commands,
    mut map_events: EventReader<AssetEvent<LdtkProject>>,
    maps: Res<Assets<LdtkProject>>,
    world: Res<GameWorld>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut query: Query<(Entity, &LevelId, &mut Map)>,
    layer_query: Query<&Layer>,
    chunk_query: Query<&Chunk>,
) {
    let changed_project = map_events
        .iter()
        .inspect(|x| info!("{:?}", x))
        .any(|event| matches!(event, AssetEvent::Modified { handle } | AssetEvent::Created { handle } if handle == &world.ldtk));

    if !changed_project {
        return;
    }

    info!("Project was changed, updating map");

    for (_, level_id, mut map) in query.iter_mut() {
        if let Some(ldtk_map) = maps.get(&world.ldtk) {
            // Despawn all tiles/chunks/layers.
            for (layer_id, layer_entity) in map.get_layers() {
                if let Ok(layer) = layer_query.get(layer_entity) {
                    for x in 0..layer.get_layer_size_in_tiles().0 {
                        for y in 0..layer.get_layer_size_in_tiles().1 {
                            let tile_pos = TilePos(x, y);
                            let chunk_pos = ChunkPos(
                                tile_pos.0 / layer.settings.chunk_size.0,
                                tile_pos.1 / layer.settings.chunk_size.1,
                            );
                            if let Some(chunk_entity) = layer.get_chunk(chunk_pos) {
                                if let Ok(chunk) = chunk_query.get(chunk_entity) {
                                    let chunk_tile_pos = chunk.to_chunk_pos(tile_pos);
                                    if let Ok(chunk_tile_pos) = chunk_tile_pos {
                                        if let Some(tile) = chunk.get_tile_entity(chunk_tile_pos) {
                                            commands.entity(tile).despawn_recursive();
                                        }
                                    }
                                }

                                commands.entity(chunk_entity).despawn_recursive();
                            }
                        }
                    }
                }
                map.remove_layer(&mut commands, layer_id);
            }

            // Pull out tilesets.
            let mut tilesets = HashMap::new();
            ldtk_map
                .project
                .defs
                .tilesets
                .iter()
                // Filter out internal icons
                .filter(|tileset| tileset.rel_path.is_some())
                .for_each(|tileset| {
                    tilesets.insert(
                        tileset.uid,
                        (
                            ldtk_map.tilesets.get(&tileset.uid).unwrap().clone(),
                            tileset.clone(),
                        ),
                    );
                });

            let default_grid_size = ldtk_map.project.default_grid_size;
            let level = ldtk_map
                .project
                .levels
                .iter()
                .find(|&l| l.identifier == **level_id)
                .unwrap();

            let map_tile_count_x = (level.px_wid / default_grid_size) as u32;
            let map_tile_count_y = (level.px_hei / default_grid_size) as u32;

            let map_size = MapSize(
                (map_tile_count_x as f32 / 32.0).ceil() as u32,
                (map_tile_count_y as f32 / 32.0).ceil() as u32,
            );

            for (layer_id, layer) in level
                .layer_instances
                .as_ref()
                .unwrap()
                .iter()
                .rev()
                .enumerate()
            {
                let (texture, tileset) = if let Some(uid) = layer.tileset_def_uid {
                    tilesets.get(&uid).unwrap().clone()
                } else {
                    continue;
                };

                let settings = LayerSettings::new(
                    map_size,
                    ChunkSize(32, 32),
                    TileSize(tileset.tile_grid_size as f32, tileset.tile_grid_size as f32),
                    TextureSize(tileset.px_wid as f32, tileset.px_hei as f32),
                );

                let (mut layer_builder, layer_entity) = LayerBuilder::<TileBundle>::new(
                    &mut commands,
                    settings,
                    map.id,
                    layer_id as u16,
                );

                let tileset_width_in_tiles = (tileset.px_wid / default_grid_size) as u32;

                for tile in layer.auto_layer_tiles.iter() {
                    let tileset_x = (tile.src[0] / default_grid_size) as u32;
                    let tileset_y = (tile.src[1] / default_grid_size) as u32;

                    let mut pos = TilePos(
                        (tile.px[0] / default_grid_size) as u32,
                        (tile.px[1] / default_grid_size) as u32,
                    );

                    pos.1 = map_tile_count_y - pos.1 - 1;

                    layer_builder
                        .set_tile(
                            pos,
                            bevy_ecs_tilemap::Tile {
                                texture_index: (tileset_y * tileset_width_in_tiles + tileset_x)
                                    as u16,
                                ..default()
                            }
                            .into(),
                        )
                        .unwrap();
                }

                let layer_bundle = layer_builder.build(&mut commands, &mut meshes, texture);
                let layer = layer_bundle.layer;
                let mut transform = Transform::from_xyz(
                    0.0,
                    -level.px_hei as f32 / 16.,
                    layer_bundle.transform.translation.z,
                )
                .with_scale(vec3(1. / 16., 1. / 16., 1.));

                transform.translation.z = layer_id as f32;
                map.add_layer(&mut commands, layer_id as u16, layer_entity);
                commands.entity(layer_entity).insert_bundle(LayerBundle {
                    layer,
                    transform,
                    ..layer_bundle
                });
            }
        }
    }
}
