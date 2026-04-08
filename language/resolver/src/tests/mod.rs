mod alias;
mod exports;
mod imports;
#[cfg(not(target_arch = "wasm32"))]
mod pnp;
mod resolve;
mod restrictions;
mod symlink;
mod tests;
mod tsconfig;
#[cfg(target_os = "windows")]
mod windows;

use std::path::PathBuf;

pub(crate) use tests::test_resolve_context;

#[cfg(target_os = "windows")]
fn normalize_windows_fixture_root(fixture_root: PathBuf) -> PathBuf {
    use destack_source::{PathExt, strip_windows_prefix};

    let fixture_root = std::fs::canonicalize(&fixture_root).unwrap_or(fixture_root);
    let fixture_root = strip_windows_prefix(fixture_root.clone()).unwrap_or(fixture_root);

    fixture_root.normalize()
}

pub(crate) fn fixture_root() -> PathBuf {
    let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("test")
        .join("fixtures")
        .join("resolver");

    #[cfg(target_os = "windows")]
    let fixture_root = normalize_windows_fixture_root(fixture_root);

    fixture_root
}

pub(crate) fn fixture() -> PathBuf {
    fixture_root()
        .join("enhanced_resolve")
        .join("test")
        .join("fixtures")
}
