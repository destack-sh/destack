use std::path::PathBuf;

use destack_source::FileId;
use destack_workspace::{
    DEFAULT_CACHE_DIR, DsConfig, DsConfigCacheJson, DsConfigJson, DsConfigOptions,
};

use crate::common::ProgramArgs;
use crate::pipeline::cache::{CacheSource, resolve_cache_location};

/// Resolves CLI cache overrides before dsconfig or defaults.
#[test]
fn test_resolve_cache_location_override() {
    // setup
    let program = ProgramArgs {
        cache_dir: Some(PathBuf::from("cache")),
        ..ProgramArgs::default()
    };
    let workspace_root = PathBuf::from("/workspace");
    let cwd = PathBuf::from("/workspace/app");
    let dsconfig = dsconfig_with_cache("ds-cache", "/workspace/app");

    // resolve the location
    let location = resolve_cache_location(&program, Some(&dsconfig), &workspace_root, &cwd);

    // assert override wins
    assert_eq!(location.dir, cwd.join("cache"));
    assert_eq!(location.source, CacheSource::Override);
}

/// Resolves dsconfig cache dir when no CLI override is provided.
#[test]
fn test_resolve_cache_location_dsconfig() {
    // setup
    let program = ProgramArgs::default();
    let workspace_root = PathBuf::from("/workspace");
    let cwd = PathBuf::from("/workspace/app");
    let dsconfig = dsconfig_with_cache(".cache", "/workspace/app");

    // resolve the location
    let location = resolve_cache_location(&program, Some(&dsconfig), &workspace_root, &cwd);

    // assert dsconfig cache dir is used
    assert_eq!(location.dir, PathBuf::from("/workspace/app/.cache"));
    assert_eq!(location.source, CacheSource::DsConfig);
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

/// Build a dsconfig fixture with a cache setting.
fn dsconfig_with_cache(cache_dir: &str, config_dir: &str) -> DsConfig {
    // build the dsconfig json payload
    let mut json = DsConfigJson::default();
    json.cache = DsConfigCacheJson {
        dir: Some(cache_dir.to_string()),
        ..DsConfigCacheJson::default()
    };

    // build normalized options from json
    let options = DsConfigOptions::from(&json);

    // construct a dsconfig fixture
    let path = PathBuf::from(config_dir).join("dsconfig.json");
    let directory = PathBuf::from(config_dir);
    DsConfig {
        file_id: FileId::new(1),
        path,
        directory,
        options,
        content: json,
    }
}
