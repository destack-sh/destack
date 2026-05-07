use crate::{AliasValue, Resolution, ResolverError, ResolverOptions};

use super::support::{project, resolver};

/// Bare specifiers only resolve through explicit aliases.
#[test]
fn test_resolves_prefix_aliases() {
    let resolver = resolver(
        &[("/src/main.ds", ""), ("/src/lib/user.ds", "")],
        ResolverOptions {
            alias: vec![("@lib".into(), vec![AliasValue::from("./lib")])],
            ..ResolverOptions::default()
        },
    );

    let aliased = resolver
        .resolve_test_file(project("/src/main.ds"), "@lib/user")
        .map(Resolution::into_path_buf);
    assert_eq!(aliased, Ok(project("/src/lib/user.ds")));

    let bare = resolver.resolve_test_file(project("/src/main.ds"), "react");
    assert_eq!(
        bare,
        Err(ResolverError::NotFound {
            specifier: "react".into()
        })
    );
}

/// Exact aliases do not match subpaths.
#[test]
fn test_resolves_exact_aliases() {
    let resolver = resolver(
        &[
            ("/src/main.ds", ""),
            ("/src/exact.ds", ""),
            ("/src/lib/exact.ds", ""),
        ],
        ResolverOptions {
            alias: vec![("@exact$".into(), vec![AliasValue::from("./exact")])],
            ..ResolverOptions::default()
        },
    );

    let exact = resolver
        .resolve_test_file(project("/src/main.ds"), "@exact")
        .map(Resolution::into_path_buf);
    assert_eq!(exact, Ok(project("/src/exact.ds")));

    let subpath = resolver.resolve_test_file(project("/src/main.ds"), "@exact/subpath");
    assert_eq!(
        subpath,
        Err(ResolverError::NotFound {
            specifier: "@exact/subpath".into()
        })
    );
}

/// Wildcard aliases substitute the matched segment.
#[test]
fn test_resolves_wildcard_aliases() {
    let resolver = resolver(
        &[("/src/main.ds", ""), ("/src/features/user/view.ds", "")],
        ResolverOptions {
            alias: vec![("@features/*".into(), vec![AliasValue::from("./features/*")])],
            ..ResolverOptions::default()
        },
    );

    let result = resolver
        .resolve_test_file(project("/src/main.ds"), "@features/user/view")
        .map(Resolution::into_path_buf);
    assert_eq!(result, Ok(project("/src/features/user/view.ds")));
}

/// Alias values are tried in order.
#[test]
fn test_resolves_ordered_alias_values() {
    let resolver = resolver(
        &[("/src/main.ds", ""), ("/src/second/user.ds", "")],
        ResolverOptions {
            alias: vec![(
                "@lib".into(),
                vec![AliasValue::from("./first"), AliasValue::from("./second")],
            )],
            ..ResolverOptions::default()
        },
    );

    let result = resolver
        .resolve_test_file(project("/src/main.ds"), "@lib/user")
        .map(Resolution::into_path_buf);
    assert_eq!(result, Ok(project("/src/second/user.ds")));
}

/// Ignored aliases fail before path probing.
#[test]
fn test_resolves_ignored_aliases() {
    let resolver = resolver(
        &[("/src/main.ds", ""), ("/src/secret.ds", "")],
        ResolverOptions {
            alias: vec![("@secret".into(), vec![AliasValue::Ignore])],
            ..ResolverOptions::default()
        },
    );

    let result = resolver.resolve_test_file(project("/src/main.ds"), "@secret");
    assert_eq!(
        result,
        Err(ResolverError::Ignored {
            path: project("/src/@secret")
        })
    );
}

/// Matched aliases fail before ordinary bare lookup.
#[test]
fn test_rejects_matched_alias_misses() {
    let resolver = resolver(
        &[("/src/main.ds", "")],
        ResolverOptions {
            alias: vec![("@missing".into(), vec![AliasValue::from("./missing")])],
            ..ResolverOptions::default()
        },
    );

    let result = resolver.resolve_test_file(project("/src/main.ds"), "@missing/value");
    assert_eq!(
        result,
        Err(ResolverError::MatchedAliasNotFound {
            specifier: "@missing/value".into(),
            alias_key: "@missing".into()
        })
    );
}

/// Alias cycles fail loudly.
#[test]
fn test_alias_cycles_fail() {
    let resolver = resolver(
        &[("/src/main.ds", "")],
        ResolverOptions {
            alias: vec![
                ("@a".into(), vec![AliasValue::from("@b")]),
                ("@b".into(), vec![AliasValue::from("@a")]),
            ],
            ..ResolverOptions::default()
        },
    );

    let result = resolver.resolve_test_file(project("/src/main.ds"), "@a");
    assert!(matches!(
        result,
        Err(ResolverError::RecursiveDependency { .. })
    ));
}
