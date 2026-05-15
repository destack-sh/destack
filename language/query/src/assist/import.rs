use std::collections::HashSet;

use destack_dir as dir;
use destack_source::{ModuleId, ModuleRelation, Span};
use destack_workspace::Repository;

use crate::core::{DirQueryContext, SourceQueryContext};
use crate::dir::import_clause_bounds;
use crate::source::{
    enclosing_spans_at_cursor, extract_string_literal_prefix, previous_significant_token,
    token_span_at_cursor_offset, token_text,
};

use super::CompletionContext;

/// Detect import related context.
pub(super) fn detect_import_context(
    repository: &Repository,
    parsed: SourceQueryContext<'_>,
    dir: DirQueryContext<'_>,
    source: &str,
    offset: u32,
) -> Option<CompletionContext> {
    // resolve enclosing spans from innermost to outermost
    let enclosing = enclosing_spans_at_cursor(parsed, offset);

    // scan enclosing expressions for import nodes under the cursor
    for enc in &enclosing {
        if parsed.tree().get_node_type(enc.idx) != dir::NodeType::Expression {
            continue;
        }

        let expr_id = dir::LocalNodeId::<dir::Expression>::new(enc.idx);
        let expr = parsed.tree().get(expr_id);

        if !matches!(expr, dir::Expression::Import { .. }) {
            continue;
        }

        // detect path completions inside the import string
        let import_span = parsed.tree().source_map.get(expr_id.id);
        let main_span = parsed.tree().source_map.get_main(expr_id.id);
        if let Some(span) = main_span
            && span.contains(offset)
        {
            let partial_path = extract_string_literal_prefix(source, span, offset);
            return Some(CompletionContext::ImportPath { partial_path });
        }

        if let Some(span) = import_path_span_from_tokens(parsed, import_span, offset) {
            let partial_path = extract_string_literal_prefix(source, span, offset);
            return Some(CompletionContext::ImportPath { partial_path });
        }

        // detect import clause completions inside the brace list
        if let Some(info) = import_clause_info(parsed, expr, offset, source, import_span, main_span)
        {
            let dir::Expression::Import { target, .. } = expr else {
                continue;
            };

            let target_module = resolved_import_target_module(repository, parsed, dir, *target);

            return Some(CompletionContext::ImportClause {
                target_module,
                existing_names: info.existing_names,
                space_filter: info.space_filter,
            });
        }
    }

    None
}

/// Resolve a string literal span for an import path at the cursor.
fn import_path_span_from_tokens(
    parsed: SourceQueryContext<'_>,
    import_span: Span,
    offset: u32,
) -> Option<Span> {
    // find the token under the cursor
    let token = token_span_at_cursor_offset(parsed, offset)?;

    // require the token to stay inside the import statement span
    if token.span.start < import_span.start || token.span.end > import_span.end {
        return None;
    }

    // require a string literal token
    if token.token.ty != dir::TokenType::Literal {
        return None;
    }

    if !matches!(token.token.literal, Some(dir::TokenLiteral::String { .. })) {
        return None;
    }

    Some(token.span)
}

/// Resolve one import target module from compiler resolved import edges.
fn resolved_import_target_module(
    _repository: &Repository,
    parsed: SourceQueryContext<'_>,
    dir: DirQueryContext<'_>,
    target: destack_core::StringId,
) -> Option<ModuleId> {
    let specifier = parsed.strings().get(target);
    let target_id = dir.strings().intern(specifier);
    let dependency = dir.imported().dependencies.iter().find(|dependency| {
        dependency.specifier == target_id
            && dependency.relation == ModuleRelation::Import
            && dependency.loader.is_none()
    })?;

    dependency.target.module_id()
}

/// Parsed import clause info for completion.
struct ImportClauseInfo {
    /// Existing names in the clause.
    existing_names: Vec<String>,
    /// Optional symbol space filter.
    space_filter: Option<dir::SymbolSpace>,
}

/// Extract import clause info at the given offset.
fn import_clause_info(
    parsed: SourceQueryContext<'_>,
    expr: &dir::Expression,
    offset: u32,
    source: &str,
    import_span: Span,
    target_span: Option<Span>,
) -> Option<ImportClauseInfo> {
    // require an import expression
    let dir::Expression::Import { items, space, .. } = expr else {
        return None;
    };
    let items = items.as_ref()?;

    // resolve the import clause braces before the target string
    let bounds = import_clause_bounds(parsed, import_span, target_span)?;
    let open_brace = bounds.open_brace;
    let end_boundary = bounds.end_boundary;

    // detect cursor inside the clause braces
    let cursor_in_clause = offset >= open_brace.end && offset <= end_boundary.start;

    // collect existing names and detect item space under the cursor
    let mut existing_names = Vec::new();
    let mut in_item_space = None;

    for item_id in items {
        let item = parsed.tree().get(*item_id);
        let span = parsed.tree().source_map.get(item_id.id);

        let (item_space, item_name, item_alias) = match item {
            dir::DependencyItem::Item {
                space, name, alias, ..
            } => (*space, *name, *alias),
            dir::DependencyItem::Error => continue,
        };

        if span.contains(offset) {
            in_item_space = item_space;
            continue;
        }

        if let Some(name_id) = item_name {
            existing_names.push(parsed.strings().get(name_id.string()).to_string());
        }

        if let Some(alias_id) = item_alias {
            existing_names.push(parsed.strings().get(alias_id).to_string());
        }
    }

    if !cursor_in_clause && in_item_space.is_none() {
        return None;
    }

    // detect cursor after a type keyword inside the clause
    let cursor_is_type = if cursor_in_clause {
        if let Some(token) = previous_significant_token(parsed, offset) {
            let token_in_clause =
                token.span.start >= open_brace.start && token.span.end <= end_boundary.end;
            token_in_clause
                && token.token.ty == dir::TokenType::Identifier
                && token_text(source, token.span) == Some("type")
        } else {
            false
        }
    } else {
        false
    };

    let space_filter = match space {
        dir::DependencySpace::Type => Some(dir::SymbolSpace::Type),
        dir::DependencySpace::Value => match in_item_space {
            Some(dir::DependencySpace::Type) => Some(dir::SymbolSpace::Type),
            Some(dir::DependencySpace::Value) => Some(dir::SymbolSpace::Value),
            None if cursor_is_type => Some(dir::SymbolSpace::Type),
            None => None,
        },
    };

    // deduplicate existing names to keep behavior stable
    deduplicate_names(&mut existing_names);

    Some(ImportClauseInfo {
        existing_names,
        space_filter,
    })
}

/// Deduplicate names while preserving their first occurrence order.
fn deduplicate_names(names: &mut Vec<String>) {
    // retain the first occurrence of each name
    let mut deduped = HashSet::new();
    names.retain(|name| deduped.insert(name.clone()));
}
