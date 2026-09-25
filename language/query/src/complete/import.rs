use std::collections::hash_map::Entry;

use rustc_hash::{FxHashMap, FxHashSet};
use tspp_core::{FxIndexMap, StringId};
use tspp_dir as dir;
use tspp_source::{FileId, ModuleId, NodeSpanRegion, NodeSpanType, Span};

use crate::cursor::Cursor;
use crate::source::token_text;
use crate::{
    CompletionCandidate, CompletionCandidates, CompletionItemKind, CompletionOrigin,
    DeclarationUse, ExportCandidate, ExportDeclaration, ImportOrder, ImportPathOrder, MatchKind,
    MatchOrder, ModuleQueryContext, ProgramQueryContext, QueryError, QueryResult, match_quality,
};

use super::{AutoImportContext, CompletionCollector, CompletionPosition, CompletionPrefix};

// auto import completion thresholds
const AUTO_IMPORT_MIN_PREFIX: usize = 2;
const AUTO_IMPORT_SHORT_PREFIX_LIMIT: usize = 50;

/// One partially authored import path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PartialImportPath {
    /// The authored path before the cursor.
    text: String,
    /// The byte offset of the editable path segment.
    segment_start: usize,
    /// The end of the authored path, excluding its closing quote.
    end: u32,
}

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

impl Cursor<'_, '_> {
    /// Classify import completion at one offset.
    pub(super) fn classify_import(&self, source: &str) -> QueryResult<Option<CompletionPosition>> {
        // read source spans for the selected import position
        let file_id = self.file_id;
        let offset = self.offset;
        let enclosing = self.enclosing();
        let view = self.module.view()?;
        let index = self.module.source_index()?;

        // scan enclosing expressions for import nodes under the cursor
        for enclosing_span in enclosing {
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
            let import_span = index.get(enclosing_span.source_id);
            let main_span = index.get_main(enclosing_span.source_id);
            if let Some(span) = main_span
                && span.contains(offset)
            {
                let path = PartialImportPath::parse(source, span, offset)?;

                return Ok(path.map(|path| CompletionPosition::ImportPath { path }));
            }

            if let Some(span) = self
                .module
                .import_path_token_span(file_id, import_span, offset)?
            {
                let path = PartialImportPath::parse(source, span, offset)?;

                return Ok(path.map(|path| CompletionPosition::ImportPath { path }));
            }

            // detect import clause completions inside the brace list
            if let Some(existing_names) =
                self.module
                    .import_clause_names(expression, offset, import_span, main_span)?
            {
                let target_module = self.module.resolved_import_target_module(expression_id)?;

                return Ok(Some(CompletionPosition::ImportClause {
                    target_module,
                    existing_names,
                }));
            }
        }

        Ok(None)
    }
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
    ) -> QueryResult<Option<Vec<StringId>>> {
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

            let item_name = match item {
                dir::DependencyItem::Binding { name, .. } => *name,
                dir::DependencyItem::Error => continue,
            };

            // distinguish the exported name from the local alias
            if span.owns_cursor(offset) {
                if item_name.is_none() {
                    return Ok(None);
                }
                let name_span = self
                    .source_index()?
                    .get_side(source_id, NodeSpanType::Region(NodeSpanRegion::Type))
                    .ok_or(QueryError::missing(format!("import name span: {node:?}")))?;
                if !name_span.owns_cursor(offset) {
                    return Ok(None);
                }

                cursor_in_item = true;
                continue;
            }

            if let Some(name_id) = item_name {
                existing_names.push(name_id.string());
            }
        }

        if !cursor_in_clause && !cursor_in_item {
            return Ok(None);
        }

        Ok(Some(existing_names))
    }
}

impl CompletionCollector<'_, '_, '_> {
    /// Collect exports from one imported module namespace.
    pub(super) fn collect_namespace_members(
        &self,
        module_id: ModuleId,
        usage: DeclarationUse,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let mut results = Vec::new();

        // collect each module export as a member candidate
        for (name, declaration) in self.program.module_exports(module_id)? {
            if !usage.accepts_export(declaration) {
                continue;
            }
            let kind = declaration.completion_kind(self.program)?;
            let mut completion = CompletionCandidate::new(name, kind, CompletionOrigin::Local);

            if let ExportDeclaration::Symbol { symbol, .. } = declaration {
                completion = completion.with_declaration(symbol, usage);
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

        // match unresolved exports before reading paths or declaration DIR
        let current_module_id = self.module.module_id();
        let visible_names = self.collect_visible_names(context.scope, prefix)?;
        let exports = self
            .program
            .search_export_candidates(prefix, Some(current_module_id))?;
        let mut matches = Vec::new();
        for export in exports {
            let name = StringId::for_text(export.binding.name());
            if visible_names.contains(&name) {
                continue;
            }

            let lexical = match_quality(export.binding.name(), prefix).ok_or(
                QueryError::invalid(format!(
                    "auto import candidate does not match: name={}, prefix={prefix}",
                    export.binding.name()
                )),
            )?;
            let lexical = lexical.order();

            matches.push((export, lexical));
        }

        // expand matched exports through their addressable import paths
        let mut candidates = Vec::new();
        let mut import_paths = FxHashMap::default();
        for (export, lexical) in matches {
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
        let mut has_stronger_match = false;

        // resolve declarations until the result limit is filled
        'candidate: for candidate in candidates {
            // omit loose subsequences after selecting a usable stronger match
            if has_stronger_match && candidate.lexical.kind() == MatchKind::Subsequence {
                break;
            }

            for declaration in candidate.export.resolve_declarations(self.program)? {
                if !context.usage.accepts_export(declaration) {
                    continue;
                }

                let kind = declaration.completion_kind(self.program)?;

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

                has_stronger_match |= candidate.lexical.kind() > MatchKind::Subsequence;
                items.push(self.resolve_auto_import(candidate, declaration, kind, context)?);

                continue 'candidate;
            }
        }

        Ok(CompletionCandidates {
            items,
            is_incomplete,
        })
    }

    /// Collect names already bound in the current scope.
    fn collect_visible_names(
        &self,
        scope: dir::LocalScope,
        prefix: &str,
    ) -> QueryResult<FxHashSet<StringId>> {
        // exclude bound names independently from their eligibility for completion
        let bindings = self.module.bindings()?;
        let environment = self.program.environment_bound()?;
        let locals = bindings.visible_bindings(scope).map(|binding| binding.key);
        let globals = environment
            .global_resolutions_by_key
            .iter()
            .filter(|(_, resolutions)| resolutions.len() == 1)
            .map(|(key, _)| *key);
        let mut names = FxHashSet::default();
        for key in locals.chain(globals) {
            let dir::StaticKey::Name(name) = key else {
                continue;
            };

            if match_quality(self.module.strings().get(name), prefix).is_some() {
                names.insert(name);
            }
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

        let completion = CompletionCandidate::new(name, kind, CompletionOrigin::AutoImport)
            .with_description(description)
            .with_import_order(import_order)
            .with_import(candidate.export.binding, candidate.specifier);

        match declaration {
            ExportDeclaration::Symbol { symbol, .. } => {
                // select the insertion used by local and qualified declarations
                let completion = completion.with_declaration(symbol, context.usage);

                Ok(completion)
            }
            ExportDeclaration::Namespace { .. } => Ok(completion),
        }
    }

    /// Collect imports from one module.
    pub(super) fn collect_imports(
        &self,
        target_module: Option<ModuleId>,
        existing_names: &[StringId],
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let Some(module_id) = target_module else {
            return Ok(Vec::new());
        };

        let mut results = Vec::new();

        // complete only declarations exposed by the exact resolved module export table
        for (name, declaration) in self.program.module_exports(module_id)? {
            if existing_names.contains(&StringId::for_text(&name)) {
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

    /// Collect every module path addressable from the current module.
    pub(super) fn collect_import_paths(
        &self,
        path: &PartialImportPath,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let current_module = self.module.module_id();
        let mut completions = FxIndexMap::default();

        // project every valid module specifier to its next lexical component
        for specifier in self.program.addressable_specifiers(current_module)? {
            let Some((label, kind)) = path.completion(&specifier) else {
                continue;
            };

            // prefer an exact module when the same text is also a path prefix
            let folder = format!("{label}/");
            if let Some(existing) = completions.get_mut(&label) {
                if kind == CompletionItemKind::Module {
                    *existing = kind;
                }
            } else if kind == CompletionItemKind::Folder
                && label.ends_with('/')
                && completions.contains_key(label.trim_end_matches('/'))
            {
                continue;
            } else {
                if kind == CompletionItemKind::Module {
                    completions.shift_remove(&folder);
                }
                completions.insert(label, kind);
            }
        }

        Ok(completions
            .into_iter()
            .map(|(label, kind)| {
                CompletionCandidate::new(label, kind, CompletionOrigin::Contextual)
            })
            .collect())
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

impl PartialImportPath {
    /// Parse the authored import path and its editable range.
    fn parse(source: &str, span: Span, offset: u32) -> QueryResult<Option<Self>> {
        // exclude the literal delimiters from the editable path
        let literal = token_text(source, span)?;
        let quoted = matches!(literal.as_bytes().first(), Some(b'"' | b'\''));
        let start = span.start + u32::from(quoted);
        let closed =
            quoted && literal.len() > 1 && literal.as_bytes().first() == literal.as_bytes().last();
        let end = span.end - u32::from(closed);

        // complete only positions inside the string contents
        if offset < start || offset > end {
            return Ok(None);
        }

        // retain the prefix used to filter candidates
        let text = source
            .get(start as usize..offset as usize)
            .ok_or_else(|| QueryError::invalid(format!("import path cursor: {span:?}, {offset}")))?
            .to_string();

        // select the directory or package prefix preceding the editable name
        let slash_count = text.bytes().filter(|byte| *byte == b'/').count();
        let root_is_incomplete = text.starts_with('@') && slash_count <= 1;
        let separator = if text.starts_with("tspp:") {
            text.rfind([':', '/'])
        } else if root_is_incomplete {
            None
        } else {
            text.rfind('/')
        };
        let segment_start = separator.map_or(0, |index| index + 1);

        Ok(Some(Self {
            text,
            segment_start,
            end,
        }))
    }

    /// Build the editable path segment at one source offset.
    pub(crate) fn prefix(&self, offset: u32) -> QueryResult<CompletionPrefix> {
        let text = self.text[self.segment_start..].to_string();
        let start = offset
            .checked_sub(text.len() as u32)
            .ok_or(QueryError::invalid("import path completion range"))?;

        Ok(CompletionPrefix {
            text,
            start,
            end: self.end,
        })
    }

    /// Project one addressable specifier to its next completion.
    pub(crate) fn completion(&self, specifier: &str) -> Option<(String, CompletionItemKind)> {
        // project relative paths one segment at a time
        if specifier.starts_with("./") || specifier.starts_with("../") {
            if !self.text.is_empty()
                && !self.text.starts_with("./")
                && !self.text.starts_with("../")
            {
                return None;
            }

            return self.segment_completion(specifier);
        }

        // project the builtin root before its public subpaths
        if specifier.starts_with("tspp:") {
            return self.package_completion("tspp:", specifier);
        }

        // project an external package root before its public subpaths
        let package_end = if specifier.starts_with('@') {
            let scope_end = specifier.find('/')?;
            specifier[scope_end + 1..]
                .find('/')
                .map_or(specifier.len(), |index| scope_end + index + 1)
        } else {
            specifier.find('/').unwrap_or(specifier.len())
        };
        let package = &specifier[..package_end];

        self.package_completion(package, specifier)
    }

    /// Project one package root or subpath.
    fn package_completion(
        &self,
        package: &str,
        specifier: &str,
    ) -> Option<(String, CompletionItemKind)> {
        // complete an unfinished package root as one lexical unit
        if package.starts_with(&self.text) {
            let is_module = package == specifier;
            let mut label = package.to_string();
            if !is_module && !label.ends_with(':') {
                label.push('/');
            }
            let kind = if is_module {
                CompletionItemKind::Module
            } else {
                CompletionItemKind::Folder
            };

            return Some((label, kind));
        }

        // complete only subpaths below the exact package root
        let subpath = self.text.strip_prefix(package)?;
        if !subpath.starts_with('/') && !(package.ends_with(':') && !subpath.is_empty()) {
            return None;
        }

        self.segment_completion(specifier)
    }

    /// Project one specifier through the current path directory.
    fn segment_completion(&self, specifier: &str) -> Option<(String, CompletionItemKind)> {
        let directory = &self.text[..self.segment_start];
        let remainder = specifier.strip_prefix(directory)?;
        if remainder.is_empty() {
            return None;
        }

        match remainder.split_once('/') {
            Some((segment, _)) => Some((format!("{segment}/"), CompletionItemKind::Folder)),
            None => Some((remainder.to_string(), CompletionItemKind::Module)),
        }
    }
}
