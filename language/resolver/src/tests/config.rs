use crate::{CachePolicy, ResolverError, ResolverOptions};

use super::support::{project, resolver, test_resolve_context};

/// Destack config paths materialize from directories and extensionless paths.
#[test]
fn test_reads_destack_paths() {
    let resolver = resolver(
        &[
            ("/app/destack.json", r#"{"name": "app"}"#),
            ("/other.json", r#"{"name": "other"}"#),
        ],
        ResolverOptions::default(),
    );
    let mut ctx = test_resolve_context();

    let directory = resolver
        .read_destack(&project("/app"), &mut ctx, CachePolicy::Reload)
        .map(|config| config.name);
    assert_eq!(directory, Ok(Some("app".into())));

    let extensionless = resolver
        .read_destack(&project("/other"), &mut ctx, CachePolicy::Reload)
        .map(|config| config.name);
    assert_eq!(extensionless, Ok(Some("other".into())));
}

/// Destack config extends merge parent package defaults.
#[test]
fn test_reads_destack_extends() {
    let resolver = resolver(
        &[
            ("/base.json", r#"{"name": "base"}"#),
            ("/app/destack.json", r#"{"extends": "../base.json"}"#),
        ],
        ResolverOptions::default(),
    );
    let mut ctx = test_resolve_context();

    let result = resolver
        .read_destack(&project("/app"), &mut ctx, CachePolicy::Reload)
        .map(|config| config.name);
    assert_eq!(result, Ok(Some("base".into())));
}

/// Destack config extends only accepts path specifiers.
#[test]
fn test_rejects_bare_destack_extends() {
    let resolver = resolver(
        &[("/app/destack.json", r#"{"extends": "preset"}"#)],
        ResolverOptions::default(),
    );
    let mut ctx = test_resolve_context();

    let result = resolver.read_destack(&project("/app"), &mut ctx, CachePolicy::Reload);
    assert!(matches!(
        result,
        Err(ResolverError::InvalidSpecifier {
            specifier,
            message: Some(message),
        })
        if specifier == "preset"
            && message == "destack config extends must use an absolute or relative path"
    ));
}

/// Destack config extends rejects cycles.
#[test]
fn test_rejects_circular_destack_extends() {
    let resolver = resolver(
        &[
            ("/a.json", r#"{"extends": "./b.json"}"#),
            ("/b.json", r#"{"extends": "./a.json"}"#),
        ],
        ResolverOptions::default(),
    );
    let mut ctx = test_resolve_context();

    let result = resolver.read_destack(&project("/a.json"), &mut ctx, CachePolicy::Reload);
    assert!(matches!(result, Err(ResolverError::DestackCircular { .. })));
}
