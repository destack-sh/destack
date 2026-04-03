use std::path::{Path, PathBuf};

use destack_artifact::ModuleEdgeRelation;
use destack_dir::DependencyKind;
use destack_resolver::{
    ResolveOptions, TypeScriptOptionsDiscovery, TypeScriptOptionsLocation,
    TypeScriptOptionsReferences,
};
use destack_source::LanguageType;
use destack_workspace::{ModuleFormat, ModuleResolution, NodeLinker, TsCompilerOptions};
use indexmap::IndexMap;

/// Extension alias order for TypeScript source imports.
const TYPESCRIPT_EXTENSION_ALIASES: &[(&str, &[&str])] = &[
    (".js", &[".ts", ".tsx", ".d.ts", ".js"]),
    (".jsx", &[".tsx", ".d.ts", ".jsx"]),
    (".mjs", &[".mts", ".d.mts", ".mjs"]),
    (".cjs", &[".cts", ".d.cts", ".cjs"]),
];

/// Default extension order for compiler import resolution.
const DEFAULT_IMPORT_EXTENSIONS: &[&str] = &[
    ".ds", ".d.ds", ".tsx", ".ts", ".d.ts", ".jsx", ".js", ".mjs", ".cjs", ".json", ".node",
];

/// Context used when materializing import resolve options.
#[derive(Debug, Clone, Copy)]
pub struct ImportResolveContext {
    /// The dependency kind for the import edge.
    pub dependency_kind: DependencyKind,
    /// The source language of the importing module.
    pub source_language_type: Option<LanguageType>,
    /// The edge semantics for this dependency.
    pub edge_relation: ModuleEdgeRelation,
}

impl ImportResolveContext {
    /// Return true when this request resolves a type dependency.
    pub fn is_type_dependency(self) -> bool {
        self.dependency_kind == DependencyKind::Type
    }

    /// Return true when this request resolves from TypeScript source.
    pub fn is_typescript_source(self) -> bool {
        self.source_language_type
            .is_some_and(|language_type| language_type.is_typescript())
    }

    /// Return true when this request resolves a require-like edge.
    pub fn is_require_edge(self) -> bool {
        self.edge_relation.is_require_like()
    }

    /// Return whether one default import may fall back to a CommonJS namespace symbol.
    pub fn allows_commonjs_default_namespace_import(
        self,
        target_module_format: Option<ModuleFormat>,
        is_typescript_commonjs_default_interop_enabled: bool,
    ) -> bool {
        // only value imports on import edges may use default namespace interop
        if self.dependency_kind != DependencyKind::Value || !self.edge_relation.is_import_like() {
            return false;
        }

        // esm targets must keep explicit default export requirements
        if target_module_format == Some(ModuleFormat::Esm) {
            return false;
        }

        // javascript source imports use runtime interop semantics
        if self
            .source_language_type
            .is_some_and(|language_type| language_type.is_javascript())
        {
            return true;
        }

        // typescript source imports require interop policy
        if self
            .source_language_type
            .is_some_and(|language_type| language_type.is_typescript())
        {
            return is_typescript_commonjs_default_interop_enabled;
        }

        // destack source keeps strict explicit default imports
        false
    }
}

/// Return whether TypeScript compiler options enable CommonJS default import interop.
pub fn typescript_commonjs_default_interop_is_enabled(options: &TsCompilerOptions) -> bool {
    // node esm module resolution semantics include commonjs default interop
    matches!(
        options.module_resolution,
        ModuleResolution::Node16 | ModuleResolution::NodeNext
    )
}

/// Materialize compiler import resolve options for one import request.
pub fn materialize_import_resolve_options(
    base: &ResolveOptions,
    context: ImportResolveContext,
) -> ResolveOptions {
    let mut options = base.clone();

    // normalize extension behavior for import resolution
    apply_import_extension_policy(&mut options.extensions);

    // normalize condition behavior for value and type dependencies
    apply_import_condition_policy(&mut options.conditions, context);

    // apply typescript extension aliases for typescript sources
    apply_import_extension_alias_policy(&mut options.extension_alias, context);

    options
}

/// Apply TypeScript compiler options to import resolve options for one source module.
pub fn apply_typescript_import_resolve_policy(
    options: &mut ResolveOptions,
    compiler_options: &TsCompilerOptions,
    source_language_type: Option<LanguageType>,
    config_file: PathBuf,
) {
    // use the source tsconfig for path mapping and project references
    options.tsconfig = Some(TypeScriptOptionsDiscovery::Manual(
        TypeScriptOptionsLocation {
            config_file,
            references: TypeScriptOptionsReferences::Automatic,
        },
    ));

    // mirror package json exports and imports toggles from tsconfig
    options.resolve_package_json_exports = compiler_options.resolve_package_json_exports;
    options.resolve_package_json_imports = compiler_options.resolve_package_json_imports;

    // add custom export conditions without duplicates
    append_missing_conditions(&mut options.conditions, &compiler_options.custom_conditions);

    // align json extension lookup for typescript source files
    apply_typescript_json_extension_policy(
        &mut options.extensions,
        source_language_type,
        compiler_options.resolve_json_module,
    );
}

/// Apply node linker policy to import resolve options for one workspace context.
pub fn apply_node_linker_resolve_policy(
    options: &mut ResolveOptions,
    node_linker: NodeLinker,
    cwd: &Path,
) {
    options.apply_node_linker_for_cwd(node_linker, cwd);
}

/// Append condition names when missing.
fn append_missing_conditions(conditions: &mut Vec<String>, extra_conditions: &[String]) {
    for condition in extra_conditions {
        if conditions.iter().any(|existing| existing == condition) {
            continue;
        }

        conditions.push(condition.clone());
    }
}

/// Apply tsconfig json extension policy for one source language.
fn apply_typescript_json_extension_policy(
    extensions: &mut Vec<String>,
    source_language_type: Option<LanguageType>,
    resolve_json_module: bool,
) {
    // only typescript source resolution is gated by resolveJsonModule
    if !source_language_type.is_some_and(|language_type| language_type.is_typescript()) {
        return;
    }

    // include json extension when tsconfig enables json modules
    if resolve_json_module {
        if !extensions.iter().any(|extension| extension == ".json") {
            extensions.push(".json".to_string());
        }

        return;
    }

    // remove json extension when tsconfig disables json modules
    extensions.retain(|extension| extension != ".json");
}

/// Resolve a declaration companion path for one JavaScript-like module path.
pub fn declaration_companion_path_for_module_path(path: &Path) -> Option<PathBuf> {
    let file_name = path.file_name()?.to_str()?;

    // map js-like extensions through the shared typescript alias policy
    for (from_extension, aliases) in TYPESCRIPT_EXTENSION_ALIASES {
        let Some(prefix) = file_name.strip_suffix(from_extension) else {
            continue;
        };

        let companion_extension = declaration_companion_extension_for_aliases(aliases)?;
        let companion_name = format!("{prefix}{companion_extension}");

        return Some(path.with_file_name(companion_name));
    }

    None
}

/// Resolve a declaration companion extension for one alias set.
fn declaration_companion_extension_for_aliases<'a>(aliases: &'a [&'a str]) -> Option<&'a str> {
    aliases
        .iter()
        .copied()
        .find(|alias| alias.starts_with(".d."))
}

/// Apply extension policy for import resolution.
fn apply_import_extension_policy(extensions: &mut Vec<String>) {
    // fill default import extensions when no extensions were configured
    if extensions.is_empty() {
        *extensions = DEFAULT_IMPORT_EXTENSIONS
            .iter()
            .map(|extension| (*extension).to_string())
            .collect();

        return;
    }

    // keep explicit extension order, but add declaration companions when source extensions exist
    insert_extension_after_if_present(extensions, ".ds", ".d.ds");
    insert_extension_after_if_present(extensions, ".ts", ".d.ts");
}

/// Insert one extension after a preferred predecessor when present.
fn insert_extension_after_if_present(extensions: &mut Vec<String>, after: &str, extension: &str) {
    if extensions.iter().any(|entry| entry == extension) {
        return;
    }

    let Some(index) = extensions.iter().position(|entry| entry == after) else {
        return;
    };

    extensions.insert(index + 1, extension.to_string());
}

/// Apply condition policy for import resolution.
fn apply_import_condition_policy(conditions: &mut Vec<String>, context: ImportResolveContext) {
    let mut normalized = Vec::new();

    // preserve existing conditions except edge and type conditions and duplicates
    for condition in conditions.iter() {
        if condition == "types" || condition == "import" || condition == "require" {
            continue;
        }
        if normalized.iter().any(|existing| existing == condition) {
            continue;
        }
        normalized.push(condition.clone());
    }

    // force types first for type dependencies
    if context.is_type_dependency() {
        normalized.insert(0, "types".to_string());
    }

    // ensure edge condition for compiler import resolution
    let exports_condition = if context.is_require_edge() {
        "require"
    } else {
        "import"
    };
    if !normalized
        .iter()
        .any(|condition| condition == exports_condition)
    {
        normalized.push(exports_condition.to_string());
    }

    *conditions = normalized;
}

/// Apply extension alias policy for import resolution.
fn apply_import_extension_alias_policy(
    extension_alias: &mut IndexMap<String, Vec<String>>,
    context: ImportResolveContext,
) {
    // skip extension alias expansion for non typescript source files
    if !context.is_typescript_source() {
        return;
    }

    // append typescript extension aliases without duplicating configured entries
    for (from_extension, aliases) in TYPESCRIPT_EXTENSION_ALIASES {
        let alias_entries = extension_alias
            .entry((*from_extension).to_string())
            .or_default();

        for alias in *aliases {
            if alias_entries.iter().any(|entry| entry == alias) {
                continue;
            }
            alias_entries.push((*alias).to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ImportResolveContext, apply_node_linker_resolve_policy,
        apply_typescript_import_resolve_policy, declaration_companion_path_for_module_path,
        materialize_import_resolve_options, typescript_commonjs_default_interop_is_enabled,
    };
    use std::path::{Path, PathBuf};

    use destack_artifact::ModuleEdgeRelation;
    use destack_dir::DependencyKind;
    use destack_resolver::{ResolveOptions, TypeScriptOptionsDiscovery};
    use destack_source::LanguageType;
    use destack_workspace::{ModuleFormat, ModuleResolution, NodeLinker, TsCompilerOptions};

    /// Build default import options for value dependencies.
    #[test]
    fn test_materialize_import_options_value_dependency_defaults() {
        let context = ImportResolveContext {
            dependency_kind: DependencyKind::Value,
            source_language_type: None,
            edge_relation: ModuleEdgeRelation::Import,
        };
        let options = materialize_import_resolve_options(&ResolveOptions::blank(), context);

        assert_eq!(
            options.extensions,
            vec![
                ".ds", ".d.ds", ".tsx", ".ts", ".d.ts", ".jsx", ".js", ".mjs", ".cjs", ".json",
                ".node"
            ]
        );
        assert_eq!(options.conditions, vec!["import"]);
        assert!(options.extension_alias.is_empty());
    }

    /// Build import options for TypeScript type dependencies.
    #[test]
    fn test_materialize_import_options_typescript_type_dependency() {
        let context = ImportResolveContext {
            dependency_kind: DependencyKind::Type,
            source_language_type: Some(LanguageType::TypeScript),
            edge_relation: ModuleEdgeRelation::Import,
        };
        let options = materialize_import_resolve_options(&ResolveOptions::blank(), context);

        assert_eq!(options.conditions, vec!["types", "import"]);

        let js_aliases = options
            .extension_alias
            .get(".js")
            .expect("missing .js extension aliases");
        assert_eq!(js_aliases, &vec![".ts", ".tsx", ".d.ts", ".js"]);
    }

    /// Preserve explicit extension lists without automatic default replacement.
    #[test]
    fn test_materialize_import_options_preserve_explicit_extensions() {
        let mut base = ResolveOptions::blank();
        base.extensions = vec![".js".to_string()];
        let context = ImportResolveContext {
            dependency_kind: DependencyKind::Value,
            source_language_type: None,
            edge_relation: ModuleEdgeRelation::Import,
        };

        let options = materialize_import_resolve_options(&base, context);

        assert_eq!(options.extensions, vec![".js"]);
    }

    /// Add declaration extensions only when matching source extensions exist.
    #[test]
    fn test_materialize_import_options_add_companion_declaration_extensions() {
        let mut base = ResolveOptions::blank();
        base.extensions = vec![".ts".to_string(), ".js".to_string(), ".ds".to_string()];
        let context = ImportResolveContext {
            dependency_kind: DependencyKind::Value,
            source_language_type: None,
            edge_relation: ModuleEdgeRelation::Import,
        };

        let options = materialize_import_resolve_options(&base, context);

        assert_eq!(
            options.extensions,
            vec![".ts", ".d.ts", ".js", ".ds", ".d.ds"]
        );
    }

    /// Build import options for require style dependencies.
    #[test]
    fn test_materialize_import_options_require_dependency_conditions() {
        let mut base = ResolveOptions::blank();
        base.conditions = vec!["import".to_string(), "node".to_string()];
        let context = ImportResolveContext {
            dependency_kind: DependencyKind::Value,
            source_language_type: Some(LanguageType::JavaScript),
            edge_relation: ModuleEdgeRelation::Require,
        };

        let options = materialize_import_resolve_options(&base, context);

        assert_eq!(options.conditions, vec!["node", "require"]);
    }

    /// Replace require conditions with import conditions for import edges.
    #[test]
    fn test_materialize_import_options_replace_require_with_import_condition() {
        let mut base = ResolveOptions::blank();
        base.conditions = vec!["require".to_string(), "node".to_string()];
        let context = ImportResolveContext {
            dependency_kind: DependencyKind::Value,
            source_language_type: Some(LanguageType::JavaScript),
            edge_relation: ModuleEdgeRelation::Import,
        };

        let options = materialize_import_resolve_options(&base, context);

        assert_eq!(options.conditions, vec!["node", "import"]);
    }

    /// Apply tsconfig resolver policy for TypeScript source imports.
    #[test]
    fn test_apply_typescript_import_resolve_policy_typescript_source() {
        let mut options = ResolveOptions::blank();
        options.conditions = vec!["node".to_string()];
        options.extensions = vec![".ts".to_string(), ".json".to_string()];

        let compiler_options = TsCompilerOptions {
            resolve_package_json_exports: false,
            resolve_package_json_imports: false,
            resolve_json_module: false,
            custom_conditions: vec!["development".to_string(), "node".to_string()],
            ..TsCompilerOptions::default()
        };

        apply_typescript_import_resolve_policy(
            &mut options,
            &compiler_options,
            Some(LanguageType::TypeScript),
            PathBuf::from("/tmp/tsconfig.json"),
        );

        // verify tsconfig path binding for resolver paths lookups
        assert!(matches!(
            options.tsconfig,
            Some(TypeScriptOptionsDiscovery::Manual(ref location))
                if location.config_file == Path::new("/tmp/tsconfig.json")
        ));

        // verify package json resolver toggles from tsconfig
        assert!(!options.resolve_package_json_exports);
        assert!(!options.resolve_package_json_imports);

        // verify custom condition merge and json extension removal
        assert_eq!(options.conditions, vec!["node", "development"]);
        assert_eq!(options.extensions, vec![".ts"]);
    }

    /// Keep json extension behavior for non TypeScript sources.
    #[test]
    fn test_apply_typescript_import_resolve_policy_javascript_source_keeps_json() {
        let mut options = ResolveOptions::blank();
        options.extensions = vec![".js".to_string(), ".json".to_string()];

        let compiler_options = TsCompilerOptions {
            resolve_json_module: false,
            ..TsCompilerOptions::default()
        };

        apply_typescript_import_resolve_policy(
            &mut options,
            &compiler_options,
            Some(LanguageType::JavaScript),
            PathBuf::from("/tmp/tsconfig.json"),
        );

        // verify javascript resolution keeps runtime json imports
        assert_eq!(options.extensions, vec![".js", ".json"]);
    }

    /// Apply explicit node_modules linker policy to resolver options.
    #[test]
    fn test_apply_node_linker_resolve_policy_node_modules() {
        let mut options = ResolveOptions::blank();

        apply_node_linker_resolve_policy(
            &mut options,
            NodeLinker::NodeModules,
            Path::new("/workspace"),
        );

        assert_eq!(options.cwd, Some(PathBuf::from("/workspace")));
        assert!(!options.yarn_pnp);
    }

    /// Apply explicit pnp linker policy to resolver options.
    #[test]
    fn test_apply_node_linker_resolve_policy_pnp() {
        let mut options = ResolveOptions::blank();

        apply_node_linker_resolve_policy(&mut options, NodeLinker::Pnp, Path::new("/workspace"));

        assert_eq!(options.cwd, Some(PathBuf::from("/workspace")));
        assert!(options.yarn_pnp);
    }

    /// Build declaration companion paths from JavaScript module paths.
    #[test]
    fn test_declaration_companion_path_for_module_path_javascript_extensions() {
        // .js and .jsx map to .d.ts
        assert_eq!(
            declaration_companion_path_for_module_path(Path::new("/tmp/mod.js")),
            Some(PathBuf::from("/tmp/mod.d.ts")),
        );
        assert_eq!(
            declaration_companion_path_for_module_path(Path::new("/tmp/view.jsx")),
            Some(PathBuf::from("/tmp/view.d.ts")),
        );

        // module format extensions map to declaration companions
        assert_eq!(
            declaration_companion_path_for_module_path(Path::new("/tmp/index.mjs")),
            Some(PathBuf::from("/tmp/index.d.mts")),
        );
        assert_eq!(
            declaration_companion_path_for_module_path(Path::new("/tmp/index.cjs")),
            Some(PathBuf::from("/tmp/index.d.cts")),
        );

        // non JavaScript-like extensions do not map
        assert_eq!(
            declaration_companion_path_for_module_path(Path::new("/tmp/mod.ts")),
            None,
        );
    }
    /// Allow javascript default imports from CommonJS namespace targets.
    #[test]
    fn test_commonjs_default_namespace_import_policy_javascript_source() {
        let context = ImportResolveContext {
            dependency_kind: DependencyKind::Value,
            source_language_type: Some(LanguageType::JavaScript),
            edge_relation: ModuleEdgeRelation::Import,
        };

        let allowed =
            context.allows_commonjs_default_namespace_import(Some(ModuleFormat::CommonJs), false);

        assert!(allowed);
    }

    /// Allow javascript fallback when runtime module format is unknown.
    #[test]
    fn test_commonjs_default_namespace_import_policy_javascript_unknown_target_format() {
        let context = ImportResolveContext {
            dependency_kind: DependencyKind::Value,
            source_language_type: Some(LanguageType::JavaScript),
            edge_relation: ModuleEdgeRelation::Import,
        };

        let allowed = context.allows_commonjs_default_namespace_import(None, false);

        assert!(allowed);
    }

    /// Reject typescript default imports from CommonJS without interop options.
    #[test]
    fn test_commonjs_default_namespace_import_policy_typescript_strict_default() {
        let context = ImportResolveContext {
            dependency_kind: DependencyKind::Value,
            source_language_type: Some(LanguageType::TypeScript),
            edge_relation: ModuleEdgeRelation::Import,
        };

        let allowed = context.allows_commonjs_default_namespace_import(
            Some(ModuleFormat::CommonJs),
            typescript_commonjs_default_interop_is_enabled(&TsCompilerOptions::default()),
        );

        assert!(!allowed);
    }

    /// Allow typescript default imports from CommonJS under nodenext resolution.
    #[test]
    fn test_commonjs_default_namespace_import_policy_typescript_nodenext_resolution() {
        let context = ImportResolveContext {
            dependency_kind: DependencyKind::Value,
            source_language_type: Some(LanguageType::TypeScript),
            edge_relation: ModuleEdgeRelation::Import,
        };
        let options = TsCompilerOptions {
            module_resolution: ModuleResolution::NodeNext,
            ..TsCompilerOptions::default()
        };

        let allowed = context.allows_commonjs_default_namespace_import(
            Some(ModuleFormat::CommonJs),
            typescript_commonjs_default_interop_is_enabled(&options),
        );

        assert!(allowed);
    }

    /// Allow typescript default imports from CommonJS under node16 style resolution.
    #[test]
    fn test_commonjs_default_namespace_import_policy_typescript_node16_resolution() {
        let context = ImportResolveContext {
            dependency_kind: DependencyKind::Value,
            source_language_type: Some(LanguageType::TypeScript),
            edge_relation: ModuleEdgeRelation::Import,
        };
        let options = TsCompilerOptions {
            module_resolution: ModuleResolution::Node16,
            ..TsCompilerOptions::default()
        };

        let allowed = context.allows_commonjs_default_namespace_import(
            Some(ModuleFormat::CommonJs),
            typescript_commonjs_default_interop_is_enabled(&options),
        );

        assert!(allowed);
    }

    /// Reject default namespace fallback for non commonjs targets.
    #[test]
    fn test_commonjs_default_namespace_import_policy_reject_non_commonjs_target() {
        let context = ImportResolveContext {
            dependency_kind: DependencyKind::Value,
            source_language_type: Some(LanguageType::JavaScript),
            edge_relation: ModuleEdgeRelation::Import,
        };

        let allowed =
            context.allows_commonjs_default_namespace_import(Some(ModuleFormat::Esm), false);

        assert!(!allowed);
    }
}
