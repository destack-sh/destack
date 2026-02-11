use destack_dir::DependencyKind;
use destack_resolver::ResolveOptions;
use destack_source::LanguageType;
use destack_workspace::ImportEdgeKind;
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
    /// Dependency kind for the import edge.
    pub dependency_kind: DependencyKind,
    /// Source language of the importing module.
    pub source_language_type: Option<LanguageType>,
    /// Edge semantics for this dependency.
    pub edge_kind: ImportEdgeKind,
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
        self.edge_kind == ImportEdgeKind::Require
    }
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
    use super::{ImportResolveContext, materialize_import_resolve_options};
    use destack_dir::DependencyKind;
    use destack_resolver::ResolveOptions;
    use destack_source::LanguageType;
    use destack_workspace::ImportEdgeKind;

    /// Build default import options for value dependencies.
    #[test]
    fn test_materialize_import_options_value_dependency_defaults() {
        let context = ImportResolveContext {
            dependency_kind: DependencyKind::Value,
            source_language_type: None,
            edge_kind: ImportEdgeKind::Import,
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
            edge_kind: ImportEdgeKind::Import,
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
            edge_kind: ImportEdgeKind::Import,
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
            edge_kind: ImportEdgeKind::Import,
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
            edge_kind: ImportEdgeKind::Require,
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
            edge_kind: ImportEdgeKind::Import,
        };

        let options = materialize_import_resolve_options(&base, context);

        assert_eq!(options.conditions, vec!["node", "import"]);
    }
}
