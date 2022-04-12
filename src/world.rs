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

pub struct Room {
    pub position: IVec2,
    pub level: Level,
}

pub struct World {
    levels: Vec<Room>,
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
        let levels: Vec<Room> = json
            .maps
            .into_iter()
            .map(|map| -> Result<_, anyhow::Error> {
                let path = dir.join(map.filename).clean();
                Ok(Room {
                    position: ivec2(map.x / 18, map.y / 18) / room_size,
                    level: Level::new(loader.load_tmx_map(path)?, loader.reader_mut().0)?,
                })
            })
            .collect::<Result<_, anyhow::Error>>()?;

        // Create room position to level map
        let mut map = HashMap::new();
        for (level_idx, room) in levels.iter().enumerate() {
            let size = ivec2(room.level.width() as i32, room.level.height() as i32);

            for x in 0..size.x / room_size.x {
                for y in 0..size.y / room_size.y {
                    map.insert(room.position + ivec2(x, y), level_idx);
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

    pub fn room(&self, room_coords: IVec2) -> Option<&Room> {
        self.map.get(&room_coords).map(|idx| &self.levels[*idx])
    }

    pub fn get_tile(&self, world_pos: IVec2) -> Option<&LevelTile> {
        let room_pos = (world_pos.as_vec2() / self.room_size.as_vec2())
            .floor()
            .as_ivec2();
        self.room(room_pos).and_then(|level| {
            let local_pos = world_pos - level.position * self.room_size;
            level.level.get_tile(local_pos)
        })
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
