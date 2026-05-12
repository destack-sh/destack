use std::sync::Arc;

use destack_artifact::{
    Ast, DirChecked, DirDeclared, DirExpanded, DirExported, DirImported, GlobalEnvironment,
    WellKnownSymbols,
};
use destack_ast::{StringId, StringPool};
use destack_dir::{LanguageItem, WellKnownSymbol};
use destack_source::{EditBuilder, File, FileId, ModuleId, PackageId, Span};
use destack_workspace::{
    ArtifactCache, LintSeverity, LinterOptions, Module, Package, Profile, ProfileId, Repository,
    Revision,
};
use {destack_ast as ast, destack_dir as dir};

use crate::linter::library::is_library_module;
use crate::rules::common::expression_unwrap_transparent;
use crate::{ConstValue, LintDirAnalysisCache, LintMeta, LintReport, LintRequirement};

/// Severity override from a `@allow`/`@warn`/`@deny`/`@forbid` decorator.
#[derive(Debug, Clone, Copy)]
struct LintSeverityOverride {
    severity: LintSeverity,
    /// `@forbid` prevents inner scopes from overriding.
    is_forbidden: bool,
}

/// The parsed shape of a decorator expression in the DIR.
#[derive(Debug, Clone, Copy)]
struct DecoratorCall<'a> {
    /// The decorator callee expression.
    callee: dir::LocalNodeId<dir::Expression>,
    /// The decorator arguments when the expression is a call.
    arguments: Option<&'a [dir::LocalNodeId<dir::Argument>]>,
}

/// Context for DIR-level linting of a single module.
pub struct LintModuleDirContext<'a> {
    /// The repository containing this module.
    pub repository: Arc<Repository>,
    /// Revision artifact cache for this lint pass.
    pub artifacts: Arc<ArtifactCache>,
    /// The module being linted.
    pub module: &'a Module,
    /// The source revision for this lint pass.
    pub revision: Revision,
    /// The semantic profile for this module.
    pub profile: Profile,
    /// The profile id for this module.
    pub profile_id: ProfileId,
    /// The source file.
    pub file: Arc<File>,

    /// The AST tree.
    pub ast: &'a ast::Tree,
    /// The DIR tree.
    pub tree: &'a dir::Tree,
    /// The visible DIR tree view.
    pub view: dir::View<'a>,
    /// The DIR string pool.
    pub strings: &'a StringPool,
    /// The symbol table.
    pub symbols: &'a dir::BindingTable,
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

    /// Linter configuration.
    pub options: &'a LinterOptions,

    /// Whether to compute fixes for diagnostics.
    pub include_fixes: bool,

    /// Cached analysis results.
    analysis: LintDirAnalysisCache,

    /// Collected diagnostics.
    diagnostics: Vec<LintReport>,
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
        repository: Arc<Repository>,
        artifacts: Arc<ArtifactCache>,
        module: &'a Module,
        revision: Revision,
        profile: Profile,
        file: Arc<File>,
        ast: &'a ast::Tree,
        declared: &'a DirDeclared,
        expanded: &'a DirExpanded,
        strings: &'a StringPool,
        symbols: &'a dir::BindingTable,
        types: &'a dir::TypeTable,
        roots: Vec<dir::LocalNodeId<dir::Expression>>,
        namespace_symbol: dir::LocalSymbolId,
        namespace_scope: dir::LocalScopeId,
        default_symbol: dir::LocalSymbolId,
        options: &'a LinterOptions,
        include_fixes: bool,
    ) -> Self {
        let profile_id = profile.id();

        Self {
            repository,
            artifacts,
            module,
            revision,
            profile,
            profile_id,
            file,
            ast,
            tree: &declared.tree,
            view: dir::View::with_patches(&declared.tree, std::slice::from_ref(&expanded.patch)),
            strings,
            symbols,
            types,
            roots,
            namespace_symbol,
            namespace_scope,
            default_symbol,
            options,
            include_fixes,
            analysis: LintDirAnalysisCache::default(),
            diagnostics: Vec::new(),
        }
    }

    /// Return the file id.
    pub fn file_id(&self) -> FileId {
        self.module.file_id
    }

    /// Return the module id.
    pub fn module_id(&self) -> ModuleId {
        self.module.id
    }

    /// Return the stable string id for one static text.
    pub fn string_id(&self, text: &str) -> StringId {
        StringId::for_text(text)
    }

    /// Return one module for the active revision when present.
    pub fn repository_module(&self, module_id: ModuleId) -> Option<Arc<Module>> {
        self.repository
            .module(self.revision, module_id)
            .ok()
            .flatten()
    }

    /// Return one package for the active revision when present.
    pub fn repository_package(&self, package_id: PackageId) -> Option<Arc<Package>> {
        self.repository
            .package(self.revision, package_id)
            .ok()
            .flatten()
    }

    /// Return one source file for the active revision when present.
    pub fn repository_file(&self, file_id: FileId) -> Option<Arc<File>> {
        self.repository.file(self.revision, file_id).ok().flatten()
    }

    /// Return one AST artifact for one revision-scoped module.
    pub fn module_ast(&self, module_id: ModuleId) -> Option<Arc<Ast>> {
        self.artifacts.ast(module_id)
    }

    /// Return one checked DIR artifact for one revision-scoped module.
    pub fn checked_dir(&self, module_id: ModuleId) -> Option<Arc<DirChecked>> {
        self.artifacts.dir_checked(module_id, self.profile_id)
    }

    /// Return one declared DIR artifact for one revision-scoped module.
    pub fn declared_dir(&self, module_id: ModuleId) -> Option<Arc<DirDeclared>> {
        self.artifacts.dir_declared(module_id, self.profile_id)
    }

    /// Return one imported DIR artifact for one revision-scoped module.
    pub fn imported_dir(&self, module_id: ModuleId) -> Option<Arc<DirImported>> {
        self.artifacts.dir_imported(module_id, self.profile_id)
    }

    /// Return one exported DIR artifact for one revision-scoped module.
    pub fn exported_dir(&self, module_id: ModuleId) -> Option<Arc<DirExported>> {
        self.artifacts.dir_exported(module_id, self.profile_id)
    }

    /// Return the global environment for the active revision and profile.
    pub fn global_environment(&self) -> Option<Arc<GlobalEnvironment>> {
        self.artifacts.global_environment(self.profile_id)
    }

    /// Return all visible module ids for the active revision.
    pub fn workspace_module_ids(&self) -> Vec<ModuleId> {
        self.repository
            .module_ids(self.revision)
            .unwrap_or_default()
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

        // then use value types for direct references
        self.expression_target_symbol(expression_id)
            .and_then(|symbol| self.types.get_value_type_id(symbol))
    }

    /// Resolve the lexical target symbol for one expression.
    pub fn expression_target_symbol(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::GlobalSymbolId> {
        if !self.view.is_active(expression_id.into_any()) {
            return None;
        }

        let expression_id = expression_unwrap_transparent(self.tree, expression_id);
        let global_id = expression_id.into_global_any(self.module.id);
        let symbol = self.types.symbol_resolution(global_id)?;
        if !self.symbol_is_active(symbol.local_id) {
            return None;
        }

        Some(symbol)
    }

    /// Return whether one local DIR symbol is visible in this lint view.
    fn symbol_is_active(&self, symbol_id: dir::LocalSymbolId) -> bool {
        let symbol = self.symbols.get_symbol(symbol_id);
        let Some(declaration) = symbol.declaration else {
            return true;
        };

        self.view.is_active(declaration.local_id)
    }

    /// Return the source AST node id for one DIR node.
    pub fn source_node_id<T: ast::Node>(
        &self,
        node_id: dir::LocalNodeIdAny,
    ) -> Option<ast::LocalNodeId<T>> {
        let source_id = self.view.get_source_any(node_id);
        if self.ast.get_node_type(source_id) != T::TYPE {
            return None;
        }

        Some(ast::LocalNodeId::<T>::new(source_id))
    }

    /// Get a language item from the cache, returning None if not found.
    pub fn get_language_item(&self, item: LanguageItem) -> Option<dir::GlobalSymbolId> {
        let environment = self.global_environment()?;
        environment.language.item(item)
    }

    /// Get a language item from the cache, panicking if not found.
    pub fn language_item(&self, item: LanguageItem) -> dir::GlobalSymbolId {
        self.get_language_item(item)
            .unwrap_or_else(|| panic!("language item {item:?} not available"))
    }

    /// Get a cached declared library symbol for the module profile and name.
    pub fn get_declared_library_symbol(&self, name: StringId) -> Option<dir::GlobalSymbolId> {
        let environment = self.global_environment()?;
        let key = dir::StaticKey::Name(name);

        environment.symbol_from_key(key, dir::SymbolSpace::Value)
    }

    /// Get a declared library symbol from the cache, panicking if not found.
    pub fn declared_library_symbol(&self, name: StringId) -> dir::GlobalSymbolId {
        self.get_declared_library_symbol(name)
            .unwrap_or_else(|| panic!("declared library symbol {name} not available"))
    }

    /// Get well-known symbols for the module profile.
    pub fn get_well_known_symbols(&self) -> Option<WellKnownSymbols> {
        let environment = self.global_environment()?;
        Some(environment.well_known_symbols())
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
        self.options.resolve_severity(
            meta.id,
            meta.category,
            meta.category.default_severity(),
            meta.is_recommended(),
            meta.is_strict(),
        )
    }

    /// Check if a requirement is met.
    pub fn is_requirement_met(&self, requirement: &LintRequirement) -> bool {
        match requirement {
            LintRequirement::RequireLibSymbol(name, libs) => {
                if !self.is_lib_available(libs) {
                    return false;
                }
                let name = self.string_id(name);
                self.get_declared_library_symbol(name).is_some()
            }
            LintRequirement::RequireWellKnownSymbol(symbol) => {
                self.get_well_known_symbol(*symbol).is_some()
            }
        }
    }

    /// Return true when at least one of the required libs is available.
    fn is_lib_available(&self, libs: &[&str]) -> bool {
        if libs.is_empty() {
            return true;
        }
        let Some(environment) = self.global_environment() else {
            return false;
        };

        environment
            .modules
            .iter()
            .filter_map(|module_id| self.repository_module(*module_id))
            .any(|module| is_library_module(module.as_ref(), libs))
    }

    /// Check if a rule is supported.
    pub fn is_rule_supported(&self, meta: &LintMeta) -> bool {
        if !self.options.include_declaration_files && !meta.supports_file_type(self.file.ty) {
            return false;
        }

        // requires all
        if !meta.requires_all.is_empty() {
            for requirement in meta.requires_all {
                if !self.is_requirement_met(requirement) {
                    return false;
                }
            }
        }
        // requires any
        if !meta.requires_any.is_empty() {
            for requirement in meta.requires_any {
                if self.is_requirement_met(requirement) {
                    return true;
                }
            }

            return false;
        }

        true
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
        let mut current = Some(node_id.into_any());

        while let Some(id) = current {
            for decorator_id in self.view.get_decorators_any(id) {
                if let Some(item) = self.eat_decorator(decorator_id, meta) {
                    overrides.push(item);
                }
            }
            current = self.view.get_parent_any(id);
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

    /// Unwrap parenthesized decorator expressions.
    fn unwrap_decorator_expression(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> dir::LocalNodeId<dir::Expression> {
        let mut current = expression_id;
        loop {
            let expression = self.view.get(current);
            let dir::Expression::Parenthesized { expression } = expression else {
                return current;
            };
            current = *expression;
        }
    }

    /// Resolve decorator call information for an annotation.
    fn decorator_call(
        &self,
        decorator_id: dir::LocalNodeId<dir::Decorator>,
    ) -> Option<DecoratorCall<'_>> {
        let decorator = self.view.get(decorator_id);
        let expression_id = self.unwrap_decorator_expression(decorator.expression);
        match self.view.get(expression_id) {
            dir::Expression::Call {
                left, arguments, ..
            } => Some(DecoratorCall {
                callee: self.unwrap_decorator_expression(*left),
                arguments: Some(arguments.as_slice()),
            }),
            _ => Some(DecoratorCall {
                callee: expression_id,
                arguments: None,
            }),
        }
    }

    /// Parse a decorator annotation and return the severity override if it matches this lint.
    fn eat_decorator(
        &self,
        decorator_id: dir::LocalNodeId<dir::Decorator>,
        meta: &LintMeta,
    ) -> Option<LintSeverityOverride> {
        let call = self.decorator_call(decorator_id)?;

        // extract path from the decorator expression
        let callee_expression = self.view.get(call.callee);
        let path = match callee_expression {
            dir::Expression::Path { path, .. } => path,
            _ => return None,
        };

        // check decorator name (must be single segment: allow, warn, deny, forbid)
        if path.segments.len() != 1 {
            return None;
        }

        let name = self.strings.get(path.segments[0]);
        let (severity, is_forbidden) = match name {
            "allow" => (LintSeverity::Off, false),
            "warn" => (LintSeverity::Warning, false),
            "deny" => (LintSeverity::Error, false),
            "forbid" => (LintSeverity::Error, true),
            _ => return None,
        };

        // extract the string argument (lint ID or code)
        let arguments = call.arguments?;
        let first_argument = self.view.get(*arguments.first()?);
        let dir::Argument::Positional { value, .. } = first_argument else {
            return None;
        };

        let argument_expression = self.view.get(*value);
        let dir::Expression::ScalarLiteral {
            value: dir::ScalarLiteral::String(string_id),
        } = argument_expression
        else {
            return None;
        };

        // match against lint ID or code
        let specifier = self.strings.get(*string_id);
        if specifier == meta.id || specifier == meta.code {
            Some(LintSeverityOverride {
                severity,
                is_forbidden,
            })
        } else {
            None
        }
    }

    /// Report a lint diagnostic.
    pub fn report(&mut self, diagnostic: LintReport) {
        if diagnostic.is_enabled() {
            self.diagnostics.push(diagnostic);
        }
    }

    /// Take the collected diagnostics.
    pub fn take_diagnostics(&mut self) -> Vec<LintReport> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Return a reference to collected diagnostics.
    pub fn diagnostics(&self) -> &[LintReport] {
        &self.diagnostics
    }

    /// Return the source span for a DIR node by looking up its AST source node.
    pub fn get_span<T: dir::Node>(&self, id: dir::LocalNodeId<T>) -> Span {
        let ast_node_id = self.view.get_source(id);
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
    use crate::linter::TestProgram;
    use crate::rules::correctness::{NO_SELF_COMPARE, NoSelfCompare};

    #[test]
    fn test_allow_suppresses_by_id() {
        let test = TestProgram::for_rule_with_prelude(NoSelfCompare);
        let result = test.lint_dir(
            "dir/test_allow_suppresses_by_id.ds",
            r#"
@allow("no-self-compare")
function foo() {
    let x = 1
    x == x
}
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-self-compare");
    }

    #[test]
    fn test_allow_suppresses_by_code() {
        let test = TestProgram::for_rule_with_prelude(NoSelfCompare);
        let source = format!(
            r#"
@allow("{code}")
function foo() {{
    let x = 1
    x == x
}}
"#,
            code = NO_SELF_COMPARE.code
        );
        let result = test.lint_dir("dir/test_allow_suppresses_by_code.ds", source.as_str());
        test.check_clean();
        test.result(result).assert_no_lint("no-self-compare");
    }

    #[test]
    fn test_allow_does_not_affect_other_lints() {
        let test = TestProgram::for_rule_with_prelude(NoSelfCompare);
        let result = test.lint_dir(
            "dir/test_allow_does_not_affect_other_lints.ds",
            r#"
@allow("some-other-lint")
function foo() {
    let x = 1
    x == x
}
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-self-compare");
    }

    #[test]
    fn test_forbid_prevents_inner_allow() {
        let test = TestProgram::for_rule_with_prelude(NoSelfCompare);
        let result = test.lint_dir(
            "dir/test_forbid_prevents_inner_allow.ds",
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
        );
        test.check_clean();
        // inner @allow should be ignored due to outer @forbid
        test.result(result).assert_lint("no-self-compare");
    }

    #[test]
    fn test_warn_changes_severity() {
        let test = TestProgram::for_rule_with_prelude(NoSelfCompare);
        let result = test.lint_dir(
            "dir/test_warn_changes_severity.ds",
            r#"
@warn("no-self-compare")
function foo() {
    let x = 1
    x == x
}
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-self-compare");
    }

    #[test]
    fn test_deny_changes_severity() {
        let test = TestProgram::for_rule_with_prelude(NoSelfCompare);
        let result = test.lint_dir(
            "dir/test_deny_changes_severity.ds",
            r#"
@deny("no-self-compare")
function foo() {
    let x = 1
    x == x
}
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-self-compare");
    }
}
