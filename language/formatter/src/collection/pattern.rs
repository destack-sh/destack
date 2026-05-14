use std::borrow::Cow;

use destack_fir::format::FormatResult;

use crate::annotation::{
    FormatLeadingComments, block_infix_annotations, format_dangling_comments,
    infix_or_postfix_annotations, prefix_annotations,
};
use crate::collection::{TrailingSeparator, separated_entries};
use crate::context::MemoizeFormatExt;
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_dir::{
    AssignPattern, AssignPatternField, Declarator, DecoratorPosition, Expression, LocalNodeId,
    Mutability, Node, NodeType, Parameter, Pattern, PatternField, RangeEnd, Tree, TreeStore,
    TypeExpression,
};
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_workspace::TrailingComma;

impl<'ast> Format<DestackFormatContext<'ast>> for Mutability {
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Mutability::Immutable => write!(f, [token("readonly")]),
            Mutability::Exclusive => write!(f, [token("exclusive")]),
            Mutability::Mutable => Ok(()),
        }
    }
}

/// Return object-pattern fields to render, normalizing out parser elision artifacts.
fn object_pattern_render_fields<'a>(
    tree: &Tree,
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
    tree: &Tree,
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

    if let Some(mutability) = mutability {
        match mutability {
            Mutability::Immutable => write!(f, [token("readonly"), space()])?,
            Mutability::Exclusive => write!(f, [token("exclusive"), space()])?,
            Mutability::Mutable => {}
        }
    }

    write!(f, [right])?;

    Ok(())
}

/// Format one ordered range pattern.
fn format_range_pattern<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    start: Option<LocalNodeId<Expression>>,
    end: Option<LocalNodeId<Expression>>,
    end_kind: RangeEnd,
) -> FormatResult<()> {
    if let Some(start) = start {
        write!(f, [start])?;
    }

    let operator = match end_kind {
        RangeEnd::Open => "..",
        RangeEnd::Inclusive => "..=",
    };
    write!(f, [token(operator)])?;

    if let Some(end) = end {
        write!(f, [end])?;
    }

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
    let trailing_separator =
        if !allow_trailing_separator || f.context().options.trailing_comma == TrailingComma::None {
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

/// Return whether trailing separators are invalid for the current assign-pattern field list.
fn assign_pattern_fields_disallow_trailing_separator(
    tree: &Tree,
    fields: &[LocalNodeId<AssignPatternField>],
) -> bool {
    fields.last().is_some_and(|field_id| {
        matches!(
            tree.get(*field_id),
            AssignPatternField::Spread { .. } | AssignPatternField::Elision
        )
    })
}

/// Format one list-like assign-pattern field collection with shared trailing-separator behavior.
fn format_assign_pattern_field_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<AssignPattern>,
    open: &'static str,
    close: &'static str,
    fields: &[LocalNodeId<AssignPatternField>],
    should_expand: bool,
) -> FormatResult<()> {
    if fields.is_empty() {
        return format_empty_pattern_delimiter_with_interior_annotations(f, node_id, open, close);
    }

    let allow_trailing_separator =
        !assign_pattern_fields_disallow_trailing_separator(f.context().tree, fields);
    let trailing_separator =
        if !allow_trailing_separator || f.context().options.trailing_comma == TrailingComma::None {
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
fn format_empty_pattern_delimiter_with_interior_annotations<'ast, T>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<T>,
    open: &'static str,
    close: &'static str,
) -> FormatResult<()>
where
    T: Node + Clone + 'ast,
    Tree: TreeStore<T>,
{
    let span = f.context().span(node_id);
    let mut interior_items = Vec::new();
    for annotation_id in f.context().annotation_ids(node_id).iter().copied() {
        if f.context().annotation(annotation_id).position == DecoratorPosition::BlockInfix {
            interior_items.push(annotation_id);
        }
    }

    let has_dangling_comments = {
        let comments = f.context().comments();
        !comments.comments_before(span.end).is_empty()
    };

    if interior_items.is_empty() && !has_dangling_comments {
        write!(f, [token(open), token(close)])?;
        return Ok(());
    }

    if interior_items.is_empty() {
        write!(
            f,
            [group(&format_args![
                token(open),
                format_dangling_comments(span).with_block_indent(),
                token(close)
            ])]
        )?;
        return Ok(());
    }

    write!(
        f,
        [group(&format_args![
            token(open),
            block_indent(&format_with(|f| {
                write!(f, [block_infix_annotations(f.context(), node_id)])?;
                write!(f, [format_dangling_comments(span)])
            })),
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

    let parameter = context.tree.get(LocalNodeId::<Parameter>::new(parent_id));
    let parameter_pattern = match parameter {
        Parameter::Pattern { pattern, .. } | Parameter::VariadicPattern { pattern, .. } => {
            Some(*pattern)
        }
        Parameter::Named { .. } | Parameter::VariadicNamed { .. } | Parameter::Error => None,
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

    let declarator = context.tree.get(LocalNodeId::<Declarator>::new(parent_id));
    declarator.pattern.id == node_id.id
}

/// Return whether one object-like pattern should break its properties.
fn object_pattern_should_break_properties(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Pattern>,
    fields: &[LocalNodeId<PatternField>],
) -> bool {
    // assignment wrappers keep nested object patterns flat
    if object_pattern_has_assignment_wrapper_parent(context, node_id) {
        return false;
    }

    // direct nested destructuring
    let has_direct_nested_pattern = fields
        .iter()
        .copied()
        .any(|field_id| object_pattern_field_has_direct_nested_pattern(context.tree, field_id));

    // separator comments
    let has_separator_comments = object_pattern_has_separator_comments(context, fields);

    has_direct_nested_pattern || has_separator_comments
}

/// Return whether one object-like pattern is wrapped by a defaulting pattern.
fn object_pattern_has_assignment_wrapper_parent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Pattern>,
) -> bool {
    // parent kind
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };

    if parent_type != NodeType::Pattern {
        return false;
    }

    // assignment wrapper
    matches!(
        context.tree.get(LocalNodeId::<Pattern>::new(parent_id)),
        Pattern::Assign { .. }
    )
}

/// Return whether one pattern field contains a direct nested object or sequence pattern.
fn object_pattern_field_has_direct_nested_pattern(
    tree: &Tree,
    field_id: LocalNodeId<PatternField>,
) -> bool {
    match tree.get(field_id) {
        // nested value
        PatternField::Named {
            pattern: Some(pattern_id),
            ..
        }
        | PatternField::Computed {
            pattern: pattern_id,
            ..
        }
        | PatternField::Positional {
            pattern: pattern_id,
        } => pattern_is_direct_object_or_array_like(tree, *pattern_id),

        // flat field
        PatternField::Named { pattern: None, .. }
        | PatternField::Spread { .. }
        | PatternField::Elision => false,
    }
}

/// Return whether one pattern is directly object-like or array-like.
fn pattern_is_direct_object_or_array_like(tree: &Tree, pattern_id: LocalNodeId<Pattern>) -> bool {
    match tree.get(pattern_id) {
        // direct nested destructuring
        Pattern::Object { .. }
        | Pattern::TaggedObject { .. }
        | Pattern::Sequence { .. }
        | Pattern::TaggedTuple { .. }
        | Pattern::Tuple { .. } => true,

        // assignment wrappers stay owned by assignment-like layout
        Pattern::Assign { .. } => false,

        // transparent wrappers
        Pattern::Must(pattern)
        | Pattern::BorrowOf { right: pattern, .. }
        | Pattern::MoveOf { right: pattern, .. } => {
            pattern_is_direct_object_or_array_like(tree, *pattern)
        }

        // non-destructuring patterns
        Pattern::Binding { pattern: None, .. }
        | Pattern::Binding {
            pattern: Some(_), ..
        }
        | Pattern::Wildcard
        | Pattern::Expression { .. }
        | Pattern::Range { .. }
        | Pattern::TypeExpression { .. }
        | Pattern::Union { .. } => false,
    }
}

/// Return whether field separators have comments.
fn object_pattern_has_separator_comments(
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
    // empty pattern
    if fields.is_empty() {
        return ObjectPatternLayout::Empty;
    }

    // inline parameter pattern
    if object_pattern_is_inline(context, node_id) {
        return ObjectPatternLayout::Inline;
    }

    // expanded nested destructuring
    if object_pattern_should_break_properties(context, node_id, fields) {
        return ObjectPatternLayout::Group { expand: true };
    }

    // assignment-like layout
    if object_pattern_is_in_assignment_like(context, node_id) {
        return ObjectPatternLayout::Inline;
    }

    ObjectPatternLayout::Group { expand: false }
}

/// Format one object-like pattern, optionally prefixed with a type expression.
fn format_object_pattern_like<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Pattern>,
    ty: Option<LocalNodeId<TypeExpression>>,
    fields: &[LocalNodeId<PatternField>],
) -> FormatResult<()> {
    // tagged prefix
    if let Some(ty) = ty {
        write!(f, [ty, space()])?;
    }

    // layout
    let render_fields = object_pattern_render_fields(f.context().tree, fields);
    let layout = object_pattern_layout(f.context(), node_id, render_fields.as_ref());

    if matches!(layout, ObjectPatternLayout::Empty) {
        return format_empty_pattern_delimiter_with_interior_annotations(f, node_id, "{", "}");
    }

    // separator policy
    let allow_trailing_separator =
        !pattern_fields_disallow_trailing_separator(f.context().tree, render_fields.as_ref());
    let trailing_separator =
        if !allow_trailing_separator || f.context().options.trailing_comma == TrailingComma::None {
            TrailingSeparator::Omit
        } else {
            TrailingSeparator::Allowed
        };

    // field writers
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

    // bracket spacing
    let format_properties = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if f.context().options.bracket_spacing {
            write!(f, [soft_space_or_block_indent(&format_fields)])?;
        } else {
            write!(f, [soft_block_indent(&format_fields)])?;
        }
        Ok(())
    });

    // layout
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

/// Return whether one assign-pattern is wrapped by a defaulting target.
fn object_assign_pattern_has_assignment_wrapper_parent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<AssignPattern>,
) -> bool {
    // parent kind
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };

    if parent_type != NodeType::AssignPattern {
        return false;
    }

    // assignment wrapper
    matches!(
        context
            .tree
            .get(LocalNodeId::<AssignPattern>::new(parent_id)),
        AssignPattern::Assign { .. }
    )
}

/// Return whether one object-like assign-pattern is the direct target of an assignment.
fn object_assign_pattern_is_assignment_target(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<AssignPattern>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let expression = context.tree.get(LocalNodeId::<Expression>::new(parent_id));
    matches!(expression, Expression::Assign { left, .. } if *left == node_id)
}

/// Return whether one assign-pattern field contains a direct nested object or sequence pattern.
fn object_assign_pattern_field_has_direct_nested_pattern(
    tree: &Tree,
    field_id: LocalNodeId<AssignPatternField>,
) -> bool {
    match tree.get(field_id) {
        // nested value
        AssignPatternField::Named {
            pattern: Some(pattern_id),
            ..
        }
        | AssignPatternField::Computed {
            pattern: pattern_id,
            ..
        }
        | AssignPatternField::Positional {
            pattern: pattern_id,
        } => assign_pattern_is_direct_object_or_array_like(tree, *pattern_id),

        // flat field
        AssignPatternField::Named { pattern: None, .. }
        | AssignPatternField::Spread { .. }
        | AssignPatternField::Elision => false,
    }
}

/// Return whether one assign-pattern is directly object-like or array-like.
fn assign_pattern_is_direct_object_or_array_like(
    tree: &Tree,
    pattern_id: LocalNodeId<AssignPattern>,
) -> bool {
    match tree.get(pattern_id) {
        // direct nested destructuring
        AssignPattern::Object { .. } | AssignPattern::Sequence { .. } => true,

        // assignment wrappers stay owned by assignment-like layout
        AssignPattern::Assign { .. } => false,

        // simple target
        AssignPattern::Expression { .. } => false,
    }
}

/// Return whether assign-pattern field separators have comments.
fn object_assign_pattern_has_separator_comments(
    context: &DestackFormatContext<'_>,
    fields: &[LocalNodeId<AssignPatternField>],
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

/// Return whether one object-like assign-pattern should break its properties.
fn object_assign_pattern_should_break_properties(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<AssignPattern>,
    fields: &[LocalNodeId<AssignPatternField>],
) -> bool {
    // assignment wrappers keep nested object patterns flat
    if object_assign_pattern_has_assignment_wrapper_parent(context, node_id) {
        return false;
    }

    // direct nested destructuring
    let has_direct_nested_pattern = fields.iter().copied().any(|field_id| {
        object_assign_pattern_field_has_direct_nested_pattern(context.tree, field_id)
    });

    // separator comments
    let has_separator_comments = object_assign_pattern_has_separator_comments(context, fields);

    has_direct_nested_pattern || has_separator_comments
}

#[derive(Clone, Copy, Debug)]
enum ObjectAssignPatternLayout {
    Empty,
    Inline,
    Group { expand: bool },
}

/// Return the layout for one object-like assign-pattern.
fn object_assign_pattern_layout(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<AssignPattern>,
    fields: &[LocalNodeId<AssignPatternField>],
) -> ObjectAssignPatternLayout {
    // empty pattern
    if fields.is_empty() {
        return ObjectAssignPatternLayout::Empty;
    }

    // expanded nested destructuring
    if object_assign_pattern_should_break_properties(context, node_id, fields) {
        return ObjectAssignPatternLayout::Group { expand: true };
    }

    // assignment-like layout
    if object_assign_pattern_is_assignment_target(context, node_id) {
        return ObjectAssignPatternLayout::Inline;
    }

    ObjectAssignPatternLayout::Group { expand: false }
}

/// Format one object-like assign-pattern.
fn format_object_assign_pattern_like<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<AssignPattern>,
    fields: &[LocalNodeId<AssignPatternField>],
) -> FormatResult<()> {
    // layout
    let layout = object_assign_pattern_layout(f.context(), node_id, fields);

    if matches!(layout, ObjectAssignPatternLayout::Empty) {
        return format_empty_pattern_delimiter_with_interior_annotations(f, node_id, "{", "}");
    }

    // separator policy
    let allow_trailing_separator =
        !assign_pattern_fields_disallow_trailing_separator(f.context().tree, fields);
    let trailing_separator =
        if !allow_trailing_separator || f.context().options.trailing_comma == TrailingComma::None {
            TrailingSeparator::Omit
        } else {
            TrailingSeparator::Allowed
        };

    // field writers
    let format_fields = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(
            f,
            [separated_entries(",", fields, trailing_separator, None)]
        )?;
        Ok(())
    });

    // bracket spacing
    let format_properties = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if f.context().options.bracket_spacing {
            write!(f, [soft_space_or_block_indent(&format_fields)])?;
        } else {
            write!(f, [soft_block_indent(&format_fields)])?;
        }
        Ok(())
    });

    // layout
    write!(f, [token("{")])?;

    match layout {
        ObjectAssignPatternLayout::Empty => unreachable!(),
        ObjectAssignPatternLayout::Inline => write!(f, [format_properties])?,
        ObjectAssignPatternLayout::Group { expand } => {
            write!(f, [group(&format_properties).should_expand(expand)])?;
        }
    }
    write!(f, [token("}")])?;

    Ok(())
}

/// Format one binding assignment pattern in assignment-pattern order.
fn format_pattern_assignment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    pattern: LocalNodeId<Pattern>,
    value: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let left = pattern.memoized();
    left.inspect(f)?;

    let value_start = f.context().span(value).start;
    let comments = f
        .context()
        .comments()
        .own_line_comments_before(value_start)
        .to_vec();

    write!(
        f,
        [
            FormatLeadingComments::Comments(&comments),
            group(&left),
            space(),
            token("="),
            space(),
            value
        ]
    )
}

/// Format one assignment target default in assignment pattern order.
fn format_assign_pattern_assignment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    pattern: LocalNodeId<AssignPattern>,
    value: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let left = pattern.memoized();
    left.inspect(f)?;

    let value_start = f.context().span(value).start;
    let comments = f
        .context()
        .comments()
        .own_line_comments_before(value_start)
        .to_vec();

    write!(
        f,
        [
            FormatLeadingComments::Comments(&comments),
            group(&left),
            space(),
            token("="),
            space(),
            value
        ]
    )
}

impl<'ast> FormatNode<'ast, Pattern> for Pattern {
    fn format_node(
        &self,
        node_id: LocalNodeId<Pattern>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        if let Pattern::Assign { pattern, value } = self {
            format_pattern_assignment(f, *pattern, *value)?;
            write!(f, [infix_or_postfix_annotations(f.context(), node_id)])?;
            return Ok(());
        }

        write!(f, [prefix_annotations(f.context(), node_id)])?;

        match self {
            Pattern::Wildcard => write!(f, [token("_")])?,
            Pattern::Must(unwrap) => write!(f, [unwrap, token("!")])?,

            Pattern::Assign { .. } => unreachable!("assignment pattern is formatted above"),

            Pattern::BorrowOf { right, mutability } => {
                format_prefixed_pattern(f, "&", *right, *mutability)?;
            }

            Pattern::MoveOf { right, mutability } => {
                format_prefixed_pattern(f, "^", *right, *mutability)?;
            }

            Pattern::Binding { name, pattern } => {
                // binding name
                write!(f, [name])?;

                // nested pattern
                if let Some(pattern) = pattern {
                    write!(f, [token(":"), space(), pattern])?;
                }
            }

            Pattern::Expression { value } => write!(f, [value])?,
            Pattern::Range {
                start,
                end,
                end_kind,
            } => {
                format_range_pattern(f, *start, *end, *end_kind)?;
            }
            Pattern::TypeExpression { value } => write!(f, [value])?,

            Pattern::Tuple { fields } => {
                format_pattern_field_list(f, node_id, "(", ")", fields, false)?;
            }

            Pattern::TaggedTuple { ty, fields } => {
                write!(f, [ty])?;
                format_pattern_field_list(f, node_id, "(", ")", fields, false)?;
            }

            Pattern::Sequence { fields } => {
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
                name,
                is_shorthand,
                pattern,
            } => {
                // expanded field
                if !is_shorthand {
                    let pattern = pattern.expect("expanded named pattern field");
                    write!(f, [name, token(":"), space(), pattern])?;
                }
                // shorthand assignment field
                else if let Some(pattern) = pattern {
                    write!(f, [name])?;
                    write_shorthand_assignment_value(f, *pattern)?;
                }
                // plain shorthand field
                else {
                    write!(f, [name])?;
                }
            }

            PatternField::Computed { key, pattern } => {
                // computed key and value
                write!(f, [token("["), key, token("]")])?;
                write!(f, [token(":"), space(), pattern])?;
            }

            PatternField::Positional { pattern } => {
                write!(f, [pattern])?;
            }

            PatternField::Spread { pattern } => {
                // spread value
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

impl<'ast> FormatNode<'ast, AssignPattern> for AssignPattern {
    fn format_node(
        &self,
        node_id: LocalNodeId<AssignPattern>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        if let AssignPattern::Assign { pattern, value } = self {
            format_assign_pattern_assignment(f, *pattern, *value)?;
            write!(f, [infix_or_postfix_annotations(f.context(), node_id)])?;
            return Ok(());
        }

        write!(f, [prefix_annotations(f.context(), node_id)])?;

        match self {
            AssignPattern::Expression { value } => {
                write!(f, [value])?;
            }

            AssignPattern::Assign { .. } => unreachable!("assignment pattern is formatted above"),

            AssignPattern::Sequence { fields } => {
                format_assign_pattern_field_list(f, node_id, "[", "]", fields, false)?;
            }

            AssignPattern::Object { fields } => {
                format_object_assign_pattern_like(f, node_id, fields)?;
            }
        }

        write!(f, [infix_or_postfix_annotations(f.context(), node_id)])?;

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, AssignPatternField> for AssignPatternField {
    fn format_node(
        &self,
        node_id: LocalNodeId<AssignPatternField>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [prefix_annotations(f.context(), node_id)])?;

        match self {
            AssignPatternField::Named {
                name,
                is_shorthand,
                pattern,
            } => {
                // expanded field
                if !is_shorthand {
                    let pattern = pattern.expect("expanded named assign pattern field");
                    write!(f, [name, token(":"), space(), pattern])?;
                }
                // shorthand assignment field
                else if let Some(pattern) = pattern {
                    write!(f, [name])?;
                    write_shorthand_assign_pattern_value(f, *pattern)?;
                }
                // plain shorthand field
                else {
                    write!(f, [name])?;
                }
            }

            AssignPatternField::Computed { key, pattern } => {
                // computed key and value
                write!(f, [token("["), key, token("]")])?;
                write!(f, [token(":"), space(), pattern])?;
            }

            AssignPatternField::Positional { pattern } => {
                write!(f, [pattern])?;
            }

            AssignPatternField::Spread { pattern } => {
                // spread value
                write!(f, [token("...")])?;

                if let Some(pattern) = pattern {
                    write!(f, [pattern])?;
                }
            }

            AssignPatternField::Elision => {
                // elision is represented by empty slot; comma is handled at list level
            }
        }

        write!(f, [infix_or_postfix_annotations(f.context(), node_id)])?;

        Ok(())
    }
}

/// Write the value side of one shorthand assignment pattern.
fn write_shorthand_assignment_value(
    f: &mut DestackFormatter<'_, '_>,
    pattern_id: LocalNodeId<Pattern>,
) -> FormatResult<()> {
    // shorthand defaults always lower to assignment wrappers
    let pattern = f.context().tree.get(pattern_id);

    let Pattern::Assign { value, .. } = pattern else {
        unreachable!("expected shorthand assignment pattern");
    };

    write!(f, [space(), token("="), space(), value])
}

/// Write the value side of one shorthand assignment target.
fn write_shorthand_assign_pattern_value(
    f: &mut DestackFormatter<'_, '_>,
    pattern_id: LocalNodeId<AssignPattern>,
) -> FormatResult<()> {
    // shorthand defaults always lower to assignment wrappers
    let pattern = f.context().tree.get(pattern_id);

    let AssignPattern::Assign { value, .. } = pattern else {
        unreachable!("expected shorthand assignment target");
    };

    write!(f, [space(), token("="), space(), value])
}
