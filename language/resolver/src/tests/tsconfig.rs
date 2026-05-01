use crate::{
    Resolver, ResolverError, ResolverOptions, TypeScriptOptionsDiscovery,
    TypeScriptOptionsLocation, TypeScriptOptionsReferences,
};

/// Test extending a tsconfig file.
#[test]
fn test_extend_tsconfig() {
    let f = super::fixture_root().join("tsconfig/cases/extends");

    let resolver = Resolver::for_tests(ResolverOptions {
        tsconfig: Some(TypeScriptOptionsDiscovery::Manual(
            TypeScriptOptionsLocation {
                config_file: f.join("tsconfig.json"),
                references: TypeScriptOptionsReferences::Automatic,
            },
        )),
        ..ResolverOptions::default()
    });

    let resolution = resolver.resolve_tsconfig(&f).expect("resolved");

    // Should inherit tsconfig from parent
    assert_eq!(resolution.json.files, Some(vec!["files".to_string()]));
    assert_eq!(resolution.json.include, Some(vec!["include".to_string()]));
    assert_eq!(resolution.json.exclude, Some(vec!["exclude".to_string()]));

    let compiler_options = &resolution.json.compiler_options;
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

    let resolver = Resolver::for_tests(ResolverOptions {
        tsconfig: Some(TypeScriptOptionsDiscovery::Manual(
            TypeScriptOptionsLocation {
                config_file: f.join("tsconfig.json"),
                references: TypeScriptOptionsReferences::Automatic,
            },
        )),
        extensions: vec![".ts".into(), ".js".into()],
        ..ResolverOptions::default()
    });

    // Test that paths are resolved correctly after inheritance
    let resolved_path = resolver
        .resolve_test_directory(&f, "@/test")
        .map(|f| f.full_path());
    assert_eq!(resolved_path, Ok(f.join("src/test.ts")));
}

/// Test extending tsconfig override behavior.
#[test]
fn test_extend_tsconfig_override_behavior() {
    let f = super::fixture_root().join("tsconfig/cases/extends-override");

    let resolver = Resolver::for_tests(ResolverOptions {
        tsconfig: Some(TypeScriptOptionsDiscovery::Manual(
            TypeScriptOptionsLocation {
                config_file: f.join("tsconfig.json"),
                references: TypeScriptOptionsReferences::Automatic,
            },
        )),
        ..ResolverOptions::default()
    });

    let resolution = resolver.resolve_tsconfig(&f).expect("resolved");
    let compiler_options = &resolution.json.compiler_options;

    // Child should override parent values
    assert_eq!(compiler_options.jsx, Some("react".to_string()));
    assert_eq!(compiler_options.target, Some("ES2020".to_string()));
}

/// Test extending tsconfig template variables.
#[test]
fn test_extend_tsconfig_template_variables() {
    let f = super::fixture_root().join("tsconfig/cases/extends-template-vars");

    let resolver = Resolver::for_tests(ResolverOptions {
        tsconfig: Some(TypeScriptOptionsDiscovery::Manual(
            TypeScriptOptionsLocation {
                config_file: f.join("tsconfig.json"),
                references: TypeScriptOptionsReferences::Automatic,
            },
        )),
        extensions: vec![".ts".into(), ".js".into()],
        ..ResolverOptions::default()
    });

    // Test that template variables work correctly with extends
    let resolved_path = resolver
        .resolve_test_directory(&f, "@/utils")
        .map(|f| f.full_path());
    assert_eq!(resolved_path, Ok(f.join("src/utils.ts")));
}

/// Test extending tsconfig missing file.
#[test]
fn test_extend_tsconfig_missing_file() {
    use crate::ResolverError;

    let f = super::fixture_root().join("tsconfig/cases");

    let resolver = Resolver::for_tests(ResolverOptions {
        tsconfig: Some(TypeScriptOptionsDiscovery::Manual(
            TypeScriptOptionsLocation {
                config_file: f.join("nonexistent-tsconfig.json"),
                references: TypeScriptOptionsReferences::Automatic,
            },
        )),
        ..ResolverOptions::default()
    });

    let result = resolver.resolve_tsconfig(&f);
    assert!(matches!(
        result,
        Err(ResolverError::TsConfigNotFound { path: _ })
    ));
}

/// Test extending tsconfig multiple inheritance.
#[test]
fn test_extend_tsconfig_multiple_inheritance() {
    let f = super::fixture_root().join("tsconfig/cases/extends-chain");

    let resolver = Resolver::for_tests(ResolverOptions {
        tsconfig: Some(TypeScriptOptionsDiscovery::Manual(
            TypeScriptOptionsLocation {
                config_file: f.join("tsconfig.json"),
                references: TypeScriptOptionsReferences::Automatic,
            },
        )),
        ..ResolverOptions::default()
    });

    let resolution = resolver.resolve_tsconfig(&f).expect("resolved");
    let compiler_options = &resolution.json.compiler_options;

    // Should have settings from all configs in the chain
    assert_eq!(compiler_options.experimental_decorators, Some(true));
    assert_eq!(compiler_options.target, Some("ES2022".to_string()));
    assert_eq!(compiler_options.module, Some("ESNext".to_string()));
}

/// Test extending tsconfig preserves child settings.
#[test]
fn test_extend_tsconfig_preserves_child_settings() {
    let f = super::fixture_root().join("tsconfig/cases/extends-preserve-child");

    let resolver = Resolver::for_tests(ResolverOptions {
        tsconfig: Some(TypeScriptOptionsDiscovery::Manual(
            TypeScriptOptionsLocation {
                config_file: f.join("tsconfig.json"),
                references: TypeScriptOptionsReferences::Automatic,
            },
        )),
        ..ResolverOptions::default()
    });

    let resolution = resolver.resolve_tsconfig(&f).expect("resolved");
    let compiler_options = &resolution.json.compiler_options;

    // Child should preserve its own settings and not inherit conflicting ones
    assert_eq!(compiler_options.jsx, Some("preserve".to_string())); // Child value
    assert_eq!(compiler_options.target, Some("ES2020".to_string())); // Inherited from parent
}

/// Prefer tsconfig paths aliases over package exports subpath mappings.
#[test]
fn test_paths_prefer_over_package_exports_subpath() {
    let fixture = super::fixture_root().join("tsconfig/cases/paths-prefer-over-exports");

    let resolver = Resolver::for_tests(ResolverOptions {
        tsconfig: Some(TypeScriptOptionsDiscovery::Manual(
            TypeScriptOptionsLocation {
                config_file: fixture.join("tsconfig.json"),
                references: TypeScriptOptionsReferences::Automatic,
            },
        )),
        extensions: vec![".ts".into(), ".d.ts".into(), ".js".into()],
        ..ResolverOptions::default()
    });

    let resolved_path = resolver
        .resolve_test_directory(
            fixture.join("src"),
            "@angular/compiler-cli/private/localize",
        )
        .map(|resolution| resolution.full_path());

    assert_eq!(
        resolved_path,
        Ok(fixture.join("packages/compiler-cli/private/localize.ts"))
    );
}

/// Effective tsconfig lookup should follow solution references for files.
#[test]
fn test_find_tsconfig_prefers_referenced_solution_for_file() {
    let fixture = super::fixture_root().join("tsconfig/cases/find-solution-references");

    let resolver = Resolver::for_tests(ResolverOptions {
        tsconfig: Some(TypeScriptOptionsDiscovery::Automatic),
        ..ResolverOptions::default()
    });

    // each source file should map to its referenced project tsconfig
    let foo_tsconfig = resolver
        .find_test_tsconfig_for_file(&fixture.join("src/foo.ts"))
        .expect("expected file lookup to succeed")
        .expect("expected foo tsconfig");
    let bar_tsconfig = resolver
        .find_test_tsconfig_for_file(&fixture.join("src/bar.ts"))
        .expect("expected file lookup to succeed")
        .expect("expected bar tsconfig");

    assert_eq!(foo_tsconfig.path, fixture.join("tsconfig.foo.json"));
    assert_eq!(bar_tsconfig.path, fixture.join("tsconfig.bar.json"));
}

/// File base tsconfig lookup should reject directories loudly.
#[test]
fn test_find_tsconfig_for_file_rejects_directory_input() {
    let fixture = super::fixture_root().join("tsconfig/cases/find-solution-references");

    let resolver = Resolver::for_tests(ResolverOptions {
        tsconfig: Some(TypeScriptOptionsDiscovery::Automatic),
        ..ResolverOptions::default()
    });

    let directory = fixture.join("src");
    let tsconfig_id = resolver.find_test_tsconfig_for_file(&directory);

    assert!(matches!(
        tsconfig_id,
        Err(ResolverError::ExpectedFilePath { path }) if path == directory
    ));
}

/// Directory base tsconfig lookup should reject files loudly.
#[test]
fn test_find_tsconfig_for_directory_rejects_file_input() {
    let fixture = super::fixture_root().join("tsconfig/cases/find-solution-references");

    let resolver = Resolver::for_tests(ResolverOptions {
        tsconfig: Some(TypeScriptOptionsDiscovery::Automatic),
        ..ResolverOptions::default()
    });

    let file = fixture.join("src/foo.ts");
    let tsconfig_id = resolver.find_test_tsconfig_for_directory(&file);

    assert!(matches!(
        tsconfig_id,
        Err(ResolverError::ExpectedDirectoryPath { path }) if path == file
    ));
}

/// File base resolution should disambiguate referenced projects.
#[test]
fn test_resolve_from_file_prefers_referenced_solution_project() {
    let fixture = super::fixture_root().join("tsconfig/cases/find-solution-references");

    let resolver = Resolver::for_tests(ResolverOptions {
        tsconfig: Some(TypeScriptOptionsDiscovery::Manual(
            TypeScriptOptionsLocation {
                config_file: fixture.join("tsconfig.json"),
                references: TypeScriptOptionsReferences::Automatic,
            },
        )),
        extensions: vec![".ts".into(), ".js".into()],
        ..ResolverOptions::default()
    });

    // file base should select the referenced project that owns the file
    let foo_resolution = resolver
        .resolve_test_file(fixture.join("src/foo.ts"), "@/util")
        .map(|resolution| resolution.full_path());
    let bar_resolution = resolver
        .resolve_test_file(fixture.join("src/bar.ts"), "@/util")
        .map(|resolution| resolution.full_path());

    assert_eq!(foo_resolution, Ok(fixture.join("src/foo/util.ts")));
    assert_eq!(bar_resolution, Ok(fixture.join("src/bar/util.ts")));
}

/// File base resolution should reject directories loudly.
#[test]
fn test_resolve_from_file_rejects_directory_input() {
    let fixture = super::fixture_root().join("tsconfig/cases/find-solution-references");

    let resolver = Resolver::for_tests(ResolverOptions {
        tsconfig: Some(TypeScriptOptionsDiscovery::Manual(
            TypeScriptOptionsLocation {
                config_file: fixture.join("tsconfig.json"),
                references: TypeScriptOptionsReferences::Automatic,
            },
        )),
        extensions: vec![".ts".into(), ".js".into()],
        ..ResolverOptions::default()
    });

    let directory = fixture.join("src");
    let resolution = resolver.resolve_test_file(&directory, "@/util");

    assert_eq!(
        resolution,
        Err(ResolverError::ExpectedFilePath {
            path: directory.clone()
        })
    );
}

/// Directory base resolution should reject files loudly.
#[test]
fn test_resolve_from_directory_rejects_file_input() {
    let fixture = super::fixture_root().join("tsconfig/cases/find-solution-references");

    let resolver = Resolver::for_tests(ResolverOptions {
        tsconfig: Some(TypeScriptOptionsDiscovery::Manual(
            TypeScriptOptionsLocation {
                config_file: fixture.join("tsconfig.json"),
                references: TypeScriptOptionsReferences::Automatic,
            },
        )),
        extensions: vec![".ts".into(), ".js".into()],
        ..ResolverOptions::default()
    });

    let file = fixture.join("src/foo.ts");
    let resolution = resolver.resolve_test_directory(&file, "@/util");

    assert_eq!(
        resolution,
        Err(ResolverError::ExpectedDirectoryPath { path: file.clone() })
    );
}

/// Directory base should remain heuristic when multiple referenced projects overlap.
#[test]
fn test_resolve_from_directory_remains_heuristic_for_solution_project() {
    let fixture = super::fixture_root().join("tsconfig/cases/find-solution-references");

    let resolver = Resolver::for_tests(ResolverOptions {
        tsconfig: Some(TypeScriptOptionsDiscovery::Manual(
            TypeScriptOptionsLocation {
                config_file: fixture.join("tsconfig.json"),
                references: TypeScriptOptionsReferences::Automatic,
            },
        )),
        extensions: vec![".ts".into(), ".js".into()],
        ..ResolverOptions::default()
    });

    // directory base does not identify which referenced project owns the import
    let resolution = resolver
        .resolve_test_directory(fixture.join("src"), "@/util")
        .map(|resolution| resolution.full_path());

    assert_eq!(resolution, Ok(fixture.join("src/foo/util.ts")));
}

/// Manual solution configs should still select the referenced project for paths.
#[test]
fn test_manual_tsconfig_prefers_referenced_project_paths() {
    let fixture = super::fixture_root().join("tsconfig/cases/manual-solution-references");

    let resolver = Resolver::for_tests(ResolverOptions {
        tsconfig: Some(TypeScriptOptionsDiscovery::Manual(
            TypeScriptOptionsLocation {
                config_file: fixture.join("tsconfig.json"),
                references: TypeScriptOptionsReferences::Automatic,
            },
        )),
        extensions: vec![".ts".into(), ".js".into()],
        ..ResolverOptions::default()
    });

    // referenced project aliases should resolve even when the root config is manual
    let resolved_path = resolver
        .resolve_test_directory(fixture.join("packages/foo/src"), "@/util")
        .map(|resolution| resolution.full_path());

    assert_eq!(
        resolved_path,
        Ok(fixture.join("packages/foo/src/aliased/util.ts"))
    );
}

/// Derived tsconfig options should track merged and built content.
#[test]
fn test_extend_tsconfig_refreshes_options() {
    let fixture = super::fixture_root().join("tsconfig/cases/extends");

    let resolver = Resolver::for_tests(ResolverOptions {
        tsconfig: Some(TypeScriptOptionsDiscovery::Manual(
            TypeScriptOptionsLocation {
                config_file: fixture.join("tsconfig.json"),
                references: TypeScriptOptionsReferences::Automatic,
            },
        )),
        ..ResolverOptions::default()
    });

    // derived options should reflect inherited content instead of raw parse state
    let tsconfig = resolver.resolve_tsconfig(&fixture).expect("resolved");

    let tsconfig_options = tsconfig.options();

    assert_eq!(
        tsconfig_options.compiler.base_url,
        Some(fixture.join("src"))
    );
    assert!(tsconfig_options.compiler.allow_js);
    assert!(tsconfig_options.compiler.emit_decorator_metadata);
    assert!(tsconfig_options.compiler.use_define_for_class_fields);
    assert!(tsconfig_options.compiler.rewrite_relative_import_extensions);
}
