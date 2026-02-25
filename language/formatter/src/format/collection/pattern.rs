use std::borrow::Cow;

use destack_fir::format::FormatResult;

use crate::format::collection::{CollectionBreakScore, list_like};
use crate::{Annotation, DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{Expression, LocalNodeId, Mutability, NodeTree, NodeType, Pattern, PatternField};
use destack_fir::prelude::*;
use destack_fir::{format_args, write};

// object pattern expansion thresholds
const OBJECT_PATTERN_FORCE_EXPAND_MIN_FIELDS: usize = 3;
const OBJECT_PATTERN_INLINE_MAX_FIELDS: usize = 1;

impl<'ast> Format<DestackFormatContext<'ast>> for Mutability {
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Mutability::Immutable => write!(f, [token("const")]),
            Mutability::Mutable => write!(f, [token("var")]),
        }
    }
}

/// Decide whether a default expression in a pattern field should prefer multiline layout.
fn default_expression_prefers_multiline(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    matches!(
        tree.get(expression_id),
        Expression::ObjectExpression { .. } | Expression::ArrayExpression { .. }
    )
}

/// Decide whether a pattern has nested structure that benefits from multiline layout.
fn pattern_prefers_multiline(tree: &NodeTree, pattern_id: LocalNodeId<Pattern>) -> bool {
    match tree.get(pattern_id) {
        Pattern::Binding {
            pattern: Some(pattern),
            ..
        } => pattern_prefers_multiline(tree, *pattern),
        Pattern::Object { fields } | Pattern::TaggedObject { fields, .. } => fields
            .iter()
            .copied()
            .any(|field_id| pattern_field_prefers_multiline(tree, field_id)),
        Pattern::Array { fields }
        | Pattern::Tuple { fields }
        | Pattern::TaggedTuple { fields, .. } => fields
            .iter()
            .copied()
            .any(|field_id| pattern_field_prefers_multiline(tree, field_id)),
        _ => false,
    }
}

/// Decide whether a pattern field has nested structure that benefits from multiline layout.
fn pattern_field_prefers_multiline(tree: &NodeTree, field_id: LocalNodeId<PatternField>) -> bool {
    match tree.get(field_id) {
        PatternField::Named {
            pattern, default, ..
        }
        | PatternField::Computed {
            pattern, default, ..
        } => {
            pattern.is_some_and(|pattern_id| pattern_prefers_multiline(tree, pattern_id))
                || default.is_some_and(|default_id| {
                    default_expression_prefers_multiline(tree, default_id)
                })
        }
        PatternField::Alias { default, .. } => {
            default.is_some_and(|default_id| default_expression_prefers_multiline(tree, default_id))
        }
        PatternField::Positional { pattern, default } => {
            pattern_prefers_multiline(tree, *pattern)
                || default.is_some_and(|default_id| {
                    default_expression_prefers_multiline(tree, default_id)
                })
        }
        PatternField::Spread { pattern, .. } => {
            pattern.is_some_and(|pattern_id| pattern_prefers_multiline(tree, pattern_id))
        }
        PatternField::Elision => false,
    }
}

/// Return whether a pattern source is multiline inside its delimiters.
fn pattern_is_multiline_span(
    context: &DestackFormatContext<'_>,
    pattern_id: LocalNodeId<Pattern>,
) -> bool {
    let span = context.span(pattern_id);
    context.has_newline(span)
}

/// Decide whether a pattern field default should force expanded formatting.
fn should_expand_pattern_field_default<'ast>(
    f: &DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<PatternField>,
    default_id: LocalNodeId<Expression>,
) -> bool {
    let default_is_collection = matches!(
        f.context().tree.get(default_id),
        Expression::ObjectExpression { properties, .. } if !properties.is_empty()
    ) || matches!(
        f.context().tree.get(default_id),
        Expression::ArrayExpression { elements } if !elements.is_empty()
    );
    if !default_is_collection {
        return false;
    }

    let Some((parent_id, parent_type)) = f.context().parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Pattern {
        return false;
    }

    let parent_pattern = LocalNodeId::<Pattern>::new(parent_id);
    f.context().node_has_newline(parent_pattern)
}

/// Decide whether an object parameter pattern should expand for readability.
fn should_expand_parameter_object_pattern(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Pattern>,
    fields: &[LocalNodeId<PatternField>],
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Parameter {
        return false;
    }

    let parameter = context
        .tree
        .get(LocalNodeId::<destack_ast::Parameter>::new(parent_id));
    let parameter_pattern = match parameter {
        destack_ast::Parameter::Pattern { pattern, .. }
        | destack_ast::Parameter::VariadicPattern { pattern, .. } => Some(*pattern),
        destack_ast::Parameter::Named { .. } | destack_ast::Parameter::VariadicNamed { .. } => None,
    };
    if parameter_pattern.is_none_or(|pattern_id| pattern_id.id != node_id.id) {
        return false;
    }

    if fields
        .iter()
        .copied()
        .any(|field_id| pattern_field_prefers_multiline(context.tree, field_id))
    {
        return true;
    }

    if fields.len() >= OBJECT_PATTERN_FORCE_EXPAND_MIN_FIELDS {
        return true;
    }

    if fields.len() <= OBJECT_PATTERN_INLINE_MAX_FIELDS {
        return false;
    }

    fields
        .iter()
        .any(|field_id| match context.tree.get(*field_id) {
            PatternField::Named { default, .. }
            | PatternField::Computed { default, .. }
            | PatternField::Alias { default, .. } => default.is_some(),
            PatternField::Positional { .. }
            | PatternField::Spread { .. }
            | PatternField::Elision => false,
        })
}

/// Return object-pattern fields to render, normalizing out parser elision artifacts.
fn object_pattern_render_fields<'a>(
    tree: &NodeTree,
    fields: &'a [LocalNodeId<PatternField>],
) -> Cow<'a, [LocalNodeId<PatternField>]> {
    let has_elision = fields
        .iter()
        .copied()
        .any(|field_id| matches!(tree.get(field_id), PatternField::Elision));

    if !has_elision {
        return Cow::Borrowed(fields);
    }

    Cow::Owned(
        fields
            .iter()
            .copied()
            .filter(|field_id| !matches!(tree.get(*field_id), PatternField::Elision))
            .collect(),
    )
}

/// Return whether trailing separators are invalid for the current pattern field list.
fn pattern_fields_disallow_trailing_separator(
    tree: &NodeTree,
    fields: &[LocalNodeId<PatternField>],
) -> bool {
    fields
        .last()
        .is_some_and(|field_id| matches!(tree.get(*field_id), PatternField::Spread { .. }))
}

/// Return whether any pattern field has one default assignment.
fn pattern_fields_have_default_assignments(
    tree: &NodeTree,
    fields: &[LocalNodeId<PatternField>],
) -> bool {
    fields.iter().any(|field_id| match tree.get(*field_id) {
        PatternField::Named { default, .. }
        | PatternField::Computed { default, .. }
        | PatternField::Alias { default, .. }
        | PatternField::Positional { default, .. } => default.is_some(),
        PatternField::Spread { .. } | PatternField::Elision => false,
    })
}

/// Format a prefix pattern like `&pattern` or `^pattern`.
fn format_prefixed_pattern<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    prefix: &'static str,
    right: LocalNodeId<Pattern>,
    mutability: Option<Mutability>,
) -> FormatResult<()> {
    write!(f, [token(prefix)])?;

    if mutability == Some(Mutability::Immutable) {
        write!(f, [token("readonly"), space()])?;
    }

    write!(f, [right])?;

    Ok(())
}

/// Format one list-like pattern field collection with shared trailing-separator behavior.
fn format_pattern_field_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Pattern>,
    open: &'static str,
    close: &'static str,
    fields: &[LocalNodeId<PatternField>],
    should_expand: bool,
) -> FormatResult<()> {
    if fields.is_empty() {
        return format_empty_pattern_delimiter_with_interior_annotations(f, node_id, open, close);
    }

    let has_inline_comment_seams =
        pattern_fields_have_inline_spread_comment_seams(f.context(), fields);
    if !should_expand && has_inline_comment_seams {
        write!(f, [token(open)])?;
        for (index, field_id) in fields.iter().enumerate() {
            if index > 0 {
                write!(f, [token(","), space()])?;
            }
            write!(f, [*field_id])?;
        }
        write!(f, [token(close)])?;
        return Ok(());
    }

    let mut list = list_like(open, close, ",", fields);
    list.as_collection().should_expand(should_expand);

    if pattern_fields_disallow_trailing_separator(f.context().tree, fields) {
        list.disallow_trailing_separator();
    }

    write!(f, [list])?;

    Ok(())
}

/// Format one empty pattern delimiter pair with interior infix annotations.
fn format_empty_pattern_delimiter_with_interior_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Pattern>,
    open: &'static str,
    close: &'static str,
) -> FormatResult<()> {
    if !f.context().has_delimited_interior_annotation(node_id) {
        write!(f, [token(open), token(close)])?;
        return Ok(());
    }

    write!(
        f,
        [group(&format_args![
            token(open),
            soft_block_indent(&f.context().delimited_interior_annotations(node_id)),
            token(close)
        ])]
    )?;
    Ok(())
}

/// Return whether one pattern field list carries inline comment seams.
fn pattern_fields_have_inline_spread_comment_seams(
    context: &DestackFormatContext<'_>,
    fields: &[LocalNodeId<PatternField>],
) -> bool {
    let mut has_inline_spread_comment = false;

    for field_id in fields {
        let Some(annotation_ids) = context.annotations(*field_id) else {
            continue;
        };
        let field_is_spread = matches!(context.tree.get(*field_id), PatternField::Spread { .. });

        for annotation_id in annotation_ids {
            let is_inline_comment = match context.annotation(annotation_id) {
                Annotation::Comment { .. } | Annotation::Doc { .. } => {
                    let annotation_span = context.annotation_span(annotation_id);
                    !context.span_starts_on_own_line(annotation_span)
                }
                _ => false,
            };
            if !is_inline_comment {
                continue;
            }

            if !field_is_spread {
                return false;
            }
            has_inline_spread_comment = true;
        }
    }

    has_inline_spread_comment
}

/// Return whether an array pattern should expand over multiple lines.
fn array_pattern_should_expand(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Pattern>,
    fields: &[LocalNodeId<PatternField>],
) -> bool {
    if pattern_fields_have_inline_spread_comment_seams(context, fields) {
        return false;
    }

    let has_newline = pattern_is_multiline_span(context, node_id);
    let has_field_comments = pattern_fields_have_comment_annotations(context, fields);
    if has_newline && has_field_comments {
        return true;
    }

    let has_nested_fields = fields
        .iter()
        .copied()
        .any(|field_id| pattern_field_prefers_multiline(context.tree, field_id));
    let has_field_annotations = pattern_fields_have_layout_forcing_annotations(context, fields);

    CollectionBreakScore {
        has_newline_in_source: has_newline,
        has_item_annotations: has_field_annotations,
        has_nested_complexity: has_nested_fields,
    }
    .should_expand_multiline()
}

/// Return whether an object-like pattern should expand over multiple lines.
fn object_pattern_should_expand(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Pattern>,
    fields: &[LocalNodeId<PatternField>],
) -> bool {
    if pattern_fields_have_inline_spread_comment_seams(context, fields) {
        return false;
    }

    let has_newline = pattern_is_multiline_span(context, node_id);
    let has_field_comments = pattern_fields_have_comment_annotations(context, fields);
    if has_newline && has_field_comments {
        return true;
    }

    let has_nested_fields = fields
        .iter()
        .copied()
        .any(|field_id| pattern_field_prefers_multiline(context.tree, field_id));
    let has_default_assignments = pattern_fields_have_default_assignments(context.tree, fields);
    let has_field_annotations = pattern_fields_have_layout_forcing_annotations(context, fields);
    let should_expand_for_parameter =
        should_expand_parameter_object_pattern(context, node_id, fields);
    let should_expand_for_comments = CollectionBreakScore {
        has_newline_in_source: has_newline,
        has_item_annotations: has_field_annotations,
        has_nested_complexity: false,
    }
    .should_expand_multiline();

    (has_newline && has_nested_fields)
        || (has_nested_fields && fields.len() > 1 && !has_default_assignments)
        || should_expand_for_comments
        || should_expand_for_parameter
}

/// Return whether pattern fields carry annotations that should force multiline layout.
fn pattern_fields_have_layout_forcing_annotations(
    context: &DestackFormatContext<'_>,
    fields: &[LocalNodeId<PatternField>],
) -> bool {
    fields.iter().any(|field_id| {
        let Some(annotation_ids) = context.annotations(*field_id) else {
            return false;
        };

        annotation_ids
            .into_iter()
            .any(|annotation_id| match context.annotation(annotation_id) {
                Annotation::Blank { .. }
                | Annotation::Doc { .. }
                | Annotation::Decorator { .. } => true,
                Annotation::Comment { .. } => {
                    let annotation_span = context.annotation_span(annotation_id);
                    context.span_starts_on_own_line(annotation_span)
                }
            })
    })
}

/// Return whether pattern fields carry comment-like annotations.
fn pattern_fields_have_comment_annotations(
    context: &DestackFormatContext<'_>,
    fields: &[LocalNodeId<PatternField>],
) -> bool {
    fields.iter().any(|field_id| {
        context
            .annotations(*field_id)
            .is_some_and(|annotation_ids| {
                annotation_ids.into_iter().any(|annotation_id| {
                    match context.annotation(annotation_id) {
                        Annotation::Comment { .. } | Annotation::Doc { .. } => true,
                        Annotation::Blank { .. } | Annotation::Decorator { .. } => false,
                    }
                })
            })
    })
}

/// Format one object-like pattern, optionally prefixed with a type expression.
fn format_object_pattern_like<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Pattern>,
    ty: Option<LocalNodeId<Expression>>,
    fields: &[LocalNodeId<PatternField>],
) -> FormatResult<()> {
    if let Some(ty) = ty {
        write!(f, [ty, space()])?;
    }

    let render_fields = object_pattern_render_fields(f.context().tree, fields);
    if render_fields.is_empty() {
        return format_empty_pattern_delimiter_with_interior_annotations(f, node_id, "{", "}");
    }

    let should_expand = object_pattern_should_expand(f.context(), node_id, render_fields.as_ref());
    if !should_expand
        && pattern_fields_have_inline_spread_comment_seams(f.context(), render_fields.as_ref())
    {
        let include_bracket_space =
            f.context().options.bracket_spacing && !render_fields.is_empty();
        write!(f, [token("{")])?;
        if include_bracket_space {
            write!(f, [space()])?;
        }
        for (index, field_id) in render_fields.iter().enumerate() {
            if index > 0 {
                write!(f, [token(","), space()])?;
            }
            write!(f, [*field_id])?;
        }
        if include_bracket_space {
            write!(f, [space()])?;
        }
        write!(f, [token("}")])?;
        return Ok(());
    }

    let mut list = list_like("{", "}", ",", render_fields.as_ref());
    list.as_collection()
        .include_space()
        .should_expand(should_expand);

    if pattern_fields_disallow_trailing_separator(f.context().tree, render_fields.as_ref()) {
        list.disallow_trailing_separator();
    }

    write!(f, [list])?;

    Ok(())
}

impl<'ast> FormatNode<'ast, Pattern> for Pattern {
    fn format_node(
        &self,
        node_id: LocalNodeId<Pattern>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        match self {
            Pattern::Wildcard => write!(f, [token("_")])?,
            Pattern::Must(unwrap) => write!(f, [unwrap, token("!")])?,
            Pattern::ReferenceOf { right, mutability } => {
                format_prefixed_pattern(f, "&", *right, *mutability)?;
            }
            Pattern::ValueOf { right, mutability } => {
                format_prefixed_pattern(f, "^", *right, *mutability)?;
            }
            Pattern::Binding {
                mutability,
                name,
                pattern,
            } => {
                if let Some(mutability) = mutability
                    && *mutability == Mutability::Mutable
                {
                    write!(f, [mutability, space()])?;
                }
                write!(f, [name])?;
                if let Some(pattern) = pattern {
                    write!(f, [token(":"), space(), pattern])?;
                }
            }
            Pattern::Expression { value } => write!(f, [value])?,
            Pattern::Tuple { fields } => {
                format_pattern_field_list(f, node_id, "(", ")", fields, false)?;
            }
            Pattern::TaggedTuple { ty, fields } => {
                write!(f, [ty])?;
                format_pattern_field_list(f, node_id, "(", ")", fields, false)?;
            }
            Pattern::Array { fields } => {
                let should_expand = array_pattern_should_expand(f.context(), node_id, fields);
                format_pattern_field_list(f, node_id, "[", "]", fields, should_expand)?;
            }
            Pattern::Object { fields } => {
                format_object_pattern_like(f, node_id, None, fields)?;
            }
            Pattern::TaggedObject { ty, fields } => {
                format_object_pattern_like(f, node_id, Some(*ty), fields)?;
            }
            Pattern::Union { patterns } => write!(
                f,
                [format_with(|f| f
                    .join_with(&format_args![space(), token("|"), space()])
                    .entries(patterns)
                    .finish())]
            )?,
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, PatternField> for PatternField {
    fn format_node(
        &self,
        node_id: LocalNodeId<PatternField>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        match self {
            PatternField::Named {
                mutability,
                name,
                pattern,
                default,
            } => {
                if let Some(mutability) = mutability {
                    write!(f, [mutability, space()])?;
                }
                if let Some(pattern) = pattern {
                    write!(f, [name, token(":"), space(), pattern])?;
                } else {
                    write!(f, [name])?;
                }
                if let Some(default) = default {
                    let should_expand_default =
                        should_expand_pattern_field_default(f, node_id, *default);
                    if should_expand_default {
                        write!(
                            f,
                            [
                                space(),
                                token("="),
                                space(),
                                group(default).should_expand(true)
                            ]
                        )?;
                    } else {
                        write!(f, [space(), token("="), space(), default])?;
                    }
                }
            }
            PatternField::Computed {
                mutability,
                key,
                pattern,
                default,
            } => {
                if let Some(mutability) = mutability {
                    write!(f, [mutability, space()])?;
                }
                write!(f, [token("["), key, token("]")])?;
                if let Some(pattern) = pattern {
                    write!(f, [token(":"), space(), pattern])?;
                }
                if let Some(default) = default {
                    let should_expand_default =
                        should_expand_pattern_field_default(f, node_id, *default);
                    if should_expand_default {
                        write!(
                            f,
                            [
                                space(),
                                token("="),
                                space(),
                                group(default).should_expand(true)
                            ]
                        )?;
                    } else {
                        write!(f, [space(), token("="), space(), default])?;
                    }
                }
            }
            PatternField::Alias {
                mutability,
                name,
                alias,
                default,
            } => {
                if let Some(mutability) = mutability {
                    write!(f, [mutability, space()])?;
                }
                write!(f, [name, token(":"), space(), alias])?;
                if let Some(default) = default {
                    let should_expand_default =
                        should_expand_pattern_field_default(f, node_id, *default);
                    if should_expand_default {
                        write!(
                            f,
                            [
                                space(),
                                token("="),
                                space(),
                                group(default).should_expand(true)
                            ]
                        )?;
                    } else {
                        write!(f, [space(), token("="), space(), default])?;
                    }
                }
            }
            PatternField::Positional { pattern, default } => {
                write!(f, [pattern])?;
                if let Some(default) = default {
                    let should_expand_default =
                        should_expand_pattern_field_default(f, node_id, *default);
                    if should_expand_default {
                        write!(
                            f,
                            [
                                space(),
                                token("="),
                                space(),
                                group(default).should_expand(true)
                            ]
                        )?;
                    } else {
                        write!(f, [space(), token("="), space(), default])?;
                    }
                }
            }
            PatternField::Spread {
                mutability,
                pattern,
            } => {
                if let Some(mutability) = mutability {
                    write!(f, [mutability, space()])?;
                }
                write!(f, [token("...")])?;
                if let Some(pattern) = pattern {
                    write!(f, [pattern])?;
                }
            }
            PatternField::Elision => {
                // elision is represented by empty slot; comma is handled at list level
            }
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{DestackFormatOptions, TestFormatter, assert_format};

    #[test]
    fn test_format_pattern_wildcard() {
        assert_format!("_", "_", |p| p.eat_pattern());
    }

    #[test]
    fn test_format_pattern_reference() {
        assert_format!("&_", "&_", |p| p.eat_pattern());

        assert_format!("&1", "&1", |p| p.eat_pattern());
    }

    #[test]
    fn test_format_pattern_must() {
        assert_format!("T!", "T!", |p| p.eat_pattern());
    }

    #[test]
    fn test_format_pattern_identifier() {
        assert_format!("x", "x", |p| p.eat_pattern());
    }

    #[test]
    fn test_format_pattern_path() {
        assert_format!("MyEnum.A", "MyEnum.A", |p| p.eat_pattern());
    }

    #[test]
    fn test_format_pattern_tuple() {
        assert_format!("(x: 1, 2, ...)", "(x: 1, 2, ...)", |p| p.eat_pattern());
    }

    #[test]
    fn test_format_pattern_tuple_with_path() {
        assert_format!("Result.Success(_, ...)", "Result.Success(_, ...)", |p| p
            .eat_pattern());
    }

    #[test]
    fn test_format_pattern_slice() {
        assert_format!("[1, 2, ...]", "[1, 2, ...]", |p| p.eat_pattern());
    }

    #[test]
    fn test_format_pattern_union() {
        assert_format!("1 | 2 | 3 | 4 | 5", "1 | 2 | 3 | 4 | 5", |p| p
            .eat_pattern());
    }

    #[test]
    fn test_format_pattern_array_rest_disallows_trailing_comma() {
        assert_format!(
            "[a, ...rest]",
            "[\n    a,\n    ...rest\n]",
            |p| p.eat_pattern(),
            DestackFormatOptions::default_with_line_width(1)
        );
    }

    #[test]
    fn test_format_pattern_tuple_rest_disallows_trailing_comma() {
        assert_format!(
            "(a, ...rest)",
            "(\n    a,\n    ...rest\n)",
            |p| p.eat_pattern(),
            DestackFormatOptions::default_with_line_width(1)
        );
    }
}
