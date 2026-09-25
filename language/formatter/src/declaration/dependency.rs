use crate::annotation::{
    block_infix_annotations, format_comment, infix_or_postfix_annotations, prefix_annotations,
    prefix_comment_nodes,
};
use crate::collection::TrailingSeparator;
use crate::collection::literal::format_scalar_literal;
use crate::collection::property::{can_unquote_name, format_name_with_quotes};
use crate::declaration::expression_needs_statement_terminator;
use crate::{FormatNode, TsppFormatContext, TsppFormatter};
use std::borrow::Cow;
use std::cmp::Ordering;
use tspp_core::{StringId, StringPool};
use tspp_dir::{
    DecoratorPosition, DependencyBinding, DependencyItem, Expression, ImportAttribute,
    ImportAttributeClause, ImportAttributeClauseKind, ImportAttributeValue, Keyword, Literal,
    LocalNodeId, Name, TokenSpan, TokenType, Tree,
};
use tspp_fir::format::{FormatError, FormatResult};
use tspp_fir::prelude::*;
use tspp_fir::write;
use tspp_repository::{ImportSortOrder, QuoteProperty, TrailingComma};
use tspp_source::{NodeSpanList, NodeSpanRegion, NodeSpanType, Span};

/// The import group category for declaration ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ImportGroup {
    /// Builtin modules with protocol prefixes.
    Builtin = 0,
    /// External packages.
    Package = 1,
    /// Path aliases.
    Alias = 2,
    /// Relative imports.
    Relative = 3,
}

impl ImportGroup {
    /// Categorize one import target path into a group.
    pub(crate) fn from_path(path: &str) -> Self {
        // builtin protocols: `protocol:module` but not urls
        if let Some(colon_position) = path.find(':')
            && !path[colon_position..].starts_with("://")
        {
            return Self::Builtin;
        }

        // relative imports
        if path.starts_with("./") || path.starts_with("../") || path.starts_with('/') {
            return Self::Relative;
        }

        // alias imports
        if is_alias_specifier(path) {
            return Self::Alias;
        }

        Self::Package
    }
}

/// Format a dependency item name.
fn format_dependency_item_name<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    name: Name,
) -> FormatResult<()> {
    match name {
        Name::Identifier(name) => {
            write!(f, [name])?;
        }
        Name::String(name) => {
            let literal = Literal::String(name);
            let span = Span::empty(f.context().file.id);
            format_scalar_literal(&literal, span, f)?;
        }
        Name::Index(index) => write!(f, [copied_text(&index.to_string())])?,
    }

    Ok(())
}

impl<'ast> FormatNode<'ast, DependencyItem> for DependencyItem {
    fn format_node(
        &self,
        node_id: LocalNodeId<DependencyItem>,
        f: &mut TsppFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let DependencyItem::Binding {
            binding,
            name,
            alias,
            value: _,
        } = self
        else {
            return Ok(());
        };

        write!(f, [prefix_annotations(f.context(), node_id)])?;

        let is_default_binding = *binding == DependencyBinding::Default
            || (*binding == DependencyBinding::Named && name.is_none());

        // default
        if is_default_binding {
            write!(f, [Keyword::Default])?;

            // alias
            if let Some(alias) = alias {
                write_dependency_item_alias_clause(f, node_id, *alias)?;
            }
        }
        // namespace
        else if *binding == DependencyBinding::Namespace {
            write!(f, [token("*")])?;
            if let Some(alias) = alias {
                write_dependency_item_alias_clause(f, node_id, *alias)?;
            }
        }
        // item
        else {
            // name
            if let Some(name) = name {
                format_dependency_item_name(f, *name)?;
            }

            // alias
            if let Some(alias) = alias {
                write_dependency_item_alias_clause(f, node_id, *alias)?;
            }
        }

        write!(f, [infix_or_postfix_annotations(f.context(), node_id)])?;

        Ok(())
    }
}

/// Return the inner import expression, unwrapping statement wrappers when needed.
pub(crate) fn import_expression(
    expr_id: LocalNodeId<Expression>,
    tree: &Tree,
) -> Option<&Expression> {
    let expr = tree.get(expr_id);
    match expr {
        Expression::Import { .. } => Some(expr),
        _ => None,
    }
}

/// Check if an expression is an import (unwrapping Statement if needed).
pub(crate) fn is_import(expr_id: LocalNodeId<Expression>, tree: &Tree) -> bool {
    import_expression(expr_id, tree).is_some()
}

/// Format one dependency-shaped statement expression.
pub(crate) fn format_dependency_statement_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> FormatResult<bool> {
    match expression {
        Expression::Import {
            target,
            items,
            attributes,
        } => {
            format_import_expression(f, node_id, *target, items.as_deref(), attributes.as_ref())?;
            Ok(true)
        }
        Expression::Export {
            target,
            items,
            attributes,
        } => {
            format_export_expression(f, node_id, *target, items, attributes.as_ref())?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// Sort import expressions by group and then alphabetically within each group.
///
/// Side-effect imports (no items) preserve their relative order and stay at the top.
pub(crate) fn sort_imports(
    imports: &[LocalNodeId<Expression>],
    tree: &Tree,
    strings: &StringPool,
) -> Vec<LocalNodeId<Expression>> {
    let mut imports = imports
        .iter()
        .copied()
        .filter_map(|expression_id| {
            let Expression::Import { items, target, .. } = import_expression(expression_id, tree)?
            else {
                return None;
            };

            Some((expression_id, strings.get(*target), items.is_none()))
        })
        .collect::<Vec<_>>();

    // preserve side effects first, then order regular targets canonically
    imports.sort_by(
        |(_, left_target, left_is_side_effect), (_, right_target, right_is_side_effect)| match (
            *left_is_side_effect,
            *right_is_side_effect,
        ) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => compare_import_targets(left_target, right_target),
        },
    );

    imports
        .into_iter()
        .map(|(expression_id, _, _)| expression_id)
        .collect()
}

/// Categorize one import target path into a group.
pub(crate) fn categorize_import(target: &str) -> ImportGroup {
    ImportGroup::from_path(target)
}

/// Compare two import targets by canonical declaration order.
pub(crate) fn compare_import_targets(left: &str, right: &str) -> Ordering {
    match categorize_import(left).cmp(&categorize_import(right)) {
        Ordering::Equal => left.cmp(right),
        ordering => ordering,
    }
}

/// Sort dependency items by kind and configured key order.
pub(crate) fn sort_dependency_items(
    items: &[LocalNodeId<DependencyItem>],
    tree: &Tree,
    strings: &StringPool,
    sort_order: ImportSortOrder,
) -> Vec<LocalNodeId<DependencyItem>> {
    let mut sorted_items = items.to_vec();
    sorted_items.sort_by(|left_id, right_id| {
        let left_item = tree.get(*left_id);
        let right_item = tree.get(*right_id);

        // sort by declared item key
        let left_key = dependency_item_sort_key(left_item)
            .map(|string_id| strings.get(string_id))
            .unwrap_or("");
        let right_key = dependency_item_sort_key(right_item)
            .map(|string_id| strings.get(string_id))
            .unwrap_or("");

        match sort_order {
            ImportSortOrder::Natural => natural_cmp(left_key, right_key),
            ImportSortOrder::Alphabetical => left_key.cmp(right_key),
        }
    });

    sorted_items
}

/// Determine if a blank line should be inserted between two imports.
///
/// Returns true if:
/// - Transitioning from side-effect to regular imports
/// - Different import groups (for regular imports)
pub(crate) fn should_insert_blank_between(
    prev_expr_id: LocalNodeId<Expression>,
    curr_expr_id: LocalNodeId<Expression>,
    tree: &Tree,
    strings: &StringPool,
) -> bool {
    let (prev_is_side_effect, prev_group) = match import_expression(prev_expr_id, tree) {
        Some(Expression::Import { items, target, .. }) => {
            let target_str = strings.get(*target);
            (items.is_none(), categorize_import(target_str))
        }
        _ => return false,
    };

    let (curr_is_side_effect, curr_group) = match import_expression(curr_expr_id, tree) {
        Some(Expression::Import { items, target, .. }) => {
            let target_str = strings.get(*target);
            (items.is_none(), categorize_import(target_str))
        }
        _ => return false,
    };

    // blank line between side effect and regular imports
    if prev_is_side_effect && !curr_is_side_effect {
        return true;
    }

    // blank line between different groups (for regular imports)
    if !prev_is_side_effect && !curr_is_side_effect && prev_group != curr_group {
        return true;
    }

    false
}

/// Return true when one import path uses a known alias prefix.
fn is_alias_specifier(specifier: &str) -> bool {
    specifier.starts_with("@/") || specifier.starts_with("~/") || specifier.starts_with('#')
}

/// Return one dependency item's sort key when present.
fn dependency_item_sort_key(item: &DependencyItem) -> Option<StringId> {
    match item {
        DependencyItem::Binding { alias, name, .. } => {
            alias.or_else(|| name.map(|name| name.string()))
        }
        DependencyItem::Error => None,
    }
}

/// Compare two strings using natural sort order.
fn natural_cmp(left: &str, right: &str) -> Ordering {
    let mut left_characters = left.chars().peekable();
    let mut right_characters = right.chars().peekable();

    loop {
        match (left_characters.peek(), right_characters.peek()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(left_character), Some(right_character)) => {
                // compare numeric runs as integers
                if left_character.is_ascii_digit() && right_character.is_ascii_digit() {
                    let mut left_number: u64 = 0;
                    while let Some(&character) = left_characters.peek()
                        && character.is_ascii_digit()
                    {
                        left_number = left_number
                            .saturating_mul(10)
                            .saturating_add((character as u64) - ('0' as u64));
                        left_characters.next();
                    }

                    let mut right_number: u64 = 0;
                    while let Some(&character) = right_characters.peek()
                        && character.is_ascii_digit()
                    {
                        right_number = right_number
                            .saturating_mul(10)
                            .saturating_add((character as u64) - ('0' as u64));
                        right_characters.next();
                    }

                    match left_number.cmp(&right_number) {
                        Ordering::Equal => continue,
                        ordering => return ordering,
                    }
                }

                // compare case insensitively first
                let left_lower = left_character.to_ascii_lowercase();
                let right_lower = right_character.to_ascii_lowercase();
                match left_lower.cmp(&right_lower) {
                    Ordering::Equal => match left_character.cmp(right_character) {
                        Ordering::Equal => {
                            left_characters.next();
                            right_characters.next();
                        }
                        ordering => return ordering,
                    },
                    ordering => return ordering,
                }
            }
        }
    }
}

/// Return one dependency item's binding when it is valid.
fn dependency_item_mode(item: &DependencyItem) -> Option<DependencyBinding> {
    match item {
        DependencyItem::Binding { binding, .. } => Some(*binding),
        DependencyItem::Error => None,
    }
}

/// Return one dependency item's alias when it is valid.
fn dependency_item_alias(item: &DependencyItem) -> Option<StringId> {
    match item {
        DependencyItem::Binding { alias, .. } => *alias,
        DependencyItem::Error => None,
    }
}

/// Return one dependency item's value when it is valid.
fn dependency_item_value(item: &DependencyItem) -> Option<LocalNodeId<Expression>> {
    match item {
        DependencyItem::Binding { value, .. } => *value,
        DependencyItem::Error => None,
    }
}

/// Return the earliest prefix start for one dependency item.
fn dependency_item_prefix_start(
    context: &TsppFormatContext<'_>,
    item_id: LocalNodeId<DependencyItem>,
) -> u32 {
    let mut start = context.span(item_id).start;

    for comment in prefix_comment_nodes(context, item_id) {
        start = start.min(comment.span.start);
    }

    for annotation_id in context.annotation_ids(item_id).iter().copied() {
        if matches!(
            context.annotation(annotation_id).position,
            DecoratorPosition::BlockPrefix | DecoratorPosition::LinePrefix
        ) {
            start = start.min(context.annotation_span(annotation_id).start);
        }
    }

    start
}

/// Return whether one dependency gap contains a blank line.
fn dependency_gap_has_blank_line(context: &TsppFormatContext<'_>, start: u32, end: u32) -> bool {
    if start >= end {
        return false;
    }

    context.has_blank_line(Span::new(context.file.id, start, end))
}

/// Write spacing for one dependency gap using source-relative line shape.
fn write_dependency_gap_spacing<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    start: u32,
    end: u32,
    should_allow_soft_break: bool,
) -> FormatResult<()> {
    if start < end {
        let gap_span = Span::new(f.context().file.id, start, end);

        // preserve blank separator lines
        if dependency_gap_has_blank_line(f.context(), start, end) {
            write!(f, [hard_line_break(), empty_line()])?;
            return Ok(());
        }

        // preserve single-line breaks
        if f.context().has_newline(gap_span) {
            write!(f, [hard_line_break()])?;
            return Ok(());
        }
    }

    if should_allow_soft_break {
        write!(f, [soft_line_break_or_space()])
    } else {
        write!(f, [space()])
    }
}

/// Write comments in one dependency gap and return the last emitted end position.
fn write_dependency_gap_comments<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    start: u32,
    end: u32,
) -> FormatResult<u32> {
    let mut previous_end = start;

    let comments = {
        let comment_cursor = f.context().comments();
        comment_cursor.comments_in_range(start, end).to_vec()
    };

    for comment in comments {
        let comment_span = comment.span;

        write_dependency_gap_spacing(f, previous_end, comment_span.start, false)?;
        format_comment(f, comment)?;

        previous_end = comment_span.end;
    }

    Ok(previous_end)
}

/// Write one dependency gap using local comments plus normalized spacing.
fn write_dependency_gap<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    start: u32,
    end: u32,
    should_allow_soft_break: bool,
) -> FormatResult<()> {
    if start >= end {
        if should_allow_soft_break {
            write!(f, [soft_line_break_or_space()])?;
        } else {
            write!(f, [space()])?;
        }

        return Ok(());
    }

    let has_comments = dependency_gap_has_comments(f.context(), start, end);
    let spacing_start = write_dependency_gap_comments(f, start, end)?;

    if spacing_start < end {
        write_dependency_gap_spacing(f, spacing_start, end, should_allow_soft_break)?;
    }
    // block comments that directly touch the next token still need one separator
    else if has_comments {
        if should_allow_soft_break {
            write!(f, [soft_line_break_or_space()])?;
        } else {
            write!(f, [space()])?;
        }
    }

    Ok(())
}

/// Write one dependency item alias clause with preserved comments.
fn write_dependency_item_alias_clause<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<DependencyItem>,
    alias: StringId,
) -> FormatResult<()> {
    let Some(alias_span) = f.context().tree.get_main_span(node_id) else {
        write!(f, [space(), Keyword::As, space(), alias])?;
        return Ok(());
    };

    let Some(as_token) = f.context().previous_token_before_span(alias_span) else {
        write!(f, [space(), Keyword::As, space(), alias])?;
        return Ok(());
    };
    let Some(anchor_token) = f.context().previous_token_before_span(as_token.span) else {
        write!(f, [space(), Keyword::As, space(), alias])?;
        return Ok(());
    };

    if f.context().token_keyword(as_token) != Some(Keyword::As) {
        write!(f, [space(), Keyword::As, space(), alias])?;
        return Ok(());
    }

    write_dependency_gap(f, anchor_token.span.end, as_token.span.start, false)?;
    write!(f, [Keyword::As])?;

    write_dependency_gap(f, as_token.span.end, alias_span.start, false)?;

    write!(f, [alias])
}

/// Write one import attribute value.
fn write_import_attribute_value<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    value: &ImportAttributeValue,
) -> FormatResult<()> {
    match value {
        // scalar
        ImportAttributeValue::Literal(value) => {
            let span = Span::empty(f.context().file.id);
            format_scalar_literal(value, span, f)
        }
        // array
        ImportAttributeValue::Array(values) => {
            write!(f, [token("[")])?;

            for (index, value) in values.iter().enumerate() {
                // separator
                if index > 0 {
                    write!(f, [token(","), space()])?;
                }

                // value
                write_import_attribute_value(f, value)?;
            }

            write!(f, [token("]")])
        }
        // object
        ImportAttributeValue::Object(attributes) => {
            let use_inner_space = f.context().options.bracket_spacing && !attributes.is_empty();
            let force_quote_keys = f.context().options.quote_props == QuoteProperty::Consistent
                && attributes
                    .iter()
                    .any(|attribute| import_attribute_key_requires_quotes(f.context(), attribute));

            write!(f, [token("{")])?;
            if use_inner_space {
                write!(f, [space()])?;
            }

            for (index, attribute) in attributes.iter().enumerate() {
                // separator
                if index > 0 {
                    write!(f, [token(","), space()])?;
                }

                // attribute
                format_import_attribute(f, attribute, force_quote_keys)?;
            }

            if use_inner_space {
                write!(f, [space()])?;
            }
            write!(f, [token("}")])
        }
        // invalid parser node
        ImportAttributeValue::Error => Err(FormatError::SyntaxError {
            message: "import attribute value contains a parser error",
        }),
    }
}

/// Write one import attribute entry.
fn format_import_attribute<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    attribute: &ImportAttribute,
    force_quote_keys: bool,
) -> FormatResult<()> {
    // key
    format_name_with_quotes(f, attribute.key, force_quote_keys)?;

    // value
    write!(f, [token(":"), space()])?;
    write_import_attribute_value(f, &attribute.value)
}

/// Return whether one import attribute key requires quotes.
fn import_attribute_key_requires_quotes(
    context: &TsppFormatContext<'_>,
    attribute: &ImportAttribute,
) -> bool {
    match attribute.key {
        Name::Identifier(_) | Name::Index(_) => false,
        Name::String(name) => !can_unquote_name(context.strings.get(name)),
    }
}

/// Format `with { ... }` arguments for import and export statements.
fn format_dependency_with_arguments<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    clause_kind: ImportAttributeClauseKind,
    attributes: &[ImportAttribute],
) -> FormatResult<()> {
    // annotations
    let has_attribute_head_annotation = f.context().has_infix_annotation(node_id);
    if has_attribute_head_annotation {
        write!(f, [block_infix_annotations(f.context(), node_id)])?;
    }
    let force_quote_keys = f.context().options.quote_props == QuoteProperty::Consistent
        && attributes
            .iter()
            .any(|attribute| import_attribute_key_requires_quotes(f.context(), attribute));
    let attribute_clause_span = f
        .context()
        .tree
        .get_side_span(node_id, NodeSpanType::Region(NodeSpanRegion::Attributes));
    let first_attribute_segment_span = f
        .context()
        .tree
        .get_side_span(node_id, NodeSpanType::ListItem(NodeSpanList::Entry, 0));
    let has_closing_comment = attribute_clause_span
        .is_some_and(|span| f.context().comments().has_comment_before(span.end));
    let should_use_block = attributes.len() > 1
        || attributes
            .first()
            .is_some_and(|attribute| f.context().strings.get(attribute.key.string()) != "type")
        || has_closing_comment;
    let should_expand = first_attribute_segment_span
        .is_some_and(|span| f.context().source_text().has_newline_before(span.start));

    // body
    let format_arguments = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        let use_inner_space = f.context().options.bracket_spacing && !attributes.is_empty();
        if use_inner_space {
            write!(f, [if_group_fits_on_line(&space())])?;
        }

        for (index, attribute) in attributes.iter().enumerate() {
            // separator
            if index > 0 {
                write!(f, [token(",")])?;
                if should_use_block {
                    write!(f, [soft_line_break_or_space()])?;
                } else {
                    write!(f, [space()])?;
                }
            }

            // entry
            format_import_attribute(f, attribute, force_quote_keys)?;
        }

        if use_inner_space {
            write!(f, [if_group_fits_on_line(&space())])?;
        }

        if should_use_block {
            write!(f, [if_group_breaks(&token(","))])?;
        }

        Ok(())
    });
    let with_arguments = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        write!(
            f,
            [token("{"), soft_block_indent(&format_arguments), token("}")]
        )
    });
    let with_arguments = group(&with_arguments).should_expand(should_expand);

    let clause_keyword = match clause_kind {
        ImportAttributeClauseKind::With => Keyword::With,
    };

    if has_attribute_head_annotation {
        write!(f, [clause_keyword, space(), with_arguments])?;
        return Ok(());
    }

    write!(f, [space(), clause_keyword, space(), with_arguments])
}

/// Return whether any dependency item in one list has annotations.
fn dependency_items_have_annotations(
    ctx: &TsppFormatContext<'_>,
    items: &[LocalNodeId<DependencyItem>],
) -> bool {
    items.iter().any(|item| ctx.has_annotation(*item))
}

/// Return whether one dependency item list has separator comments.
fn dependency_items_have_separator_signal(
    ctx: &TsppFormatContext<'_>,
    items: &[LocalNodeId<DependencyItem>],
) -> bool {
    items.windows(2).any(|item_pair| {
        let left_span = ctx.span(item_pair[0]);
        let right_span = ctx.span(item_pair[1]);

        !ctx.comments()
            .comments_in_range(left_span.end, right_span.start)
            .is_empty()
    })
}

/// Return whether one dependency gap contains comments.
fn dependency_gap_has_comments(context: &TsppFormatContext<'_>, start: u32, end: u32) -> bool {
    if start >= end {
        return false;
    }

    !context.comments().comments_in_range(start, end).is_empty()
}

/// Return one dependency item collection close-brace token.
fn dependency_item_collection_close_brace_token(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    items: &[LocalNodeId<DependencyItem>],
) -> Option<TokenSpan> {
    if let Some(last_item_id) = items.last().copied() {
        let mut candidate = context.next_token_after_span(context.span(last_item_id))?;
        if candidate.token.ty() == TokenType::Comma {
            candidate = context.next_token_after_span(candidate.span)?;
        }

        return (candidate.token.ty() == TokenType::CloseBrace).then_some(candidate);
    }

    let expression_span = context.span(node_id);
    let clause_end = context
        .tree
        .get_main_span(node_id)
        .and_then(|target_span| {
            context
                .previous_token_before_span(target_span)
                .map(|token| {
                    if context.token_keyword(token) == Some(Keyword::From) {
                        token.span.start
                    } else {
                        target_span.start
                    }
                })
        })
        .unwrap_or(expression_span.end);
    let clause_span = Span::new(expression_span.file, expression_span.start, clause_end);

    context
        .tokens_in_span(clause_span)
        .iter()
        .copied()
        .rev()
        .find(|token| token.token.ty() == TokenType::CloseBrace)
}

/// Return one dependency item collection open-brace token.
fn dependency_item_collection_open_brace_token(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    items: &[LocalNodeId<DependencyItem>],
) -> Option<TokenSpan> {
    if let Some(first_item_id) = items.first().copied() {
        let candidate = context.previous_token_before_span(context.span(first_item_id))?;
        return (candidate.token.ty() == TokenType::OpenBrace).then_some(candidate);
    }

    let close_brace = dependency_item_collection_close_brace_token(context, node_id, items)?;
    let expression_span = context.span(node_id);
    let clause_span = Span::new(
        expression_span.file,
        expression_span.start,
        close_brace.span.end,
    );

    context
        .tokens_in_span(clause_span)
        .iter()
        .copied()
        .find(|token| token.token.ty() == TokenType::OpenBrace)
}

/// Return whether one dependency item collection interior has preserved source signal.
fn dependency_item_collection_has_interior_signal(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    items: &[LocalNodeId<DependencyItem>],
) -> bool {
    let Some(open_brace) = dependency_item_collection_open_brace_token(context, node_id, items)
    else {
        return false;
    };
    let Some(close_brace) = dependency_item_collection_close_brace_token(context, node_id, items)
    else {
        return false;
    };
    if close_brace.span.start <= open_brace.span.end {
        return false;
    }

    if items.is_empty() {
        let interior_span = Span::new(
            open_brace.span.file,
            open_brace.span.end,
            close_brace.span.start,
        );

        return dependency_gap_has_comments(context, interior_span.start, interior_span.end)
            || context.has_newline(interior_span);
    }

    let Some((&first_item, remaining_items)) = items.split_first() else {
        return false;
    };
    let last_item = remaining_items.last().copied().unwrap_or(first_item);
    let first_item_span = context.span(first_item);
    let leading_span = Span::new(
        open_brace.span.file,
        open_brace.span.end,
        first_item_span.start,
    );
    if dependency_gap_has_comments(context, leading_span.start, leading_span.end) {
        return true;
    }

    let trailing_start = context
        .previous_token_before_span(close_brace.span)
        .filter(|token| token.token.ty() == TokenType::Comma)
        .map_or_else(|| context.span(last_item).end, |token| token.span.end);
    let trailing_span = Span::new(
        close_brace.span.file,
        trailing_start,
        close_brace.span.start,
    );

    dependency_gap_has_comments(context, trailing_span.start, trailing_span.end)
}

/// Return dependency items in output order with optional organize-imports sorting.
fn dependency_items_for_output<'a>(
    context: &TsppFormatContext<'_>,
    items: &'a [LocalNodeId<DependencyItem>],
    has_item_annotations: bool,
) -> Cow<'a, [LocalNodeId<DependencyItem>]> {
    let has_separator_signal = dependency_items_have_separator_signal(context, items);
    let should_sort = context.options.organize_imports.is_enabled()
        && !has_item_annotations
        && !has_separator_signal;

    if should_sort {
        let sorted_items = sort_dependency_items(
            items,
            context.tree,
            context.strings,
            context.options.import_sort_order,
        );

        return Cow::Owned(sorted_items);
    }

    Cow::Borrowed(items)
}

/// Write one dependency item list body with preserved separator comments.
fn write_dependency_item_entries<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    open_brace: Option<TokenSpan>,
    items: &[LocalNodeId<DependencyItem>],
    trailing_separator: TrailingSeparator,
) -> FormatResult<()> {
    for (index, item_id) in items.iter().copied().enumerate() {
        if index == 0
            && let Some(open_brace) = open_brace
        {
            let item_start = dependency_item_prefix_start(f.context(), item_id);
            if open_brace.span.end >= item_start
                || !dependency_gap_has_comments(f.context(), open_brace.span.end, item_start)
            {
                if f.context().options.bracket_spacing {
                    write!(f, [if_group_fits_on_line(&space())])?;
                }
            } else {
                write_dependency_gap_spacing(f, open_brace.span.end, item_start, true)?;
            }
        }

        write!(f, [item_id])?;

        let Some(next_item_id) = items.get(index + 1).copied() else {
            break;
        };

        write!(f, [token(",")])?;

        let item_span = f.context().span(item_id);
        let next_item_start = dependency_item_prefix_start(f.context(), next_item_id);
        let gap_start = f
            .context()
            .next_token_after_span(item_span)
            .filter(|token| {
                token.token.ty() == TokenType::Comma && token.span.end <= next_item_start
            })
            .map_or(item_span.end, |token| token.span.end);

        if dependency_gap_has_comments(f.context(), gap_start, next_item_start) {
            write_dependency_gap(f, gap_start, next_item_start, true)?;
        } else {
            write!(f, [soft_line_break_or_space()])?;
        }
    }

    // trailing separator
    if trailing_separator == TrailingSeparator::Allowed {
        write!(f, [if_group_breaks(&token(","))])?;
    }

    Ok(())
}

/// Write one reordered dependency item list body.
fn write_reordered_dependency_item_entries<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    items: &[LocalNodeId<DependencyItem>],
    trailing_separator: TrailingSeparator,
) -> FormatResult<()> {
    for (index, item_id) in items.iter().copied().enumerate() {
        if index == 0 && f.context().options.bracket_spacing {
            write!(f, [if_group_fits_on_line(&space())])?;
        }

        write!(f, [item_id])?;

        if index + 1 < items.len() {
            write!(f, [token(","), soft_line_break_or_space()])?;
        }
    }

    if trailing_separator == TrailingSeparator::Allowed {
        write!(f, [if_group_breaks(&token(","))])?;
    }

    Ok(())
}

/// Write one dependency item collection list with stable expansion rules.
fn write_dependency_item_collection<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    source_items: &[LocalNodeId<DependencyItem>],
    items: &[LocalNodeId<DependencyItem>],
    should_expand: bool,
) -> FormatResult<()> {
    let trailing_separator = match f.context().options.trailing_comma {
        TrailingComma::None => TrailingSeparator::Omit,
        TrailingComma::Es5 | TrailingComma::All => TrailingSeparator::Allowed,
    };

    let is_source_order = source_items == items;
    let open_brace =
        dependency_item_collection_open_brace_token(f.context(), node_id, source_items);
    let close_brace =
        dependency_item_collection_close_brace_token(f.context(), node_id, source_items);

    if let Some(open_brace) = open_brace
        && let Some(anchor_token) = f.context().previous_token_before_span(Span::new(
            open_brace.span.file,
            open_brace.span.start,
            open_brace.span.start,
        ))
    {
        if anchor_token.span.end >= open_brace.span.start {
            write!(f, [space()])?;
        } else {
            write_dependency_gap(f, anchor_token.span.end, open_brace.span.start, false)?;
        }
    }

    write!(
        f,
        [group(&format_with(|f: &mut TsppFormatter<'ast, '_>| {
            write!(f, [token("{")])?;

            let format_interior = format_with(|f: &mut TsppFormatter<'ast, '_>| {
                if !items.is_empty() {
                    if is_source_order {
                        write_dependency_item_entries(f, open_brace, items, trailing_separator)?;
                    } else {
                        write_reordered_dependency_item_entries(f, items, trailing_separator)?;
                    }

                    if let Some(close_brace) = close_brace {
                        let last_item = items[items.len() - 1];
                        let last_item_end = f.context().span(last_item).end;
                        if is_source_order {
                            let trailing_start = if trailing_separator == TrailingSeparator::Allowed
                            {
                                f.context()
                                    .previous_token_before_span(close_brace.span)
                                    .filter(|token| token.token.ty() == TokenType::Comma)
                                    .map_or(last_item_end, |token| token.span.end)
                            } else {
                                last_item_end
                            };

                            if trailing_start >= close_brace.span.start
                                || !dependency_gap_has_comments(
                                    f.context(),
                                    trailing_start,
                                    close_brace.span.start,
                                )
                            {
                                if f.context().options.bracket_spacing {
                                    write!(f, [if_group_fits_on_line(&space())])?;
                                }
                            } else {
                                write_dependency_gap(
                                    f,
                                    trailing_start,
                                    close_brace.span.start,
                                    true,
                                )?;
                            }
                        } else if f.context().options.bracket_spacing {
                            write!(f, [if_group_fits_on_line(&space())])?;
                        }
                    } else if f.context().options.bracket_spacing {
                        write!(f, [if_group_fits_on_line(&space())])?;
                    }
                } else if let (Some(open_brace), Some(close_brace)) = (open_brace, close_brace) {
                    let interior_span = Span::new(
                        open_brace.span.file,
                        open_brace.span.end,
                        close_brace.span.start,
                    );
                    let has_interior_comments = !f
                        .context()
                        .comments()
                        .comments_in_range(interior_span.start, interior_span.end)
                        .is_empty();
                    let has_interior_newline = f.context().has_newline(interior_span);

                    if has_interior_comments || has_interior_newline {
                        write_dependency_gap(f, open_brace.span.end, close_brace.span.start, true)?;
                    }
                } else if f.context().options.bracket_spacing {
                    write!(f, [if_group_fits_on_line(&space())])?;
                }

                Ok(())
            });

            write!(f, [soft_block_indent(&format_interior), token("}")])
        }))
        .should_expand(should_expand)]
    )
}

/// Write one dependency item collection using shared output ordering options.
fn write_dependency_items<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    items: &[LocalNodeId<DependencyItem>],
) -> FormatResult<()> {
    let has_item_annotations = dependency_items_have_annotations(f.context(), items);
    let has_separator_signal = dependency_items_have_separator_signal(f.context(), items);
    let has_interior_signal =
        dependency_item_collection_has_interior_signal(f.context(), node_id, items);
    let output_items = dependency_items_for_output(f.context(), items, has_item_annotations);

    write_dependency_item_collection(
        f,
        node_id,
        items,
        output_items.as_ref(),
        has_item_annotations || has_separator_signal || has_interior_signal,
    )
}

/// Write one quoted dependency source target.
fn write_dependency_target<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    target: StringId,
) -> FormatResult<()> {
    write!(f, [token("\""), target, token("\"")])
}

/// Write one `from "<target>"` dependency source clause.
fn write_dependency_from_target_clause<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    target: StringId,
) -> FormatResult<()> {
    let Some(target_span) = f.context().tree.get_main_span(node_id) else {
        write!(f, [space(), Keyword::From, space()])?;
        return write_dependency_target(f, target);
    };
    let Some(from_token) = f.context().previous_token_before_span(target_span) else {
        write!(f, [space(), Keyword::From, space()])?;
        return write_dependency_target(f, target);
    };
    let Some(anchor_token) = f.context().previous_token_before_span(from_token.span) else {
        write!(f, [space(), Keyword::From, space()])?;
        return write_dependency_target(f, target);
    };

    write_dependency_gap(f, anchor_token.span.end, from_token.span.start, false)?;
    write!(f, [Keyword::From])?;

    write_dependency_gap(f, from_token.span.end, target_span.start, false)?;

    write_dependency_target(f, target)
}

/// Write one side-effect dependency target after `import`.
fn write_dependency_direct_target_clause<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    target: StringId,
) -> FormatResult<()> {
    let Some(target_span) = f.context().tree.get_main_span(node_id) else {
        write!(f, [space()])?;
        return write_dependency_target(f, target);
    };
    let Some(anchor_token) = f.context().previous_token_before_span(target_span) else {
        write!(f, [space()])?;
        return write_dependency_target(f, target);
    };

    write_dependency_gap(f, anchor_token.span.end, target_span.start, false)?;

    write_dependency_target(f, target)
}

/// Write one optional dependency attribute clause.
fn write_dependency_attribute_clause<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    attributes: Option<&ImportAttributeClause>,
) -> FormatResult<()> {
    let Some(attributes) = attributes else {
        return Ok(());
    };

    format_dependency_with_arguments(f, node_id, attributes.kind, &attributes.attributes)
}

/// Write one namespace import clause.
fn write_namespace_import_clause<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    first_item: &DependencyItem,
) -> FormatResult<()> {
    let namespace_alias = dependency_item_alias(first_item).ok_or(FormatError::SyntaxError {
        message: "namespace import requires an alias",
    })?;

    write!(
        f,
        [
            space(),
            token("*"),
            space(),
            Keyword::As,
            space(),
            namespace_alias
        ]
    )
}

/// Write one default-led import clause.
fn write_default_import_clause<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    first_item: &DependencyItem,
    rest_items: &[LocalNodeId<DependencyItem>],
) -> FormatResult<()> {
    let tree = f.context().tree;
    let default_alias = dependency_item_alias(first_item).ok_or(FormatError::SyntaxError {
        message: "default import requires an alias",
    })?;

    write!(f, [space(), default_alias])?;

    if rest_items.len() == 1
        && dependency_item_mode(tree.get(rest_items[0])) == Some(DependencyBinding::Namespace)
    {
        let namespace_item = tree.get(rest_items[0]);
        let namespace_alias =
            dependency_item_alias(namespace_item).ok_or(FormatError::SyntaxError {
                message: "namespace import requires an alias",
            })?;

        return write!(
            f,
            [
                token(","),
                space(),
                token("*"),
                space(),
                Keyword::As,
                space(),
                namespace_alias
            ]
        );
    }

    if rest_items.is_empty() {
        return Ok(());
    }

    write!(f, [token(",")])?;
    write_dependency_items(f, node_id, rest_items)
}

/// Write one normal import clause after the `import` keyword.
fn write_import_clause<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    items: &[LocalNodeId<DependencyItem>],
    has_item_clause: bool,
) -> FormatResult<()> {
    let tree = f.context().tree;
    let import_empty_items = has_item_clause && items.is_empty();
    let first_item = items.first().map(|item| tree.get(*item));

    if items.len() == 1
        && first_item
            .is_some_and(|item| dependency_item_mode(item) == Some(DependencyBinding::Namespace))
    {
        let Some(first_item) = first_item else {
            return Err(FormatError::SyntaxError {
                message: "namespace import requires at least one dependency item",
            });
        };

        return write_namespace_import_clause(f, first_item);
    }

    if let Some(first_item) = first_item
        && dependency_item_mode(first_item) == Some(DependencyBinding::Default)
    {
        return write_default_import_clause(f, node_id, first_item, &items[1..]);
    }

    if !items.is_empty() || import_empty_items {
        return write_dependency_items(f, node_id, items);
    }

    Ok(())
}

/// Write one export clause after the `export` keyword.
fn write_export_clause<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    items: &[LocalNodeId<DependencyItem>],
    target: Option<StringId>,
) -> FormatResult<bool> {
    let tree = f.context().tree;
    let export_empty_items_with_target = items.is_empty() && target.is_some();
    let first_item = items.first().map(|item| tree.get(*item));

    if items.len() == 1
        && first_item.is_some_and(|item| {
            dependency_item_mode(item) == Some(DependencyBinding::Default)
                && dependency_item_value(item).is_some()
        })
    {
        let Some(first_item) = first_item else {
            return Err(FormatError::SyntaxError {
                message: "default export requires at least one dependency item",
            });
        };
        let Some(value) = dependency_item_value(first_item) else {
            return Err(FormatError::SyntaxError {
                message: "default export requires a dependency value",
            });
        };

        write!(f, [space(), Keyword::Default, space(), value])?;

        let value_expression = f.context().tree.get(value);
        let needs_statement_terminator =
            expression_needs_statement_terminator(f.context(), value_expression, false);

        return Ok(needs_statement_terminator);
    }

    if items.len() == 1
        && first_item
            .is_some_and(|item| dependency_item_mode(item) == Some(DependencyBinding::Namespace))
    {
        let Some(first_item) = first_item else {
            return Err(FormatError::SyntaxError {
                message: "namespace export requires at least one dependency item",
            });
        };

        if dependency_item_value(first_item).is_some() && target.is_none() {
            let Some(value) = dependency_item_value(first_item) else {
                return Err(FormatError::SyntaxError {
                    message: "namespace export assignment requires a dependency value",
                });
            };

            write!(f, [space(), token("="), space(), value])?;
            return Ok(true);
        }

        write!(f, [space(), token("*")])?;
        if let Some(alias) = dependency_item_alias(first_item) {
            write!(f, [space(), Keyword::As, space(), alias])?;
        }
        return Ok(false);
    }

    if let Some(first_item) = first_item
        && dependency_item_mode(first_item) == Some(DependencyBinding::Default)
        && items.len() == 2
        && target.is_some()
        && dependency_item_mode(tree.get(items[1])) == Some(DependencyBinding::Namespace)
    {
        let default_alias = dependency_item_alias(first_item).ok_or(FormatError::SyntaxError {
            message: "default re-export requires an alias",
        })?;
        let namespace_item = tree.get(items[1]);
        let namespace_alias =
            dependency_item_alias(namespace_item).ok_or(FormatError::SyntaxError {
                message: "namespace re-export requires an alias",
            })?;

        write!(
            f,
            [
                space(),
                default_alias,
                token(","),
                space(),
                token("*"),
                space(),
                Keyword::As,
                space(),
                namespace_alias
            ]
        )?;
        return Ok(false);
    }

    if !items.is_empty() || target.is_none() || export_empty_items_with_target {
        write_dependency_items(f, node_id, items)?;
    }

    Ok(false)
}
/// Write one ordinary import statement body.
fn write_import_declaration_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    target: StringId,
    items: &[LocalNodeId<DependencyItem>],
    has_item_clause: bool,
    attributes: Option<&ImportAttributeClause>,
) -> FormatResult<()> {
    write!(f, [Keyword::Import])?;

    write_import_clause(f, node_id, items, has_item_clause)?;

    if has_item_clause {
        write_dependency_from_target_clause(f, node_id, target)?;
    } else {
        write_dependency_direct_target_clause(f, node_id, target)?;
    }

    write_dependency_attribute_clause(f, node_id, attributes)?;

    Ok(())
}

/// Format an import expression.
pub(crate) fn format_import_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    target: StringId,
    items: Option<&[LocalNodeId<DependencyItem>]>,
    attributes: Option<&ImportAttributeClause>,
) -> FormatResult<()> {
    let has_item_clause = items.is_some();
    let items = items.unwrap_or(&[]);

    write_import_declaration_expression(f, node_id, target, items, has_item_clause, attributes)
}

/// Write one export declaration body.
fn write_export_declaration_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    target: Option<StringId>,
    items: &[LocalNodeId<DependencyItem>],
    attributes: Option<&ImportAttributeClause>,
) -> FormatResult<()> {
    write!(f, [Keyword::Export])?;

    let needs_trailing_semicolon = write_export_clause(f, node_id, items, target)?;

    if let Some(target) = target {
        write_dependency_from_target_clause(f, node_id, target)?;
    }

    write_dependency_attribute_clause(f, node_id, attributes)?;

    if needs_trailing_semicolon {
        write!(f, [token(";")])?;
    }

    Ok(())
}

/// Format an export expression.
pub(crate) fn format_export_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    target: Option<StringId>,
    items: &[LocalNodeId<DependencyItem>],
    attributes: Option<&ImportAttributeClause>,
) -> FormatResult<()> {
    write_export_declaration_expression(f, node_id, target, items, attributes)
}
