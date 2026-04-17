use std::borrow::Cow;

use destack_fir::format::FormatResult;

use crate::format::annotation::{
    block_infix_annotations, infix_or_postfix_annotations, prefix_annotations,
};
use crate::format::collection::{TrailingSeparator, separated_entries};
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    DecoratorPosition, LocalNodeId, Mutability, NodeTree, NodeType, Pattern, PatternField,
    TypeExpression,
};
use destack_fir::prelude::*;
use destack_fir::{format_args, write};

impl<'ast> Format<DestackFormatContext<'ast>> for Mutability {
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Mutability::Immutable => write!(f, [token("const")]),
            Mutability::Mutable => write!(f, [token("var")]),
        }
    }
}

/// Return whether one pattern is object-like or array-like.
fn pattern_is_object_or_array_like(tree: &NodeTree, pattern_id: LocalNodeId<Pattern>) -> bool {
    match tree.get(pattern_id) {
        Pattern::Binding {
            pattern: Some(pattern),
            ..
        } => pattern_is_object_or_array_like(tree, *pattern),
        Pattern::Object { .. } | Pattern::TaggedObject { .. } | Pattern::Array { .. } => true,
        _ => false,
    }
}

/// Return whether one field contains a nested object-like or array-like pattern.
fn pattern_field_has_nested_object_or_array_like_pattern(
    tree: &NodeTree,
    field_id: LocalNodeId<PatternField>,
) -> bool {
    match tree.get(field_id) {
        PatternField::Named {
            pattern: Some(pattern),
            ..
        }
        | PatternField::Computed {
            pattern: Some(pattern),
            ..
        }
        | PatternField::Spread {
            pattern: Some(pattern),
            ..
        } => pattern_is_object_or_array_like(tree, *pattern),
        PatternField::Positional { pattern, .. } => pattern_is_object_or_array_like(tree, *pattern),
        PatternField::Named { pattern: None, .. }
        | PatternField::Computed { pattern: None, .. }
        | PatternField::Alias { .. }
        | PatternField::Spread { pattern: None, .. }
        | PatternField::Elision => false,
    }
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
    fields.last().is_some_and(|field_id| {
        matches!(
            tree.get(*field_id),
            PatternField::Spread { .. } | PatternField::Elision
        )
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

    let allow_trailing_separator =
        !pattern_fields_disallow_trailing_separator(f.context().tree, fields);
    let trailing_separator = if !allow_trailing_separator
        || f.context().options.trailing_comma == destack_workspace::TrailingComma::None
    {
        TrailingSeparator::Omit
    } else {
        TrailingSeparator::Allowed
    };

    write!(
        f,
        [group(&format_args![
            token(open),
            soft_block_indent(&separated_entries(",", fields, trailing_separator, None)),
            token(close)
        ])
        .should_expand(should_expand)]
    )?;

    Ok(())
}

/// Format one empty pattern delimiter pair with interior infix annotations.
fn format_empty_pattern_delimiter_with_interior_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Pattern>,
    open: &'static str,
    close: &'static str,
) -> FormatResult<()> {
    let mut interior_items = Vec::new();
    for annotation_id in f.context().annotation_ids(node_id).iter().copied() {
        if f.context().annotation(annotation_id).position == DecoratorPosition::BlockInfix {
            interior_items.push(annotation_id);
        }
    }
    if interior_items.is_empty() {
        write!(f, [token(open), token(close)])?;
        return Ok(());
    }

    write!(
        f,
        [group(&format_args![
            token(open),
            soft_block_indent(&block_infix_annotations(f.context(), node_id)),
            token(close)
        ])]
    )?;
    Ok(())
}

/// Return whether one object-like pattern is inline in its parent.
fn object_pattern_is_inline(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Pattern>,
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
        destack_ast::Parameter::Named { .. }
        | destack_ast::Parameter::VariadicNamed { .. }
        | destack_ast::Parameter::Error => None,
    };

    parameter_pattern.is_some_and(|pattern_id| pattern_id.id == node_id.id)
}

/// Return whether one object-like pattern is in assignment form.
fn object_pattern_is_in_assignment_like(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Pattern>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Declarator {
        return false;
    }

    let declarator = context
        .tree
        .get(LocalNodeId::<destack_ast::Declarator>::new(parent_id));
    declarator.pattern.id == node_id.id
}

/// Return whether one object-like pattern should break its properties.
fn object_pattern_should_break_properties(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Pattern>,
    fields: &[LocalNodeId<PatternField>],
) -> bool {
    if object_pattern_is_inline(context, node_id) {
        return false;
    }

    fields.iter().copied().any(|field_id| {
        pattern_field_has_nested_object_or_array_like_pattern(context.tree, field_id)
    }) || object_pattern_has_boundary_comments(context, fields)
}

/// Return whether field boundaries own raw comments.
fn object_pattern_has_boundary_comments(
    context: &DestackFormatContext<'_>,
    fields: &[LocalNodeId<PatternField>],
) -> bool {
    fields.windows(2).any(|pair| {
        let [left_id, right_id] = pair else {
            return false;
        };

        let left_span = context.span(*left_id);
        let right_span = context.span(*right_id);

        !context
            .comments()
            .comments_in_range(left_span.end, right_span.start)
            .is_empty()
    })
}

#[derive(Clone, Copy, Debug)]
enum ObjectPatternLayout {
    Empty,
    Inline,
    Group { expand: bool },
}

/// Return the layout for one object-like pattern.
fn object_pattern_layout(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Pattern>,
    fields: &[LocalNodeId<PatternField>],
) -> ObjectPatternLayout {
    if fields.is_empty() {
        return ObjectPatternLayout::Empty;
    }

    if object_pattern_is_inline(context, node_id) {
        return ObjectPatternLayout::Inline;
    }

    if object_pattern_should_break_properties(context, node_id, fields) {
        ObjectPatternLayout::Group { expand: true }
    } else if object_pattern_is_in_assignment_like(context, node_id) {
        ObjectPatternLayout::Inline
    } else {
        ObjectPatternLayout::Group { expand: false }
    }
}

/// Format one object-like pattern, optionally prefixed with a type expression.
fn format_object_pattern_like<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Pattern>,
    ty: Option<LocalNodeId<TypeExpression>>,
    fields: &[LocalNodeId<PatternField>],
) -> FormatResult<()> {
    if let Some(ty) = ty {
        write!(f, [ty, space()])?;
    }

    let render_fields = object_pattern_render_fields(f.context().tree, fields);
    let layout = object_pattern_layout(f.context(), node_id, render_fields.as_ref());
    if matches!(layout, ObjectPatternLayout::Empty) {
        return format_empty_pattern_delimiter_with_interior_annotations(f, node_id, "{", "}");
    }

    let allow_trailing_separator =
        !pattern_fields_disallow_trailing_separator(f.context().tree, render_fields.as_ref());
    let trailing_separator = if !allow_trailing_separator
        || f.context().options.trailing_comma == destack_workspace::TrailingComma::None
    {
        TrailingSeparator::Omit
    } else {
        TrailingSeparator::Allowed
    };

    let format_fields = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(
            f,
            [separated_entries(
                ",",
                render_fields.as_ref(),
                trailing_separator,
                None,
            )]
        )?;
        Ok(())
    });

    let format_properties = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if f.context().options.bracket_spacing {
            write!(f, [soft_space_or_block_indent(&format_fields)])?;
        } else {
            write!(f, [soft_block_indent(&format_fields)])?;
        }
        Ok(())
    });

    write!(f, [token("{")])?;
    match layout {
        ObjectPatternLayout::Empty => unreachable!(),
        ObjectPatternLayout::Inline => write!(f, [format_properties])?,
        ObjectPatternLayout::Group { expand } => {
            write!(f, [group(&format_properties).should_expand(expand)])?;
        }
    }
    write!(f, [token("}")])?;

    Ok(())
}

impl<'ast> FormatNode<'ast, Pattern> for Pattern {
    fn format_node(
        &self,
        node_id: LocalNodeId<Pattern>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [prefix_annotations(f.context(), node_id)])?;

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
            Pattern::TypeExpression { value } => write!(f, [value])?,
            Pattern::Tuple { fields } => {
                format_pattern_field_list(f, node_id, "(", ")", fields, false)?;
            }
            Pattern::TaggedTuple { ty, fields } => {
                write!(f, [ty])?;
                format_pattern_field_list(f, node_id, "(", ")", fields, false)?;
            }
            Pattern::Array { fields } => {
                format_pattern_field_list(f, node_id, "[", "]", fields, false)?;
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

        write!(f, [infix_or_postfix_annotations(f.context(), node_id)])?;

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, PatternField> for PatternField {
    fn format_node(
        &self,
        node_id: LocalNodeId<PatternField>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [prefix_annotations(f.context(), node_id)])?;

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
                    write!(f, [space(), token("="), space(), default])?;
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
                    write!(f, [space(), token("="), space(), default])?;
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
                    write!(f, [space(), token("="), space(), default])?;
                }
            }
            PatternField::Positional { pattern, default } => {
                write!(f, [pattern])?;
                if let Some(default) = default {
                    write!(f, [space(), token("="), space(), default])?;
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

        write!(f, [infix_or_postfix_annotations(f.context(), node_id)])?;

        Ok(())
    }
}
