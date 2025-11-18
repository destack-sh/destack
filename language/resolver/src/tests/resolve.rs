use dyst_source::PhysicalFileSystem;

use super::{fixture, fixture_root};
use crate::resolve::{Resolution, ResolveError, ResolveOptions};
use crate::tests::Resolver;

type TestResolver = Resolver<PhysicalFileSystem>;

/// Run the tests from the enhanced-resolve test suite (webpack).
/// https://github.com/webpack/enhanced-resolve/tree/main/test/fixtures
#[test]
fn test_resolve_enhanced_resolve() {
    let f = fixture();
    let resolver = TestResolver::default();
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
        // enhanced-resolve has `#` prepended with a `\0`, they are removed from the
        // following 3 expected test results.
        // See https://github.com/webpack/enhanced-resolve#escaping
        ("handle fragment edge case (no fragment)", f.clone(), "./no#fragment/#/#", f.join("no#fragment/#/#.js")),
        ("handle fragment edge case (fragment)", f.clone(), "./no#fragment/#/", f.join("no.js#fragment/#/")),
        ("handle fragment escaping", f.clone(), "./no\0#fragment/\0#/\0##fragment", f.join("no#fragment/#/#.js#fragment")),
    ];

    for (comment, path, request, expected) in pass {
        let resolution = resolver.resolve(&path, request).ok();
        let resolved_path = resolution.as_ref().map(Resolution::full_path);
        let resolved_package_json = resolution
            .as_ref()
            .and_then(|r| r.package_json())
            .map(|p| p.path.clone());
        if expected.to_str().unwrap().contains("node_modules") {
            assert!(
                resolved_package_json.is_some(),
                "{comment} {path:?} {request}"
            );
        }
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
    let resolver = TestResolver::new(ResolveOptions {
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

    let resolver = TestResolver::new(ResolveOptions {
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
fn test_resolve_to_context() {
    let f = fixture();
    let resolver = TestResolver::new(ResolveOptions {
        resolve_to_context: true,
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
    let resolver = TestResolver::default();
    let resolution = resolver.resolve(f, "#a");
    assert_eq!(resolution, Err(ResolveError::NotFound("#a".into())));
}

#[test]
fn test_resolve_edge_cases() {
    let f = fixture();
    let resolver = TestResolver::default();

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
    let resolver = TestResolver::default();
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

    let resolver = TestResolver::new(ResolveOptions {
        main_files: vec![],
        ..ResolveOptions::default()
    });
    #[rustfmt::skip]
    let data = [
        (
            "dot dir",
            foo_dir.clone(),
            ".",
            ResolveError::NotFound(".".into()),
        ),
        (
            "dot dir slash",
            foo_dir,
            "./",
            ResolveError::NotFound("./".into()),
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

    let resolver = TestResolver::default();

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
            Err(ResolveError::NotFound(request.into())),
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
            Err(ResolveError::NotFound(request.into())),
            "{comment} {request}"
        );
    }
}

/// Test resolving a directory / specifier with Chinese characters.
#[test]
fn test_resolve_chinese() {
    let dir = fixture_root();
    let specifier = "./misc/中文/中文.js";
    let resolution = TestResolver::new(ResolveOptions::default()).resolve(&dir, specifier);
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

    let options = ResolveOptions {
        alias_fields: vec![vec!["browser".into()]],
        ..ResolveOptions::default()
    };
    let resolution = TestResolver::new(options).resolve(&path, specifier);
    assert_eq!(
        resolution.map(|r| r.full_path()),
        Ok(module_path.join("dist/styled-components.browser.cjs.js"))
    );

    let options = ResolveOptions {
        alias_fields: vec![vec!["browser".into()]],
        main_fields: vec!["module".into()],
        ..ResolveOptions::default()
    };
    let resolution = TestResolver::new(options).resolve(&path, specifier);
    assert_eq!(
        resolution.map(|r| r.full_path()),
        Ok(module_path.join("dist/styled-components.browser.esm.js"))
    );
}

/// Test resolving against the axios package.
#[test]
fn test_resolve_axios() {
    let dir = fixture_root();
    let path = dir.join("pnpm");
    let module_path = path.join("node_modules/.pnpm/axios@1.8.4/node_modules/axios");
    let specifier = "axios";

    let options = ResolveOptions::default();
    let resolution = TestResolver::new(options).resolve(&path, specifier);
    assert_eq!(
        resolution.map(|r| r.full_path()),
        Ok(module_path.join("index.js"))
    );

    let options = ResolveOptions {
        condition_names: vec!["browser".into(), "require".into()],
        ..ResolveOptions::default()
    };
    let resolution = TestResolver::new(options).resolve(&path, specifier);
    assert_eq!(
        resolution.map(|r| r.full_path()),
        Ok(module_path.join("dist/browser/axios.cjs"))
    );

    let options = ResolveOptions {
        condition_names: vec!["node".into(), "require".into()],
        ..ResolveOptions::default()
    };
    let resolution = TestResolver::new(options).resolve(&path, specifier);
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
    let resolver = TestResolver::new(ResolveOptions {
        alias_fields: vec![vec!["browser".into()]],
        symlinks: false,
        ..ResolveOptions::default()
    });

    let resolution = resolver.resolve(&module_path, "path");
    assert_eq!(resolution, Err(ResolveError::Ignored(module_path.clone())));

    let resolution = resolver.resolve(&module_path, "./lib/terminal-highlight");
    assert_eq!(
        resolution,
        Err(ResolveError::Ignored(
            module_path.join("lib/terminal-highlight")
        ))
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
        TestResolver::new(ResolveOptions {
            extension_alias: vec![(
                ".js".into(),
                vec![".js".into(), ".ts".into(), ".tsx".into()],
            )],
            ..ResolveOptions::default()
        }),
        TestResolver::new(ResolveOptions {
            extensions: vec![".ts".into()],
            ..ResolveOptions::default()
        }),
        TestResolver::default(),
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
        TestResolver::new(ResolveOptions {
            extension_alias: vec![(
                ".js".into(),
                vec![".js".into(), ".ts".into(), ".tsx".into()],
            )],
            condition_names: vec!["import".into()],
            ..ResolveOptions::default()
        }),
        TestResolver::new(ResolveOptions {
            condition_names: vec!["import".into()],
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
        TestResolver::new(ResolveOptions {
            extension_alias: vec![(
                ".js".into(),
                vec![".js".into(), ".ts".into(), ".tsx".into()],
            )],
            condition_names: vec!["import".into()],
            ..ResolveOptions::default()
        }),
        TestResolver::new(ResolveOptions {
            condition_names: vec!["import".into()],
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
    let esm_resolver = TestResolver::new(ResolveOptions {
        condition_names: vec!["import".into()],
        ..ResolveOptions::default()
    });
    let resolution = esm_resolver.resolve(&path, "minimatch").unwrap();
    assert_eq!(
        resolution.full_path(),
        dir.join(
            "pnpm/node_modules/.pnpm/minimatch@10.0.1/node_modules/minimatch/dist/esm/index.js",
        )
    );

    let cjs_resolver = esm_resolver.clone_with_options(ResolveOptions {
        condition_names: vec!["require".into()],
        ..ResolveOptions::default()
    });
    let resolution = cjs_resolver.resolve(&path, "minimatch").unwrap();
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
        TestResolver::new(ResolveOptions::default())
            .resolve(&dir, "./apps/web/nm/@repo/typescript-config/index.js")
            .map(Resolution::into_path_buf),
        Ok(dir.join("nm/index.js"))
    );
    assert_eq!(
        TestResolver::new(ResolveOptions::default())
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
        TestResolver::new(ResolveOptions::default())
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
    let resolver = TestResolver::default();

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

    let resolver = TestResolver::default();

    let resolution = resolver.resolve(&f, file_protocol_path.as_str()).ok();
    let resolved_path = resolution.as_ref().map(Resolution::full_path);
    assert_eq!(resolved_path, Some(f.join("main1.js")));

    let resolve_error = ResolveError::NotFound("\\\\.\\main.js".into());

    assert_eq!(resolver.resolve(f, "file://./main.js"), Err(resolve_error));
}
