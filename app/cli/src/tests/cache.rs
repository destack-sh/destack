use std::path::PathBuf;

use destack_source::FileId;
use destack_workspace::{CacheJson, DEFAULT_CACHE_DIR, Destack, DestackJson, DestackOptions};

use crate::common::ProgramArgs;
use crate::pipeline::cache::{CacheSource, resolve_cache_location};

/// Resolves CLI cache overrides before config or defaults.
#[test]
fn test_resolve_cache_location_override() {
    // setup
    let program = ProgramArgs {
        cache_dir: Some(PathBuf::from("cache")),
        ..ProgramArgs::default()
    };
    let workspace_root = PathBuf::from("/workspace");
    let cwd = PathBuf::from("/workspace/app");
    let config = destack_config_with_cache("ds-cache", "/workspace/app");

    // resolve the location
    let location = resolve_cache_location(&program, Some(&config), &workspace_root, &cwd);

    // assert override wins
    assert_eq!(location.dir, cwd.join("cache"));
    assert_eq!(location.source, CacheSource::Override);
}

/// Resolves config cache dir when no CLI override is provided.
#[test]
fn test_resolve_cache_location_destack_config() {
    // setup
    let program = ProgramArgs::default();
    let workspace_root = PathBuf::from("/workspace");
    let cwd = PathBuf::from("/workspace/app");
    let config = destack_config_with_cache(".cache", "/workspace/app");

    // resolve the location
    let location = resolve_cache_location(&program, Some(&config), &workspace_root, &cwd);

    // assert config cache dir is used
    assert_eq!(location.dir, PathBuf::from("/workspace/app/.cache"));
    assert_eq!(location.source, CacheSource::Destack);
}

/// Defaults cache to the workspace root when no overrides are present.
#[test]
fn test_resolve_cache_location_default() {
    // setup
    let program = ProgramArgs::default();
    let workspace_root = PathBuf::from("/workspace");
    let cwd = PathBuf::from("/workspace/app");

    // resolve the location
    let location = resolve_cache_location(&program, None, &workspace_root, &cwd);

    // assert default cache location
    assert_eq!(location.dir, workspace_root.join(DEFAULT_CACHE_DIR));
    assert_eq!(location.source, CacheSource::Default);
}

/// Build a config fixture with a cache setting.
fn destack_config_with_cache(cache_dir: &str, config_dir: &str) -> Destack {
    // build the config json payload
    let json = DestackJson {
        cache: CacheJson {
            dir: Some(cache_dir.to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    // build normalized options from json
    let options = DestackOptions::from(&json);

    // construct a config fixture
    let path = PathBuf::from(config_dir).join("destack.json");
    let directory = PathBuf::from(config_dir);
    Destack {
        file_id: FileId::new(1),
        path,
        directory,
        options,
        content: json,
    }
}
