//! <https://github.com/webpack/enhanced-resolve/blob/main/test/alias.test.js>

use std::path::Path;
#[cfg(not(target_os = "windows"))]
use std::path::PathBuf;

use indexmap::IndexMap;

use crate::{AliasValue, Resolution, Resolver, ResolverError, ResolverOptions};
#[cfg(not(target_os = "windows"))]
use destack_source::MemoryFileSystem;

/// Test resolving aliases.
#[allow(clippy::too_many_lines)]
#[test]
#[cfg(not(target_os = "windows"))]
fn test_resolve_alias() {
    use std::sync::Arc;

    let f = Path::new("/");

    let fs = MemoryFileSystem::from_files(&[
        ("/a/index.js", ""),
        ("/a/dir/index.js", ""),
        ("/recursive/index.js", ""),
        ("/recursive/dir/index.js", ""),
        ("/b/index.js", ""),
        ("/b/dir/index.js", ""),
        ("/c/index.js", ""),
        ("/c/dir/index.js", ""),
        ("/d/index.js.js", ""),
        ("/d/dir/.empty", ""),
        ("/e/index.js", ""),
        ("/e/anotherDir/index.js", ""),
        ("/e/dir/file", ""),
        ("/dashed-name", ""),
    ]);

    let resolver = Resolver::for_test_file_system(
        Arc::new(fs),
        ResolverOptions {
            alias: vec![
                ("aliasA".into(), vec![AliasValue::from("a")]),
                ("b$".into(), vec![AliasValue::from("a/index.js")]),
                ("c$".into(), vec![AliasValue::from("/a/index.js")]),
                (
                    "multiAlias".into(),
                    vec![
                        AliasValue::from("b"),
                        AliasValue::from("c"),
                        AliasValue::from("d"),
                        AliasValue::from("e"),
                        AliasValue::from("a"),
                    ],
                ),
                ("recursive".into(), vec![AliasValue::from("recursive/dir")]),
                ("/d/dir".into(), vec![AliasValue::from("/c/dir")]),
                ("/d/index.js".into(), vec![AliasValue::from("/c/index.js")]),
                ("#".into(), vec![AliasValue::from("/c/dir")]),
                ("@".into(), vec![AliasValue::from("/c/dir")]),
                ("ignored".into(), vec![AliasValue::Ignore]),
                // not part of enhanced-resolve, added to make sure query in alias value would work
                (
                    "alias_query".into(),
                    vec![AliasValue::from("a?query_after")],
                ),
                (
                    "alias_fragment".into(),
                    vec![AliasValue::from("a#fragment_after")],
                ),
                ("dash".into(), vec![AliasValue::Ignore]),
                (
                    "@scope/package-name/file$".into(),
                    vec![AliasValue::from("/c/dir")],
                ),
                // wildcard https://github.com/webpack/enhanced-resolve/pull/439
                ("@adir/*".into(), vec![AliasValue::from("./a/")]), // added to test value without wildcard
                ("@*".into(), vec![AliasValue::from("/*")]),
                ("@e*".into(), vec![AliasValue::from("/e/*")]),
                ("@e*file".into(), vec![AliasValue::from("/e*file")]),
            ],
            modules: vec!["/".into()],
            ..ResolverOptions::default()
        },
    );

    #[rustfmt::skip]
    let pass = [
        ("should resolve a not aliased module 1", "a", "/a/index.js"),
        ("should resolve a not aliased module 2", "a/index.js", "/a/index.js"),
        ("should resolve a not aliased module 3", "a/dir", "/a/dir/index.js"),
        ("should resolve a not aliased module 4", "a/dir/index.js", "/a/dir/index.js"),
        ("should resolve an aliased module 1", "aliasA", "/a/index.js"),
        ("should resolve an aliased module 2", "aliasA/index.js", "/a/index.js"),
        ("should resolve an aliased module 3", "aliasA/dir", "/a/dir/index.js"),
        ("should resolve an aliased module 4", "aliasA/dir/index.js", "/a/dir/index.js"),
        ("should resolve '#' alias 1", "#", "/c/dir/index.js"),
        ("should resolve '#' alias 2", "#/index.js", "/c/dir/index.js"),
        ("should resolve '@' alias 1", "@", "/c/dir/index.js"),
        ("should resolve '@' alias 2", "@/index.js", "/c/dir/index.js"),
        ("should resolve '@' alias 3", "@/", "/c/dir/index.js"),
        ("should resolve a recursive aliased module 1", "recursive", "/recursive/dir/index.js"),
        ("should resolve a recursive aliased module 2", "recursive/index.js", "/recursive/dir/index.js"),
        ("should resolve a recursive aliased module 3", "recursive/dir", "/recursive/dir/index.js"),
        ("should resolve a recursive aliased module 4", "recursive/dir/index.js", "/recursive/dir/index.js"),
        ("should resolve a file aliased module 1", "b", "/a/index.js"),
        ("should resolve a file aliased module 2", "c", "/a/index.js"),
        ("should resolve a file aliased module with a query 1", "b?query", "/a/index.js?query"),
        ("should resolve a file aliased module with a query 2", "c?query", "/a/index.js?query"),
        ("should resolve a path in a file aliased module 1", "b/index.js", "/b/index.js"),
        ("should resolve a path in a file aliased module 2", "b/dir", "/b/dir/index.js"),
        ("should resolve a path in a file aliased module 3", "b/dir/index.js", "/b/dir/index.js"),
        ("should resolve a path in a file aliased module 4", "c/index.js", "/c/index.js"),
        ("should resolve a path in a file aliased module 5", "c/dir", "/c/dir/index.js"),
        ("should resolve a path in a file aliased module 6", "c/dir/index.js", "/c/dir/index.js"),
        ("should resolve a file aliased file 1", "d", "/c/index.js"),
        ("should resolve a file aliased file 2", "d/dir/index.js", "/c/dir/index.js"),
        ("should resolve a file in multiple aliased dirs 1", "multiAlias/dir/file", "/e/dir/file"),
        ("should resolve a file in multiple aliased dirs 2", "multiAlias/anotherDir", "/e/anotherDir/index.js"),
        // wildcard
        ("should resolve wildcard alias 1", "@a", "/a/index.js"),
        ("should resolve wildcard alias 2", "@a/dir", "/a/dir/index.js"),
        ("should resolve wildcard alias 3", "@e/dir/file", "/e/dir/file"),
        ("should resolve wildcard alias 4", "@e/anotherDir", "/e/anotherDir/index.js"),
        ("should resolve wildcard alias 5", "@e/dir/file", "/e/dir/file"),
        // added to test value without wildcard
        ("should resolve scoped package name with sub dir 1", "@adir/index.js", "/a/index.js"),
        ("should resolve scoped package name with sub dir 2", "@adir/dir", "/a/index.js"),
        // not part of enhanced-resolve, added to make sure query in alias value works
        ("should resolve query in alias value", "alias_query?query_before", "/a/index.js?query_after"),
        ("should resolve query in alias value", "alias_fragment#fragment_before", "/a/index.js#fragment_after"),
        ("should resolve dashed name", "dashed-name", "/dashed-name"),
        ("should resolve scoped package name with sub dir", "@scope/package-name/file", "/c/dir/index.js"),
    ];

    for (comment, request, expected) in pass {
        let resolved_path = resolver
            .resolve_test_directory(f, request)
            .map(|r| r.full_path());
        assert_eq!(
            resolved_path,
            Ok(PathBuf::from(expected)),
            "{comment} {request}"
        );
    }

    #[rustfmt::skip]
    let ignore = [
        ("should resolve an ignore module", "ignored", ResolverError::Ignored { path: f.join("ignored") })
    ];

    for (comment, request, expected) in ignore {
        let resolution = resolver.resolve_test_directory(f, request);
        assert_eq!(resolution, Err(expected), "{comment} {request}");
    }
}

/// Test resolving infinite alias recursion.
#[test]
fn test_resolve_infinite_alias_recursion() {
    let f = super::fixture();
    let resolver = Resolver::for_tests(ResolverOptions {
        alias: vec![
            ("./a".into(), vec![AliasValue::from("./b")]),
            ("./b".into(), vec![AliasValue::from("./a")]),
        ],
        ..ResolverOptions::default()
    });
    let resolution = resolver.resolve_test_directory(f, "./a");
    assert_eq!(
        resolution,
        Err(ResolverError::RecursiveDependency { depth: 64 })
    );
}

fn check_os_path_slashes(path: &Path) {
    let s = path.to_string_lossy().to_string();
    #[cfg(target_os = "windows")]
    {
        assert!(!s.contains('/'), "{s}");
        assert!(s.contains('\\'), "{s}");
    }
    #[cfg(not(target_os = "windows"))]
    {
        assert!(s.contains('/'), "{s}");
        assert!(!s.contains('\\'), "{s}");
    }
}

/// Test resolving an alias to an absolute path.
#[test]
fn test_resolve_alias_to_absolute_path() {
    let f = super::fixture();
    let resolver = Resolver::for_tests(ResolverOptions {
        alias: vec![(
            f.join("foo").to_str().unwrap().to_string(),
            vec![AliasValue::Ignore],
        )],
        modules: vec![f.clone().to_str().unwrap().to_string()],
        ..ResolverOptions::default()
    });
    let resolution = resolver.resolve_test_directory(&f, "foo/index");
    assert_eq!(
        resolution,
        Err(ResolverError::Ignored {
            path: f.join("foo")
        })
    );
}

/// Test resolving an alias to a system path.
#[test]
fn test_resolve_alias_to_system_path() {
    let f = super::fixture();
    let resolver = Resolver::for_tests(ResolverOptions {
        alias: vec![(
            "@app".into(),
            vec![AliasValue::from(f.join("alias").to_string_lossy())],
        )],
        ..ResolverOptions::default()
    });

    let specifiers = ["@app/files/a", "@app/files/a.js"];

    for specifier in specifiers {
        let path = resolver
            .resolve_test_directory(&f, specifier)
            .map(Resolution::into_path_buf)
            .unwrap();
        assert_eq!(path, f.join("alias/files/a.js"));
        check_os_path_slashes(&path);
    }
}

/// Test resolving an alias to a full path.
#[test]
fn test_resolve_alias_is_full_path() {
    let f = super::fixture();
    let dir = f.join("foo");
    let dir_str = dir.to_string_lossy().to_string();

    let resolver = Resolver::for_tests(ResolverOptions {
        alias: vec![("@".into(), vec![AliasValue::Path(dir_str.clone())])],
        ..ResolverOptions::default()
    });

    let specifiers = [
        "@/index".to_string(),
        "@/index.js".to_string(),
        // specifier has multiple `/` for reasons we'll never know
        "@////index".to_string(),
        // specifier is a full path
        dir_str,
    ];

    let mut dependencies = Vec::new();
    for specifier in specifiers {
        let resolution = resolver.resolve_test_directory_with_dependencies(&f, &specifier);
        let resolution = resolution.map(|(resolution, observed_dependencies)| {
            dependencies.extend(observed_dependencies);
            resolution.full_path()
        });

        assert_eq!(resolution, Ok(dir.join("index.js")));
    }

    assert!(!dependencies.is_empty());
}

/// Test resolving an alias to a non-existent path.
#[test]
fn test_resolve_all_alias_values_are_not_found() {
    let f = super::fixture();
    let resolver = Resolver::for_tests(ResolverOptions {
        alias: vec![(
            "m1".to_string(),
            vec![AliasValue::Path(
                f.join("node_modules")
                    .join("m2")
                    .to_string_lossy()
                    .to_string(),
            )],
        )],
        ..ResolverOptions::default()
    });
    let resolution = resolver.resolve_test_directory(&f, "m1/a.js");
    assert_eq!(
        resolution,
        Err(ResolverError::MatchedAliasNotFound {
            specifier: "m1/a.js".to_string(),
            alias_key: "m1".to_string()
        })
    );
}

/// Test resolving an alias with a fragment.
#[test]
fn test_resolve_alias_with_fragment() {
    let f = super::fixture();

    let data = [
        // enhanced-resolve has `#` prepended with a `\0`, they are removed from the
        // following 3 expected test results.
        // See https://github.com/webpack/enhanced-resolve#escaping
        (
            "handle fragment edge case (no fragment)",
            "./no#fragment/#/#",
            f.join("no#fragment/#/#.js"),
        ),
        (
            "handle fragment edge case (fragment)",
            "./no#fragment/#/",
            f.join("no.js#fragment/#/"),
        ),
        (
            "handle fragment escaping",
            "./no\0#fragment/\0#/\0##fragment",
            f.join("no#fragment/#/#.js#fragment"),
        ),
    ];

    for (comment, request, expected) in data {
        let resolver = Resolver::for_tests(ResolverOptions {
            alias: vec![(
                "foo".to_string(),
                vec![AliasValue::Path(request.to_string())],
            )],
            ..ResolverOptions::default()
        });
        let resolved_path = resolver
            .resolve_test_directory(&f, "foo")
            .map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {request}");
    }
}

/// Test resolving an alias with a fragment as a path.
#[test]
fn test_resolve_alias_try_fragment_as_path() {
    let f = super::fixture();
    let resolver = Resolver::for_tests(ResolverOptions {
        alias: vec![(
            "#".to_string(),
            vec![AliasValue::Path(f.join("#").to_string_lossy().to_string())],
        )],
        ..ResolverOptions::default()
    });
    let resolution = resolver
        .resolve_test_directory(&f, "#/a")
        .map(|r| r.full_path());
    assert_eq!(resolution, Ok(f.join("#").join("a.js")));
}

/// Test resolving an alias with multiple fallbacks.
#[test]
fn test_resolve_alias_with_multiple_fallbacks() {
    let f = super::fixture();
    let resolver = Resolver::for_tests(ResolverOptions {
        alias: vec![(
            "multi".to_string(),
            vec![
                AliasValue::Path(f.join("nonexistent").to_string_lossy().to_string()),
                AliasValue::Path(f.join("foo").to_string_lossy().to_string()),
            ],
        )],
        ..ResolverOptions::default()
    });
    let resolution = resolver
        .resolve_test_directory(&f, "multi/index.js")
        .map(|r| r.full_path());
    assert_eq!(resolution, Ok(f.join("foo/index.js")));
}

/// Test resolving extension aliases.
#[test]
fn test_resolve_extension_alias() {
    let f = super::fixture().join("extension-alias");

    let resolver = Resolver::for_tests(ResolverOptions {
        extensions: vec![".js".into()],
        main_files: vec!["index.js".into()],
        extension_alias: IndexMap::from([
            (".js".into(), vec![".ts".into(), ".js".into()]),
            (".mjs".into(), vec![".mts".into()]),
        ]),
        ..ResolverOptions::default()
    });

    #[rustfmt::skip]
    let pass = [
        ("should alias fully specified file", f.clone(), "./index.js", f.join("index.ts")),
        (
            "should alias fully specified file when there are two alternatives",
            f.clone(),
            "./dir/index.js",
            f.join("dir/index.ts"),
        ),
        (
            "should also allow the second alternative",
            f.clone(),
            "./dir2/index.js",
            f.join("dir2/index.js"),
        ),
        (
            "should support alias option without an array",
            f.clone(),
            "./dir2/index.mjs",
            f.join("dir2/index.mts"),
        ),
    ];

    for (comment, path, request, expected) in pass {
        let resolved_path = resolver
            .resolve_test_directory(&path, request)
            .map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {path:?} {request}");
    }

    // should not allow to fallback to the original extension or add extensions
    let resolution = resolver
        .resolve_test_directory(&f, "./index.mjs")
        .unwrap_err();
    let expected = ResolverError::ExtensionAliasNotFound {
        filename: "index.mjs".into(),
        tried: "index.mts".into(),
        dir: f,
    };
    assert_eq!(resolution, expected);

    #[cfg(all(not(target_os = "windows"), target_endian = "little"))]
    {
        let resolver = Resolver::for_tests(ResolverOptions {
            extension_alias: IndexMap::from([(".js".into(), vec![".ts".into(), ".d.ts".into()])]),
            ..ResolverOptions::default()
        });

        let f = super::fixture_root().join("yarn");

        let resolution = resolver
            .resolve_test_directory(&f, "typescript/lib/typescript.js")
            .map(|r| r.full_path());
        assert_eq!(
            resolution,
            Ok(f.join("node_modules/typescript/lib/typescript.d.ts"))
        );
    }
}

/// Test resolving extension aliases that do not apply to main files.
#[test]
fn test_resolve_extension_alias_do_not_apply_to_main_files() {
    let f = super::fixture().join("extension-alias");

    let resolver = Resolver::for_tests(ResolverOptions {
        extensions: vec![".js".into()],
        main_files: vec!["index".into()],
        extension_alias: IndexMap::from([(".js".into(), vec![])]),
        ..ResolverOptions::default()
    });

    #[rustfmt::skip]
    let pass = [
        ("directory", f.clone(), "./dir2", "dir2/index.js"),
        ("file", f.clone(), "./dir2/index", "dir2/index.js"),
    ];

    for (comment, path, request, expected) in pass {
        let resolved_path = resolver
            .resolve_test_directory(&path, request)
            .map(|r| r.full_path());
        let expected = f.join(expected);
        assert_eq!(resolved_path, Ok(expected), "{comment} {path:?} {request}");
    }
}
