use std::sync::Arc;

use destack_artifact::{
    DirBound, DirChecked, DirExpanded, DirExported, DirImported, DirParsed, GlobalEnvironment,
};
use destack_dir as dir;
use destack_dir::{LanguageItem, StringId, StringPool};
use destack_source::{EditBuilder, File, FileId, ModuleId, PackageId, Span};
use destack_workspace::{
    ArtifactCache, LintSeverity, LinterOptions, Module, Package, Profile, ProfileId, Repository,
    Revision,
};

use crate::linter::library::is_library_module;
use crate::rules::common::expression_path_segments;
use crate::{
    ConstValue, LintDirAnalysisCache, LintMeta, LintRegexParse, LintReport, LintRequirement,
    find_control_character, find_control_characters, find_misleading_character_class,
    find_useless_backreference,
};

/// Severity override from a `@allow`/`@warn`/`@deny`/`@forbid` decorator.
#[derive(Debug, Clone, Copy)]
struct LintSeverityOverride {
    severity: LintSeverity,
    /// `@forbid` prevents inner scopes from overriding.
    is_forbidden: bool,
}

/// A compiler-recognized lint control directive.
#[derive(Debug, Clone, Copy)]
pub(crate) enum LintDirective {
    /// Disable the lint.
    Allow,
    /// Report the lint as a warning.
    Warn,
    /// Report the lint as an error.
    Deny,
    /// Report the lint as an error and prevent inner overrides.
    Forbid,
}

impl LintDirective {
    /// Return the effective severity declared by this directive.
    fn severity(self) -> LintSeverity {
        match self {
            Self::Allow => LintSeverity::Off,
            Self::Warn => LintSeverity::Warning,
            Self::Deny | Self::Forbid => LintSeverity::Error,
        }
    }

    /// Return whether this directive prevents inner overrides.
    fn is_forbidden(self) -> bool {
        matches!(self, Self::Forbid)
    }
}

/// Unwrap transparent expression nodes.
fn expression_unwrap_transparent(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> dir::LocalNodeId<dir::Expression> {
    match tree.get(expression_id) {
        dir::Expression::Parenthesized { expression } => {
            expression_unwrap_transparent(tree, *expression)
        }
        _ => expression_id,
    }
}

/// The source shape of a decorator expression in the DIR.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DecoratorCall<'a> {
    /// The decorator callee expression.
    pub(crate) callee: dir::LocalNodeId<dir::Expression>,
    /// The decorator arguments when the expression is a call.
    pub(crate) arguments: Option<&'a [dir::LocalNodeId<dir::Argument>]>,
}

/// Context for DIR-level linting of a single module.
pub struct LintModuleContext<'a> {
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

    /// The active DIR view.
    pub dir: dir::View<'a>,
    /// The DIR string pool.
    pub strings: &'a StringPool,
    /// The symbol table.
    pub symbols: dir::BindingTable,
    /// The type table.
    pub types: &'a dir::TypeTable,
    /// The top-level expressions of the Module.
    pub roots: Vec<dir::LocalNodeId<dir::Expression>>,

    /// The scope of the Module.
    pub namespace_scope: dir::LocalScopeId,

    /// Linter configuration.
    pub options: &'a LinterOptions,

    /// Whether to compute fixes for diagnostics.
    pub compute_fixes: bool,

    /// Cached DIR analysis results.
    dir_analysis: LintDirAnalysisCache,

    /// Collected diagnostics.
    diagnostics: Vec<LintReport>,
}

impl<'a> std::fmt::Debug for LintModuleContext<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LintModuleContext")
            .field("module_id", &self.module.id)
            .finish()
    }
}

#[allow(clippy::too_many_arguments)]
impl<'a> LintModuleContext<'a> {
    /// Create a new DIR lint context for a module.
    pub fn new(
        repository: Arc<Repository>,
        artifacts: Arc<ArtifactCache>,
        module: &'a Module,
        revision: Revision,
        profile: Profile,
        file: Arc<File>,
        parsed: &'a DirParsed,
        expanded: &'a DirExpanded,
        strings: &'a StringPool,
        symbols: dir::BindingTable,
        types: &'a dir::TypeTable,
        namespace_scope: dir::LocalScopeId,
        options: &'a LinterOptions,
        compute_fixes: bool,
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
            dir: dir::View::with_patches(&parsed.tree, std::slice::from_ref(&expanded.patch)),
            strings,
            symbols,
            types,
            roots: expanded.roots.clone(),
            namespace_scope,
            options,
            compute_fixes,
            dir_analysis: LintDirAnalysisCache::default(),
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

    /// Return one checked DIR artifact for one revision-scoped module.
    pub fn dir_checked(&self, module_id: ModuleId) -> Option<Arc<DirChecked>> {
        self.artifacts.dir_checked(module_id, self.profile_id)
    }

    /// Return one checked type table for one revision-scoped module.
    pub fn dir_type_table(&self, module_id: ModuleId) -> Option<dir::TypeTable> {
        let bound = self.dir_bound(module_id)?;
        let expanded = self.dir_expanded(module_id)?;
        let checked = self.dir_checked(module_id)?;

        Some(checked.type_table(&bound, &expanded))
    }

    /// Return one parsed DIR artifact for one revision-scoped module.
    pub fn dir_parsed(&self, module_id: ModuleId) -> Option<Arc<DirParsed>> {
        self.artifacts.dir_parsed(module_id)
    }

    /// Return one bound DIR artifact for one revision-scoped module.
    pub fn dir_bound(&self, module_id: ModuleId) -> Option<Arc<DirBound>> {
        self.artifacts.dir_bound(module_id, self.profile_id)
    }

    /// Return one expanded DIR artifact for one revision-scoped module.
    pub fn dir_expanded(&self, module_id: ModuleId) -> Option<Arc<DirExpanded>> {
        self.artifacts.dir_expanded(module_id, self.profile_id)
    }

    /// Return the local symbol declared by one DIR node.
    pub fn local_symbol_for_node<T: dir::Node>(
        &self,
        node_id: dir::LocalNodeId<T>,
    ) -> Option<dir::LocalSymbolId> {
        self.symbols
            .symbol_for_declaration(node_id.into_global_any(self.module.id))
    }

    /// Return the global symbol declared by one DIR node.
    pub fn symbol_for_node<T: dir::Node>(
        &self,
        node_id: dir::LocalNodeId<T>,
    ) -> Option<dir::GlobalSymbolId> {
        self.local_symbol_for_node(node_id)
            .map(|symbol_id| symbol_id.into_global(self.module.id))
    }

    /// Return the scope attached to one DIR node.
    pub fn scope_for_node<T: dir::Node>(
        &self,
        node_id: dir::LocalNodeId<T>,
    ) -> Option<dir::LocalScope> {
        self.symbols
            .scope_for_node(node_id.into_global_any(self.module.id))
    }

    /// Return one imported DIR artifact for one revision-scoped module.
    pub fn dir_imported(&self, module_id: ModuleId) -> Option<Arc<DirImported>> {
        self.artifacts.dir_imported(module_id, self.profile_id)
    }

    /// Return one exported DIR artifact for one revision-scoped module.
    pub fn dir_exported(&self, module_id: ModuleId) -> Option<Arc<DirExported>> {
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
        let expression = self.dir.get(expression_id);
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
        if !self.dir.is_visible(expression_id.into_any()) {
            return None;
        }

        let expression_id = expression_unwrap_transparent(self.dir.tree(), expression_id);
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

        self.dir.is_visible(declaration.local_id)
    }

    /// Return the source DIR node id for one DIR node.
    pub fn source_node_id<T: dir::Node>(
        &self,
        node_id: dir::LocalNodeIdAny,
    ) -> Option<dir::LocalNodeId<T>> {
        let source_id = self.dir.get_source_any(node_id);
        if self.dir.tree().get_node_type(source_id) != T::TYPE {
            return None;
        }

        Some(dir::LocalNodeId::<T>::new(source_id))
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

        environment.language.symbols.get(&name).copied()
    }

    /// Get a declared library symbol from the cache, panicking if not found.
    pub fn declared_library_symbol(&self, name: StringId) -> dir::GlobalSymbolId {
        self.get_declared_library_symbol(name)
            .unwrap_or_else(|| panic!("declared library symbol {name} not available"))
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
            LintRequirement::RequireLanguageItem(symbol) => {
                self.get_language_item(*symbol).is_some()
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

    /// Get effective severity for a rule at a specific active DIR node.
    ///
    /// Checks for `@allow`/`@deny`/`@warn`/`@forbid` source directives on the
    /// node and its ancestors, returning the effective severity at that location.
    /// Rules should call this before reporting to respect per-node suppressions.
    pub fn get_effective_severity<T: dir::Node>(
        &self,
        meta: &LintMeta,
        node_id: dir::LocalNodeId<T>,
    ) -> LintSeverity {
        let source_id = self.dir.get_source_any(node_id.into_any());

        self.get_effective_severity_at_source_node(meta, source_id)
    }

    /// Get effective severity for one source node id.
    fn get_effective_severity_at_source_node(&self, meta: &LintMeta, node_id: u32) -> LintSeverity {
        // walk up source parent chain
        let mut overrides: Vec<LintSeverityOverride> = Vec::new();
        let mut current = Some(node_id);

        while let Some(id) = current {
            let Some(node_id) = self.dir.get_node_id_by_source_id(id) else {
                break;
            };
            for decorator_id in self.dir.get_decorators_any(node_id) {
                if let Some(item) = self.lint_directive_severity_override(decorator_id, meta) {
                    overrides.push(item);
                }
            }
            current = self.dir.get_parent_id(id);
        }

        // apply outer decorators before inner decorators
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

    /// Resolve source decorator call information for a decorator node.
    pub(crate) fn decorator_call(
        &self,
        decorator_id: dir::LocalNodeId<dir::Decorator>,
    ) -> DecoratorCall<'_> {
        let decorator = self.dir.get(decorator_id);
        let expression_id = expression_unwrap_transparent(self.dir.tree(), decorator.expression);
        match self.dir.get(expression_id) {
            dir::Expression::Call {
                left, arguments, ..
            } => DecoratorCall {
                callee: expression_unwrap_transparent(self.dir.tree(), *left),
                arguments: Some(arguments.as_slice()),
            },
            _ => DecoratorCall {
                callee: expression_id,
                arguments: None,
            },
        }
    }

    /// Resolve the source decorator path when the decorator is reference-like.
    pub(crate) fn decorator_path(
        &self,
        decorator_id: dir::LocalNodeId<dir::Decorator>,
    ) -> Option<Vec<dir::StringId>> {
        let call = self.decorator_call(decorator_id);
        expression_path_segments(self.dir.tree(), call.callee)
    }

    /// Resolve the source decorator name as a dot separated string.
    pub(crate) fn decorator_name(
        &self,
        decorator_id: dir::LocalNodeId<dir::Decorator>,
    ) -> Option<String> {
        let path = self.decorator_path(decorator_id)?;
        if path.is_empty() {
            return None;
        }

        let mut segments = Vec::new();
        for segment in path {
            segments.push(self.strings.get(segment).to_string());
        }

        Some(segments.join("."))
    }

    /// Return the compiler-recognized lint directive represented by one decorator.
    pub(crate) fn lint_directive_for_decorator(
        &self,
        decorator_id: dir::LocalNodeId<dir::Decorator>,
    ) -> Option<LintDirective> {
        let call = self.decorator_call(decorator_id);

        self.lint_directive_for_callee(call.callee)
    }

    /// Return the severity override declared by one lint directive.
    fn lint_directive_severity_override(
        &self,
        decorator_id: dir::LocalNodeId<dir::Decorator>,
        meta: &LintMeta,
    ) -> Option<LintSeverityOverride> {
        // resolve the decorator directive
        let directive = self.lint_directive_for_decorator(decorator_id)?;

        // read the lint id or code argument
        let call = self.decorator_call(decorator_id);
        let arguments = call.arguments?;
        let first_argument = self.dir.get(*arguments.first()?);
        let dir::Argument::Positional { value, .. } = first_argument else {
            return None;
        };

        let argument_expression = self.dir.get(*value);
        let dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(string_id)) =
            argument_expression
        else {
            return None;
        };

        let specifier = self.strings.get(*string_id);
        Self::severity_override_for_lint_specifier(directive, specifier, meta)
    }

    /// Return the lint directive resolved by one decorator callee.
    fn lint_directive_for_callee(
        &self,
        callee_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<LintDirective> {
        let global_callee_id = callee_id.into_global_any(self.module.id);
        let symbol_id = self.types.symbol_resolution(global_callee_id)?;

        self.lint_directive_for_symbol(symbol_id)
    }

    /// Return the lint directive represented by one language item symbol.
    fn lint_directive_for_symbol(&self, symbol_id: dir::GlobalSymbolId) -> Option<LintDirective> {
        let environment = self.global_environment()?;

        if environment.language.item(LanguageItem::Allow) == Some(symbol_id) {
            return Some(LintDirective::Allow);
        }

        if environment.language.item(LanguageItem::Warn) == Some(symbol_id) {
            return Some(LintDirective::Warn);
        }

        if environment.language.item(LanguageItem::Deny) == Some(symbol_id) {
            return Some(LintDirective::Deny);
        }

        if environment.language.item(LanguageItem::Forbid) == Some(symbol_id) {
            return Some(LintDirective::Forbid);
        }

        None
    }

    /// Return the severity override for one lint specifier.
    fn severity_override_for_lint_specifier(
        directive: LintDirective,
        specifier: &str,
        meta: &LintMeta,
    ) -> Option<LintSeverityOverride> {
        if specifier == meta.id || specifier == meta.code {
            Some(LintSeverityOverride {
                severity: directive.severity(),
                is_forbidden: directive.is_forbidden(),
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

    /// Return the source span for one DIR node.
    pub fn get_span<T: dir::Node>(&self, id: dir::LocalNodeId<T>) -> Span {
        let source_node_id = self.dir.get_source(id);
        self.dir
            .get_span_by_id(source_node_id)
            .expect("lint DIR source view requires source span")
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
        self.dir_analysis.const_value(self.dir.tree(), id)
    }

    /// Return a constant boolean value if the expression can be evaluated.
    pub fn const_bool(&mut self, id: dir::LocalNodeId<dir::Expression>) -> Option<bool> {
        self.const_value(id).map(ConstValue::to_bool)
    }

    /// Return cached regex parse info for a pattern string.
    pub fn regex_parse(&mut self, id: dir::StringId) -> LintRegexParse {
        self.dir_analysis.regex_parse(self.strings, id)
    }

    /// Return cached regex parse info for a pattern and optional flags.
    pub fn regex_parse_with_flags(
        &mut self,
        pattern_id: dir::StringId,
        flags_id: Option<dir::StringId>,
    ) -> LintRegexParse {
        self.dir_analysis
            .regex_parse_with_flags(self.strings, pattern_id, flags_id)
    }

    /// Return a control character found in the pattern string.
    pub fn regex_control_character(&self, id: dir::StringId) -> Option<char> {
        let pattern = self.strings.get(id);

        find_control_character(pattern)
    }

    /// Return control characters found in the pattern string.
    pub fn regex_control_characters(
        &self,
        pattern_id: dir::StringId,
        flags_id: Option<dir::StringId>,
    ) -> Vec<String> {
        let pattern = self.strings.get(pattern_id);
        let flags = flags_id.map(|id| self.strings.get(id));

        find_control_characters(pattern, flags)
    }

    /// Return a misleading character class description for the pattern string.
    pub fn regex_misleading_character_class(&self, id: dir::StringId) -> Option<&'static str> {
        let pattern = self.strings.get(id);

        find_misleading_character_class(pattern)
    }

    /// Return a useless backreference description for the pattern string.
    pub fn regex_useless_backreference(
        &mut self,
        pattern_id: dir::StringId,
        flags_id: Option<dir::StringId>,
    ) -> Option<String> {
        let parse = self.regex_parse_with_flags(pattern_id, flags_id);
        let error_kind = parse.error.as_ref().map(|error| &error.kind);
        let pattern = self.strings.get(pattern_id);
        let flags = flags_id.map(|id| self.strings.get(id));

        find_useless_backreference(pattern, flags, error_kind)
    }
}
