use std::{collections::HashMap, path::PathBuf};

use ggez::*;
use glam::{vec2, IVec2, Vec2};
use path_clean::PathClean;

pub enum LevelTile {
    Solid,
}

pub struct Level {
    tiles: Vec<Option<LevelTile>>,
    width: u32,
    height: u32,
    tile_batch: Vec<graphics::spritebatch::SpriteBatch>,
    pub spawn_point: Vec2,
}

impl Level {
    pub fn new(map: tiled::Map, ctx: &mut Context) -> GameResult<Self> {
        let mut image_path = PathBuf::from("/");
        image_path.push(&map.tilesets()[0].image.as_ref().unwrap().source);
        let tile_batch = Self::generate_tile_batch(ctx, &map)?;
        let tiles = Self::extract_level_tiles(&map);

        let spawn_point = Self::locate_spawn_point(&map).unwrap();

        Ok(Level {
            tile_batch,
            tiles,
            width: map.width,
            height: map.height,
            spawn_point,
        })
    }

    pub fn get_tile(&self, pos: IVec2) -> Option<&LevelTile> {
        if pos.x >= 0 && pos.x < self.width as i32 && pos.y >= 0 && pos.y < self.height as i32 {
            self.tiles[(pos.x + pos.y * self.width as i32) as usize].as_ref()
        } else {
            None
        }
    }

    pub fn draw(&self, ctx: &mut Context, draw_param: graphics::DrawParam) -> GameResult {
        for batch in self.tile_batch.iter() {
            graphics::draw(ctx, batch, draw_param)?;
        }
        Ok(())
    }

    /// Get the level's width.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Get the level's height.
    pub fn height(&self) -> u32 {
        self.height
    }
}

impl Level {
    fn generate_tile_batch(
        ctx: &mut Context,
        map: &tiled::Map,
    ) -> GameResult<Vec<graphics::spritebatch::SpriteBatch>> {
        let mut all_batches = Vec::<graphics::spritebatch::SpriteBatch>::with_capacity(16);
        for layer in map.layers() {
            let mut batches =
                HashMap::<PathBuf, (graphics::spritebatch::SpriteBatch, graphics::Image)>::new(); // FIXME: Wait until SpriteBatch::image() is public
            if let tiled::LayerType::Tiles(tiled::TileLayer::Finite(layer)) = layer.layer_type() {
                for x in 0..layer.width() as i32 {
                    for y in 0..layer.height() as i32 {
                        if let Some(tile) = layer.get_tile(x, y) {
                            let (batch, image) = batches
                                .entry(tile.get_tileset().image.as_ref().unwrap().source.clone())
                                .or_insert_with_key(|path| {
                                    let p = PathBuf::from("/").join(path).clean();
                                    let image = graphics::Image::new(ctx, p).unwrap();
                                    let batch =
                                        graphics::spritebatch::SpriteBatch::new(image.clone());
                                    (batch, image)
                                });
                            batch.add(
                                graphics::DrawParam::default()
                                    .src(get_tile_rect(
                                        map.tilesets()[0].as_ref(),
                                        tile.id(),
                                        image.width(),
                                        image.height(),
                                    ))
                                    .dest(vec2(x as f32, y as f32))
                                    .scale(vec2(1. / 18., 1. / 18.)),
                            );
                        }
                    }
                }
            }
            all_batches.extend(batches.into_iter().map(|(_ts_path, (batch, _img))| batch))
        }

        Ok(all_batches)
    }

    fn extract_level_tiles(map: &tiled::Map) -> Vec<Option<LevelTile>> {
        if let tiled::LayerType::Tiles(tiled::TileLayer::Finite(layer)) = map
            .layers()
            .find(|layer| layer.name == "solid")
            .unwrap()
            .layer_type()
        {
            (0..layer.height() as i32)
                .flat_map(|y| (0..layer.width() as i32).map(move |x| (x, y)))
                .map(|(x, y)| layer.get_tile(x, y).map(|_| LevelTile::Solid))
                .collect()
        } else {
            panic!()
        }
    }

    fn locate_spawn_point(map: &tiled::Map) -> Option<Vec2> {
        map.layers().find_map(|layer| match layer.layer_type() {
            tiled::LayerType::Objects(layer) => layer.objects().nth(0).map(|obj| {
                vec2(
                    obj.x / map.tile_width as f32,
                    obj.y / map.tile_height as f32,
                )
            }),
            _ => None,
        })
    }
}

fn get_tile_rect(
    tileset: &tiled::Tileset,
    id: u32,
    ts_img_width: u16,
    ts_img_height: u16,
) -> graphics::Rect {
    let ts_x = id % tileset.columns;
    let ts_y = id / tileset.columns;

    let x = (tileset.margin + (tileset.tile_width + tileset.spacing) * ts_x) as f32;
    let y = (tileset.margin + (tileset.tile_height + tileset.spacing) * ts_y) as f32;

    let ts_img_width = ts_img_width as f32;
    let ts_img_height = ts_img_height as f32;

    let x = x / ts_img_width;
    let y = y / ts_img_height;
    let w = tileset.tile_width as f32 / ts_img_width;
    let h = tileset.tile_height as f32 / ts_img_height;

    graphics::Rect { x, y, w, h }
}
