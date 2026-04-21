use std::collections::{HashMap, HashSet};

use destack_dir::{self as dir, NodeVisitor};
use destack_source::{BatchEdit, Edit, FileEdit, FileId, ModuleId, Span, Uri};
use destack_workspace::{Repository, Revision};
use serde::{Deserialize, Serialize};

use crate::ast::{
    get_module_by_file_id, is_simple_identifier, line_start_for_offset, main_span_for_dir_node,
    span_for_dir_node,
};
use crate::core::{QueryContext, RepositoryQueryIndexExt, query_context};
use crate::dir::{
    ReferenceCollectionOptions, collect_symbol_references_in_context, find_symbol_at_offset,
    get_canonical_symbol, get_member_access_name_span, get_symbol_definition_span, member_key_name,
    resolve_symbol_name,
};

/// Request payload for inline refactor queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InlineRequest {
    /// The document URI.
    pub uri: Uri,
    /// The byte offset in the document.
    pub offset: u32,
}

/// Response payload for inline refactor queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InlineResponse {
    /// Inline result, if available.
    pub result: Option<InlineResult>,
}

/// Result of an inline refactor query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InlineResult {
    /// All edits to apply.
    pub edits: BatchEdit,
}

impl InlineResult {
    /// Create an empty inline result.
    pub fn empty() -> Self {
        Self {
            edits: BatchEdit::new(),
        }
    }

    /// Create a result from a batch edit.
    pub fn from_edits(edits: BatchEdit) -> Self {
        Self { edits }
    }

    /// Whether there are any edits.
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }
}

/// Inline the symbol at the given position.
pub fn inline_symbol(
    repository: &Repository,
    revision: Revision,
    file: FileId,
    offset: u32,
) -> Option<InlineResult> {
    // resolve the module and query context
    let module = get_module_by_file_id(repository, revision, file)?;
    let ctx = query_context(repository, revision, module.id)?;

    // find the symbol at the cursor
    let symbol_at = find_symbol_at_offset(repository, revision, file, offset)?;
    let canonical_id = get_canonical_symbol(repository, revision, symbol_at.symbol_id);

    // only inline symbols defined in this file
    let definition_span = get_symbol_definition_span(repository, revision, canonical_id)?;
    if definition_span.file != file {
        return None;
    }

    // resolve the symbol metadata
    let declaration = {
        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(canonical_id.local_id);
        let declaration = symbol.primary_declaration?;

        // avoid inlining symbols that are exported from the module
        if symbol.export.is_some() {
            return None;
        }

        declaration
    };

    // find the declarator that owns the symbol
    let dir_tree = ctx.dir().tree();
    let declaration_id = declaration.local_id;
    let (declarator_id, statement_id) = find_declarator_and_statement(dir_tree, declaration_id)?;

    // resolve the initializer expression
    let value_id = declarator_value(dir_tree, declarator_id, statement_id)?;
    let declarator = dir_tree.get::<dir::Declarator>(declarator_id);
    let statement_span = span_for_dir_node(ctx.ast(), dir_tree, statement_id.into());

    // resolve the initializer text
    let source_file = repository.file(revision, file).ok().flatten()?;
    let value_span = span_for_dir_node(ctx.ast(), dir_tree, value_id.into());
    let value_expression = dir_tree.get::<dir::Expression>(value_id);
    let value_text = source_file.span_str(value_span);
    let inline_base = format_inline_expression(value_text, value_expression);
    if inline_base.is_empty() {
        return None;
    }

    // resolve destructuring paths for inline expressions
    let binding_count = count_pattern_bindings(dir_tree, declarator.pattern);
    if binding_count != 1 {
        return None;
    }

    let access_path = pattern_access_path(
        repository,
        dir_tree,
        declarator.pattern,
        canonical_id.local_id,
    )?;
    let inline_text = apply_access_path(&inline_base, &access_path);
    if inline_text.is_empty() {
        return None;
    }

    let mut edits_by_file: HashMap<FileId, Vec<Edit>> = HashMap::new();
    let reference_name = resolve_symbol_name(repository, revision, canonical_id);
    let reference_entries =
        collect_inline_reference_entries(repository, &ctx, canonical_id, file, reference_name);
    if reference_entries.is_empty() {
        return None;
    }

    // refuse to inline when there are references outside the defining file
    let reference_options = ReferenceCollectionOptions {
        include_expressions: true,
        include_members: true,
        include_dependencies: true,
        include_namespace_receivers: true,
        skip_dependency_aliases: false,
        use_dependency_name_spans: false,
        target_name: None,
        require_target_name_match: false,
        limit_to_file: None,
    };
    for module_id in repository.reference_index_modules_for_target(revision, canonical_id) {
        let Some(ctx) = query_context(repository, ctx.revision(), module_id) else {
            continue;
        };
        let spans = collect_symbol_references_in_context(
            repository,
            ctx.ast(),
            ctx.dir(),
            canonical_id,
            reference_options,
        );
        if spans.iter().any(|span| span.file != file) {
            return None;
        }
    }

    // avoid duplicating side effects when inlining into multiple references
    let has_side_effects = expression_has_side_effects(dir_tree, value_id);
    if has_side_effects && reference_entries.len() > 1 {
        return None;
    }

    // ensure the symbol is not reassigned
    if symbol_is_assigned(dir_tree, canonical_id) {
        return None;
    }

    // avoid shadowing captured symbols in new contexts
    let captured_symbols =
        collect_captured_symbols(repository, &ctx, dir_tree, value_id, canonical_id)?;
    if !captured_symbols.is_empty()
        && !inline_shadow_safe(
            repository,
            &ctx,
            dir_tree,
            &reference_entries,
            &captured_symbols,
        )
    {
        return None;
    }

    for entry in reference_entries {
        let replacement_text = match &entry.replacement {
            ReferenceReplacement::Inline => inline_text.clone(),
            ReferenceReplacement::ObjectShorthand { name } => format!("{name}: {inline_text}"),
        };
        edits_by_file
            .entry(entry.span.file)
            .or_default()
            .push(Edit::replace(entry.span, replacement_text));
    }

    // remove the declaration statement or declarator
    let declarator_spans = statement_declarator_spans(&ctx, dir_tree, statement_id)?;
    let removal_span = declarator_removal_span(
        source_file.text(),
        statement_span,
        &declarator_spans,
        declarator_id,
    )?;
    edits_by_file
        .entry(file)
        .or_default()
        .push(Edit::replace(removal_span, String::new()));

    // build batch edits
    let mut batch_edit = BatchEdit::new();
    for (file_id, edits) in edits_by_file {
        let mut file_edit = FileEdit::with_edits(file_id, edits);
        file_edit.sort();
        batch_edit.push(file_edit);
    }

    Some(InlineResult::from_edits(batch_edit))
}

/// Replacement strategy for an inline reference.
enum ReferenceReplacement {
    /// Inline the value expression directly.
    Inline,
    /// Expand an object shorthand property with an explicit value.
    ObjectShorthand {
        /// The property name to emit.
        name: String,
    },
}

/// Reference metadata for inline edits.
struct ReferenceEntry {
    /// The expression id for the reference.
    expr_id: dir::LocalNodeId<dir::Expression>,
    /// The span to replace.
    span: Span,
    /// The replacement strategy.
    replacement: ReferenceReplacement,
}

/// Captured symbol metadata for shadowing checks.
struct CapturedSymbol {
    /// The symbol name key for fast comparisons.
    name_key: dir::StaticKey,
    /// The canonical symbol id.
    canonical_id: dir::GlobalSymbolId,
}

/// Access path segment for destructured bindings.
#[derive(Debug, Clone)]
enum AccessSegment {
    /// Property access by name.
    Property(String),
    /// Index access by position.
    Index(usize),
}

/// Collect reference entries for the inline target symbol.
fn collect_inline_reference_entries(
    repository: &Repository,
    ctx: &QueryContext,
    canonical_id: dir::GlobalSymbolId,
    file: FileId,
    reference_name: Option<String>,
) -> Vec<ReferenceEntry> {
    // collect reference expressions for the inline target
    let dir_tree = ctx.dir().tree();
    let mut entries = Vec::new();

    for (expr_id, expression) in dir_tree.iter_nodes_of_type::<dir::Expression>() {
        if matches!(expression, dir::Expression::Member { .. }) {
            continue;
        }

        let Some(target_symbol) = expression.target_symbol() else {
            continue;
        };
        let target_canonical = get_canonical_symbol(repository, ctx.revision(), target_symbol);
        if target_canonical != canonical_id {
            continue;
        }

        let span = reference_span_for_expression(ctx, dir_tree, expr_id);
        if span.file != file {
            continue;
        }

        entries.push(ReferenceEntry {
            expr_id,
            span,
            replacement: ReferenceReplacement::Inline,
        });
    }

    // collect object literal shorthand references
    let Some(reference_name) = reference_name.as_deref() else {
        entries.sort_by_key(|entry| (entry.span.start, entry.span.end));
        entries.dedup_by(|left, right| {
            left.span.start == right.span.start && left.span.end == right.span.end
        });
        return entries;
    };

    let symbols = ctx.dir().symbols();
    for (property_id, property) in dir_tree.iter_nodes_of_type::<dir::Property>() {
        let dir::Property::Field { key, value, .. } = property else {
            continue;
        };

        let Some(key_name) = member_key_name(repository, key) else {
            continue;
        };
        if key_name != reference_name {
            continue;
        }
        let value = dir_tree.get::<dir::Expression>(*value);
        let Some(target_symbol) = value.target_symbol() else {
            continue;
        };
        let target_symbol = get_canonical_symbol(repository, ctx.revision(), target_symbol);
        if target_symbol != canonical_id {
            continue;
        }

        let Some(expr_id) = property_parent_expression(dir_tree, property_id) else {
            continue;
        };
        let (scope_id, scope_mark) = dir_tree.get_scope::<dir::Expression>(expr_id);
        let Some(static_key) = key_name_key(key) else {
            continue;
        };
        let Some(resolved_local) =
            resolve_symbol_in_scope(symbols, scope_id, scope_mark, static_key)
        else {
            continue;
        };
        let resolved_global = dir::GlobalSymbolId::new(ctx.module_id(), resolved_local);
        let resolved_canonical = get_canonical_symbol(repository, ctx.revision(), resolved_global);
        if resolved_canonical != canonical_id {
            continue;
        }

        let Some(span) = main_span_for_dir_node(ctx.ast(), dir_tree, property_id.into()) else {
            continue;
        };
        if span.file != file {
            continue;
        }

        entries.push(ReferenceEntry {
            expr_id,
            span,
            replacement: ReferenceReplacement::ObjectShorthand { name: key_name },
        });
    }

    entries.sort_by_key(|entry| (entry.span.start, entry.span.end));
    entries.dedup_by(|left, right| {
        left.span.start == right.span.start && left.span.end == right.span.end
    });
    entries
}

/// Count the number of bindings in a pattern.
fn count_pattern_bindings(
    dir_tree: &dir::NodeTree,
    pattern_id: dir::LocalNodeId<dir::Pattern>,
) -> usize {
    let mut bindings = HashSet::new();
    collect_pattern_bindings(dir_tree, pattern_id, &mut bindings);
    bindings.len()
}

/// Collect binding symbols from a pattern.
fn collect_pattern_bindings(
    dir_tree: &dir::NodeTree,
    pattern_id: dir::LocalNodeId<dir::Pattern>,
    bindings: &mut HashSet<dir::LocalSymbolId>,
) {
    // walk patterns and collect binding symbols
    let pattern = dir_tree.get::<dir::Pattern>(pattern_id);
    match pattern {
        dir::Pattern::Assign { pattern, .. } => {
            collect_pattern_bindings(dir_tree, *pattern, bindings);
        }
        dir::Pattern::Binding {
            symbol, pattern, ..
        } => {
            bindings.insert(*symbol);
            if let Some(inner) = pattern {
                collect_pattern_bindings(dir_tree, *inner, bindings);
            }
        }
        dir::Pattern::Must(inner)
        | dir::Pattern::ReferenceOf { right: inner, .. }
        | dir::Pattern::ValueOf { right: inner, .. } => {
            collect_pattern_bindings(dir_tree, *inner, bindings);
        }
        dir::Pattern::Tuple { fields }
        | dir::Pattern::TaggedTuple { fields, .. }
        | dir::Pattern::Array { fields }
        | dir::Pattern::Object { fields }
        | dir::Pattern::TaggedObject { fields, .. } => {
            for field_id in fields {
                collect_pattern_bindings_field(dir_tree, *field_id, bindings);
            }
        }
        dir::Pattern::Union { patterns } => {
            for pattern_id in patterns {
                collect_pattern_bindings(dir_tree, *pattern_id, bindings);
            }
        }
        dir::Pattern::Wildcard
        | dir::Pattern::Expression { .. }
        | dir::Pattern::TypeExpression { .. } => {}
    }
}

/// Collect binding symbols from a pattern field.
fn collect_pattern_bindings_field(
    dir_tree: &dir::NodeTree,
    field_id: dir::LocalNodeId<dir::PatternField>,
    bindings: &mut HashSet<dir::LocalSymbolId>,
) {
    // walk pattern fields and collect binding symbols
    let field = dir_tree.get::<dir::PatternField>(field_id);
    match field {
        dir::PatternField::Named {
            symbol, pattern, ..
        } => {
            if let Some(symbol) = symbol {
                bindings.insert(*symbol);
            }
            if let Some(pattern) = pattern {
                collect_pattern_bindings(dir_tree, *pattern, bindings);
            }
        }
        dir::PatternField::Positional { pattern, .. } => {
            collect_pattern_bindings(dir_tree, *pattern, bindings);
        }
        dir::PatternField::Computed { pattern, .. } => {
            collect_pattern_bindings(dir_tree, *pattern, bindings);
        }
        dir::PatternField::Spread { pattern, .. } => {
            if let Some(pattern) = pattern {
                collect_pattern_bindings(dir_tree, *pattern, bindings);
            }
        }
        dir::PatternField::Elision => {}
    }
}

/// Resolve the access path for a destructured binding.
fn pattern_access_path(
    repository: &Repository,
    dir_tree: &dir::NodeTree,
    pattern_id: dir::LocalNodeId<dir::Pattern>,
    target_symbol: dir::LocalSymbolId,
) -> Option<Vec<AccessSegment>> {
    let pattern = dir_tree.get::<dir::Pattern>(pattern_id);
    match pattern {
        dir::Pattern::Assign { pattern, .. } => {
            pattern_access_path(repository, dir_tree, *pattern, target_symbol)
        }
        dir::Pattern::Binding {
            symbol, pattern, ..
        } => {
            if *symbol == target_symbol {
                return Some(Vec::new());
            }
            if let Some(inner) = pattern {
                return pattern_access_path(repository, dir_tree, *inner, target_symbol);
            }
            None
        }
        dir::Pattern::Must(inner)
        | dir::Pattern::ReferenceOf { right: inner, .. }
        | dir::Pattern::ValueOf { right: inner, .. } => {
            pattern_access_path(repository, dir_tree, *inner, target_symbol)
        }
        dir::Pattern::Object { fields } | dir::Pattern::TaggedObject { fields, .. } => {
            pattern_access_path_object_fields(repository, dir_tree, fields, target_symbol)
        }
        dir::Pattern::Tuple { fields }
        | dir::Pattern::TaggedTuple { fields, .. }
        | dir::Pattern::Array { fields } => {
            pattern_access_path_indexed(repository, dir_tree, fields, target_symbol)
        }
        dir::Pattern::Union { patterns } => {
            let mut resolved: Option<Vec<AccessSegment>> = None;
            for pattern_id in patterns {
                if let Some(path) =
                    pattern_access_path(repository, dir_tree, *pattern_id, target_symbol)
                {
                    if resolved.is_some() {
                        return None;
                    }
                    resolved = Some(path);
                }
            }
            resolved
        }
        dir::Pattern::Wildcard
        | dir::Pattern::Expression { .. }
        | dir::Pattern::TypeExpression { .. } => None,
    }
}

/// Resolve access paths for object fields.
fn pattern_access_path_object_fields(
    repository: &Repository,
    dir_tree: &dir::NodeTree,
    fields: &[dir::LocalNodeId<dir::PatternField>],
    target_symbol: dir::LocalSymbolId,
) -> Option<Vec<AccessSegment>> {
    // resolve object field access paths
    for field_id in fields {
        let field = dir_tree.get::<dir::PatternField>(*field_id);
        match field {
            dir::PatternField::Named {
                name,
                symbol,
                pattern,
                ..
            } => {
                if symbol.is_some_and(|symbol| symbol == target_symbol) {
                    let name = repository.strings.get(*name).to_string();
                    return Some(vec![AccessSegment::Property(name)]);
                }

                let name = repository.strings.get(*name).to_string();
                if let Some(pattern) = pattern
                    && let Some(path) =
                        pattern_access_path(repository, dir_tree, *pattern, target_symbol)
                {
                    let mut path = path;
                    path.insert(0, AccessSegment::Property(name));
                    return Some(path);
                }
            }
            dir::PatternField::Positional { pattern, .. } => {
                if let Some(path) =
                    pattern_access_path(repository, dir_tree, *pattern, target_symbol)
                {
                    return Some(path);
                }
            }
            dir::PatternField::Computed { .. }
            | dir::PatternField::Spread { .. }
            | dir::PatternField::Elision => {}
        }
    }

    None
}

/// Resolve access paths for tuple and array fields.
fn pattern_access_path_indexed(
    repository: &Repository,
    dir_tree: &dir::NodeTree,
    fields: &[dir::LocalNodeId<dir::PatternField>],
    target_symbol: dir::LocalSymbolId,
) -> Option<Vec<AccessSegment>> {
    // resolve array or tuple access paths
    let mut index = 0usize;
    for field_id in fields {
        let field = dir_tree.get::<dir::PatternField>(*field_id);
        match field {
            dir::PatternField::Elision => {
                index += 1;
            }
            dir::PatternField::Spread { .. } => {
                return None;
            }
            dir::PatternField::Positional { pattern, .. } => {
                if let Some(path) =
                    pattern_access_path(repository, dir_tree, *pattern, target_symbol)
                {
                    let mut path = path;
                    path.insert(0, AccessSegment::Index(index));
                    return Some(path);
                }
                index += 1;
            }
            dir::PatternField::Named {
                symbol, pattern, ..
            } => {
                if symbol.is_some_and(|symbol| symbol == target_symbol) {
                    return Some(vec![AccessSegment::Index(index)]);
                }
                if let Some(pattern) = pattern
                    && let Some(path) =
                        pattern_access_path(repository, dir_tree, *pattern, target_symbol)
                {
                    let mut path = path;
                    path.insert(0, AccessSegment::Index(index));
                    return Some(path);
                }
                index += 1;
            }
            dir::PatternField::Computed { pattern, .. } => {
                if let Some(path) =
                    pattern_access_path(repository, dir_tree, *pattern, target_symbol)
                {
                    let mut path = path;
                    path.insert(0, AccessSegment::Index(index));
                    return Some(path);
                }
                index += 1;
            }
        }
    }

    None
}

/// Apply an access path to a base expression.
fn apply_access_path(base: &str, path: &[AccessSegment]) -> String {
    if path.is_empty() {
        return base.to_string();
    }

    let mut expr = base.to_string();
    for segment in path {
        match segment {
            AccessSegment::Property(name) => {
                if is_simple_identifier(name) {
                    expr.push('.');
                    expr.push_str(name);
                } else {
                    let escaped = escape_string_literal(name);
                    expr.push_str("[\"");
                    expr.push_str(&escaped);
                    expr.push_str("\"]");
                }
            }
            AccessSegment::Index(index) => {
                expr.push('[');
                expr.push_str(&index.to_string());
                expr.push(']');
            }
        }
    }

    expr
}

/// Escape a string literal for bracket property access.
fn escape_string_literal(value: &str) -> String {
    // escape quotes and backslashes for bracket access
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            _ => escaped.push(ch),
        }
    }
    escaped
}

/// Resolve the nearest expression that owns a property shorthand.
fn property_parent_expression(
    dir_tree: &dir::NodeTree,
    property_id: dir::LocalNodeId<dir::Property>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    // walk upward to find the containing expression for a property
    let mut current = dir::LocalNodeIdAny::from(property_id);
    while let Some(parent) = dir_tree.get_parent(current.id) {
        if parent.ty == dir::NodeType::Expression {
            return parent.try_into().ok();
        }
        current = parent;
    }

    None
}

/// Resolve a static key for a property key when possible.
fn key_name_key(key: &dir::Key) -> Option<dir::StaticKey> {
    // resolve a static key for shorthand property matching
    match key {
        dir::Key::Name(dir::Name::Identifier(name) | dir::Name::String(name)) => {
            Some(dir::StaticKey::Name(*name))
        }
        dir::Key::Name(dir::Name::Number(name)) => Some(dir::StaticKey::Number(*name)),
        _ => None,
    }
}

/// Resolve the precise span for a reference expression.
fn reference_span_for_expression(
    ctx: &QueryContext,
    dir_tree: &dir::NodeTree,
    expr_id: dir::LocalNodeId<dir::Expression>,
) -> Span {
    let span = main_span_for_dir_node(ctx.ast(), dir_tree, expr_id.into())
        .unwrap_or_else(|| span_for_dir_node(ctx.ast(), dir_tree, expr_id.into()));

    let Some(parent) = dir_tree.get_parent(expr_id.id) else {
        return span;
    };
    if parent.ty != dir::NodeType::Expression {
        return span;
    }

    let Ok(parent_expr_id) = parent.try_into() else {
        return span;
    };
    let parent_expr = dir_tree.get::<dir::Expression>(parent_expr_id);
    let dir::Expression::Member { left, .. } = parent_expr else {
        return span;
    };
    if *left != expr_id {
        return span;
    }

    let Some(name_span) = get_member_access_name_span(ctx.ast(), ctx.dir(), parent_expr_id) else {
        return span;
    };
    let receiver_end = name_span.start.saturating_sub(1);

    // clamp spans that include `.member` to only the receiver expression
    if span.file == name_span.file && span.start < receiver_end && span.end >= receiver_end {
        return Span::new(span.file, span.start, receiver_end);
    }

    // recover receiver spans when direct mapping points at member names
    let member_span = span_for_dir_node(ctx.ast(), dir_tree, parent);
    if member_span.file == name_span.file && member_span.start < receiver_end {
        return Span::new(member_span.file, member_span.start, receiver_end);
    }

    span
}

/// Collect captured symbols referenced inside the inline value.
fn collect_captured_symbols(
    repository: &Repository,
    ctx: &QueryContext,
    dir_tree: &dir::NodeTree,
    value_id: dir::LocalNodeId<dir::Expression>,
    inline_symbol: dir::GlobalSymbolId,
) -> Option<Vec<CapturedSymbol>> {
    // collect symbols referenced inside the initializer expression
    let symbols = ctx.dir().symbols();
    let mut captured: HashMap<dir::StaticKey, dir::GlobalSymbolId> = HashMap::new();
    let mut has_unknown = false;

    let expression = dir_tree.get::<dir::Expression>(value_id);
    let mut visitor = CapturedSymbolVisitor::new(
        repository,
        ctx.revision(),
        symbols,
        ctx.module_id(),
        inline_symbol,
        &mut captured,
        &mut has_unknown,
    );
    visitor.visit_expression(dir_tree, value_id, expression);

    if has_unknown {
        return None;
    }

    Some(
        captured
            .into_iter()
            .map(|(name_key, canonical_id)| CapturedSymbol {
                name_key,
                canonical_id,
            })
            .collect(),
    )
}

/// Check whether inlining would introduce shadowing.
fn inline_shadow_safe(
    repository: &Repository,
    ctx: &QueryContext,
    dir_tree: &dir::NodeTree,
    reference_entries: &[ReferenceEntry],
    captured_symbols: &[CapturedSymbol],
) -> bool {
    // verify captured symbols resolve identically at each reference site
    let symbols = ctx.dir().symbols();

    for entry in reference_entries {
        let (scope_id, scope_mark) = dir_tree.get_scope::<dir::Expression>(entry.expr_id);
        for captured in captured_symbols {
            let Some(resolved_local) =
                resolve_symbol_in_scope(symbols, scope_id, scope_mark, captured.name_key)
            else {
                return false;
            };
            let resolved_global = dir::GlobalSymbolId::new(ctx.module_id(), resolved_local);
            let resolved_canonical =
                get_canonical_symbol(repository, ctx.revision(), resolved_global);
            if resolved_canonical != captured.canonical_id {
                return false;
            }
        }
    }

    true
}

/// Resolve a symbol within a scope chain.
fn resolve_symbol_in_scope(
    symbols: &dir::SymbolTable,
    mut scope_id: dir::LocalScopeId,
    mut scope_mark: dir::LocalScopeMark,
    key: dir::StaticKey,
) -> Option<dir::LocalSymbolId> {
    // resolve a name through the scope chain honoring scope marks
    loop {
        let scope = symbols.get_scope_by_id(scope_id);
        if let Some(symbol_id) = scope.find_up_to(key, scope_mark) {
            return Some(symbol_id);
        }

        let (parent_id, parent_mark) = scope.parent?;
        scope_id = parent_id;
        scope_mark = parent_mark;
    }
}

/// Find the declarator and statement ids for a declaration.
fn find_declarator_and_statement(
    dir_tree: &dir::NodeTree,
    declaration_id: dir::LocalNodeIdAny,
) -> Option<(
    dir::LocalNodeId<dir::Declarator>,
    dir::LocalNodeId<dir::Expression>,
)> {
    // walk up to the declarator node
    let mut current = declaration_id;
    let mut declarator_id = None;
    let mut statement_id = None;

    while let Some(parent) = dir_tree.get_parent(current.id) {
        if parent.ty == dir::NodeType::Declarator {
            let Ok(typed) = parent.try_into() else {
                return None;
            };
            declarator_id = Some(typed);
        }

        if parent.ty == dir::NodeType::Expression {
            let Ok(typed) = parent.try_into() else {
                return None;
            };
            let expr = dir_tree.get::<dir::Expression>(typed);
            if matches!(expr, dir::Expression::Let { .. }) {
                statement_id = Some(typed);
            }
        }

        if declarator_id.is_some() && statement_id.is_some() {
            break;
        }

        current = parent;
    }

    Some((declarator_id?, statement_id?))
}

/// Resolve the initializer expression for a declarator.
fn declarator_value(
    dir_tree: &dir::NodeTree,
    declarator_id: dir::LocalNodeId<dir::Declarator>,
    statement_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    // ensure the declarator belongs to the let statement
    let statement = dir_tree.get::<dir::Expression>(statement_id);
    let declarators = match statement {
        dir::Expression::Let { declarators, .. } => declarators,
        _ => return None,
    };
    if !declarators.contains(&declarator_id) {
        return None;
    }

    let declarator = dir_tree.get::<dir::Declarator>(declarator_id);
    let value_id = declarator.value?;

    Some(value_id)
}

/// Resolve declarator spans for a let statement.
fn statement_declarator_spans(
    ctx: &QueryContext,
    dir_tree: &dir::NodeTree,
    statement_id: dir::LocalNodeId<dir::Expression>,
) -> Option<Vec<(dir::LocalNodeId<dir::Declarator>, Span)>> {
    // resolve declarator spans from the let statement
    let statement = dir_tree.get::<dir::Expression>(statement_id);
    let declarators = match statement {
        dir::Expression::Let { declarators, .. } => declarators,
        _ => return None,
    };

    let mut spans = Vec::with_capacity(declarators.len());
    for declarator_id in declarators {
        let span = span_for_dir_node(ctx.ast(), dir_tree, (*declarator_id).into());
        spans.push((*declarator_id, span));
    }

    Some(spans)
}

/// Resolve the removal span for a declarator.
fn declarator_removal_span(
    source: &str,
    statement_span: Span,
    declarator_spans: &[(dir::LocalNodeId<dir::Declarator>, Span)],
    target_id: dir::LocalNodeId<dir::Declarator>,
) -> Option<Span> {
    let mut spans = declarator_spans.to_vec();
    spans.sort_by_key(|(_, span)| (span.start, span.end));
    let target_index = spans.iter().position(|(id, _)| *id == target_id)?;

    if spans.len() == 1 {
        return Some(expand_removal_span(source, statement_span));
    }

    if target_index + 1 < spans.len() {
        let start = spans[target_index].1.start;
        let end = spans[target_index + 1].1.start;
        return Some(Span::new(statement_span.file, start, end));
    }

    if target_index > 0 {
        let start = spans[target_index - 1].1.end;
        let end = spans[target_index].1.end;
        return Some(Span::new(statement_span.file, start, end));
    }

    None
}

/// Expand a removal span to include trailing whitespace and blank lines.
fn expand_removal_span(source: &str, statement_span: Span) -> Span {
    // move start to the beginning of the line
    let start = line_start_for_offset(source, statement_span.start as usize);
    let mut end = statement_span.end as usize;

    // extend to end of line when possible
    if let Some(rest) = source.get(end..)
        && let Some(line_end) = rest.find('\n')
    {
        end += line_end + 1;
    }

    // remove a trailing blank line when possible
    if let Some(rest) = source.get(end..) {
        if let Some(line_end) = rest.find('\n') {
            let line = &rest[..line_end];
            if line.trim().is_empty() {
                end += line_end + 1;
            }
        } else if rest.trim().is_empty() {
            end = source.len();
        }
    }

    Span::new(statement_span.file, start as u32, end as u32)
}

/// Format the inline expression text for replacement.
fn format_inline_expression(value: &str, expression: &dir::Expression) -> String {
    // trim whitespace and trailing semicolons
    let trimmed = value.trim().trim_end_matches(';').trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if should_parenthesize(trimmed, expression) {
        format!("({trimmed})")
    } else {
        trimmed.to_string()
    }
}

/// Decide whether an inline expression should be parenthesized.
fn should_parenthesize(value: &str, expression: &dir::Expression) -> bool {
    // avoid wrapping simple identifiers and member accesses
    if value.starts_with('(') && value.ends_with(')') {
        return false;
    }

    if expression_is_simple(expression) {
        return false;
    }

    let is_simple = value
        .chars()
        .all(|ch| ch.is_alphanumeric() || ch == '_' || ch == '.' || ch == '$');
    if is_simple {
        return false;
    }

    true
}

/// Check if an expression is simple enough to inline without parentheses.
fn expression_is_simple(expression: &dir::Expression) -> bool {
    matches!(
        expression,
        dir::Expression::LocalReference { .. }
            | dir::Expression::ModuleReference { .. }
            | dir::Expression::GlobalReference { .. }
            | dir::Expression::Member { .. }
            | dir::Expression::PrivateMember { .. }
            | dir::Expression::Index { .. }
            | dir::Expression::Call { .. }
            | dir::Expression::New { .. }
            | dir::Expression::ScalarLiteral { .. }
            | dir::Expression::ImportMeta
            | dir::Expression::This
            | dir::Expression::PrivateIdentifier { .. }
    )
}

/// Detect whether an expression produces side effects.
fn expression_has_side_effects(
    dir_tree: &dir::NodeTree,
    expr_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // walk the expression subtree and detect side effects
    let expr = dir_tree.get::<dir::Expression>(expr_id);
    let mut visitor = SideEffectVisitor::new();
    visitor.visit_expression(dir_tree, expr_id, expr);
    visitor.has_side_effects
}

/// Detect whether a symbol is assigned within a scope.
fn symbol_is_assigned(dir_tree: &dir::NodeTree, symbol_id: dir::GlobalSymbolId) -> bool {
    // scan for assignments to this symbol
    for (_expr_id, expr) in dir_tree.iter_nodes_of_type::<dir::Expression>() {
        let target_symbol = match expr {
            dir::Expression::Assign { left, .. } => assign_pattern_target_symbol(dir_tree, *left),
            dir::Expression::AssignBinary { left, .. } => expression_target_symbol(dir_tree, *left),
            _ => None,
        };

        if target_symbol == Some(symbol_id) {
            return true;
        }
    }

    false
}

/// Return the target symbol for one direct expression assignment target.
fn expression_target_symbol(
    dir_tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::GlobalSymbolId> {
    let expression = dir_tree.get::<dir::Expression>(expression_id);

    match expression {
        dir::Expression::LocalReference { target_symbol, .. }
        | dir::Expression::ModuleReference { target_symbol, .. }
        | dir::Expression::GlobalReference { target_symbol, .. } => Some(*target_symbol),
        _ => None,
    }
}

/// Return the target symbol for one assign pattern when it is a simple reference.
fn assign_pattern_target_symbol(
    dir_tree: &dir::NodeTree,
    assign_pattern_id: dir::LocalNodeId<dir::AssignPattern>,
) -> Option<dir::GlobalSymbolId> {
    let assign_pattern = dir_tree.get(assign_pattern_id);

    match assign_pattern {
        dir::AssignPattern::Expression { value } => expression_target_symbol(dir_tree, *value),
        dir::AssignPattern::Assign { pattern, .. } => {
            assign_pattern_target_symbol(dir_tree, *pattern)
        }
        dir::AssignPattern::Array { .. } | dir::AssignPattern::Object { .. } => None,
    }
}

/// Visitor that collects symbols captured by an inline value.
struct CapturedSymbolVisitor<'a> {
    /// The repository for symbol lookups.
    repository: &'a Repository,
    /// The revision for semantic lookups.
    revision: Revision,
    /// The symbol table for the current module.
    symbols: &'a dir::SymbolTable,
    /// The module id for symbol resolution.
    module_id: destack_source::ModuleId,
    /// The symbol being inlined.
    inline_symbol: dir::GlobalSymbolId,
    /// Collected captured symbols keyed by name.
    captured: &'a mut HashMap<dir::StaticKey, dir::GlobalSymbolId>,
    /// Whether an unresolved symbol was encountered.
    has_unknown: &'a mut bool,
    /// The visitor options for traversal.
    options: dir::NodeVisitorOptions,
}

impl<'a> CapturedSymbolVisitor<'a> {
    /// Create a visitor for captured symbols.
    fn new(
        repository: &'a Repository,
        revision: Revision,
        symbols: &'a dir::SymbolTable,
        module_id: ModuleId,
        inline_symbol: dir::GlobalSymbolId,
        captured: &'a mut HashMap<dir::StaticKey, dir::GlobalSymbolId>,
        has_unknown: &'a mut bool,
    ) -> Self {
        Self {
            repository,
            revision,
            symbols,
            module_id,
            inline_symbol,
            captured,
            has_unknown,
            options: dir::NodeVisitorOptions::default(),
        }
    }
}

impl dir::NodeVisitor for CapturedSymbolVisitor<'_> {
    /// Return visitor options.
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.options
    }

    /// Visit expressions and collect captured symbols.
    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        if *self.has_unknown {
            return;
        }

        if let Some(target_symbol) = expression.target_symbol() {
            let canonical = get_canonical_symbol(self.repository, self.revision, target_symbol);
            if canonical != self.inline_symbol {
                if target_symbol.module_id != self.module_id {
                    *self.has_unknown = true;
                    return;
                }

                let symbol = self.symbols.get_symbol(target_symbol.local_id);
                let Some(key) = symbol.key else {
                    *self.has_unknown = true;
                    return;
                };

                if let Some(existing) = self.captured.get(&key) {
                    if *existing != canonical {
                        *self.has_unknown = true;
                        return;
                    }
                } else {
                    self.captured.insert(key, canonical);
                }
            }
        }

        dir::walk_expression(self, tree, id, expression);
    }
}

/// Visitor that detects side effects in expressions.
struct SideEffectVisitor {
    /// Whether any side effect was found.
    has_side_effects: bool,
    /// The visitor options for traversal.
    options: dir::NodeVisitorOptions,
}

impl SideEffectVisitor {
    /// Create a new side effect visitor.
    fn new() -> Self {
        Self {
            has_side_effects: false,
            options: dir::NodeVisitorOptions::default(),
        }
    }
}

impl dir::NodeVisitor for SideEffectVisitor {
    /// Return visitor options.
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.options
    }

    /// Visit expressions and detect side effects.
    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        if self.has_side_effects {
            return;
        }

        if matches!(
            expression,
            dir::Expression::Call { .. }
                | dir::Expression::New { .. }
                | dir::Expression::Assign { .. }
                | dir::Expression::AssignBinary { .. }
                | dir::Expression::Delete { .. }
                | dir::Expression::Throw { .. }
                | dir::Expression::Await { .. }
                | dir::Expression::AwaitMaybe { .. }
                | dir::Expression::Yield { .. }
                | dir::Expression::Return { .. }
                | dir::Expression::Break { .. }
                | dir::Expression::Continue { .. }
                | dir::Expression::Loop { .. }
                | dir::Expression::ForEach { .. }
                | dir::Expression::For { .. }
        ) {
            self.has_side_effects = true;
            return;
        }

        dir::walk_expression(self, tree, id, expression);
    }
}
