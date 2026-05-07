use crate::{Mount, Resolution, ResolverError, ResolverOptions};

use super::support::{project, resolver};

/// Mounts resolve exact package imports through their entry path.
#[test]
fn test_resolves_mounted_entries() {
    let resolver = resolver(
        &[("/src/main.ds", ""), ("/packages/fs/src/index.ds", "")],
        ResolverOptions {
            mounts: vec![Mount::with_entry(
                "destack:fs",
                project("/packages/fs/src"),
                "index",
            )],
            ..ResolverOptions::default()
        },
    );

    let result = resolver
        .resolve_test_file(project("/src/main.ds"), "destack:fs")
        .map(Resolution::into_path_buf);
    assert_eq!(result, Ok(project("/packages/fs/src/index.ds")));
}

/// Mounts resolve package subpaths inside the mounted source root.
#[test]
fn test_resolves_mounted_subpaths() {
    let resolver = resolver(
        &[("/src/main.ds", ""), ("/packages/path/src/posix.ds", "")],
        ResolverOptions {
            mounts: vec![Mount::new("destack:path", project("/packages/path/src"))],
            ..ResolverOptions::default()
        },
    );

    let result = resolver
        .resolve_test_file(project("/src/main.ds"), "destack:path/posix")
        .map(Resolution::into_path_buf);
    assert_eq!(result, Ok(project("/packages/path/src/posix.ds")));
}

/// Mount misses do not fall back to package discovery.
#[test]
fn test_rejects_missing_mounted_paths() {
    let resolver = resolver(
        &[("/src/main.ds", "")],
        ResolverOptions {
            mounts: vec![Mount::new("destack:path", project("/packages/path/src"))],
            ..ResolverOptions::default()
        },
    );

    let result = resolver.resolve_test_file(project("/src/main.ds"), "destack:path/posix");
    assert_eq!(
        result,
        Err(ResolverError::NotFound {
            specifier: "destack:path/posix".into()
        })
    );
}

/// Mounts work before platform path classification.
#[test]
fn test_resolves_colon_mounts() {
    let resolver = resolver(
        &[("/src/main.ds", ""), ("/packages/gpu/src/device.ds", "")],
        ResolverOptions {
            mounts: vec![Mount::new("destack:gpu", project("/packages/gpu/src"))],
            ..ResolverOptions::default()
        },
    );

    let result = resolver
        .resolve_test_file(project("/src/main.ds"), "destack:gpu/device")
        .map(Resolution::into_path_buf);
    assert_eq!(result, Ok(project("/packages/gpu/src/device.ds")));
}

/// Mounts use the longest matching specifier prefix.
#[test]
fn test_resolves_longest_mount_prefix() {
    let resolver = resolver(
        &[
            ("/src/main.ds", ""),
            ("/packages/base/src/ui/button.ds", ""),
            ("/packages/ui/src/button.ds", ""),
        ],
        ResolverOptions {
            mounts: vec![
                Mount::new("@scope", project("/packages/base/src")),
                Mount::new("@scope/ui", project("/packages/ui/src")),
            ],
            ..ResolverOptions::default()
        },
    );

    let result = resolver
        .resolve_test_file(project("/src/main.ds"), "@scope/ui/button")
        .map(Resolution::into_path_buf);
    assert_eq!(result, Ok(project("/packages/ui/src/button.ds")));
}
