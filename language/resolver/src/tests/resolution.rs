use std::sync::Arc;

use crate::{Resolution, ResolverError, ResolverOptions, Restriction};

use super::support::{project, resolver};

/// Relative imports resolve exact files and extensionless source files.
#[test]
fn test_resolves_relative_files() {
    let resolver = resolver(
        &[("/src/main.ds", ""), ("/src/user.ds", "")],
        ResolverOptions::default(),
    );

    let exact = resolver
        .resolve_test_file(project("/src/main.ds"), "./user.ds")
        .map(Resolution::into_path_buf);
    assert_eq!(exact, Ok(project("/src/user.ds")));

    let extensionless = resolver
        .resolve_test_file(project("/src/main.ds"), "./user")
        .map(Resolution::into_path_buf);
    assert_eq!(extensionless, Ok(project("/src/user.ds")));
}

/// Extensionless imports use the Destack source probe order.
#[test]
fn test_resolves_source_extensions() {
    let resolver = resolver(
        &[
            ("/src/main.ds", ""),
            ("/src/model.d.ds", ""),
            ("/src/page.tsx", ""),
            ("/src/tool.ts", ""),
            ("/src/config.json", ""),
            ("/src/script.js", ""),
        ],
        ResolverOptions::default(),
    );

    let declaration = resolver
        .resolve_test_file(project("/src/main.ds"), "./model")
        .map(Resolution::into_path_buf);
    assert_eq!(declaration, Ok(project("/src/model.d.ds")));

    let page = resolver
        .resolve_test_file(project("/src/main.ds"), "./page")
        .map(Resolution::into_path_buf);
    assert_eq!(page, Ok(project("/src/page.tsx")));

    let tool = resolver
        .resolve_test_file(project("/src/main.ds"), "./tool")
        .map(Resolution::into_path_buf);
    assert_eq!(tool, Ok(project("/src/tool.ts")));

    let config = resolver
        .resolve_test_file(project("/src/main.ds"), "./config")
        .map(Resolution::into_path_buf);
    assert_eq!(config, Ok(project("/src/config.json")));

    let script = resolver.resolve_test_file(project("/src/main.ds"), "./script");
    assert_eq!(
        script,
        Err(ResolverError::NotFound {
            specifier: "./script".into()
        })
    );
}

/// Exact files win before extension probing.
#[test]
fn test_resolves_exact_files_before_extensions() {
    let resolver = resolver(
        &[
            ("/src/main.ds", ""),
            ("/src/user", ""),
            ("/src/user.ds", ""),
        ],
        ResolverOptions::default(),
    );

    let result = resolver
        .resolve_test_file(project("/src/main.ds"), "./user")
        .map(Resolution::into_path_buf);
    assert_eq!(result, Ok(project("/src/user")));
}

/// Directory imports do not resolve to implicit files.
#[test]
fn test_rejects_directory_imports() {
    let resolver = resolver(
        &[("/src/main.ds", ""), ("/src/view/index.ds", "")],
        ResolverOptions::default(),
    );

    let result = resolver.resolve_test_file(project("/src/main.ds"), "./view");
    assert_eq!(
        result,
        Err(ResolverError::NotFound {
            specifier: "./view".into()
        })
    );
}

/// A trailing slash always stays a directory request.
#[test]
fn test_rejects_trailing_slash_imports() {
    let resolver = resolver(
        &[
            ("/src/main.ds", ""),
            ("/src/view.ds", ""),
            ("/src/view/index.ds", ""),
        ],
        ResolverOptions::default(),
    );

    let result = resolver.resolve_test_file(project("/src/main.ds"), "./view/");
    assert_eq!(
        result,
        Err(ResolverError::NotFound {
            specifier: "./view/".into()
        })
    );
}

/// Root imports resolve against configured project roots.
#[test]
fn test_resolves_roots() {
    let resolver = resolver(
        &[("/src/main.ds", ""), ("/src/lib/user.ds", "")],
        ResolverOptions {
            roots: vec![project("/src")],
            ..ResolverOptions::default()
        },
    );

    let result = resolver
        .resolve_test_file(project("/src/main.ds"), "/lib/user")
        .map(Resolution::into_path_buf);
    assert_eq!(result, Ok(project("/src/lib/user.ds")));
}

/// Root imports try configured roots in order.
#[test]
fn test_resolves_root_order() {
    let resolver = resolver(
        &[
            ("/src/main.ds", ""),
            ("/generated/view.ds", ""),
            ("/source/view.ds", ""),
        ],
        ResolverOptions {
            roots: vec![project("/generated"), project("/source")],
            ..ResolverOptions::default()
        },
    );

    let result = resolver
        .resolve_test_file(project("/src/main.ds"), "/view")
        .map(Resolution::into_path_buf);
    assert_eq!(result, Ok(project("/generated/view.ds")));
}

/// A root import must still name a file-shaped path.
#[test]
fn test_rejects_bare_root_imports() {
    let resolver = resolver(
        &[("/src/main.ds", "")],
        ResolverOptions {
            roots: vec![project("/src")],
            ..ResolverOptions::default()
        },
    );

    let result = resolver.resolve_test_file(project("/src/main.ds"), "/");
    assert_eq!(
        result,
        Err(ResolverError::NotFound {
            specifier: "/".into()
        })
    );
}

/// Directory bases resolve relative imports from that directory.
#[test]
fn test_resolves_from_directory_bases() {
    let resolver = resolver(
        &[("/src/main.ds", ""), ("/src/user.ds", "")],
        ResolverOptions::default(),
    );

    let result = resolver
        .resolve_test_directory(project("/src"), "./user")
        .map(Resolution::into_path_buf);
    assert_eq!(result, Ok(project("/src/user.ds")));
}

/// File bases must be files and directory bases must be directories.
#[test]
fn test_validates_base_paths() {
    let resolver = resolver(
        &[("/src/main.ds", ""), ("/src/user.ds", "")],
        ResolverOptions::default(),
    );

    let file_base = resolver.resolve_test_file(project("/src"), "./user");
    assert_eq!(
        file_base,
        Err(ResolverError::ExpectedFilePath {
            path: project("/src")
        })
    );

    let directory_base = resolver.resolve_test_directory(project("/src/main.ds"), "./user");
    assert_eq!(
        directory_base,
        Err(ResolverError::ExpectedDirectoryPath {
            path: project("/src/main.ds")
        })
    );
}

/// Query and fragment suffixes stay attached to the resolved path.
#[test]
fn test_preserves_query_and_fragment() {
    let resolver = resolver(
        &[("/src/main.ds", ""), ("/src/view.ds", "")],
        ResolverOptions::default(),
    );

    let result = resolver
        .resolve_test_file(project("/src/main.ds"), "./view?raw#panel")
        .map(|resolution| resolution.full_path());
    assert_eq!(result, Ok(project("/src/view.ds?raw#panel")));
}

/// Fragment-like text can name a real source file.
#[test]
fn test_resolves_fragment_path_candidates() {
    let resolver = resolver(
        &[("/src/main.ds", ""), ("/src/view#panel.ds", "")],
        ResolverOptions::default(),
    );

    let result = resolver
        .resolve_test_file(project("/src/main.ds"), "./view#panel")
        .map(|resolution| resolution.full_path());
    assert_eq!(result, Ok(project("/src/view#panel.ds")));
}

/// Fragments remain metadata when no fragment-named file exists.
#[test]
fn test_preserves_fragment_suffixes() {
    let resolver = resolver(
        &[("/src/main.ds", ""), ("/src/view.ds", "")],
        ResolverOptions::default(),
    );

    let result = resolver
        .resolve_test_file(project("/src/main.ds"), "./view#panel")
        .map(|resolution| resolution.full_path());
    assert_eq!(result, Ok(project("/src/view.ds#panel")));
}

/// Package-shaped directories are still just directories.
#[test]
fn test_rejects_package_directory_imports() {
    let resolver = resolver(
        &[
            ("/src/main.ds", ""),
            ("/src/pkg/destack.json", "{}"),
            ("/src/pkg/main.ds", ""),
            ("/src/pkg/index.ds", ""),
        ],
        ResolverOptions::default(),
    );

    let result = resolver.resolve_test_file(project("/src/main.ds"), "./pkg");
    assert_eq!(
        result,
        Err(ResolverError::NotFound {
            specifier: "./pkg".into()
        })
    );
}

/// Hash and bare imports are not package semantics.
#[test]
fn test_rejects_package_imports() {
    let resolver = resolver(
        &[("/src/main.ds", ""), ("/src/pkg.ds", "")],
        ResolverOptions::default(),
    );

    let hash = resolver.resolve_test_file(project("/src/main.ds"), "#pkg");
    assert_eq!(
        hash,
        Err(ResolverError::NotFound {
            specifier: "#pkg".into()
        })
    );

    let bare = resolver.resolve_test_file(project("/src/main.ds"), "pkg");
    assert_eq!(
        bare,
        Err(ResolverError::NotFound {
            specifier: "pkg".into()
        })
    );
}

/// Empty specifiers are invalid.
#[test]
fn test_rejects_empty_specifiers() {
    let resolver = resolver(&[("/src/main.ds", "")], ResolverOptions::default());

    let result = resolver.resolve_test_file(project("/src/main.ds"), "");
    assert_eq!(
        result,
        Err(ResolverError::InvalidSpecifier {
            specifier: "".into(),
            message: Some("empty specifier".into())
        })
    );
}

/// Restrictions apply to direct and extensionless candidates.
#[test]
fn test_restrictions_apply_to_candidates() {
    let resolver = resolver(
        &[
            ("/src/main.ds", ""),
            ("/src/private/user.ds", ""),
            ("/src/public/user.ds", ""),
        ],
        ResolverOptions {
            roots: vec![project("/src")],
            restrictions: vec![Restriction::Function(Arc::new(|candidate| {
                !candidate.starts_with(project("/src/private"))
            }))],
            ..ResolverOptions::default()
        },
    );

    let blocked = resolver.resolve_test_file(project("/src/main.ds"), "/private/user");
    assert_eq!(
        blocked,
        Err(ResolverError::NotFound {
            specifier: "/private/user".into()
        })
    );

    let allowed = resolver
        .resolve_test_file(project("/src/main.ds"), "/public/user")
        .map(Resolution::into_path_buf);
    assert_eq!(allowed, Ok(project("/src/public/user.ds")));
}
