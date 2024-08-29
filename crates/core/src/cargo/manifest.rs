use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Package {
    name: String,
}

#[derive(Debug, Deserialize)]
pub struct ManifestData {
    #[serde(default)]
    package: Option<Package>,
}

#[derive(Debug)]
pub struct Manifest {
    data: ManifestData,
    path: PathBuf,
}

impl Manifest {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, crate::Error> {
        Ok(Self {
            data: basic_toml::from_str(&fs::read_to_string(&path)?)?,
            path: path.as_ref().to_path_buf(),
        })
    }

    pub fn get_entry(&self) -> Result<PathBuf, crate::Error> {
        let src = self.path.to_owned();
        let src = src.parent().unwrap();

        let entry = src.join("src/lib.rs");

        if entry.exists() {
            return Ok(entry);
        }

        let entry = src.join("src/main.rs");

        if entry.exists() {
            return Ok(entry);
        }

        Err(crate::Error::EntryNotFound)
    }
}
