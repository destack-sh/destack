#[cfg(not(target_os = "windows"))]
use destack_source::MemoryFileSystem;
use indexmap::IndexMap;

use super::{fixture, fixture_root};
use crate::{AliasValue, Resolution, Resolver, ResolverError, ResolverOptions};

/// Test resolving a simple module.
#[test]
fn test_resolve_simple() {
    let dirname = fixture_root();
    let f = dirname.join("enhanced_resolve/test");

    let resolver = Resolver::for_tests(ResolverOptions::default());

    let data = [
        ("direct", f.clone(), "../lib/index"),
        ("as directory", f, ".."),
        ("as module", dirname.clone(), "./enhanced_resolve"),
    ];

    for (comment, path, request) in data {
        let resolved_path = resolver
            .resolve_test_directory(&path, request)
            .map(|f| f.full_path());
        let expected = dirname.join("enhanced_resolve/lib/index.js");
        assert_eq!(resolved_path, Ok(expected), "{comment} {path:?} {request}");
    }
}

/// Test resolving a module with a dashed name.
#[test]
fn test_resolve_dashed_name() {
    let f = fixture();

    let resolver = Resolver::for_tests(ResolverOptions::default());

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
        let resolution = resolver.resolve_test_directory(&path, request).ok();
        let resolved_path = resolution.as_ref().map(|r| r.full_path());
        assert_eq!(resolved_path, Some(expected), "{path:?} {request}");
    }
}

/// Run the tests from the enhanced-resolve test suite (webpack).
/// https://github.com/webpack/enhanced-resolve/tree/main/test/fixtures
#[test]
fn test_resolve_enhanced_resolve() {
    let f = fixture();
    let resolver = Resolver::for_tests(ResolverOptions::default());
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
        let resolution = resolver.resolve_test_directory(&path, request).ok();
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
    let resolver = Resolver::for_tests(ResolverOptions {
        extensions: vec![".js".into(), ".jsx".into(), ".ts".into(), ".tsx".into()],
        modules: vec![
            "src/a".into(),
            "src/b".into(),
            "src/common".into(),
            "node_modules".into(),
        ],
        ..ResolverOptions::default()
    });
    let resolved_path = resolver
        .resolve_test_directory(f.join("src/common"), "config/myObjectFile")
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

    let resolver = Resolver::for_tests(ResolverOptions {
        prefer_relative: true,
        ..ResolverOptions::default()
    });

    #[rustfmt::skip]
    let pass = [
        ("should correctly resolve with preferRelative 1", "main1.js", f.join("main1.js")),
        ("should correctly resolve with preferRelative 2", "m1/a.js", f.join("node_modules/m1/a.js")),
    ];

    for (comment, request, expected) in pass {
        let resolved_path = resolver
            .resolve_test_directory(&f, request)
            .map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {request}");
    }
}

#[test]
fn test_resolve_directory() {
    let f = fixture();
    let resolver = Resolver::for_tests(ResolverOptions {
        resolve_to_context: true,
        ..ResolverOptions::default()
    });

    #[rustfmt::skip]
    let data = [
        ("context for fixtures", f.clone(), "./", f.clone()),
        ("context for fixtures/lib", f.clone(), "./lib", f.join("lib")),
        ("context for fixtures with ..", f.clone(), "./lib/../../fixtures/./lib/..", f.clone()),
        ("context for fixtures with query", f.clone(), "./?query", f.clone().with_file_name("fixtures?query")),
    ];

    for (comment, path, request, expected) in data {
        let resolved_path = resolver
            .resolve_test_directory(&path, request)
            .map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {path:?} {request}");
    }
}

/// Test resolving a specifier with a hash in it.
#[test]
fn test_resolve_hash_as_module() {
    let f = fixture();
    let resolver = Resolver::for_tests(ResolverOptions::default());
    let resolution = resolver.resolve_test_directory(f, "#a");
    assert_eq!(
        resolution,
        Err(ResolverError::NotFound {
            specifier: "#a".into()
        })
    );
}

#[test]
fn test_resolve_edge_cases() {
    let f = fixture();
    let resolver = Resolver::for_tests(ResolverOptions::default());

    #[rustfmt::skip]
    let data = [(
        "resolve with multiple dots",
        f.clone(),
        "./a/../main1.js",
        f.join("main1.js"),
    )];

    for (comment, path, request, expected) in data {
        let resolved_path = resolver
            .resolve_test_directory(&path, request)
            .map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {path:?} {request}");
    }
}

/// Test resolving a specifier with "dot", "dir", "slash", etc. spelled out.
#[test]
fn test_resolve_dot_spelled_out() {
    let f = fixture_root().join("dot");
    let foo_dir: std::path::PathBuf = f.join("foo");
    let resolver = Resolver::for_tests(ResolverOptions::default());
    let foo_index = foo_dir.join("index.js");

    #[rustfmt::skip]
    let data = [
        ("dot dir", foo_dir.clone(), ".", foo_index.clone()),
        ("dot dir slash", foo_dir.clone(), "./", foo_index),
    ];
    for (comment, path, request, expected) in data {
        let resolved_path = resolver
            .resolve_test_directory(&path, request)
            .map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {path:?} {request}");
    }

    let resolver = Resolver::for_tests(ResolverOptions {
        main_files: vec![],
        ..ResolverOptions::default()
    });
    #[rustfmt::skip]
    let data = [
        (
            "dot dir",
            foo_dir.clone(),
            ".",
            ResolverError::NotFound { specifier: ".".into() },
        ),
        (
            "dot dir slash",
            foo_dir,
            "./",
            ResolverError::NotFound { specifier: "./".into() },
        ),
    ];
    for (comment, path, request, expected) in data {
        let resolve_error = resolver.resolve_test_directory(&path, request);
        assert_eq!(resolve_error, Err(expected), "{comment} {path:?} {request}");
    }
}

/// Test resolving abnormal relative paths (e.g., with "../..").
#[test]
fn test_resolve_abnormal_relative() {
    let f = fixture_root().join("abnormal-relative-with-node_modules");

    // keep the empty fixture directory present across checkouts
    std::fs::create_dir_all(f.join("node_modules")).unwrap();

    let base = f.join("foo/bar/baz");

    let resolver = Resolver::for_tests(ResolverOptions::default());

    #[rustfmt::skip]
    let data = [
        ("2-level abnormal relative path 1", "jest-runner-../../.."),
        ("2-level abnormal relative path 2", "jest-runner-../../../"),
        ("2-level abnormal relative path 3", "jest-runner-/../.."),
        ("2-level abnormal relative path 4", "jest-runner-/../../"),
    ];

    for (comment, request) in data {
        let resolved_path = resolver
            .resolve_test_directory(&base, request)
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
        let resolved_path = resolver.resolve_test_directory(&base, request);
        assert_eq!(
            resolved_path,
            Err(ResolverError::NotFound {
                specifier: request.into()
            }),
            "{comment} {request}"
        );
    }

    let f = fixture_root().join("abnormal-relative-without-node_modules");
    assert!(
        !f.join("node_modules").exists(),
        "abnormal-relative-without-node_modules must not contain node_modules fixture directory"
    );

    let base = f.join("foo/bar/baz");

    let data = [
        ("2-level abnormal relative path 1", "jest-runner-../../.."),
        ("2-level abnormal relative path 2", "jest-runner-../../../"),
        ("2-level abnormal relative path 3", "jest-runner-/../.."),
        ("2-level abnormal relative path 4", "jest-runner-/../../"),
    ];

    for (comment, request) in data {
        let resolved_path = resolver.resolve_test_directory(&base, request);
        assert_eq!(
            resolved_path,
            Err(ResolverError::NotFound {
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
    let resolution =
        Resolver::for_tests(ResolverOptions::default()).resolve_test_directory(&dir, specifier);
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

    let options = ResolverOptions::default();
    let resolution = Resolver::for_tests(options).resolve_test_directory(&path, specifier);
    assert_eq!(
        resolution.map(|r| r.full_path()),
        Ok(module_path.join("dist/styled-components.browser.cjs.js"))
    );
}

/// Test resolving against one long pnpm package directory name.
#[test]
fn test_resolve_pnpm_symlinked_longfilename() {
    let dir = fixture_root();
    let path = dir.join("pnpm");
    let package_name = "fixture-test-longfilename-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let module_path = path
        .join("node_modules")
        .join(".pnpm")
        .join(
            "fixture-test-longfilename-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@file+longfilename",
        )
        .join("node_modules")
        .join(package_name)
        .join("index.js");

    // pnpm v10 shortens long store directory names unless virtual-store-dir-max-length is set
    assert!(
        module_path.as_os_str().len() > 260,
        "fixture path must keep long store directory names"
    );

    let resolution = Resolver::for_tests(ResolverOptions::default())
        .resolve_test_directory(&path, package_name)
        .map(|r| r.full_path());
    assert_eq!(resolution, Ok(module_path));
}

/// Test resolving one linked package from one pnpm workspace app package.
#[test]
fn test_resolve_pnpm_workspace_linked_package() {
    let root = fixture_root().join("pnpm-workspace");
    let app_path = root.join("packages/app");
    let specifier = "@monorepo/lib/package.json";
    let expected = root.join("packages/lib/package.json");

    let resolution = Resolver::for_tests(ResolverOptions::default())
        .resolve_test_directory(&app_path, specifier)
        .map(|r| r.full_path());
    assert_eq!(resolution, Ok(expected));
}

/// Test resolving one transitive dependency from one pnpm workspace symlinked package.
#[test]
fn test_resolve_pnpm_workspace_transitive_dependency() {
    let root = fixture_root().join("pnpm-workspace");
    let symlinked_package_path = root.join("packages/app/node_modules/@monorepo/lib");
    assert!(
        symlinked_package_path.is_dir(),
        "missing pnpm workspace symlink fixture directory"
    );

    let resolution = Resolver::for_tests(ResolverOptions::default())
        .resolve_test_directory(&symlinked_package_path, "react")
        .map(|r| r.full_path())
        .expect("expected react to resolve from pnpm workspace package");
    let normalized = resolution.to_string_lossy().replace('\\', "/");

    assert!(
        normalized.contains("/pnpm-workspace/node_modules/.pnpm/react@"),
        "unexpected react resolution path: {normalized}"
    );
    assert!(
        normalized.ends_with("/node_modules/react/index.js"),
        "unexpected react resolution path: {normalized}"
    );
}

/// Test resolving against the axios package with various conditions.
#[test]
fn test_resolve_axios() {
    let dir = fixture_root();
    let path = dir.join("pnpm");
    let module_path = path.join("node_modules/.pnpm/axios@1.8.4/node_modules/axios");
    let specifier = "axios";

    let options = ResolverOptions::default();
    let resolution = Resolver::for_tests(options).resolve_test_directory(&path, specifier);
    assert_eq!(
        resolution.map(|r| r.full_path()),
        Ok(module_path.join("index.js"))
    );

    let options = ResolverOptions {
        conditions: vec!["browser".into(), "require".into()],
        ..ResolverOptions::default()
    };
    let resolution = Resolver::for_tests(options).resolve_test_directory(&path, specifier);
    assert_eq!(
        resolution.map(|r| r.full_path()),
        Ok(module_path.join("dist/browser/axios.cjs"))
    );

    let options = ResolverOptions {
        conditions: vec!["node".into(), "require".into()],
        ..ResolverOptions::default()
    };
    let resolution = Resolver::for_tests(options).resolve_test_directory(&path, specifier);
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
    let resolver = Resolver::for_tests(ResolverOptions {
        canonicalize_symlinks: false,
        ..ResolverOptions::default()
    });

    let resolution = resolver.resolve_test_directory(&module_path, "path");
    assert_eq!(
        resolution,
        Err(ResolverError::Ignored {
            path: module_path.clone()
        })
    );

    let resolution = resolver.resolve_test_directory(&module_path, "./lib/terminal-highlight");
    assert_eq!(
        resolution,
        Err(ResolverError::Ignored {
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
        Resolver::for_tests(ResolverOptions {
            extension_alias: IndexMap::from([(
                ".js".into(),
                vec![".js".into(), ".ts".into(), ".tsx".into()],
            )]),
            ..ResolverOptions::default()
        }),
        Resolver::for_tests(ResolverOptions {
            extensions: vec![".ts".into()],
            ..ResolverOptions::default()
        }),
        Resolver::for_tests(ResolverOptions::default()),
    ];

    for resolver in resolvers {
        let resolution = resolver
            .resolve_test_directory(&path, "ipaddr.js")
            .map(|r| r.full_path());
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
        Resolver::for_tests(ResolverOptions {
            extension_alias: IndexMap::from([(
                ".js".into(),
                vec![".js".into(), ".ts".into(), ".tsx".into()],
            )]),
            conditions: vec!["import".into()],
            ..ResolverOptions::default()
        }),
        Resolver::for_tests(ResolverOptions {
            conditions: vec!["import".into()],
            ..ResolverOptions::default()
        }),
    ];

    for resolver in resolvers {
        let resolution = resolver
            .resolve_test_directory(&path, "decimal.js")
            .map(|r| r.full_path());
        assert_eq!(resolution, Ok(module_path.clone()));
    }
}

/// Test resolving against the decimal.js package from the mathjs package.
#[test]
fn test_resolve_decimal_js_from_mathjs() {
    let dir = fixture_root();
    let path = dir.join("pnpm/node_modules/.pnpm/mathjs@14.4.0/node_modules/mathjs/lib/esm");
    let module_path =
        dir.join("pnpm/node_modules/.pnpm/decimal.js@10.5.0/node_modules/decimal.js/decimal.mjs");

    let resolvers = [
        Resolver::for_tests(ResolverOptions {
            extension_alias: IndexMap::from([(
                ".js".into(),
                vec![".js".into(), ".ts".into(), ".tsx".into()],
            )]),
            conditions: vec!["import".into()],
            ..ResolverOptions::default()
        }),
        Resolver::for_tests(ResolverOptions {
            conditions: vec!["import".into()],
            ..ResolverOptions::default()
        }),
    ];

    for resolver in resolvers {
        let resolution = resolver
            .resolve_test_directory(&path, "decimal.js")
            .map(|r| r.full_path());
        assert_eq!(resolution, Ok(module_path.clone()));
    }
}

/// Test resolving against the minimatch package.
#[test]
fn test_resolve_minimatch() {
    let dir = fixture_root();
    let path = dir.join("pnpm");
    let resolver = Resolver::for_tests(ResolverOptions {
        conditions: vec!["import".into()],
        ..ResolverOptions::default()
    });
    let resolution = resolver.resolve_test_directory(&path, "minimatch").unwrap();
    assert_eq!(
        resolution.full_path(),
        dir.join(
            "pnpm/node_modules/.pnpm/minimatch@10.0.1/node_modules/minimatch/dist/esm/index.js",
        )
    );

    let resolver = Resolver::for_tests(ResolverOptions {
        conditions: vec!["require".into()],
        ..ResolverOptions::default()
    });
    let resolution = resolver.resolve_test_directory(&path, "minimatch").unwrap();
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
        Resolver::for_tests(ResolverOptions::default())
            .resolve_test_directory(&dir, "./apps/web/nm/@repo/typescript-config/index.js")
            .map(Resolution::into_path_buf),
        Ok(dir.join("nm/index.js"))
    );
    assert_eq!(
        Resolver::for_tests(ResolverOptions::default())
            .resolve_test_directory(&dir, "./apps/tooling/typescript-config/index.js")
            .map(Resolution::into_path_buf),
        Ok(dir.join("nm/index.js"))
    );
}

/// Test resolving against a package.json with a BOM.
#[test]
fn test_resolve_package_json_with_bom() {
    let dir = fixture_root().join("misc");
    assert_eq!(
        Resolver::for_tests(ResolverOptions::default())
            .resolve_test_directory(&dir, "./package-json-with-bom")
            .map(Resolution::into_path_buf),
        Ok(dir.join("package-json-with-bom/index.js"))
    );
}

/// Test resolving on Windows (should normalize the path).
#[cfg(windows)]
#[test]
fn test_resolve_normalized_on_windows() {
    use destack_source::PathExt;

    let f = fixture();
    let absolute = f.join("./foo/index.js").normalize();
    let absolute_str = absolute.to_string_lossy();
    let normalized_absolute = absolute_str.replace('\\', "/");
    let resolver = Resolver::for_tests(ResolverOptions::default());

    let resolution = resolver
        .resolve_test_directory(&f, &normalized_absolute)
        .map(|r| r.full_path());
    assert_eq!(
        resolution.map(|r| r.to_string_lossy().into_owned()),
        Ok(absolute_str.clone().into_owned())
    );

    let normalized_f = f.to_str().unwrap().replace('\\', "/");
    let resolution = resolver
        .resolve_test_directory(normalized_f, ".\\foo\\index.js")
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

    let resolver = Resolver::for_tests(ResolverOptions::default());

    let resolution = resolver
        .resolve_test_directory(&f, file_protocol_path.as_str())
        .ok();
    let resolved_path = resolution.as_ref().map(Resolution::full_path);
    assert_eq!(resolved_path, Some(f.join("main1.js")));

    let resolve_error = ResolverError::NotFound {
        specifier: "\\\\.\\main.js".into(),
    };

    assert_eq!(
        resolver.resolve_test_directory(f, "file://./main.js"),
        Err(resolve_error)
    );
}

/// Test resolving against scoped packages.
/// https://github.com/webpack/enhanced-resolve/blob/main/test/scoped-packages.test.js
#[test]
fn test_resolve_scoped_packages() {
    let f = fixture().join("scoped");
    let resolver = Resolver::for_tests(ResolverOptions::default());

    #[rustfmt::skip]
    let pass = [
        ("main field should work", f.clone(), "@scope/pack1", "@scope/pack1", f.join("./node_modules/@scope/pack1/main.js")),
        ("browser field should work", f.clone(), "@scope/pack2", "@scope/pack2", f.join("./node_modules/@scope/pack2/main.js")),
        ("folder request should work", f.clone(), "@scope/pack2/lib", "@scope/pack2", f.join("./node_modules/@scope/pack2/lib/index.js"))
    ];

    for (comment, path, request, _, expected) in pass {
        let resolution = resolver.resolve_test_directory(&path, request).ok();
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

    let resolver = Resolver::for_tests(ResolverOptions {
        extensions: vec![".js".into()],
        alias: vec![("foo".into(), vec![AliasValue::from("/fixtures")])],
        roots: vec![
            fixture_root().join("enhanced_resolve").join("test"),
            f.clone(),
        ],
        ..ResolverOptions::default()
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
        let resolved_path = resolver
            .resolve_test_directory(&f, request)
            .map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {request}");
    }

    #[rustfmt::skip]
    let fail = [
        ("should not work with relative path", "fixtures/b.js", ResolverError::NotFound { specifier: "fixtures/b.js".into() })
    ];

    for (comment, request, expected) in fail {
        let resolution = resolver.resolve_test_directory(&f, request);
        assert_eq!(resolution, Err(expected), "{comment} {request}");
    }
}

#[test]
fn test_prefer_absolute() {
    let f = super::fixture();
    let resolver = Resolver::for_tests(ResolverOptions {
        extensions: vec![".js".into()],
        alias: vec![("foo".into(), vec![AliasValue::from("/fixtures")])],
        roots: vec![
            fixture_root().join("enhanced_resolve").join("test"),
            f.clone(),
        ],
        prefer_absolute: true,
        ..ResolverOptions::default()
    });

    #[rustfmt::skip]
    let pass = [
        ("should resolve an absolute path (prefer absolute)", f.join("b.js").to_string_lossy().to_string(), f.join("b.js")),
    ];

    for (comment, request, expected) in pass {
        let resolved_path = resolver
            .resolve_test_directory(&f, &request)
            .map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {request}");
    }
}

#[test]
fn test_roots_fall_through() {
    let f = super::fixture();
    let absolute_path = f.join("roots_fall_through/index.js");
    let specifier = absolute_path.to_string_lossy();
    let mut options = ResolverOptions::default();
    options.roots.push(f.clone());
    let resolution = Resolver::for_tests(options).resolve_test_directory(&f, &specifier);
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
        let resolver = Resolver::for_tests(ResolverOptions {
            roots: roots.clone(),
            ..ResolverOptions::default()
        });
        let resolved_path = resolver
            .resolve_test_directory(directory, "/")
            .map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(expected), "{comment} {roots:?}");
    }

    #[rustfmt::skip]
    let fail = [
        ("should not resolve if not found", vec![f.clone()], &f),
        ("should not resolve if importer is not root", vec![dir_with_index], &f)
    ];

    for (comment, roots, directory) in fail {
        let resolver = Resolver::for_tests(ResolverOptions {
            roots: roots.clone(),
            ..ResolverOptions::default()
        });
        let resolution = resolver.resolve_test_directory(directory, "/");
        assert_eq!(
            resolution,
            Err(ResolverError::NotFound {
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

    let resolver = Resolver::for_test_file_system(
        fs.clone(),
        ResolverOptions {
            alias: vec![
                ("alias1".into(), vec![AliasValue::from("/a/abc")]),
                ("alias2".into(), vec![AliasValue::from("/a")]),
            ],
            is_fully_specified: true,
            ..ResolverOptions::default()
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
        let resolution = resolver.resolve_test_directory("/a", request);
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

        let resolution = resolver
            .resolve_test_directory("/a", request)
            .map(|r| r.full_path());
        assert_eq!(
            resolution,
            Ok(PathBuf::from(expected)),
            "{comment} {request}"
        );
    }

    let resolver = Resolver::for_test_file_system(
        fs.clone(),
        ResolverOptions {
            alias: vec![
                ("alias1".into(), vec![AliasValue::from("/a/abc")]),
                ("alias2".into(), vec![AliasValue::from("/a")]),
            ],
            is_fully_specified: true,
            resolve_to_context: true,
            ..ResolverOptions::default()
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

        let resolution = resolver
            .resolve_test_directory("/a", request)
            .map(|r| r.full_path());
        assert_eq!(
            resolution,
            Ok(PathBuf::from(expected)),
            "{comment} {request}"
        );
    }
}

/// Keep Node strict mode for package-name self imports without exports.
#[test]
#[cfg(not(target_os = "windows"))]
fn test_resolve_self_package_name_without_exports_is_not_resolved_in_node_strict() {
    use std::sync::Arc;

    let fs = Arc::new(MemoryFileSystem::from_files(&[
        (
            "/repo/package.json",
            r#"{"name":"pkg","main":"./src/main.js"}"#,
        ),
        ("/repo/src/main.js", ""),
        ("/repo/src/feature.js", ""),
    ]));
    let resolver = Resolver::for_test_file_system(fs, ResolverOptions::default());

    let resolution = resolver.resolve_test_directory("/repo/src", "pkg");
    assert_eq!(
        resolution,
        Err(ResolverError::NotFound {
            specifier: "pkg".into()
        })
    );
}

/// Resolve self package root and subpath imports when exports explicitly allow them.
#[test]
#[cfg(not(target_os = "windows"))]
fn test_resolve_self_package_name_and_subpath_with_exports() {
    use std::sync::Arc;

    let fs = Arc::new(MemoryFileSystem::from_files(&[
        (
            "/repo/package.json",
            r#"{"name":"pkg","exports":{".":"./src/main.js","./feature":"./src/feature.js"}}"#,
        ),
        ("/repo/src/main.js", ""),
        ("/repo/src/feature.js", ""),
        ("/repo/src/importer.js", ""),
    ]));
    let resolver = Resolver::for_test_file_system(fs, ResolverOptions::default());

    let root_resolution = resolver
        .resolve_test_directory("/repo/src", "pkg")
        .map(|r| r.full_path());
    assert_eq!(
        root_resolution,
        Ok(std::path::PathBuf::from("/repo/src/main.js")),
    );

    let subpath_resolution = resolver
        .resolve_test_directory("/repo/src", "pkg/feature")
        .map(|r| r.full_path());
    assert_eq!(
        subpath_resolution,
        Ok(std::path::PathBuf::from("/repo/src/feature.js")),
    );
}

/// Keep Node strict mode for package-name self subpath imports without exports.
#[test]
#[cfg(not(target_os = "windows"))]
fn test_resolve_self_package_subpath_without_exports_is_not_resolved_in_node_strict() {
    use std::sync::Arc;

    let fs = Arc::new(MemoryFileSystem::from_files(&[
        ("/repo/package.json", r#"{"name":"pkg"}"#),
        ("/repo/lib/util.js", ""),
        ("/repo/src/feature.js", ""),
    ]));
    let resolver = Resolver::for_test_file_system(fs, ResolverOptions::default());

    let resolution = resolver.resolve_test_directory("/repo/src", "pkg/lib/util.js");
    assert_eq!(
        resolution,
        Err(ResolverError::NotFound {
            specifier: "pkg/lib/util.js".into()
        })
    );
}

/// Keep percent-encoded traversal segments from escaping the importer directory.
#[test]
#[cfg(not(target_os = "windows"))]
fn test_resolve_does_not_decode_percent_encoded_relative_traversal() {
    use std::sync::Arc;

    let fs = Arc::new(MemoryFileSystem::from_files(&[
        ("/repo/secret.js", ""),
        ("/repo/src/importer.js", ""),
        ("/repo/src/%2e%2e/secret.js", ""),
    ]));
    let resolver = Resolver::for_test_file_system(fs, ResolverOptions::default());

    let resolution = resolver.resolve_test_directory("/repo/src", "./%2e%2e/secret.js");
    assert_eq!(
        resolution.map(Resolution::into_path_buf),
        Ok(std::path::PathBuf::from("/repo/src/%2e%2e/secret.js")),
    );
}

/// Keep percent-encoded slashes from spoofing scoped package names.
#[test]
#[cfg(not(target_os = "windows"))]
fn test_resolve_does_not_decode_percent_encoded_package_slash() {
    use std::sync::Arc;

    let fs = Arc::new(MemoryFileSystem::from_files(&[
        (
            "/repo/node_modules/@scope/pkg/package.json",
            r#"{"name":"@scope/pkg","main":"./index.js"}"#,
        ),
        ("/repo/node_modules/@scope/pkg/index.js", ""),
        ("/repo/src/importer.js", ""),
    ]));
    let resolver = Resolver::for_test_file_system(fs, ResolverOptions::default());

    let resolution = resolver.resolve_test_directory("/repo/src", "@scope%2fpkg");
    assert_eq!(
        resolution,
        Err(ResolverError::NotFound {
            specifier: "@scope%2fpkg".into()
        }),
    );
}

/// Reject self package fallback when exports are present and root is not exported.
#[test]
#[cfg(not(target_os = "windows"))]
fn test_resolve_self_package_name_with_exports_does_not_fallback_to_main() {
    use std::sync::Arc;

    let fs = Arc::new(MemoryFileSystem::from_files(&[
        (
            "/repo/package.json",
            r#"{"name":"pkg","main":"./src/main.js","exports":{"./feature":"./src/feature.js"}}"#,
        ),
        ("/repo/src/main.js", ""),
        ("/repo/src/feature.js", ""),
    ]));
    let resolver = Resolver::for_test_file_system(fs, ResolverOptions::default());

    let resolution = resolver.resolve_test_directory("/repo/src", "pkg");
    assert!(matches!(
        resolution,
        Err(ResolverError::PackagePathNotExported { .. })
    ));
}

/// Prefer `@types` package declarations for type-conditioned bare package resolution.
#[test]
#[cfg(not(target_os = "windows"))]
fn test_resolve_types_condition_prefers_types_package_fallback() {
    use std::sync::Arc;

    let fs = Arc::new(MemoryFileSystem::from_files(&[
        ("/repo/src/index.ts", ""),
        (
            "/repo/node_modules/react/package.json",
            r#"{"name":"react","exports":{".":{"default":"./index.js"}}}"#,
        ),
        ("/repo/node_modules/react/index.js", ""),
        (
            "/repo/node_modules/@types/react/package.json",
            r#"{"name":"@types/react","types":"./index.d.ts"}"#,
        ),
        (
            "/repo/node_modules/@types/react/index.d.ts",
            "export declare function useCallback(): void;",
        ),
    ]));

    let resolver = Resolver::for_test_file_system(
        fs,
        ResolverOptions {
            conditions: vec!["types".into(), "import".into()],
            ..ResolverOptions::default()
        },
    );

    let resolution = resolver
        .resolve_test_directory("/repo/src", "react")
        .map(|r| r.full_path());
    assert_eq!(
        resolution,
        Ok(std::path::PathBuf::from(
            "/repo/node_modules/@types/react/index.d.ts",
        )),
    );
}

/// Prefer scoped `@types` fallback package declarations for type-conditioned bare package resolution.
#[test]
#[cfg(not(target_os = "windows"))]
fn test_resolve_types_condition_prefers_scoped_types_package_fallback() {
    use std::sync::Arc;

    let fs = Arc::new(MemoryFileSystem::from_files(&[
        ("/repo/src/index.ts", ""),
        (
            "/repo/node_modules/@babel/core/package.json",
            r#"{"name":"@babel/core","exports":{".":{"default":"./lib/index.js"}}}"#,
        ),
        ("/repo/node_modules/@babel/core/lib/index.js", ""),
        (
            "/repo/node_modules/@types/babel__core/package.json",
            r#"{"name":"@types/babel__core","types":"./index.d.ts"}"#,
        ),
        (
            "/repo/node_modules/@types/babel__core/index.d.ts",
            "export interface PluginObj {}",
        ),
    ]));

    let resolver = Resolver::for_test_file_system(
        fs,
        ResolverOptions {
            conditions: vec!["types".into(), "import".into()],
            ..ResolverOptions::default()
        },
    );

    let resolution = resolver
        .resolve_test_directory("/repo/src", "@babel/core")
        .map(|r| r.full_path());
    assert_eq!(
        resolution,
        Ok(std::path::PathBuf::from(
            "/repo/node_modules/@types/babel__core/index.d.ts",
        )),
    );
}

/// Keep runtime package resolution when no `@types` fallback package exists.
#[test]
#[cfg(not(target_os = "windows"))]
fn test_resolve_types_condition_falls_back_to_runtime_package_without_types_package() {
    use std::sync::Arc;

    let fs = Arc::new(MemoryFileSystem::from_files(&[
        ("/repo/src/index.ts", ""),
        (
            "/repo/node_modules/react/package.json",
            r#"{"name":"react","main":"./index.js"}"#,
        ),
        ("/repo/node_modules/react/index.js", ""),
    ]));

    let resolver = Resolver::for_test_file_system(
        fs,
        ResolverOptions {
            conditions: vec!["types".into(), "import".into()],
            ..ResolverOptions::default()
        },
    );

    let resolution = resolver
        .resolve_test_directory("/repo/src", "react")
        .map(|r| r.full_path());
    assert_eq!(
        resolution,
        Ok(std::path::PathBuf::from(
            "/repo/node_modules/react/index.js",
        )),
    );
}

/// Prefer exports condition matches based on object key order, not option condition order.
#[test]
#[cfg(not(target_os = "windows"))]
fn test_resolve_exports_conditions_follow_object_key_order() {
    use std::sync::Arc;

    let fs = Arc::new(MemoryFileSystem::from_files(&[
        ("/repo/src/index.ts", ""),
        (
            "/repo/node_modules/pkg/package.json",
            r#"{"name":"pkg","exports":{".":{"require":"./require.js","types":"./index.d.ts","import":"./import.js","default":"./default.js"}}}"#,
        ),
        ("/repo/node_modules/pkg/require.js", ""),
        ("/repo/node_modules/pkg/import.js", ""),
        ("/repo/node_modules/pkg/index.d.ts", ""),
        ("/repo/node_modules/pkg/default.js", ""),
    ]));

    let resolver = Resolver::for_test_file_system(
        fs.clone(),
        ResolverOptions {
            conditions: vec!["types".into(), "import".into(), "require".into()],
            ..ResolverOptions::default()
        },
    );
    let resolution = resolver
        .resolve_test_directory("/repo/src", "pkg")
        .map(|r| r.full_path());
    assert_eq!(
        resolution,
        Ok(std::path::PathBuf::from(
            "/repo/node_modules/pkg/require.js"
        )),
    );

    let resolver = Resolver::for_test_file_system(
        fs,
        ResolverOptions {
            conditions: vec!["types".into(), "import".into()],
            ..ResolverOptions::default()
        },
    );
    let resolution = resolver
        .resolve_test_directory("/repo/src", "pkg")
        .map(|r| r.full_path());
    assert_eq!(
        resolution,
        Ok(std::path::PathBuf::from(
            "/repo/node_modules/pkg/index.d.ts"
        )),
    );
}

/// Resolve extension alias targets with fully specified requests while preserving query and fragment.
#[test]
#[cfg(not(target_os = "windows"))]
fn test_resolve_fully_specified_with_extension_alias_query_fragment() {
    use std::sync::Arc;

    let fs = Arc::new(MemoryFileSystem::from_files(&[
        ("/repo/src/importer.ts", ""),
        ("/repo/src/entry.ts", "export {};"),
    ]));
    let resolver = Resolver::for_test_file_system(
        fs,
        ResolverOptions {
            is_fully_specified: true,
            extension_alias: IndexMap::from([(".js".into(), vec![".ts".into()])]),
            ..ResolverOptions::default()
        },
    );

    let resolution = resolver.resolve_test_directory("/repo/src", "./entry.js?raw#module");
    assert_eq!(
        resolution,
        Ok(Resolution::new(
            std::path::PathBuf::from("/repo/src/entry.ts"),
            Some("?raw".into()),
            Some("#module".into()),
        )),
    );
}

/// Continue exports array fallback after invalid targets and resolve the first valid candidate.
#[test]
#[cfg(not(target_os = "windows"))]
fn test_resolve_exports_array_fallback_continues_on_invalid_target() {
    use std::sync::Arc;

    let fs = Arc::new(MemoryFileSystem::from_files(&[
        ("/repo/src/importer.js", ""),
        (
            "/repo/node_modules/pkg/package.json",
            r#"{"name":"pkg","exports":{".":["../escape.js",null,"./good.js"]}}"#,
        ),
        ("/repo/node_modules/pkg/good.js", ""),
    ]));
    let resolver = Resolver::for_test_file_system(fs, ResolverOptions::default());

    let resolution = resolver
        .resolve_test_directory("/repo/src", "pkg")
        .map(|r| r.full_path());
    assert_eq!(
        resolution,
        Ok(std::path::PathBuf::from("/repo/node_modules/pkg/good.js")),
    );
}

/// Prefer query and fragment from exports targets over query and fragment from the request.
#[test]
#[cfg(not(target_os = "windows"))]
fn test_resolve_exports_target_query_fragment_override_request_query_fragment() {
    use std::sync::Arc;

    let fs = Arc::new(MemoryFileSystem::from_files(&[
        ("/repo/src/importer.js", ""),
        (
            "/repo/node_modules/pkg/package.json",
            r#"{"name":"pkg","exports":{".":"./entry.js?from_exports#from_exports"}}"#,
        ),
        ("/repo/node_modules/pkg/entry.js", ""),
    ]));
    let resolver = Resolver::for_test_file_system(fs, ResolverOptions::default());

    let resolution = resolver.resolve_test_directory("/repo/src", "pkg?from_request#from_request");
    assert_eq!(
        resolution,
        Ok(Resolution::new(
            std::path::PathBuf::from("/repo/node_modules/pkg/entry.js"),
            Some("?from_exports".into()),
            Some("#from_exports".into()),
        )),
    );
}

/// Resolve nested exports condition objects using object key order at each nesting level.
#[test]
#[cfg(not(target_os = "windows"))]
fn test_resolve_exports_nested_conditions_follow_key_order() {
    use std::sync::Arc;

    let fs = Arc::new(MemoryFileSystem::from_files(&[
        ("/repo/src/index.ts", ""),
        (
            "/repo/node_modules/pkg/package.json",
            r#"{"name":"pkg","exports":{".":{"types":{"import":"./types-import.d.ts","default":"./types-default.d.ts"},"import":"./runtime-import.js","default":"./default.js"}}}"#,
        ),
        ("/repo/node_modules/pkg/types-import.d.ts", ""),
        ("/repo/node_modules/pkg/types-default.d.ts", ""),
        ("/repo/node_modules/pkg/runtime-import.js", ""),
        ("/repo/node_modules/pkg/default.js", ""),
    ]));
    let resolver = Resolver::for_test_file_system(
        fs,
        ResolverOptions {
            conditions: vec!["import".into(), "types".into()],
            ..ResolverOptions::default()
        },
    );

    let resolution = resolver
        .resolve_test_directory("/repo/src", "pkg")
        .map(|r| r.full_path());
    assert_eq!(
        resolution,
        Ok(std::path::PathBuf::from(
            "/repo/node_modules/pkg/types-import.d.ts",
        )),
    );
}

/// Resolve self references against the nearest package scope only.
#[test]
#[cfg(not(target_os = "windows"))]
fn test_resolve_self_reference_uses_nearest_package_scope() {
    use std::sync::Arc;

    let fs = Arc::new(MemoryFileSystem::from_files(&[
        (
            "/repo/package.json",
            r#"{"name":"rootpkg","exports":{".":"./root.js"}}"#,
        ),
        ("/repo/root.js", ""),
        (
            "/repo/packages/inner/package.json",
            r#"{"name":"inner","exports":{".":"./src/inner.js"}}"#,
        ),
        ("/repo/packages/inner/src/importer.js", ""),
        ("/repo/packages/inner/src/inner.js", ""),
    ]));
    let resolver = Resolver::for_test_file_system(fs, ResolverOptions::default());

    let inner_resolution = resolver
        .resolve_test_directory("/repo/packages/inner/src", "inner")
        .map(|r| r.full_path());
    assert_eq!(
        inner_resolution,
        Ok(std::path::PathBuf::from(
            "/repo/packages/inner/src/inner.js"
        )),
    );

    let root_resolution = resolver.resolve_test_directory("/repo/packages/inner/src", "rootpkg");
    assert_eq!(
        root_resolution,
        Err(ResolverError::NotFound {
            specifier: "rootpkg".into()
        }),
    );
}

/// Allow extension alias remapping even when fully specified and extension enforcement are enabled.
#[test]
#[cfg(not(target_os = "windows"))]
fn test_resolve_fully_specified_with_enforce_extension_and_extension_alias() {
    use std::sync::Arc;

    let fs = Arc::new(MemoryFileSystem::from_files(&[
        ("/repo/src/importer.ts", ""),
        ("/repo/src/entry.ts", "export {};"),
    ]));
    let resolver = Resolver::for_test_file_system(
        fs,
        ResolverOptions {
            is_fully_specified: true,
            enforce_extension: crate::EnforceExtension::Enabled,
            extension_alias: IndexMap::from([(".js".into(), vec![".ts".into()])]),
            ..ResolverOptions::default()
        },
    );

    let resolution = resolver.resolve_test_directory("/repo/src", "./entry.js");
    assert_eq!(
        resolution.map(Resolution::into_path_buf),
        Ok(std::path::PathBuf::from("/repo/src/entry.ts")),
    );
}

#[cfg(not(target_os = "windows"))] // MemoryFS's path separator is always `/` so the test will not pass in windows.
mod windows {
    use crate::{Resolver, ResolverOptions};
    use destack_source::MemoryFileSystem;

    #[test]
    fn test_resolve_no_package() {
        use std::path::Path;
        use std::sync::Arc;

        let f = Path::new("/");
        let fs = MemoryFileSystem::from_files(&[]);
        let resolver = Resolver::for_test_file_system(Arc::new(fs), ResolverOptions::default());
        let resolved_path = resolver.resolve_test_directory(f, "package");
        assert!(resolved_path.is_err());
    }
}
