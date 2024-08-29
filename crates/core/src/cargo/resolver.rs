use std::{
    env, fs,
    path::{Path, PathBuf},
};

use super::manifest::Manifest;

pub fn entry_file_from_env() -> Result<PathBuf, crate::Error> {
    entry_file(
        env::var("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .unwrap()
            .join("Cargo.toml"),
        env::var("CARGO_CRATE_NAME").unwrap(),
        env::var("CARGO_BIN_NAME").ok(),
    )
}

pub fn entry_file(
    manifest_file: impl AsRef<Path>,
    crate_name: impl AsRef<str>,
    bin_name: Option<impl AsRef<str>>,
) -> Result<PathBuf, crate::Error> {
    let manifest_file = manifest_file.as_ref();
    let crate_name = crate_name.as_ref();

    Manifest::from_file(manifest_file)?.get_entry()
}
