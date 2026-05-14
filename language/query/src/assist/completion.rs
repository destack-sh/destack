#![allow(clippy::too_many_arguments)]

use std::cmp::Ordering;
use std::collections::HashSet;
use std::sync::Arc;

use destack_dir as dir;
use destack_dir::{FloatType, IntegerType, Keyword, SymbolForm, SymbolSpace};
use destack_source::{Edit, File, FileId, FileType, Loader, ModuleId, PackageId, Uri};
use destack_workspace::{Module, Repository, Revision};
use serde::{Deserialize, Serialize};

use super::{CompletionContext, CompletionInput, CursorToken, completion_input_at_offset};
use crate::core::{
    ImportSortKey, MatchKind, MatchQuality, QueryContext, import_sort_key, import_sort_text,
    match_quality, query_context, query_context_for_profile, repository_import_relevance,
};
use crate::dir::{
    ImportEditSpace, MemberInfo, MemberKind, MemberName, build_import_display_path,
    build_import_edits, doc_text_for_symbol, get_canonical_symbol,
    matches_import_clause_space_filter, matches_symbol_space_filter, module_name_from_path,
    parameter_names_for_symbol, resolve_extension_members_for_symbol, resolve_reference_members,
    resolve_type_members, search_importable_symbols, visible_symbols,
};
use crate::format::format_local_type;
use crate::source::{current_initializer_binding_names, get_module_by_file_id};
// sort order priorities: lower = higher priority in completion list
const SORT_LOCAL_SYMBOL: u32 = 10;
const SORT_BUILTIN: u32 = 20;
const SORT_DEFAULT: u32 = 100;
const SORT_KEYWORD: u32 = 700;

/// Kind of completion item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompletionKind {
    Text,
    Method,
    Function,
    Constructor,
    Field,
    Variable,
    Class,
    Interface,
    Module,
    Property,
    Unit,
    Value,
    Enum,
    Keyword,
    Snippet,
    Color,
    File,
    Reference,
    Folder,
    EnumMember,
    Constant,
    Struct,
    Event,
    Operator,
    TypeParameter,
}

/// The semantic origin bucket for one completion candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
enum CompletionOrigin {
    /// An uncategorized completion candidate.
    #[default]
    Unknown,
    /// A context-shaped completion, such as an expected object-literal field.
    Contextual,
    /// A local or in-scope semantic candidate.
    Local,
    /// A builtin or ambient candidate.
    Builtin,
    /// A candidate that requires a new import.
    AutoImport,
    /// A language keyword candidate.
    Keyword,
}

/// The callable and constructable shape of one completion value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
struct CompletionValueShape {
    /// Whether the completion can be called like one function.
    is_callable: bool,
    /// Whether the completion can be used as one constructor target.
    is_constructable: bool,
}

impl From<SymbolForm> for CompletionKind {
    /// Convert a symbol type into a completion kind.
    fn from(ty: SymbolForm) -> Self {
        match ty {
            SymbolForm::Variable => CompletionKind::Variable,
            SymbolForm::Class => CompletionKind::Class,
            SymbolForm::Struct => CompletionKind::Struct,
            SymbolForm::Interface => CompletionKind::Interface,
            SymbolForm::Enum => CompletionKind::Enum,
            SymbolForm::Function => CompletionKind::Function,
            SymbolForm::Extension => CompletionKind::Class,
            SymbolForm::TypeAlias => CompletionKind::TypeParameter,
            SymbolForm::Newtype => CompletionKind::TypeParameter,
        }
    }
}

/// A completion item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Completion {
    /// The label shown in the completion list.
    pub label: String,
    /// The kind of completion.
    pub kind: CompletionKind,
    /// Detail shown alongside the label.
    pub detail: Option<String>,
    /// Documentation for the item.
    pub documentation: Option<String>,
    /// Text to insert when selected if different from the label.
    pub insert_text: Option<String>,
    /// Whether the insert text is one snippet.
    pub is_snippet: bool,
    /// Sort priority: lower = higher priority.
    pub sort_order: u32,
    /// Sort text for LSP if different from the label.
    pub sort_text: Option<String>,
    /// Whether to preselect this item.
    pub preselect: bool,
    /// Whether the item is deprecated.
    pub deprecated: bool,
    /// Additional text edits to apply, for example auto imports.
    pub additional_text_edits: Vec<Edit>,
    /// Whether this completion inserts one auto import.
    #[serde(default)]
    pub is_auto_import: bool,
    /// Matched character positions in the label.
    pub match_positions: Vec<usize>,
    /// The semantic origin bucket for ranking.
    #[serde(skip)]
    origin: CompletionOrigin,
    /// The structured import ordering key for ranking.
    #[serde(skip)]
    import_sort_key: Option<ImportSortKey>,
    /// Whether this member comes from one extension lookup.
    #[serde(skip)]
    is_extension_member: bool,
    /// The direct nominal type symbol for semantic ranking when available.
    #[serde(skip)]
    type_symbol: Option<dir::GlobalSymbolId>,
    /// Related nominal type symbols for semantic ranking.
    #[serde(skip)]
    type_symbols: Vec<dir::GlobalSymbolId>,
    /// The callable and constructable shape for semantic ranking.
    #[serde(skip)]
    value_shape: CompletionValueShape,
}

impl Completion {
    /// Create a simple completion.
    pub fn new(label: impl Into<String>, kind: CompletionKind) -> Self {
        Self {
            label: label.into(),
            kind,
            detail: None,
            documentation: None,
            insert_text: None,
            is_snippet: false,
            sort_order: SORT_DEFAULT,
            sort_text: None,
            preselect: false,
            deprecated: false,
            additional_text_edits: Vec::new(),
            is_auto_import: false,
            match_positions: Vec::new(),
            origin: CompletionOrigin::Unknown,
            import_sort_key: None,
            is_extension_member: false,
            type_symbol: None,
            type_symbols: Vec::new(),
            value_shape: CompletionValueShape::default(),
        }
    }

    /// Set the detail text.
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Set the documentation.
    pub fn with_documentation(mut self, doc: impl Into<String>) -> Self {
        self.documentation = Some(doc.into());
        self
    }

    /// Set the insert text.
    pub fn with_insert_text(mut self, text: impl Into<String>) -> Self {
        self.insert_text = Some(text.into());
        self
    }

    /// Mark the item as one snippet.
    pub fn as_snippet(mut self) -> Self {
        self.is_snippet = true;
        self
    }

    /// Mark the item as contextual.
    pub fn as_contextual(mut self) -> Self {
        self.origin = CompletionOrigin::Contextual;
        self
    }

    /// Mark the item as local.
    pub fn as_local(mut self) -> Self {
        self.origin = CompletionOrigin::Local;
        self
    }

    /// Mark the item as builtin.
    pub fn as_builtin(mut self) -> Self {
        self.origin = CompletionOrigin::Builtin;
        self
    }

    /// Mark the item as one auto import.
    pub fn as_auto_import(mut self) -> Self {
        self.is_auto_import = true;
        self.origin = CompletionOrigin::AutoImport;
        self
    }

    /// Set the sort order.
    pub fn with_sort_order(mut self, order: u32) -> Self {
        self.sort_order = order;
        self
    }

    /// Mark the item as preselected.
    pub fn preselected(mut self) -> Self {
        self.preselect = true;
        self
    }

    /// Add additional edits.
    pub fn with_additional_edits(mut self, edits: Vec<Edit>) -> Self {
        self.additional_text_edits = edits;
        self
    }

    /// Mark the item as deprecated.
    pub fn deprecated(mut self) -> Self {
        self.deprecated = true;
        self
    }

    /// Mark the item as one keyword completion.
    pub fn as_keyword(mut self) -> Self {
        self.origin = CompletionOrigin::Keyword;
        self
    }

    /// Set the sort text.
    pub fn with_sort_text(mut self, text: impl Into<String>) -> Self {
        self.sort_text = Some(text.into());
        self
    }

    /// Set the structured import sort key.
    fn with_import_sort_key(mut self, key: ImportSortKey) -> Self {
        self.import_sort_key = Some(key);
        self
    }

    /// Mark the item as one extension member.
    fn with_extension_member(mut self) -> Self {
        self.is_extension_member = true;
        self
    }

    /// Set the direct nominal type symbol.
    fn with_type_symbol(mut self, symbol_id: dir::GlobalSymbolId) -> Self {
        self.type_symbol = Some(symbol_id);
        self
    }

    /// Set the related nominal type symbols.
    fn with_type_symbols(mut self, symbols: Vec<dir::GlobalSymbolId>) -> Self {
        self.type_symbols = symbols;
        self
    }

    /// Set the callable and constructable value shape.
    fn with_value_shape(mut self, value_shape: CompletionValueShape) -> Self {
        self.value_shape = value_shape;
        self
    }

    /// Return the stable ordering text for one completion.
    fn ordering_text(&self) -> &str {
        self.sort_text.as_deref().unwrap_or(&self.label)
    }
}

/// Trigger character that caused the completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompletionTrigger {
    /// Invoked manually or automatically.
    Invoked,
    /// Triggered by one character, for example `.`.
    Character(char),
    /// Retriggered for incomplete results.
    Incomplete,
}

/// Request completion items at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// The document URI.
    pub uri: Uri,
    /// The byte offset in the document.
    pub offset: u32,
    /// The trigger that initiated completion.
    pub trigger: CompletionTrigger,
    /// Whether to include auto import completions.
    pub include_imports: bool,
}

/// Response payload for completion queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// Completion items.
    pub items: Vec<Completion>,
    /// Whether the results are incomplete.
    pub is_incomplete: bool,
}

/// The repository-bound builder for completion candidates.
struct CompletionBuilder<'a> {
    /// The shared workspace repository.
    repository: &'a Repository,
    /// The current revision.
    revision: Revision,
    /// The current file being completed.
    file_id: FileId,
}

impl<'a> CompletionBuilder<'a> {
    /// Build one completion builder.
    fn new(repository: &'a Repository, revision: Revision, file_id: FileId) -> Self {
        Self {
            repository,
            revision,
            file_id,
        }
    }

    /// Return the current file snapshot.
    fn source_file(&self) -> Option<Arc<File>> {
        self.repository
            .file(self.revision, self.file_id)
            .ok()
            .flatten()
    }

    /// Return the current module snapshot for the focused file.
    fn current_module(&self) -> Option<Arc<Module>> {
        get_module_by_file_id(self.repository, self.revision, self.file_id)
    }

    /// Build one query context for the focused file.
    fn current_query_context(&self) -> Option<QueryContext> {
        let module = self.current_module()?;
        query_context(self.repository, self.revision, module.id)
    }

    /// Build one query context for one module id.
    fn query_context_for_module(&self, module_id: ModuleId) -> Option<QueryContext> {
        query_context(self.repository, self.revision, module_id)
    }

    /// Collect the raw completion candidates for one context.
    fn completion_candidates(
        &self,
        offset: u32,
        trigger: CompletionTrigger,
        context: &CompletionContext,
        token: Option<&CursorToken>,
    ) -> Vec<Completion> {
        // collect query-shaping inputs once up front
        let excluded_labels = if context.uses_initializer_exclusions() {
            self.completion_excluded_labels(offset)
        } else {
            HashSet::new()
        };
        let prefix = token.map(|token| token.text.as_str()).unwrap_or("");
        let allow_short_prefix = matches!(trigger, CompletionTrigger::Invoked);

        // dispatch the primary context-specific candidate builder
        let mut results = match context {
            CompletionContext::MemberAccess {
                receiver_node: _,
                receiver_symbol,
                receiver_type,
            } => self.complete_members(*receiver_type, *receiver_symbol),
            CompletionContext::TypePosition {
                scope_id,
                scope_mark,
            } => self.complete_types(*scope_id, *scope_mark),
            CompletionContext::ValuePosition {
                scope_id,
                scope_mark,
            } => self.complete_values(*scope_id, *scope_mark, &excluded_labels, false),
            CompletionContext::StatementPosition {
                scope_id,
                scope_mark,
            } => self.complete_values(
                *scope_id,
                *scope_mark,
                &excluded_labels,
                matches!(trigger, CompletionTrigger::Invoked),
            ),
            CompletionContext::ObjectLiteral {
                contextual_type,
                existing_fields,
                scope_id,
                scope_mark,
                ..
            } => self.complete_object_literal(
                *contextual_type,
                existing_fields,
                *scope_id,
                *scope_mark,
                &excluded_labels,
            ),
            CompletionContext::ObjectLiteralValue {
                scope_id,
                scope_mark,
            } => self.complete_values(*scope_id, *scope_mark, &excluded_labels, false),
            CompletionContext::CallArgument {
                scope_id,
                scope_mark,
                ..
            } => self.complete_values(*scope_id, *scope_mark, &excluded_labels, false),
            CompletionContext::NewExpression {
                scope_id,
                scope_mark,
            } => self.complete_new_expression(*scope_id, *scope_mark, &excluded_labels),
            CompletionContext::ImportPath { partial_path } => {
                self.complete_import_paths(partial_path)
            }
            CompletionContext::ImportClause {
                target_module,
                existing_names,
                space_filter,
            } => self.complete_imports(*target_module, existing_names, *space_filter),
            CompletionContext::Suppressed => Vec::new(),
            CompletionContext::Unknown => {
                Self::complete_all(matches!(trigger, CompletionTrigger::Invoked))
            }
        };

        // layer in auto imports when this context supports them
        if let Some((space_filter, scope_id, scope_mark, constructable_only)) =
            context.auto_import_settings()
        {
            let mut auto_imports = self.complete_auto_imports_with_visibility(
                prefix,
                Some(space_filter),
                scope_id,
                scope_mark,
                &excluded_labels,
                allow_short_prefix,
            );

            if constructable_only {
                auto_imports.retain(is_constructable_completion);
            }

            results.extend(auto_imports);
        }

        results
    }

    /// Collect completion labels excluded at one cursor offset.
    fn completion_excluded_labels(&self, offset: u32) -> HashSet<String> {
        let Some(source_file) = self.source_file() else {
            return HashSet::new();
        };
        let source = source_file.text();

        let Some(ctx) = self.current_query_context() else {
            return HashSet::new();
        };

        current_initializer_binding_names(ctx.source(), source, offset)
            .into_iter()
            .map(|name| ctx.source().strings().get(name).to_string())
            .collect()
    }

    /// Resolve one callable and constructable value shape from one symbol.
    fn value_shape_for_symbol(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<CompletionValueShape> {
        let ctx = self.query_context_for_module(symbol_id.module_id)?;
        let types = ctx.dir().types();
        let symbols = ctx.dir().symbols();
        let type_id = types.symbol_type_id(symbols, symbol_id)?;

        Some(Self::value_shape_for_type(types, type_id))
    }

    /// Resolve one callable and constructable value shape from one type id.
    fn value_shape_for_type(
        types: &dir::TypeTable,
        type_id: dir::LocalTypeId,
    ) -> CompletionValueShape {
        match types.get_type(type_id) {
            dir::Type::Function(_) => CompletionValueShape {
                is_callable: true,
                is_constructable: false,
            },
            dir::Type::Object(object) => CompletionValueShape {
                is_callable: !object.call_signatures.is_empty(),
                is_constructable: !object.construct_signatures.is_empty(),
            },
            _ => CompletionValueShape::default(),
        }
    }

    /// Return one canonical nominal type symbol from one symbol.
    fn type_symbol_for_symbol(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalSymbolId> {
        let ctx = self.query_context_for_module(symbol_id.module_id)?;
        let types = ctx.dir().types();
        let symbols = ctx.dir().symbols();
        let type_id = types.symbol_type_id(symbols, symbol_id)?;

        self.type_symbol_for_type(types, type_id)
    }

    /// Return one canonical nominal type symbol from one type id.
    fn type_symbol_for_type(
        &self,
        types: &dir::TypeTable,
        type_id: dir::LocalTypeId,
    ) -> Option<dir::GlobalSymbolId> {
        let symbol_id = types.get_type(type_id).symbol()?;

        Some(get_canonical_symbol(
            self.repository,
            self.revision,
            symbol_id,
        ))
    }

    /// Return related nominal type symbols from one symbol.
    fn type_symbols_for_symbol(&self, symbol_id: dir::GlobalSymbolId) -> Vec<dir::GlobalSymbolId> {
        let Some(ctx) = self.query_context_for_module(symbol_id.module_id) else {
            return Vec::new();
        };
        let types = ctx.dir().types();
        let symbols = ctx.dir().symbols();
        let Some(type_id) = types.symbol_type_id(symbols, symbol_id) else {
            return Vec::new();
        };

        self.type_symbols_for_type(types, type_id)
    }

    /// Return related nominal type symbols from one type id.
    fn type_symbols_for_type(
        &self,
        types: &dir::TypeTable,
        type_id: dir::LocalTypeId,
    ) -> Vec<dir::GlobalSymbolId> {
        let mut symbols = Vec::new();
        let mut seen_types = HashSet::new();
        let mut seen_symbols = HashSet::new();

        self.collect_type_symbols(
            types,
            type_id,
            &mut seen_types,
            &mut seen_symbols,
            &mut symbols,
        );

        symbols
    }

    /// Collect related nominal type symbols from one type.
    fn collect_type_symbols(
        &self,
        types: &dir::TypeTable,
        type_id: dir::LocalTypeId,
        seen_types: &mut HashSet<dir::LocalTypeId>,
        seen_symbols: &mut HashSet<dir::GlobalSymbolId>,
        symbols: &mut Vec<dir::GlobalSymbolId>,
    ) {
        let type_id = types.unwrap_value_type_id(type_id);
        if !seen_types.insert(type_id) {
            return;
        }

        let ty = types.get_type(type_id);

        if let dir::Type::Reference(reference) = ty {
            let symbol = reference.symbol;
            let canonical_symbol = get_canonical_symbol(self.repository, self.revision, symbol);
            if seen_symbols.insert(canonical_symbol) {
                symbols.push(canonical_symbol);
            }

            if let Some(target_type_id) = types.get_alias_target_type_id(symbol) {
                self.collect_type_symbols(types, target_type_id, seen_types, seen_symbols, symbols);
            }

            return;
        }

        match ty {
            dir::Type::Union(union) => {
                for &element_id in &union.elements {
                    self.collect_type_symbols(types, element_id, seen_types, seen_symbols, symbols);
                }
            }
            dir::Type::Intersection(intersection) => {
                for &element_id in &intersection.elements {
                    self.collect_type_symbols(types, element_id, seen_types, seen_symbols, symbols);
                }
            }
            dir::Type::Value(value) => {
                self.collect_type_symbols(types, value.value, seen_types, seen_symbols, symbols);
            }
            _ => {}
        }
    }

    /// Format a type detail string for a symbol's declared or inferred type.
    fn format_symbol_form_detail(&self, symbol_id: dir::GlobalSymbolId) -> Option<String> {
        let ctx = self.query_context_for_module(symbol_id.module_id)?;

        let symbols = ctx.dir().symbols();
        let types = ctx.dir().types();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        let declaration = symbol.declaration?;
        let type_id = types.get_declared_or_inferred_type_id(declaration)?;

        Some(format_local_type(
            type_id,
            types,
            self.repository,
            self.revision,
            ctx.dir().strings(),
        ))
    }

    /// Attach documentation to a completion when the backing symbol has docs.
    fn attach_completion_documentation(
        &self,
        completion: Completion,
        symbol_id: dir::GlobalSymbolId,
    ) -> Completion {
        let symbol_id = get_canonical_symbol(self.repository, self.revision, symbol_id);
        let documentation = doc_text_for_symbol(self.repository, self.revision, symbol_id);
        let Some(documentation) = documentation else {
            return completion;
        };

        completion.with_documentation(documentation)
    }

    /// Build a completion item from a resolved member entry.
    fn completion_for_member(
        &self,
        member: MemberInfo,
        types: Option<&dir::TypeTable>,
        is_extension_member: bool,
    ) -> Option<Completion> {
        // only string-named members become surface completions
        let MemberName::String(name) = member.name else {
            return None;
        };

        // seed the completion from the coarse member shape
        let kind = match member.kind {
            MemberKind::Method => CompletionKind::Method,
            MemberKind::Field => CompletionKind::Field,
            MemberKind::CallSignature => CompletionKind::Function,
            MemberKind::ConstructSignature => CompletionKind::Constructor,
            MemberKind::EnumMember => CompletionKind::EnumMember,
        };

        let mut completion = Completion::new(name, kind)
            .with_sort_order(SORT_LOCAL_SYMBOL)
            .as_local();

        if is_extension_member {
            completion = completion.with_extension_member();
        }

        // enrich from direct type information when this path has it
        if let Some(member_type_id) = member.type_id {
            if let Some(types) = types {
                completion =
                    completion.with_value_shape(Self::value_shape_for_type(types, member_type_id));

                if let Some(type_symbol) = self.type_symbol_for_type(types, member_type_id) {
                    completion = completion.with_type_symbol(type_symbol);
                }

                completion =
                    completion.with_type_symbols(self.type_symbols_for_type(types, member_type_id));

                if let Some(ctx) = self.current_query_context() {
                    let type_text = format_local_type(
                        member_type_id,
                        types,
                        self.repository,
                        self.revision,
                        ctx.dir().strings(),
                    );
                    completion = completion.with_detail(type_text);
                }
            }
        }
        // otherwise fall back to the backing symbol
        else if let Some(symbol_id) = member.symbol_id {
            if let Some(value_shape) = self.value_shape_for_symbol(symbol_id) {
                completion = completion.with_value_shape(value_shape);
            }

            if let Some(type_symbol) = self.type_symbol_for_symbol(symbol_id) {
                completion = completion.with_type_symbol(type_symbol);
            }

            completion = completion.with_type_symbols(self.type_symbols_for_symbol(symbol_id));

            if let Some(type_text) = self.format_symbol_form_detail(symbol_id) {
                completion = completion.with_detail(type_text);
            }
        }

        if let Some(symbol_id) = member.symbol_id {
            completion = self.attach_completion_documentation(completion, symbol_id);
        }

        // methods prefer callable insertion text
        if member.kind == MemberKind::Method
            && let Some(symbol_id) = member.symbol_id
            && let Some(param_names) =
                get_function_param_names(self.repository, self.revision, symbol_id)
        {
            let (snippet, is_snippet) = generate_call_snippet(&completion.label, &param_names);
            completion = completion.with_insert_text(snippet);
            if is_snippet {
                completion = completion.as_snippet();
            }
        } else if member.kind == MemberKind::Method {
            let label = completion.label.clone();
            completion = completion.with_insert_text(format!("{label}()"));
        }

        Some(completion)
    }

    /// Complete members of a type after `.`.
    fn complete_members(
        &self,
        receiver_type: Option<dir::LocalTypeId>,
        receiver_symbol: Option<dir::GlobalSymbolId>,
    ) -> Vec<Completion> {
        let mut results = Vec::new();

        // resolve the current module context once
        let Some(ctx) = self.current_query_context() else {
            return Vec::new();
        };
        let current_module_id = ctx.module_id();

        // prefer direct type members first
        if let Some(type_id) = receiver_type {
            let types = ctx.dir().types();
            let symbols = ctx.dir().symbols();
            let members = resolve_type_members(
                types,
                symbols,
                type_id,
                self.repository,
                self.revision,
                current_module_id,
            );

            for member in members {
                let Some(completion) = self.completion_for_member(member, Some(types), false)
                else {
                    continue;
                };

                results.push(completion);
            }

            if !results.is_empty() {
                return results;
            }
        }

        // then fall back to resolved reference members
        if let Some(symbol_id) = receiver_symbol {
            let members = resolve_reference_members(
                symbol_id,
                self.repository,
                self.revision,
                current_module_id,
            );
            for member in members {
                let Some(completion) = self.completion_for_member(member, None, false) else {
                    continue;
                };

                results.push(completion);
            }

            if !results.is_empty() {
                return results;
            }
        }

        // extension members are last and stay marked for ranking
        if let Some(symbol_id) = receiver_symbol {
            let extension_members = resolve_extension_members_for_symbol(
                self.repository,
                self.revision,
                symbol_id,
                current_module_id,
            );
            let mut seen_names: HashSet<String> = results
                .iter()
                .map(|completion| completion.label.clone())
                .collect();

            for member in extension_members {
                let Some(completion) = self.completion_for_member(member, None, true) else {
                    continue;
                };

                if !seen_names.insert(completion.label.clone()) {
                    continue;
                }

                results.push(completion);
            }
        }

        results
    }

    /// Complete fields inside an object literal.
    fn complete_object_literal(
        &self,
        contextual_type: Option<dir::LocalTypeId>,
        existing_fields: &[String],
        scope_id: Option<dir::LocalScopeId>,
        scope_mark: Option<dir::LocalScopeMark>,
        excluded_labels: &HashSet<String>,
    ) -> Vec<Completion> {
        let mut results = Vec::new();
        let mut seen_names = HashSet::new();

        // prefer contextual object fields when a target type exists
        if let Some(type_id) = contextual_type {
            let Some(ctx) = self.current_query_context() else {
                return results;
            };
            let types = ctx.dir().types();
            let symbols = ctx.dir().symbols();
            let current_module_id = ctx.module_id();
            let members = resolve_type_members(
                types,
                symbols,
                type_id,
                self.repository,
                self.revision,
                current_module_id,
            );

            for member in members {
                if member.kind != MemberKind::Field {
                    continue;
                }

                let MemberName::String(name) = member.name else {
                    continue;
                };

                if existing_fields.contains(&name) {
                    continue;
                }

                let mut completion = Completion::new(&name, CompletionKind::Field)
                    .with_insert_text(format!("{name}: $0"))
                    .as_snippet()
                    .with_sort_order(5)
                    .as_contextual();

                if let Some(member_type_id) = member.type_id
                    && let Some(ctx) = self.current_query_context()
                {
                    let type_text = format_local_type(
                        member_type_id,
                        types,
                        self.repository,
                        self.revision,
                        ctx.dir().strings(),
                    );
                    completion = completion.with_detail(type_text);
                }

                results.push(completion);
                seen_names.insert(name);
            }
        }

        // then offer visible value names for ad hoc object literals
        if let Some(scope_id) = scope_id {
            let Some(ctx) = self.current_query_context() else {
                return results;
            };
            let symbols = ctx.dir().symbols();
            let mark = scope_mark.unwrap_or(dir::LocalScopeMark::end());

            for visible in visible_symbols(symbols, scope_id, mark, Some(SymbolSpace::Value)) {
                let dir::StaticKey::Name(name_id) = visible.key else {
                    continue;
                };

                let name = ctx.dir().strings().get(name_id).to_string();
                if excluded_labels.contains(&name) {
                    continue;
                }

                if !seen_names.insert(name.clone()) {
                    continue;
                }

                if existing_fields.contains(&name) {
                    continue;
                }

                let kind = CompletionKind::from(visible.symbol.form);
                results.push(
                    Completion::new(name, kind)
                        .with_sort_order(SORT_BUILTIN)
                        .as_local(),
                );
            }
        }

        results
    }

    /// Complete types in type position.
    fn complete_types(
        &self,
        scope_id: Option<dir::LocalScopeId>,
        scope_mark: Option<dir::LocalScopeMark>,
    ) -> Vec<Completion> {
        // start from the current semantic module when possible
        let Some(ctx) = self.current_query_context() else {
            return primitive_type_completions();
        };
        let symbols = ctx.dir().symbols();
        let dir_tree = ctx.dir().view();

        let mut results = Vec::new();
        let mut seen_names = HashSet::new();

        // normalize void aliases through their canonical exported type
        let resolve_symbol_form = |symbol_id: dir::LocalSymbolId, symbol: &dir::Symbol| {
            if symbol.form != SymbolForm::Variable {
                return symbol.form;
            }

            let global_id = dir::GlobalSymbolId {
                module_id: ctx.module_id(),
                local_id: symbol_id,
            };
            let canonical_id = get_canonical_symbol(self.repository, self.revision, global_id);
            if canonical_id == global_id {
                return symbol.form;
            }

            let Some(ctx) = query_context_for_profile(
                self.repository,
                self.revision,
                canonical_id.module_id,
                ctx.profile_id(),
            ) else {
                return symbol.form;
            };
            let canonical_symbol = ctx.dir().symbols().get_symbol(canonical_id.local_id);
            canonical_symbol.form
        };

        let visible_scope_id = scope_id.unwrap_or(ctx.dir().namespace_scope());
        let mark = scope_mark.unwrap_or(dir::LocalScopeMark::end());

        // collect visible type-space names first
        for visible in visible_symbols(symbols, visible_scope_id, mark, Some(SymbolSpace::Type)) {
            let dir::StaticKey::Name(name_id) = visible.key else {
                continue;
            };

            let name = ctx.dir().strings().get(name_id).to_string();
            if !seen_names.insert(name.clone()) {
                continue;
            }

            let kind = CompletionKind::from(resolve_symbol_form(visible.id, visible.symbol));
            let completion = Completion::new(name, kind)
                .with_sort_order(SORT_LOCAL_SYMBOL)
                .as_local();
            let symbol_id = dir::GlobalSymbolId {
                module_id: ctx.module_id(),
                local_id: visible.id,
            };

            results.push(self.attach_completion_documentation(completion, symbol_id));
        }

        // then include exported dependency items in the same type space
        for (item_id, _) in dir_tree.iter_nodes_of_type::<dir::DependencyItem>() {
            let Some(symbol_id) = ctx.dir().symbol_for_node(item_id.into()) else {
                continue;
            };

            let symbol = symbols.get_symbol(symbol_id);
            if symbol.space != dir::SymbolSpace::Type {
                continue;
            }

            let Some(name_id) = symbol.name() else {
                continue;
            };

            let name = ctx.dir().strings().get(name_id).to_string();
            if !seen_names.insert(name.clone()) {
                continue;
            }

            let kind = CompletionKind::from(resolve_symbol_form(symbol_id, symbol));
            let completion = Completion::new(name, kind)
                .with_sort_order(SORT_LOCAL_SYMBOL)
                .as_local();
            let symbol_id = dir::GlobalSymbolId {
                module_id: ctx.module_id(),
                local_id: symbol_id,
            };

            results.push(self.attach_completion_documentation(completion, symbol_id));
        }

        // partial dir still falls back to source declarations for local types
        if results.is_empty() {
            for declaration_id in ctx.source().tree().iter_nodes::<dir::Declaration>() {
                let declaration = ctx.source().tree().get(declaration_id);
                let kind = match declaration {
                    dir::Declaration::Class { .. } => CompletionKind::Class,
                    dir::Declaration::Struct { .. } => CompletionKind::Struct,
                    dir::Declaration::Interface { .. } => CompletionKind::Interface,
                    dir::Declaration::Enum { .. } => CompletionKind::Enum,
                    dir::Declaration::Type { .. } => CompletionKind::TypeParameter,
                    _ => continue,
                };

                let Some(name) = declaration.name() else {
                    continue;
                };
                let name = ctx.source().strings().get(name.string()).to_string();
                if !seen_names.insert(name.clone()) {
                    continue;
                }
                results.push(
                    Completion::new(name, kind)
                        .with_sort_order(SORT_LOCAL_SYMBOL)
                        .as_local(),
                );
            }
        }

        // primitive types are always available
        results.extend(primitive_type_completions());

        results
    }

    /// Complete values in expression position.
    fn complete_values(
        &self,
        scope_id: Option<dir::LocalScopeId>,
        scope_mark: Option<dir::LocalScopeMark>,
        excluded_labels: &HashSet<String>,
        include_keywords: bool,
    ) -> Vec<Completion> {
        // degraded contexts fall back to plain keyword completions
        let Some(ctx) = self.current_query_context() else {
            return keyword_completions();
        };
        let module_id = ctx.module_id();

        let mut results = Vec::new();

        // snapshot the visible value-space symbols before building completions
        let symbols_to_process: Vec<(dir::LocalSymbolId, String, SymbolForm)> = {
            let symbols = ctx.dir().symbols();
            let mut symbols_to_process = Vec::new();
            let mut seen_names = HashSet::new();
            let scope_id = scope_id.unwrap_or(ctx.dir().namespace_scope());
            let mark = scope_mark.unwrap_or(dir::LocalScopeMark::end());

            for visible in visible_symbols(symbols, scope_id, mark, Some(SymbolSpace::Value)) {
                let dir::StaticKey::Name(name_id) = visible.key else {
                    continue;
                };

                let name = ctx.dir().strings().get(name_id).to_string();
                if excluded_labels.contains(&name) {
                    continue;
                }

                if !seen_names.insert(name.clone()) {
                    continue;
                }

                symbols_to_process.push((visible.id, name, visible.symbol.form));
            }

            symbols_to_process
        };

        // build one completion per visible symbol
        for (local_id, name, symbol_form) in symbols_to_process {
            let kind = CompletionKind::from(symbol_form);
            let mut completion = Completion::new(&name, kind)
                .with_sort_order(SORT_LOCAL_SYMBOL)
                .as_local();
            let symbol_id = dir::GlobalSymbolId {
                module_id,
                local_id,
            };

            if let Some(value_shape) = self.value_shape_for_symbol(symbol_id) {
                completion = completion.with_value_shape(value_shape);
            }

            if let Some(type_symbol) = self.type_symbol_for_symbol(symbol_id) {
                completion = completion.with_type_symbol(type_symbol);
            }
            completion = completion.with_type_symbols(self.type_symbols_for_symbol(symbol_id));

            if symbol_form == SymbolForm::Function
                && let Some(param_names) =
                    get_function_param_names(self.repository, self.revision, symbol_id)
            {
                let (snippet, is_snippet) = generate_call_snippet(&name, &param_names);
                completion = completion.with_insert_text(snippet);
                if is_snippet {
                    completion = completion.as_snippet();
                }
            }

            completion = self.attach_completion_documentation(completion, symbol_id);
            results.push(completion);
        }

        // statement contexts can opt into keyword completions as well
        if include_keywords {
            results.extend(keyword_completions());
        }

        results
    }

    /// Complete constructable symbols for a new expression.
    fn complete_new_expression(
        &self,
        scope_id: Option<dir::LocalScopeId>,
        scope_mark: Option<dir::LocalScopeMark>,
        excluded_labels: &HashSet<String>,
    ) -> Vec<Completion> {
        // degraded contexts cannot classify constructable values
        let Some(ctx) = self.current_query_context() else {
            return Vec::new();
        };
        let symbols = ctx.dir().symbols();

        let mut results = Vec::new();
        let mut seen = HashSet::new();

        // prefer visible constructable names from the active scope
        if let Some(scope_id) = scope_id {
            let mark = scope_mark.unwrap_or(dir::LocalScopeMark::end());

            for visible in visible_symbols(symbols, scope_id, mark, None) {
                if !is_constructable_symbol(visible.symbol.form) {
                    continue;
                }

                let dir::StaticKey::Name(name_id) = visible.key else {
                    continue;
                };

                let name = ctx.dir().strings().get(name_id).to_string();
                if excluded_labels.contains(&name) {
                    continue;
                }

                if !seen.insert(name.clone()) {
                    continue;
                }

                let kind = CompletionKind::from(visible.symbol.form);
                let completion = Completion::new(name, kind)
                    .with_sort_order(SORT_LOCAL_SYMBOL)
                    .as_local();
                let symbol_id = dir::GlobalSymbolId {
                    module_id: ctx.module_id(),
                    local_id: visible.id,
                };
                let completion = if let Some(value_shape) = self.value_shape_for_symbol(symbol_id) {
                    completion.with_value_shape(value_shape)
                } else {
                    completion
                };
                let completion = if let Some(type_symbol) = self.type_symbol_for_symbol(symbol_id) {
                    completion.with_type_symbol(type_symbol)
                } else {
                    completion
                };
                let completion =
                    completion.with_type_symbols(self.type_symbols_for_symbol(symbol_id));
                let completion = self.attach_completion_documentation(completion, symbol_id);

                results.push(completion);
            }
        }

        // fall back to all active constructable symbols when scope lookup is empty
        if results.is_empty() {
            for symbol_id in symbols.symbol_ids() {
                let symbol = symbols.get_symbol(symbol_id);
                if !is_constructable_symbol(symbol.form) {
                    continue;
                }

                let Some(name_id) = symbol.name() else {
                    continue;
                };
                let name = ctx.dir().strings().get(name_id).to_string();
                if excluded_labels.contains(&name) {
                    continue;
                }

                if !seen.insert(name.clone()) {
                    continue;
                }

                let kind = CompletionKind::from(symbol.form);
                let completion = Completion::new(name, kind)
                    .with_sort_order(SORT_LOCAL_SYMBOL)
                    .as_local();
                let symbol_id = dir::GlobalSymbolId {
                    module_id: ctx.module_id(),
                    local_id: symbol_id,
                };
                let completion = if let Some(value_shape) = self.value_shape_for_symbol(symbol_id) {
                    completion.with_value_shape(value_shape)
                } else {
                    completion
                };
                let completion = if let Some(type_symbol) = self.type_symbol_for_symbol(symbol_id) {
                    completion.with_type_symbol(type_symbol)
                } else {
                    completion
                };
                let completion =
                    completion.with_type_symbols(self.type_symbols_for_symbol(symbol_id));
                let completion = self.attach_completion_documentation(completion, symbol_id);

                results.push(completion);
            }
        }

        results
    }

    /// Generate auto import completions for a prefix using indexed lookup.
    fn complete_auto_imports_with_visibility(
        &self,
        prefix: &str,
        space_filter: Option<SymbolSpace>,
        scope_id: Option<dir::LocalScopeId>,
        scope_mark: Option<dir::LocalScopeMark>,
        excluded_labels: &HashSet<String>,
        allow_short_prefix: bool,
    ) -> Vec<Completion> {
        let mut completions = self.complete_auto_imports(prefix, space_filter, allow_short_prefix);

        if let Some(visible_names) = self.collect_visible_names(scope_id, scope_mark, space_filter)
        {
            completions.retain(|item| !visible_names.contains(item.label.as_str()));
        }

        if !excluded_labels.is_empty() {
            completions.retain(|item| !excluded_labels.contains(&item.label));
        }

        apply_short_prefix_auto_import_limit(&mut completions, prefix, allow_short_prefix);

        completions
    }

    /// Generate auto import completions for one prefix.
    fn complete_auto_imports(
        &self,
        prefix: &str,
        space_filter: Option<SymbolSpace>,
        allow_short_prefix: bool,
    ) -> Vec<Completion> {
        // empty and very short prefixes do not earn import search
        if prefix.is_empty() {
            return Vec::new();
        }

        if prefix.len() < AUTO_IMPORT_MIN_PREFIX && !allow_short_prefix {
            return Vec::new();
        }

        let Some(module) = self.current_module() else {
            return Vec::new();
        };
        let current_module_id = Some(module.id);
        let current_package_id = Some(module.package_id);

        let mut results = Vec::new();
        let mut seen: HashSet<(ModuleId, dir::LocalSymbolId)> = HashSet::new();
        let exports =
            search_importable_symbols(self.repository, self.revision, prefix, current_module_id);

        // turn indexed export matches into importable completions
        for export in exports {
            if !matches_symbol_space_filter(export.kind, export.space, space_filter) {
                continue;
            }

            let Some(module_path) = &export.module_path else {
                continue;
            };

            let key = (export.module_id, export.local_id);
            if !seen.insert(key) {
                continue;
            }

            let import_space = ImportEditSpace::for_auto_import(space_filter, export.space);

            self.push_auto_import_completion(
                current_package_id,
                prefix,
                export.module_id,
                export.local_id,
                module_path,
                &export.name,
                export.kind,
                space_filter,
                export.space,
                import_space,
                &mut results,
            );
        }

        results
    }

    /// Collect visible symbol names for a scope and space.
    fn collect_visible_names(
        &self,
        scope_id: Option<dir::LocalScopeId>,
        scope_mark: Option<dir::LocalScopeMark>,
        space_filter: Option<SymbolSpace>,
    ) -> Option<HashSet<String>> {
        let ctx = self.current_query_context()?;
        let symbols = ctx.dir().symbols();
        let scope_id = scope_id.unwrap_or(ctx.dir().namespace_scope());
        let mark = scope_mark.unwrap_or(dir::LocalScopeMark::end());

        let mut names = HashSet::new();
        for visible in visible_symbols(symbols, scope_id, mark, space_filter) {
            let dir::StaticKey::Name(name_id) = visible.key else {
                continue;
            };

            names.insert(ctx.dir().strings().get(name_id).to_string());
        }

        Some(names)
    }

    /// Push one auto import completion into the results list.
    fn push_auto_import_completion(
        &self,
        current_package_id: Option<PackageId>,
        prefix: &str,
        module_id: ModuleId,
        local_id: dir::LocalSymbolId,
        module_path: &str,
        export_name: &str,
        symbol_form: SymbolForm,
        expected_space: Option<SymbolSpace>,
        symbol_space: SymbolSpace,
        import_space: ImportEditSpace,
        results: &mut Vec<Completion>,
    ) {
        let display_path =
            build_import_display_path(self.repository, self.revision, self.file_id, module_path);
        let import_edits = build_import_edits(
            self.repository,
            self.revision,
            self.file_id,
            export_name,
            &display_path,
            import_space,
        );
        if import_edits.is_empty() {
            return;
        }

        let Some(relevance) = repository_import_relevance(
            self.repository,
            self.revision,
            self.file_id,
            current_package_id,
            prefix,
            export_name,
            expected_space,
            symbol_space,
            module_id,
            module_path,
        ) else {
            return;
        };

        let sort_text = import_sort_text(&relevance, &display_path, export_name);
        let import_sort_key = import_sort_key(&relevance, &display_path, export_name);
        let kind = CompletionKind::from(symbol_form);
        let detail = format!("Auto import from {display_path}");
        let mut completion = Completion::new(export_name, kind)
            .with_detail(detail)
            .with_sort_order(SORT_DEFAULT)
            .with_import_sort_key(import_sort_key)
            .with_sort_text(sort_text)
            .with_additional_edits(import_edits)
            .with_value_shape(CompletionValueShape::for_symbol_form(symbol_form))
            .as_auto_import();

        let symbol_id = dir::GlobalSymbolId {
            module_id,
            local_id,
        };
        if let Some(type_symbol) = self.type_symbol_for_symbol(symbol_id) {
            completion = completion.with_type_symbol(type_symbol);
        }
        completion = completion.with_type_symbols(self.type_symbols_for_symbol(symbol_id));

        results.push(completion);
    }

    /// Complete imports from one module.
    fn complete_imports(
        &self,
        target_module: Option<ModuleId>,
        existing_names: &[String],
        space_filter: Option<SymbolSpace>,
    ) -> Vec<Completion> {
        let Some(module_id) = target_module else {
            return Vec::new();
        };

        let Some(ctx) = self.query_context_for_module(module_id) else {
            return Vec::new();
        };
        let symbols = ctx.dir().symbols();
        let existing_names: HashSet<&str> = existing_names.iter().map(String::as_str).collect();
        let mut results = Vec::new();

        for symbol in symbols.symbols() {
            if symbol.export_kind.is_none() {
                continue;
            }

            let Some(string_id) = symbol.name() else {
                continue;
            };

            if !matches_import_clause_space_filter(symbol.form, symbol.space, space_filter) {
                continue;
            }

            let name = ctx.dir().strings().get(string_id).to_string();
            if existing_names.contains(name.as_str()) {
                continue;
            }

            let kind = CompletionKind::from(symbol.form);
            results.push(
                Completion::new(name, kind)
                    .with_sort_order(SORT_LOCAL_SYMBOL)
                    .as_contextual(),
            );
        }

        results
    }

    /// Complete import paths, relative paths, or package names.
    fn complete_import_paths(&self, partial: &str) -> Vec<Completion> {
        let mut results = Vec::new();

        if partial.starts_with("./") || partial.starts_with("../") {
            let path = self
                .source_file()
                .and_then(|source_file| source_file.path.clone());
            let path = path.as_ref();
            if let Some(path) = path
                && let Some(base_dir) = path.parent()
            {
                results.extend(complete_relative_path(self.repository, base_dir, partial));
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
            results.extend(complete_package_names(self.repository, self.revision, ""));
        } else {
            results.extend(complete_package_names(
                self.repository,
                self.revision,
                partial,
            ));
        }

        results
    }

    /// Complete all symbols for an unknown context.
    fn complete_all(include_keywords: bool) -> Vec<Completion> {
        if include_keywords {
            keyword_completions()
        } else {
            Vec::new()
        }
    }
}

/// Get completions at the given position.
pub fn completions(
    repository: &Repository,
    revision: Revision,
    file: FileId,
    offset: u32,
    trigger: CompletionTrigger,
) -> Vec<Completion> {
    let CompletionInput { context, token } =
        completion_input_at_offset(repository, revision, file, offset);
    let builder = CompletionBuilder::new(repository, revision, file);
    let completions = builder.completion_candidates(offset, trigger, &context, token.as_ref());

    filter_and_rank_completions(completions, &context, token.as_ref())
}

// auto import completion thresholds
const AUTO_IMPORT_MIN_PREFIX: usize = 2;
const AUTO_IMPORT_SHORT_PREFIX_LIMIT: usize = 50;

/// Generate a snippet for a function call with parameter placeholders.
///
/// Returns `(snippet_text, is_snippet)`.
/// For no params: `"foo()"` with `is_snippet = false`.
/// For params: `"foo(${1:param1}, ${2:param2})$0"` with `is_snippet = true`.
fn generate_call_snippet(name: &str, param_names: &[String]) -> (String, bool) {
    if param_names.is_empty() {
        (format!("{name}()"), false)
    } else {
        let params = param_names
            .iter()
            .enumerate()
            .map(|(index, name)| format!("${{{}:{}}}", index + 1, name))
            .collect::<Vec<_>>()
            .join(", ");

        (format!("{name}({params})$0"), true)
    }
}

/// Get parameter names from a function symbol declaration.
fn get_function_param_names(
    repository: &Repository,
    revision: Revision,
    symbol_id: dir::GlobalSymbolId,
) -> Option<Vec<String>> {
    let param_names = parameter_names_for_symbol(repository, revision, symbol_id)?;

    if param_names.is_empty() {
        return None;
    }

    Some(param_names)
}

/// Apply short-prefix pruning after visibility filtering.
fn apply_short_prefix_auto_import_limit(
    completions: &mut Vec<Completion>,
    prefix: &str,
    allow_short_prefix: bool,
) {
    if !allow_short_prefix || prefix.len() >= AUTO_IMPORT_MIN_PREFIX {
        return;
    }

    sort_auto_imports_for_short_prefix(completions, prefix);
    completions.truncate(AUTO_IMPORT_SHORT_PREFIX_LIMIT);
}

/// Sort auto import completions for short prefixes.
fn sort_auto_imports_for_short_prefix(completions: &mut [Completion], prefix: &str) {
    completions.sort_by(|left, right| {
        let left_lexical = match_quality(&left.label, prefix);
        let right_lexical = match_quality(&right.label, prefix);

        let left_key = (
            left_lexical.as_ref().map(MatchQuality::sort_key),
            left.import_sort_key.as_ref(),
            left.ordering_text(),
            left.label.as_str(),
        );
        let right_key = (
            right_lexical.as_ref().map(MatchQuality::sort_key),
            right.import_sort_key.as_ref(),
            right.ordering_text(),
            right.label.as_str(),
        );

        left_key.cmp(&right_key)
    });
}

/// Primitive type completions.
fn primitive_type_completions() -> Vec<Completion> {
    let mut names = vec![
        "any".to_string(),
        "unknown".to_string(),
        "never".to_string(),
        "void".to_string(),
        "null".to_string(),
        "undefined".to_string(),
        "object".to_string(),
        "boolean".to_string(),
        "character".to_string(),
        "string".to_string(),
        "bigint".to_string(),
        "number".to_string(),
        "symbol".to_string(),
        "unique symbol".to_string(),
        "int".to_string(),
        "uint".to_string(),
        "float".to_string(),
    ];

    for int_type in [
        IntegerType::Fixed {
            width: 8,
            is_signed: true,
        },
        IntegerType::Fixed {
            width: 16,
            is_signed: true,
        },
        IntegerType::Fixed {
            width: 32,
            is_signed: true,
        },
        IntegerType::Fixed {
            width: 64,
            is_signed: true,
        },
        IntegerType::Fixed {
            width: 128,
            is_signed: true,
        },
        IntegerType::Fixed {
            width: 256,
            is_signed: true,
        },
        IntegerType::Pointer { is_signed: true },
        IntegerType::Fixed {
            width: 8,
            is_signed: false,
        },
        IntegerType::Fixed {
            width: 16,
            is_signed: false,
        },
        IntegerType::Fixed {
            width: 32,
            is_signed: false,
        },
        IntegerType::Fixed {
            width: 64,
            is_signed: false,
        },
        IntegerType::Fixed {
            width: 128,
            is_signed: false,
        },
        IntegerType::Fixed {
            width: 256,
            is_signed: false,
        },
        IntegerType::Pointer { is_signed: false },
    ] {
        names.push(int_type.as_str());
    }

    for float_type in [FloatType::Float32, FloatType::Float64] {
        names.push(float_type.as_str().to_string());
    }

    let mut seen = HashSet::new();
    let mut completions = Vec::new();
    for name in names {
        if !seen.insert(name.clone()) {
            continue;
        }

        completions.push(
            Completion::new(&name, CompletionKind::TypeParameter)
                .with_sort_order(SORT_BUILTIN)
                .with_sort_text(length_sort_text(&name))
                .as_builtin(),
        );
    }

    completions
}

/// Check whether one symbol type is constructable with `new`.
fn is_constructable_symbol(symbol_form: SymbolForm) -> bool {
    matches!(symbol_form, SymbolForm::Class | SymbolForm::Struct)
}

/// Check whether one completion entry is constructable with `new`.
pub(super) fn is_constructable_completion(completion: &Completion) -> bool {
    matches!(
        completion.kind,
        CompletionKind::Class | CompletionKind::Struct
    )
}

/// Get keyword completions.
fn keyword_completions() -> Vec<Completion> {
    let keywords = [
        Keyword::Public,
        Keyword::Protected,
        Keyword::Private,
        Keyword::Readonly,
        Keyword::Static,
        Keyword::Final,
        Keyword::Accessor,
        Keyword::Default,
        Keyword::Self_,
        Keyword::This,
        Keyword::Super,
        Keyword::Package,
        Keyword::Import,
        Keyword::Export,
        Keyword::From,
        Keyword::Const,
        Keyword::Let,
        Keyword::Namespace,
        Keyword::Type,
        Keyword::Newtype,
        Keyword::Struct,
        Keyword::Class,
        Keyword::Enum,
        Keyword::Union,
        Keyword::Interface,
        Keyword::Function,
        Keyword::Extension,
        Keyword::Declare,
        Keyword::New,
        Keyword::Constructor,
        Keyword::Asserts,
        Keyword::Extends,
        Keyword::Implements,
        Keyword::Satisfies,
        Keyword::Abstract,
        Keyword::Override,
        Keyword::InstanceOf,
        Keyword::Where,
        Keyword::Typeof,
        Keyword::Keyof,
        Keyword::Infer,
        Keyword::Any,
        Keyword::Never,
        Keyword::As,
        Keyword::Is,
        Keyword::In,
        Keyword::Of,
        Keyword::Using,
        Keyword::Provides,
        Keyword::Comptime,
        Keyword::If,
        Keyword::Else,
        Keyword::Match,
        Keyword::Switch,
        Keyword::Case,
        Keyword::Do,
        Keyword::While,
        Keyword::For,
        Keyword::Loop,
        Keyword::Break,
        Keyword::Continue,
        Keyword::Debugger,
        Keyword::Return,
        Keyword::Yield,
        Keyword::Goto,
        Keyword::Try,
        Keyword::Catch,
        Keyword::Throw,
        Keyword::Finally,
        Keyword::Async,
        Keyword::Await,
        Keyword::Get,
        Keyword::Set,
        Keyword::Move,
        Keyword::With,
    ];

    let mut seen = HashSet::new();
    let mut completions = Vec::new();
    for keyword in keywords {
        let label = keyword.as_str();
        if !seen.insert(label) {
            continue;
        }

        let mut completion = Completion::new(label, CompletionKind::Keyword)
            .with_sort_order(SORT_KEYWORD)
            .with_sort_text(length_sort_text(label))
            .as_keyword();
        if let Some(snippet) = keyword_snippet(keyword) {
            completion = completion.with_insert_text(snippet).as_snippet();
        }

        completions.push(completion);
    }

    for literal in ["true", "false", "null", "undefined"] {
        if !seen.insert(literal) {
            continue;
        }

        completions.push(
            Completion::new(literal, CompletionKind::Keyword)
                .with_sort_order(SORT_KEYWORD)
                .with_sort_text(length_sort_text(literal))
                .as_keyword(),
        );
    }

    completions
}

/// Resolve a snippet template for control-flow keywords.
fn keyword_snippet(keyword: Keyword) -> Option<&'static str> {
    match keyword {
        Keyword::If => Some("if (${1:condition}) {\n    $0\n}"),
        Keyword::For => Some("for (${1:item} in ${2:items}) {\n    $0\n}"),
        Keyword::While => Some("while (${1:condition}) {\n    $0\n}"),
        Keyword::Switch => {
            Some("switch (${1:value}) {\n    case ${2:pattern}:\n        $0\n    default:\n}")
        }
        Keyword::Try => Some("try {\n    $1\n} catch (${2:error}) {\n    $0\n}"),
        _ => None,
    }
}

/// Complete relative import paths by listing directory contents.
fn complete_relative_path(
    repository: &Repository,
    base_dir: &std::path::Path,
    partial: &str,
) -> Vec<Completion> {
    let (directory, prefix) = if let Some(slash) = partial.rfind('/') {
        (base_dir.join(&partial[..=slash]), &partial[slash + 1..])
    } else {
        (base_dir.to_path_buf(), partial)
    };

    let Ok(entries) = repository.file_system().read_dir(&directory) else {
        return vec![];
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

            let meta = repository.file_system().metadata(entry).ok()?;
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

/// Complete package names from the registry.
fn complete_package_names(
    repository: &Repository,
    revision: Revision,
    prefix: &str,
) -> Vec<Completion> {
    let mut results = Vec::new();
    let mut seen_packages = HashSet::new();

    for module_id in repository.module_ids(revision).ok().unwrap_or_default() {
        let Some(module) = repository.module(revision, module_id).ok().flatten() else {
            continue;
        };
        if !seen_packages.insert(module.package_id) {
            continue;
        }
        let Some(package) = repository
            .package(revision, module.package_id)
            .ok()
            .flatten()
        else {
            continue;
        };
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

/// Build one stable length-aware sort text.
pub(super) fn length_sort_text(label: &str) -> String {
    format!("{:02}:{}", label.chars().count(), label.to_lowercase())
}

/// The semantic and lexical relevance for one completion candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CompletionRelevance {
    /// The lexical match quality for the label.
    lexical: MatchQuality,
    /// The context-fit rank.
    context_rank: u8,
    /// The active-parameter name-fit rank.
    parameter_name_rank: u8,
    /// The expected nominal type-fit rank.
    type_rank: u8,
    /// The expected callable and constructable fit rank.
    callability_rank: u8,
    /// The semantic origin rank.
    origin_rank: u8,
    /// The context-shaped semantic rank.
    semantic_rank: u8,
    /// The producer order rank.
    producer_rank: u32,
    /// The direct-member preference rank.
    member_rank: u8,
    /// The deprecated rank.
    deprecated_rank: u8,
}

/// The ranker for one completion query.
struct CompletionRanker<'a> {
    /// The completion context being ranked.
    context: &'a CompletionContext,
    /// The typed lexical prefix.
    prefix: &'a str,
}

/// The scored ranking state for one completion candidate.
pub(super) struct RankedCompletion {
    /// The original stable candidate order.
    stable_index: usize,
    /// The completion candidate.
    completion: Completion,
    /// The derived lexical and semantic relevance.
    relevance: CompletionRelevance,
}

impl CompletionRelevance {
    /// Build the relevance for one completion candidate in one context.
    fn new(ranker: &CompletionRanker<'_>, completion: &Completion, lexical: MatchQuality) -> Self {
        Self {
            lexical,
            context_rank: ranker.context_rank(completion),
            parameter_name_rank: ranker.parameter_name_rank(completion),
            type_rank: ranker.type_rank(completion),
            callability_rank: ranker.callability_rank(completion),
            origin_rank: ranker.origin_rank(completion),
            semantic_rank: ranker.semantic_rank(completion),
            producer_rank: completion.sort_order,
            member_rank: ranker.member_rank(completion),
            deprecated_rank: u8::from(completion.deprecated),
        }
    }
}

impl CompletionRanker<'_> {
    /// Build one completion ranker.
    fn new<'a>(context: &'a CompletionContext, prefix: &'a str) -> CompletionRanker<'a> {
        CompletionRanker { context, prefix }
    }

    /// Score one completion candidate when it matches the typed prefix.
    fn score_completion(
        &self,
        stable_index: usize,
        completion: Completion,
    ) -> Option<RankedCompletion> {
        let lexical = match_quality(&completion.label, self.prefix)?;
        let relevance = CompletionRelevance::new(self, &completion, lexical);

        Some(RankedCompletion {
            stable_index,
            completion,
            relevance,
        })
    }

    /// Compare two ranked completion candidates.
    fn compare(&self, left: &RankedCompletion, right: &RankedCompletion) -> Ordering {
        left.relevance
            .context_rank
            .cmp(&right.relevance.context_rank)
            .then(
                left.relevance
                    .parameter_name_rank
                    .cmp(&right.relevance.parameter_name_rank),
            )
            .then(left.relevance.type_rank.cmp(&right.relevance.type_rank))
            .then(
                left.relevance
                    .callability_rank
                    .cmp(&right.relevance.callability_rank),
            )
            .then(left.relevance.origin_rank.cmp(&right.relevance.origin_rank))
            .then(
                left.relevance
                    .semantic_rank
                    .cmp(&right.relevance.semantic_rank),
            )
            .then(
                match_kind_order(left.relevance.lexical.kind)
                    .cmp(&match_kind_order(right.relevance.lexical.kind)),
            )
            .then(
                left.relevance
                    .producer_rank
                    .cmp(&right.relevance.producer_rank),
            )
            .then_with(|| self.compare_auto_imports(left, right))
            .then(
                right
                    .relevance
                    .lexical
                    .score
                    .cmp(&left.relevance.lexical.score),
            )
            .then(left.relevance.member_rank.cmp(&right.relevance.member_rank))
            .then(
                left.relevance
                    .deprecated_rank
                    .cmp(&right.relevance.deprecated_rank),
            )
            .then_with(|| self.compare_sort_text(left, right))
            .then(left.stable_index.cmp(&right.stable_index))
            .then_with(|| {
                left.completion
                    .ordering_text()
                    .cmp(right.completion.ordering_text())
            })
    }

    /// Compare two auto-import candidates by import ordering.
    fn compare_auto_imports(&self, left: &RankedCompletion, right: &RankedCompletion) -> Ordering {
        let left_is_auto_import = left.completion.origin == CompletionOrigin::AutoImport;
        let right_is_auto_import = right.completion.origin == CompletionOrigin::AutoImport;

        if left_is_auto_import && right_is_auto_import {
            return left
                .completion
                .import_sort_key
                .cmp(&right.completion.import_sort_key);
        }

        Ordering::Equal
    }

    /// Compare two candidates by explicit sort-text tie breaks.
    fn compare_sort_text(&self, left: &RankedCompletion, right: &RankedCompletion) -> Ordering {
        let left_uses_text = self.prefers_sort_text_tiebreak(&left.completion);
        let right_uses_text = self.prefers_sort_text_tiebreak(&right.completion);

        if !self.prefix.is_empty() || (left_uses_text && right_uses_text) {
            return left
                .completion
                .ordering_text()
                .cmp(right.completion.ordering_text());
        }

        Ordering::Equal
    }

    /// Return whether this completion prefers sort-text tie breaking.
    fn prefers_sort_text_tiebreak(&self, completion: &Completion) -> bool {
        completion.origin == CompletionOrigin::AutoImport
            || matches!(
                completion.origin,
                CompletionOrigin::Builtin | CompletionOrigin::Keyword
            )
            || matches!(
                self.context,
                CompletionContext::ImportPath { .. } | CompletionContext::ImportClause { .. }
            )
    }

    /// Return the active-parameter name-fit rank for this completion.
    fn parameter_name_rank(&self, completion: &Completion) -> u8 {
        let CompletionContext::CallArgument {
            expected_parameter, ..
        } = self.context
        else {
            return 1;
        };
        let Some(expected_parameter) = expected_parameter.as_ref() else {
            return 1;
        };
        let Some(expected_name) = expected_parameter.name.as_ref() else {
            return 1;
        };

        u8::from(!completion.label.eq_ignore_ascii_case(expected_name))
    }

    /// Return the expected-type fit rank for this completion.
    fn type_rank(&self, completion: &Completion) -> u8 {
        let CompletionContext::CallArgument {
            expected_parameter, ..
        } = self.context
        else {
            return 0;
        };
        let Some(expected_parameter) = expected_parameter.as_ref() else {
            return 0;
        };
        if expected_parameter.type_symbols.is_empty() {
            return 0;
        }

        let expected_type_symbol = expected_parameter.type_symbol;
        match completion.type_symbol {
            Some(type_symbol) if Some(type_symbol) == expected_type_symbol => 0,
            Some(_)
                if completion
                    .type_symbols
                    .iter()
                    .any(|type_symbol| expected_parameter.type_symbols.contains(type_symbol)) =>
            {
                1
            }
            None => 2,
            Some(_) => 3,
        }
    }

    /// Return the expected-value-shape fit rank for this completion.
    fn callability_rank(&self, completion: &Completion) -> u8 {
        let CompletionContext::CallArgument {
            expected_parameter, ..
        } = self.context
        else {
            return 0;
        };
        let Some(expected_parameter) = expected_parameter.as_ref() else {
            return 0;
        };

        if expected_parameter.prefers_constructable {
            return u8::from(!completion.value_shape.is_constructable);
        }

        if expected_parameter.prefers_callable {
            return u8::from(!completion.value_shape.is_callable);
        }

        0
    }

    /// Return the ranking bucket for this completion origin.
    fn origin_rank(&self, completion: &Completion) -> u8 {
        match completion.origin {
            CompletionOrigin::Contextual => 0,
            CompletionOrigin::Local => 1,
            CompletionOrigin::Builtin => 2,
            CompletionOrigin::AutoImport => 3,
            CompletionOrigin::Keyword => 4,
            CompletionOrigin::Unknown => 5,
        }
    }

    /// Return the member-source rank for this completion.
    fn member_rank(&self, completion: &Completion) -> u8 {
        if !matches!(self.context, CompletionContext::MemberAccess { .. }) {
            return 0;
        }

        u8::from(completion.is_extension_member)
    }

    /// Return the context-fit rank for this completion.
    fn context_rank(&self, completion: &Completion) -> u8 {
        match self.context {
            CompletionContext::TypePosition { .. } => u8::from(!completion.kind.is_type_like()),
            CompletionContext::NewExpression { .. } => {
                u8::from(!is_constructable_completion(completion))
            }
            CompletionContext::ObjectLiteral { .. } => {
                u8::from(completion.kind != CompletionKind::Field)
            }
            CompletionContext::MemberAccess { .. } => u8::from(!completion.kind.is_member_like()),
            CompletionContext::ImportPath { .. } => u8::from(!matches!(
                completion.kind,
                CompletionKind::Folder | CompletionKind::Module
            )),
            _ => 0,
        }
    }

    /// Return the context-shaped semantic rank for this completion.
    fn semantic_rank(&self, completion: &Completion) -> u8 {
        match self.context {
            CompletionContext::TypePosition { .. } => completion.kind.type_position_rank(),
            CompletionContext::NewExpression { .. } => completion.kind.new_expression_rank(),
            CompletionContext::ObjectLiteral { .. } => completion.kind.object_literal_rank(),
            CompletionContext::ImportClause { .. } => completion.kind.import_clause_rank(),
            CompletionContext::MemberAccess { .. } => completion.kind.member_access_rank(),
            CompletionContext::ImportPath { .. } => completion.kind.import_path_rank(),
            _ => completion.kind.value_position_rank(),
        }
    }
}

impl CompletionContext {
    /// Return auto import search settings for this completion context.
    fn auto_import_settings(
        &self,
    ) -> Option<(
        SymbolSpace,
        Option<dir::LocalScopeId>,
        Option<dir::LocalScopeMark>,
        bool,
    )> {
        match self {
            CompletionContext::ValuePosition {
                scope_id,
                scope_mark,
            }
            | CompletionContext::StatementPosition {
                scope_id,
                scope_mark,
            }
            | CompletionContext::ObjectLiteralValue {
                scope_id,
                scope_mark,
            }
            | CompletionContext::CallArgument {
                scope_id,
                scope_mark,
                ..
            } => Some((SymbolSpace::Value, *scope_id, *scope_mark, false)),
            CompletionContext::TypePosition {
                scope_id,
                scope_mark,
            } => Some((SymbolSpace::Type, *scope_id, *scope_mark, false)),
            CompletionContext::NewExpression {
                scope_id,
                scope_mark,
            } => Some((SymbolSpace::Value, *scope_id, *scope_mark, true)),
            _ => None,
        }
    }

    /// Return whether this context should exclude initializer bindings.
    fn uses_initializer_exclusions(&self) -> bool {
        matches!(
            self,
            CompletionContext::ValuePosition { .. }
                | CompletionContext::StatementPosition { .. }
                | CompletionContext::ObjectLiteral { .. }
                | CompletionContext::ObjectLiteralValue { .. }
                | CompletionContext::CallArgument { .. }
                | CompletionContext::NewExpression { .. }
        )
    }
}

impl CompletionValueShape {
    /// Build one coarse callable and constructable shape from one symbol type.
    fn for_symbol_form(symbol_form: SymbolForm) -> Self {
        match symbol_form {
            SymbolForm::Function => Self {
                is_callable: true,
                is_constructable: false,
            },
            SymbolForm::Class | SymbolForm::Struct => Self {
                is_callable: false,
                is_constructable: true,
            },
            _ => Self::default(),
        }
    }
}

impl CompletionKind {
    /// Return the semantic rank for type-position completions.
    fn type_position_rank(self) -> u8 {
        match self {
            CompletionKind::Class
            | CompletionKind::Struct
            | CompletionKind::Interface
            | CompletionKind::Enum
            | CompletionKind::TypeParameter => 0,
            CompletionKind::Module | CompletionKind::Folder | CompletionKind::File => 1,
            CompletionKind::Keyword => 3,
            _ => 2,
        }
    }

    /// Return the semantic rank for new-expression completions.
    fn new_expression_rank(self) -> u8 {
        match self {
            CompletionKind::Struct => 0,
            CompletionKind::Class => 1,
            CompletionKind::Function | CompletionKind::Constructor => 2,
            CompletionKind::Keyword => 4,
            _ => 3,
        }
    }

    /// Return the semantic rank for object-literal completions.
    fn object_literal_rank(self) -> u8 {
        match self {
            CompletionKind::Field => 0,
            CompletionKind::Variable | CompletionKind::Constant | CompletionKind::Value => 1,
            CompletionKind::EnumMember => 2,
            CompletionKind::Method | CompletionKind::Function | CompletionKind::Constructor => 3,
            CompletionKind::Class | CompletionKind::Struct | CompletionKind::Enum => 4,
            CompletionKind::Keyword => 6,
            _ => 5,
        }
    }

    /// Return the semantic rank for member completions.
    fn member_access_rank(self) -> u8 {
        match self {
            CompletionKind::Field | CompletionKind::Property => 0,
            CompletionKind::Method | CompletionKind::Function | CompletionKind::Constructor => 1,
            CompletionKind::EnumMember => 2,
            CompletionKind::Keyword => 4,
            _ => 3,
        }
    }

    /// Return the semantic rank for import-clause completions.
    fn import_clause_rank(self) -> u8 {
        match self {
            CompletionKind::Class
            | CompletionKind::Struct
            | CompletionKind::Interface
            | CompletionKind::Enum
            | CompletionKind::TypeParameter => 0,
            CompletionKind::Variable | CompletionKind::Constant | CompletionKind::Value => 1,
            CompletionKind::Method | CompletionKind::Function | CompletionKind::Constructor => 2,
            CompletionKind::Module | CompletionKind::Folder | CompletionKind::File => 3,
            CompletionKind::Keyword => 5,
            _ => 4,
        }
    }

    /// Return the semantic rank for import-path completions.
    fn import_path_rank(self) -> u8 {
        match self {
            CompletionKind::Module => 0,
            CompletionKind::Folder => 1,
            _ => 2,
        }
    }

    /// Return the semantic rank for value-position completions.
    fn value_position_rank(self) -> u8 {
        match self {
            CompletionKind::Variable | CompletionKind::Constant | CompletionKind::Value => 0,
            CompletionKind::EnumMember => 1,
            CompletionKind::Field | CompletionKind::Property => 2,
            CompletionKind::Method | CompletionKind::Function | CompletionKind::Constructor => 3,
            CompletionKind::Class | CompletionKind::Struct | CompletionKind::Enum => 4,
            CompletionKind::Interface | CompletionKind::TypeParameter => 5,
            CompletionKind::Keyword => 7,
            _ => 6,
        }
    }

    /// Return true when this completion kind fits type positions well.
    fn is_type_like(self) -> bool {
        matches!(
            self,
            CompletionKind::Class
                | CompletionKind::Struct
                | CompletionKind::Interface
                | CompletionKind::Enum
                | CompletionKind::TypeParameter
        )
    }

    /// Return true when this completion kind fits member positions well.
    fn is_member_like(self) -> bool {
        matches!(
            self,
            CompletionKind::Method
                | CompletionKind::Function
                | CompletionKind::Constructor
                | CompletionKind::Field
                | CompletionKind::Property
                | CompletionKind::EnumMember
        )
    }
}

/// Score and rank completions using lexical and semantic relevance.
pub(super) fn score_and_rank_completions(
    completions: Vec<Completion>,
    context: &CompletionContext,
    token: Option<&CursorToken>,
) -> Vec<RankedCompletion> {
    let prefix = token.map(|token| token.text.as_str()).unwrap_or("");
    let ranker = CompletionRanker::new(context, prefix);
    let mut ranked: Vec<RankedCompletion> = completions
        .into_iter()
        .enumerate()
        .filter_map(|(stable_index, completion)| ranker.score_completion(stable_index, completion))
        .collect();

    ranked.sort_by(|left, right| ranker.compare(left, right));

    ranked
}

/// Filter and rank completions using lexical and semantic relevance.
pub(super) fn filter_and_rank_completions(
    completions: Vec<Completion>,
    context: &CompletionContext,
    token: Option<&CursorToken>,
) -> Vec<Completion> {
    let prefix = token.map(|token| token.text.as_str()).unwrap_or("");
    let scored = score_and_rank_completions(completions, context, token);

    let mut results: Vec<Completion> = scored
        .into_iter()
        .map(|ranked| {
            let mut completion = ranked.completion;
            let relevance = ranked.relevance;
            completion.match_positions = relevance.lexical.matched_indices;
            completion
        })
        .collect();

    let has_exact_label_match = !prefix.is_empty()
        && results
            .first()
            .map(|completion| completion.label == prefix)
            .unwrap_or(false);

    if (results.len() == 1 || has_exact_label_match)
        && let Some(first) = results.first_mut()
    {
        first.preselect = true;
    }

    results
}

/// Return the ordering bucket for one lexical match kind.
fn match_kind_order(kind: MatchKind) -> u8 {
    match kind {
        MatchKind::ExactWhole => 0,
        MatchKind::CaseInsensitiveWhole => 1,
        MatchKind::ExactPrefix => 2,
        MatchKind::CaseInsensitivePrefix => 3,
        MatchKind::ExactBoundary => 4,
        MatchKind::CaseInsensitiveBoundary => 5,
        MatchKind::Subsequence => 6,
        MatchKind::NoFilter => 7,
    }
}
