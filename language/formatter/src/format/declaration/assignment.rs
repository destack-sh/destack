use crate::format::annotation::{format_raw_comment, prefix_annotations};
use crate::format::chain::{MemberChain, transparent_inner_expression};
use crate::format::declaration::declaration::format_declaration_export_modifier;
use crate::format::declaration::signature::{
    default_static_parameter_trailing_separator, write_static_parameter_list,
};
use crate::format::expression::expression_has_static_type_arguments;
use crate::format::operator::{
    expression_has_type_grouping_semantics, format_binary_expression, format_static_argument_list,
    should_drop_parenthesized_type_expression, transparent_type_binary_root_expression,
    write_type_expression_with_inline_prefix_annotations,
    write_type_expression_without_prefix_annotations,
};
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Argument, BinaryOperator, Declaration, DeclarationDescriptor, DeclarationKind, Expression,
    Keyword, LocalNodeId, Mutability, Parameter, ScalarLiteral, TypeKind,
};
use destack_fir::format::{
    FormatNode, FormatNodes, FormatResult, Formatter as FirFormatter, VecBuffer,
};
use destack_fir::prelude::*;
use destack_fir::write;

const MIN_OVERLAP_FOR_BREAK: u32 = 3;

/// One assignment-like layout for type aliases.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TypeAliasLayout {
    Fluid,
    BreakAfterOperator,
    NeverBreakAfterOperator,
    BreakLeftHandSide,
}

/// Return whether one argument expression is short enough to keep a call attached.
fn is_short_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
    threshold: u32,
) -> bool {
    let argument_expression_id = match context.tree.get(argument_id) {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => *value,
        Argument::Error => return false,
    };

    is_short_expression(context, argument_expression_id, threshold)
}

/// Return whether one expression is short enough to keep a call attached.
fn is_short_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    threshold: u32,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);

    match context.tree.get(expression_id) {
        Expression::Identifier { name } => context.strings.get(*name).len() <= threshold as usize,
        Expression::Unary { right, .. } => is_short_expression(context, *right, threshold),
        Expression::ScalarLiteral(
            ScalarLiteral::Boolean(_)
            | ScalarLiteral::Integer(_)
            | ScalarLiteral::Bigint(_)
            | ScalarLiteral::Float(_),
        )
        | Expression::This => true,
        Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
            context.strings.get(*string_id).len() <= threshold as usize
        }
        Expression::ScalarLiteral(ScalarLiteral::RegexString { content, .. }) => {
            context.strings.get(*content).len() <= threshold as usize
        }
        Expression::TemplateExpression { .. } => !context.node_has_newline(expression_id),
        Expression::Call {
            left,
            dynamic_arguments,
            ..
        } => {
            dynamic_arguments.is_empty()
                && matches!(
                    context.tree.get(transparent_inner_expression(context, *left)),
                    Expression::Identifier { name }
                        if context.strings.get(*name).len()
                            <= threshold.saturating_sub(2) as usize
                )
        }
        _ => false,
    }
}

/// Return whether one static argument list is complex enough to break a call chain.
fn is_complex_type_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    static_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if static_arguments.len() > 1 {
        return true;
    }

    let Some(argument_id) = static_arguments.first().copied() else {
        return false;
    };
    let argument_expression_id = match f.context().tree.get(argument_id) {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => *value,
        Argument::Error => return false,
    };
    let argument_expression_id = transparent_inner_expression(f.context(), argument_expression_id);

    if matches!(
        f.context().tree.get(argument_expression_id),
        Expression::Binary {
            operator: BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd,
            ..
        } | Expression::TypeLiteral(_)
            | Expression::TypeMapped { .. }
    ) {
        return true;
    }

    let mut buffer = VecBuffer::new(f.state_mut());
    let formatter = &mut FirFormatter::new(&mut buffer);
    if format_static_argument_list(formatter, static_arguments).is_err() {
        return true;
    }

    buffer.into_vec().as_slice().will_break()
}

/// Return whether one call or member chain is awkward to break inside an assignment shell.
pub(crate) fn is_poorly_breakable_member_or_call_chain<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let threshold = u32::from(f.context().options.line_width) / 4;
    let root_expression_id = transparent_inner_expression(f.context(), expression_id);
    let mut current_expression_id = root_expression_id;
    let mut is_chain = false;
    let mut has_simple_head = false;
    let mut call_expression_ids = Vec::new();
    let mut call_static_argument_groups = Vec::<Vec<LocalNodeId<Argument>>>::new();

    loop {
        current_expression_id = match f.context().tree.get(current_expression_id) {
            Expression::Call {
                left,
                static_arguments,
                ..
            } => {
                is_chain = true;
                call_expression_ids.push(current_expression_id);
                call_static_argument_groups.push(static_arguments.clone().unwrap_or_default());
                transparent_inner_expression(f.context(), *left)
            }
            Expression::Instantiation {
                left,
                static_arguments,
            } => {
                is_chain = true;
                if is_complex_type_arguments(f, static_arguments) {
                    return false;
                }

                transparent_inner_expression(f.context(), *left)
            }
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Index { left, .. } => {
                is_chain = true;
                transparent_inner_expression(f.context(), *left)
            }
            Expression::Maybe { left, .. } | Expression::Must { left, .. } => {
                is_chain = true;
                transparent_inner_expression(f.context(), *left)
            }
            Expression::Identifier { .. } | Expression::This => {
                has_simple_head = true;
                break;
            }
            _ => break,
        };
    }

    if !is_chain || !has_simple_head {
        return false;
    }

    if f.context()
        .comments()
        .has_comment_in_span(f.context().span(root_expression_id))
    {
        return false;
    }

    if call_expression_ids.is_empty() {
        return true;
    }

    if f.context()
        .comments()
        .has_comment_in_span(f.context().span(call_expression_ids[0]))
    {
        return false;
    }

    for (index, call_expression_id) in call_expression_ids.iter().copied().enumerate() {
        let Expression::Call {
            dynamic_arguments, ..
        } = f.context().tree.get(call_expression_id)
        else {
            continue;
        };

        let is_breakable_call = match dynamic_arguments.len() {
            0 => false,
            1 => {
                let argument_id = dynamic_arguments[0];
                !is_short_argument(f.context(), argument_id, threshold)
            }
            _ => true,
        };
        if is_breakable_call {
            return false;
        }

        if let Some(static_arguments) = call_static_argument_groups.get(index)
            && is_complex_type_arguments(f, static_arguments)
        {
            return false;
        }
    }

    MemberChain::tail_group_count(f.context(), root_expression_id)
        .map(|count| count <= 1)
        .unwrap_or(true)
}

/// One assignment-like helper for type alias declarations.
struct TypeAliasAssignmentLike<'a> {
    /// The declaration node id.
    node_id: LocalNodeId<Declaration>,
    /// The declaration descriptor.
    descriptor: &'a DeclarationDescriptor,
    /// The alias kind.
    kind: TypeKind,
    /// The alias mutability.
    mutability: Option<Mutability>,
    /// The static parameter list.
    static_parameters: &'a Option<Vec<LocalNodeId<Parameter>>>,
    /// The alias rhs expression.
    value_id: LocalNodeId<Expression>,
}

impl<'a> TypeAliasAssignmentLike<'a> {
    /// Return whether rhs comments should stay leading on the type body.
    fn should_print_rhs_comments_as_leading(&self, context: &DestackFormatContext<'_>) -> bool {
        matches!(context.tree.get(self.value_id), Expression::TypeLiteral(_))
    }

    /// Return the raw source offset where left-trailing boundary comments begin.
    fn left_trailing_comment_start(&self, context: &DestackFormatContext<'_>) -> Option<u32> {
        if let Some(static_parameters) = self.static_parameters
            && let Some(last_parameter_id) = static_parameters.last().copied()
        {
            let last_parameter_span = context.span(last_parameter_id);
            let close_angle = context.next_non_whitespace_token_after_span(last_parameter_span);
            return Some(close_angle.map_or(last_parameter_span.end, |token| token.span.end));
        }

        context
            .tree
            .get_main_span(self.node_id)
            .map(|span| span.end)
    }

    /// Buffer the left side so the assignment shell can classify the whole lhs first.
    fn buffer_left<'ast>(
        &self,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<(Vec<FormatNode>, bool, bool)> {
        let mut buffer = VecBuffer::new(f.state_mut());
        self.write_left(&mut FirFormatter::new(&mut buffer))?;

        let nodes = buffer.into_vec();
        let may_break = nodes.may_directly_break();
        let is_short = nodes.single_line_width().is_some_and(|width| {
            width < (u32::from(f.context().options.indent_width) + MIN_OVERLAP_FOR_BREAK)
        });

        Ok((nodes, is_short, may_break))
    }

    /// Write comments that belong after the left side and before `=`.
    fn write_left_trailing_comments<'ast>(
        &self,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // type literals keep leading comments on the right side
        if self.should_print_rhs_comments_as_leading(f.context()) {
            return Ok(());
        }

        let Some(start) = self.left_trailing_comment_start(f.context()) else {
            return Ok(());
        };

        let end_of_line_comments = f.context().end_of_line_raw_comments_after(start);
        let comment_nodes = if end_of_line_comments.is_empty() {
            let comment_nodes = {
                let comments = f.context().comments();
                comments.comments_before_character(start, b'=').to_vec()
            };
            if comment_nodes
                .iter()
                .any(|comment_id| f.context().span_starts_on_own_line(comment_id.span))
            {
                Vec::new()
            } else {
                comment_nodes
            }
        } else if end_of_line_comments
            .last()
            .is_some_and(|comment| comment.is_block())
        {
            Vec::new()
        } else {
            end_of_line_comments
        };
        if comment_nodes.is_empty() {
            return Ok(());
        }

        for comment_id in &comment_nodes {
            let comment_span = comment_id.span;
            if f.context().span_starts_on_own_line(comment_span) {
                write!(f, [hard_line_break()])?;
                format_raw_comment(f, *comment_id)?;
            } else {
                write!(f, [space()])?;
                format_raw_comment(f, *comment_id)?;
            }
        }

        // slash comments own the rest of the line before the operator
        if comment_nodes.iter().any(|comment_id| comment_id.is_line()) {
            write!(f, [hard_line_break()])?;
        }

        Ok(())
    }

    /// Write the left side of the type alias.
    fn write_left<'ast>(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        // export
        format_declaration_export_modifier(f, self.node_id, self.descriptor)?;

        // kind
        if self.descriptor.kind == DeclarationKind::Declaration {
            write!(f, [Keyword::Declare, space()])?;
        }

        // keyword
        if self.mutability == Some(Mutability::Immutable) {
            write!(f, [Keyword::Readonly])?;
        } else if self.kind == TypeKind::Structural {
            write!(f, [Keyword::Type])?;
        } else {
            write!(f, [Keyword::Newtype])?;
        }

        // name
        if let Some(name) = self.descriptor.name {
            write!(f, [space(), name])?;
        }

        // static parameters
        if let Some(static_parameters) = self.static_parameters {
            write_static_parameter_list(
                f,
                static_parameters,
                default_static_parameter_trailing_separator(f),
            )?;
            write!(f, [prefix_annotations(f.context(), self.node_id)])?;
        }

        // left trailing comments
        self.write_left_trailing_comments(f)?;

        Ok(())
    }

    /// Write the operator boundary of the type alias.
    fn write_operator<'ast>(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        write!(f, [space(), token("=")])
    }

    /// Return whether one type-conditional test operand is generic-like for assignment layout.
    fn type_conditional_test_operand_is_generic_like(
        &self,
        context: &DestackFormatContext<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        if expression_has_static_type_arguments(context, expression_id) {
            return true;
        }

        let expression_id = transparent_inner_expression(context, expression_id);
        let Expression::Declaration(declaration_id) = context.tree.get(expression_id) else {
            return false;
        };

        context
            .tree
            .get(*declaration_id)
            .static_parameters()
            .is_some_and(|parameters| !parameters.is_empty())
    }

    /// Return whether one static parameter has a constraint or default.
    fn static_parameter_has_constraint_or_default(
        &self,
        context: &DestackFormatContext<'_>,
        parameter_id: LocalNodeId<Parameter>,
    ) -> bool {
        match context.tree.get(parameter_id) {
            Parameter::Named { ty, default, .. } | Parameter::Pattern { ty, default, .. } => {
                ty.is_some() || default.is_some()
            }
            Parameter::VariadicNamed { ty, .. } | Parameter::VariadicPattern { ty, .. } => {
                ty.is_some()
            }
            Parameter::Error => false,
        }
    }

    /// Return whether the current type alias should use left-hand-side breaking.
    fn is_complex_type_alias(&self, context: &DestackFormatContext<'_>) -> bool {
        self.static_parameters.as_ref().is_some_and(|parameters| {
            parameters.len() > 1
                && parameters.iter().copied().any(|parameter_id| {
                    self.static_parameter_has_constraint_or_default(context, parameter_id)
                })
        })
    }

    /// Return whether the current type alias should break after `=`.
    fn should_break_after_operator(
        &self,
        context: &DestackFormatContext<'_>,
        _is_left_short: bool,
    ) -> bool {
        let value_span = context.span(self.value_id);
        let has_leading_comments = !context.comment_tokens_before(value_span.start).is_empty();
        let transparent_type_binary_root = [
            destack_ast::BinaryOperator::ElementwiseOr,
            destack_ast::BinaryOperator::ElementwiseAnd,
        ]
        .into_iter()
        .find_map(|operator| {
            let root_id = transparent_type_binary_root_expression(context, self.value_id, operator);
            matches!(
                context.tree.get(root_id),
                Expression::Binary {
                    operator: root_operator,
                    ..
                } if *root_operator == operator
            )
            .then_some(root_id)
        });

        match context.tree.get(self.value_id) {
            Expression::TypeConditional { left, right, .. } => {
                self.type_conditional_test_operand_is_generic_like(context, *left)
                    || self.type_conditional_test_operand_is_generic_like(context, *right)
                    || has_leading_comments
            }
            _ if transparent_type_binary_root.is_some() => false,
            Expression::Binary { .. } => false,
            _ => has_leading_comments,
        }
    }

    /// Return whether the current type alias should keep the rhs attached after `=`.
    fn should_never_break_after_operator(
        &self,
        context: &DestackFormatContext<'_>,
        is_left_short: bool,
        left_may_break: bool,
    ) -> bool {
        if left_may_break {
            return false;
        }

        if is_left_short {
            return true;
        }

        matches!(context.tree.get(self.value_id), Expression::TypeLiteral(_))
    }

    /// Return the layout for the current type alias shell.
    fn layout(
        &self,
        context: &DestackFormatContext<'_>,
        is_left_short: bool,
        left_may_break: bool,
    ) -> TypeAliasLayout {
        if self.should_break_after_operator(context, is_left_short) {
            return TypeAliasLayout::BreakAfterOperator;
        }

        if self.is_complex_type_alias(context) {
            return TypeAliasLayout::BreakLeftHandSide;
        }

        if self.should_never_break_after_operator(context, is_left_short, left_may_break) {
            return TypeAliasLayout::NeverBreakAfterOperator;
        }

        TypeAliasLayout::Fluid
    }

    /// Return the normalized rhs expression id for one type alias value.
    fn normalized_right_value_id(
        &self,
        context: &DestackFormatContext<'_>,
    ) -> Option<LocalNodeId<Expression>> {
        [
            destack_ast::BinaryOperator::ElementwiseOr,
            destack_ast::BinaryOperator::ElementwiseAnd,
        ]
        .into_iter()
        .find_map(|operator| {
            let normalized_root_id =
                transparent_type_binary_root_expression(context, self.value_id, operator);

            matches!(
                context.tree.get(normalized_root_id),
                Expression::Binary {
                    operator: root_operator,
                    ..
                } if *root_operator == operator
            )
            .then_some(normalized_root_id)
        })
        .or_else(|| match context.tree.get(self.value_id) {
            Expression::Parenthesized { expression } => {
                let normalized_inner_id = transparent_inner_expression(context, *expression);
                let should_route_through_type_binary_owner = matches!(
                    context.tree.get(normalized_inner_id),
                    Expression::Binary {
                        operator: destack_ast::BinaryOperator::ElementwiseOr
                            | destack_ast::BinaryOperator::ElementwiseAnd,
                        ..
                    }
                )
                    && expression_has_type_grouping_semantics(context, normalized_inner_id);

                if should_route_through_type_binary_owner {
                    Some(normalized_inner_id)
                } else if should_drop_parenthesized_type_expression(
                    context,
                    self.value_id,
                    *expression,
                ) {
                    Some(*expression)
                } else {
                    None
                }
            }
            _ => None,
        })
    }

    /// Write the rhs through the shared binary owner when the value is one type binary.
    fn write_binary_right<'ast>(
        &self,
        f: &mut DestackFormatter<'ast, '_>,
        normalized_value_id: LocalNodeId<Expression>,
    ) -> FormatResult<bool> {
        let Expression::Binary {
            left,
            operator:
                operator @ (destack_ast::BinaryOperator::ElementwiseOr
                | destack_ast::BinaryOperator::ElementwiseAnd),
            right,
        } = f.context().tree.get(normalized_value_id)
        else {
            return Ok(false);
        };

        let context = f.context().clone();
        context.with_assignment_like_type_root(self.value_id, || {
            context.with_type_expression_root(self.value_id, || {
                format_binary_expression(f, self.value_id, *left, operator, *right)
            })
        })?;

        Ok(true)
    }

    /// Write the rhs with inline prefix annotations when possible.
    fn write_non_binary_right<'ast>(
        &self,
        f: &mut DestackFormatter<'ast, '_>,
        value_id: LocalNodeId<Expression>,
    ) -> FormatResult<()> {
        let has_inline_prefix_annotation = f
            .context()
            .annotation_ids(value_id)
            .iter()
            .copied()
            .any(|annotation_id| {
                matches!(
                    f.context().annotation(annotation_id).position(),
                    destack_ast::AnnotationPosition::BlockPrefix
                        | destack_ast::AnnotationPosition::LinePrefix
                )
            });

        if !has_inline_prefix_annotation {
            return write_type_expression_with_inline_prefix_annotations(f, value_id);
        }

        write!(f, [prefix_annotations(f.context(), value_id)])?;

        let last_prefix_annotation_id = f
            .context()
            .annotation_ids(value_id)
            .iter()
            .copied()
            .filter(|annotation_id| {
                matches!(
                    f.context().annotation(*annotation_id).position(),
                    destack_ast::AnnotationPosition::BlockPrefix
                        | destack_ast::AnnotationPosition::LinePrefix
                )
            })
            .last();

        if let Some(last_prefix_annotation_id) = last_prefix_annotation_id
            && f.context()
                .annotation_next_token_is_on_same_line(last_prefix_annotation_id)
        {
            write!(f, [space()])?;
        }

        write_type_expression_without_prefix_annotations(f, value_id)
    }

    /// Write the rhs of the type alias for one assignment-like layout.
    fn write_right<'ast>(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        let normalized_value_id = self.normalized_right_value_id(f.context());

        if let Some(normalized_value_id) = normalized_value_id
            && self.write_binary_right(f, normalized_value_id)?
        {
            return Ok(());
        }

        self.write_non_binary_right(f, normalized_value_id.unwrap_or(self.value_id))
    }

    /// Write the rhs with the selected assignment-like layout.
    fn write_layout_right<'ast>(
        &self,
        f: &mut DestackFormatter<'ast, '_>,
        layout: TypeAliasLayout,
        right: &impl destack_fir::format::Format<DestackFormatContext<'ast>>,
    ) -> FormatResult<()> {
        match layout {
            TypeAliasLayout::Fluid => {
                let group_id = f.group_id("type_alias_rhs");

                write!(
                    f,
                    [
                        group(&indent(&soft_line_break_or_space())).with_id(Some(group_id)),
                        line_suffix_boundary(),
                        indent_if_group_breaks(right, group_id)
                    ]
                )
            }
            TypeAliasLayout::BreakAfterOperator => {
                write!(f, [group(&soft_line_indent_or_space(right))])
            }
            TypeAliasLayout::NeverBreakAfterOperator => {
                write!(f, [space(), right])
            }
            TypeAliasLayout::BreakLeftHandSide => {
                write!(f, [space(), group(right)])
            }
        }
    }

    /// Write the full assignment-like content for one selected layout.
    fn write_content<'ast>(
        &self,
        f: &mut DestackFormatter<'ast, '_>,
        layout: TypeAliasLayout,
        left: &impl destack_fir::format::Format<DestackFormatContext<'ast>>,
        right: &impl destack_fir::format::Format<DestackFormatContext<'ast>>,
    ) -> FormatResult<()> {
        if layout == TypeAliasLayout::BreakLeftHandSide {
            write!(f, [left])?;
        } else {
            write!(f, [group(left)])?;
        }

        self.write_operator(f)?;
        self.write_layout_right(f, layout, right)
    }

    /// Format the full assignment-like type alias shell.
    fn fmt<'ast>(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        // lhs buffering
        let (left_nodes, is_left_short, left_may_break) = self.buffer_left(f)?;
        let layout = self.layout(f.context(), is_left_short, left_may_break);

        // buffered lhs doc
        let left = f.intern_vec(left_nodes);
        let left = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
            if let Some(left) = &left {
                f.write_node(left.clone());
            }

            Ok(())
        });
        let right = format_with(|f: &mut DestackFormatter<'ast, '_>| self.write_right(f));
        let inner_content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            self.write_content(f, layout, &left, &right)
        });

        write!(f, [group(&inner_content)])
    }
}

/// Format one type alias declaration with an assignment-like shell.
pub(crate) fn format_type_alias_assignment_like<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &DeclarationDescriptor,
    kind: TypeKind,
    mutability: Option<Mutability>,
    static_parameters: &Option<Vec<LocalNodeId<Parameter>>>,
    value_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let assignment_like = TypeAliasAssignmentLike {
        node_id,
        descriptor,
        kind,
        mutability,
        static_parameters,
        value_id,
    };

    assignment_like.fmt(f)
}
