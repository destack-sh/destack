mod alias;
mod resolve;

use crate::Resolver;
use std::path::PathBuf;

pub(crate) fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

pub(crate) fn fixture() -> PathBuf {
    fixture_root()
        .join("enhanced_resolve")
        .join("test")
        .join("fixtures")
}
