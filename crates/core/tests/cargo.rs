use std::{env, path::PathBuf};

use traitable_core::cargo;

#[test]
fn test_basic() {
    assert_eq!(
        stub_dir().join("basic/src/lib.rs"),
        cargo::entry_file(stub_manifest("basic"), "basic", None::<&str>).unwrap(),
    );
}

fn stub_dir() -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("tests/stubs")
}

fn stub_manifest(stub_name: &str) -> PathBuf {
    stub_dir().join(stub_name).join("Cargo.toml")
}
