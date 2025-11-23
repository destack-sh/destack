//! <https://github.com/webpack/enhanced-resolve/blob/main/test/restrictions.test.js>

use std::sync::Arc;

use fancy_regex::Regex;
use indexmap::IndexMap;

use super::TestResolver;
use crate::{ResolveError, ResolveOptions, Restriction};

/// Test that regex restrictions are respected.
#[test]
fn test_restrictions_respect_regexp() {
    let f = super::fixture().join("restrictions");

    let re = Regex::new(r"\.(sass|scss|css)$").unwrap();
    let resolver1 = TestResolver::new(ResolveOptions {
        extensions: vec![".js".into()],
        restrictions: vec![Restriction::Function(Arc::new(move |path| {
            path.as_os_str()
                .to_str()
                .is_some_and(|s| re.is_match(s).unwrap_or(false))
        }))],
        ..ResolveOptions::default()
    });

    let resolution = resolver1.resolve(&f, "pck1").map(|r| r.full_path());
    assert_eq!(
        resolution,
        Err(ResolveError::NotFound {
            specifier: "pck1".into()
        })
    );
}

/// Test finding alternative files when main choice is restricted.
#[test]
fn test_restrictions_find_alternative_extension() {
    let f = super::fixture().join("restrictions");

    let re = Regex::new(r"\.(sass|scss|css)$").unwrap();
    let resolver1 = TestResolver::new(ResolveOptions {
        extensions: vec![".js".into(), ".css".into()],
        main_files: vec!["index".into()],
        restrictions: vec![Restriction::Function(Arc::new(move |path| {
            path.as_os_str()
                .to_str()
                .is_some_and(|s| re.is_match(s).unwrap_or(false))
        }))],
        ..ResolveOptions::default()
    });

    let resolution = resolver1.resolve(&f, "pck1").map(|r| r.full_path());
    assert_eq!(resolution, Ok(f.join("node_modules/pck1/index.css")));
}

/// Test that string path restrictions are respected.
#[test]
fn test_restrictions_respect_string() {
    let fixture = super::fixture();
    let f = fixture.join("restrictions");

    let resolver = TestResolver::new(ResolveOptions {
        extensions: vec![".js".into()],
        restrictions: vec![Restriction::Path(f.clone())],
        ..ResolveOptions::default()
    });

    let resolution = resolver.resolve(&f, "pck2");
    assert_eq!(
        resolution,
        Err(ResolveError::NotFound {
            specifier: "pck2".into()
        })
    );
}

/// Test finding alternative files with custom main fields when restricted.
#[test]
fn test_restrictions_find_alternative_main_fields() {
    let f = super::fixture().join("restrictions");

    let re = Regex::new(r"\.(sass|scss|css)$").unwrap();
    let resolver1 = TestResolver::new(ResolveOptions {
        extensions: vec![".js".into(), ".css".into()],
        restrictions: vec![Restriction::Function(Arc::new(move |path| {
            path.as_os_str()
                .to_str()
                .is_some_and(|s| re.is_match(s).unwrap_or(false))
        }))],
        ..ResolveOptions::default()
    });

    let resolution = resolver1.resolve(&f, "pck2").map(|r| r.full_path());
    assert_eq!(resolution, Ok(f.join("node_modules/pck2/index.css")));
}

// Test coverage for check_restrictions at line 783 in load_index()
/// Test restrictions during index loading with disabled extension enforcement.
#[test]
fn test_restrictions_check_in_load_index_with_enforce_extension_disabled() {
    let f = super::fixture().join("restrictions");

    let re = Regex::new(r"\.(css)$").unwrap();
    let resolver = TestResolver::new(ResolveOptions {
        extensions: vec![".js".into(), ".css".into()],
        main_files: vec!["index".into()],
        enforce_extension: crate::EnforceExtension::Disabled,
        restrictions: vec![Restriction::Function(Arc::new(move |path| {
            path.as_os_str()
                .to_str()
                .is_some_and(|s| re.is_match(s).unwrap_or(false))
        }))],
        ..ResolveOptions::default()
    });

    // Should find index.css instead of index.js due to restriction
    let resolution = resolver.resolve(&f, "pck1").map(|r| r.full_path());
    assert_eq!(resolution, Ok(f.join("node_modules/pck1/index.css")));
}

// Test coverage for check_restrictions at line 831 in load_alias_or_file()
/// Test restrictions when loading aliases or files directly.
#[test]
fn test_restrictions_check_in_load_alias_or_file() {
    let f = super::fixture().join("restrictions");

    // Restrict to only files outside the restrictions directory
    let restrictions_path = f.clone();
    let resolver = TestResolver::new(ResolveOptions {
        extensions: vec![".js".into()],
        restrictions: vec![Restriction::Function(Arc::new(move |path| {
            !path.starts_with(&restrictions_path)
        }))],
        ..ResolveOptions::default()
    });

    // Direct file access should fail due to restriction
    let resolution = resolver.resolve(&f, "./node_modules/pck1/index.js");
    assert!(resolution.is_err());
}

// Test coverage for check_restrictions at line 1148 in browser field/alias resolution
/// Test restrictions applied during browser field resolution.
#[test]
fn test_restrictions_check_in_browser_field_alias() {
    let f = super::fixture().join("browser-module");

    let resolver = TestResolver::new(ResolveOptions {
        restrictions: vec![Restriction::Function(Arc::new(|path| {
            // Restrict files containing "browser" in their path
            !path.to_str().is_some_and(|s| s.contains("browser"))
        }))],
        ..ResolveOptions::default()
    });

    // Should fail to resolve due to restriction on browser field
    let resolution = resolver.resolve(&f, "./lib/self.js");
    assert!(resolution.is_err());
}

// Test coverage for check_restrictions at line 1326 in load_extension_alias()
/// Test restrictions applied during extension alias resolution.
#[test]
fn test_restrictions_check_in_extension_alias() {
    let f = super::fixture().join("extension-alias");

    let resolver = TestResolver::new(ResolveOptions {
        extension_alias: IndexMap::from([
            (".js".into(), vec![".ts".into(), ".js".into()]),
            (".mjs".into(), vec![".mts".into(), ".mjs".into()]),
        ]),
        restrictions: vec![Restriction::Function(Arc::new(|path| {
            // Only allow .js files, not .ts files
            path.extension().and_then(|e| e.to_str()) == Some("js")
        }))],
        ..ResolveOptions::default()
    });

    // Should resolve to .js file even though .ts exists, due to restriction
    let resolution = resolver.resolve(&f, "./index.js").map(|r| r.full_path());
    assert_eq!(resolution, Ok(f.join("index.js")));
}

// Test coverage for check_restrictions at line 1570 in package main field resolution
/// Test restrictions applied during package main field resolution.
#[test]
fn test_restrictions_check_in_package_main_fields() {
    let f = super::fixture().join("restrictions");

    let resolver = TestResolver::new(ResolveOptions {
        restrictions: vec![Restriction::Function(Arc::new(|path| {
            // Restrict .js files
            path.extension().and_then(|e| e.to_str()) != Some("js")
        }))],
        ..ResolveOptions::default()
    });

    // Should skip module.js and main field due to restriction
    let resolution = resolver.resolve(&f, "pck2");
    assert_eq!(
        resolution,
        Err(ResolveError::NotFound {
            specifier: "pck2".into()
        })
    );
}

// Test multiple restrictions together (Path + Fn)
/// Test that multiple restrictions are applied correctly.
#[test]
fn test_restrictions_apply_multiple() {
    let f = super::fixture().join("restrictions");

    // Use two function restrictions to test that both are applied
    let re_css = Regex::new(r"\.(css)$").unwrap();
    let re_no_js = Regex::new(r"\.(js)$").unwrap();
    let resolver = TestResolver::new(ResolveOptions {
        extensions: vec![".js".into(), ".css".into()],
        main_files: vec!["index".into()],
        restrictions: vec![
            Restriction::Function(Arc::new(move |path| {
                path.as_os_str()
                    .to_str()
                    .is_some_and(|s| re_css.is_match(s).unwrap_or(false))
            })),
            Restriction::Function(Arc::new(move |path| {
                // Reject .js files
                path.as_os_str()
                    .to_str()
                    .is_some_and(|s| !re_no_js.is_match(s).unwrap_or(false))
            })),
        ],
        ..ResolveOptions::default()
    });

    // Should pass both restrictions and resolve to CSS file
    let resolution = resolver.resolve(&f, "pck1").map(|r| r.full_path());
    assert_eq!(resolution, Ok(f.join("node_modules/pck1/index.css")));
}

// Test that all restrictions must pass
/// Test that resolution fails if any single restriction fails.
#[test]
fn test_restrictions_fail_if_any_fails() {
    let f = super::fixture().join("restrictions");

    // Use two function restrictions where one will fail
    let re_css = Regex::new(r"\.(css)$").unwrap();
    let re_no_css = Regex::new(r"\.(css)$").unwrap();
    let resolver = TestResolver::new(ResolveOptions {
        extensions: vec![".js".into(), ".css".into()],
        main_files: vec!["index".into()],
        restrictions: vec![
            Restriction::Function(Arc::new(move |path| {
                // First restriction: must be CSS
                path.as_os_str()
                    .to_str()
                    .is_some_and(|s| re_css.is_match(s).unwrap_or(false))
            })),
            Restriction::Function(Arc::new(move |path| {
                // Second restriction: must NOT be CSS (contradicts first)
                path.as_os_str()
                    .to_str()
                    .is_some_and(|s| !re_no_css.is_match(s).unwrap_or(false))
            })),
        ],
        ..ResolveOptions::default()
    });

    // Should fail because restrictions contradict each other
    let resolution = resolver.resolve(&f, "pck1");
    assert_eq!(
        resolution,
        Err(ResolveError::NotFound {
            specifier: "pck1".into()
        })
    );
}

// Test is_inside() edge case: exact path match
/// Test restriction behavior when allowing exact path matches.
#[test]
fn test_restrictions_allow_exact_path() {
    let f = super::fixture().join("restrictions");
    let exact_file = f.join("node_modules/pck1/index.css");

    let resolver = TestResolver::new(ResolveOptions {
        extensions: vec![".css".into()],
        main_files: vec!["index".into()],
        restrictions: vec![Restriction::Path(exact_file.clone())],
        ..ResolveOptions::default()
    });

    // Exact path should pass is_inside check
    let resolution = resolver.resolve(&f, "pck1").map(|r| r.full_path());
    assert_eq!(resolution, Ok(exact_file));
}

// Test is_inside() edge case: parent directory restriction
/// Test that restrictions respect parent directory boundaries.
#[test]
fn test_restrictions_respect_parent_directory() {
    let fixture = super::fixture();
    let f = fixture.join("restrictions");

    let resolver = TestResolver::new(ResolveOptions {
        extensions: vec![".js".into()],
        restrictions: vec![Restriction::Path(fixture)],
        ..ResolveOptions::default()
    });

    // Files outside the fixture directory should be rejected
    // pck2's main field points to ../../../c.js which is outside restrictions dir
    let resolution = resolver.resolve(&f, "pck2");
    assert_eq!(
        resolution,
        Err(ResolveError::NotFound {
            specifier: "pck2".into()
        })
    );
}
