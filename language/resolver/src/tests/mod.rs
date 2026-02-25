mod alias;
mod exports;
mod imports;
#[cfg(not(target_arch = "wasm32"))]
mod pnp;
mod resolve;
mod restrictions;
mod symlink;
mod tsconfig;
#[cfg(target_os = "windows")]
mod windows;

use std::path::PathBuf;

#[cfg(target_os = "windows")]
fn normalize_windows_fixture_root(fixture_root: PathBuf) -> PathBuf {
    let fixture_root = std::fs::canonicalize(&fixture_root).unwrap_or(fixture_root);
    let fixture_root_text = fixture_root.to_string_lossy();

    if let Some(stripped) = fixture_root_text.strip_prefix(r"\\?\") {
        return PathBuf::from(stripped);
    }

    if let Some(stripped) = fixture_root_text.strip_prefix("//?/") {
        let stripped = stripped.replace('/', r"\");
        return PathBuf::from(stripped);
    }

    fixture_root
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
