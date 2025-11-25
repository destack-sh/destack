use dyst_source::MemoryFileSystem;
use indexmap::IndexMap;

use super::{fixture, fixture_root};
use crate::resolve::{Resolution, ResolveError, ResolveOptions};
use crate::{AliasValue, Resolver};

/// Test resolving a simple module.
#[test]
fn test_resolve_simple() {
    let dirname = fixture_root();
    let f = dirname.join("enhanced_resolve/test");

    let resolver = Resolver::blank(ResolveOptions::default());

    let data = [
        ("direct", f.clone(), "../lib/index"),
        ("as directory", f, ".."),
        ("as module", dirname.clone(), "./enhanced_resolve"),
    ];

    for (comment, path, request) in data {
        let resolved_path = resolver.resolve(&path, request).map(|f| f.full_path());
        let expected = dirname.join("enhanced_resolve/lib/index.js");
        assert_eq!(resolved_path, Ok(expected), "{comment} {path:?} {request}");
    }
}

/// Test resolving a module with a dashed name.
#[test]
fn test_resolve_dashed_name() {
    let f = fixture();

    let resolver = Resolver::blank(ResolveOptions::default());

    let data = [
        (f.clone(), "dash", f.join("node_modules/dash/index.js")),
        (
            f.clone(),
            "dash-name",
            f.join("node_modules/dash-name/index.js"),
        ),
        (
            f.join("node_modules/dash"),
            "dash",
            f.join("node_modules/dash/index.js"),
        ),
        (
            f.join("node_modules/dash"),
            "dash-name",
            f.join("node_modules/dash-name/index.js"),
        ),
        (
            f.join("node_modules/dash-name"),
            "dash",
            f.join("node_modules/dash/index.js"),
        ),
        (
            f.join("node_modules/dash-name"),
            "dash-name",
            f.join("node_modules/dash-name/index.js"),
        ),
    ];

    for (path, request, expected) in data {
        let resolution = resolver.resolve(&path, request).ok();
        let resolved_path = resolution.as_ref().map(|r| r.full_path());
        assert_eq!(resolved_path, Some(expected), "{path:?} {request}");
    }
}

/// Run the tests from the enhanced-resolve test suite (webpack).
/// https://github.com/webpack/enhanced-resolve/tree/main/test/fixtures
#[test]
fn test_resolve_enhanced_resolve() {
    let f = fixture();
    let resolver = Resolver::blank(ResolveOptions::default());
    let main1_js_path = f.join("main1.js").to_string_lossy().to_string();

    #[rustfmt::skip]
    let pass = [
        ("absolute path", f.clone(), main1_js_path.as_str(), f.join("main1.js")),
        ("file with .js", f.clone(), "./main1.js", f.join("main1.js")),
        ("file without extension", f.clone(), "./main1", f.join("main1.js")),
        ("another file with .js", f.clone(), "./a.js", f.join("a.js")),
        ("another file without extension", f.clone(), "./a", f.join("a.js")),
        ("file in module with .js", f.clone(), "m1/a.js", f.join("node_modules/m1/a.js")),
        ("file in module without extension", f.clone(), "m1/a", f.join("node_modules/m1/a.js")),
        ("another file in module without extension", f.clone(), "complexm/step1", f.join("node_modules/complexm/step1.js")),
        ("from submodule to file in sibling module", f.join("node_modules/complexm"), "m2/b.js", f.join("node_modules/m2/b.js")),
        ("from nested directory to overwritten file in module", f.join("multiple_modules"), "m1/a.js", f.join("multiple_modules/node_modules/m1/a.js")),
        ("from nested directory to not overwritten file in module", f.join("multiple_modules"), "m1/b.js", f.join("node_modules/m1/b.js")),
        ("file with query", f.clone(), "./main1.js?query", f.join("main1.js?query")),
        ("file with fragment", f.clone(), "./main1.js#fragment", f.join("main1.js#fragment")),
        ("file with fragment and query", f.clone(), "./main1.js#fragment?query", f.join("main1.js#fragment?query")),
        ("file with query and fragment", f.clone(), "./main1.js?#fragment", f.join("main1.js?#fragment")),

        ("file with query (unicode)", f.clone(), "./测试.js?query", f.join("测试.js?query")),
        ("file with fragment (unicode)", f.clone(), "./测试.js#fragment", f.join("测试.js#fragment")),
        ("file with fragment and query (unicode)", f.clone(), "./测试.js#fragment?query", f.join("测试.js#fragment?query")),
        ("file with query and fragment (unicode)", f.clone(), "./测试.js?#fragment", f.join("测试.js?#fragment")),

        ("file in module with query", f.clone(), "m1/a?query", f.join("node_modules/m1/a.js?query")),
        ("file in module with fragment", f.clone(), "m1/a#fragment", f.join("node_modules/m1/a.js#fragment")),
        ("file in module with fragment and query", f.clone(), "m1/a#fragment?query", f.join("node_modules/m1/a.js#fragment?query")),
        ("file in module with query and fragment", f.clone(), "m1/a?#fragment", f.join("node_modules/m1/a.js?#fragment")),
        ("file in module with query and fragment", f.clone(), "m1/a?#fragment", f.join("node_modules/m1/a.js?#fragment")),
        ("differ between directory and file, resolve file", f.clone(), "./dirOrFile", f.join("dirOrFile.js")),
        ("differ between directory and file, resolve directory", f.clone(), "./dirOrFile/", f.join("dirOrFile/index.js")),
        ("find node_modules outside of node_modules", f.join("browser-module/node_modules"), "m1/a", f.join("node_modules/m1/a.js")),
        ("don't crash on main field pointing to self", f.clone(), "./main-field-self", f.join("./main-field-self/index.js")),
        ("don't crash on main field pointing to self (2)", f.clone(), "./main-field-self2", f.join("./main-field-self2/index.js")),
        
        ("handle fragment edge case (no fragment)", f.clone(), "./no#fragment/#/#", f.join("no#fragment/#/#.js")),
        ("handle fragment edge case (fragment)", f.clone(), "./no#fragment/#/", f.join("no.js#fragment/#/")),
        ("handle fragment escaping", f.clone(), "./no\0#fragment/\0#/\0##fragment", f.join("no#fragment/#/#.js#fragment")),
    ];

    for (comment, path, request, expected) in pass {
        let resolution = resolver.resolve(&path, request).ok();
        let resolved_path = resolution.as_ref().map(Resolution::full_path);
        assert_eq!(
            resolved_path,
            Some(expected),
            "{comment} {path:?} {request}"
        );
    }
}

/// (Not entirely sure where issue #238 is from.)
#[test]
fn test_resolve_issue238() {
    let f = fixture().join("issue-238");
    let resolver = Resolver::blank(ResolveOptions {
        extensions: vec![".js".into(), ".jsx".into(), ".ts".into(), ".tsx".into()],
        modules: vec![
            "src/a".into(),
            "src/b".into(),
            "src/common".into(),
            "node_modules".into(),
        ],
        ..ResolveOptions::default()
    });
    let resolved_path = resolver
        .resolve(f.join("src/common"), "config/myObjectFile")
        .map(|r| r.full_path());
    assert_eq!(
        resolved_path,
        Ok(f.join("src/common/config/myObjectFile.js"))
    );
}

/// Test the `prefer_relative` option (should prefer relative paths over node_modules).
#[test]
fn test_resolve_prefer_relative() {
    let f = fixture();

    let resolver = Resolver::blank(ResolveOptions {
        prefer_relative: true,
        ..ResolveOptions::default()
    });

    #[rustfmt::skip]
    let pass = [
        ("should correctly resolve with preferRelative 1", "main1.js", f.join("main1.js")),
        ("should correctly resolve with preferRelative 2", "m1/a.js", f.join("node_modules/m1/a.js")),
    ];

    for (comment, request, expected) in pass {
        let resolved_path = resolver.resolve(&f, request).map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {request}");
    }
}

#[test]
fn test_resolve_directory() {
    let f = fixture();
    let resolver = Resolver::blank(ResolveOptions {
        resolve_directory: true,
        ..ResolveOptions::default()
    });

    #[rustfmt::skip]
    let data = [
        ("context for fixtures", f.clone(), "./", f.clone()),
        ("context for fixtures/lib", f.clone(), "./lib", f.join("lib")),
        ("context for fixtures with ..", f.clone(), "./lib/../../fixtures/./lib/..", f.clone()),
        ("context for fixtures with query", f.clone(), "./?query", f.clone().with_file_name("fixtures?query")),
    ];

    for (comment, path, request, expected) in data {
        let resolved_path = resolver.resolve(&path, request).map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {path:?} {request}");
    }
}

/// Test resolving a specifier with a hash in it.
#[test]
fn test_resolve_hash_as_module() {
    let f = fixture();
    let resolver = Resolver::blank(ResolveOptions::default());
    let resolution = resolver.resolve(f, "#a");
    assert_eq!(
        resolution,
        Err(ResolveError::NotFound {
            specifier: "#a".into()
        })
    );
}

#[test]
fn test_resolve_edge_cases() {
    let f = fixture();
    let resolver = Resolver::blank(ResolveOptions::default());

    #[rustfmt::skip]
    let data = [(
        "resolve with multiple dots",
        f.clone(),
        "./a/../main1.js",
        f.join("main1.js"),
    )];

    for (comment, path, request, expected) in data {
        let resolved_path = resolver.resolve(&path, request).map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {path:?} {request}");
    }
}

/// Test resolving a specifier with "dot", "dir", "slash", etc. spelled out.
#[test]
fn test_resolve_dot_spelled_out() {
    let f = fixture_root().join("dot");
    let foo_dir: std::path::PathBuf = f.join("foo");
    let resolver = Resolver::blank(ResolveOptions::default());
    let foo_index = foo_dir.join("index.js");

    #[rustfmt::skip]
    let data = [
        ("dot dir", foo_dir.clone(), ".", foo_index.clone()),
        ("dot dir slash", foo_dir.clone(), "./", foo_index),
    ];
    for (comment, path, request, expected) in data {
        let resolved_path = resolver.resolve(&path, request).map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {path:?} {request}");
    }

    let resolver = Resolver::blank(ResolveOptions {
        main_files: vec![],
        ..ResolveOptions::default()
    });
    #[rustfmt::skip]
    let data = [
        (
            "dot dir",
            foo_dir.clone(),
            ".",
            ResolveError::NotFound { specifier: ".".into() },
        ),
        (
            "dot dir slash",
            foo_dir,
            "./",
            ResolveError::NotFound { specifier: "./".into() },
        ),
    ];
    for (comment, path, request, expected) in data {
        let resolve_error = resolver.resolve(&path, request);
        assert_eq!(resolve_error, Err(expected), "{comment} {path:?} {request}");
    }
}

/// Test resolving abnormal relative paths (e.g., with "../..").
#[test]
fn test_resolve_abnormal_relative() {
    let f = fixture_root().join("abnormal-relative-with-node_modules");

    let base = f.join("foo/bar/baz");

    let resolver = Resolver::blank(ResolveOptions::default());

    #[rustfmt::skip]
    let data = [
        ("2-level abnormal relative path 1", "jest-runner-../../.."),
        ("2-level abnormal relative path 2", "jest-runner-../../../"),
        ("2-level abnormal relative path 3", "jest-runner-/../.."),
        ("2-level abnormal relative path 4", "jest-runner-/../../"),
    ];

    for (comment, request) in data {
        let resolved_path = resolver
            .resolve(&base, request)
            .map(|r| r.full_path())
            .unwrap();
        assert_eq!(
            resolved_path,
            f.join("runner.js"),
            "{comment} {}",
            resolved_path.display()
        );
    }

    #[rustfmt::skip]
    let data = [
        ("1-level abnormal relative path 1", "jest-runner-../.."),
        ("1-level abnormal relative path 2", "jest-runner-../../"),
        ("1-level abnormal relative path 3", "jest-runner-/.."),
        ("1-level abnormal relative path 4", "jest-runner-/../"),
    ];

    for (comment, request) in data {
        let resolved_path = resolver.resolve(&base, request);
        assert_eq!(
            resolved_path,
            Err(ResolveError::NotFound {
                specifier: request.into()
            }),
            "{comment} {request}"
        );
    }

    let f = fixture_root().join("abnormal-relative-without-node_modules");

    let base = f.join("foo/bar/baz");

    let data = [
        ("2-level abnormal relative path 1", "jest-runner-../../.."),
        ("2-level abnormal relative path 2", "jest-runner-../../../"),
        ("2-level abnormal relative path 3", "jest-runner-/../.."),
        ("2-level abnormal relative path 4", "jest-runner-/../../"),
    ];

    for (comment, request) in data {
        let resolved_path = resolver.resolve(&base, request);
        assert_eq!(
            resolved_path,
            Err(ResolveError::NotFound {
                specifier: request.into()
            }),
            "{comment} {request}"
        );
    }
}

/// Test resolving a directory / specifier with Chinese characters.
#[test]
fn test_resolve_chinese() {
    let dir = fixture_root();
    let specifier = "./misc/中文/中文.js";
    let resolution = Resolver::blank(ResolveOptions::default()).resolve(&dir, specifier);
    assert_eq!(
        resolution.map(Resolution::into_path_buf),
        Ok(dir.join("misc/中文/中文.js"))
    );
}

/// Test resolving against the styled-components package.
#[test]
fn test_resolve_styled_components() {
    let dir = fixture_root();
    let path = dir.join("pnpm");
    let module_path = path
        .join("node_modules/.pnpm")
        .join(
            "styled-components@6.1.17_react-dom@19.2.0_react@19.2.0__react@19.2.0/node_modules/styled-components",
        );
    let specifier = "styled-components";

    let options = ResolveOptions::default();
    let resolution = Resolver::blank(options).resolve(&path, specifier);
    assert_eq!(
        resolution.map(|r| r.full_path()),
        Ok(module_path.join("dist/styled-components.browser.cjs.js"))
    );
}

/// Test resolving against the axios package with various conditions.
#[test]
fn test_resolve_axios() {
    let dir = fixture_root();
    let path = dir.join("pnpm");
    let module_path = path.join("node_modules/.pnpm/axios@1.8.4/node_modules/axios");
    let specifier = "axios";

    let options = ResolveOptions::default();
    let resolution = Resolver::blank(options).resolve(&path, specifier);
    assert_eq!(
        resolution.map(|r| r.full_path()),
        Ok(module_path.join("index.js"))
    );

    let options = ResolveOptions {
        conditions: vec!["browser".into(), "require".into()],
        ..ResolveOptions::default()
    };
    let resolution = Resolver::blank(options).resolve(&path, specifier);
    assert_eq!(
        resolution.map(|r| r.full_path()),
        Ok(module_path.join("dist/browser/axios.cjs"))
    );

    let options = ResolveOptions {
        conditions: vec!["node".into(), "require".into()],
        ..ResolveOptions::default()
    };
    let resolution = Resolver::blank(options).resolve(&path, specifier);
    assert_eq!(
        resolution.map(|r| r.full_path()),
        Ok(module_path.join("dist/node/axios.cjs"))
    );
}

/// Test resolving against the postcss package.
#[test]
fn test_resolve_postcss() {
    let dir = fixture_root();
    let path = dir.join("pnpm");
    let module_path = path.join("node_modules/postcss");
    let resolver = Resolver::blank(ResolveOptions {
        canonicalize_symlinks: false,
        ..ResolveOptions::default()
    });

    let resolution = resolver.resolve(&module_path, "path");
    assert_eq!(
        resolution,
        Err(ResolveError::Ignored {
            path: module_path.clone()
        })
    );

    let resolution = resolver.resolve(&module_path, "./lib/terminal-highlight");
    assert_eq!(
        resolution,
        Err(ResolveError::Ignored {
            path: module_path.join("lib/terminal-highlight")
        })
    );
}

/// Test resolving against the ipaddr.js package.
#[test]
fn test_resolve_ipaddr_js() {
    let dir = fixture_root();
    let path = dir.join("pnpm");
    let module_path =
        path.join("node_modules/.pnpm/ipaddr.js@2.2.0/node_modules/ipaddr.js/lib/ipaddr.js");

    let resolvers = [
        Resolver::blank(ResolveOptions {
            extension_alias: IndexMap::from([(
                ".js".into(),
                vec![".js".into(), ".ts".into(), ".tsx".into()],
            )]),
            ..ResolveOptions::default()
        }),
        Resolver::blank(ResolveOptions {
            extensions: vec![".ts".into()],
            ..ResolveOptions::default()
        }),
        Resolver::blank(ResolveOptions::default()),
    ];

    for resolver in resolvers {
        let resolution = resolver.resolve(&path, "ipaddr.js").map(|r| r.full_path());
        assert_eq!(resolution, Ok(module_path.clone()));
    }
}

/// Test resolving against the decimal.js package.
#[test]
fn test_resolve_decimal_js() {
    let dir = fixture_root();
    let path = dir.join("pnpm");
    let module_path =
        path.join("node_modules/.pnpm/decimal.js@10.5.0/node_modules/decimal.js/decimal.mjs");

    let resolvers = [
        Resolver::blank(ResolveOptions {
            extension_alias: IndexMap::from([(
                ".js".into(),
                vec![".js".into(), ".ts".into(), ".tsx".into()],
            )]),
            conditions: vec!["import".into()],
            ..ResolveOptions::default()
        }),
        Resolver::blank(ResolveOptions {
            conditions: vec!["import".into()],
            ..ResolveOptions::default()
        }),
    ];

    for resolver in resolvers {
        let resolution = resolver.resolve(&path, "decimal.js").map(|r| r.full_path());
        assert_eq!(resolution, Ok(module_path.clone()));
    }
}

/// Test resolving against the decimal.js package from the mathjs package.
#[test]
fn test_resolve_decimal_js_from_mathjs() {
    let dir = fixture_root();
    let path = dir.join("pnpm/node_modules/.pnpm/mathjs@13.2.0/node_modules/mathjs/lib/esm");
    let module_path =
        dir.join("pnpm/node_modules/.pnpm/decimal.js@10.5.0/node_modules/decimal.js/decimal.mjs");

    let resolvers = [
        Resolver::blank(ResolveOptions {
            extension_alias: IndexMap::from([(
                ".js".into(),
                vec![".js".into(), ".ts".into(), ".tsx".into()],
            )]),
            conditions: vec!["import".into()],
            ..ResolveOptions::default()
        }),
        Resolver::blank(ResolveOptions {
            conditions: vec!["import".into()],
            ..ResolveOptions::default()
        }),
    ];

    for resolver in resolvers {
        let resolution = resolver.resolve(&path, "decimal.js").map(|r| r.full_path());
        assert_eq!(resolution, Ok(module_path.clone()));
    }
}

/// Test resolving against the minimatch package.
#[test]
fn test_resolve_minimatch() {
    let dir = fixture_root();
    let path = dir.join("pnpm");
    let resolver = Resolver::blank(ResolveOptions {
        conditions: vec!["import".into()],
        ..ResolveOptions::default()
    });
    let resolution = resolver.resolve(&path, "minimatch").unwrap();
    assert_eq!(
        resolution.full_path(),
        dir.join(
            "pnpm/node_modules/.pnpm/minimatch@10.0.1/node_modules/minimatch/dist/esm/index.js",
        )
    );

    let resolver = Resolver::blank(ResolveOptions {
        conditions: vec!["require".into()],
        ..ResolveOptions::default()
    });
    let resolution = resolver.resolve(&path, "minimatch").unwrap();
    assert_eq!(
        resolution.full_path(),
        dir.join(
            "pnpm/node_modules/.pnpm/minimatch@10.0.1/node_modules/minimatch/dist/commonjs/index.js",
        )
    );
}

/// Test resolving against nested symlinks.
#[test]
fn test_resolve_nested_symlinks() {
    let dir = fixture_root().join("nested-symlink");
    assert_eq!(
        Resolver::blank(ResolveOptions::default())
            .resolve(&dir, "./apps/web/nm/@repo/typescript-config/index.js")
            .map(Resolution::into_path_buf),
        Ok(dir.join("nm/index.js"))
    );
    assert_eq!(
        Resolver::blank(ResolveOptions::default())
            .resolve(&dir, "./apps/tooling/typescript-config/index.js")
            .map(Resolution::into_path_buf),
        Ok(dir.join("nm/index.js"))
    );
}

/// Test resolving against a package.json with a BOM.
#[test]
fn test_resolve_package_json_with_bom() {
    let dir = fixture_root().join("misc");
    assert_eq!(
        Resolver::blank(ResolveOptions::default())
            .resolve(&dir, "./package-json-with-bom")
            .map(Resolution::into_path_buf),
        Ok(dir.join("package-json-with-bom/index.js"))
    );
}

/// Test resolving on Windows (should normalize the path).
#[cfg(windows)]
#[test]
fn test_resolve_normalized_on_windows() {
    use dyst_source::PathExt;

    let f = fixture();
    let absolute = f.join("./foo/index.js").normalize();
    let absolute_str = absolute.to_string_lossy();
    let normalized_absolute = absolute_str.replace('\\', "/");
    let resolver = Resolver::blank(ResolveOptions::default());

    let resolution = resolver
        .resolve(&f, &normalized_absolute)
        .map(|r| r.full_path());
    assert_eq!(
        resolution.map(|r| r.to_string_lossy().into_owned()),
        Ok(absolute_str.clone().into_owned())
    );

    let normalized_f = f.to_str().unwrap().replace('\\', "/");
    let resolution = resolver
        .resolve(normalized_f, ".\\foo\\index.js")
        .map(|r| r.full_path());
    assert_eq!(
        resolution.map(|r| r.to_string_lossy().into_owned()),
        Ok(absolute_str.clone().into_owned())
    );
}

/// Test resolving against a file protocol path.
#[cfg(windows)]
#[test]
fn test_resolve_file_protocol() {
    use url::Url;

    let f = fixture();

    let main1_js_path = f.join("main1.js").to_string_lossy().to_string();
    let file_protocol_path = Url::from_file_path(main1_js_path.clone()).unwrap();

    let resolver = Resolver::blank(ResolveOptions::default());

    let resolution = resolver.resolve(&f, file_protocol_path.as_str()).ok();
    let resolved_path = resolution.as_ref().map(Resolution::full_path);
    assert_eq!(resolved_path, Some(f.join("main1.js")));

    let resolve_error = ResolveError::NotFound {
        specifier: "\\\\.\\main.js".into(),
    };

    assert_eq!(resolver.resolve(f, "file://./main.js"), Err(resolve_error));
}

/// Test resolving against scoped packages.
/// https://github.com/webpack/enhanced-resolve/blob/main/test/scoped-packages.test.js
#[test]
fn test_resolve_scoped_packages() {
    let f = fixture().join("scoped");
    let resolver = Resolver::blank(ResolveOptions::default());

    #[rustfmt::skip]
    let pass = [
        ("main field should work", f.clone(), "@scope/pack1", "@scope/pack1", f.join("./node_modules/@scope/pack1/main.js")),
        ("browser field should work", f.clone(), "@scope/pack2", "@scope/pack2", f.join("./node_modules/@scope/pack2/main.js")),
        ("folder request should work", f.clone(), "@scope/pack2/lib", "@scope/pack2", f.join("./node_modules/@scope/pack2/lib/index.js"))
    ];

    for (comment, path, request, _, expected) in pass {
        let resolution = resolver.resolve(&path, request).ok();
        let resolved_path = resolution.as_ref().map(Resolution::full_path);
        assert_eq!(
            resolved_path,
            Some(expected),
            "{comment} {path:?} {request}"
        );
    }
}

/// Test resolving against the roots option.
/// https://github.com/webpack/enhanced-resolve/blob/main/test/roots.test.js>
#[test]
fn test_resolve_roots() {
    let f = super::fixture();

    let resolver = Resolver::blank(ResolveOptions {
        extensions: vec![".js".into()],
        alias: vec![("foo".into(), vec![AliasValue::from("/fixtures")])],
        roots: vec![
            fixture_root().join("enhanced_resolve").join("test"),
            f.clone(),
        ],
        ..ResolveOptions::default()
    });

    #[rustfmt::skip]
    let pass = [
        ("should respect roots option", "/fixtures/b.js", f.join("b.js")),
        ("should try another root option, if it exists", "/b.js", f.join("b.js")),
        ("should respect extension", "/fixtures/b", f.join("b.js")),
        ("should resolve in directory", "/fixtures/extensions/dir", f.join("extensions/dir/index.js")),
        ("should respect aliases", "foo/b", f.join("b.js")),
    ];

    for (comment, request, expected) in pass {
        let resolved_path = resolver.resolve(&f, request).map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {request}");
    }

    #[rustfmt::skip]
    let fail = [
        ("should not work with relative path", "fixtures/b.js", ResolveError::NotFound { specifier: "fixtures/b.js".into() })
    ];

    for (comment, request, expected) in fail {
        let resolution = resolver.resolve(&f, request);
        assert_eq!(resolution, Err(expected), "{comment} {request}");
    }
}

#[test]
fn test_prefer_absolute() {
    let f = super::fixture();
    let resolver = Resolver::blank(ResolveOptions {
        extensions: vec![".js".into()],
        alias: vec![("foo".into(), vec![AliasValue::from("/fixtures")])],
        roots: vec![
            fixture_root().join("enhanced_resolve").join("test"),
            f.clone(),
        ],
        prefer_absolute: true,
        ..ResolveOptions::default()
    });

    #[rustfmt::skip]
    let pass = [
        ("should resolve an absolute path (prefer absolute)", f.join("b.js").to_string_lossy().to_string(), f.join("b.js")),
    ];

    for (comment, request, expected) in pass {
        let resolved_path = resolver.resolve(&f, &request).map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {request}");
    }
}

#[test]
fn test_roots_fall_through() {
    let f = super::fixture();
    let absolute_path = f.join("roots_fall_through/index.js");
    let specifier = absolute_path.to_string_lossy();
    let mut options = ResolveOptions::default();
    options.roots.push(f.clone());
    let resolution = Resolver::blank(options).resolve(&f, &specifier);
    assert_eq!(resolution.map(Resolution::into_path_buf), Ok(absolute_path));
}

#[test]
fn test_should_resolve_slash() {
    let f = super::fixture();
    let dir_with_index = super::fixture_root().join("./misc/dir-with-index");

    #[rustfmt::skip]
    let pass = [
        ("should resolve if importer is root", vec![dir_with_index.clone()], &dir_with_index, dir_with_index.join("index.js")),
    ];

    for (comment, roots, directory, expected) in pass {
        let resolver = Resolver::blank(ResolveOptions {
            roots: roots.clone(),
            ..ResolveOptions::default()
        });
        let resolved_path = resolver.resolve(directory, "/").map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {roots:?}");
    }

    #[rustfmt::skip]
    let fail = [
        ("should not resolve if not found", vec![f.clone()], &f),
        ("should not resolve if importer is not root", vec![dir_with_index], &f)
    ];

    for (comment, roots, directory) in fail {
        let resolver = Resolver::blank(ResolveOptions {
            roots: roots.clone(),
            ..ResolveOptions::default()
        });
        let resolution = resolver.resolve(directory, "/");
        assert_eq!(
            resolution,
            Err(ResolveError::NotFound {
                specifier: "/".into()
            }),
            "{comment} {roots:?}"
        );
    }
}

/// Test resolving against a fully specified path.
/// https://github.com/webpack/enhanced-resolve/blob/main/test/fullSpecified.test.js
#[test]
#[cfg(not(target_os = "windows"))]
fn test_resolve_fully_specified_paths() {
    use std::sync::Arc;

    let fs = Arc::new(MemoryFileSystem::from_files(&[
        ("/a/node_modules/package1/index.js", ""),
        ("/a/node_modules/package1/file.js", ""),
        ("/a/node_modules/package2/package.json", r#"{"main":"a"}"#),
        ("/a/node_modules/package2/a.js", ""),
        ("/a/node_modules/package3/package.json", r#"{"main":"dir"}"#),
        ("/a/node_modules/package3/dir/index.js", ""),
        (
            "/a/node_modules/package4/package.json",
            r#"{"browser":{"./a.js":"./b"}}"#,
        ),
        ("/a/node_modules/package4/a.js", ""),
        ("/a/node_modules/package4/b.js", ""),
        ("/a/abc.js", ""),
        ("/a/dir/index.js", ""),
        ("/a/index.js", ""),
    ]));

    let resolver = Resolver::blank_with_fs(
        fs.clone(),
        ResolveOptions {
            alias: vec![
                ("alias1".into(), vec![AliasValue::from("/a/abc")]),
                ("alias2".into(), vec![AliasValue::from("/a")]),
            ],
            is_fully_specified: true,
            ..ResolveOptions::default()
        },
    );

    let failing_resolves = [
        ("no extensions", "./abc"),
        ("no extensions (absolute)", "/a/abc"),
        ("no extensions in packages", "package1/file"),
        ("no directories", "."),
        ("no directories 2", "./"),
        ("no directories in packages", "package3/dir"),
        ("no extensions in packages 2", "package3/a"),
    ];

    for (comment, request) in failing_resolves {
        let resolution = resolver.resolve("/a", request);
        assert!(resolution.is_err(), "{comment} {request}");
    }

    let successful_resolves = [
        ("fully relative", "./abc.js", "/a/abc.js"),
        ("fully absolute", "/a/abc.js", "/a/abc.js"),
        (
            "fully relative in package",
            "package1/file.js",
            "/a/node_modules/package1/file.js",
        ),
        (
            "extensions in mainFiles",
            "package1",
            "/a/node_modules/package1/index.js",
        ),
        (
            "extensions in mainFields",
            "package2",
            "/a/node_modules/package2/a.js",
        ),
        ("extensions in alias", "alias1", "/a/abc.js"),
        ("directories in alias", "alias2", "/a/index.js"),
        (
            "directories in packages",
            "package3",
            "/a/node_modules/package3/dir/index.js",
        ),
    ];

    for (comment, request, expected) in successful_resolves {
        use std::path::PathBuf;

        let resolution = resolver.resolve("/a", request).map(|r| r.full_path());
        assert_eq!(
            resolution,
            Ok(PathBuf::from(expected)),
            "{comment} {request}"
        );
    }

    let resolver = Resolver::blank_with_fs(
        fs.clone(),
        ResolveOptions {
            alias: vec![
                ("alias1".into(), vec![AliasValue::from("/a/abc")]),
                ("alias2".into(), vec![AliasValue::from("/a")]),
            ],
            is_fully_specified: true,
            resolve_directory: true,
            ..ResolveOptions::default()
        },
    );

    let successful_resolves = [
        ("current folder", ".", "/a"),
        ("current folder 2", "./", "/a"),
        ("relative directory", "./dir", "/a/dir"),
        ("relative directory 2", "./dir/", "/a/dir"),
        (
            "relative directory with query and fragment",
            "./dir?123#456",
            "/a/dir?123#456",
        ),
        (
            "relative directory with query and fragment 2",
            "./dir/?123#456",
            "/a/dir?123#456",
        ),
        ("absolute directory", "/a/dir", "/a/dir"),
        (
            "directory in package",
            "package3/dir",
            "/a/node_modules/package3/dir",
        ),
    ];

    for (comment, request, expected) in successful_resolves {
        use std::path::PathBuf;

        let resolution = resolver.resolve("/a", request).map(|r| r.full_path());
        assert_eq!(
            resolution,
            Ok(PathBuf::from(expected)),
            "{comment} {request}"
        );
    }
}

#[cfg(not(target_os = "windows"))] // MemoryFS's path separator is always `/` so the test will not pass in windows.
mod windows {
    use crate::{ResolveOptions, Resolver};
    use dyst_source::MemoryFileSystem;

    #[test]
    fn test_resolve_no_package() {
        use std::path::Path;
        use std::sync::Arc;

        let f = Path::new("/");
        let fs = MemoryFileSystem::from_files(&[]);
        let resolver = Resolver::blank_with_fs(Arc::new(fs), ResolveOptions::default());
        let resolved_path = resolver.resolve(f, "package");
        assert!(resolved_path.is_err());
    }
}
