use std::collections::hash_map::Entry;

use destack_artifact::PackageDependency;
use destack_core::StringId;
use destack_dir as dir;
use destack_repository::RepositoryError;
use destack_source::{FileId, ModuleId, Span};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::source::extract_string_literal_prefix;
use crate::{
    CompletionCandidate, CompletionCandidates, CompletionItemKind, CompletionOrigin,
    ExportCandidate, ExportDeclaration, ImportOrder, ImportPathOrder, MatchOrder,
    ModuleQueryContext, ProgramQueryContext, QueryError, QueryResult, SymbolUse, match_quality,
};

use super::builtin::length_ordering_text;
use super::{AutoImportContext, CompletionCollector, CompletionContext};

// auto import completion thresholds
const AUTO_IMPORT_MIN_PREFIX: usize = 2;
const AUTO_IMPORT_SHORT_PREFIX_LIMIT: usize = 50;

/// One unresolved export and import path considered for auto import.
struct AutoImportCandidate {
    /// The indexed program export.
    export: ExportCandidate,
    /// The lexical match order.
    lexical: MatchOrder,
    /// The import path inserted into the current module.
    specifier: String,
    /// The structural order of the import path.
    path_order: ImportPathOrder,
}

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

        // collect each module export as a member candidate
        for (name, declaration) in self.program.module_exports(module_id)? {
            let kind = declaration.completion_kind(self.program)?;
            let mut completion = CompletionCandidate::new(name, kind, CompletionOrigin::Local);

            if let ExportDeclaration::Symbol { symbol, .. } = declaration {
                if kind.is_callable() {
                    completion = completion.with_call();
                }
                completion = completion.with_symbol(symbol);
            }

            results.push(completion);
        }

        Ok(results)
    }

    /// Collect auto imports for one prefix and scope.
    pub(super) fn collect_auto_imports(
        &self,
        prefix: &str,
        context: AutoImportContext,
        allow_short_prefix: bool,
    ) -> QueryResult<CompletionCandidates> {
        if prefix.is_empty() {
            return Ok(CompletionCandidates {
                items: Vec::new(),
                is_incomplete: false,
            });
        }

        if prefix.len() < AUTO_IMPORT_MIN_PREFIX && !allow_short_prefix {
            return Ok(CompletionCandidates {
                items: Vec::new(),
                is_incomplete: false,
            });
        }

        // collect unresolved exports and import paths before reading declaration DIR
        let current_module_id = self.module.module_id();
        let visible_names =
            self.collect_visible_names(context.scope, context.symbol_use, prefix)?;
        let mut candidates = Vec::new();
        let mut import_paths = FxHashMap::default();
        let exports = self
            .program
            .search_export_candidates(prefix, Some(current_module_id))?;

        for export in exports {
            let name = StringId::for_text(export.binding.name());
            if visible_names.contains(&name) {
                continue;
            }

            let lexical = match_quality(export.binding.name(), prefix)
                .ok_or(QueryError::invalid(format!(
                    "auto import candidate does not match: name={}, prefix={prefix}",
                    export.binding.name()
                )))?
                .order();
            let module_id = export.module;
            if let Entry::Vacant(entry) = import_paths.entry(module_id) {
                let path_order = ImportPathOrder::between(
                    self.module.repository(),
                    self.module.revision(),
                    current_module_id,
                    module_id,
                )?;
                let specifiers = self
                    .program
                    .import_specifiers(current_module_id, module_id)?;
                entry.insert((path_order, specifiers));
            }
            let (path_order, specifiers) = &import_paths[&module_id];

            for specifier in specifiers {
                candidates.push(AutoImportCandidate {
                    export: export.clone(),
                    lexical,
                    specifier: specifier.clone(),
                    path_order: path_order.clone(),
                });
            }
        }

        // shortlist by the same lexical and path order used by final items
        candidates.sort_by(AutoImportCandidate::compare);
        let limit = (allow_short_prefix && prefix.len() < AUTO_IMPORT_MIN_PREFIX)
            .then_some(AUTO_IMPORT_SHORT_PREFIX_LIMIT);
        let mut items = Vec::new();
        let mut selected = FxHashSet::default();
        let mut is_incomplete = false;

        // resolve declarations until the result limit is filled
        'candidate: for candidate in candidates {
            for declaration in candidate.export.resolve_declarations(self.program)? {
                if !context.symbol_use.accepts_export(declaration) {
                    continue;
                }

                let kind = declaration.completion_kind(self.program)?;
                if context.is_constructable_only && !kind.is_constructable() {
                    continue;
                }

                let key = (
                    candidate.export.module,
                    candidate.export.binding.clone(),
                    candidate.specifier.clone(),
                );
                if !selected.insert(key) {
                    continue 'candidate;
                }
                if limit.is_some_and(|limit| items.len() == limit) {
                    is_incomplete = true;

                    break 'candidate;
                }

                items.push(self.resolve_auto_import(candidate, declaration, kind, context)?);

                continue 'candidate;
            }
        }

        Ok(CompletionCandidates {
            items,
            is_incomplete,
        })
    }

    /// Collect visible symbol names for a scope and use.
    fn collect_visible_names(
        &self,
        scope: dir::LocalScope,
        symbol_use: SymbolUse,
        prefix: &str,
    ) -> QueryResult<FxHashSet<StringId>> {
        let mut names = FxHashSet::default();
        for key in self
            .visible_bindings(scope, symbol_use, prefix)?
            .into_keys()
        {
            let dir::StaticKey::Name(name) = key else {
                continue;
            };

            names.insert(name);
        }

        Ok(names)
    }

    /// Resolve one shortlisted auto import candidate.
    fn resolve_auto_import(
        &self,
        candidate: AutoImportCandidate,
        declaration: ExportDeclaration,
        kind: CompletionItemKind,
        context: AutoImportContext,
    ) -> QueryResult<CompletionCandidate> {
        let name = candidate.export.binding.name();
        let import_order = ImportOrder::new(candidate.path_order, &candidate.specifier, name);
        let description = format!("from {}", candidate.specifier);

        // use the same item kind as the matching local completion
        let completion_kind =
            if context.symbol_use == SymbolUse::Value && kind == CompletionItemKind::Newtype {
                CompletionItemKind::Constructor
            } else {
                kind
            };
        let completion =
            CompletionCandidate::new(name, completion_kind, CompletionOrigin::AutoImport)
                .with_description(description)
                .with_import_order(import_order)
                .with_import(candidate.export.binding, candidate.specifier);

        match declaration {
            ExportDeclaration::Symbol { symbol, .. } => {
                let completion = completion.with_symbol(symbol);

                // select the insertion used by the matching local declaration
                let completion =
                    if context.is_constructable_only && kind == CompletionItemKind::Class {
                        completion.with_class_constructors(symbol)
                    } else if context.symbol_use == SymbolUse::Value
                        && kind == CompletionItemKind::Struct
                    {
                        completion.with_struct(symbol)
                    } else if context.symbol_use == SymbolUse::Value
                        && kind == CompletionItemKind::Newtype
                    {
                        completion.with_newtype_constructors(symbol)
                    } else if kind.is_callable() {
                        completion.with_call()
                    } else {
                        completion
                    };

                Ok(completion)
            }
            ExportDeclaration::Namespace { .. } => Ok(completion),
        }
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
            let completion = CompletionCandidate::new(name, kind, CompletionOrigin::Contextual);
            let completion = match declaration {
                ExportDeclaration::Symbol { symbol, .. } => completion.with_symbol(symbol),
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
                )
                .with_description("relative"),
            );
            results.push(
                CompletionCandidate::new(
                    "../",
                    CompletionItemKind::Folder,
                    CompletionOrigin::Contextual,
                )
                .with_description("parent"),
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
            )
            .with_ordering_text(length_ordering_text(name));
            results.push(completion);
        }

        Ok(results)
    }
}

impl AutoImportCandidate {
    /// Compare unresolved candidates by final lexical and import-path order.
    fn compare(&self, other: &Self) -> std::cmp::Ordering {
        self.lexical
            .cmp(&other.lexical)
            .then(self.path_order.cmp(&other.path_order))
            .then(
                self.specifier
                    .chars()
                    .count()
                    .cmp(&other.specifier.chars().count()),
            )
            .then(self.specifier.cmp(&other.specifier))
            .then(self.export.binding.name().cmp(other.export.binding.name()))
            .then(self.export.target.cmp(&other.export.target))
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
                ));
            }
        }

        Ok(completions)
    }
}
