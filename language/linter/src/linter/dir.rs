use std::sync::Arc;

use destack_ast::StringId;
use destack_builtin::LanguageItem;
use destack_dir::WellKnownSymbol;
use destack_source::{EditBuilder, File, ModuleId, Span};
use destack_workspace::{
    LintSeverity, LinterOptions, Module, ProfileId, Program, WellKnownSymbols,
};
use indexmap::IndexMap;
use {destack_ast as ast, destack_dir as dir};

use crate::{ConstValue, LintDiagnostic, LintDirAnalysisCache, LintMeta};

/// Severity override from a `@allow`/`@warn`/`@deny`/`@forbid` decorator.
#[derive(Debug, Clone, Copy)]
struct LintSeverityOverride {
    severity: LintSeverity,
    /// `@forbid` prevents inner scopes from overriding.
    is_forbidden: bool,
}

/// Context for DIR-level linting of a single module. Unfurls ModuleDir.
pub struct LintModuleDirContext<'a> {
    /// The program containing this module.
    pub program: Arc<Program>,
    /// The module being linted.
    pub module: &'a Module,
    /// The profile for this module.
    pub profile_id: ProfileId,
    /// The source file.
    pub file: Arc<File>,

    /// The AST tree.
    pub ast: &'a ast::NodeTree,
    /// The DIR tree.
    pub tree: &'a dir::NodeTree,
    /// The symbol table.
    pub symbols: &'a dir::SymbolTable,
    /// The type table.
    pub types: &'a dir::TypeTable,
    /// The top-level expressions of the Module.
    pub roots: Vec<dir::LocalNodeId<dir::Expression>>,

    /// The symbol of the Module namespace.
    pub namespace_symbol: dir::LocalSymbolId,
    /// The scope of the Module.
    pub namespace_scope: dir::LocalScopeId,
    /// The symbol of the Module default.
    pub default_symbol: dir::LocalSymbolId,
    /// Namespace exports: modules whose exports are re-exported via `export * from "..."`.
    pub namespace_exports: Vec<ModuleId>,
    /// Resolved import specifiers to module ids (keyed by (relative_module, specifier)).
    pub imported_modules: IndexMap<(Option<ModuleId>, StringId), ModuleId>,
    /// Exported symbols by key (space, name).
    pub exported_symbols: IndexMap<(dir::SymbolSpace, dir::StaticKey), dir::LocalSymbolId>,

    /// Linter configuration.
    pub options: &'a LinterOptions,

    /// Whether to compute fixes for diagnostics.
    pub compute_fixes: bool,

    /// Cached analysis results.
    analysis: LintDirAnalysisCache,

    /// Collected diagnostics.
    diagnostics: Vec<LintDiagnostic>,
}

impl<'a> std::fmt::Debug for LintModuleDirContext<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LintModuleDirContext")
            .field("module_id", &self.module.id)
            .finish()
    }
}

#[allow(clippy::too_many_arguments)]
impl<'a> LintModuleDirContext<'a> {
    /// Create a new DIR lint context for a module.
    pub fn new(
        program: Arc<Program>,
        module: &'a Module,
        profile_id: ProfileId,
        file: Arc<File>,
        ast: &'a ast::NodeTree,
        tree: &'a dir::NodeTree,
        symbols: &'a dir::SymbolTable,
        types: &'a dir::TypeTable,
        roots: Vec<dir::LocalNodeId<dir::Expression>>,
        namespace_symbol: dir::LocalSymbolId,
        namespace_scope: dir::LocalScopeId,
        default_symbol: dir::LocalSymbolId,
        namespace_exports: Vec<ModuleId>,
        imported_modules: IndexMap<(Option<ModuleId>, StringId), ModuleId>,
        exported_symbols: IndexMap<(dir::SymbolSpace, dir::StaticKey), dir::LocalSymbolId>,
        options: &'a LinterOptions,
        compute_fixes: bool,
    ) -> Self {
        Self {
            program,
            module,
            profile_id,
            file,
            ast,
            tree,
            symbols,
            types,
            roots,
            namespace_symbol,
            namespace_scope,
            default_symbol,
            namespace_exports,
            imported_modules,
            exported_symbols,
            options,
            compute_fixes,
            analysis: LintDirAnalysisCache::default(),
            diagnostics: Vec::new(),
        }
    }

    /// Return the file id.
    pub fn file_id(&self) -> destack_source::FileId {
        self.module.file_id
    }

    /// Return the module id.
    pub fn module_id(&self) -> destack_source::ModuleId {
        self.module.id
    }

    /// Resolve the inferred type id for a DIR expression.
    pub fn expression_type_id(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalTypeId> {
        // unwrap parenthesized expressions first
        let expression = self.tree.get(expression_id);
        if let dir::Expression::Parenthesized { expression } = expression {
            return self.expression_type_id(*expression);
        }

        // build a global id for the expression
        let global_id = dir::GlobalNodeIdAny::new(self.module.id, expression_id.into_any());

        // fetch the inferred type id from the module type table
        if let Some(type_id) = self.types.get_inferred_type_id(global_id) {
            return Some(type_id);
        }

        // fall back to value types for direct references
        expression
            .target_symbol()
            .and_then(|symbol| self.types.get_value_type_id(symbol))
    }

    /// Get a language item from the cache, returning None if not found.
    pub fn get_language_item(&self, item: LanguageItem) -> Option<dir::GlobalSymbolId> {
        let builtins = self.program.builtins.as_ref()?;
        builtins.items.get(&item).map(|value| *value)
    }

    /// Get a language item from the cache, panicking if not found.
    pub fn language_item(&self, item: LanguageItem) -> dir::GlobalSymbolId {
        self.get_language_item(item)
            .unwrap_or_else(|| panic!("language item {item:?} not available"))
    }

    /// Get a cached lib symbol for the module profile and name.
    pub fn get_lib_item(&self, name: StringId) -> Option<dir::GlobalSymbolId> {
        let builtins = self.program.builtins.as_ref()?;
        builtins.lib_symbol(self.profile_id, name)
    }

    /// Get a lib symbol from the cache, panicking if not found.
    pub fn lib_item(&self, name: StringId) -> dir::GlobalSymbolId {
        self.get_lib_item(name).unwrap_or_else(|| {
            let name = self.program.strings.get(name);
            panic!("lib symbol '{}' not available", name.as_ref())
        })
    }

    /// Get well-known symbols for the module profile.
    pub fn get_well_known_symbols(&self) -> Option<WellKnownSymbols> {
        let builtins = self.program.builtins.as_ref()?;
        builtins.well_known_symbols(self.profile_id)
    }

    /// Get well-known symbols for the module profile, panicking if not found.
    pub fn well_known_symbols(&self) -> WellKnownSymbols {
        self.get_well_known_symbols().unwrap_or_else(|| {
            panic!(
                "well-known symbols not available for profile {:?}",
                self.profile_id
            )
        })
    }

    /// Get a specific well-known symbol for the module profile.
    pub fn get_well_known_symbol(&self, symbol: WellKnownSymbol) -> Option<dir::GlobalSymbolId> {
        let well_known_symbols = self.get_well_known_symbols()?;
        well_known_symbols.get_symbol(symbol)
    }

    /// Get a specific well-known symbol for the module profile, panicking if not found.
    pub fn well_known_symbol(&self, symbol: WellKnownSymbol) -> dir::GlobalSymbolId {
        self.get_well_known_symbol(symbol).unwrap_or_else(|| {
            panic!(
                "well-known symbol {symbol:?} not available for profile {:?}",
                self.profile_id
            )
        })
    }

    /// Resolve severity for a rule.
    pub fn get_severity(&self, meta: &LintMeta) -> LintSeverity {
        self.options
            .resolve_severity(meta.id, meta.category, meta.category.default_severity())
    }

    /// Check if a rule is enabled.
    pub fn is_rule_enabled(&self, meta: &LintMeta) -> bool {
        self.get_severity(meta).is_enabled()
    }

    /// Get effective severity for a rule at a specific node.
    ///
    /// Checks for `@allow`/`@deny`/`@warn`/`@forbid` decorators on the node
    /// and its ancestors, returning the effective severity at that location.
    /// Rules should call this before reporting to respect per-node suppressions.
    pub fn get_effective_severity<T: dir::Node>(
        &self,
        meta: &LintMeta,
        node_id: dir::LocalNodeId<T>,
    ) -> LintSeverity {
        // walk up parent chain, collecting decorator overrides (innermost first)
        let mut overrides: Vec<LintSeverityOverride> = Vec::new();
        let mut current = Some(node_id.id);

        while let Some(id) = current {
            for annotation_id in self.tree.get_annotations(id) {
                if let Some(item) = self.parse_decorator(annotation_id, meta) {
                    overrides.push(item);
                }
            }
            current = self.tree.get_parent_id(id);
        }

        // apply from outermost to innermost (reverse since we collected innermost first)
        let mut effective = self.get_severity(meta);
        let mut is_forbidden = false;

        for item in overrides.into_iter().rev() {
            if is_forbidden {
                continue;
            }
            effective = item.severity;
            is_forbidden = item.is_forbidden;
        }

        effective
    }

    /// Parse a decorator annotation and return the severity override if it matches this lint.
    fn parse_decorator(
        &self,
        annotation_id: dir::LocalNodeId<dir::Annotation>,
        meta: &LintMeta,
    ) -> Option<LintSeverityOverride> {
        let annotation = self.tree.get(annotation_id);
        let dir::Annotation::Decorator {
            left, arguments, ..
        } = annotation
        else {
            return None;
        };

        // extract path from the decorator expression
        let left_expression = self.tree.get(*left);
        let path = match left_expression {
            dir::Expression::LocalReference { path, .. }
            | dir::Expression::ModuleReference { path, .. }
            | dir::Expression::GlobalReference { path, .. }
            | dir::Expression::UnresolvedPath { path, .. } => path,
            _ => return None,
        };

        // check decorator name (must be single segment: allow, warn, deny, forbid)
        if path.segments.len() != 1 {
            return None;
        }

        let name = self.program.strings.get(path.segments[0]);
        let (severity, is_forbidden) = match name.as_ref() {
            "allow" => (LintSeverity::Off, false),
            "warn" => (LintSeverity::Warning, false),
            "deny" => (LintSeverity::Error, false),
            "forbid" => (LintSeverity::Error, true),
            _ => return None,
        };

        // extract the string argument (lint ID or code)
        let arguments = arguments.as_ref()?;
        let first_argument = self.tree.get(*arguments.first()?);
        let dir::Argument::Positional { value } = first_argument else {
            return None;
        };

        let argument_expression = self.tree.get(*value);
        let dir::Expression::ScalarLiteral {
            value: dir::ScalarLiteral::String(string_id),
        } = argument_expression
        else {
            return None;
        };

        // match against lint ID or code
        let specifier = self.program.strings.get(*string_id);
        if specifier.as_ref() == meta.id || specifier.as_ref() == meta.code {
            Some(LintSeverityOverride {
                severity,
                is_forbidden,
            })
        } else {
            None
        }
    }

    /// Report a lint diagnostic.
    pub fn report(&mut self, diagnostic: LintDiagnostic) {
        if diagnostic.is_enabled() {
            self.diagnostics.push(diagnostic);
        }
    }

    /// Take the collected diagnostics.
    pub fn take_diagnostics(&mut self) -> Vec<LintDiagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Return a reference to collected diagnostics.
    pub fn diagnostics(&self) -> &[LintDiagnostic] {
        &self.diagnostics
    }

    /// Return the source span for a DIR node by looking up its AST source node.
    pub fn get_span<T: dir::Node>(&self, id: dir::LocalNodeId<T>) -> Span {
        let ast_node_id = self.tree.get_source(id.id);
        self.ast.get_span_by_id(ast_node_id)
    }

    /// Get the full source text.
    pub fn source_text(&self) -> &str {
        self.file.text()
    }

    /// Get the source text for a span.
    pub fn get_span_text(&self, span: Span) -> &str {
        &self.file.text()[span.start as usize..span.end as usize]
    }

    /// Create an EditBuilder with source text for text-aware operations.
    pub fn edit_builder(&self) -> EditBuilder<'_> {
        EditBuilder::from_file(self.module.file_id, self.file.text())
    }

    /// Return a constant value if the expression can be evaluated.
    pub fn const_value(&mut self, id: dir::LocalNodeId<dir::Expression>) -> Option<ConstValue> {
        self.analysis.const_value(self.tree, id)
    }

    /// Return a constant boolean value if the expression can be evaluated.
    pub fn const_bool(&mut self, id: dir::LocalNodeId<dir::Expression>) -> Option<bool> {
        self.const_value(id).map(ConstValue::to_bool)
    }
}

#[cfg(test)]
mod tests {
    use crate::LintLevel;
    use crate::linter::TestProgram;
    use crate::rules::correctness::NoSelfCompare;

    #[test]
    fn test_allow_suppresses_by_id() {
        let test = TestProgram::for_rule_with_builtins(NoSelfCompare);
        let result = test.lint(
            "test.ds",
            r#"
@allow("no-self-compare")
function foo() {
    let x = 1
    x == x
}
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-self-compare");
    }

    #[test]
    fn test_allow_suppresses_by_code() {
        let test = TestProgram::for_rule_with_builtins(NoSelfCompare);
        let result = test.lint(
            "test.ds",
            r#"
@allow("LC038")
function foo() {
    let x = 1
    x == x
}
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-self-compare");
    }

    #[test]
    fn test_allow_does_not_affect_other_lints() {
        let test = TestProgram::for_rule_with_builtins(NoSelfCompare);
        let result = test.lint(
            "test.ds",
            r#"
@allow("some-other-lint")
function foo() {
    let x = 1
    x == x
}
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result).assert_lint("no-self-compare");
    }

    #[test]
    fn test_forbid_prevents_inner_allow() {
        let test = TestProgram::for_rule_with_builtins(NoSelfCompare);
        let result = test.lint(
            "test.ds",
            r#"
@forbid("no-self-compare")
function outer() {
    @allow("no-self-compare")
    function inner() {
        let x = 1
        x == x
    }
}
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        // inner @allow should be ignored due to outer @forbid
        test.result(result).assert_lint("no-self-compare");
    }

    #[test]
    fn test_warn_changes_severity() {
        let test = TestProgram::for_rule_with_builtins(NoSelfCompare);
        let result = test.lint(
            "test.ds",
            r#"
@warn("no-self-compare")
function foo() {
    let x = 1
    x == x
}
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result).assert_lint("no-self-compare");
    }

    #[test]
    fn test_deny_changes_severity() {
        let test = TestProgram::for_rule_with_builtins(NoSelfCompare);
        let result = test.lint(
            "test.ds",
            r#"
@deny("no-self-compare")
function foo() {
    let x = 1
    x == x
}
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result).assert_lint("no-self-compare");
    }
}
