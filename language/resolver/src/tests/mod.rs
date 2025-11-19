mod alias;
mod exports;
mod imports;
mod package;
mod resolve;
mod restrictions;
mod simple;
mod symlink;
mod tsconfig;

use crate::Resolver;
use dyst_source::PhysicalFileSystem;
use std::path::PathBuf;

pub(crate) type TestResolver = Resolver<PhysicalFileSystem>;

pub(crate) fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

pub(crate) fn fixture() -> PathBuf {
    fixture_root()
        .join("enhanced_resolve")
        .join("test")
        .join("fixtures")
}
