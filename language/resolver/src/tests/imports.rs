//! https://github.com/webpack/enhanced-resolve/blob/main/test/importsField.test.js

use destack_source::{MemoryFileSystem, PathExt};
use std::path::Path;
use std::sync::Arc;

use crate::{Resolution, Resolver, ResolverError, ResolverOptions};

/// Test simple imports field resolution.
#[test]
fn test_imports_field_simple() {
    let f = super::fixture().join("imports-field");
    let f2 = super::fixture().join("imports-exports-wildcard/node_modules/m/");

    let resolver = Resolver::for_tests(ResolverOptions {
        extensions: vec![".js".into()],
        main_files: vec!["index".into()],
        conditions: vec!["webpack".into()],
        ..ResolverOptions::default()
    });

    #[rustfmt::skip]
    let pass = [
        ("should resolve using imports field instead of self-referencing", f.clone(), "#imports-field", f.join("b.js")),
        ("should resolve query", f.clone(), "#query", f.join("a.js?query")),
        ("should resolve fragment", f.clone(), "#fragment", f.join("a.js#fragment")),
        ("should resolve using imports field instead of self-referencing for a subpath", f.join("dir"), "#imports-field", f.join("b.js")),
        ("should resolve package #1", f.clone(), "#a/dist/main.js", f.join("node_modules/a/lib/lib2/main.js")),
        ("should resolve package #3", f.clone(), "#ccc/index.js", f.join("node_modules/c/index.js")),
        ("should resolve package #4", f.clone(), "#c", f.join("node_modules/c/index.js")),
        ("should resolve with wildcard pattern", f2.clone(), "#internal/i.js", f2.join("src/internal/i.js")),
    ];

    for (comment, path, request, expected) in pass {
        let resolved_path = resolver
            .resolve_test_directory(&path, request)
            .map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {path:?} {request}");
    }

    // added cases:
    // * should resolve absolute path as an imports field target
    // * should log the correct details

    #[rustfmt::skip]
    let fail = [
        ("should disallow resolve out of package scope", f.clone(), "#b", ResolverError::InvalidPackageTarget { target: "../b.js".to_string(), name: "#b".to_string(), package_path: f.join("package.json") }),
        ("should resolve package #2", f.clone(), "#a", ResolverError::PackageImportNotDefined { specifier: "#a".to_string(), package_path: f.join("package.json") }),
    ];

    for (comment, path, request, error) in fail {
        let resolution = resolver.resolve_test_directory(&path, request);
        assert_eq!(resolution, Err(error), "{comment} {path:?} {request}");
    }
}

/// Return not found when package imports resolution is disabled.
#[test]
fn test_imports_field_disabled_returns_not_found() {
    let f = super::fixture().join("imports-field");

    let resolver = Resolver::for_tests(ResolverOptions {
        extensions: vec![".js".into()],
        resolve_package_json_imports: false,
        ..ResolverOptions::default()
    });

    let resolution = resolver.resolve_test_directory(&f, "#imports-field");
    assert_eq!(
        resolution,
        Err(ResolverError::NotFound {
            specifier: "#imports-field".into()
        })
    );
}

struct TestCase {
    #[allow(dead_code)]
    name: &'static str,
    expect: Option<Vec<&'static str>>,
    imports: serde_json::Map<String, serde_json::Value>,
    request: &'static str,
    conditions: Vec<&'static str>,
}

fn imports_field(value: &serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
    // clone and leak the value to get a 'static reference for big-endian
    let value = Box::leak::<'static>(Box::new(value.clone()));
    let serde_json::Value::Object(map) = value else {
        panic!("Expected an object");
    };
    map.clone()
}

/// Test various imports field cases.
#[allow(clippy::too_many_lines)]
#[test]
fn test_imports_field_cases() {
    use serde_json::json;
    let test_cases = vec![
        TestCase {
            name: "sample #1",
            expect: Some(vec!["./dist/test/file.js"]),
            imports: imports_field(&json!({
              "#abc/": {
                "import": [
                  "./dist/",
                  "./src/"
                ],
                "webpack": "./wp/"
              },
              "#abc": "./main.js"
            })),
            request: "#abc/test/file.js",
            conditions: vec!["import", "webpack"],
        },
        // (test is repeated because we don't support returning an array)
        TestCase {
            name: "sample #1",
            expect: Some(vec!["./src/test/file.js"]),
            imports: imports_field(&json!({
              "#abc/": {
                "import": [
                  "./src/"
                ],
                "webpack": "./wp/"
              },
              "#abc": "./main.js"
            })),
            request: "#abc/test/file.js",
            conditions: vec!["import", "webpack"],
        },
        TestCase {
            name: "sample #2",
            expect: Some(vec!["./data/timezones/pdt.mjs"]),
            imports: imports_field(&json!({
              "#1/timezones/": "./data/timezones/"
            })),
            request: "#1/timezones/pdt.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "sample #3",
            expect: Some(vec!["./data/timezones/timezones/pdt.mjs"]),
            imports: imports_field(&json!({
              "#aaa/": "./data/timezones/",
              "#a/": "./data/timezones/"
            })),
            request: "#a/timezones/pdt.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "sample #4",
            expect: Some(vec![]),
            imports: imports_field(&json!({
              "#a/lib/": {
                "browser": [
                  "./browser/"
                ]
              },
              "#a/dist/index.js": {
                "node": "./index.js"
              }
            })),
            request: "#a/dist/index.js",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "sample #5",
            expect: Some(vec!["./browser/index.js"]),
            imports: imports_field(&json!({
              "#a/lib/": {
                "browser": [
                  "./browser/"
                ]
              },
              "#a/dist/index.js": {
                "node": "./index.js",
                "default": "./browser/index.js"
              }
            })),
            request: "#a/dist/index.js",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "sample #6",
            expect: Some(vec![]),
            imports: imports_field(&json!({
              "#a/dist/a": "./dist/index.js"
            })),
            request: "#a/dist/aaa",
            conditions: vec![],
        },
        TestCase {
            name: "sample #7",
            expect: Some(vec![]),
            imports: imports_field(&json!({
              "#a/a/a/": "./dist/index.js"
            })),
            request: "#a/a/a",
            conditions: vec![],
        },
        TestCase {
            name: "sample #8",
            expect: Some(vec![]),
            imports: imports_field(&json!({
              "#a": "./index.js"
            })),
            request: "#a/timezones/pdt.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "sample #9",
            expect: Some(vec!["./main.js"]),
            imports: imports_field(&json!({
              "#a/index.js": "./main.js"
            })),
            request: "#a/index.js",
            conditions: vec![],
        },
        TestCase {
            name: "sample #10",
            expect: Some(vec!["./ok.js"]),
            imports: imports_field(&json!({
              "#a/#foo": "./ok.js",
              "#a/module": "./ok.js",
              "#a/🎉": "./ok.js",
              "#a/%F0%9F%8E%89": "./other.js",
              "#a/bar#foo": "./ok.js",
              "#a/#zapp/": "./"
            })),
            request: "#a/#foo",
            conditions: vec![],
        },
        TestCase {
            name: "sample #11",
            expect: Some(vec!["./ok.js"]),
            imports: imports_field(&json!({
              "#a/#foo": "./ok.js",
              "#a/module": "./ok.js",
              "#a/🎉": "./ok.js",
              "#a/%F0%9F%8E%89": "./other.js",
              "#a/bar#foo": "./ok.js",
              "#a/#zapp/": "./"
            })),
            request: "#a/bar#foo",
            conditions: vec![],
        },
        TestCase {
            name: "sample #12",
            expect: Some(vec!["./ok.js#abc"]),
            imports: imports_field(&json!({
              "#a/#foo": "./ok.js",
              "#a/module": "./ok.js",
              "#a/🎉": "./ok.js",
              "#a/%F0%9F%8E%89": "./other.js",
              "#a/bar#foo": "./ok.js",
              "#a/#zapp/": "./"
            })),
            request: "#a/#zapp/ok.js#abc",
            conditions: vec![],
        },
        TestCase {
            name: "sample #13",
            expect: Some(vec!["./ok.js?abc"]),
            imports: imports_field(&json!({
              "#a/#foo": "./ok.js",
              "#a/module": "./ok.js",
              "#a/🎉": "./ok.js",
              "#a/%F0%9F%8E%89": "./other.js",
              "#a/bar#foo": "./ok.js",
              "#a/#zapp/": "./"
            })),
            request: "#a/#zapp/ok.js?abc",
            conditions: vec![],
        },
        TestCase {
            name: "sample #14",
            expect: Some(vec!["./🎉.js"]),
            imports: imports_field(&json!({
              "#a/#foo": "./ok.js",
              "#a/module": "./ok.js",
              "#a/🎉": "./ok.js",
              "#a/%F0%9F%8E%89": "./other.js",
              "#a/bar#foo": "./ok.js",
              "#a/#zapp/": "./"
            })),
            request: "#a/#zapp/🎉.js",
            conditions: vec![],
        },
        TestCase {
            name: "sample #15",
            expect: Some(vec!["./%F0%9F%8E%89.js"]),
            imports: imports_field(&json!({
              "#a/#foo": "./ok.js",
              "#a/module": "./ok.js",
              "#a/🎉": "./ok.js",
              "#a/%F0%9F%8E%89": "./other.js",
              "#a/bar#foo": "./ok.js",
              "#a/#zapp/": "./"
            })),
            request: "#a/#zapp/%F0%9F%8E%89.js",
            conditions: vec![],
        },
        TestCase {
            name: "sample #16",
            expect: Some(vec!["./ok.js"]),
            imports: imports_field(&json!({
              "#a/#foo": "./ok.js",
              "#a/module": "./ok.js",
              "#a/🎉": "./ok.js",
              "#a/%F0%9F%8E%89": "./other.js",
              "#a/bar#foo": "./ok.js",
              "#a/#zapp/": "./"
            })),
            request: "#a/🎉",
            conditions: vec![],
        },
        TestCase {
            name: "sample #17",
            expect: Some(vec!["./other.js"]),
            imports: imports_field(&json!({
              "#a/#foo": "./ok.js",
              "#a/module": "./ok.js",
              "#a/🎉": "./ok.js",
              "#a/%F0%9F%8E%89": "./other.js",
              "#a/bar#foo": "./ok.js",
              "#a/#zapp/": "./"
            })),
            request: "#a/%F0%9F%8E%89",
            conditions: vec![],
        },
        TestCase {
            name: "sample #18",
            expect: Some(vec!["./ok.js"]),
            imports: imports_field(&json!({
              "#a/#foo": "./ok.js",
              "#a/module": "./ok.js",
              "#a/🎉": "./ok.js",
              "#a/%F0%9F%8E%89": "./other.js",
              "#a/bar#foo": "./ok.js",
              "#a/#zapp/": "./"
            })),
            request: "#a/module",
            conditions: vec![],
        },
        TestCase {
            name: "sample #19",
            expect: Some(vec![]),
            imports: imports_field(&json!({
              "#a/#foo": "./ok.js",
              "#a/module": "./ok.js",
              "#a/🎉": "./ok.js",
              "#a/%F0%9F%8E%89": "./other.js",
              "#a/bar#foo": "./ok.js",
              "#a/#zapp/": "./"
            })),
            request: "#a/module#foo",
            conditions: vec![],
        },
        TestCase {
            name: "sample #20",
            expect: Some(vec![]),
            imports: imports_field(&json!({
              "#a/#foo": "./ok.js",
              "#a/module": "./ok.js",
              "#a/🎉": "./ok.js",
              "#a/%F0%9F%8E%89": "./other.js",
              "#a/bar#foo": "./ok.js",
              "#a/#zapp/": "./"
            })),
            request: "#a/module?foo",
            conditions: vec![],
        },
        TestCase {
            name: "sample #21",
            expect: Some(vec!["./d?e?f"]),
            imports: imports_field(&json!({
              "#a/a?b?c/": "./"
            })),
            request: "#a/a?b?c/d?e?f",
            conditions: vec![],
        },
        TestCase {
            name: "sample #22",
            expect: None,
            imports: imports_field(&json!({
              "#a/": "/user/a/"
            })),
            request: "#a/index",
            conditions: vec![],
        },
        TestCase {
            name: "path tree edge case #1",
            expect: Some(vec!["./A/b/d.js"]),
            imports: imports_field(&json!({
              "#a/": "./A/",
              "#a/b/c": "./c.js"
            })),
            request: "#a/b/d.js",
            conditions: vec![],
        },
        TestCase {
            name: "path tree edge case #2",
            expect: Some(vec!["./A/c.js"]),
            imports: imports_field(&json!({
              "#a/": "./A/",
              "#a/b": "./b.js"
            })),
            request: "#a/c.js",
            conditions: vec![],
        },
        TestCase {
            name: "path tree edge case #3",
            expect: Some(vec!["./A/b/c/d.js"]),
            imports: imports_field(&json!({
              "#a/": "./A/",
              "#a/b/c/d": "./c.js"
            })),
            request: "#a/b/c/d.js",
            conditions: vec![],
        },
        TestCase {
            name: "Direct mapping #1",
            expect: Some(vec!["./dist/index.js"]),
            imports: imports_field(&json!({
              "#a": "./dist/index.js"
            })),
            request: "#a",
            conditions: vec![],
        },
        TestCase {
            name: "Direct mapping #2",
            expect: Some(vec![]),
            imports: imports_field(&json!({
              "#a/": "./"
            })),
            request: "#a",
            conditions: vec![],
        },
        TestCase {
            name: "Direct mapping #3",
            expect: Some(vec!["./dist/a.js"]),
            imports: imports_field(&json!({
              "#a/": "./dist/",
              "#a/index.js": "./dist/a.js"
            })),
            request: "#a/index.js",
            conditions: vec![],
        },
        TestCase {
            name: "Direct mapping #4",
            expect: Some(vec!["./index.js"]),
            imports: imports_field(&json!({
              "#a/": {
                "browser": [
                  "./browser/"
                ]
              },
              "#a/index.js": {
                "browser": "./index.js"
              }
            })),
            request: "#a/index.js",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "Direct mapping #5",
            expect: Some(vec![]),
            imports: imports_field(&json!({
              "#a/": {
                "browser": [
                  "./browser/"
                ]
              },
              "#a/index.js": {
                "node": "./node.js"
              }
            })),
            request: "#a/index.js",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "Direct mapping #6",
            expect: Some(vec!["./index.js"]),
            imports: imports_field(&json!({
              "#a": {
                "browser": "./index.js",
                "node": "./src/node/index.js",
                "default": "./src/index.js"
              }
            })),
            request: "#a",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "Direct mapping #7",
            expect: Some(vec!["./src/index.js"]), // `enhanced_resolve` is `None`
            imports: imports_field(&json!({
              "#a": {
                "default": "./src/index.js",
                "browser": "./index.js",
                "node": "./src/node/index.js"
              }
            })),
            request: "#a",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "Direct mapping #8",
            expect: Some(vec!["./src/index.js"]),
            imports: imports_field(&json!({
              "#a": {
                "browser": "./index.js",
                "node": "./src/node/index.js",
                "default": "./src/index.js"
              }
            })),
            request: "#a",
            conditions: vec![],
        },
        TestCase {
            name: "Direct mapping #9",
            expect: Some(vec!["./index"]),
            imports: imports_field(&json!({
              "#a": "./index"
            })),
            request: "#a",
            conditions: vec![],
        },
        TestCase {
            name: "Direct mapping #10",
            expect: Some(vec!["./index.js"]),
            imports: imports_field(&json!({
              "#a/index": "./index.js"
            })),
            request: "#a/index",
            conditions: vec![],
        },
        TestCase {
            name: "Direct mapping #11",
            expect: None,
            imports: imports_field(&json!({
              "#a": "b"
            })),
            request: "#a",
            conditions: vec![],
        },
        TestCase {
            name: "Direct mapping #12",
            expect: None,
            imports: imports_field(&json!({
              "#a/": "b/"
            })),
            request: "#a/index",
            conditions: vec![],
        },
        TestCase {
            name: "Direct mapping #13",
            expect: None,
            imports: imports_field(&json!({
              "#a?q=a#hashishere": "b#anotherhashishere"
            })),
            request: "#a?q=a#hashishere",
            conditions: vec![],
        },
        TestCase {
            name: "Direct and conditional mapping #1",
            expect: Some(vec![]),
            imports: imports_field(&json!({
              "#a": [
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
            request: "#a",
            conditions: vec![],
        },
        TestCase {
            name: "Direct and conditional mapping #2",
            expect: Some(vec!["./import.mjs"]),
            imports: imports_field(&json!({
              "#a": [
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
            request: "#a",
            conditions: vec!["import"],
        },
        TestCase {
            name: "Direct and conditional mapping #3",
            expect: Some(vec!["./require.js"]),
            imports: imports_field(&json!({
              "#a": [
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
            request: "#a",
            conditions: vec!["import", "require"],
        },
        // (test is repeated because we don't support returning an array)
        TestCase {
            name: "Direct and conditional mapping #3",
            expect: Some(vec!["./import.mjs"]),
            imports: imports_field(&json!({
              "#a": [
                {
                  "browser": "./browser.js"
                },
                {
                  "import": "./import.mjs"
                }
              ]
            })),
            request: "#a",
            conditions: vec!["import", "require"],
        },
        TestCase {
            name: "Direct and conditional mapping #4",
            expect: Some(vec!["./require.js"]),
            imports: imports_field(&json!({
              "#a": [
                {
                  "browser": "./browser.js"
                },
                {
                  "require": [
                    "./require.js"
                  ]
                },
                {
                  "import": [
                    "./import.mjs",
                    "#b/import.js"
                  ]
                }
              ]
            })),
            request: "#a",
            conditions: vec!["import", "require"],
        },
        // (test is repeated because we don't support returning an array)
        TestCase {
            name: "Direct and conditional mapping #4",
            expect: Some(vec!["./import.mjs"]),
            imports: imports_field(&json!({
              "#a": [
                {
                  "browser": "./browser.js"
                },
                {
                  "import": [
                    "./import.mjs",
                    "#b/import.js"
                  ]
                }
              ]
            })),
            request: "#a",
            conditions: vec!["import", "require"],
        },
        // (test is repeated because we don't support returning an array)
        TestCase {
            name: "Direct and conditional mapping #4",
            expect: Some(vec![]),
            imports: imports_field(&json!({
              "#a": [
                {
                  "browser": "./browser.js"
                },
                {
                  "import": [
                    "#b/import.js"
                  ]
                }
              ]
            })),
            request: "#a",
            conditions: vec!["import", "require"],
        },
        TestCase {
            name: "mapping to a folder root #1",
            expect: Some(vec![]),
            imports: imports_field(&json!({
              "#timezones": "./data/timezones/"
            })),
            request: "#timezones/pdt.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "mapping to a folder root #2",
            expect: None,
            imports: imports_field(&json!({
              "#timezones/": "./data/timezones"
            })),
            request: "#timezones/pdt.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "mapping to a folder root #3",
            expect: Some(vec!["./data/timezones/pdt/index.mjs"]),
            imports: imports_field(&json!({
              "#timezones/pdt/": "./data/timezones/pdt/"
            })),
            request: "#timezones/pdt/index.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "mapping to a folder root #4",
            expect: Some(vec!["./timezones/pdt.mjs"]),
            imports: imports_field(&json!({
              "#a/": "./timezones/"
            })),
            request: "#a/pdt.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "mapping to a folder root #5",
            expect: Some(vec!["./timezones/pdt.mjs"]),
            imports: imports_field(&json!({
              "#a/": "./"
            })),
            request: "#a/timezones/pdt.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "mapping to a folder root #6",
            expect: None,
            imports: imports_field(&json!({
              "#a/": "."
            })),
            request: "#a/timezones/pdt.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "mapping to a folder root #7",
            expect: Some(vec![]),
            imports: imports_field(&json!({
              "#a": "./"
            })),
            request: "#a/timezones/pdt.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "the longest matching path prefix is prioritized #1",
            expect: Some(vec!["./lib/index.mjs"]),
            imports: imports_field(&json!({
              "#a/": "./",
              "#a/dist/": "./lib/"
            })),
            request: "#a/dist/index.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "the longest matching path prefix is prioritized #2",
            expect: Some(vec!["./dist/utils/index.js"]),
            imports: imports_field(&json!({
              "#a/dist/utils/": "./dist/utils/",
              "#a/dist/": "./lib/"
            })),
            request: "#a/dist/utils/index.js",
            conditions: vec![],
        },
        TestCase {
            name: "the longest matching path prefix is prioritized #3",
            expect: Some(vec!["./dist/utils/index.js"]),
            imports: imports_field(&json!({
              "#a/dist/utils/index.js": "./dist/utils/index.js",
              "#a/dist/utils/": "./dist/utils/index.mjs",
              "#a/dist/": "./lib/"
            })),
            request: "#a/dist/utils/index.js",
            conditions: vec![],
        },
        TestCase {
            name: "the longest matching path prefix is prioritized #4",
            expect: Some(vec!["./lib/index.mjs"]),
            imports: imports_field(&json!({
              "#a/": {
                "browser": "./browser/"
              },
              "#a/dist/": "./lib/"
            })),
            request: "#a/dist/index.mjs",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "conditional mapping folder #1",
            expect: Some(vec!["./utils/index.js"]),
            imports: imports_field(&json!({
              "#a/": {
                "browser": [
                  "lodash/",
                  "./utils/"
                ],
                "node": [
                  "./utils-node/"
                ]
              }
            })),
            request: "#a/index.js",
            conditions: vec!["browser"],
        },
        // (test is repeated because we don't support returning an array)
        TestCase {
            name: "conditional mapping folder #1",
            expect: Some(vec!["./utils/index.js"]),
            imports: imports_field(&json!({
              "#a/": {
                "browser": [
                  "./utils/"
                ],
                "node": [
                  "./utils-node/"
                ]
              }
            })),
            request: "#a/index.js",
            conditions: vec!["browser"],
        },
        TestCase {
            name: "conditional mapping folder #2",
            expect: Some(vec![]),
            imports: imports_field(&json!({
              "#a/": {
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
            request: "#a/index.mjs",
            conditions: vec![],
        },
        TestCase {
            name: "conditional mapping folder #3",
            expect: Some(vec!["./wpk/index.mjs"]),
            imports: imports_field(&json!({
              "#a/": {
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
            request: "#a/index.mjs",
            conditions: vec!["browser", "webpack"],
        },
    ];

    for case in test_cases {
        let package_url = Path::new(".");
        let resolver = Resolver::for_test_file_system(
            Arc::new(MemoryFileSystem::default()),
            ResolverOptions::default(),
        );
        let resolved_path = resolver.resolve_package_map(
            case.request,
            &case.imports,
            package_url,
            true,
            &case
                .conditions
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            crate::ResolverSearch::root(resolver.options()),
            &mut super::test_resolve_context(),
        );
        if let Some(expect) = case.expect {
            if expect.is_empty() {
                assert!(
                    matches!(resolved_path, Ok(None)),
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
