use std::collections::HashMap;
use std::str::FromStr;

use destack_core::StringPool;
use destack_source::{BatchEdit, Edit, FileEdit, FileId, Span, Uri};
use destack_workspace::{Repository, Revision};
use serde::{Deserialize, Serialize};
use {destack_ast as ast, destack_dir as dir};

use crate::ast::{is_simple_identifier, sort_and_dedup_spans, token_at_offset};
use crate::core::{
    NominalRelation, modules_referencing_symbol, nominal_relations_for_target, query_context,
};
use crate::dir::{
    ReferenceCollectionOptions, SymbolAtOffset, collect_default_import_alias_symbols_for_export,
    collect_symbol_references_in_context, declaration_name, find_symbol_at_offset,
    get_canonical_symbol, get_symbol_definition_span, get_symbol_local_definition_span,
    member_key_name, resolve_local_import_alias_name, resolve_symbol_name,
};

/// Result of a prepare rename query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrepareRenameResult {
    /// The range of the symbol to rename.
    pub range: Span,
    /// The current name (placeholder for rename dialog).
    pub placeholder: String,
}

/// Result of a rename query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenameResult {
    /// All edits to apply.
    pub edits: BatchEdit,
}

impl RenameResult {
    /// Create an empty rename result.
    pub fn empty() -> Self {
        Self {
            edits: BatchEdit::new(),
        }
    }

    /// Create a rename result from a batch edit.
    pub fn from_edits(edits: BatchEdit) -> Self {
        Self { edits }
    }

    /// Whether there are any edits.
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    /// Total number of edits.
    pub fn edit_count(&self) -> usize {
        self.edits.total_edits()
    }

    /// Number of files affected.
    pub fn file_count(&self) -> usize {
        self.edits.file_count()
    }
}

/// Request prepare rename at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrepareRenameRequest {
    /// The document URI.
    pub uri: Uri,
    /// The byte offset in the document.
    pub offset: u32,
}

/// Response payload for prepare rename queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrepareRenameResponse {
    /// Prepare rename result, if available.
    pub result: Option<PrepareRenameResult>,
}

/// Request rename edits at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenameRequest {
    /// The document URI.
    pub uri: Uri,
    /// The byte offset in the document.
    pub offset: u32,
    /// The new name for the symbol.
    pub new_name: String,
}

/// Response payload for rename queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenameResponse {
    /// Rename result, if available.
    pub result: Option<RenameResult>,
}

/// Check if the symbol at the given position can be renamed.
///
/// Returns the range and current name if renameable.
pub fn prepare_rename(
    repository: &Repository,
    revision: Revision,
    file: FileId,
    offset: u32,
) -> Option<PrepareRenameResult> {
    // resolve the rename target at the cursor
    let (symbol_at, _, name) = resolve_rename_target(repository, revision, file, offset)?;

    // return the range and current name
    Some(PrepareRenameResult {
        range: symbol_at.span,
        placeholder: name,
    })
}

/// Rename the symbol at the given position.
///
/// Returns edits for all files that need to be modified.
pub fn rename(
    repository: &Repository,
    revision: Revision,
    file: FileId,
    offset: u32,
    new_name: &str,
) -> Option<RenameResult> {
    // validate new_name is a valid identifier
    if !is_simple_identifier(new_name) {
        return None;
    }

    // resolve the rename target at the cursor
    let (_, canonical_id, old_name) = resolve_rename_target(repository, revision, file, offset)?;
    let interface_member_target =
        resolve_interface_member_target(repository, revision, canonical_id);
    let preserve_local_definition =
        resolve_local_import_alias_name(repository, revision, canonical_id).is_some();

    // collect primary symbol spans and group by file
    let mut edits_by_file: HashMap<FileId, Vec<Span>> = HashMap::new();
    let primary_spans = collect_symbol_rename_spans(
        repository,
        revision,
        canonical_id,
        &old_name,
        preserve_local_definition,
    );
    extend_spans_by_file(&mut edits_by_file, primary_spans);

    // include default import aliases that bind this export in other modules
    let default_import_alias_symbols =
        collect_default_import_alias_symbols_for_export(repository, revision, canonical_id);
    for alias_symbol in default_import_alias_symbols {
        let Some(alias_name) = resolve_local_import_alias_name(repository, revision, alias_symbol)
        else {
            continue;
        };

        let alias_spans =
            collect_symbol_rename_spans(repository, revision, alias_symbol, &alias_name, true);
        extend_spans_by_file(&mut edits_by_file, alias_spans);
    }

    // include implementation member spans when renaming interface members
    if let Some(interface_member_target) = interface_member_target {
        let implementation_members = collect_interface_member_implementations(
            repository,
            revision,
            &interface_member_target,
            &old_name,
        );

        for member_symbol in implementation_members {
            if member_symbol == canonical_id {
                continue;
            }

            let spans =
                collect_symbol_rename_spans(repository, revision, member_symbol, &old_name, false);
            extend_spans_by_file(&mut edits_by_file, spans);
        }
    }

    // normalize span ordering and remove duplicates per file
    for spans in edits_by_file.values_mut() {
        sort_and_dedup_spans(spans);
        prune_overlapping_spans(spans);
    }

    // create BatchEdit from collected spans
    let mut batch_edit = BatchEdit::new();
    for (file_id, spans) in edits_by_file {
        let edits: Vec<Edit> = spans
            .into_iter()
            .map(|span| Edit::replace(span, new_name.to_string()))
            .collect();
        batch_edit.push(FileEdit::with_edits(file_id, edits));
    }

    Some(RenameResult::from_edits(batch_edit))
}

/// Resolve the symbol targeted by rename at a file offset.
fn resolve_rename_target(
    repository: &Repository,
    revision: Revision,
    file: FileId,
    offset: u32,
) -> Option<(SymbolAtOffset, dir::GlobalSymbolId, String)> {
    // find the symbol at offset
    let symbol_at = find_symbol_at_offset(repository, revision, file, offset)?;

    // reject non modifier keywords at the cursor
    let token = token_at_offset(repository, revision, file, offset);
    if token.is_some_and(|token| {
        ast::Keyword::from_str(&token)
            .map(|keyword| !is_rename_target_modifier_keyword(keyword))
            .unwrap_or(false)
    }) {
        return None;
    }

    let symbol_id = symbol_at.symbol_id;

    // keep explicit local import aliases as local rename targets
    if let Some(local_alias_name) = resolve_local_import_alias_name(repository, revision, symbol_id)
    {
        return Some((symbol_at, symbol_id, local_alias_name));
    }

    // resolve canonical symbol and stable rename name
    let canonical_id = get_canonical_symbol(repository, revision, symbol_id);
    let name = resolve_rename_name(repository, revision, canonical_id)?;

    Some((symbol_at, canonical_id, name))
}

/// Build reference collection options used by rename.
fn rename_reference_options<'a>(target_name: &'a str) -> ReferenceCollectionOptions<'a> {
    ReferenceCollectionOptions {
        include_expressions: true,
        include_members: true,
        include_dependencies: true,
        include_namespace_receivers: true,
        skip_dependency_aliases: true,
        use_dependency_name_spans: true,
        target_name: Some(target_name),
        require_target_name_match: true,
        limit_to_file: None,
    }
}

/// Collect all rename spans for one canonical symbol.
fn collect_symbol_rename_spans(
    repository: &Repository,
    revision: Revision,
    canonical_id: dir::GlobalSymbolId,
    target_name: &str,
    preserve_local_definition: bool,
) -> Vec<Span> {
    // seed spans with the declaration site
    let mut spans = Vec::new();
    let definition_span = if preserve_local_definition {
        get_symbol_local_definition_span(repository, revision, canonical_id)
            .or_else(|| get_symbol_definition_span(repository, revision, canonical_id))
    } else {
        get_symbol_definition_span(repository, revision, canonical_id)
    };
    if let Some(definition_span) = definition_span {
        spans.push(definition_span);
    }

    // collect references across user modules
    let reference_options = rename_reference_options(target_name);
    let reference_spans = collect_symbol_reference_spans_across_user_modules(
        repository,
        revision,
        canonical_id,
        reference_options,
    );
    spans.extend(reference_spans);

    // normalize for deterministic edits
    sort_and_dedup_spans(&mut spans);
    spans
}

/// Collect symbol reference spans across user modules.
fn collect_symbol_reference_spans_across_user_modules(
    repository: &Repository,
    revision: Revision,
    canonical_id: dir::GlobalSymbolId,
    options: ReferenceCollectionOptions<'_>,
) -> Vec<Span> {
    let mut spans = Vec::new();

    for module_id in modules_referencing_symbol(repository, revision, canonical_id) {
        let Some(ctx) = query_context(repository, revision, module_id) else {
            continue;
        };

        let module_spans = collect_symbol_references_in_context(
            repository,
            ctx.ast(),
            ctx.dir(),
            canonical_id,
            options,
        );
        spans.extend(module_spans);
    }

    spans
}

/// Append spans into a per-file span map.
fn extend_spans_by_file(edits_by_file: &mut HashMap<FileId, Vec<Span>>, spans: Vec<Span>) {
    for span in spans {
        edits_by_file.entry(span.file).or_default().push(span);
    }
}

/// Remove overlapping spans by keeping the most specific span at each overlap.
fn prune_overlapping_spans(spans: &mut Vec<Span>) {
    if spans.len() < 2 {
        return;
    }

    let mut filtered = Vec::with_capacity(spans.len());
    for span in spans.iter().copied() {
        let Some(last_span) = filtered.last_mut() else {
            filtered.push(span);
            continue;
        };

        if !last_span.intersects(span) {
            filtered.push(span);
            continue;
        }

        if span.len() < last_span.len()
            || (span.len() == last_span.len() && span.start >= last_span.start)
        {
            *last_span = span;
        }
    }

    *spans = filtered;
}

/// Resolve the stable rename source name for a symbol.
fn resolve_rename_name(
    repository: &Repository,
    revision: Revision,
    canonical_id: dir::GlobalSymbolId,
) -> Option<String> {
    // prefer the canonical symbol metadata name when present
    if let Some(name) = resolve_symbol_name(repository, revision, canonical_id) {
        return Some(name);
    }

    // fall back to declaration based name extraction
    resolve_name_from_declaration(repository, revision, canonical_id)
}

/// Resolve a symbol name from its declaration when symbol metadata has no name.
fn resolve_name_from_declaration(
    repository: &Repository,
    revision: Revision,
    canonical_id: dir::GlobalSymbolId,
) -> Option<String> {
    // resolve query context for the symbol module
    let ctx = query_context(repository, revision, canonical_id.module_id)?;

    // resolve the declaration node id
    let declaration = {
        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(canonical_id.local_id);
        symbol.declaration?
    };

    let dir_tree = ctx.dir().view();
    match declaration.local_id.ty {
        dir::NodeType::Member => {
            let member_id = declaration.local_id.try_into().ok()?;
            let member = dir_tree.get::<dir::Member>(member_id);
            let key = member.key()?;
            member_key_name(ctx.dir().strings(), key)
        }
        dir::NodeType::EnumField => {
            let field_id = declaration.local_id.try_into().ok()?;
            let field = dir_tree.get::<dir::EnumField>(field_id);
            Some(ctx.dir().strings().get(field.name.string()).to_string())
        }
        dir::NodeType::Declaration => {
            let declaration_id = declaration.local_id.try_into().ok()?;
            let declaration = dir_tree.get::<dir::Declaration>(declaration_id);
            declaration_name(declaration)
                .map(|name| ctx.dir().strings().get(name.string()).to_string())
        }
        dir::NodeType::Parameter => {
            let parameter_id = declaration.local_id.try_into().ok()?;
            let parameter = dir_tree.get::<dir::Parameter>(parameter_id);
            match parameter {
                dir::Parameter::Named { name, .. } => {
                    Some(ctx.dir().strings().get(*name).to_string())
                }
                dir::Parameter::VariadicNamed { name, .. } => {
                    Some(ctx.dir().strings().get(*name).to_string())
                }
                dir::Parameter::Pattern { .. }
                | dir::Parameter::VariadicPattern { .. }
                | dir::Parameter::Error { .. } => None,
            }
        }
        dir::NodeType::Pattern => {
            let pattern_id = declaration.local_id.try_into().ok()?;
            let pattern = dir_tree.get::<dir::Pattern>(pattern_id);
            match pattern {
                dir::Pattern::Binding { name, .. } => {
                    Some(ctx.dir().strings().get(*name).to_string())
                }
                _ => None,
            }
        }
        dir::NodeType::PatternField => {
            let field_id = declaration.local_id.try_into().ok()?;
            let field = dir_tree.get::<dir::PatternField>(field_id);
            match field {
                dir::PatternField::Named {
                    name,
                    symbol,
                    pattern,
                    ..
                } => {
                    let Some(symbol) = symbol else {
                        return None;
                    };

                    if let Some(pattern) = pattern {
                        return rename_pattern_binding_name(
                            ctx.dir().strings(),
                            dir_tree,
                            *pattern,
                            *symbol,
                        );
                    }

                    Some(ctx.dir().strings().get(*name).to_string())
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Return the binding name for one pattern subtree.
fn rename_pattern_binding_name(
    strings: &StringPool,
    dir_tree: dir::View<'_>,
    pattern_id: dir::LocalNodeId<dir::Pattern>,
    target_symbol: dir::LocalSymbolId,
) -> Option<String> {
    match dir_tree.get::<dir::Pattern>(pattern_id) {
        dir::Pattern::Binding {
            symbol,
            name,
            pattern,
            ..
        } => {
            if *symbol == target_symbol {
                return Some(strings.get(*name).to_string());
            }

            pattern.and_then(|pattern| {
                rename_pattern_binding_name(strings, dir_tree, pattern, target_symbol)
            })
        }
        dir::Pattern::Assign { pattern, .. }
        | dir::Pattern::Must(pattern)
        | dir::Pattern::BorrowOf { right: pattern, .. }
        | dir::Pattern::MoveOf { right: pattern, .. } => {
            rename_pattern_binding_name(strings, dir_tree, *pattern, target_symbol)
        }
        dir::Pattern::Tuple { .. }
        | dir::Pattern::TaggedTuple { .. }
        | dir::Pattern::Sequence { .. }
        | dir::Pattern::Object { .. }
        | dir::Pattern::TaggedObject { .. }
        | dir::Pattern::Union { .. }
        | dir::Pattern::Wildcard
        | dir::Pattern::Expression { .. }
        | dir::Pattern::Range { .. }
        | dir::Pattern::TypeExpression { .. } => None,
    }
}

/// Interface member kind used for implementation matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InterfaceMemberKind {
    /// A callable member declaration.
    Method,
    /// A field-like member declaration.
    Field,
}

/// Interface member information used for implementation propagation.
#[derive(Debug, Clone)]
struct InterfaceMemberTarget {
    /// The canonical interface symbol.
    interface_symbol: dir::GlobalSymbolId,
    /// The member name to propagate.
    member_name: String,
    /// The member shape to match.
    member_kind: InterfaceMemberKind,
}

/// Resolve the interface member target for an interface member symbol.
fn resolve_interface_member_target(
    repository: &Repository,
    revision: Revision,
    canonical_id: dir::GlobalSymbolId,
) -> Option<InterfaceMemberTarget> {
    // resolve query context for the symbol module
    let ctx = query_context(repository, revision, canonical_id.module_id)?;

    // resolve the member declaration node
    let declaration = {
        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(canonical_id.local_id);
        symbol.declaration?
    };

    if declaration.local_id.ty != dir::NodeType::Member {
        return None;
    }

    let dir_tree = ctx.dir().view();
    let Ok(member_id) = declaration.local_id.try_into() else {
        return None;
    };
    let member = dir_tree.get::<dir::Member>(member_id);
    let (member_kind, member_key) = interface_member_kind_and_key(member)?;
    let member_name = member_key_name(ctx.dir().strings(), member_key)?;

    // resolve the parent declaration and ensure it is an interface
    let parent = dir_tree.get_parent(member_id)?;
    if parent.ty != dir::NodeType::Declaration {
        return None;
    }
    let Ok(declaration_id) = parent.try_into() else {
        return None;
    };
    let declaration = dir_tree.get::<dir::Declaration>(declaration_id);
    let dir::Declaration::Interface(declaration) = declaration else {
        return None;
    };
    let interface_symbol = get_canonical_symbol(
        repository,
        revision,
        dir::GlobalSymbolId::new(ctx.module_id(), declaration.symbol),
    );

    Some(InterfaceMemberTarget {
        interface_symbol,
        member_name,
        member_kind,
    })
}

/// Collect implementation member symbols for a resolved interface member target.
fn collect_interface_member_implementations(
    repository: &Repository,
    revision: Revision,
    target: &InterfaceMemberTarget,
    expected_name: &str,
) -> Vec<dir::GlobalSymbolId> {
    let mut members = Vec::new();
    let interface_symbol = get_canonical_symbol(repository, revision, target.interface_symbol);
    let implementing_symbols: Vec<dir::GlobalSymbolId> =
        nominal_relations_for_target(repository, revision, interface_symbol)
            .into_iter()
            .filter(|entry| entry.relation == NominalRelation::Implements)
            .map(|entry| entry.source_symbol)
            .collect();

    if implementing_symbols.is_empty() {
        return members;
    }

    let implementing_module_ids: HashMap<destack_source::ModuleId, Vec<dir::LocalSymbolId>> =
        implementing_symbols
            .into_iter()
            .fold(HashMap::new(), |mut modules, symbol_id| {
                modules
                    .entry(symbol_id.module_id)
                    .or_default()
                    .push(symbol_id.local_id);
                modules
            });

    for (module_id, implementing_symbols) in implementing_module_ids {
        let Some(ctx) = query_context(repository, revision, module_id) else {
            continue;
        };

        let dir_tree = ctx.dir().view();
        for (member_id, member) in dir_tree.iter_nodes_of_type::<dir::Member>() {
            let Some(parent) = dir_tree.get_parent(member_id) else {
                continue;
            };
            if parent.ty != dir::NodeType::Declaration {
                continue;
            }
            let Ok(declaration_id) = parent.try_into() else {
                continue;
            };
            let declaration = dir_tree.get::<dir::Declaration>(declaration_id);
            let owner_symbol = match declaration {
                dir::Declaration::Class(declaration) => declaration.symbol,
                dir::Declaration::Struct(declaration) => declaration.symbol,
                dir::Declaration::Interface(declaration) => declaration.symbol,
                _ => continue,
            };
            if !implementing_symbols.contains(&owner_symbol) {
                continue;
            }

            let Some((member_kind, member_key)) = interface_member_kind_and_key(member) else {
                continue;
            };
            if member_kind != target.member_kind {
                continue;
            }

            let Some(member_name) = member_key_name(ctx.dir().strings(), member_key) else {
                continue;
            };
            if member_name != expected_name && member_name != target.member_name {
                continue;
            }

            let symbol_id = dir::GlobalSymbolId::new(ctx.module_id(), member.symbol());
            members.push(get_canonical_symbol(repository, revision, symbol_id));
        }
    }

    members.sort();
    members.dedup();
    members
}

/// Resolve the interface-member kind and key for one declaration member.
fn interface_member_kind_and_key(member: &dir::Member) -> Option<(InterfaceMemberKind, &dir::Key)> {
    match member {
        dir::Member::Method { key, .. } => Some((InterfaceMemberKind::Method, key.as_ref()?)),
        dir::Member::Field { key, .. } => Some((InterfaceMemberKind::Field, key)),
        _ => None,
    }
}

/// Check whether a modifier keyword can target the declaration for rename.
fn is_rename_target_modifier_keyword(keyword: ast::Keyword) -> bool {
    matches!(
        keyword,
        ast::Keyword::Export
            | ast::Keyword::Declare
            | ast::Keyword::Abstract
            | ast::Keyword::Async
            | ast::Keyword::Static
            | ast::Keyword::Public
            | ast::Keyword::Protected
            | ast::Keyword::Private
            | ast::Keyword::Readonly
            | ast::Keyword::Final
            | ast::Keyword::Accessor
            | ast::Keyword::Default
            | ast::Keyword::Override
    )
}
