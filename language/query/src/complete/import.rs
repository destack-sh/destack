use destack_artifact::PackageDependency;
use destack_dir as dir;
use destack_repository::RepositoryError;
use destack_source::{FileId, FileType, Loader, ModuleId, Span};
use rustc_hash::FxHashSet;

use crate::source::{ImportBinding, extract_string_literal_prefix, strip_module_extension};
use crate::{
    CompletionCandidate, CompletionCandidates, CompletionItemKind, CompletionOrigin,
    ExportDeclaration, ImportCandidate, ImportOrder, MatchOrder, MatchQuality, ModuleQueryContext,
    ProgramQueryContext, QueryError, QueryResult, SORT_BUILTIN, SORT_DEFAULT, SORT_LOCAL_SYMBOL,
    SymbolUse, match_quality,
};

use super::CompletionContext;
use super::builder::CompletionBuilder;
use super::builtin::length_ordering_text;
use super::call::CallSnippet;

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

impl CompletionBuilder<'_, '_, '_> {
    /// Complete the exports of one imported module namespace.
    pub(super) fn complete_namespace_members(
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
                completion = self.attach_symbol_completion(completion, symbol)?;

                if kind == CompletionItemKind::Function
                    && let Some(parameter_names) = self.program.symbol_parameter_names(symbol)?
                {
                    let snippet = CallSnippet::named(&completion.label, &parameter_names);
                    completion = completion.with_insert_text(snippet.text);
                    if snippet.is_snippet {
                        completion = completion.with_snippet();
                    }
                }
            }

            results.push(completion);
        }

        Ok(results)
    }

    /// Generate auto import completions for a prefix using indexed lookup.
    pub(super) fn complete_auto_imports_with_visibility(
        &self,
        prefix: &str,
        use_filter: Option<SymbolUse>,
        scope: dir::LocalScope,
        allow_short_prefix: bool,
    ) -> QueryResult<CompletionCandidates> {
        let mut completions = self.complete_auto_imports(prefix, use_filter, allow_short_prefix)?;

        let visible_names = self.collect_visible_names(scope, use_filter)?;
        completions.retain(|item| !visible_names.contains(item.label.as_str()));

        let is_incomplete =
            ShortPrefixImportOrder::apply_limit(&mut completions, prefix, allow_short_prefix);

        Ok(CompletionCandidates {
            items: completions,
            is_incomplete,
        })
    }

    /// Generate auto import completions for one prefix.
    fn complete_auto_imports(
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
            let import_edits =
                self.module
                    .build_import_edits(self.file_id, binding, &import_specifier)?;
            if import_edits.is_empty() {
                continue;
            }

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
            let detail = format!("Auto import from {import_specifier}");
            let completion =
                CompletionCandidate::new(name, kind, CompletionOrigin::AutoImport, SORT_DEFAULT)
                    .with_detail(detail)
                    .with_import_order(import_order)
                    .with_additional_edits(import_edits);

            results.push(completion);
        }

        Ok(())
    }

    /// Complete imports from one module.
    pub(super) fn complete_imports(
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
                    self.attach_symbol_completion(completion, symbol)?
                }
                ExportDeclaration::Namespace { .. } => completion,
            };

            results.push(completion);
        }

        Ok(results)
    }

    /// Complete import paths, relative paths, or package names.
    pub(super) fn complete_import_paths(
        &self,
        partial: &str,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let mut results = Vec::new();

        if partial.starts_with("./") || partial.starts_with("../") {
            let path = self.file.path.as_deref();
            if let Some(path) = path
                && let Some(base_dir) = path.parent()
            {
                results.extend(self.module.complete_relative_path(base_dir, partial)?);
            }
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
            results.extend(self.complete_package_names("")?);
        } else {
            results.extend(self.complete_package_names(partial)?);
        }

        Ok(results)
    }

    /// Complete package names from the active package graph.
    fn complete_package_names(&self, prefix: &str) -> QueryResult<Vec<CompletionCandidate>> {
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
            if !prefix.is_empty() && !name.to_lowercase().starts_with(&prefix.to_lowercase()) {
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

impl ModuleQueryContext<'_> {
    /// Complete relative import paths by listing directory contents.
    fn complete_relative_path(
        &self,
        base_dir: &std::path::Path,
        partial: &str,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let (directory, prefix) = if let Some(slash) = partial.rfind('/') {
            (base_dir.join(&partial[..=slash]), &partial[slash + 1..])
        } else {
            (base_dir.to_path_buf(), partial)
        };

        // a partial path may name a directory that does not exist yet
        let entries = match self.repository().file_system().read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Vec::new());
            }
            Err(error) => return Err(error.into()),
        };
        let mut completions = Vec::new();

        // collect matching directories and source modules
        for entry in &entries {
            let file_name = entry.file_name().ok_or_else(|| {
                QueryError::missing(format!("path file name: {:?}", entry.to_path_buf()))
            })?;
            let name = file_name
                .to_str()
                .ok_or_else(|| QueryError::invalid(format!("non-Unicode path: {file_name:?}")))?
                .to_string();
            if name.starts_with('.') {
                continue;
            }
            if !prefix.is_empty() && !name.to_lowercase().starts_with(&prefix.to_lowercase()) {
                continue;
            }

            let metadata = self.repository().file_system().metadata(entry)?;
            if metadata.is_directory {
                let completion = CompletionCandidate::new(
                    format!("{name}/"),
                    CompletionItemKind::Folder,
                    CompletionOrigin::Contextual,
                    SORT_LOCAL_SYMBOL,
                );
                completions.push(completion);
                continue;
            }

            let Some(file_type) = FileType::from_path(entry) else {
                continue;
            };
            let Ok(loader) = Loader::try_from(file_type) else {
                continue;
            };
            if !loader.is_code() {
                continue;
            }

            let module_name = strip_module_extension(&name);
            let completion = CompletionCandidate::new(
                module_name,
                CompletionItemKind::Module,
                CompletionOrigin::Contextual,
                SORT_LOCAL_SYMBOL,
            );
            completions.push(completion);
        }

        Ok(completions)
    }
}
