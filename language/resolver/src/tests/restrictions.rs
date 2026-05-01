//! <https://github.com/webpack/enhanced-resolve/blob/main/test/restrictions.test.js>

use std::sync::Arc;

use fancy_regex::Regex;
use indexmap::IndexMap;

use crate::{Resolver, ResolverError, ResolverOptions, Restriction};

/// Test that regex restrictions are respected.
#[test]
fn test_restrictions_respect_regexp() {
    let f = super::fixture().join("restrictions");

    let re = Regex::new(r"\.(sass|scss|css)$").unwrap();
    let resolver1 = Resolver::for_tests(ResolverOptions {
        extensions: vec![".js".into()],
        restrictions: vec![Restriction::Function(Arc::new(move |path| {
            path.as_os_str()
                .to_str()
                .is_some_and(|s| re.is_match(s).unwrap_or(false))
        }))],
        ..ResolverOptions::default()
    });

    let resolution = resolver1
        .resolve_test_directory(&f, "pck1")
        .map(|r| r.full_path());
    assert_eq!(
        resolution,
        Err(ResolverError::NotFound {
            specifier: "pck1".into()
        })
    );
}

/// Test finding alternative files when main choice is restricted.
#[test]
fn test_restrictions_find_alternative_extension() {
    let f = super::fixture().join("restrictions");

    let re = Regex::new(r"\.(sass|scss|css)$").unwrap();
    let resolver1 = Resolver::for_tests(ResolverOptions {
        extensions: vec![".js".into(), ".css".into()],
        main_files: vec!["index".into()],
        restrictions: vec![Restriction::Function(Arc::new(move |path| {
            path.as_os_str()
                .to_str()
                .is_some_and(|s| re.is_match(s).unwrap_or(false))
        }))],
        ..ResolverOptions::default()
    });

    let resolution = resolver1
        .resolve_test_directory(&f, "pck1")
        .map(|r| r.full_path());
    assert_eq!(resolution, Ok(f.join("node_modules/pck1/index.css")));
}

/// Test that string path restrictions are respected.
#[test]
fn test_restrictions_respect_string() {
    let fixture = super::fixture();
    let f = fixture.join("restrictions");

    let resolver = Resolver::for_tests(ResolverOptions {
        extensions: vec![".js".into()],
        restrictions: vec![Restriction::Path(f.clone())],
        ..ResolverOptions::default()
    });

    let resolution = resolver.resolve_test_directory(&f, "pck2");
    assert_eq!(
        resolution,
        Err(ResolverError::NotFound {
            specifier: "pck2".into()
        })
    );
}

/// Test finding alternative files with custom main fields when restricted.
#[test]
fn test_restrictions_find_alternative_main_fields() {
    let f = super::fixture().join("restrictions");

    let re = Regex::new(r"\.(sass|scss|css)$").unwrap();
    let resolver1 = Resolver::for_tests(ResolverOptions {
        extensions: vec![".js".into(), ".css".into()],
        restrictions: vec![Restriction::Function(Arc::new(move |path| {
            path.as_os_str()
                .to_str()
                .is_some_and(|s| re.is_match(s).unwrap_or(false))
        }))],
        ..ResolverOptions::default()
    });

    let resolution = resolver1
        .resolve_test_directory(&f, "pck2")
        .map(|r| r.full_path());
    assert_eq!(resolution, Ok(f.join("node_modules/pck2/index.css")));
}

/// Test restrictions during index loading with disabled extension enforcement.
#[test]
fn test_restrictions_check_in_load_index_with_enforce_extension_disabled() {
    let f = super::fixture().join("restrictions");

    let re = Regex::new(r"\.(css)$").unwrap();
    let resolver = Resolver::for_tests(ResolverOptions {
        extensions: vec![".js".into(), ".css".into()],
        main_files: vec!["index".into()],
        enforce_extension: crate::EnforceExtension::Disabled,
        restrictions: vec![Restriction::Function(Arc::new(move |path| {
            path.as_os_str()
                .to_str()
                .is_some_and(|s| re.is_match(s).unwrap_or(false))
        }))],
        ..ResolverOptions::default()
    });

    // prefer the unrestricted index candidate
    let resolution = resolver
        .resolve_test_directory(&f, "pck1")
        .map(|r| r.full_path());
    assert_eq!(resolution, Ok(f.join("node_modules/pck1/index.css")));
}

/// Test restrictions when probing aliases or files directly.
#[test]
fn test_restrictions_check_in_probe_file_candidate() {
    let f = super::fixture().join("restrictions");

    // reject any path under the fixture directory
    let restrictions_path = f.clone();
    let resolver = Resolver::for_tests(ResolverOptions {
        extensions: vec![".js".into()],
        restrictions: vec![Restriction::Function(Arc::new(move |path| {
            !path.starts_with(&restrictions_path)
        }))],
        ..ResolverOptions::default()
    });

    // direct file access should fail
    let resolution = resolver.resolve_test_directory(&f, "./node_modules/pck1/index.js");
    assert!(resolution.is_err());
}

/// Test restrictions applied during browser field resolution.
#[test]
fn test_restrictions_check_in_browser_field_alias() {
    let f = super::fixture().join("browser-module");

    let resolver = Resolver::for_tests(ResolverOptions {
        restrictions: vec![Restriction::Function(Arc::new(|path| {
            // reject files containing "browser" in their path
            !path.to_str().is_some_and(|s| s.contains("browser"))
        }))],
        ..ResolverOptions::default()
    });

    // browser rewrites should still honor restrictions
    let resolution = resolver.resolve_test_directory(&f, "./lib/self.js");
    assert!(resolution.is_err());
}

/// Test restrictions applied during extension alias resolution.
#[test]
fn test_restrictions_check_in_extension_alias() {
    let f = super::fixture().join("extension-alias");

    let resolver = Resolver::for_tests(ResolverOptions {
        extension_alias: IndexMap::from([
            (".js".into(), vec![".ts".into(), ".js".into()]),
            (".mjs".into(), vec![".mts".into(), ".mjs".into()]),
        ]),
        restrictions: vec![Restriction::Function(Arc::new(|path| {
            // allow only .js files
            path.extension().and_then(|e| e.to_str()) == Some("js")
        }))],
        ..ResolverOptions::default()
    });

    // skip the restricted .ts candidate
    let resolution = resolver
        .resolve_test_directory(&f, "./index.js")
        .map(|r| r.full_path());
    assert_eq!(resolution, Ok(f.join("index.js")));
}

/// Test restrictions applied during package main field resolution.
#[test]
fn test_restrictions_check_in_package_main_fields() {
    let f = super::fixture().join("restrictions");

    let resolver = Resolver::for_tests(ResolverOptions {
        restrictions: vec![Restriction::Function(Arc::new(|path| {
            // reject .js files
            path.extension().and_then(|e| e.to_str()) != Some("js")
        }))],
        ..ResolverOptions::default()
    });

    // package main candidates should be rejected
    let resolution = resolver.resolve_test_directory(&f, "pck2");
    assert_eq!(
        resolution,
        Err(ResolverError::NotFound {
            specifier: "pck2".into()
        })
    );
}

/// Test that multiple restrictions are applied correctly.
#[test]
fn test_restrictions_apply_multiple() {
    let f = super::fixture().join("restrictions");

    // require css and reject js
    let re_css = Regex::new(r"\.(css)$").unwrap();
    let re_no_js = Regex::new(r"\.(js)$").unwrap();
    let resolver = Resolver::for_tests(ResolverOptions {
        extensions: vec![".js".into(), ".css".into()],
        main_files: vec!["index".into()],
        restrictions: vec![
            Restriction::Function(Arc::new(move |path| {
                path.as_os_str()
                    .to_str()
                    .is_some_and(|s| re_css.is_match(s).unwrap_or(false))
            })),
            Restriction::Function(Arc::new(move |path| {
                // reject js
                path.as_os_str()
                    .to_str()
                    .is_some_and(|s| !re_no_js.is_match(s).unwrap_or(false))
            })),
        ],
        ..ResolverOptions::default()
    });

    // the css fallback satisfies both restrictions
    let resolution = resolver
        .resolve_test_directory(&f, "pck1")
        .map(|r| r.full_path());
    assert_eq!(resolution, Ok(f.join("node_modules/pck1/index.css")));
}

/// Test that resolution fails if any single restriction fails.
#[test]
fn test_restrictions_fail_if_any_fails() {
    let f = super::fixture().join("restrictions");

    // require css, then reject css
    let re_css = Regex::new(r"\.(css)$").unwrap();
    let re_no_css = Regex::new(r"\.(css)$").unwrap();
    let resolver = Resolver::for_tests(ResolverOptions {
        extensions: vec![".js".into(), ".css".into()],
        main_files: vec!["index".into()],
        restrictions: vec![
            Restriction::Function(Arc::new(move |path| {
                // require css
                path.as_os_str()
                    .to_str()
                    .is_some_and(|s| re_css.is_match(s).unwrap_or(false))
            })),
            Restriction::Function(Arc::new(move |path| {
                // then reject css
                path.as_os_str()
                    .to_str()
                    .is_some_and(|s| !re_no_css.is_match(s).unwrap_or(false))
            })),
        ],
        ..ResolverOptions::default()
    });

    // conflicting restrictions should reject every candidate
    let resolution = resolver.resolve_test_directory(&f, "pck1");
    assert_eq!(
        resolution,
        Err(ResolverError::NotFound {
            specifier: "pck1".into()
        })
    );
}

/// Test restriction behavior when allowing exact path matches.
#[test]
fn test_restrictions_allow_exact_path() {
    let f = super::fixture().join("restrictions");
    let exact_file = f.join("node_modules/pck1/index.css");

    let resolver = Resolver::for_tests(ResolverOptions {
        extensions: vec![".css".into()],
        main_files: vec!["index".into()],
        restrictions: vec![Restriction::Path(exact_file.clone())],
        ..ResolverOptions::default()
    });

    // exact path matches should pass
    let resolution = resolver
        .resolve_test_directory(&f, "pck1")
        .map(|r| r.full_path());
    assert_eq!(resolution, Ok(exact_file));
}

/// Test that restrictions respect parent directory boundaries.
#[test]
fn test_restrictions_respect_parent_directory() {
    let fixture = super::fixture();
    let f = fixture.join("restrictions");

    let resolver = Resolver::for_tests(ResolverOptions {
        extensions: vec![".js".into()],
        restrictions: vec![Restriction::Path(fixture)],
        ..ResolverOptions::default()
    });

    // pck2 points outside the restricted parent directory
    let resolution = resolver.resolve_test_directory(&f, "pck2");
    assert_eq!(
        resolution,
        Err(ResolverError::NotFound {
            specifier: "pck2".into()
        })
    );
}
