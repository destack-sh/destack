use crate::{
    ResolveError, ResolveOptions, Resolver, TypeScriptOptionsDiscovery, TypeScriptOptionsLocation,
    TypeScriptOptionsReferences,
};

/// Test discovering a tsconfig file in a virtual file importer.
#[test]
fn tsconfig_discovery_virtual_file_importer() {
    let f = super::fixture_root().join("tsconfig");

    let resolver = Resolver::physical(ResolveOptions {
        tsconfig: Some(TypeScriptOptionsDiscovery::Automatic),
        cwd: Some(f.join("cases/index")),
        ..ResolveOptions::default()
    });

    let resolved_path = resolver
        .resolve("\0virtual-module", "random-import")
        .map(|f| f.full_path());
    assert_eq!(
        resolved_path,
        Err(ResolveError::NotFound {
            specifier: "random-import".into()
        })
    );
}

/// Test extending a tsconfig file.
#[test]
fn test_extend_tsconfig() {
    let f = super::fixture_root().join("tsconfig/cases/extends");

    let resolver = Resolver::physical(ResolveOptions {
        tsconfig: Some(TypeScriptOptionsDiscovery::Manual(
            TypeScriptOptionsLocation {
                config_file: f.join("tsconfig.json"),
                references: TypeScriptOptionsReferences::Automatic,
            },
        )),
        ..ResolveOptions::default()
    });

    let tsconfig_id = resolver.resolve_tsconfig(&f).expect("resolved");
    let resolution = resolver.get_tsconfig(tsconfig_id);

    // Should inherit tsconfig from parent
    assert_eq!(resolution.content.files, Some(vec!["files".to_string()]));
    assert_eq!(
        resolution.content.include,
        Some(vec!["include".to_string()])
    );
    assert_eq!(
        resolution.content.exclude,
        Some(vec!["exclude".to_string()])
    );

    let compiler_options = &resolution.content.compiler_options;
    assert_eq!(compiler_options.base_url, Some(f.join("src")));
    assert_eq!(compiler_options.allow_js, Some(true));
    assert_eq!(compiler_options.emit_decorator_metadata, Some(true));
    assert_eq!(compiler_options.use_define_for_class_fields, Some(true));
    assert_eq!(
        compiler_options.rewrite_relative_import_extensions,
        Some(true)
    );

    assert_eq!(compiler_options.jsx, Some("react-jsx".to_string()));
    assert_eq!(
        compiler_options.jsx_factory,
        Some("React.createElement".to_string())
    );
    assert_eq!(
        compiler_options.jsx_fragment_factory,
        Some("React.Fragment".to_string())
    );
    assert_eq!(
        compiler_options.jsx_import_source,
        Some("react".to_string())
    );
}

/// Test extending tsconfig paths.
#[test]
fn test_extend_tsconfig_paths() {
    let f = super::fixture_root().join("tsconfig/cases/extends-paths-inheritance");

    let resolver = Resolver::physical(ResolveOptions {
        tsconfig: Some(TypeScriptOptionsDiscovery::Manual(
            TypeScriptOptionsLocation {
                config_file: f.join("tsconfig.json"),
                references: TypeScriptOptionsReferences::Automatic,
            },
        )),
        extensions: vec![".ts".into(), ".js".into()],
        ..ResolveOptions::default()
    });

    // Test that paths are resolved correctly after inheritance
    let resolved_path = resolver.resolve(&f, "@/test").map(|f| f.full_path());
    assert_eq!(resolved_path, Ok(f.join("src/test.ts")));
}

/// Test extending tsconfig override behavior.
#[test]
fn test_extend_tsconfig_override_behavior() {
    let f = super::fixture_root().join("tsconfig/cases/extends-override");

    let resolver = Resolver::physical(ResolveOptions {
        tsconfig: Some(TypeScriptOptionsDiscovery::Manual(
            TypeScriptOptionsLocation {
                config_file: f.join("tsconfig.json"),
                references: TypeScriptOptionsReferences::Automatic,
            },
        )),
        ..ResolveOptions::default()
    });

    let tsconfig_id = resolver.resolve_tsconfig(&f).expect("resolved");
    let resolution = resolver.get_tsconfig(tsconfig_id);
    let compiler_options = &resolution.content.compiler_options;

    // Child should override parent values
    assert_eq!(compiler_options.jsx, Some("react".to_string()));
    assert_eq!(compiler_options.target, Some("ES2020".to_string()));
}

/// Test extending tsconfig template variables.
#[test]
fn test_extend_tsconfig_template_variables() {
    let f = super::fixture_root().join("tsconfig/cases/extends-template-vars");

    let resolver = Resolver::physical(ResolveOptions {
        tsconfig: Some(TypeScriptOptionsDiscovery::Manual(
            TypeScriptOptionsLocation {
                config_file: f.join("tsconfig.json"),
                references: TypeScriptOptionsReferences::Automatic,
            },
        )),
        extensions: vec![".ts".into(), ".js".into()],
        ..ResolveOptions::default()
    });

    // Test that template variables work correctly with extends
    let resolved_path = resolver.resolve(&f, "@/utils").map(|f| f.full_path());
    assert_eq!(resolved_path, Ok(f.join("src/utils.ts")));
}

/// Test extending tsconfig missing file.
#[test]
fn test_extend_tsconfig_missing_file() {
    use crate::ResolveError;

    let f = super::fixture_root().join("tsconfig/cases");

    let resolver = Resolver::physical(ResolveOptions {
        tsconfig: Some(TypeScriptOptionsDiscovery::Manual(
            TypeScriptOptionsLocation {
                config_file: f.join("nonexistent-tsconfig.json"),
                references: TypeScriptOptionsReferences::Automatic,
            },
        )),
        ..ResolveOptions::default()
    });

    let result = resolver.resolve_tsconfig(&f);
    assert!(matches!(
        result,
        Err(ResolveError::TsConfigNotFound { path: _ })
    ));
}

/// Test extending tsconfig multiple inheritance.
#[test]
fn test_extend_tsconfig_multiple_inheritance() {
    let f = super::fixture_root().join("tsconfig/cases/extends-chain");

    let resolver = Resolver::physical(ResolveOptions {
        tsconfig: Some(TypeScriptOptionsDiscovery::Manual(
            TypeScriptOptionsLocation {
                config_file: f.join("tsconfig.json"),
                references: TypeScriptOptionsReferences::Automatic,
            },
        )),
        ..ResolveOptions::default()
    });

    let tsconfig_id = resolver.resolve_tsconfig(&f).expect("resolved");
    let resolution = resolver.get_tsconfig(tsconfig_id);
    let compiler_options = &resolution.content.compiler_options;

    // Should have settings from all configs in the chain
    assert_eq!(compiler_options.experimental_decorators, Some(true));
    assert_eq!(compiler_options.target, Some("ES2022".to_string()));
    assert_eq!(compiler_options.module, Some("ESNext".to_string()));
}

/// Test extending tsconfig preserves child settings.
#[test]
fn test_extend_tsconfig_preserves_child_settings() {
    let f = super::fixture_root().join("tsconfig/cases/extends-preserve-child");

    let resolver = Resolver::physical(ResolveOptions {
        tsconfig: Some(TypeScriptOptionsDiscovery::Manual(
            TypeScriptOptionsLocation {
                config_file: f.join("tsconfig.json"),
                references: TypeScriptOptionsReferences::Automatic,
            },
        )),
        ..ResolveOptions::default()
    });

    let tsconfig_id = resolver.resolve_tsconfig(&f).expect("resolved");
    let resolution = resolver.get_tsconfig(tsconfig_id);
    let compiler_options = &resolution.content.compiler_options;

    // Child should preserve its own settings and not inherit conflicting ones
    assert_eq!(compiler_options.jsx, Some("preserve".to_string())); // Child value
    assert_eq!(compiler_options.target, Some("ES2020".to_string())); // Inherited from parent
}
