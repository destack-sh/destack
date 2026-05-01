//! https://github.com/webpack/enhanced-resolve/blob/main/test/exportsField.test.js

use std::borrow::Cow;
use std::path::Path;
use std::sync::Arc;

use destack_source::{MemoryFileSystem, PathExt};
use indexmap::IndexMap;
use serde_json::json;

use crate::{Resolution, Resolver, ResolverError, ResolverOptions};

/// Test simple exports field resolution.
#[test]
fn test_resolve_exports_field_simple() {
    let f = super::fixture().join("exports-field");
    let f2 = super::fixture().join("exports-field2");
    let f4 = super::fixture().join("exports-field-error");
    let f5 = super::fixture().join("imports-exports-wildcard");

    let resolver = Resolver::for_tests(ResolverOptions {
        extensions: vec![".js".into()],
        is_fully_specified: true,
        conditions: vec!["webpack".into()],
        ..ResolverOptions::default()
    });

    #[rustfmt::skip]
    let pass = [
        ("resolve root using exports field, not a main field", f.clone(), "exports-field", f.join("node_modules/exports-field/x.js")),
        ("resolver should respect condition names", f.clone(), "exports-field/dist/main.js", f.join("node_modules/exports-field/lib/lib2/main.js")),
        ("should resolve query", f.clone(), "exports-field/query.js", f.join("node_modules/exports-field/x.js?query")),
        ("should resolve fragment", f.clone(), "exports-field/fragment.js", f.join("node_modules/exports-field/x.js#fragment")),
        ("resolver should respect query parameters #1", f2.clone(), "exports-field/dist/main.js?foo", f2.join("node_modules/exports-field/lib/lib2/main.js?foo")),
        ("resolver should respect fragment parameters #1", f2.clone(), "exports-field/dist/main.js#foo", f2.join("node_modules/exports-field/lib/lib2/main.js#foo")),
        ("resolver should respect query parameters #2. Direct matching", f2.clone(), "exports-field?foo", f2.join("node_modules/exports-field/index.js?foo")),
        ("resolver should respect fragment parameters #2. Direct matching", f2.clone(), "exports-field#foo", f2.join("node_modules/exports-field/index.js#foo")),
        ("relative path should work, if relative path as request is used", f.clone(), "./node_modules/exports-field/lib/main.js", f.join("node_modules/exports-field/lib/main.js")),
        ("self-resolving root", f.clone(), "@exports-field/core", f.join("a.js")),
        ("should resolve with wildcard pattern #1", f5.clone(), "m/features/f.js", f5.join("node_modules/m/src/features/f.js")),
        ("should resolve with wildcard pattern #2", f5.clone(), "m/features/y/y.js", f5.join("node_modules/m/src/features/y/y.js")),
        ("should resolve with wildcard pattern #3", f5.clone(), "m/features-no-ext/y/y.js", f5.join("node_modules/m/src/features/y/y.js")),
        ("should resolve with wildcard pattern #4", f5.clone(), "m/middle/nested/f.js", f5.join("node_modules/m/src/middle/nested/f.js")),
        ("should resolve with wildcard pattern #5", f5.clone(), "m/middle-1/nested/f.js", f5.join("node_modules/m/src/middle-1/nested/f.js")),
        ("should resolve with wildcard pattern #6", f5.clone(), "m/middle-2/nested/f.js", f5.join("node_modules/m/src/middle-2/nested/f.js")),
        ("should resolve with wildcard pattern #7", f5.clone(), "m/middle-3/nested/f", f5.join("node_modules/m/src/middle-3/nested/f/nested/f.js")),
        ("should resolve with wildcard pattern #8", f5.clone(), "m/middle-4/f/nested", f5.join("node_modules/m/src/middle-4/f/f.js")),
        ("should resolve with wildcard pattern #9", f5.clone(), "m/middle-5/f$/$", f5.join("node_modules/m/src/middle-5/f$/$.js")),
    ];

    for (comment, path, request, expected) in pass {
        let resolved_path = resolver
            .resolve_test_directory(&path, request)
            .map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {path:?} {request}");
    }

    let p = f.join("node_modules/exports-field/package.json");
    let p4 = f4.join("node_modules/exports-field/package.json");
    let p5 = f5.join("node_modules/m/package.json");

    #[rustfmt::skip]
    let fail = [
        ("relative path should not work with exports field", f.clone(), "./node_modules/exports-field/dist/main.js", ResolverError::NotFound { specifier: "./node_modules/exports-field/dist/main.js".into() }),
        ("backtracking should not work for request", f.clone(), "exports-field/dist/../../../a.js", ResolverError::InvalidPackageTarget { target: "./lib/../../../a.js".to_string(), name: "./dist/".to_string(), package_path: p.clone() }),
        ("backtracking should not work for exports field target", f.clone(), "exports-field/dist/a.js", ResolverError::InvalidPackageTarget { target: "./../../a.js".to_string(), name: "./dist/a.js".to_string(), package_path: p.clone() }),
        ("not exported error", f.clone(), "exports-field/anything/else", ResolverError::PackagePathNotExported { subpath: "./anything/else".to_string(), package_path: f.join("node_modules/exports-field"), package_json_path: p.clone(), conditions: vec!["webpack".into()] }),
        ("request ending with slash #1", f.clone(), "exports-field/", ResolverError::PackagePathNotExported { subpath: "./".to_string(), package_path: f.join("node_modules/exports-field"), package_json_path: p.clone(), conditions: vec!["webpack".into()] }),
        ("request ending with slash #2", f.clone(), "exports-field/dist/", ResolverError::PackagePathNotExported { subpath: "./dist/".to_string(), package_path: f.join("node_modules/exports-field"), package_json_path: p.clone(), conditions: vec!["webpack".into()] }),
        ("request ending with slash #3", f.clone(), "exports-field/lib/", ResolverError::PackagePathNotExported { subpath: "./lib/".to_string(), package_path: f.join("node_modules/exports-field"), package_json_path: p, conditions: vec!["webpack".into()] }),
        ("should throw error if target is invalid", f4, "exports-field", ResolverError::InvalidPackageTarget { target: "./a/../b/../../pack1/index.js".to_string(), name: ".".to_string(), package_path: p4 }),
        ("throw error if exports field is invalid", f.clone(), "invalid-exports-field", ResolverError::InvalidPackageJson { path: f.join("node_modules/invalid-exports-field/package.json") }),
        ("should throw error if target is 'null'", f5.clone(), "m/features/internal/file.js", ResolverError::PackagePathNotExported { subpath: "./features/internal/file.js".to_string(), package_path: f5.join("node_modules/m"), package_json_path: p5, conditions: vec!["webpack".into()] }),
    ];

    for (comment, path, request, error) in fail {
        let resolution = resolver.resolve_test_directory(&path, request);
        assert_eq!(resolution, Err(error), "{comment} {path:?} {request}");
    }
}

/// Fall back to main field when package exports resolution is disabled.
#[test]
fn test_resolve_exports_field_disabled_uses_main_field() {
    let f = super::fixture().join("exports-field");

    let resolver = Resolver::for_tests(ResolverOptions {
        extensions: vec![".js".into()],
        resolve_package_json_exports: false,
        ..ResolverOptions::default()
    });

    let resolved_path = resolver
        .resolve_test_directory(&f, "exports-field")
        .map(|r| r.full_path());
    assert_eq!(
        resolved_path,
        Ok(f.join("node_modules/exports-field/main.js"))
    );
}

/// Test resolving using exports field, ignoring browser field.
#[test]
fn test_resolve_exports_field_not_browser_field1() {
    let f = super::fixture().join("exports-field");

    let resolver = Resolver::for_tests(ResolverOptions {
        conditions: vec!["webpack".into()],
        extensions: vec![".js".into()],
        ..ResolverOptions::default()
    });

    let resolved_path = resolver
        .resolve_test_directory(&f, "exports-field/dist/main.js")
        .map(|r| r.full_path());
    assert_eq!(
        resolved_path,
        Ok(f.join("node_modules/exports-field/lib/lib2/main.js"))
    );
}

/// Test resolving using exports field with browser alias field.
#[test]
fn test_resolve_exports_field_not_browser_field2() {
    let f2 = super::fixture().join("exports-field2");

    let resolver = Resolver::for_tests(ResolverOptions {
        extensions: vec![".js".into()],
        conditions: vec!["node".into()],
        ..ResolverOptions::default()
    });

    let resolved_path = resolver
        .resolve_test_directory(&f2, "exports-field/dist/main.js")
        .map(|r| r.full_path());
    assert_eq!(
        resolved_path,
        Ok(f2.join("node_modules/exports-field/lib/browser.js"))
    );
}

/// Test resolution of extension without fullySpecified.
#[test]
fn test_resolve_exports_field_extension_without_fully_specified() {
    let f2 = super::fixture().join("exports-field2");

    let commonjs_resolver = Resolver::for_tests(ResolverOptions {
        extensions: vec![".js".into()],
        conditions: vec!["webpack".into()],
        ..ResolverOptions::default()
    });

    let resolved_path = commonjs_resolver
        .resolve_test_directory(&f2, "exports-field/dist/main")
        .map(|r| r.full_path());
    assert_eq!(
        resolved_path,
        Ok(f2.join("node_modules/exports-field/lib/lib2/main.js"))
    );
}

/// Test exports field with extension alias.
#[test]
fn test_resolve_exports_field_extension_alias() {
    let f = super::fixture().join("exports-field-and-extension-alias");

    let resolver = Resolver::for_tests(ResolverOptions {
        extensions: vec![".js".into()],
        extension_alias: IndexMap::from([(".js".into(), vec![".ts".into(), ".js".into()])]),
        is_fully_specified: true,
        conditions: vec!["webpack".into(), "default".into()],
        ..ResolverOptions::default()
    });

    #[rustfmt::skip]
    let pass = [
        ("should resolve with the `extensionAlias` option", f.clone(), "@org/pkg/string.js", f.join("node_modules/@org/pkg/dist/string.js")),
        ("should resolve with the `extensionAlias` option #2", f.clone(), "pkg/string.js", f.join("node_modules/pkg/dist/string.js")),
    ];

    for (comment, path, request, expected) in pass {
        let resolved_path = resolver
            .resolve_test_directory(&path, request)
            .map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {path:?} {request}");
    }
}

/// Test complex extension alias with exports field.
#[test]
fn test_resolve_exports_field_extension_alias_complex() {
    let f = super::fixture().join("exports-field-and-extension-alias");

    let resolver = Resolver::for_tests(ResolverOptions {
        extensions: vec![".js".into()],
        extension_alias: IndexMap::from([(
            ".js".into(),
            vec![
                ".foo".into(),
                ".baz".into(),
                ".baz".into(),
                ".ts".into(),
                ".js".into(),
            ],
        )]),
        is_fully_specified: true,
        conditions: vec!["webpack".into(), "default".into()],
        ..ResolverOptions::default()
    });

    #[rustfmt::skip]
    let pass = [
        ("should resolve with the `extensionAlias` option #3", f.clone(), "pkg/string.js", f.join("node_modules/pkg/dist/string.js")),
    ];

    for (comment, path, request, expected) in pass {
        let resolved_path = resolver
            .resolve_test_directory(&path, request)
            .map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {path:?} {request}");
    }
}

/// Test extension alias errors in exports field.
#[test]
fn test_resolve_exports_field_extension_alias_error() {
    let f = super::fixture().join("exports-field-and-extension-alias");

    let resolver = Resolver::for_tests(ResolverOptions {
        extensions: vec![".js".into()],
        extension_alias: IndexMap::from([(".js".into(), vec![".ts".into()])]),
        is_fully_specified: true,
        conditions: vec!["webpack".into(), "default".into()],
        ..ResolverOptions::default()
    });

    #[rustfmt::skip]
    let fail = [
        // https://github.com/webpack/enhanced-resolve/blob/a998c7d218b7a9ec2461fc4fddd1ad5dd7687485/test/exportsField.test.js#L2976-L3024
        ("should throw error with the `extensionAlias` option", f.clone(), "pkg/string.js", ResolverError::ExtensionAliasNotFound {
            filename: "string.js".into(),
            tried: "string.ts".into(),
            dir: f.join("node_modules/pkg/dist")
        }),
    ];

    for (comment, path, request, error) in fail {
        let resolution = resolver.resolve_test_directory(&path, request);
        assert_eq!(resolution, Err(error), "{comment} {path:?} {request}");
    }
}

/// Test exports field directory resolution.
#[test]
fn test_resolve_exports_field_directory() {
    let f = super::fixture();
    let resolver = Resolver::for_tests(ResolverOptions::default());
    let resolution = resolver.resolve_test_directory(f.join("foo"), "../exports-field");
    let path = resolution.unwrap().full_path();
    assert_eq!(path, f.join("exports-field").join("a.js"));
}

struct TestCase {
    #[allow(dead_code)]
    name: &'static str,
    expect: Option<Vec<&'static str>>,
    exports: Cow<'static, serde_json::Value>,
    request: &'static str,
    conditions: Vec<&'static str>,
}

fn exports_field(value: &serde_json::Value) -> Cow<'static, serde_json::Value> {
    // Clone and leak the value to get a 'static reference for big-endian
    let value = Box::leak::<'static>(Box::new(value.clone()));
    Cow::Borrowed(value)
}

/// Test various exports field cases.
#[test]
fn test_resolve_exports_field_cases() {
    let test_cases = vec![
        TestCase {
            name: "sample #1",
            expect: Some(vec!["./dist/test/file.js"]),
            exports: exports_field(&json!({
                "./foo/": {
                    "import": [
                        "./dist/",
                        "./src/"
                    ],
                    "webpack": "./wp/"
                },
                ".": "./main.js"
            })),
            request: "./foo/test/file.js",
            conditions: vec!["import", "webpack"],
        },
        // (test is repeated because we don't support returning an array)
        TestCase {
            name: "sample #1",
            expect: Some(vec!["./src/test/file.js"]),
            exports: exports_field(&json!({
                "./foo/": {
                    "import": [
                        "./src/"
                    ],
                    "webpack": "./wp/"
                },
                ".": "./main.js"
            })),
            request: "./foo/test/file.js",
            conditions: vec!["import", "webpack"],
        },
        TestCase {
            name: "sample #1 (wildcard)",
            expect: Some(vec!["./dist/test/file.js"]),
            exports: exports_field(&json!({
                "./foo/*": {
                    "import": [
                        "./dist/*",
                        "./src/*"
                    ],
                    "webpack": "./wp/*"
                },
                ".": "./main.js"
            })),
            request: "./foo/test/file.js",
            conditions: vec!["import", "webpack"],
        },
        // (test is repeated because we don't support returning an array)
        TestCase {
            name: "sample #1 (wildcard)",
            expect: Some(vec!["./src/test/file.js"]),
            exports: exports_field(&json!({
                "./foo/*": {
                    "import": [
                        "./src/*"
                    ],
                    "webpack": "./wp/*"
                },
                ".": "./main.js"
            })),
            request: "./foo/test/file.js",
            conditions: vec!["import", "webpack"],
        },
        TestCase {
            name: "sample #2",
            expect: Some(vec!["./data/timezones/pdt.mjs"]),
            exports: exports_field(&json!({
                "./timezones/": "./data/timezones/"
            })),
            request: "./timezones/pdt.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "sample #2 (wildcard)",
            expect: Some(vec!["./data/timezones/pdt.mjs"]),
            exports: exports_field(&json!({
                "./timezones/*": "./data/timezones/*"
            })),
            request: "./timezones/pdt.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "sample #3",
            expect: Some(vec!["./data/timezones/timezones/pdt.mjs"]),
            exports: exports_field(&json!({
                "./": "./data/timezones/"
            })),
            request: "./timezones/pdt.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "sample #3 (wildcard)",
            expect: Some(vec!["./data/timezones/timezones/pdt.mjs"]),
            exports: exports_field(&json!({
                "./*": "./data/timezones/*"
            })),
            request: "./timezones/pdt.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "sample #4",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                "./lib/": {
                    "browser": [
                        "./browser/"
                    ]
                },
                ".": {
                    "node": "./index.js"
                }
            })),
            request: ".",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "sample #5",
            expect: Some(vec!["./browser/index.js"]),
            exports: exports_field(&json!({
                "./lib/": {
                    "browser": [
                        "./browser/"
                    ]
                },
                ".": {
                    "node": "./index.js",
                    "default": "./browser/index.js"
                }
            })),
            request: ".",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "sample #6",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                "./dist/a": "./dist/index.js"
            })),
            request: "./dist/aaa",
            conditions: vec![],
        },
        TestCase {
            name: "sample #6 (wildcard)",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                "./dist/a*": "./dist/index.js"
            })),
            request: "./dist/aaa",
            conditions: vec![],
        },
        TestCase {
            name: "sample #7",
            expect: None,
            exports: exports_field(&json!({
                "./a/a/": "./dist/index.js"
            })),
            request: "./a/a/a",
            conditions: vec![],
        },
        TestCase {
            name: "sample #7 (wildcard)",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                "./a/a/*": "./dist/index.js"
            })),
            request: "./a/a/a",
            conditions: vec![],
        },
        TestCase {
            name: "sample #8",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                ".": "./index.js"
            })),
            request: "./timezones/pdt.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "sample #9",
            expect: Some(vec!["./main.js"]),
            exports: exports_field(&json!({
                "./index.js": "./main.js"
            })),
            request: "./index.js",
            conditions: vec![],
        },
        TestCase {
            name: "sample #10",
            expect: Some(vec!["./ok.js"]),
            exports: exports_field(&json!({
                "./#foo": "./ok.js",
                "./module": "./ok.js",
                "./🎉": "./ok.js",
                "./%F0%9F%8E%89": "./other.js",
                "./bar#foo": "./ok.js",
                "./#zapp/": "./"
            })),
            request: "./#foo",
            conditions: vec![],
        },
        TestCase {
            name: "sample #11",
            expect: Some(vec!["./ok.js"]),
            exports: exports_field(&json!({
                "./#foo": "./ok.js",
                "./module": "./ok.js",
                "./🎉": "./ok.js",
                "./%F0%9F%8E%89": "./other.js",
                "./bar#foo": "./ok.js",
                "./#zapp/": "./"
            })),
            request: "./bar#foo",
            conditions: vec![],
        },
        TestCase {
            name: "sample #12",
            expect: Some(vec!["./ok.js#abc"]),
            exports: exports_field(&json!({
                "./#foo": "./ok.js",
                "./module": "./ok.js",
                "./🎉": "./ok.js",
                "./%F0%9F%8E%89": "./other.js",
                "./bar#foo": "./ok.js",
                "./#zapp/": "./"
            })),
            request: "./#zapp/ok.js#abc",
            conditions: vec![],
        },
        TestCase {
            name: "sample #12 (wildcard)",
            expect: Some(vec!["./ok.js#abc"]),
            exports: exports_field(&json!({
                "./#foo": "./ok.js",
                "./module": "./ok.js",
                "./🎉": "./ok.js",
                "./%F0%9F%8E%89": "./other.js",
                "./bar#foo": "./ok.js",
                "./#zapp/*": "./*"
            })),
            request: "./#zapp/ok.js#abc",
            conditions: vec![],
        },
        TestCase {
            name: "sample #13",
            expect: Some(vec!["./ok.js?abc"]),
            exports: exports_field(&json!({
                "./#foo": "./ok.js",
                "./module": "./ok.js",
                "./🎉": "./ok.js",
                "./%F0%9F%8E%89": "./other.js",
                "./bar#foo": "./ok.js",
                "./#zapp/": "./"
            })),
            request: "./#zapp/ok.js?abc",
            conditions: vec![],
        },
        TestCase {
            name: "sample #13 (wildcard)",
            expect: Some(vec!["./ok.js?abc"]),
            exports: exports_field(&json!({
                "./#foo": "./ok.js",
                "./module": "./ok.js",
                "./🎉": "./ok.js",
                "./%F0%9F%8E%89": "./other.js",
                "./bar#foo": "./ok.js",
                "./#zapp/*": "./*"
            })),
            request: "./#zapp/ok.js?abc",
            conditions: vec![],
        },
        TestCase {
            name: "sample #14",
            expect: Some(vec!["./🎉.js"]),
            exports: exports_field(&json!({
                "./#foo": "./ok.js",
                "./module": "./ok.js",
                "./🎉": "./ok.js",
                "./%F0%9F%8E%89": "./other.js",
                "./bar#foo": "./ok.js",
                "./#zapp/": "./"
            })),
            request: "./#zapp/🎉.js",
            conditions: vec![],
        },
        TestCase {
            name: "sample #14 (wildcard)",
            expect: Some(vec!["./🎉.js"]),
            exports: exports_field(&json!({
                "./#foo": "./ok.js",
                "./module": "./ok.js",
                "./🎉": "./ok.js",
                "./%F0%9F%8E%89": "./other.js",
                "./bar#foo": "./ok.js",
                "./#zapp/*": "./*"
            })),
            request: "./#zapp/🎉.js",
            conditions: vec![],
        },
        TestCase {
            name: "sample #15",
            expect: Some(vec!["./%F0%9F%8E%89.js"]),
            exports: exports_field(&json!({
                "./#foo": "./ok.js",
                "./module": "./ok.js",
                "./🎉": "./ok.js",
                "./%F0%9F%8E%89": "./other.js",
                "./bar#foo": "./ok.js",
                "./#zapp/": "./"
            })),
            request: "./#zapp/%F0%9F%8E%89.js",
            conditions: vec![],
        },
        TestCase {
            name: "sample #15 (wildcard)",
            expect: Some(vec!["./%F0%9F%8E%89.js"]),
            exports: exports_field(&json!({
                "./#foo": "./ok.js",
                "./module": "./ok.js",
                "./🎉": "./ok.js",
                "./%F0%9F%8E%89": "./other.js",
                "./bar#foo": "./ok.js",
                "./#zapp/*": "./*"
            })),
            request: "./#zapp/%F0%9F%8E%89.js",
            conditions: vec![],
        },
        TestCase {
            name: "sample #16",
            expect: Some(vec!["./ok.js"]),
            exports: exports_field(&json!({
                "./#foo": "./ok.js",
                "./module": "./ok.js",
                "./🎉": "./ok.js",
                "./%F0%9F%8E%89": "./other.js",
                "./bar#foo": "./ok.js",
                "./#zapp/": "./"
            })),
            request: "./🎉",
            conditions: vec![],
        },
        TestCase {
            name: "sample #17",
            expect: Some(vec!["./other.js"]),
            exports: exports_field(&json!({
                "./#foo": "./ok.js",
                "./module": "./ok.js",
                "./🎉": "./ok.js",
                "./%F0%9F%8E%89": "./other.js",
                "./bar#foo": "./ok.js",
                "./#zapp/": "./"
            })),
            request: "./%F0%9F%8E%89",
            conditions: vec![],
        },
        TestCase {
            name: "sample #18",
            expect: Some(vec!["./ok.js"]),
            exports: exports_field(&json!({
                "./#foo": "./ok.js",
                "./module": "./ok.js",
                "./🎉": "./ok.js",
                "./%F0%9F%8E%89": "./other.js",
                "./bar#foo": "./ok.js",
                "./#zapp/": "./"
            })),
            request: "./module",
            conditions: vec![],
        },
        TestCase {
            name: "sample #19",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                "./#foo": "./ok.js",
                "./module": "./ok.js",
                "./🎉": "./ok.js",
                "./%F0%9F%8E%89": "./other.js",
                "./bar#foo": "./ok.js",
                "./#zapp/": "./"
            })),
            request: "./module#foo",
            conditions: vec![],
        },
        TestCase {
            name: "sample #20",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                "./#foo": "./ok.js",
                "./module": "./ok.js",
                "./🎉": "./ok.js",
                "./%F0%9F%8E%89": "./other.js",
                "./bar#foo": "./ok.js",
                "./#zapp/": "./"
            })),
            request: "./module?foo",
            conditions: vec![],
        },
        TestCase {
            name: "sample #21",
            expect: Some(vec!["./zizizi"]),
            exports: exports_field(&json!({
                "./#foo": "./ok.js",
                "./module": "./ok.js",
                "./🎉": "./ok.js",
                "./%F0%9F%8E%89": "./other.js",
                "./bar#foo": "./ok.js",
                "./#zapp/": "./",
                "./#zipp*": "./z*z*z*"
            })),
            request: "./#zippi",
            conditions: vec![],
        },
        TestCase {
            name: "sample #22",
            expect: Some(vec!["./d?e?f"]),
            exports: exports_field(&json!({
                "./a?b?c/": "./"
            })),
            request: "./a?b?c/d?e?f",
            conditions: vec![],
        },
        TestCase {
            name: "Direct mapping #1",
            expect: Some(vec!["./dist/index.js"]),
            exports: exports_field(&json!({
                ".": "./dist/index.js"
            })),
            request: ".",
            conditions: vec![],
        },
        TestCase {
            name: "Direct mapping #2",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                "./": "./",
                "./*": "./*",
                "./dist/index.js": "./dist/index.js"
            })),
            request: ".",
            conditions: vec![],
        },
        TestCase {
            name: "Direct mapping #3",
            expect: Some(vec!["./dist/a.js"]),
            exports: exports_field(&json!({
                "./dist/": "./dist/",
                "./dist/*": "./dist/*",
                "./dist*": "./dist*",
                "./dist/index.js": "./dist/a.js"
            })),
            request: "./dist/index.js",
            conditions: vec![],
        },
        TestCase {
            name: "Direct mapping #4",
            expect: Some(vec!["./index.js"]),
            exports: exports_field(&json!({
                "./": {
                    "browser": [
                        "./browser/"
                    ]
                },
                "./*": {
                    "browser": [
                        "./browser/*"
                    ]
                },
                "./dist/index.js": {
                    "browser": "./index.js"
                }
            })),
            request: "./dist/index.js",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "Direct mapping #5",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                "./": {
                    "browser": [
                        "./browser/"
                    ]
                },
                "./*": {
                    "browser": [
                        "./browser/*"
                    ]
                },
                "./dist/index.js": {
                    "node": "./node.js"
                }
            })),
            request: "./dist/index.js",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "Direct mapping #6",
            expect: Some(vec!["./index.js"]),
            exports: exports_field(&json!({
                ".": {
                    "browser": "./index.js",
                    "node": "./src/node/index.js",
                    "default": "./src/index.js"
                }
            })),
            request: ".",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "Direct mapping #7",
            expect: Some(vec!["./src/index.js"]),
            exports: exports_field(&json!({
                ".": {
                    "default": "./src/index.js",
                    "browser": "./index.js",
                    "node": "./src/node/index.js"
                }
            })),
            request: ".",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "Direct mapping #8",
            expect: Some(vec!["./src/index.js"]),
            exports: exports_field(&json!({
                ".": {
                    "browser": "./index.js",
                    "node": "./src/node/index.js",
                    "default": "./src/index.js"
                }
            })),
            request: ".",
            conditions: vec![],
        },
        TestCase {
            name: "Direct mapping #9",
            expect: Some(vec!["./index"]),
            exports: exports_field(&json!({
                ".": "./index"
            })),
            request: ".",
            conditions: vec![],
        },
        TestCase {
            name: "Direct mapping #10",
            expect: Some(vec!["./index.js"]),
            exports: exports_field(&json!({
                "./index": "./index.js"
            })),
            request: "./index",
            conditions: vec![],
        },
        TestCase {
            name: "Direct mapping #11",
            expect: Some(vec!["./foo.js"]),
            exports: exports_field(&json!({
                "./": "./",
                "./*": "./*",
                "./dist/index.js": "./dist/index.js"
            })),
            request: "./foo.js",
            conditions: vec![],
        },
        TestCase {
            name: "Direct mapping #12",
            expect: Some(vec!["./foo/bar/baz.js"]),
            exports: exports_field(&json!({
                "./": "./",
                "./*": "./*",
                "./dist/index.js": "./dist/index.js"
            })),
            request: "./foo/bar/baz.js",
            conditions: vec![],
        },
        TestCase {
            name: "Direct mapping #13",
            expect: Some(vec!["./foo/bar/baz.js"]),
            exports: exports_field(&json!({
                "./": "./",
                "./dist/index.js": "./dist/index.js"
            })),
            request: "./foo/bar/baz.js",
            conditions: vec![],
        },
        TestCase {
            name: "Direct mapping #14",
            expect: Some(vec!["./foo/bar/baz.js"]),
            exports: exports_field(&json!({
                "./*": "./*",
                "./dist/index.js": "./dist/index.js"
            })),
            request: "./foo/bar/baz.js",
            conditions: vec![],
        },
        TestCase {
            name: "Direct and conditional mapping #1",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                ".": [{
                    "browser": "./browser.js"
                }, {
                    "require": "./require.js"
                }, {
                    "import": "./import.mjs"
                }]
            })),
            request: ".",
            conditions: vec![],
        },
        TestCase {
            name: "Direct and conditional mapping #2",
            expect: Some(vec!["./import.mjs"]),
            exports: exports_field(&json!({
                ".": [{
                    "browser": "./browser.js"
                }, {
                    "require": "./require.js"
                }, {
                    "import": "./import.mjs"
                }]
            })),
            request: ".",
            conditions: vec!["import"],
        },
        TestCase {
            name: "Direct and conditional mapping #3",
            expect: Some(vec!["./require.js"]),
            exports: exports_field(&json!({
                ".": [
                {
                    "browser": "./browser.js"
                },
                {
                    "require": "./require.js"
                },
                {
                    "import": "./import.mjs"
                }
                ]
            })),
            request: ".",
            conditions: vec!["import", "require"],
        },
        // (test is repeated because we don't support returning an array)
        TestCase {
            name: "Direct and conditional mapping #3",
            expect: Some(vec!["./import.mjs"]),
            exports: exports_field(&json!({
                ".": [{
                    "browser": "./browser.js"
                }, {
                    "import": "./import.mjs"
                }]
            })),
            request: ".",
            conditions: vec!["import", "require"],
        },
        TestCase {
            name: "Direct and conditional mapping #4",
            expect: Some(vec!["./require.js"]),
            exports: exports_field(&json!({
                ".": [{
                    "browser": "./browser.js"
                }, {
                    "require": [
                        "./require.js"
                    ]
                }, {
                    "import": [
                        "./import.mjs",
                        "./import.js"
                    ]
                }]
            })),
            request: ".",
            conditions: vec!["import", "require"],
        },
        // (test is repeated because we don't support returning an array)
        TestCase {
            name: "Direct and conditional mapping #4",
            expect: Some(vec!["./import.mjs"]),
            exports: exports_field(&json!({
                ".": [
                {
                    "browser": "./browser.js"
                },
                {
                    "import": [
                        "./import.mjs",
                        "./import.js"
                    ]
                }
                ]
            })),
            request: ".",
            conditions: vec!["import", "require"],
        },
        // (test is repeated because we don't support returning an array)
        TestCase {
            name: "Direct and conditional mapping #4",
            expect: Some(vec!["./import.js"]),
            exports: exports_field(&json!({
                ".": [
                {
                    "browser": "./browser.js"
                },
                {
                    "import": [
                        "./import.js"
                    ]
                }
                ]
            })),
            request: ".",
            conditions: vec!["import", "require"],
        },
        TestCase {
            name: "mapping to a folder root #1",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                "./timezones": "./data/timezones/"
            })),
            request: "./timezones/pdt.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "mapping to a folder root #2",
            expect: None,
            exports: exports_field(&json!({
                "./timezones/": "./data/timezones"
            })),
            request: "./timezones/pdt.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "mapping to a folder root #3",
            expect: Some(vec!["./data/timezones/pdt/index.mjs"]),
            exports: exports_field(&json!({
                "./timezones/pdt/": "./data/timezones/pdt/"
            })),
            request: "./timezones/pdt/index.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "mapping to a folder root #3 (wildcard)",
            expect: Some(vec!["./data/timezones/pdt/index.mjs"]),
            exports: exports_field(&json!({
                "./timezones/pdt/*": "./data/timezones/pdt/*"
            })),
            request: "./timezones/pdt/index.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "mapping to a folder root #4",
            expect: Some(vec!["./timezones/pdt.mjs"]),
            exports: exports_field(&json!({
                "./": "./timezones/"
            })),
            request: "./pdt.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "mapping to a folder root #4 (wildcard)",
            expect: Some(vec!["./timezones/pdt.mjs"]),
            exports: exports_field(&json!({
                "./*": "./timezones/*"
            })),
            request: "./pdt.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "mapping to a folder root #5",
            expect: Some(vec!["./timezones/pdt.mjs"]),
            exports: exports_field(&json!({
                "./": "./"
            })),
            request: "./timezones/pdt.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "mapping to a folder root #5 (wildcard)",
            expect: Some(vec!["./timezones/pdt.mjs"]),
            exports: exports_field(&json!({
                "./*": "./*"
            })),
            request: "./timezones/pdt.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "mapping to a folder root #6",
            expect: None,
            exports: exports_field(&json!({
                "./": "."
            })),
            request: "./timezones/pdt.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "mapping to a folder root #6 (wildcard)",
            expect: None,
            exports: exports_field(&json!({
                "./*": "."
            })),
            request: "./timezones/pdt.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "mapping to a folder root #7",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                ".": "./"
            })),
            request: "./timezones/pdt.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "mapping to a folder root #7 (wildcard)",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                ".": "./*"
            })),
            request: "./timezones/pdt.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "the longest matching path prefix is prioritized #1",
            expect: Some(vec!["./lib/index.mjs"]),
            exports: exports_field(&json!({
                "./": "./",
                "./dist/": "./lib/"
            })),
            request: "./dist/index.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "the longest matching path prefix is prioritized #1 (wildcard)",
            expect: Some(vec!["./lib/index.mjs"]),
            exports: exports_field(&json!({
                "./*": "./*",
                "./dist/*": "./lib/*"
            })),
            request: "./dist/index.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "the longest matching path prefix is prioritized #2",
            expect: Some(vec!["./dist/utils/index.js"]),
            exports: exports_field(&json!({
                "./dist/utils/": "./dist/utils/",
                "./dist/": "./lib/"
            })),
            request: "./dist/utils/index.js",
            conditions: vec![],
        },
        TestCase {
            name: "the longest matching path prefix is prioritized #2 (wildcard)",
            expect: Some(vec!["./dist/utils/index.js"]),
            exports: exports_field(&json!({
                "./dist/utils/*": "./dist/utils/*",
                "./dist/*": "./lib/*"
            })),
            request: "./dist/utils/index.js",
            conditions: vec![],
        },
        TestCase {
            name: "the longest matching path prefix is prioritized #3",
            expect: Some(vec!["./dist/utils/index.js"]),
            exports: exports_field(&json!({
                "./dist/utils/index.js": "./dist/utils/index.js",
                "./dist/utils/": "./dist/utils/index.mjs",
                "./dist/": "./lib/"
            })),
            request: "./dist/utils/index.js",
            conditions: vec![],
        },
        TestCase {
            name: "the longest matching path prefix is prioritized #3 (wildcard)",
            expect: Some(vec!["./dist/utils/index.js"]),
            exports: exports_field(&json!({
                "./dist/utils/index.js": "./dist/utils/index.js",
                "./dist/utils/*": "./dist/utils/index.mjs",
                "./dist/*": "./lib/*"
            })),
            request: "./dist/utils/index.js",
            conditions: vec![],
        },
        TestCase {
            name: "the longest matching path prefix is prioritized #4",
            expect: Some(vec!["./lib/index.mjs"]),
            exports: exports_field(&json!({
                "./": {
                    "browser": "./browser/"
                },
                "./dist/": "./lib/"
            })),
            request: "./dist/index.mjs",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "the longest matching path prefix is prioritized #4 (wildcard)",
            expect: Some(vec!["./lib/index.mjs"]),
            exports: exports_field(&json!({
                "./*": {
                    "browser": "./browser/*"
                },
                "./dist/*": "./lib/*"
            })),
            request: "./dist/index.mjs",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "conditional mapping folder #1",
            // `lodash/` does not start with './' so fallbacks to util
            expect: Some(vec!["./utils/index.js"]),
            exports: exports_field(&json!({
                "./utils/": {
                    "browser": [
                        "lodash/",
                        "./utils/"
                    ],
                    "node": [
                        "./utils-node/"
                    ]
                }
            })),
            request: "./utils/index.js",
            conditions: vec!["browser"],
        },
        // (test is repeated because we don't support returning an array)
        TestCase {
            name: "conditional mapping folder #1",
            expect: Some(vec!["./utils/index.js"]),
            exports: exports_field(&json!({
                "./utils/": {
                    "browser": [
                        "./utils/"
                    ],
                    "node": [
                        "./utils-node/"
                    ]
                }
            })),
            request: "./utils/index.js",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "conditional mapping folder #1 (wildcard)",
            // `lodash/` does not start with './' so fallbacks to util
            expect: Some(vec!["./utils/index.js"]),
            exports: exports_field(&json!({
                "./utils/*": {
                    "browser": [
                        "lodash/*",
                        "./utils/*"
                    ],
                    "node": [
                        "./utils-node/*"
                    ]
                }
            })),
            request: "./utils/index.js",
            conditions: vec!["browser"],
        },
        // (test is repeated because we don't support returning an array)
        TestCase {
            name: "conditional mapping folder #1 (wildcard)",
            expect: Some(vec!["./utils/index.js"]),
            exports: exports_field(&json!({
                "./utils/*": {
                    "browser": [
                        "./utils/*"
                    ],
                    "node": [
                        "./utils-node/*"
                    ]
                }
            })),
            request: "./utils/index.js",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "conditional mapping folder #2",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                "./utils/": {
                    "webpack": "./wpk/",
                    "browser": [
                        "lodash/",
                        "./utils/"
                    ],
                    "node": [
                        "./node/"
                    ]
                }
            })),
            request: "./utils/index.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "conditional mapping folder #2 (wildcard)",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                "./utils/*": {
                    "webpack": "./wpk/*",
                    "browser": [
                        "lodash/*",
                        "./utils/*"
                    ],
                    "node": [
                        "./node/*"
                    ]
                }
            })),
            request: "./utils/index.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "conditional mapping folder #3",
            expect: Some(vec!["./wpk/index.mjs"]),
            exports: exports_field(&json!({
                "./utils/": {
                    "webpack": "./wpk/",
                    "browser": [
                        "lodash/",
                        "./utils/"
                    ],
                    "node": [
                        "./utils/"
                    ]
                }
            })),
            request: "./utils/index.mjs",
            conditions: vec!["browser", "webpack"],
        },
        TestCase {
            name: "conditional mapping folder #3 (wildcard)",
            expect: Some(vec!["./wpk/index.mjs"]),
            exports: exports_field(&json!({
                "./utils/*": {
                    "webpack": "./wpk/*",
                    "browser": [
                        "lodash/*",
                        "./utils/*"
                    ],
                    "node": [
                        "./utils/*"
                    ]
                }
            })),
            request: "./utils/index.mjs",
            conditions: vec!["browser", "webpack"],
        },
        TestCase {
            name: "incorrect exports field #1",
            expect: None,
            exports: exports_field(&json!({
                "/utils/": "./a/"
            })),
            request: "./utils/index.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "incorrect exports field #2",
            expect: None,
            exports: exports_field(&json!({
                "./utils/": "/a/"
            })),
            request: "./utils/index.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "incorrect exports field #3",
            expect: None,
            exports: exports_field(&json!({
                "/utils/": {
                    "browser": "./a/",
                    "default": "./b/"
                }
            })),
            request: "./utils/index.mjs",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "incorrect exports field #4",
            expect: None,
            exports: exports_field(&json!({
                "./utils/": {
                    "browser": "/a/",
                    "default": "/b/"
                }
            })),
            request: "./utils/index.mjs",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "incorrect exports field #4 (wildcard)",
            expect: None,
            exports: exports_field(&json!({
                "./utils/*": {
                    "browser": "/a/",
                    "default": "/b/"
                }
            })),
            request: "./utils/index.mjs",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "incorrect exports field #5",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                "./utils/index": "./a/index.js"
            })),
            request: "./utils/index.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "incorrect exports field #6",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                "./utils/index.mjs": "./a/index.js"
            })),
            request: "./utils/index",
            conditions: vec![],
        },
        TestCase {
            name: "incorrect exports field #7",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                "./utils/index": {
                    "browser": "./a/index.js",
                    "default": "./b/index.js"
                }
            })),
            request: "./utils/index.mjs",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "incorrect exports field #8",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                "./utils/index.mjs": {
                    "browser": "./a/index.js",
                    "default": "./b/index.js"
                }
            })),
            request: "./utils/index",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "backtracking package base #1",
            expect: Some(vec!["./dist/index"]),
            exports: exports_field(&json!({
                "./../../utils/": "./dist/"
            })),
            request: "./../../utils/index",
            conditions: vec![],
        },
        TestCase {
            name: "backtracking package base #1 (wildcard)",
            expect: Some(vec!["./dist/index"]),
            exports: exports_field(&json!({
                "./../../utils/*": "./dist/*"
            })),
            request: "./../../utils/index",
            conditions: vec![],
        },
        TestCase {
            name: "backtracking package base #2",
            expect: None,
            exports: exports_field(&json!({
                "../../utils/": "./dist/"
            })),
            request: "../../utils/index",
            conditions: vec![],
        },
        TestCase {
            name: "backtracking package base #2 (wildcard)",
            expect: None,
            exports: exports_field(&json!({
                "../../utils/*": "./dist/*"
            })),
            request: "../../utils/index",
            conditions: vec![],
        },
        TestCase {
            name: "backtracking package base #3",
            expect: None,
            exports: exports_field(&json!({
                "./utils/": "../src/"
            })),
            request: "./utils/index",
            conditions: vec![],
        },
        TestCase {
            name: "backtracking package base #3 (wildcard)",
            expect: None,
            exports: exports_field(&json!({
                "./utils/*": "../src/*"
            })),
            request: "./utils/index",
            conditions: vec![],
        },
        TestCase {
            name: "backtracking package base #6",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                "./utils/../utils/index": "./src/../index.js"
            })),
            request: "./utils/index",
            conditions: vec![],
        },
        TestCase {
            name: "backtracking package base #7",
            expect: None,
            exports: exports_field(&json!({
                "./utils/": {
                    "browser": "../this/"
                }
            })),
            request: "./utils/index",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "backtracking package base #7",
            expect: None,
            exports: exports_field(&json!({
                "./utils/*": {
                    "browser": "../this/*"
                }
            })),
            request: "./utils/index",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "backtracking package base #8",
            expect: None,
            exports: exports_field(&json!({
                "./utils/": {
                    "browser": "./utils/../"
                }
            })),
            request: "./utils/index",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "backtracking package base #8 (wildcard)",
            expect: None,
            exports: exports_field(&json!({
                "./utils/*": {
                    "browser": "./utils/../*"
                }
            })),
            request: "./utils/index",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "backtracking package base #9",
            expect: Some(vec!["./dist/index"]),
            exports: exports_field(&json!({
                "./": "./src/../../",
                "./dist/": "./dist/"
            })),
            request: "./dist/index",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "backtracking package base #9 (wildcard)",
            expect: Some(vec!["./dist/index"]),
            exports: exports_field(&json!({
                "./*": "./src/../../*",
                "./dist/*": "./dist/*"
            })),
            request: "./dist/index",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "backtracking target folder #1",
            expect: None,
            exports: exports_field(&json!({
                "./utils/": "./dist/"
            })),
            request: "./utils/timezone/../../index",
            conditions: vec![],
        },
        TestCase {
            name: "backtracking target folder #1 (wildcard)",
            expect: None,
            exports: exports_field(&json!({
                "./utils/*": "./dist/*"
            })),
            request: "./utils/timezone/../../index",
            conditions: vec![],
        },
        TestCase {
            name: "backtracking target folder #2",
            expect: None,
            exports: exports_field(&json!({
                "./utils/": "./dist/"
            })),
            request: "./utils/timezone/../index",
            conditions: vec![],
        },
        TestCase {
            name: "backtracking target folder #2 (wildcard)",
            expect: None,
            exports: exports_field(&json!({
                "./utils/*": "./dist/*"
            })),
            request: "./utils/timezone/../index",
            conditions: vec![],
        },
        TestCase {
            name: "backtracking target folder #3",
            expect: None,
            exports: exports_field(&json!({
                "./utils/": "./dist/target/"
            })),
            request: "./utils/../../index",
            conditions: vec![],
        },
        TestCase {
            name: "backtracking target folder #3 (wildcard)",
            expect: None,
            exports: exports_field(&json!({
                "./utils/*": "./dist/target/*"
            })),
            request: "./utils/../../index",
            conditions: vec![],
        },
        // enhanced-resolve does not handle `node_modules` in target
        TestCase {
            name: "nested node_modules path #1",
            expect: None,
            exports: exports_field(&json!({
                "./utils/": {
                    "browser": "./node_modules/"
                }
            })),
            request: "./utils/lodash/dist/index.js",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "nested node_modules path #1 (wildcard)",
            expect: None,
            exports: exports_field(&json!({
                "./utils/*": {
                    "browser": "./node_modules/*"
                }
            })),
            request: "./utils/lodash/dist/index.js",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "nested node_modules path #2",
            expect: None,
            exports: exports_field(&json!({
                "./utils/": "./utils/../node_modules/"
            })),
            request: "./utils/lodash/dist/index.js",
            conditions: vec![],
        },
        TestCase {
            name: "nested node_modules path #2 (wildcard)",
            expect: None,
            exports: exports_field(&json!({
                "./utils/*": "./utils/../node_modules/*"
            })),
            request: "./utils/lodash/dist/index.js",
            conditions: vec![],
        },
        TestCase {
            name: "nested mapping #1",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                "./utils/": {
                    "browser": {
                        "webpack": "./",
                        "default": {
                            "node": "./node/"
                        }
                    }
                }
            })),
            request: "./utils/index.js",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "nested mapping #1 (wildcard)",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                "./utils/*": {
                    "browser": {
                        "webpack": "./*",
                        "default": {
                            "node": "./node/*"
                        }
                    }
                }
            })),
            request: "./utils/index.js",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "nested mapping #2",
            expect: Some(vec!["./index.js"]),
            exports: exports_field(&json!({
                "./utils/": {
                    "browser": {
                        "webpack": [
                            "./",
                            "./node/"
                        ],
                        "default": {
                            "node": "./node/"
                        }
                    }
                }
            })),
            request: "./utils/index.js",
            conditions: vec!["browser", "webpack"],
        },
        // (test is repeated because we don't support returning an array)
        TestCase {
            name: "nested mapping #2",
            expect: Some(vec!["./node/index.js"]),
            exports: exports_field(&json!({
                "./utils/": {
                    "browser": {
                        "webpack": [
                            "./node/"
                        ],
                        "default": {
                            "node": "./node/"
                        }
                    }
                }
            })),
            request: "./utils/index.js",
            conditions: vec!["browser", "webpack"],
        },
        TestCase {
            name: "nested mapping #2 (wildcard)",
            expect: Some(vec!["./index.js"]),
            exports: exports_field(&json!({
                "./utils/*": {
                    "browser": {
                        "webpack": [
                            "./*",
                            "./node/*"
                        ],
                        "default": {
                            "node": "./node/*"
                        }
                    }
                }
            })),
            request: "./utils/index.js",
            conditions: vec!["browser", "webpack"],
        },
        // (test is repeated because we don't support returning an array)
        TestCase {
            name: "nested mapping #2 (wildcard)",
            expect: Some(vec!["./node/index.js"]),
            exports: exports_field(&json!({
                "./utils/*": {
                    "browser": {
                        "webpack": [
                            "./node/*"
                        ],
                        "default": {
                            "node": "./node/*"
                        }
                    }
                }
            })),
            request: "./utils/index.js",
            conditions: vec!["browser", "webpack"],
        },
        TestCase {
            name: "nested mapping #3",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                "./utils/": {
                    "browser": {
                        "webpack": [
                            "./",
                            "./node/"
                        ],
                        "default": {
                            "node": "./node/"
                        }
                    }
                }
            })),
            request: "./utils/index.js",
            conditions: vec!["webpack"],
        },
        TestCase {
            name: "nested mapping #3 (wildcard)",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                "./utils/*": {
                    "browser": {
                        "webpack": [
                            "./*",
                            "./node/*"
                        ],
                        "default": {
                            "node": "./node/*"
                        }
                    }
                }
            })),
            request: "./utils/index.js",
            conditions: vec!["webpack"],
        },
        TestCase {
            name: "nested mapping #4",
            expect: Some(vec!["./node/index.js"]),
            exports: exports_field(&json!({
                "./utils/": {
                    "browser": {
                        "webpack": [
                            "./",
                            "./node/"
                        ],
                        "default": {
                            "node": "./node/"
                        }
                    }
                }
            })),
            request: "./utils/index.js",
            conditions: vec!["node", "browser"],
        },
        TestCase {
            name: "nested mapping #4 (wildcard)",
            expect: Some(vec!["./node/index.js"]),
            exports: exports_field(&json!({
                "./utils/*": {
                    "browser": {
                        "webpack": [
                            "./*",
                            "./node/*"
                        ],
                        "default": {
                            "node": "./node/*"
                        }
                    }
                }
            })),
            request: "./utils/index.js",
            conditions: vec!["node", "browser"],
        },
        TestCase {
            name: "nested mapping #5",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                "./utils/": {
                    "browser": {
                        "webpack": [
                            "./",
                            "./node/"
                        ],
                        "default": {
                            "node": {
                                "webpack": [
                                    "./wpck/"
                                ]
                            }
                        }
                    }
                }
            })),
            request: "./utils/index.js",
            conditions: vec!["browser", "node"],
        },
        TestCase {
            name: "nested mapping #5 (wildcard)",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                "./utils/*": {
                    "browser": {
                        "webpack": [
                            "./*",
                            "./node/*"
                        ],
                        "default": {
                            "node": {
                                "webpack": [
                                    "./wpck/*"
                                ]
                            }
                        }
                    }
                }
            })),
            request: "./utils/index.js",
            conditions: vec!["browser", "node"],
        },
        TestCase {
            name: "nested mapping #6",
            expect: Some(vec!["./index.js"]),
            exports: exports_field(&json!({
                "./utils/": {
                    "browser": {
                        "webpack": [
                            "./",
                            "./node/"
                        ],
                        "default": {
                            "node": {
                                "webpack": [
                                    "./wpck/"
                                ]
                            }
                        }
                    }
                }
            })),
            request: "./utils/index.js",
            conditions: vec!["browser", "node", "webpack"],
        },
        // (test is repeated because we don't support returning an array)
        TestCase {
            name: "nested mapping #6",
            expect: Some(vec!["./node/index.js"]),
            exports: exports_field(&json!({
                "./utils/": {
                    "browser": {
                        "webpack": [
                            "./node/"
                        ],
                        "default": {
                            "node": {
                                "webpack": [
                                    "./wpck/"
                                ]
                            }
                        }
                    }
                }
            })),
            request: "./utils/index.js",
            conditions: vec!["browser", "node", "webpack"],
        },
        TestCase {
            name: "nested mapping #6 (wildcard)",
            expect: Some(vec!["./index.js"]),
            exports: exports_field(&json!({
                "./utils/*": {
                    "browser": {
                        "webpack": [
                            "./*",
                            "./node/*"
                        ],
                        "default": {
                            "node": {
                                "webpack": [
                                    "./wpck/*"
                                ]
                            }
                        }
                    }
                }
            })),
            request: "./utils/index.js",
            conditions: vec!["browser", "node", "webpack"],
        },
        // (test is repeated because we don't support returning an array)
        TestCase {
            name: "nested mapping #6 (wildcard)",
            expect: Some(vec!["./node/index.js"]),
            exports: exports_field(&json!({
                "./utils/*": {
                    "browser": {
                        "webpack": [
                            "./node/*"
                        ],
                        "default": {
                            "node": {
                                "webpack": [
                                    "./wpck/*"
                                ]
                            }
                        }
                    }
                }
            })),
            request: "./utils/index.js",
            conditions: vec!["browser", "node", "webpack"],
        },
        TestCase {
            name: "nested mapping #7",
            expect: Some(vec!["./y.js"]),
            exports: exports_field(&json!({
                "./a.js": {
                    "abc": {
                        "def": "./x.js"
                    },
                    "ghi": "./y.js"
                }
            })),
            request: "./a.js",
            conditions: vec!["abc", "ghi"],
        },
        TestCase {
            name: "nested mapping #8",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                "./a.js": {
                    "abc": {
                        "def": "./x.js",
                        "default": []
                    },
                    "ghi": "./y.js"
                }
            })),
            request: "./a.js",
            conditions: vec!["abc", "ghi"],
        },
        TestCase {
            name: "syntax sugar #1",
            expect: Some(vec!["./main.js"]),
            exports: exports_field(&json!("./main.js")),
            request: ".",
            conditions: vec![],
        },
        TestCase {
            name: "syntax sugar #2",
            expect: Some(vec![]),
            exports: exports_field(&json!("./main.js")),
            request: "./lib.js",
            conditions: vec![],
        },
        TestCase {
            name: "syntax sugar #3",
            expect: Some(vec!["./a.js"]),
            exports: exports_field(&json!(["./a.js", "./b.js"])),
            request: ".",
            conditions: vec![],
        },
        // (test is repeated because we don't support returning an array)
        TestCase {
            name: "syntax sugar #3",
            expect: Some(vec!["./b.js"]),
            exports: exports_field(&json!(["./b.js"])),
            request: ".",
            conditions: vec![],
        },
        TestCase {
            name: "syntax sugar #4",
            expect: Some(vec![]),
            exports: exports_field(&json!(["./a.js", "./b.js"])),
            request: "./lib.js",
            conditions: vec![],
        },
        TestCase {
            name: "syntax sugar #5",
            expect: Some(vec!["./index.js"]),
            exports: exports_field(&json!({
                "browser": {
                    "default": "./index.js"
                }
            })),
            request: ".",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "syntax sugar #6",
            expect: Some(vec![]),
            exports: exports_field(&json!({
                "browser": {
                    "default": "./index.js"
                }
            })),
            request: "./lib.js",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "syntax sugar #7",
            expect: None,
            exports: exports_field(&json!({
                "./node": "./node.js",
                "browser": {
                    "default": "./index.js"
                }
            })),
            request: ".",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "syntax sugar #8",
            expect: None,
            exports: exports_field(&json!({
                "browser": {
                    "default": "./index.js"
                },
                "./node": "./node.js"
            })),
            request: ".",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "wildcard longest #1",
            expect: Some(vec!["./abc/d"]),
            exports: exports_field(&json!({
                "./ab*": "./ab/*",
                "./abc*": "./abc/*",
                "./a*": "./a/*"
            })),
            request: "./abcd",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "wildcard longest #2",
            expect: Some(vec!["./abc/d/e"]),
            exports: exports_field(&json!({
                "./ab*": "./ab/*",
                "./abc*": "./abc/*",
                "./a*": "./a/*"
            })),
            request: "./abcd/e",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "wildcard longest #3",
            expect: Some(vec!["./abc/d"]),
            exports: exports_field(&json!({
                "./x/ab*": "./ab/*",
                "./x/abc*": "./abc/*",
                "./x/a*": "./a/*"
            })),
            request: "./x/abcd",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "wildcard longest #4",
            expect: Some(vec!["./abc/d/e"]),
            exports: exports_field(&json!({
                "./x/ab*": "./ab/*",
                "./x/abc*": "./abc/*",
                "./x/a*": "./a/*"
            })),
            request: "./x/abcd/e",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "path tree edge case #1",
            expect: Some(vec!["./A/b/d.js"]),
            exports: exports_field(&json!({
                "./a/": "./A/",
                "./a/b/c": "./c.js"
            })),
            request: "./a/b/d.js",
            conditions: vec![],
        },
        TestCase {
            name: "path tree edge case #1 (wildcard)",
            expect: Some(vec!["./A/b/d.js"]),
            exports: exports_field(&json!({
                "./a/*": "./A/*",
                "./a/b/c": "./c.js"
            })),
            request: "./a/b/d.js",
            conditions: vec![],
        },
        TestCase {
            name: "path tree edge case #2",
            expect: Some(vec!["./A/c.js"]),
            exports: exports_field(&json!({
                "./a/": "./A/",
                "./a/b": "./b.js"
            })),
            request: "./a/c.js",
            conditions: vec![],
        },
        TestCase {
            name: "path tree edge case #2 (wildcard)",
            expect: Some(vec!["./A/c.js"]),
            exports: exports_field(&json!({
                "./a/*": "./A/*",
                "./a/b": "./b.js"
            })),
            request: "./a/c.js",
            conditions: vec![],
        },
        TestCase {
            name: "path tree edge case #3",
            expect: Some(vec!["./A/b/d/c.js"]),
            exports: exports_field(&json!({
                "./a/": "./A/",
                "./a/b/c/d": "./c.js"
            })),
            request: "./a/b/d/c.js",
            conditions: vec![],
        },
        TestCase {
            name: "path tree edge case #3 (wildcard)",
            expect: Some(vec!["./A/b/d/c.js"]),
            exports: exports_field(&json!({
                "./a/*": "./A/*",
                "./a/b/c/d": "./c.js"
            })),
            request: "./a/b/d/c.js",
            conditions: vec![],
        },
        TestCase {
            name: "wildcard pattern #1",
            expect: Some(vec!["./A/b.js"]),
            exports: exports_field(&json!({
                "./a/*.js": "./A/*.js"
            })),
            request: "./a/b.js",
            conditions: vec![],
        },
        TestCase {
            name: "wildcard pattern #2",
            expect: Some(vec!["./A/b/c.js"]),
            exports: exports_field(&json!({
                "./a/*.js": "./A/*.js"
            })),
            request: "./a/b/c.js",
            conditions: vec![],
        },
        TestCase {
            name: "wildcard pattern #3",
            expect: Some(vec!["./A/b/c.js"]),
            exports: exports_field(&json!({
                "./a/*/c.js": "./A/*/c.js"
            })),
            request: "./a/b/c.js",
            conditions: vec![],
        },
        TestCase {
            name: "wildcard pattern #4",
            expect: Some(vec!["./A/b/b.js"]),
            exports: exports_field(&json!({
                "./a/*/c.js": "./A/*/*.js"
            })),
            request: "./a/b/c.js",
            conditions: vec![],
        },
        TestCase {
            name: "wildcard pattern #5",
            expect: Some(vec!["./browser/index.js"]),
            exports: exports_field(&json!({
                "./lib/*": {
                    "browser": [
                        "./browser/*"
                    ]
                },
                "./dist/*.js": {
                    "node": "./*.js",
                    "default": "./browser/*.js"
                }
            })),
            request: "./dist/index.js",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "wildcard pattern #5",
            expect: Some(vec!["./browser/index.js"]),
            exports: exports_field(&json!({
                "./lib/*": {
                    "browser": [
                        "./browser/*"
                    ]
                },
                "./dist/*.js": {
                    "node": "./*.js",
                    "default": "./browser/*.js"
                }
            })),
            request: "./lib/index.js",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "wildcard pattern #6",
            expect: Some(vec!["./browser/foo/bar.js"]),
            exports: exports_field(&json!({
                "./lib/*/bar.js": {
                    "browser": [
                        "./browser/*/bar.js"
                    ]
                },
                "./dist/*/bar.js": {
                    "node": "./*.js",
                    "default": "./browser/*.js"
                }
            })),
            request: "./lib/foo/bar.js",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "wildcard pattern #6",
            expect: Some(vec!["./browser/foo.js"]),
            exports: exports_field(&json!({
                "./lib/*/bar.js": {
                    "browser": [
                        "./browser/*/bar.js"
                    ]
                },
                "./dist/*/bar.js": {
                    "node": "./*.js",
                    "default": "./browser/*.js"
                }
            })),
            request: "./dist/foo/bar.js",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "wildcard pattern #7",
            expect: Some(vec!["./browser/foo/default.js"]),
            exports: exports_field(&json!({
                "./lib/*/bar.js": {
                    "browser": [
                        "./browser/*/bar.js"
                    ]
                },
                "./dist/*/bar.js": {
                    "node": "./*.js",
                    "default": "./browser/*/default.js"
                }
            })),
            request: "./dist/foo/bar.js",
            conditions: vec!["default"],
        },
        TestCase {
            name: "wildcard pattern #8",
            expect: Some(vec!["./A/b/b/b.js"]),
            exports: exports_field(&json!({
                "./a/*/c.js": "./A/*/*/*.js"
            })),
            request: "./a/b/c.js",
            conditions: vec![],
        },
        TestCase {
            name: "wildcard pattern #9",
            expect: Some(vec!["./A/b/b/b.js"]),
            exports: exports_field(&json!({
                "./a/*/c.js": [
                    "./A/*/*/*.js",
                    "./B/*/*/*.js"
                ]
            })),
            request: "./a/b/c.js",
            conditions: vec![],
        },
        // (test is repeated because we don't support returning an array)
        TestCase {
            name: "wildcard pattern #9",
            expect: Some(vec!["./B/b/b/b.js"]),
            exports: exports_field(&json!({
                "./a/*/c.js": [
                    "./B/*/*/*.js"
                ]
            })),
            request: "./a/b/c.js",
            conditions: vec![],
        },
        TestCase {
            name: "wildcard pattern #10",
            expect: Some(vec!["./A/b/b/b.js"]),
            exports: exports_field(&json!({
                "./a/foo-*/c.js": "./A/*/*/*.js"
            })),
            request: "./a/foo-b/c.js",
            conditions: vec![],
        },
        TestCase {
            name: "wildcard pattern #11",
            expect: Some(vec!["./A/b/b/b.js"]),
            exports: exports_field(&json!({
                "./a/*-foo/c.js": "./A/*/*/*.js"
            })),
            request: "./a/b-foo/c.js",
            conditions: vec![],
        },
        TestCase {
            name: "wildcard pattern #12",
            expect: Some(vec!["./A/b/b/b.js"]),
            exports: exports_field(&json!({
                "./a/foo-*-foo/c.js": "./A/*/*/*.js"
            })),
            request: "./a/foo-b-foo/c.js",
            conditions: vec![],
        },
        TestCase {
            name: "wildcard pattern #13",
            expect: Some(vec!["./A/b/c/d.js"]),
            exports: exports_field(&json!({
                "./a/foo-*-foo/c.js": "./A/b/c/d.js"
            })),
            request: "./a/foo-b-foo/c.js",
            conditions: vec![],
        },
        TestCase {
            name: "wildcard pattern #14",
            expect: Some(vec!["./A/b/c/*.js"]),
            exports: exports_field(&json!({
                "./a/foo-foo/c.js": "./A/b/c/*.js"
            })),
            request: "./a/foo-foo/c.js",
            conditions: vec![],
        },
    ];

    for case in test_cases {
        let package_url = Path::new(".");
        let condition_names = case
            .conditions
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let file_system = MemoryFileSystem::default();
        let resolver = Resolver::for_test_file_system(
            Arc::new(file_system),
            ResolverOptions {
                conditions: condition_names,
                ..ResolverOptions::default()
            },
        );
        let resolved_path = resolver.resolve_package_exports_field(
            package_url,
            case.request,
            &case.exports,
            crate::ResolverSearch::root(resolver.options()),
            &mut super::test_resolve_context(),
        );
        if let Some(expect) = case.expect {
            if expect.is_empty() {
                assert!(
                    matches!(
                        resolved_path,
                        Err(ResolverError::PackagePathNotExported { .. })
                    ),
                    "{} {:?}",
                    &case.name,
                    &resolved_path
                );
            } else {
                for expect in expect {
                    assert_eq!(
                        resolved_path,
                        Ok(Some(Resolution::path_only(
                            package_url.normalize_with(expect)
                        ))),
                        "{}",
                        &case.name
                    );
                }
            }
        } else {
            assert!(resolved_path.is_err(), "{} {resolved_path:?}", &case.name);
        }
    }
}

/// Stop exports array fallback when the target mapping shape is invalid.
#[test]
fn test_resolve_exports_field_array_stops_on_invalid_mapping_shape() {
    let resolver = Resolver::for_test_file_system(
        Arc::new(MemoryFileSystem::default()),
        ResolverOptions::default(),
    );
    let exports = json!({
        "./a/": [
            "./bad",
            "./ok/"
        ]
    });

    let resolved_path = resolver.resolve_package_exports_field(
        Path::new("."),
        "./a/file.js",
        &exports,
        crate::ResolverSearch::root(resolver.options()),
        &mut super::test_resolve_context(),
    );

    assert_eq!(
        resolved_path,
        Err(ResolverError::InvalidPackageConfigDirectory {
            path: Path::new(".").normalize_with("package.json")
        })
    );
}
