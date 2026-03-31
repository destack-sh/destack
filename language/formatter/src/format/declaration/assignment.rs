use crate::format::chain::transparent_inner_expression;
use crate::format::collection::TrailingSeparator;
use crate::format::declaration::declaration::format_declaration_export_modifier;
use crate::format::declaration::signature::write_static_parameter_list;
use crate::format::expression::expression_has_static_type_arguments;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    CommentStyle, Declaration, DeclarationDescriptor, DeclarationKind, Expression, Keyword,
    LocalNodeId, Mutability, Parameter, TypeKind,
};
use destack_fir::format::{
    FormatNode as FirFormatNode, FormatResult, Formatter as FirFormatter, LineMode, VecBuffer,
};
use destack_fir::prelude::*;
use destack_fir::write;

const MIN_OVERLAP_FOR_BREAK: u32 = 3;

/// One OXC-style assignment-like layout for type aliases.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TypeAliasLayout {
    Fluid,
    BreakAfterOperator,
    NeverBreakAfterOperator,
    BreakLeftHandSide,
}

/// Return whether buffered format nodes may directly break.
fn buffered_nodes_will_break(nodes: &[FirFormatNode]) -> bool {
    nodes.iter().any(|node| match node {
        FirFormatNode::Line(LineMode::Hard | LineMode::Empty) => true,
        FirFormatNode::Token { text } => text.contains('\n'),
        FirFormatNode::Text { width, .. } | FirFormatNode::FileSlice { width, .. } => {
            width.width().is_none()
        }
        FirFormatNode::Interned(interned) => buffered_nodes_will_break(interned),
        FirFormatNode::BestFitting { variants, .. } => {
            buffered_nodes_will_break(variants.as_slice())
        }
        _ => false,
    })
}

/// Return the single-line width of one buffered node list when all parts are measurable.
fn buffered_nodes_single_line_width(nodes: &[FirFormatNode]) -> Option<u32> {
    let mut width = 0u32;

    for node in nodes {
        match node {
            FirFormatNode::Space => width = width.saturating_add(1),
            FirFormatNode::Token { text } => width = width.saturating_add(text.len() as u32),
            FirFormatNode::Text {
                width: text_width, ..
            }
            | FirFormatNode::FileSlice {
                width: text_width, ..
            } => {
                width = width.saturating_add(text_width.width()?.value());
            }
            FirFormatNode::Interned(interned) => {
                width = width.saturating_add(buffered_nodes_single_line_width(interned)?);
            }
            FirFormatNode::BestFitting { variants, .. } => {
                width =
                    width.saturating_add(buffered_nodes_single_line_width(variants.as_slice())?);
            }
            FirFormatNode::Line(LineMode::SoftOrSpace) => width = width.saturating_add(1),
            FirFormatNode::Line(_)
            | FirFormatNode::ExpandParent
            | FirFormatNode::SourcePosition { .. }
            | FirFormatNode::LinePostfixBoundary
            | FirFormatNode::Tag(_) => return None,
        }
    }

    Some(width)
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
    /// Return whether rhs comments should stay leading on the type body, like OXC.
    fn should_print_rhs_comments_as_leading(&self, context: &DestackFormatContext<'_>) -> bool {
        matches!(context.tree.get(self.value_id), Expression::TypeLiteral(_))
    }

    /// Return the raw source offset where left-trailing seam comments begin.
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
    ) -> FormatResult<(Vec<FirFormatNode>, bool, bool)> {
        let mut buffer = VecBuffer::new(f.state_mut());
        self.write_left(&mut FirFormatter::new(&mut buffer))?;

        let nodes = buffer.into_vec();
        let may_break = buffered_nodes_will_break(&nodes);
        let is_short = buffered_nodes_single_line_width(&nodes).is_some_and(|width| {
            width < (u32::from(f.context().options.indent_width) + MIN_OVERLAP_FOR_BREAK)
        });

        Ok((nodes, is_short, may_break))
    }

    /// Write comments that belong after the left side and before `=`.
    fn write_left_trailing_comments<'ast>(
        &self,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // type literals keep leading comments on the right side, like OXC
        if self.should_print_rhs_comments_as_leading(f.context()) {
            return Ok(());
        }

        let Some(start) = self.left_trailing_comment_start(f.context()) else {
            return Ok(());
        };

        let end_of_line_comments = f.context().end_of_line_comment_nodes_after(start);
        let comment_nodes = if end_of_line_comments.is_empty() {
            let comment_nodes = f.context().comment_nodes_before_character(start, b'=');
            if comment_nodes.iter().any(|comment_id| {
                f.context()
                    .span_starts_on_own_line(f.context().span(*comment_id))
            }) {
                Vec::new()
            } else {
                comment_nodes
            }
        } else if end_of_line_comments
            .last()
            .is_some_and(|comment_id| f.context().tree.get(*comment_id).style == CommentStyle::Star)
        {
            Vec::new()
        } else {
            end_of_line_comments
        };
        if comment_nodes.is_empty() {
            return Ok(());
        }

        for comment_id in &comment_nodes {
            let comment_span = f.context().span(*comment_id);
            if f.context().span_starts_on_own_line(comment_span) {
                write!(f, [hard_line_break(), *comment_id])?;
            } else {
                write!(f, [space(), *comment_id])?;
            }
        }

        // slash comments own the rest of the line before the operator
        if comment_nodes
            .iter()
            .any(|comment_id| f.context().tree.get(*comment_id).style == CommentStyle::Slash)
        {
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
            write_static_parameter_list(f, static_parameters, TrailingSeparator::Disallowed)?;
            write!(
                f,
                [crate::format::annotation::prefix_annotations(
                    f.context(),
                    self.node_id
                )]
            )?;
        }

        // left trailing comments
        self.write_left_trailing_comments(f)?;

        Ok(())
    }

    /// Write the operator seam of the type alias.
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
        let has_leading_comments = context.has_comments_before(value_span.start);

        match context.tree.get(self.value_id) {
            Expression::TypeConditional { left, right, .. } => {
                self.type_conditional_test_operand_is_generic_like(context, *left)
                    || self.type_conditional_test_operand_is_generic_like(context, *right)
                    || has_leading_comments
            }
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

        matches!(
            context.tree.get(self.value_id),
            Expression::TypeLiteral(_) | Expression::TypeTemplateLiteral { .. }
        )
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

    /// Write the rhs of the type alias for one assignment-like layout.
    fn write_right<'ast>(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        write!(f, [self.value_id])
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
            if layout == TypeAliasLayout::BreakLeftHandSide {
                write!(f, [left])?;
            } else {
                write!(f, [group(&left)])?;
            }

            self.write_operator(f)?;

            match layout {
                TypeAliasLayout::Fluid => {
                    let group_id = f.group_id("type_alias_rhs");

                    write!(
                        f,
                        [
                            group(&indent(&soft_line_break_or_space())).with_id(Some(group_id)),
                            line_postfix_boundary(),
                            indent_if_group_breaks(&right, group_id)
                        ]
                    )
                }
                TypeAliasLayout::BreakAfterOperator => {
                    write!(f, [group(&soft_line_indent_or_space(&right))])
                }
                TypeAliasLayout::NeverBreakAfterOperator => {
                    write!(f, [space(), right])
                }
                TypeAliasLayout::BreakLeftHandSide => {
                    write!(f, [space(), group(&right)])
                }
            }
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
