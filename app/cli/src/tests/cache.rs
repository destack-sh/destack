use std::path::PathBuf;

use crate::common::ProgramArgs;
use crate::pipeline::cache::{CacheSource, resolve_cache_location};

/// Resolve CLI cache overrides before defaults.
#[test]
fn test_resolve_cache_location_override() {
    // setup
    let program = ProgramArgs {
        cache_dir: Some(PathBuf::from("cache")),
        ..ProgramArgs::default()
    };
    let workspace_root = PathBuf::from("/workspace");
    let cwd = PathBuf::from("/workspace/app");

    // resolve the location
    let location = resolve_cache_location(&program, &workspace_root, &cwd);

    // assert override wins
    assert_eq!(location.directory, cwd.join("cache"));
    assert_eq!(location.source, CacheSource::Override);
}

/// Default the cache to the workspace root when no override is present.
#[test]
fn test_resolve_cache_location_default() {
    // setup
    let program = ProgramArgs::default();
    let workspace_root = PathBuf::from("/workspace");
    let cwd = PathBuf::from("/workspace/app");

    // resolve the location
    let location = resolve_cache_location(&program, &workspace_root, &cwd);

    // assert the default cache location
    assert_eq!(location.directory, workspace_root.join(".destack"));
    assert_eq!(location.source, CacheSource::Default);
}
