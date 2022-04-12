use std::{collections::HashMap, path::Path};

use crate::{Level, LevelTile};
use ggez::*;
use glam::{ivec2, vec2, IVec2, Vec2};
use path_clean::PathClean;
use serde::Deserialize;

// Need to do newtype to implement ResourceReader for ggez's filesystem
// FIXME: This would greatly improve with separated subcontexts (ggez 0.8.0)
pub struct FsContext<'ctx>(pub &'ctx mut ggez::Context);

impl tiled::ResourceReader for FsContext<'_> {
    type Resource = filesystem::File;

    type Error = GameError;

    fn read_from(
        &mut self,
        path: &std::path::Path,
    ) -> std::result::Result<Self::Resource, Self::Error> {
        filesystem::open(&self.0, path)
    }
}

pub struct PositionedLevel {
    room_position: IVec2,
    level: Level,
}

pub struct World {
    levels: Vec<PositionedLevel>,
    /// Keys are room positions, values are indices into the levels vector
    map: HashMap<IVec2, usize>,
    room_size: IVec2,
}

impl World {
    pub fn from_file(ctx: &mut Context, path: &Path) -> anyhow::Result<Self> {
        // TODO: allow changing room size from config file
        let room_size: IVec2 = ivec2(40, 25);

        #[derive(Deserialize)]
        struct MapRefJson {
            #[serde(rename = "fileName")]
            filename: std::path::PathBuf,
            x: i32,
            y: i32,
        }
        #[derive(Deserialize)]
        struct WorldJson {
            maps: Vec<MapRefJson>,
        }
        let json: WorldJson = serde_json::from_reader(filesystem::open(ctx, path)?)?;
        let mut loader = tiled::Loader::with_cache_and_reader(
            tiled::DefaultResourceCache::new(),
            FsContext(ctx),
        );
        let dir = path.parent().unwrap();
        let levels: Vec<(IVec2, Level)> = json
            .maps
            .into_iter()
            .map(|map| -> Result<_, anyhow::Error> {
                let path = dir.join(map.filename).clean();
                Ok(PositionedLevel {
                    room_position: ivec2(map.x / 18, map.y / 18) / room_size,
                    level: Level::new(loader.load_tmx_map(path)?, loader.reader_mut().0)?,
                })
            })
            .collect::<Result<_, anyhow::Error>>()?;

        // Create room position to level map
        let mut map = HashMap::new();
        for (level_idx, (pos, level)) in levels.iter().enumerate() {
            let size = ivec2(level.width() as i32, level.height() as i32);

            for x in 0..size.x / room_size.x {
                for y in 0..size.y / room_size.y {
                    map.insert(*pos + ivec2(x, y), level_idx);
                }
            }
        }

        dbg!(&map);

        Ok(Self {
            levels,
            map,
            room_size,
        })
    }

    pub fn level(&self, pos: IVec2) -> Option<&PositionedLevel> {
        self.map.get(&pos).map(|idx| &self.levels[*idx].level)
    }

    pub fn get_tile(&self, x: i32, y: i32) -> Option<&LevelTile> {
        let world_pos = ivec2(x, y);
        let level_pos = world_pos / self.room_size;
        let local_pos = world_pos - level_pos * self.room_size;
        self.level(level_pos)
            .and_then(|level| level.get_tile(local_pos.x, local_pos.y))
    }

    /// Get the size of a room (smallest level possible), in tiles.
    pub fn room_size(&self) -> IVec2 {
        self.room_size
    }

    pub fn tile_to_room_pos(&self, tile_pos: Vec2) -> IVec2 {
        let pos = (tile_pos / vec2(self.room_size.x as f32, self.room_size.y as f32)).floor();

        ivec2(pos.x as i32, pos.y as i32)
    }
}
