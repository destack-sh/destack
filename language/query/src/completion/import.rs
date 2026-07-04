use std::collections::HashSet;

use destack_dir as dir;
use destack_source::{FileType, Loader, ModuleId, PackageId, Span};

use crate::refactor::ImportEditForm;
use crate::source::{extract_string_literal_prefix, token_text};
use crate::{
    ImportOrder, MatchOrder, MatchQuality, ModuleQueryContext, ScopeAtOffset, SymbolUse,
    match_quality, module_name_from_path, repository_import_relevance, visible_symbols,
};

use super::builder::CompletionBuilder;
use super::builtin::length_sort_text;
use super::{
    Completion, CompletionContext, CompletionKind, CompletionValueShape, SORT_BUILTIN,
    SORT_DEFAULT, SORT_LOCAL_SYMBOL,
};

// auto import completion thresholds
const AUTO_IMPORT_MIN_PREFIX: usize = 2;
const AUTO_IMPORT_SHORT_PREFIX_LIMIT: usize = 50;

impl ModuleQueryContext<'_> {
    /// Return import related context.
    pub(super) fn import_context(&self, source: &str, offset: u32) -> Option<CompletionContext> {
        // resolve enclosing spans from innermost to outermost
        let enclosing = self.enclosing_spans_at_cursor(offset);

        // scan enclosing expressions for import nodes under the cursor
        for enc in &enclosing {
            if self.tree().get_node_type(enc.source_id) != dir::NodeType::Expression {
                continue;
            }

            let expr_id = dir::LocalNodeId::<dir::Expression>::new(enc.source_id);
            let expr = self.tree().get(expr_id);

            if !matches!(expr, dir::Expression::Import { .. }) {
                continue;
            }

            // detect path completions inside the import string
            let import_span = self.tree().source_index.get(expr_id.id);
            let main_span = self.tree().source_index.get_main(expr_id.id);
            if let Some(span) = main_span {
                if span.contains(offset) {
                    let partial_path = extract_string_literal_prefix(source, span, offset);
                    return Some(CompletionContext::ImportPath { partial_path });
                }
            }

            if let Some(span) = self.import_path_span_from_tokens(import_span, offset) {
                let partial_path = extract_string_literal_prefix(source, span, offset);
                return Some(CompletionContext::ImportPath { partial_path });
            }

            // detect import clause completions inside the brace list
            if let Some(context) =
                self.import_clause_context(expr, offset, source, import_span, main_span)
            {
                let dir::Expression::Import { target, .. } = expr else {
                    continue;
                };

                let target_module = self.resolved_import_target_module(*target);

                return Some(CompletionContext::ImportClause {
                    target_module,
                    existing_names: context.existing_names,
                    use_filter: context.use_filter,
                });
            }
        }

        None
    }

    /// Resolve a string literal span for an import path at the cursor.
    fn import_path_span_from_tokens(&self, import_span: Span, offset: u32) -> Option<Span> {
        // find the token under the cursor
        let token = self.token_span_at_offset(offset)?;

        // require the token to stay inside the import statement span
        if token.span.start < import_span.start || token.span.end > import_span.end {
            return None;
        }

        // require a string literal token
        if token.token.ty() != dir::TokenType::Literal {
            return None;
        }

        if !matches!(
            token.token.literal(),
            Some(dir::TokenLiteral::String { .. })
        ) {
            return None;
        }

        Some(token.span)
    }

    /// Resolve one import target module from compiler resolved import edges.
    fn resolved_import_target_module(&self, target: destack_core::StringId) -> Option<ModuleId> {
        let specifier = self.strings().get(target);
        let target_id = self.strings().intern(specifier);
        let dependency = self.modules().iter().find(|dependency| {
            dependency.specifier == target_id
                && dependency.relation == dir::ModuleRelation::Import
                && dependency.loader.is_none()
        })?;

        dependency.target
    }
}

/// Import clause completion context.
struct ImportClauseContext {
    /// Existing names in the clause.
    existing_names: Vec<String>,
    /// Optional symbol use filter.
    use_filter: Option<SymbolUse>,
}

impl ImportClauseContext {
    /// Create an import clause context.
    fn new(mut existing_names: Vec<String>, use_filter: Option<SymbolUse>) -> Self {
        Self::deduplicate_names(&mut existing_names);

        Self {
            existing_names,
            use_filter,
        }
    }

    /// Deduplicate names while preserving their first occurrence order.
    fn deduplicate_names(names: &mut Vec<String>) {
        let mut deduped = HashSet::new();
        names.retain(|name| deduped.insert(name.clone()));
    }
}

impl ModuleQueryContext<'_> {
    /// Resolve import clause context at the given offset.
    fn import_clause_context(
        &self,
        expr: &dir::Expression,
        offset: u32,
        source: &str,
        import_span: Span,
        target_span: Option<Span>,
    ) -> Option<ImportClauseContext> {
        // require an import expression
        let dir::Expression::Import { items, form, .. } = expr else {
            return None;
        };
        let items = items.as_ref()?;

        // resolve the import clause braces before the target string
        let bounds = self.import_clause_bounds(import_span, target_span)?;
        let open_brace = bounds.open_brace;
        let end_boundary = bounds.end_boundary;

        // detect cursor inside the clause braces
        let cursor_in_clause = offset >= open_brace.end && offset <= end_boundary.start;

        // collect existing names and detect item form under the cursor
        let mut existing_names = Vec::new();
        let mut in_item_form = None;

        for item_id in items {
            let item = self.tree().get(*item_id);
            let span = self.tree().source_index.get(item_id.id);

            let (item_space, item_name, item_alias) = match item {
                dir::DependencyItem::Binding {
                    form, name, alias, ..
                } => (*form, *name, *alias),
                dir::DependencyItem::Error => {
                    panic!("error dependency item reached import completion")
                }
            };

            if span.contains(offset) {
                in_item_form = item_space;
                continue;
            }

            if let Some(name_id) = item_name {
                existing_names.push(self.strings().get(name_id.string()).to_string());
            }

            if let Some(alias_id) = item_alias {
                existing_names.push(self.strings().get(alias_id).to_string());
            }
        }

        if !cursor_in_clause && in_item_form.is_none() {
            return None;
        }

        // detect cursor after a type keyword inside the clause
        let cursor_is_type = if cursor_in_clause {
            if let Some(token) = self.previous_significant_token(offset) {
                let token_in_clause =
                    token.span.start >= open_brace.start && token.span.end <= end_boundary.end;
                let token_source = token_text(source, token.span).unwrap_or_else(|| {
                    panic!("invalid import clause token source range: {:?}", token.span)
                });

                token_in_clause
                    && token.token.ty() == dir::TokenType::Identifier
                    && token_source == "type"
            } else {
                false
            }
        } else {
            false
        };

        let use_filter = match form {
            dir::DependencyForm::Type => Some(SymbolUse::Type),
            dir::DependencyForm::Plain => match in_item_form {
                Some(dir::DependencyForm::Type) => Some(SymbolUse::Type),
                Some(dir::DependencyForm::Plain) => Some(SymbolUse::Value),
                None if cursor_is_type => Some(SymbolUse::Type),
                None => None,
            },
        };

        Some(ImportClauseContext::new(existing_names, use_filter))
    }
}

impl CompletionBuilder<'_, '_> {
    /// Generate auto import completions for a prefix using indexed lookup.
    pub(super) fn complete_auto_imports_with_visibility(
        &self,
        prefix: &str,
        use_filter: Option<SymbolUse>,
        scope: ScopeAtOffset,
        excluded_labels: &HashSet<String>,
        allow_short_prefix: bool,
    ) -> Vec<Completion> {
        let mut completions = self.complete_auto_imports(prefix, use_filter, allow_short_prefix);

        let visible_names = self.collect_visible_names(scope, use_filter);
        completions.retain(|item| !visible_names.contains(item.label.as_str()));

        if !excluded_labels.is_empty() {
            completions.retain(|item| !excluded_labels.contains(&item.label));
        }

        ShortPrefixImportOrder::apply_limit(&mut completions, prefix, allow_short_prefix);

        completions
    }

    /// Generate auto import completions for one prefix.
    fn complete_auto_imports(
        &self,
        prefix: &str,
        use_filter: Option<SymbolUse>,
        allow_short_prefix: bool,
    ) -> Vec<Completion> {
        if prefix.is_empty() {
            return Vec::new();
        }

        if prefix.len() < AUTO_IMPORT_MIN_PREFIX && !allow_short_prefix {
            return Vec::new();
        }

        let module = self
            .module
            .repository()
            .module(self.module.revision(), self.module.module_id())
            .unwrap_or_else(|error| panic!("failed to read completion module: {error}"))
            .unwrap_or_else(|| panic!("missing completion module {:?}", self.module.module_id()));
        let current_module_id = Some(module.id);
        let current_package_id = Some(module.package_id);

        let mut results = Vec::new();
        let mut seen: HashSet<(ModuleId, dir::GlobalSymbolId)> = HashSet::new();
        let exports = self
            .program
            .search_export_candidates(prefix, current_module_id);

        for export in exports {
            if !use_filter.is_none_or(|symbol_use| symbol_use.accepts_symbol_kind(export.kind)) {
                continue;
            }

            let Some(module_path) = &export.module_path else {
                continue;
            };

            let module_id = export.source.module_id;
            let key = (module_id, export.symbol);
            if !seen.insert(key) {
                continue;
            }

            let import_form = ImportEditForm::auto_import(use_filter, export.kind);

            self.push_auto_import_completion(
                current_package_id,
                prefix,
                module_id,
                export.symbol,
                module_path,
                &export.name,
                export.kind,
                use_filter,
                import_form,
                &mut results,
            );
        }

        results
    }

    /// Collect visible symbol names for a scope and use.
    fn collect_visible_names(
        &self,
        scope: ScopeAtOffset,
        use_filter: Option<SymbolUse>,
    ) -> HashSet<String> {
        let symbols = self.module.symbols();

        let mut names = HashSet::new();
        for visible in visible_symbols(symbols, scope.scope_id, scope.scope_mark, use_filter) {
            let dir::StaticKey::Name(name_id) = visible.key else {
                continue;
            };

            names.insert(self.module.strings().get(name_id).to_string());
        }

        names
    }

    /// Push one auto import completion into the results list.
    fn push_auto_import_completion(
        &self,
        current_package_id: Option<PackageId>,
        prefix: &str,
        module_id: ModuleId,
        symbol_id: dir::GlobalSymbolId,
        module_path: &str,
        export_name: &str,
        symbol_kind: dir::SymbolKind,
        expected_use: Option<SymbolUse>,
        import_form: ImportEditForm,
        results: &mut Vec<Completion>,
    ) {
        let display_path = self.module.import_display_path(module_path);
        let import_edits = self
            .module
            .build_import_edits(export_name, &display_path, import_form);
        if import_edits.is_empty() {
            return;
        }

        let Some(relevance) = repository_import_relevance(
            self.module.repository(),
            self.module.revision(),
            self.module.file_id(),
            current_package_id,
            prefix,
            export_name,
            expected_use,
            symbol_kind,
            module_id,
            module_path,
        ) else {
            return;
        };

        let import_order = ImportOrder::new(&relevance, &display_path, export_name);
        let sort_text = import_order.text();
        let kind = CompletionKind::from(symbol_kind);
        let detail = format!("Auto import from {display_path}");
        let mut completion = Completion::new(export_name, kind)
            .with_detail(detail)
            .with_sort_order(SORT_DEFAULT)
            .with_import_order(import_order)
            .with_sort_text(sort_text)
            .with_additional_edits(import_edits)
            .with_value_shape(CompletionValueShape::symbol_kind(symbol_kind))
            .as_auto_import();

        if let Some(nominal_symbol) = self.symbol_nominal(symbol_id) {
            completion = completion.with_nominal_symbol(nominal_symbol);
        }
        completion = completion.with_related_nominals(self.symbol_related_nominals(symbol_id));

        results.push(completion);
    }

    /// Complete imports from one module.
    pub(super) fn complete_imports(
        &self,
        target_module: Option<ModuleId>,
        existing_names: &[String],
        use_filter: Option<SymbolUse>,
    ) -> Vec<Completion> {
        let Some(module_id) = target_module else {
            return Vec::new();
        };

        let target_module = self.module.module_context(module_id);
        let symbols = target_module.symbols();
        let existing_names: HashSet<&str> = existing_names.iter().map(String::as_str).collect();
        let mut results = Vec::new();

        for symbol in symbols.symbols() {
            if symbol.export_kind.is_none() {
                continue;
            }

            let Some(string_id) = symbol.name() else {
                continue;
            };

            if !use_filter.is_none_or(|symbol_use| symbol_use.accepts_symbol_kind(symbol.kind)) {
                continue;
            }

            let name = target_module.strings().get(string_id).to_string();
            if existing_names.contains(name.as_str()) {
                continue;
            }

            let kind = CompletionKind::from(symbol.kind);
            results.push(
                Completion::new(name, kind)
                    .with_sort_order(SORT_LOCAL_SYMBOL)
                    .as_contextual(),
            );
        }

        results
    }

    /// Complete import paths, relative paths, or package names.
    pub(super) fn complete_import_paths(&self, partial: &str) -> Vec<Completion> {
        let mut results = Vec::new();

        if partial.starts_with("./") || partial.starts_with("../") {
            let path = self.source_file.path.as_deref();
            if let Some(path) = path {
                if let Some(base_dir) = path.parent() {
                    results.extend(self.module.complete_relative_path(base_dir, partial));
                }
            }
        } else if partial.is_empty() {
            results.push(
                Completion::new("./", CompletionKind::Folder)
                    .with_detail("relative")
                    .with_sort_order(5)
                    .as_contextual(),
            );
            results.push(
                Completion::new("../", CompletionKind::Folder)
                    .with_detail("parent")
                    .with_sort_order(6)
                    .as_contextual(),
            );
            results.extend(self.module.complete_package_names(""));
        } else {
            results.extend(self.module.complete_package_names(partial));
        }

        results
    }
}

/// Stable auto-import ordering for short-prefix completion pruning.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ShortPrefixImportOrder {
    /// The lexical match bucket.
    lexical: Option<MatchOrder>,
    /// The import-source order.
    import: Option<ImportOrder>,
    /// The explicit completion order text.
    text: String,
    /// The rendered completion label.
    label: String,
}

impl ShortPrefixImportOrder {
    /// Apply short-prefix pruning after visibility filtering.
    fn apply_limit(completions: &mut Vec<Completion>, prefix: &str, allow_short_prefix: bool) {
        if !allow_short_prefix || prefix.len() >= AUTO_IMPORT_MIN_PREFIX {
            return;
        }

        Self::sort(completions, prefix);
        completions.truncate(AUTO_IMPORT_SHORT_PREFIX_LIMIT);
    }

    /// Sort auto import completions for short prefixes.
    fn sort(completions: &mut [Completion], prefix: &str) {
        completions.sort_by(|left, right| Self::new(left, prefix).cmp(&Self::new(right, prefix)));
    }

    /// Build the ordering for one completion candidate.
    fn new(completion: &Completion, prefix: &str) -> Self {
        Self {
            lexical: match_quality(&completion.label, prefix)
                .as_ref()
                .map(MatchQuality::order),
            import: completion.import_order.clone(),
            text: completion.ordering_text().to_string(),
            label: completion.label.clone(),
        }
    }
}

impl ModuleQueryContext<'_> {
    /// Complete package names from the registry.
    fn complete_package_names(&self, prefix: &str) -> Vec<Completion> {
        let mut results = Vec::new();
        let mut seen_packages = HashSet::new();

        let repository = self.repository();
        let revision = self.revision();
        let module_ids = repository
            .module_ids(revision)
            .unwrap_or_else(|error| panic!("failed to read completion modules: {error}"));

        for module_id in module_ids {
            let module = repository
                .module(revision, module_id)
                .unwrap_or_else(|error| {
                    panic!("failed to read completion module {module_id:?}: {error}")
                })
                .unwrap_or_else(|| panic!("missing completion module {module_id:?}"));
            if !seen_packages.insert(module.package_id) {
                continue;
            }

            let package = repository
                .package(revision, module.package_id)
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to read completion package {:?}: {error}",
                        module.package_id
                    )
                })
                .unwrap_or_else(|| panic!("missing completion package {:?}", module.package_id));
            let Some(name) = package.name.as_ref() else {
                continue;
            };

            if !prefix.is_empty() && !name.to_lowercase().starts_with(&prefix.to_lowercase()) {
                continue;
            }

            let mut completion = Completion::new(name.clone(), CompletionKind::Module)
                .with_sort_order(SORT_BUILTIN)
                .with_sort_text(length_sort_text(name))
                .as_builtin();
            if let Some(version) = &package.version {
                completion = completion.with_detail(format!("v{version}"));
            }

            results.push(completion);
        }

        results
    }

    /// Complete relative import paths by listing directory contents.
    fn complete_relative_path(&self, base_dir: &std::path::Path, partial: &str) -> Vec<Completion> {
        let (directory, prefix) = if let Some(slash) = partial.rfind('/') {
            (base_dir.join(&partial[..=slash]), &partial[slash + 1..])
        } else {
            (base_dir.to_path_buf(), partial)
        };

        let Ok(entries) = self.repository().file_system().read_dir(&directory) else {
            return Vec::new();
        };

        entries
            .iter()
            .filter_map(|entry| {
                let file_name = entry.file_name()?;
                let name = file_name.to_string_lossy().to_string();

                if name.starts_with('.') {
                    return None;
                }

                if !prefix.is_empty() && !name.to_lowercase().starts_with(&prefix.to_lowercase()) {
                    return None;
                }

                let meta = self
                    .repository()
                    .file_system()
                    .metadata(entry)
                    .unwrap_or_else(|error| {
                        panic!("failed to read completion entry metadata {entry:?}: {error}")
                    });
                if meta.is_directory {
                    Some(
                        Completion::new(format!("{name}/"), CompletionKind::Folder)
                            .with_sort_order(SORT_LOCAL_SYMBOL)
                            .as_contextual(),
                    )
                } else if let Some(file_type) = FileType::from_path(entry) {
                    let loader = Loader::from(file_type);
                    if !loader.is_code() {
                        return None;
                    }

                    let module_name = module_name_from_path(entry)?;
                    Some(
                        Completion::new(module_name, CompletionKind::Module)
                            .with_sort_order(SORT_LOCAL_SYMBOL)
                            .as_contextual(),
                    )
                } else {
                    None
                }
            })
            .collect()
    }
}
