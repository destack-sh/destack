use std::borrow::Cow;

use destack_fir::format::{FormatError, FormatResult};

use crate::annotation::{
    FormatLeadingComments, block_infix_annotations, format_dangling_comments,
    infix_or_postfix_annotations, prefix_annotations,
};
use crate::collection::{TrailingSeparator, separated_entries};
use crate::context::CapturedFormat;
use crate::operator::write_range_operator;
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_dir::{
    AssignPattern, AssignPatternField, Declarator, DecoratorPosition, Exclusivity, Expression,
    LocalNodeId, Mutability, Node, NodeType, Parameter, Pattern, PatternField, RangeEnd, Tree,
    TreeStore, TypeExpression,
};
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_repository::TrailingComma;

impl<'ast> Format<'ast, DestackFormatContext<'ast>> for Mutability {
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Mutability::Immutable => write!(f, [token("readonly")]),
            Mutability::Mutable => Ok(()),
        }
    }
}

/// One object destructuring pattern.
enum ObjectPattern<'a> {
    /// One binding pattern with optional nominal type.
    Binding {
        /// The pattern node.
        node_id: LocalNodeId<Pattern>,
        /// The nominal type prefix.
        ty: Option<LocalNodeId<TypeExpression>>,
        /// The fields rendered inside the object.
        fields: Cow<'a, [LocalNodeId<PatternField>]>,
    },
    /// One assignment target pattern.
    Assignment {
        /// The pattern node.
        node_id: LocalNodeId<AssignPattern>,
        /// The fields rendered inside the object.
        fields: &'a [LocalNodeId<AssignPatternField>],
    },
}

/// One object pattern layout.
#[derive(Clone, Copy, Debug)]
enum ObjectPatternLayout {
    /// Keep the fields on the current line.
    Inline,
    /// Group the fields and optionally force expansion.
    Group {
        /// Whether the group must expand.
        expand: bool,
    },
}

impl<'a> ObjectPattern<'a> {
    /// Create one binding object pattern and discard parser elision nodes.
    fn binding(
        tree: &Tree,
        node_id: LocalNodeId<Pattern>,
        ty: Option<LocalNodeId<TypeExpression>>,
        fields: &'a [LocalNodeId<PatternField>],
    ) -> Self {
        let has_elision = fields
            .iter()
            .copied()
            .any(|field_id| matches!(tree.get(field_id), PatternField::Elision));
        let fields = if has_elision {
            Cow::Owned(
                fields
                    .iter()
                    .copied()
                    .filter(|field_id| !matches!(tree.get(*field_id), PatternField::Elision))
                    .collect(),
            )
        } else {
            Cow::Borrowed(fields)
        };

        Self::Binding {
            node_id,
            ty,
            fields,
        }
    }

    /// Create one assignment object pattern.
    fn assignment(
        node_id: LocalNodeId<AssignPattern>,
        fields: &'a [LocalNodeId<AssignPatternField>],
    ) -> Self {
        Self::Assignment { node_id, fields }
    }

    /// Return whether the pattern has no rendered fields.
    fn is_empty(&self) -> bool {
        match self {
            Self::Binding { fields, .. } => fields.is_empty(),
            Self::Assignment { fields, .. } => fields.is_empty(),
        }
    }

    /// Return whether the last field forbids a trailing separator.
    fn forbids_trailing_separator(&self, tree: &Tree) -> bool {
        match self {
            Self::Binding { fields, .. } => {
                pattern_fields_disallow_trailing_separator(tree, fields)
            }
            Self::Assignment { fields, .. } => {
                assign_pattern_fields_disallow_trailing_separator(tree, fields)
            }
        }
    }

    /// Return whether one field directly contains another destructuring pattern.
    fn has_nested_pattern(&self, tree: &Tree) -> bool {
        match self {
            Self::Binding { fields, .. } => fields.iter().copied().any(|field_id| {
                let pattern = match tree.get(field_id) {
                    PatternField::Named { pattern, .. } => *pattern,
                    PatternField::Computed { pattern, .. }
                    | PatternField::Positional { pattern } => Some(*pattern),
                    PatternField::Rest { .. } | PatternField::Elision => None,
                };

                pattern.is_some_and(|pattern| pattern_is_destructuring(tree, pattern))
            }),
            Self::Assignment { fields, .. } => fields.iter().copied().any(|field_id| {
                let pattern = match tree.get(field_id) {
                    AssignPatternField::Named { pattern, .. }
                    | AssignPatternField::Computed { pattern, .. }
                    | AssignPatternField::Positional { pattern } => Some(*pattern),
                    AssignPatternField::Rest { .. } | AssignPatternField::Elision => None,
                };

                pattern.is_some_and(|pattern| assign_pattern_is_destructuring(tree, pattern))
            }),
        }
    }

    /// Return whether adjacent fields have source comments between them.
    fn has_separator_comments(&self, context: &DestackFormatContext<'_>) -> bool {
        match self {
            Self::Binding { fields, .. } => fields
                .windows(2)
                .any(|fields| object_field_gap_has_comments(context, fields[0], fields[1])),
            Self::Assignment { fields, .. } => fields
                .windows(2)
                .any(|fields| object_field_gap_has_comments(context, fields[0], fields[1])),
        }
    }

    /// Return whether a defaulting pattern owns this object pattern.
    fn has_default_parent(&self, context: &DestackFormatContext<'_>) -> bool {
        match self {
            Self::Binding { node_id, .. } => {
                let Some((parent_id, NodeType::Pattern)) = context.parent(*node_id) else {
                    return false;
                };

                matches!(
                    context.tree.get(LocalNodeId::<Pattern>::new(parent_id)),
                    Pattern::Default { .. }
                )
            }
            Self::Assignment { node_id, .. } => {
                let Some((parent_id, NodeType::AssignPattern)) = context.parent(*node_id) else {
                    return false;
                };

                matches!(
                    context
                        .tree
                        .get(LocalNodeId::<AssignPattern>::new(parent_id)),
                    AssignPattern::Default { .. }
                )
            }
        }
    }

    /// Return whether the parent requires this pattern to stay inline.
    fn is_inline(&self, context: &DestackFormatContext<'_>) -> bool {
        match self {
            Self::Binding { node_id, .. } => binding_object_is_inline(context, *node_id),
            Self::Assignment { node_id, .. } => assignment_object_is_inline(context, *node_id),
        }
    }

    /// Select the object field layout.
    fn layout(&self, context: &DestackFormatContext<'_>) -> ObjectPatternLayout {
        let should_expand = !self.has_default_parent(context)
            && (self.has_nested_pattern(context.tree) || self.has_separator_comments(context));

        if should_expand {
            ObjectPatternLayout::Group { expand: true }
        } else if self.is_inline(context) {
            ObjectPatternLayout::Inline
        } else {
            ObjectPatternLayout::Group { expand: false }
        }
    }

    /// Format an empty object with its interior annotations.
    fn format_empty<'ast>(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Self::Binding { node_id, .. } => {
                format_empty_pattern_delimiter_with_interior_annotations(f, *node_id, "{", "}")
            }
            Self::Assignment { node_id, .. } => {
                format_empty_pattern_delimiter_with_interior_annotations(f, *node_id, "{", "}")
            }
        }
    }

    /// Write the object fields.
    fn write_fields<'ast>(
        &self,
        f: &mut DestackFormatter<'ast, '_>,
        trailing_separator: TrailingSeparator,
    ) -> FormatResult<()> {
        match self {
            Self::Binding { fields, .. } => write!(
                f,
                [separated_entries(
                    ",",
                    fields.as_ref(),
                    trailing_separator,
                    None
                )]
            ),
            Self::Assignment { fields, .. } => {
                write!(
                    f,
                    [separated_entries(",", fields, trailing_separator, None)]
                )
            }
        }
    }

    /// Format the complete object pattern.
    fn format<'ast>(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        if let Self::Binding { ty: Some(ty), .. } = self {
            write!(f, [ty, space()])?;
        }

        if self.is_empty() {
            return self.format_empty(f);
        }

        let trailing_separator = if self.forbids_trailing_separator(f.context().tree)
            || f.context().options.trailing_comma == TrailingComma::None
        {
            TrailingSeparator::Omit
        } else {
            TrailingSeparator::Allowed
        };
        let fields = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            self.write_fields(f, trailing_separator)
        });
        let body = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            if f.context().options.bracket_spacing {
                write!(f, [soft_space_or_block_indent(&fields)])
            } else {
                write!(f, [soft_block_indent(&fields)])
            }
        });

        write!(f, [token("{")])?;
        match self.layout(f.context()) {
            ObjectPatternLayout::Inline => write!(f, [body])?,
            ObjectPatternLayout::Group { expand } => {
                write!(f, [group(&body).should_expand(expand)])?;
            }
        }
        write!(f, [token("}")])
    }
}

/// Return whether trailing separators are invalid for the current pattern field list.
fn pattern_fields_disallow_trailing_separator(
    tree: &Tree,
    fields: &[LocalNodeId<PatternField>],
) -> bool {
    fields.last().is_some_and(|field_id| {
        matches!(
            tree.get(*field_id),
            PatternField::Rest { .. } | PatternField::Elision
        )
    })
}

/// Format a prefix pattern like `&pattern` or `^pattern`.
fn format_prefixed_pattern<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Pattern>,
    prefix: &'static str,
    right: LocalNodeId<Pattern>,
    mutability: Option<Mutability>,
    exclusivity: Option<Exclusivity>,
) -> FormatResult<()> {
    write!(f, [token(prefix)])?;

    if let Some(mutability) = mutability {
        match mutability {
            Mutability::Immutable => write!(f, [token("readonly"), space()])?,
            Mutability::Mutable => {}
        }
    }

    if exclusivity == Some(Exclusivity::Exclusive) {
        write!(f, [token("exclusive"), space()])?;
    }

    let operand_has_space = matches!(mutability, Some(Mutability::Immutable))
        || exclusivity == Some(Exclusivity::Exclusive);
    write_prefix_pattern_operand(f, node_id, right, operand_has_space)?;

    Ok(())
}

/// Return whether one prefix pattern operand needs spacing.
fn prefix_pattern_operand_needs_spacing(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Pattern>,
    right: LocalNodeId<Pattern>,
) -> bool {
    let right_span = context.span(right);
    let right_start = context.node_token_start(right);
    let prefix_span = context.span(node_id);
    let comments = context.comments();

    comments.has_comment_before(right_start)
        || comments.has_comment_in_range(right_span.end, prefix_span.end)
}

/// Write one prefix pattern operand with readable boundary comments.
fn write_prefix_pattern_operand<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Pattern>,
    right: LocalNodeId<Pattern>,
    operand_has_space: bool,
) -> FormatResult<()> {
    if !operand_has_space && prefix_pattern_operand_needs_spacing(f.context(), node_id, right) {
        write!(f, [space()])?;
    }

    write!(f, [right])?;

    Ok(())
}

/// Format one ordered range pattern.
fn format_range_pattern<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Pattern>,
    start: Option<LocalNodeId<Expression>>,
    end: Option<LocalNodeId<Expression>>,
    end_kind: RangeEnd,
) -> FormatResult<()> {
    let start_end = start.map(|start| f.context().node_token_end(start));
    let end_start = end.map(|end| f.context().node_token_start(end));

    if let Some(start) = start {
        write!(f, [start])?;
    }

    write_range_operator(f, f.context().span(node_id), start_end, end_start, end_kind)?;

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
    needs_singleton_tuple_separator: bool,
    should_expand: bool,
) -> FormatResult<()> {
    if fields.is_empty() {
        return format_empty_pattern_delimiter_with_interior_annotations(f, node_id, open, close);
    }

    let allow_trailing_separator =
        !pattern_fields_disallow_trailing_separator(f.context().tree, fields);
    let trailing_separator = if needs_singleton_tuple_separator && fields.len() == 1 {
        TrailingSeparator::Mandatory
    } else if !allow_trailing_separator || f.context().options.trailing_comma == TrailingComma::None
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

/// Format one list-like assign-pattern field collection with shared trailing-separator behavior.
fn format_assign_pattern_field_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<AssignPattern>,
    open: &'static str,
    close: &'static str,
    fields: &[LocalNodeId<AssignPatternField>],
    is_tuple: bool,
    should_expand: bool,
) -> FormatResult<()> {
    if fields.is_empty() {
        return format_empty_pattern_delimiter_with_interior_annotations(f, node_id, open, close);
    }

    let allow_trailing_separator =
        !assign_pattern_fields_disallow_trailing_separator(f.context().tree, fields);
    let trailing_separator = if is_tuple && fields.len() == 1 {
        TrailingSeparator::Mandatory
    } else if !allow_trailing_separator || f.context().options.trailing_comma == TrailingComma::None
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

/// Return the single positional object payload in a newtype pattern.
fn newtype_object_payload(
    tree: &Tree,
    fields: &[LocalNodeId<PatternField>],
) -> Option<LocalNodeId<Pattern>> {
    let [field_id] = fields else {
        return None;
    };

    let PatternField::Positional { pattern } = tree.get(*field_id) else {
        return None;
    };

    if matches!(tree.get(*pattern), Pattern::Object { .. }) {
        return Some(*pattern);
    }

    None
}

/// Format one object-backed newtype pattern.
fn format_newtype_object_pattern<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    ty: LocalNodeId<TypeExpression>,
    payload: LocalNodeId<Pattern>,
) -> FormatResult<()> {
    write!(f, [ty, token("("), payload, token(")")])?;

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
            AssignPatternField::Rest { .. } | AssignPatternField::Elision
        )
    })
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

/// Return whether one binding object must stay inline in its parent.
fn binding_object_is_inline(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Pattern>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    match parent_type {
        NodeType::Parameter => {
            let parameter = context.tree.get(LocalNodeId::<Parameter>::new(parent_id));
            let pattern = match parameter {
                Parameter::Pattern { pattern, .. } | Parameter::VariadicPattern { pattern, .. } => {
                    Some(*pattern)
                }
                Parameter::Named { .. } | Parameter::VariadicNamed { .. } | Parameter::Error => {
                    None
                }
            };

            pattern.is_some_and(|pattern| pattern == node_id)
        }
        NodeType::Declarator => {
            let declarator = context.tree.get(LocalNodeId::<Declarator>::new(parent_id));

            declarator.pattern == node_id
        }
        _ => false,
    }
}

/// Return whether one assignment object is the direct target of an assignment.
fn assignment_object_is_inline(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<AssignPattern>,
) -> bool {
    let Some((parent_id, NodeType::Expression)) = context.parent(node_id) else {
        return false;
    };

    let expression = context.tree.get(LocalNodeId::<Expression>::new(parent_id));

    matches!(expression, Expression::Assign { left, .. } if *left == node_id)
}

/// Return whether one binding pattern directly destructures an object or sequence.
pub(crate) fn pattern_is_destructuring(tree: &Tree, pattern_id: LocalNodeId<Pattern>) -> bool {
    match tree.get(pattern_id) {
        Pattern::Object { .. }
        | Pattern::NominalObject { .. }
        | Pattern::Sequence { .. }
        | Pattern::NominalTuple { .. }
        | Pattern::Tuple { .. } => true,
        Pattern::Must(pattern)
        | Pattern::BorrowOf { right: pattern, .. }
        | Pattern::MoveOf { right: pattern, .. }
        | Pattern::DereferenceOf { right: pattern } => pattern_is_destructuring(tree, *pattern),
        Pattern::Binding { .. }
        | Pattern::Default { .. }
        | Pattern::Wildcard
        | Pattern::Expression { .. }
        | Pattern::Range { .. }
        | Pattern::Union { .. } => false,
    }
}

/// Return whether one assignment pattern directly destructures an object or sequence.
fn assign_pattern_is_destructuring(tree: &Tree, pattern_id: LocalNodeId<AssignPattern>) -> bool {
    match tree.get(pattern_id) {
        AssignPattern::Object { .. }
        | AssignPattern::Sequence { .. }
        | AssignPattern::Tuple { .. } => true,
        AssignPattern::Default { .. } | AssignPattern::Place { .. } => false,
    }
}

/// Return whether two object fields have comments between them.
fn object_field_gap_has_comments<T>(
    context: &DestackFormatContext<'_>,
    left_id: LocalNodeId<T>,
    right_id: LocalNodeId<T>,
) -> bool
where
    T: Node + Clone,
    Tree: TreeStore<T>,
{
    let left_span = context.span(left_id);
    let right_span = context.span(right_id);

    !context
        .comments()
        .comments_in_range(left_span.end, right_span.start)
        .is_empty()
}

/// Format one binding assignment pattern in assignment-pattern order.
fn format_pattern_assignment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    pattern: LocalNodeId<Pattern>,
    value: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let left = CapturedFormat::new(f, pattern)?;

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
    let left = CapturedFormat::new(f, pattern)?;

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
        if let Pattern::Default { pattern, value } = self {
            format_pattern_assignment(f, *pattern, *value)?;
            write!(f, [infix_or_postfix_annotations(f.context(), node_id)])?;
            return Ok(());
        }

        write!(f, [prefix_annotations(f.context(), node_id)])?;

        match self {
            Pattern::Wildcard => write!(f, [token("_")])?,
            Pattern::Must(unwrap) => write!(f, [unwrap, token("!")])?,

            Pattern::Default { .. } => {
                return Err(FormatError::SyntaxError {
                    message: "default pattern reached ordinary pattern formatting",
                });
            }

            Pattern::BorrowOf {
                right,
                mutability,
                exclusivity,
            } => {
                format_prefixed_pattern(f, node_id, "&", *right, *mutability, *exclusivity)?;
            }

            Pattern::MoveOf { right, mutability } => {
                format_prefixed_pattern(f, node_id, "^", *right, *mutability, None)?;
            }

            Pattern::DereferenceOf { right } => {
                write!(f, [token("*")])?;
                write_prefix_pattern_operand(f, node_id, *right, false)?;
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
                format_range_pattern(f, node_id, *start, *end, *end_kind)?;
            }

            Pattern::Tuple { fields } => {
                format_pattern_field_list(f, node_id, "(", ")", fields, true, false)?;
            }

            Pattern::NominalTuple { ty, fields } => {
                if let Some(payload) = newtype_object_payload(f.context().tree, fields) {
                    format_newtype_object_pattern(f, *ty, payload)?;
                } else {
                    write!(f, [ty])?;
                    format_pattern_field_list(f, node_id, "(", ")", fields, false, false)?;
                }
            }

            Pattern::Sequence { fields } => {
                format_pattern_field_list(f, node_id, "[", "]", fields, false, false)?;
            }

            Pattern::Object { fields } => {
                ObjectPattern::binding(f.context().tree, node_id, None, fields).format(f)?;
            }

            Pattern::NominalObject { ty, fields } => {
                ObjectPattern::binding(f.context().tree, node_id, Some(*ty), fields).format(f)?;
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
                    let pattern = pattern.ok_or(FormatError::SyntaxError {
                        message: "expanded named pattern field requires a pattern",
                    })?;
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

            PatternField::Rest { pattern } => {
                // rest pattern
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
        if let AssignPattern::Default { pattern, value } = self {
            format_assign_pattern_assignment(f, *pattern, *value)?;
            write!(f, [infix_or_postfix_annotations(f.context(), node_id)])?;
            return Ok(());
        }

        write!(f, [prefix_annotations(f.context(), node_id)])?;

        match self {
            AssignPattern::Place { expression: value } => {
                write!(f, [value])?;
            }

            AssignPattern::Default { .. } => {
                return Err(FormatError::SyntaxError {
                    message: "default assignment pattern reached ordinary pattern formatting",
                });
            }

            AssignPattern::Sequence { fields } => {
                format_assign_pattern_field_list(f, node_id, "[", "]", fields, false, false)?;
            }

            AssignPattern::Tuple { fields } => {
                format_assign_pattern_field_list(f, node_id, "(", ")", fields, true, false)?;
            }

            AssignPattern::Object { fields } => {
                ObjectPattern::assignment(node_id, fields).format(f)?;
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
                    write!(f, [name, token(":"), space(), pattern])?;
                }
                // shorthand default
                else if matches!(
                    f.context().tree.get(*pattern),
                    AssignPattern::Default { .. }
                ) {
                    write!(f, [name])?;
                    write_shorthand_assign_pattern_value(f, *pattern)?;
                }
                // shorthand target
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

            AssignPatternField::Rest { pattern } => {
                // rest pattern
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

    let Pattern::Default { value, .. } = pattern else {
        return Err(FormatError::SyntaxError {
            message: "shorthand assignment requires a default pattern",
        });
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

    let AssignPattern::Default { value, .. } = pattern else {
        return Err(FormatError::SyntaxError {
            message: "shorthand assignment target requires a default pattern",
        });
    };

    write!(f, [space(), token("="), space(), value])
}
