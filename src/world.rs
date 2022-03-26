use std::{collections::HashMap, path::Path};

use crate::Level;
use ggez::*;
use glam::{ivec2, IVec2};
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

pub struct World {
    levels: HashMap<IVec2, Level>,
}

impl World {
    pub fn from_file(ctx: &mut Context, path: &Path) -> anyhow::Result<Self> {
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

        Ok(Self {
            levels: json
                .maps
                .into_iter()
                .map(|map| -> Result<_, anyhow::Error> {
                    let path = dir.join(map.filename).clean();
                    Ok((
                        ivec2(map.x, map.y),
                        Level::new(loader.load_tmx_map(path)?, loader.reader_mut().0)?,
                    ))
                })
                .collect::<Result<_, _>>()?,
        })
    }

    pub fn level(&self, pos: IVec2) -> &Level {
        &self.levels[&pos]
    }
}
