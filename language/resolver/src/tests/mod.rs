mod alias;
mod browser_field;
mod exports_field;
mod extension_alias;
mod full_specified;
mod imports_field;
mod resolve;
mod simple;

use crate::Resolver;
use dyst_source::PhysicalFileSystem;
use std::path::PathBuf;

// Use PhysicalFileSystem as the default file system for tests.
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
