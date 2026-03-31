use super::write_expression_without_trailing_annotations;
use crate::format::annotation::write_annotation_sequence_without_trailing_break;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{AnnotationPosition, Comment, Expression, Keyword, LocalNodeId, NodeType};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{
    align, dedent, format_with, group, hard_line_break, if_group_fits_on_line, indent,
    soft_block_indent, soft_line_break_or_space, space, token,
};
use destack_fir::{format_args, write};
use destack_source::Span;

/// The layout role of one type-conditional relative to its parent chain.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TypeConditionalLayout {
    /// A top-level conditional type.
    Root,
    /// A conditional type nested inside the test side of a parent conditional.
    NestedTest,
    /// A conditional type nested inside the true branch of a parent conditional.
    NestedConsequent,
    /// A conditional type nested inside the false branch of a parent conditional.
    NestedAlternate,
}

impl TypeConditionalLayout {
    /// Return whether this layout is the root of one conditional chain.
    fn is_root(self) -> bool {
        matches!(self, Self::Root)
    }

    /// Return whether this layout is nested inside the test branch.
    fn is_nested_test(self) -> bool {
        matches!(self, Self::NestedTest)
    }

    /// Return whether this layout is nested inside the alternate branch.
    fn is_nested_alternate(self) -> bool {
        matches!(self, Self::NestedAlternate)
    }
}

/// One conditional-like utility for type conditional expressions.
#[derive(Clone, Copy)]
struct TypeConditionalLike {
    /// The conditional node id.
    node_id: LocalNodeId<Expression>,
    /// The test left expression.
    left: LocalNodeId<Expression>,
    /// The test right expression.
    right: LocalNodeId<Expression>,
    /// The consequent expression.
    then_type: LocalNodeId<Expression>,
    /// The alternate expression.
    else_type: LocalNodeId<Expression>,
}

/// Return whether one expression begins with an inline block-prefix annotation.
fn expression_has_inline_block_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context
        .annotation_ids(expression_id)
        .iter()
        .copied()
        .any(|annotation_id| {
            context.annotation(annotation_id).position() == AnnotationPosition::BlockPrefix
                && !context.annotation_starts_on_own_line(annotation_id)
        })
}

/// Write trailing raw comments before one operator, following the OXC conditional flow.
fn write_trailing_raw_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    mut start: u32,
    end: u32,
    operator: u8,
) -> FormatResult<()> {
    let mut comments_before_operator = None;
    let mut selected_comments = Vec::<LocalNodeId<Comment>>::new();

    for comment_id in f.context().comment_nodes_in_range(start, end) {
        let comment_span = f.context().span(comment_id);

        if f.context()
            .has_newline(Span::new(comment_span.file, start, comment_span.start))
        {
            break;
        }

        if f.context().tree.get(comment_id).style == destack_ast::CommentStyle::Slash
            || f.context()
                .span_has_newline_before_next_non_whitespace_token(comment_span)
        {
            selected_comments.push(comment_id);
            break;
        }

        if f.context()
            .range_contains_byte(start, comment_span.start, operator)
        {
            comments_before_operator = Some(selected_comments.len());
        }

        selected_comments.push(comment_id);
        start = comment_span.end;
    }

    let selected_comments = if let Some(index) = comments_before_operator {
        &selected_comments[..index]
    } else {
        &selected_comments[..]
    };

    for comment_id in selected_comments {
        write!(f, [*comment_id])?;
    }

    Ok(())
}

impl TypeConditionalLike {
    /// Return the layout role of this conditional in its parent chain.
    fn layout(self, context: &DestackFormatContext<'_>) -> TypeConditionalLayout {
        let Some((parent_id, parent_type)) = context.parent(self.node_id) else {
            return TypeConditionalLayout::Root;
        };
        if parent_type != NodeType::Expression {
            return TypeConditionalLayout::Root;
        }

        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        let Expression::TypeConditional {
            left,
            right,
            then_type,
            else_type,
        } = context.tree.get(parent_expression_id)
        else {
            return TypeConditionalLayout::Root;
        };

        if left.id == self.node_id.id || right.id == self.node_id.id {
            return TypeConditionalLayout::NestedTest;
        }

        if then_type.id == self.node_id.id {
            return TypeConditionalLayout::NestedConsequent;
        }

        if else_type.id == self.node_id.id {
            return TypeConditionalLayout::NestedAlternate;
        }

        TypeConditionalLayout::Root
    }

    /// Format one conditional seam postfix annotation list with OXC-style trailing breaks.
    fn write_trailing_annotations<'ast>(
        self,
        f: &mut DestackFormatter<'ast, '_>,
        expression_id: LocalNodeId<Expression>,
        end: u32,
        operator: u8,
    ) -> FormatResult<()> {
        let mut items = Vec::new();
        for annotation_id in f.context().annotation_ids(expression_id).iter().copied() {
            if matches!(
                f.context().annotation(annotation_id).position(),
                AnnotationPosition::LinePostfix
                    | AnnotationPosition::LinePostfixBoundary
                    | AnnotationPosition::BlockPostfix
            ) {
                items.push(annotation_id);
            }
        }
        let expression_end = f.context().span(expression_id).end;

        write_annotation_sequence_without_trailing_break(f, &items)?;
        write_trailing_raw_comments(f, expression_end, end, operator)?;

        Ok(())
    }

    /// Format the test side of this conditional.
    fn format_test<'ast>(
        self,
        f: &mut DestackFormatter<'ast, '_>,
        layout: TypeConditionalLayout,
    ) -> FormatResult<()> {
        let format_inner = format_with(|f| {
            write!(f, [self.left, space(), Keyword::Extends, space()])?;
            write_expression_without_trailing_annotations(f, self.right)?;
            let alternate_start = f.context().span(self.then_type).start;
            self.write_trailing_annotations(f, self.right, alternate_start, b'?')
        });

        if layout.is_nested_alternate() {
            write!(f, [align(2, &format_inner)])
        } else {
            write!(f, [format_inner])
        }
    }

    /// Format the consequent and alternate sides of this conditional.
    fn format_consequent_and_alternate<'ast>(
        self,
        f: &mut DestackFormatter<'ast, '_>,
        layout: TypeConditionalLayout,
    ) -> FormatResult<()> {
        let consequent_has_inline_block_prefix =
            expression_has_inline_block_prefix_annotation(f.context(), self.then_type);
        let consequent_uses_nested_alternate_block_prefix_adapter =
            consequent_has_inline_block_prefix && layout.is_nested_alternate();
        let format_consequent_with_trailing_comments =
            format_with(|f: &mut DestackFormatter<'ast, '_>| {
                write_expression_without_trailing_annotations(f, self.then_type)?;
                let alternate_start = f.context().span(self.else_type).start;
                self.write_trailing_annotations(f, self.then_type, alternate_start, b':')
            });

        let format_consequent_with_indentation =
            format_with(|f: &mut DestackFormatter<'ast, '_>| {
                if consequent_uses_nested_alternate_block_prefix_adapter {
                    let mut prefix_items = Vec::new();
                    for annotation_id in f.context().annotation_ids(self.then_type).iter().copied()
                    {
                        if matches!(
                            f.context().annotation(annotation_id).position(),
                            AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
                        ) {
                            prefix_items.push(annotation_id);
                        }
                    }
                    let format_body = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        write_expression_without_trailing_annotations(f, self.then_type)?;
                        let alternate_start = f.context().span(self.else_type).start;
                        self.write_trailing_annotations(f, self.then_type, alternate_start, b':')
                    });
                    let format_prefix = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        write_annotation_sequence_without_trailing_break(f, &prefix_items)
                    });

                    write!(f, [dedent(&format_prefix)])?;
                    write!(f, [indent(&format_args![hard_line_break(), format_body])])?;
                } else if f.context().options.indent_style.is_space() {
                    write!(f, [align(2, &format_consequent_with_trailing_comments)])?;
                } else {
                    write!(f, [indent(&format_consequent_with_trailing_comments)])?;
                }

                Ok(())
            });

        let consequent_is_nested_conditional = matches!(
            f.context().tree.get(self.then_type),
            Expression::TypeConditional { .. }
        );
        let format_consequent = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            if consequent_is_nested_conditional {
                write!(
                    f,
                    [
                        if_group_fits_on_line(&token("(")),
                        format_consequent_with_indentation,
                        if_group_fits_on_line(&token(")"))
                    ]
                )?;
            } else {
                write!(f, [format_consequent_with_indentation])?;
            }

            Ok(())
        });

        let alternate_has_inline_block_prefix =
            expression_has_inline_block_prefix_annotation(f.context(), self.else_type);
        let format_alternate = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            write_expression_without_trailing_annotations(f, self.else_type)
        });
        let format_alternate = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            if f.context().options.indent_style.is_space() {
                write!(f, [align(2, &format_alternate)])?;
            } else {
                write!(f, [indent(&format_alternate)])?;
            }

            Ok(())
        });
        let consequent_operator_space = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            if !consequent_has_inline_block_prefix {
                write!(f, [space()])?;
            }

            Ok(())
        });
        let alternate_operator_space = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            if !alternate_has_inline_block_prefix {
                write!(f, [space()])?;
            }

            Ok(())
        });

        write!(
            f,
            [
                soft_line_break_or_space(),
                token("?"),
                consequent_operator_space,
                format_consequent,
                soft_line_break_or_space(),
                token(":"),
                alternate_operator_space,
                format_alternate
            ]
        )
    }

    /// Format this conditional using local layout and seam ownership.
    fn fmt<'ast>(self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        let layout = self.layout(f.context());

        let format_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            self.format_test(f, layout)?;

            let format_tail = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                let format_consequent_and_alternate =
                    format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        self.format_consequent_and_alternate(f, layout)
                    });

                match layout {
                    TypeConditionalLayout::Root | TypeConditionalLayout::NestedTest => {
                        write!(f, [indent(&format_consequent_and_alternate)])?;
                    }
                    TypeConditionalLayout::NestedConsequent => {
                        write!(f, [dedent(&indent(&format_consequent_and_alternate))])?;
                    }
                    TypeConditionalLayout::NestedAlternate => {
                        write!(f, [format_consequent_and_alternate])?;
                    }
                }

                Ok(())
            });

            write!(f, [format_tail])
        });

        let grouped = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            if layout.is_root() || layout.is_nested_test() {
                write!(f, [group(&format_inner)])?;
            } else {
                write!(f, [format_inner])?;
            }

            Ok(())
        });

        if layout.is_nested_test() {
            write!(f, [group(&soft_block_indent(&grouped))])
        } else {
            write!(f, [grouped])
        }
    }
}

/// Format one conditional type using an OXC-like conditional utility.
pub(crate) fn format_type_conditional_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    right: LocalNodeId<Expression>,
    then_type: LocalNodeId<Expression>,
    else_type: LocalNodeId<Expression>,
) -> FormatResult<()> {
    TypeConditionalLike {
        node_id,
        left,
        right,
        then_type,
        else_type,
    }
    .fmt(f)
}
