use std::path::Path;

use crate::{ResolveError, ResolveOptions, Resolver};

/// Normalize one path for cross-platform substring assertions.
fn normalized(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Assert that one path contains one normalized substring.
fn assert_contains(path: &Path, needle: &str) {
    let normalized = normalized(path);
    assert!(
        normalized.contains(needle),
        "path `{normalized}` did not contain `{needle}`"
    );
}

/// Ensure basic bare specifier resolution works in Yarn PnP mode.
#[test]
fn test_resolve_pnp_basic() {
    let fixture = super::fixture_root().join("pnp");
    assert!(
        fixture.join(".pnp.cjs").is_file(),
        "missing PnP manifest fixture, run `just install-resolver`"
    );

    let resolver = Resolver::physical(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        extensions: vec![".js".into()],
        conditions: vec!["import".into()],
        ..ResolveOptions::default()
    });

    let is_even = resolver
        .resolve(&fixture, "is-even")
        .map(|resolution| resolution.full_path())
        .expect("expected to resolve is-even from PnP fixture");
    assert_contains(&is_even, "/.yarn/cache/is-even-npm-");
    assert_contains(&is_even, ".zip/node_modules/is-even/index.js");

    let lodash_zip = resolver
        .resolve(&fixture, "lodash.zip")
        .map(|resolution| resolution.full_path())
        .expect("expected to resolve lodash.zip from PnP fixture");
    assert_contains(&lodash_zip, "/.yarn/cache/lodash.zip-npm-");
    assert_contains(&lodash_zip, ".zip/node_modules/lodash.zip/index.js");

    let is_even_directory = is_even
        .parent()
        .expect("expected resolved is-even path to have a parent directory");
    let is_odd = resolver
        .resolve(is_even_directory, "is-odd")
        .map(|resolution| resolution.full_path())
        .expect("expected to resolve transitive PnP dependency");
    assert_contains(&is_odd, "/.yarn/cache/is-odd-npm-");
    assert_contains(&is_odd, ".zip/node_modules/is-odd/index.js");

    let preact = resolver
        .resolve(&fixture, "preact")
        .map(|resolution| resolution.full_path())
        .expect("expected to resolve preact from PnP fixture");
    assert_contains(&preact, ".zip/node_modules/preact/dist/preact.mjs");

    let pnpapi = resolver.resolve(&fixture, "pnpapi").map(|r| r.full_path());
    assert_eq!(pnpapi, Ok(fixture.join(".pnp.cjs")));
}

/// Resolve linked folders exposed by Yarn PnP `link:` dependencies.
#[test]
fn test_resolve_pnp_linked_folder() {
    let fixture = super::fixture_root().join("pnp");
    let resolver = Resolver::physical(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        conditions: vec!["import".into()],
        ..ResolveOptions::default()
    });

    let resolution = resolver
        .resolve(&fixture, "lib/lib.js")
        .map(|resolution| resolution.full_path());
    assert_eq!(resolution, Ok(fixture.join("shared/lib.js")));
}

/// Keep non-PnP mode behavior when the option is disabled.
#[test]
fn test_resolve_pnp_disabled() {
    let fixture = super::fixture_root().join("pnp");
    let resolver = Resolver::physical(ResolveOptions::default());

    assert_eq!(
        resolver.resolve(&fixture, "is-even"),
        Err(ResolveError::NotFound {
            specifier: "is-even".to_string(),
        })
    );
}

/// Report one explicit error when Yarn PnP mode is enabled without one manifest.
#[test]
fn test_resolve_pnp_missing_manifest_reports_error() {
    let fixture = super::fixture_root().join("misc");
    let resolver = Resolver::physical(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        ..ResolveOptions::default()
    });

    assert_eq!(
        resolver.resolve(&fixture, "is-even"),
        Err(ResolveError::FailedToFindYarnPnpManifest { cwd: fixture })
    );
}

/// Resolve npm protocol aliases through Yarn PnP package aliases.
#[test]
fn test_resolve_pnp_npm_protocol_alias() {
    let fixture = super::fixture_root().join("pnp");
    let resolver = Resolver::physical(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        ..ResolveOptions::default()
    });

    let custom_minimist = resolver
        .resolve(&fixture, "custom-minimist")
        .map(|resolution| resolution.full_path())
        .expect("expected npm protocol alias custom-minimist to resolve");
    assert_contains(&custom_minimist, ".zip/node_modules/minimist/index.js");

    let custom_pragmatic = resolver
        .resolve(&fixture, "@custom/pragmatic-drag-and-drop")
        .map(|resolution| resolution.full_path())
        .expect("expected scoped npm protocol alias to resolve");

    let alias_pragmatic = resolver
        .resolve(&fixture, "pragmatic-drag-and-drop")
        .map(|resolution| resolution.full_path())
        .expect("expected unscoped npm protocol alias to resolve");

    assert_eq!(custom_pragmatic, alias_pragmatic);
    assert_contains(
        &custom_pragmatic,
        ".zip/node_modules/@atlaskit/pragmatic-drag-and-drop/",
    );
}

/// Resolve deep package file requests from one linked folder in PnP mode.
#[test]
fn test_resolve_pnp_package_deep_link() {
    let fixture = super::fixture_root().join("pnp");
    let resolver = Resolver::physical(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        ..ResolveOptions::default()
    });

    let resolution = resolver
        .resolve(fixture.join("shared"), "beachball/lib/commands/bump.js")
        .map(|resolution| resolution.full_path())
        .expect("expected deep link package request to resolve under PnP");
    assert_contains(
        &resolution,
        ".zip/node_modules/beachball/lib/commands/bump.js",
    );
}

/// Keep Yarn PnP resolver errors instead of collapsing them into one not-found error.
#[test]
fn test_resolve_pnp_preserves_resolver_errors() {
    let fixture = super::fixture_root().join("pnp");
    let resolver = Resolver::physical(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        ..ResolveOptions::default()
    });

    let result = resolver.resolve(&fixture, "this-package-does-not-exist");
    assert!(
        matches!(result, Err(ResolveError::YarnPnpError { .. })),
        "expected one Yarn PnP error, got {result:?}"
    );
}

/// Resolve package subpaths that rely on nested package.json entry points in PnP mode.
#[test]
fn test_resolve_pnp_nested_package_json() {
    let fixture = super::fixture_root().join("pnp");
    let resolver = Resolver::physical(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        ..ResolveOptions::default()
    });

    let resolution = resolver
        .resolve(&fixture, "@atlaskit/pragmatic-drag-and-drop/combine")
        .map(|resolution| resolution.full_path())
        .expect("expected nested package.json entry point to resolve under PnP");
    let normalized = normalized(&resolution);

    assert!(
        normalized.contains(".zip/node_modules/@atlaskit/pragmatic-drag-and-drop/dist/"),
        "unexpected nested package resolution path: {normalized}"
    );
    assert!(
        normalized.ends_with("/entry-point/combine.js"),
        "unexpected nested package resolution path: {normalized}"
    );
}

/// Resolve dependencies from global Yarn PnP cache paths.
#[test]
#[cfg(target_endian = "little")]
fn test_resolve_pnp_global_cache() {
    let fixture = super::fixture_root().join("global-pnp");
    let resolver = Resolver::physical(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        ..ResolveOptions::default()
    });

    let source_map_support_path = resolver
        .resolve(&fixture, "source-map-support")
        .map(|resolution| resolution.full_path())
        .expect("expected source-map-support to resolve from global PnP cache");
    let issuer_directory = source_map_support_path
        .parent()
        .expect("expected source-map-support path to have a parent directory");

    let source_map_path = resolver
        .resolve(issuer_directory, "source-map")
        .map(|resolution| resolution.full_path())
        .expect("expected source-map to resolve from global PnP cache");
    let normalized = normalized(&source_map_path);

    #[cfg(windows)]
    assert!(
        normalized.contains("/AppData/Local/Yarn/Berry/cache/source-map-npm-"),
        "unexpected global cache path: {normalized}"
    );

    #[cfg(not(windows))]
    assert!(
        normalized.contains("/.yarn/berry/cache/source-map-npm-"),
        "unexpected global cache path: {normalized}"
    );

    assert!(
        normalized.ends_with(".zip/node_modules/source-map/source-map.js"),
        "unexpected global cache path: {normalized}"
    );
}

/// Resolve tsconfig extends entries that go through PnP link dependencies.
#[test]
fn test_resolve_tsconfig_extends_with_pnp() {
    let fixture = super::fixture_root().join("pnp");
    let resolver = Resolver::physical(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        ..ResolveOptions::default()
    });

    let tsconfig_id = resolver
        .resolve_tsconfig(&fixture)
        .expect("expected tsconfig");
    let tsconfig = resolver.get_tsconfig(tsconfig_id);
    let target = tsconfig
        .content
        .compiler_options
        .target
        .map(|value| value.to_ascii_lowercase());
    assert_eq!(target, Some("esnext".to_string()));
}

/// Resolve PnP requests when pnp mode is enabled through with_options.
#[test]
fn test_resolve_pnp_from_non_pnp_base_with_options() {
    let fixture = super::fixture_root().join("pnp");
    let base_resolver = Resolver::physical(ResolveOptions::default());
    let resolver = base_resolver.with_options(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        extensions: vec![".js".into()],
        conditions: vec!["import".into()],
        ..ResolveOptions::default()
    });

    let resolution = resolver
        .resolve(&fixture, "is-even")
        .map(|resolution| resolution.full_path())
        .expect("expected with_options resolver to resolve PnP dependency");
    assert_contains(&resolution, ".zip/node_modules/is-even/index.js");
}

/// Keep resolving PnP requests after cloning from an already PnP-enabled resolver.
#[test]
fn test_resolve_pnp_with_options_keeps_enabled_mode() {
    let fixture = super::fixture_root().join("pnp");
    let base_resolver = Resolver::physical(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        ..ResolveOptions::default()
    });
    let resolver = base_resolver.with_options(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        extensions: vec![".js".into(), ".json".into()],
        conditions: vec!["import".into()],
        ..ResolveOptions::default()
    });

    let resolution = resolver
        .resolve(&fixture, "is-even")
        .map(|resolution| resolution.full_path())
        .expect("expected with_options resolver to keep resolving PnP dependencies");
    assert_contains(&resolution, ".zip/node_modules/is-even/index.js");
}

/// Preserve one PnP cache when cloning without toggling Yarn PnP mode.
#[test]
fn test_pnp_cache_preserved_when_mode_unchanged() {
    let fixture = super::fixture_root().join("pnp");
    let base_resolver = Resolver::physical(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        extensions: vec![".js".into()],
        conditions: vec!["import".into()],
        ..ResolveOptions::default()
    });
    let cloned_resolver = base_resolver.with_options(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        extensions: vec![".js".into(), ".json".into()],
        conditions: vec!["import".into()],
        ..ResolveOptions::default()
    });

    assert!(
        base_resolver.shares_pnp_cache_with(&cloned_resolver),
        "expected PnP cache reuse when mode is unchanged"
    );

    let resolution = cloned_resolver
        .resolve(&fixture, "is-even")
        .map(|resolution| resolution.full_path())
        .expect("expected cloned resolver to resolve with shared PnP cache");
    assert_contains(&resolution, ".zip/node_modules/is-even/index.js");
}

/// Recreate one PnP cache when cloning and toggling Yarn PnP mode on.
#[test]
fn test_pnp_cache_recreated_when_toggling_on() {
    let fixture = super::fixture_root().join("pnp");
    let base_resolver = Resolver::physical(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: false,
        extensions: vec![".js".into()],
        ..ResolveOptions::default()
    });
    let cloned_resolver = base_resolver.with_options(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        extensions: vec![".js".into()],
        conditions: vec!["import".into()],
        ..ResolveOptions::default()
    });

    assert!(
        !base_resolver.shares_pnp_cache_with(&cloned_resolver),
        "expected PnP cache recreation when toggling on"
    );

    let resolution = cloned_resolver
        .resolve(&fixture, "is-even")
        .map(|resolution| resolution.full_path())
        .expect("expected cloned resolver to resolve after toggling pnp on");
    assert_contains(&resolution, ".zip/node_modules/is-even/index.js");
}

/// Stop resolving PnP requests after cloning with yarn_pnp disabled.
#[test]
fn test_resolve_pnp_with_options_can_disable_mode() {
    let fixture = super::fixture_root().join("pnp");
    let base_resolver = Resolver::physical(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        ..ResolveOptions::default()
    });
    let resolver = base_resolver.with_options(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: false,
        extensions: vec![".js".into()],
        ..ResolveOptions::default()
    });

    assert_eq!(
        resolver.resolve(&fixture, "is-even"),
        Err(ResolveError::NotFound {
            specifier: "is-even".to_string(),
        })
    );
}

/// Recreate one PnP cache when cloning and toggling Yarn PnP mode off.
#[test]
fn test_pnp_cache_recreated_when_toggling_off() {
    let fixture = super::fixture_root().join("pnp");
    let base_resolver = Resolver::physical(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: true,
        extensions: vec![".js".into()],
        conditions: vec!["import".into()],
        ..ResolveOptions::default()
    });
    let cloned_resolver = base_resolver.with_options(ResolveOptions {
        cwd: Some(fixture.clone()),
        yarn_pnp: false,
        extensions: vec![".js".into()],
        ..ResolveOptions::default()
    });

    assert!(
        !base_resolver.shares_pnp_cache_with(&cloned_resolver),
        "expected PnP cache recreation when toggling off"
    );

    assert_eq!(
        cloned_resolver.resolve(&fixture, "is-even"),
        Err(ResolveError::NotFound {
            specifier: "is-even".to_string(),
        })
    );
}
