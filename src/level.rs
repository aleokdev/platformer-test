use std::path::PathBuf;

use ggez::*;
use glam::{vec2, Vec2};

pub enum LevelTile {
    Solid,
}

pub struct Level {
    tiles: Vec<Option<LevelTile>>,
    width: u32,
    height: u32,
    tile_batch: graphics::spritebatch::SpriteBatch,
    pub spawn_point: Vec2,
}

impl Level {
    pub fn new(map: tiled::Map, ctx: &mut Context) -> GameResult<Self> {
        let mut image_path = PathBuf::from("/");
        image_path.push(&map.tilesets()[0].image.as_ref().unwrap().source);
        let image = graphics::Image::new(ctx, image_path)?;
        let tile_batch = Self::generate_tile_batch(image, &map)?;
        let tiles = if let tiled::LayerType::TileLayer(tiled::TileLayer::Finite(layer)) =
            map.get_layer(0).unwrap().layer_type()
        {
            (0..layer.height() as i32)
                .flat_map(|y| (0..layer.width() as i32).map(move |x| (x, y)))
                .map(|(x, y)| layer.get_tile(x, y).map(|_| LevelTile::Solid))
                .collect()
        } else {
            panic!()
        };

        let spawn_point = map
            .layers()
            .find_map(|layer| match layer.layer_type() {
                tiled::LayerType::ObjectLayer(layer) => Some(layer),
                _ => None,
            })
            .and_then(|layer| layer.objects().nth(0))
            .map(|obj| {
                vec2(
                    obj.x() / map.tile_width as f32,
                    obj.y() / map.tile_height as f32,
                )
            })
            .unwrap();

        Ok(Level {
            tile_batch,
            tiles,
            width: map.width,
            height: map.height,
            spawn_point,
        })
    }

    pub fn get_tile(&self, x: i32, y: i32) -> Option<&LevelTile> {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            self.tiles[(x + y * self.width as i32) as usize].as_ref()
        } else {
            None
        }
    }

    fn generate_tile_batch(
        image: graphics::Image,
        map: &tiled::Map,
    ) -> GameResult<graphics::spritebatch::SpriteBatch> {
        let (width, height) = (image.width(), image.height());
        let mut batch = graphics::spritebatch::SpriteBatch::new(image);
        for layer in map.layers() {
            if let tiled::LayerType::TileLayer(tiled::TileLayer::Finite(layer)) = layer.layer_type()
            {
                for x in 0..layer.width() as i32 {
                    for y in 0..layer.height() as i32 {
                        if let Some(tile) = layer.get_tile(x, y) {
                            batch.add(
                                graphics::DrawParam::default()
                                    .src(get_tile_rect(
                                        map.tilesets()[0].as_ref(),
                                        tile.id(),
                                        width,
                                        height,
                                    ))
                                    .dest(vec2(x as f32, y as f32))
                                    .scale(vec2(1. / 18., 1. / 18.)),
                            );
                        }
                    }
                }
            }
        }

        Ok(batch)
    }

    pub fn draw(&self, ctx: &mut Context, draw_param: graphics::DrawParam) -> GameResult {
        graphics::draw(ctx, &self.tile_batch, draw_param)?;
        Ok(())
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

    graphics::Rect {
        x: x / ts_img_width,
        y: y / ts_img_height,
        w: tileset.tile_width as f32 / ts_img_width,
        h: tileset.tile_height as f32 / ts_img_height,
    }
}
