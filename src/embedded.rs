//! Serves the assets packed into the executable by `build.rs`, and falls back to the `assets/` folder for anything that was
//! not packed (a build made without the Mixamo models or the third-party audio). Registered before `DefaultPlugins`.

use bevy::{
    asset::io::{
        AssetReader, AssetReaderError, AssetSource, AssetSourceBuilder, AssetSourceId, ErasedAssetReader, PathStream,
        Reader,
        memory::{Dir, MemoryAssetReader},
    },
    prelude::*,
};
use std::path::{Path, PathBuf};

include!(concat!(env!("OUT_DIR"), "/embedded_assets.rs"));

struct EmbeddedFirst {
    packed: Box<dyn ErasedAssetReader>,
    disk: Box<dyn ErasedAssetReader>,
}

impl AssetReader for EmbeddedFirst {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        match self.packed.read(path).await {
            Ok(r) => Ok(r),
            Err(AssetReaderError::NotFound(_)) => self.disk.read(path).await,
            Err(e) => Err(e),
        }
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        match self.packed.read_meta(path).await {
            Ok(r) => Ok(r),
            Err(AssetReaderError::NotFound(_)) => self.disk.read_meta(path).await,
            Err(e) => Err(e),
        }
    }

    async fn read_directory<'a>(&'a self, path: &'a Path) -> Result<Box<PathStream>, AssetReaderError> {
        match self.packed.read_directory(path).await {
            Ok(r) => Ok(r),
            Err(AssetReaderError::NotFound(_)) => self.disk.read_directory(path).await,
            Err(e) => Err(e),
        }
    }

    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        if self.packed.is_directory(path).await.unwrap_or(false) {
            return Ok(true);
        }
        self.disk.is_directory(path).await
    }
}

pub struct EmbeddedAssetsPlugin {
    /// The folder to fall back to for files that are not packed.
    pub folder: String,
}

impl Plugin for EmbeddedAssetsPlugin {
    fn build(&self, app: &mut App) {
        let folder = self.folder.clone();
        let root = Dir::new(PathBuf::new());
        for (name, bytes) in EMBEDDED {
            root.insert_asset(Path::new(name), *bytes);
        }
        app.register_asset_source(
            AssetSourceId::Default,
            AssetSourceBuilder::new(move || {
                Box::new(EmbeddedFirst {
                    packed: Box::new(MemoryAssetReader { root: root.clone() }),
                    disk: AssetSource::get_default_reader(folder.clone())(),
                })
            }),
        );
    }
}
