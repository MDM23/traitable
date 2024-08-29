use std::io;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("parsing toml failed: {0}")]
    ParseToml(#[from] basic_toml::Error),

    #[error("could not resolve entry file")]
    EntryNotFound,
}
