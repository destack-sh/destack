use destack_artifact::PackageDependency;
use destack_dir as dir;
use destack_repository::RepositoryError;
use destack_source::{FileId, ModuleId, Span};
use rustc_hash::FxHashSet;

use crate::source::{ImportBinding, extract_string_literal_prefix};
use crate::{
    CompletionCandidate, CompletionCandidates, CompletionItemKind, CompletionOrigin,
    ExportDeclaration, ImportCandidate, ImportOrder, MatchOrder, MatchQuality, ModuleQueryContext,
    ProgramQueryContext, QueryError, QueryResult, SORT_BUILTIN, SORT_DEFAULT, SORT_LOCAL_SYMBOL,
    SymbolUse, match_quality,
};

use super::builtin::length_ordering_text;
use super::{CompletionCollector, CompletionContext};

// auto import completion thresholds
const AUTO_IMPORT_MIN_PREFIX: usize = 2;
const AUTO_IMPORT_SHORT_PREFIX_LIMIT: usize = 50;

impl ExportDeclaration {
    /// Return the exact completion kind for this exported declaration.
    fn completion_kind(self, program: &ProgramQueryContext<'_>) -> QueryResult<CompletionItemKind> {
        let kind = match self {
            Self::Symbol { symbol, .. } => {
                let module = program.module(symbol.module_id)?;
                let symbols = module.bindings()?;
                let symbol = symbols.get_symbol(symbol.local_id);

                symbol.into()
            }
            Self::Namespace { .. } => CompletionItemKind::Module,
        };

        Ok(kind)
    }
}

impl ModuleQueryContext<'_> {
    /// Classify import completion at one offset.
    pub(super) fn classify_import(
        &self,
        file_id: FileId,
        source: &str,
        offset: u32,
    ) -> QueryResult<Option<CompletionContext>> {
        // resolve enclosing spans from innermost to outermost
        let enclosing = self.enclosing_spans_at_cursor(file_id, offset)?;
        let view = self.view()?;

        // scan enclosing expressions for import nodes under the cursor
        for enclosing_span in &enclosing {
            let Some(node_id) = view.get_node_id_by_source_id(enclosing_span.source_id) else {
                continue;
            };
            if node_id.ty != dir::NodeType::Expression {
                continue;
            }
            let expression_id = dir::LocalNodeId::<dir::Expression>::new(node_id.id);
            let expression = view.get(expression_id);

            if !matches!(expression, dir::Expression::Import { .. }) {
                continue;
            }

            // detect path completions inside the import string
            let import_span = self.source_index()?.get(enclosing_span.source_id);
            let main_span = self.source_index()?.get_main(enclosing_span.source_id);
            if let Some(span) = main_span
                && span.contains(offset)
            {
                let partial_path = extract_string_literal_prefix(source, span, offset)?;
                return Ok(Some(CompletionContext::ImportPath { partial_path }));
            }

            if let Some(span) = self.import_path_token_span(file_id, import_span, offset)? {
                let partial_path = extract_string_literal_prefix(source, span, offset)?;
                return Ok(Some(CompletionContext::ImportPath { partial_path }));
            }

            // detect import clause completions inside the brace list
            if let Some(existing_names) =
                self.import_clause_names(expression, offset, import_span, main_span)?
            {
                let dir::Expression::Import { .. } = expression else {
                    continue;
                };

                let target_module = self.resolved_import_target_module(expression_id)?;

                return Ok(Some(CompletionContext::ImportClause {
                    target_module,
                    existing_names,
                    use_filter: None,
                }));
            }
        }

        Ok(None)
    }

    /// Resolve a string literal span for an import path at the cursor.
    fn import_path_token_span(
        &self,
        file_id: FileId,
        import_span: Span,
        offset: u32,
    ) -> QueryResult<Option<Span>> {
        // find the token under the cursor
        let Some(token) = self.token_span_at_offset(file_id, offset)? else {
            return Ok(None);
        };

        // require the token to stay inside the import statement span
        if token.span.start < import_span.start || token.span.end > import_span.end {
            return Ok(None);
        }

        // require a string literal token
        if token.token.ty() != dir::TokenType::Literal {
            return Ok(None);
        }

        if !matches!(
            token.token.literal(),
            Some(dir::TokenLiteral::String { .. })
        ) {
            return Ok(None);
        }

        Ok(Some(token.span))
    }

    /// Resolve one import target module from compiler resolved import edges.
    fn resolved_import_target_module(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> QueryResult<Option<ModuleId>> {
        let source = expression_id.into_global_any(self.module_id());

        Ok(self
            .modules()?
            .target_for_source(source, dir::ModuleRelation::Import))
    }
}

impl ModuleQueryContext<'_> {
    /// Return existing names when the cursor belongs to an import clause.
    fn import_clause_names(
        &self,
        expression: &dir::Expression,
        offset: u32,
        import_span: Span,
        target_span: Option<Span>,
    ) -> QueryResult<Option<Vec<String>>> {
        // require an import expression
        let dir::Expression::Import { items, .. } = expression else {
            return Ok(None);
        };
        let Some(items) = items.as_ref() else {
            return Ok(None);
        };

        // resolve the import clause braces before the target string
        let Some(bounds) = self.import_clause_bounds(import_span, target_span)? else {
            return Ok(None);
        };
        let open_brace = bounds.open_brace;
        let close_brace = bounds.close_brace;

        // detect cursor inside the clause braces
        let cursor_in_clause = offset >= open_brace.end && offset <= close_brace.start;

        // collect existing names and detect an item under the cursor
        let mut existing_names = Vec::new();
        let mut cursor_in_item = false;
        let view = self.view()?;

        for item_id in items {
            let item = view.get(*item_id);
            let source_id = view.get_source(*item_id);
            let node = item_id.into_global_any(self.module_id());
            let span = self
                .source_index()?
                .try_get(source_id)
                .ok_or(QueryError::missing(format!("import item span: {node:?}")))?;

            let (item_name, item_alias) = match item {
                dir::DependencyItem::Binding { name, alias, .. } => (*name, *alias),
                dir::DependencyItem::Error => continue,
            };

            if span.contains(offset) {
                cursor_in_item = true;
                continue;
            }

            if let Some(name_id) = item_name {
                existing_names.push(self.strings().get(name_id.string()).to_string());
            }

            if let Some(alias_id) = item_alias {
                existing_names.push(self.strings().get(alias_id).to_string());
            }
        }

        if !cursor_in_clause && !cursor_in_item {
            return Ok(None);
        }

        // retain names once in source order
        let mut seen = FxHashSet::default();
        existing_names.retain(|name| seen.insert(name.clone()));

        Ok(Some(existing_names))
    }
}

impl CompletionCollector<'_, '_, '_> {
    /// Collect exports from one imported module namespace.
    pub(super) fn collect_namespace_members(
        &self,
        module_id: ModuleId,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let mut results = Vec::new();

        // transcribe each exact module export as a member candidate
        for (name, declaration) in self.program.module_exports(module_id)? {
            let kind = declaration.completion_kind(self.program)?;
            let mut completion =
                CompletionCandidate::new(name, kind, CompletionOrigin::Local, SORT_LOCAL_SYMBOL);

            if let ExportDeclaration::Symbol { symbol, .. } = declaration {
                if kind.is_callable() {
                    completion = completion.with_call();
                }
                completion = self.collect_symbol(completion, symbol)?;
            }

            results.push(completion);
        }

        Ok(results)
    }

    /// Collect auto imports for one prefix and scope.
    pub(super) fn collect_auto_imports_with_visibility(
        &self,
        prefix: &str,
        use_filter: Option<SymbolUse>,
        scope: dir::LocalScope,
        allow_short_prefix: bool,
    ) -> QueryResult<CompletionCandidates> {
        let mut completions = self.collect_auto_imports(prefix, use_filter, allow_short_prefix)?;

        let visible_names = self.collect_visible_names(scope, use_filter)?;
        completions.retain(|item| !visible_names.contains(item.label.as_str()));

        let is_incomplete =
            ShortPrefixImportOrder::apply_limit(&mut completions, prefix, allow_short_prefix);

        Ok(CompletionCandidates {
            items: completions,
            is_incomplete,
        })
    }

    /// Collect auto imports for one prefix.
    fn collect_auto_imports(
        &self,
        prefix: &str,
        use_filter: Option<SymbolUse>,
        allow_short_prefix: bool,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        if prefix.is_empty() {
            return Ok(Vec::new());
        }

        if prefix.len() < AUTO_IMPORT_MIN_PREFIX && !allow_short_prefix {
            return Ok(Vec::new());
        }

        // search every other indexed module for matching exported declarations
        let current_module_id = self.module.module_id();
        let mut results = Vec::new();
        let mut seen: FxHashSet<(ModuleId, ImportBinding)> = FxHashSet::default();
        let exports = self
            .program
            .search_export_candidates(prefix, Some(current_module_id))?;

        for export in exports {
            if !use_filter.is_none_or(|symbol_use| symbol_use.accepts_export(export.declaration)) {
                continue;
            }

            let module_id = export.module;
            let key = (module_id, export.binding.clone());
            if !seen.insert(key) {
                continue;
            }

            self.push_auto_import_completions(
                current_module_id,
                module_id,
                &export.binding,
                export.declaration,
                use_filter,
                &mut results,
            )?;
        }

        Ok(results)
    }

    /// Collect visible symbol names for a scope and use.
    fn collect_visible_names(
        &self,
        scope: dir::LocalScope,
        use_filter: Option<SymbolUse>,
    ) -> QueryResult<FxHashSet<String>> {
        let symbols = self.module.bindings()?;

        let mut names = FxHashSet::default();
        for visible in symbols.visible_bindings(scope).filter(|binding| {
            use_filter.is_none_or(|symbol_use| symbol_use.accepts_symbol_kind(binding.symbol.kind))
        }) {
            let dir::StaticKey::Name(name_id) = visible.key else {
                continue;
            };

            names.insert(self.module.strings().get(name_id).to_string());
        }

        Ok(names)
    }

    /// Push every valid auto import completion into the results list.
    fn push_auto_import_completions(
        &self,
        current_module_id: ModuleId,
        module_id: ModuleId,
        binding: &ImportBinding,
        declaration: ExportDeclaration,
        expected_use: Option<SymbolUse>,
        results: &mut Vec<CompletionCandidate>,
    ) -> QueryResult<()> {
        let import_specifiers = self
            .program
            .import_specifiers(current_module_id, module_id)?;
        let name = binding.name();

        // build one completion for each exact importable package export
        for import_specifier in import_specifiers {
            let candidate = ImportCandidate {
                repository: self.module.repository(),
                revision: self.module.revision(),
                current_module_id,
                export_name: name,
                expected_use,
                declaration,
                module_id,
            };
            let import_order = candidate.order(&import_specifier)?;
            let kind = declaration.completion_kind(self.program)?;
            let description = format!("from {import_specifier}");
            let completion =
                CompletionCandidate::new(name, kind, CompletionOrigin::AutoImport, SORT_DEFAULT)
                    .with_description(description)
                    .with_import_order(import_order)
                    .with_auto_import(binding.clone(), import_specifier);
            let completion = match declaration {
                ExportDeclaration::Symbol { symbol, .. } => {
                    let completion = if kind.is_callable() {
                        completion.with_call()
                    } else {
                        completion
                    };
                    self.collect_symbol(completion, symbol)?
                }
                ExportDeclaration::Namespace { .. } => completion,
            };

            results.push(completion);
        }

        Ok(())
    }

    /// Collect imports from one module.
    pub(super) fn collect_imports(
        &self,
        target_module: Option<ModuleId>,
        existing_names: &[String],
        use_filter: Option<SymbolUse>,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let Some(module_id) = target_module else {
            return Ok(Vec::new());
        };

        let existing_names: FxHashSet<&str> = existing_names.iter().map(String::as_str).collect();
        let mut results = Vec::new();

        // complete only declarations exposed by the exact resolved module export table
        for (name, declaration) in self.program.module_exports(module_id)? {
            if !use_filter.is_none_or(|symbol_use| symbol_use.accepts_export(declaration)) {
                continue;
            }
            if existing_names.contains(name.as_str()) {
                continue;
            }

            let kind = declaration.completion_kind(self.program)?;
            let completion = CompletionCandidate::new(
                name,
                kind,
                CompletionOrigin::Contextual,
                SORT_LOCAL_SYMBOL,
            );
            let completion = match declaration {
                ExportDeclaration::Symbol { symbol, .. } => {
                    self.collect_symbol(completion, symbol)?
                }
                ExportDeclaration::Namespace { .. } => completion,
            };

            results.push(completion);
        }

        Ok(results)
    }

    /// Collect relative paths and package names.
    pub(super) fn collect_import_paths(
        &self,
        partial: &str,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let mut results = Vec::new();

        if partial.starts_with("./") || partial.starts_with("../") {
            results.extend(self.collect_relative_path(partial)?);
        } else if partial.is_empty() {
            results.push(
                CompletionCandidate::new(
                    "./",
                    CompletionItemKind::Folder,
                    CompletionOrigin::Contextual,
                    5,
                )
                .with_detail("relative"),
            );
            results.push(
                CompletionCandidate::new(
                    "../",
                    CompletionItemKind::Folder,
                    CompletionOrigin::Contextual,
                    6,
                )
                .with_detail("parent"),
            );
            results.extend(self.collect_package_names()?);
        } else {
            results.extend(self.collect_package_names()?);
        }

        Ok(results)
    }

    /// Collect package names from the active package graph.
    fn collect_package_names(&self) -> QueryResult<Vec<CompletionCandidate>> {
        let mut results = Vec::new();

        let repository = self.module.repository();
        let revision = self.module.revision();
        let module_id = self.module.module_id();
        let module = repository
            .module(revision, module_id)?
            .ok_or(RepositoryError::MissingModule { module: module_id })?;
        let package = self.program.package_node(module.package_id)?;

        // offer only loaded direct dependencies from the active profile
        for (name, dependency) in &package.dependencies {
            if !matches!(dependency, PackageDependency::Resolved(_)) {
                continue;
            }
            let completion = CompletionCandidate::new(
                name.clone(),
                CompletionItemKind::Module,
                CompletionOrigin::Builtin,
                SORT_BUILTIN,
            )
            .with_ordering_text(length_ordering_text(name));
            results.push(completion);
        }

        Ok(results)
    }
}

/// Stable auto-import ordering for short-prefix completion pruning.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ShortPrefixImportOrder {
    /// The lexical match bucket.
    lexical: Option<MatchOrder>,
    /// The import-source order.
    import: Option<ImportOrder>,
    /// The rendered completion label.
    label: String,
}

impl ShortPrefixImportOrder {
    /// Apply short-prefix pruning after visibility filtering.
    fn apply_limit(
        completions: &mut Vec<CompletionCandidate>,
        prefix: &str,
        allow_short_prefix: bool,
    ) -> bool {
        if !allow_short_prefix || prefix.len() >= AUTO_IMPORT_MIN_PREFIX {
            return false;
        }

        let previous_length = completions.len();
        Self::sort(completions, prefix);
        completions.truncate(AUTO_IMPORT_SHORT_PREFIX_LIMIT);

        completions.len() != previous_length
    }

    /// Sort auto import completions for short prefixes.
    fn sort(completions: &mut [CompletionCandidate], prefix: &str) {
        completions.sort_by(|left, right| Self::new(left, prefix).cmp(&Self::new(right, prefix)));
    }

    /// Build the ordering for one completion candidate.
    fn new(completion: &CompletionCandidate, prefix: &str) -> Self {
        Self {
            lexical: match_quality(&completion.label, prefix)
                .as_ref()
                .map(MatchQuality::order),
            import: completion.import_order.clone(),
            label: completion.label.clone(),
        }
    }
}

impl CompletionCollector<'_, '_, '_> {
    /// Collect relative import paths from modules in the queried revision.
    fn collect_relative_path(&self, partial: &str) -> QueryResult<Vec<CompletionCandidate>> {
        let split = partial.rfind('/').map_or(0, |index| index + 1);
        let directory = &partial[..split];
        let current_module = self.module.module_id();
        let mut completions = Vec::new();
        let mut seen = FxHashSet::default();

        // project every same-package module specifier to its next path segment
        for target_module in self.program.module_ids() {
            if target_module.package_id != current_module.package_id
                || *target_module == current_module
            {
                continue;
            }
            for specifier in self
                .program
                .import_specifiers(current_module, *target_module)?
            {
                let Some(remainder) = specifier.strip_prefix(directory) else {
                    continue;
                };
                let (name, kind) = match remainder.split_once('/') {
                    Some((segment, _)) => (format!("{segment}/"), CompletionItemKind::Folder),
                    None => (remainder.to_string(), CompletionItemKind::Module),
                };
                if !seen.insert((name.clone(), kind)) {
                    continue;
                }

                completions.push(CompletionCandidate::new(
                    name,
                    kind,
                    CompletionOrigin::Contextual,
                    SORT_LOCAL_SYMBOL,
                ));
            }
        }

        Ok(completions)
    }
}
