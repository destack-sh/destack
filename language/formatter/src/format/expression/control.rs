use super::declarator::format_declarator;
use super::dispatch::format_expression;
use super::write_expression_without_prefix_annotations;
use crate::format::chain::expression_trivia_anchor_end;
use crate::format::declaration::sequence::block_statement_sequence;
use crate::format::declaration::signature::expression_body_requires_head_space;
use crate::format::declaration::statement_wrapper_needs_semicolon;
use crate::format::directive::node_has_ignore_directive;
use crate::format::tree::tree_literal_should_break;
use crate::{
    Annotation, DestackFormatContext, DestackFormatter, FormatNode,
    empty_block_with_infix_annotations,
};
use destack_ast::{
    AnnotationPosition, Asynchrony, Block, BlockFormat, Doc, DocumentationStyle, Expression,
    ForEachBinding, ForEachDeclarationKind, ForEachKind, IfCondition, IfKind, Keyword, LetKind,
    LocalNodeId, MatchCase, MatchKind, MatchSelector, Mutability, NodeType, Pattern, TokenType,
    WhileKind, YieldCardinality,
};
use destack_core::StringId;
use destack_fir::format::{Buffer, FormatError, FormatResult};
use destack_fir::prelude::{
    block_indent, format_with, group, hard_line_break, line_postfix_boundary, space, token,
};
use destack_fir::{format_args, write};
use destack_source::Span;

/// Format one statement-body expression with statement-separator semantics.
fn format_statement_body_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let expression = f.context().tree.get(expression_id);
    let is_ignored = node_has_ignore_directive(f.context(), expression_id);

    write!(
        f,
        [crate::format::annotation::prefix_annotations(
            f.context(),
            expression_id
        )]
    )?;
    format_expression(f, expression_id, expression, is_ignored)?;

    if statement_wrapper_needs_semicolon(f.context(), expression_id) {
        write!(f, [token(";")])?;
    }

    let if_chain_handles_annotations = matches!(
        expression,
        Expression::If {
            kind: destack_ast::IfKind::If,
            ..
        }
    );
    if !if_chain_handles_annotations {
        write!(
            f,
            [crate::format::annotation::infix_or_postfix_annotations(
                f.context(),
                expression_id
            )]
        )?;
    }

    Ok(())
}

/// Format a statement body block, preserving wrapper semantics.
pub(crate) fn format_statement_body_block<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    block_id: LocalNodeId<Block>,
) -> FormatResult<()> {
    let block = f.context().tree.get(block_id);
    let is_statement_wrapper = is_statement_wrapper_block(f.context(), block_id);
    if !is_statement_wrapper {
        write!(f, [block_id])?;
        return Ok(());
    }

    write!(
        f,
        [crate::format::annotation::prefix_annotations(
            f.context(),
            block_id
        )]
    )?;

    if block.expressions.is_empty() {
        write!(f, [token(";")])?;
    } else if block.expressions.len() == 1 {
        let expression_id = block.expressions[0];
        if expression_has_block_prefix_annotation(f.context(), expression_id) {
            write!(
                f,
                [
                    hard_line_break(),
                    group(&block_indent(&format_with(|f| {
                        format_statement_body_expression(f, expression_id)
                    })))
                ]
            )?;
        } else {
            format_statement_body_expression(f, expression_id)?;
        }
    } else {
        write!(f, [block_id])?;
    }

    write!(
        f,
        [crate::format::annotation::infix_or_postfix_annotations(
            f.context(),
            block_id
        )]
    )?;
    Ok(())
}

/// Return true when this block originated from a statement wrapper instead of braces.
fn is_statement_wrapper_block<'ast>(
    context: &DestackFormatContext<'ast>,
    block_id: LocalNodeId<Block>,
) -> bool {
    let block = context.tree.get(block_id);
    block.format == BlockFormat::Implicit
}

/// Return true when this block is an empty statement wrapper.
pub(crate) fn is_empty_statement_block<'ast>(
    context: &DestackFormatContext<'ast>,
    block_id: LocalNodeId<Block>,
) -> bool {
    let block = context.tree.get(block_id);
    is_statement_wrapper_block(context, block_id) && block.expressions.is_empty()
}

/// Detect a source binding keyword for a for each pattern binding.
pub(crate) fn detect_for_each_binding_keyword<'ast>(
    context: &DestackFormatContext<'ast>,
    _for_each_id: LocalNodeId<Expression>,
    pattern_id: LocalNodeId<Pattern>,
) -> Option<Keyword> {
    let pattern_span = context.span(pattern_id);
    let keyword_token = context.previous_non_whitespace_token_before_span(pattern_span)?;
    if keyword_token.token.ty != TokenType::Identifier {
        return None;
    }

    if context
        .token_keyword(keyword_token)
        .is_some_and(|keyword| keyword == Keyword::Let)
    {
        Some(Keyword::Let)
    } else if context
        .token_keyword(keyword_token)
        .is_some_and(|keyword| keyword == Keyword::Const)
    {
        Some(Keyword::Const)
    } else if context
        .token_keyword(keyword_token)
        .is_some_and(|keyword| keyword == Keyword::Var)
    {
        Some(Keyword::Var)
    } else {
        None
    }
}

/// Format a for each binding pattern without repeating root mutability keywords.
pub(crate) fn format_for_each_binding_pattern<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    pattern_id: LocalNodeId<Pattern>,
) -> FormatResult<()> {
    match f.context().tree.get(pattern_id) {
        Pattern::Binding { name, pattern, .. } => {
            write!(f, [name])?;
            if let Some(pattern) = pattern {
                write!(f, [token(":"), space(), pattern])?;
            }
            Ok(())
        }
        _ => write!(f, [pattern_id]),
    }
}

/// Return whether an if branch should include a space after the condition head.
fn expression_has_block_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let annotations = context.annotation_ids(expression_id);
    if annotations.is_empty() {
        return false;
    }

    annotations.iter().copied().any(|annotation_id| {
        if context.annotation(annotation_id).position() != AnnotationPosition::BlockPrefix {
            return false;
        }

        let annotation_span = context.annotation_span(annotation_id);
        let is_multiline = annotation_span.start < annotation_span.end
            && !context
                .file
                .is_same_line(annotation_span.start, annotation_span.end.saturating_sub(1));

        is_multiline || context.annotation_starts_on_own_line(annotation_id)
    })
}

/// Return whether expression annotations include a line prefix annotation.
fn expression_has_line_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let annotations = context.annotation_ids(expression_id);
    if annotations.is_empty() {
        return false;
    }

    annotations.iter().copied().any(|annotation_id| {
        matches!(
            context.annotation(annotation_id),
            Annotation::Doc {
                position: AnnotationPosition::LinePrefix,
                ..
            } | Annotation::Decorator {
                position: AnnotationPosition::LinePrefix,
                ..
            }
        )
    })
}

/// Format one non-block statement body after a control-flow head.
fn format_statement_body_expression_after_head<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if expression_has_block_prefix_annotation(f.context(), expression_id) {
        write!(
            f,
            [
                hard_line_break(),
                group(&block_indent(&format_with(|f| {
                    format_statement_body_expression(f, expression_id)
                })))
            ]
        )?;
        return Ok(());
    }

    format_statement_body_expression(f, expression_id)
}

/// Return whether one annotation id forces adjacent argument wrapping.
fn annotation_is_adjacent_leading_comment(
    ctx: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let annotation = ctx.annotation(annotation_id);
    let annotation_span = ctx.annotation_span(annotation_id);

    let Annotation::Doc { node, position, .. } = annotation else {
        return false;
    };
    if !matches!(
        position,
        AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
    ) {
        return false;
    }

    let doc = ctx.tree.get::<Doc>(node);
    let is_multiline_block =
        doc.style == DocumentationStyle::Star && ctx.has_newline(annotation_span);
    if is_multiline_block {
        return true;
    }

    let Some(next_token) = ctx.annotation_next_non_whitespace_token(annotation_id) else {
        return false;
    };
    if annotation_span.file != next_token.span.file {
        return false;
    }

    !ctx.file
        .is_same_line(annotation_span.end.saturating_sub(1), next_token.span.start)
}

/// Return the next left-side expression used for adjacent statement comment checks.
fn next_adjacent_argument_left_side(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    match ctx.tree.get(expression_id) {
        Expression::SequenceExpression { expressions } => expressions.first().copied(),
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Index { left, .. }
        | Expression::Call { left, .. }
        | Expression::New { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. }
        | Expression::TypeBinary { left, .. }
        | Expression::Binary { left, .. }
        | Expression::TypeIndex { left, .. }
        | Expression::Assign { left, .. } => Some(*left),
        Expression::TaggedTemplateExpression { tag, .. } => Some(*tag),
        Expression::If {
            kind: IfKind::Ternary,
            condition: IfCondition::Expression { condition },
            ..
        } => Some(*condition),
        Expression::Statement(expression) => Some(*expression),
        Expression::Parenthesized { expression } => Some(*expression),
        _ => None,
    }
}

/// Return whether one adjacent statement argument has leading comments that require wrapping.
fn adjacent_statement_argument_has_leading_comments(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Expression>,
) -> bool {
    let argument_parent_is_yield =
        ctx.parent(argument_id)
            .is_some_and(|(parent_id, parent_type)| {
                parent_type == NodeType::Expression
                    && matches!(
                        ctx.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                        Expression::Yield { .. }
                    )
            });

    let mut current_id = argument_id;
    loop {
        let has_adjacent_leading_comment = ctx
            .annotation_ids(current_id)
            .iter()
            .copied()
            .any(|annotation_id| annotation_is_adjacent_leading_comment(ctx, annotation_id));

        let has_member_gap_comment = match ctx.tree.get(current_id) {
            Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => {
                match ctx.tree.get_main_span(current_id) {
                    Some(property_span) => {
                        let left_span = ctx.span(*left);
                        let left_anchor_end = expression_trivia_anchor_end(ctx, *left);
                        if left_span.file != property_span.file
                            || property_span.start <= left_anchor_end
                        {
                            false
                        } else {
                            let gap_span =
                                Span::new(left_span.file, left_anchor_end, property_span.start);
                            ctx.has_own_line_or_multiline_comment(gap_span)
                        }
                    }
                    None => false,
                }
            }
            _ => false,
        };

        if has_adjacent_leading_comment {
            let should_ignore_for_yield_chain_continuation =
                argument_parent_is_yield && has_member_gap_comment;
            if !should_ignore_for_yield_chain_continuation {
                return true;
            }
        }

        if !argument_parent_is_yield && has_member_gap_comment {
            return true;
        }

        let Some(next_id) = next_adjacent_argument_left_side(ctx, current_id) else {
            break;
        };
        current_id = next_id;
    }

    false
}

/// Format one adjacent return, throw, or yield argument.
pub(crate) fn format_adjacent_statement_argument<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let value_check_id = f.context().transparent_inner_expression(value_id);
    let value_expression = f.context().tree.get(value_check_id);
    let sequence_value_id = match value_expression {
        Expression::SequenceExpression { .. } => Some(value_check_id),
        Expression::Parenthesized { expression }
            if matches!(
                f.context().tree.get(*expression),
                Expression::SequenceExpression { .. }
            ) =>
        {
            Some(*expression)
        }
        _ => None,
    };
    let value_has_leading_comment =
        adjacent_statement_argument_has_leading_comments(f.context(), value_id);

    if let Some(sequence_value_id) = sequence_value_id
        && value_has_leading_comment
    {
        let prefix_annotation_owner_id = if f.context().has_prefix_annotation(value_id) {
            Some(value_id)
        } else if f.context().has_prefix_annotation(sequence_value_id) {
            Some(sequence_value_id)
        } else {
            None
        };
        let grouped_sequence = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            if let Some(prefix_annotation_owner_id) = prefix_annotation_owner_id {
                write!(
                    f,
                    [crate::format::annotation::prefix_annotations(
                        f.context(),
                        prefix_annotation_owner_id
                    )]
                )?;
            }

            write!(f, [token("(")])?;
            write_expression_without_prefix_annotations(f, sequence_value_id)?;
            write!(f, [token(")")])
        });
        write!(
            f,
            [
                space(),
                token("("),
                block_indent(&grouped_sequence),
                hard_line_break(),
                token(")")
            ]
        )?;
        return Ok(());
    }

    let value_is_parenthesized = matches!(value_expression, Expression::Parenthesized { .. });
    let value_is_unwrapped_sequence =
        matches!(value_expression, Expression::SequenceExpression { .. });
    let should_wrap_value =
        !value_is_parenthesized && (value_is_unwrapped_sequence || value_has_leading_comment);

    if should_wrap_value {
        write!(
            f,
            [
                space(),
                token("("),
                block_indent(&group(&value_check_id).should_expand(true)),
                hard_line_break(),
                token(")")
            ]
        )?;
        return Ok(());
    }

    write!(f, [space(), value_id])?;
    Ok(())
}

/// Return whether expression has any prefix annotation.
fn expression_has_effective_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let annotations = context.annotation_ids(expression_id);
    if annotations.is_empty() {
        return false;
    }

    annotations.iter().copied().any(|annotation_id| {
        matches!(
            context.annotation(annotation_id).position(),
            AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
        )
    })
}

/// Return whether an if branch should include a space after the condition head.
fn if_branch_head_requires_space(
    context: &DestackFormatContext<'_>,
    then_expression_id: LocalNodeId<Expression>,
) -> bool {
    let then_is_empty_statement = matches!(
        context.tree.get(then_expression_id),
        Expression::Block(block_id) if is_empty_statement_block(context, *block_id)
    );

    if expression_has_block_prefix_annotation(context, then_expression_id) {
        return false;
    }

    if expression_has_line_prefix_annotation(context, then_expression_id) {
        return false;
    }

    !then_is_empty_statement
}

/// Walk a chain of if expressions and collect the if/else if/else nodes.
pub(crate) fn format_if_else_chain<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    // walk the chain
    let mut next_if_id = node_id;
    loop {
        let if_node = f.context().tree.get(next_if_id);
        match if_node {
            // if or else if
            Expression::If {
                kind: _, // we turn everything into regular ifs
                condition,
                then_expression: then_expression_id,
                else_expression: else_expression_id,
            } => {
                let then_requires_head_space =
                    if_branch_head_requires_space(f.context(), *then_expression_id);

                // if <condition>
                match condition {
                    IfCondition::Expression { condition } => {
                        write!(
                            f,
                            [
                                Keyword::If,
                                space(),
                                token("("),
                                *condition,
                                line_postfix_boundary(),
                                token(")")
                            ]
                        )?;
                    }
                    IfCondition::Let {
                        kind,
                        mutability: _,
                        declarator,
                    } => {
                        write!(f, [Keyword::If, space()])?;
                        match kind {
                            LetKind::Let => write!(f, [Keyword::Let])?,
                            LetKind::Var => write!(f, [Keyword::Var])?,
                            LetKind::Const => write!(f, [Keyword::Const])?,
                        }
                        write!(f, [space()])?;
                        format_declarator(f, f.context().tree, *declarator)?;
                    }
                }

                // insert canonical spacing before the then expression
                if then_requires_head_space {
                    write!(f, [space()])?;
                }

                // then block
                let then_expression = f.context().tree.get(*then_expression_id);
                let then_is_empty_statement = matches!(
                    then_expression,
                    Expression::Block(block_id) if is_empty_statement_block(f.context(), *block_id)
                );
                match then_expression {
                    Expression::Block(block_id) => {
                        write!(
                            f,
                            [crate::format::annotation::prefix_annotations(
                                f.context(),
                                *then_expression_id
                            )]
                        )?;
                        format_statement_body_block(f, *block_id)?;
                        write!(
                            f,
                            [crate::format::annotation::infix_or_postfix_annotations(
                                f.context(),
                                *then_expression_id
                            )]
                        )?;
                    }
                    // something else
                    _ => format_statement_body_expression_after_head(f, *then_expression_id)?,
                }

                // next node
                if let Some(else_expression) = else_expression_id {
                    let else_has_effective_prefix_annotation =
                        expression_has_effective_prefix_annotation(f.context(), *else_expression);

                    // keep if-else seams tight: only non-blank postfix stays before else
                    if else_has_effective_prefix_annotation
                        || f.context().has_postfix_annotation(next_if_id)
                    {
                        write!(
                            f,
                            [crate::format::annotation::postfix_annotations(
                                f.context(),
                                next_if_id
                            )]
                        )?;
                    }

                    // keep compact spacing when else prefixes do not force layout
                    let if_has_postfix_annotation = f.context().has_postfix_annotation(next_if_id);
                    let then_has_postfix_annotation =
                        f.context().has_postfix_annotation(*then_expression_id);
                    if else_has_effective_prefix_annotation
                        || if_has_postfix_annotation
                        || then_has_postfix_annotation
                        || then_is_empty_statement
                    {
                        write!(f, [hard_line_break()])?;
                    } else {
                        write!(f, [space()])?;
                    }
                    match f.context().tree.get(*else_expression) {
                        // else if
                        Expression::If { .. } => {
                            write!(
                                f,
                                [crate::format::annotation::prefix_annotations(
                                    f.context(),
                                    *else_expression
                                )]
                            )?;
                            write!(f, [Keyword::Else, space()])?;
                            // (postfix is covered by the next if above)
                            next_if_id = *else_expression;
                        }
                        // else
                        Expression::Block(else_block_id) => {
                            write!(
                                f,
                                [crate::format::annotation::prefix_annotations(
                                    f.context(),
                                    *else_expression
                                )]
                            )?;
                            write!(f, [Keyword::Else, space()])?;
                            format_statement_body_block(f, *else_block_id)?;
                            if f.context().has_postfix_annotation(*else_expression) {
                                write!(
                                    f,
                                    [crate::format::annotation::postfix_annotations(
                                        f.context(),
                                        *else_expression
                                    )]
                                )?;
                            }
                            break;
                        }
                        // something else
                        _ => {
                            write!(f, [Keyword::Else])?;
                            if expression_has_block_prefix_annotation(f.context(), *else_expression)
                            {
                                format_statement_body_expression_after_head(f, *else_expression)?;
                            } else {
                                write!(f, [space()])?;
                                format_statement_body_expression_after_head(f, *else_expression)?;
                            }
                            break;
                        }
                    }
                } else {
                    // bare if
                    write!(
                        f,
                        [crate::format::annotation::postfix_annotations(
                            f.context(),
                            next_if_id
                        )]
                    )?;
                    break;
                }
            }
            // shouldn't be anything else
            _ => {
                return Err(FormatError::SyntaxError {
                    message: "unexpected expression kind for if chain",
                });
            }
        }
    }
    Ok(())
}

/// Return whether a match case has a boundary line comment.
fn match_case_has_boundary_line_comment(
    context: &DestackFormatContext<'_>,
    case_id: LocalNodeId<MatchCase>,
) -> bool {
    let case_span = context.span(case_id);

    context
        .comments_in_range(case_span.start, case_span.end)
        .iter()
        .copied()
        .any(|comment| context.comment_is_line(comment))
}

/// Return whether one match case has an inline block boundary comment.
fn match_case_has_inline_star_line_postfix_boundary_comment(
    context: &DestackFormatContext<'_>,
    case_id: LocalNodeId<MatchCase>,
) -> bool {
    let _ = (context, case_id);
    false
}

/// Format a match selector according to the selected case style.
fn format_selector_with_style(
    f: &mut DestackFormatter<'_, '_>,
    selector: &MatchSelector,
    is_switch_style: bool,
) -> FormatResult<()> {
    if !is_switch_style {
        match selector {
            MatchSelector::Pattern { pattern, guard } => {
                write!(f, [*pattern])?;
                if let Some(guard) = guard {
                    write!(
                        f,
                        [
                            space(),
                            Keyword::If,
                            space(),
                            token("("),
                            *guard,
                            token(")")
                        ]
                    )?;
                }
            }
            MatchSelector::Default => {
                write!(f, [token("_")])?;
            }
        }
    } else {
        match selector {
            MatchSelector::Pattern { pattern, guard } => {
                write!(f, [Keyword::Case, space(), *pattern, token(":")])?;
                if let Some(guard) = guard {
                    write!(
                        f,
                        [
                            space(),
                            Keyword::If,
                            space(),
                            token("("),
                            *guard,
                            token(")")
                        ]
                    )?;
                }
            }
            MatchSelector::Default => {
                write!(f, [Keyword::Default, token(":")])?;
            }
        }
    }

    Ok(())
}

/// Format one return expression in statement position.
pub(crate) fn format_return_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    value: Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    let tree = f.context().tree;
    let return_parent_is_block = f
        .context()
        .parent(node_id)
        .is_some_and(|(_, parent_type)| parent_type == NodeType::Block);

    // return keyword
    write!(f, [token("return")])?;

    // return value
    if let Some(value_id) = value {
        let value_expression = tree.get(value_id);

        // tree returns may need wrapping parens to keep multiline layout stable
        if let Expression::TreeExpression {
            arguments,
            elements,
            ..
        } = value_expression
        {
            let has_children = elements
                .as_ref()
                .is_some_and(|elements| !elements.is_empty());
            let has_multiple_attributes = arguments
                .as_ref()
                .is_some_and(|arguments| arguments.len() > 1);
            let should_wrap_tree_return = has_children
                || has_multiple_attributes
                || tree_literal_should_break(f.context(), arguments, elements);

            if should_wrap_tree_return {
                write!(
                    f,
                    [
                        space(),
                        token("("),
                        block_indent(&value_id),
                        hard_line_break(),
                        token(")")
                    ]
                )?;
            } else {
                format_adjacent_statement_argument(f, value_id)?;
            }
        } else {
            format_adjacent_statement_argument(f, value_id)?;
        }
    }

    // trailing semicolon
    if return_parent_is_block {
        write!(f, [token(";")])?;
    }

    Ok(())
}

/// Format one yield expression in statement position.
pub(crate) fn format_yield_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    cardinality: YieldCardinality,
    value: Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    // yield keyword
    write!(f, [Keyword::Yield])?;
    if cardinality == YieldCardinality::Generator {
        write!(f, [token("*")])?;
    }

    // yield value
    if let Some(value_id) = value {
        format_adjacent_statement_argument(f, value_id)?;
    }

    // block statement yields terminate like return or throw
    let yield_parent_is_block = f
        .context()
        .parent(node_id)
        .is_some_and(|(_, parent_type)| parent_type == NodeType::Block);
    if yield_parent_is_block {
        write!(f, [token(";")])?;
    }

    Ok(())
}

/// Format one throw expression in statement position.
pub(crate) fn format_throw_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    write!(f, [token("throw")])?;
    format_adjacent_statement_argument(f, value_id)
}

/// Format one break expression in statement position.
pub(crate) fn format_break_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    label: &Option<StringId>,
    value: &Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    write!(f, [Keyword::Break])?;

    if let Some(label) = label {
        if f.context().options.language_type.is_destack() {
            write!(f, [space(), token(":"), label])?;
        } else {
            write!(f, [space(), label])?;
        }
    }

    if let Some(value) = value {
        write!(f, [space(), value])?;
    }

    Ok(())
}

/// Format one continue expression in statement position.
pub(crate) fn format_continue_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    label: &Option<StringId>,
) -> FormatResult<()> {
    write!(f, [Keyword::Continue])?;

    if let Some(label) = label {
        if f.context().options.language_type.is_destack() {
            write!(f, [space(), token(":"), label])?;
        } else {
            write!(f, [space(), label])?;
        }
    }

    Ok(())
}

/// Return whether a control-flow statement body should be preceded by a space.
fn statement_body_requires_head_space(
    context: &DestackFormatContext<'_>,
    body: LocalNodeId<Block>,
) -> bool {
    expression_body_requires_head_space(context, LocalNodeId::<Expression>::new(body.id))
}

/// Format a `while` or `do while` expression.
pub(crate) fn format_while_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    kind: WhileKind,
    condition: LocalNodeId<Expression>,
    body: LocalNodeId<Block>,
) -> FormatResult<()> {
    match kind {
        // while (<condition>) <body>
        WhileKind::While => {
            write!(
                f,
                [
                    Keyword::While,
                    space(),
                    token("("),
                    condition,
                    line_postfix_boundary(),
                    token(")")
                ]
            )?;
            if statement_body_requires_head_space(f.context(), body) {
                write!(f, [space()])?;
            }
            format_statement_body_block(f, body)?;
        }
        // do <body> while (<condition>)
        WhileKind::DoWhile => {
            let is_block_body = f.context().tree.get(body).format == BlockFormat::Explicit;

            write!(f, [Keyword::Do])?;
            if statement_body_requires_head_space(f.context(), body) {
                write!(f, [space()])?;
            }
            format_statement_body_block(f, body)?;

            if is_block_body {
                write!(f, [space()])?;
            } else {
                write!(f, [hard_line_break()])?;
            }

            write!(
                f,
                [
                    Keyword::While,
                    space(),
                    token("("),
                    condition,
                    line_postfix_boundary(),
                    token(")"),
                ]
            )?;
        }
    }

    Ok(())
}

/// Format a `for each` expression.
pub(crate) fn format_for_each_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    asynchrony: Asynchrony,
    kind: ForEachKind,
    binding: &ForEachBinding,
    iterator: LocalNodeId<Expression>,
    body: LocalNodeId<Block>,
) -> FormatResult<()> {
    let tree = f.context().tree;

    // for header
    write!(f, [Keyword::For, space()])?;
    if asynchrony == Asynchrony::Async {
        write!(f, [Keyword::Await, space()])?;
    }
    let keyword = match kind {
        ForEachKind::In => Keyword::In,
        ForEachKind::Of => Keyword::Of,
    };

    // binding
    write!(f, [token("(")])?;
    match binding {
        ForEachBinding::Pattern {
            pattern,
            declaration_kind,
        } => {
            // explicit declaration kind
            if let Some(declaration_kind) = declaration_kind {
                let keyword = match declaration_kind {
                    ForEachDeclarationKind::Var => Keyword::Var,
                    ForEachDeclarationKind::Let => Keyword::Let,
                    ForEachDeclarationKind::Const => Keyword::Const,
                };
                write!(f, [keyword, space()])?;
                format_for_each_binding_pattern(f, *pattern)?;
            }
            // source keyword recovery
            else {
                let source_keyword =
                    detect_for_each_binding_keyword(f.context(), node_id, *pattern);
                if let Some(keyword) = source_keyword {
                    write!(f, [keyword, space()])?;
                    format_for_each_binding_pattern(f, *pattern)?;
                } else {
                    let pattern_node = tree.get(*pattern);
                    let should_prefix_const = matches!(
                        pattern_node,
                        Pattern::Binding {
                            mutability: Some(Mutability::Immutable),
                            pattern: None,
                            ..
                        }
                    );

                    // keep explicit const for simple immutable bindings
                    if should_prefix_const {
                        write!(f, [Keyword::Const, space()])?;
                    }
                    write!(f, [pattern])?;
                }
            }
        }
        ForEachBinding::Using {
            asynchrony,
            pattern,
        } => {
            if *asynchrony == Asynchrony::Async {
                write!(f, [Keyword::Await, space()])?;
            }
            write!(f, [Keyword::Using, space(), pattern])?;
        }
    }

    // iterator + body
    write!(
        f,
        [
            space(),
            keyword,
            space(),
            iterator,
            line_postfix_boundary(),
            token(")")
        ]
    )?;
    if statement_body_requires_head_space(f.context(), body) {
        write!(f, [space()])?;
    }
    format_statement_body_block(f, body)?;

    Ok(())
}

/// Format a classic `for` expression.
pub(crate) fn format_for_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    initialization: Option<LocalNodeId<Expression>>,
    condition: Option<LocalNodeId<Expression>>,
    increment: Option<LocalNodeId<Expression>>,
    body: LocalNodeId<Block>,
) -> FormatResult<()> {
    write!(
        f,
        [
            Keyword::For,
            space(),
            token("("),
            initialization,
            token(";"),
            space(),
            condition,
            token(";"),
            space(),
            increment,
            line_postfix_boundary(),
            token(")")
        ]
    )?;
    if statement_body_requires_head_space(f.context(), body) {
        write!(f, [space()])?;
    }
    format_statement_body_block(f, body)
}

/// Format a `loop` expression.
pub(crate) fn format_loop_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    body: LocalNodeId<Block>,
) -> FormatResult<()> {
    write!(f, [Keyword::Loop])?;
    if statement_body_requires_head_space(f.context(), body) {
        write!(f, [space()])?;
    }
    format_statement_body_block(f, body)
}

/// Format a `try` expression.
pub(crate) fn format_try_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    try_expression: LocalNodeId<Expression>,
    catch_pattern: Option<LocalNodeId<Pattern>>,
    catch_ty: Option<LocalNodeId<Expression>>,
    catch_expression: Option<LocalNodeId<Expression>>,
    finally_expression: Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    // try block
    write!(f, [Keyword::Try])?;
    if let Expression::Block(block_id) = f.context().tree.get(try_expression) {
        if statement_body_requires_head_space(f.context(), *block_id) {
            write!(f, [space()])?;
        }
    } else {
        write!(f, [space()])?;
    }
    write!(f, [try_expression])?;

    // catch block
    if let Some(catch_expression) = catch_expression {
        write!(f, [space(), Keyword::Catch])?;
        if let Some(catch_pattern) = catch_pattern {
            write!(f, [space()])?;
            write!(f, [token("("), catch_pattern])?;
            if let Some(catch_ty) = catch_ty {
                write!(f, [token(":"), space(), catch_ty])?;
            }
            write!(f, [token(")")])?;
        }
        if let Expression::Block(block_id) = f.context().tree.get(catch_expression) {
            if statement_body_requires_head_space(f.context(), *block_id) {
                write!(f, [space()])?;
            }
        } else {
            write!(f, [space()])?;
        }
        write!(f, [catch_expression])?;
    }

    // finally block
    if let Some(finally_expression) = finally_expression {
        write!(f, [space(), Keyword::Finally])?;
        if let Expression::Block(block_id) = f.context().tree.get(finally_expression) {
            if statement_body_requires_head_space(f.context(), *block_id) {
                write!(f, [space()])?;
            }
        } else {
            write!(f, [space()])?;
        }
        write!(f, [finally_expression])?;
    }

    Ok(())
}

/// Format one match case with the selected style.
pub(crate) fn format_match_case_with_style<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    case_id: LocalNodeId<MatchCase>,
    is_switch_style: bool,
) -> FormatResult<()> {
    let case = f.context().tree.get(case_id);
    let has_boundary_line_comment = match_case_has_boundary_line_comment(f.context(), case_id);
    let has_inline_star_line_postfix_boundary_comment =
        match_case_has_inline_star_line_postfix_boundary_comment(f.context(), case_id);

    // case prefix
    write!(
        f,
        [crate::format::annotation::prefix_annotations(
            f.context(),
            case_id
        )]
    )?;

    // selector, separator, and body
    match case {
        MatchCase::Expression { selector, body } => {
            format_selector_with_style(f, selector, is_switch_style)?;
            if has_boundary_line_comment {
                write!(
                    f,
                    [
                        crate::format::annotation::line_postfix_boundary_annotations(
                            f.context(),
                            case_id
                        )
                    ]
                )?;
            }
            if !is_switch_style {
                write!(f, [space(), token("=>"), space(), *body])?;
            } else if let Some(explicit_block_expression) =
                switch_case_expression_body_collapsed_explicit_block_expression(f.context(), *body)
            {
                if has_boundary_line_comment {
                    if has_inline_star_line_postfix_boundary_comment {
                        write!(f, [space(), explicit_block_expression])?;
                    } else {
                        write!(f, [explicit_block_expression])?;
                    }
                } else {
                    write!(f, [space(), explicit_block_expression])?;
                }
            } else if switch_case_expression_body_should_break(f, *body) {
                if has_boundary_line_comment {
                    write!(f, [block_indent(body)])?;
                } else {
                    write!(f, [hard_line_break(), block_indent(body)])?;
                }
            } else if has_boundary_line_comment {
                if has_inline_star_line_postfix_boundary_comment {
                    write!(f, [space(), *body])?;
                } else {
                    write!(f, [*body])?;
                }
            } else {
                write!(f, [space(), *body])?;
            }
        }
        MatchCase::Block { selector, body } => {
            format_selector_with_style(f, selector, is_switch_style)?;
            if has_boundary_line_comment {
                write!(
                    f,
                    [
                        crate::format::annotation::line_postfix_boundary_annotations(
                            f.context(),
                            case_id
                        )
                    ]
                )?;
            }
            if !is_switch_style {
                write!(f, [space(), token("=>"), space(), *body])?;
            } else {
                let block = f.context().tree.get(*body);
                if block.format == BlockFormat::Implicit {
                    if let Some(explicit_block_expression) =
                        switch_case_implicit_body_single_explicit_block_expression(
                            f.context(),
                            *body,
                        )
                    {
                        if has_boundary_line_comment {
                            if has_inline_star_line_postfix_boundary_comment {
                                write!(f, [space(), explicit_block_expression])?;
                            } else {
                                write!(f, [explicit_block_expression])?;
                            }
                        } else {
                            write!(f, [space(), explicit_block_expression])?;
                        }
                    } else if !block.expressions.is_empty() {
                        if !has_boundary_line_comment {
                            write!(f, [hard_line_break()])?;
                        }
                        write!(
                            f,
                            [block_indent(&block_statement_sequence(
                                &block.expressions,
                                false,
                            ))]
                        )?;
                    }
                } else if has_boundary_line_comment {
                    if has_inline_star_line_postfix_boundary_comment {
                        write!(f, [space(), *body])?;
                    } else {
                        write!(f, [*body])?;
                    }
                } else {
                    write!(f, [space(), *body])?;
                }
            }
        }
    }

    // case postfix
    write!(
        f,
        [
            crate::format::annotation::infix_or_postfix_annotations_without_line_postfix_boundary(
                f.context(),
                case_id
            )
        ]
    )?;

    Ok(())
}

/// Return whether a switch case expression body should render on its own line.
fn switch_case_expression_body_should_break(
    f: &DestackFormatter<'_, '_>,
    body_expression_id: LocalNodeId<Expression>,
) -> bool {
    f.context().has_annotation(body_expression_id)
        || f.context().node_has_newline(body_expression_id)
}

/// Return one expression id with statement wrappers removed.
fn switch_case_expression_without_statement_wrapper(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let mut expression_id = expression_id;

    loop {
        let Expression::Statement(inner_expression_id) = context.tree.get(expression_id) else {
            return expression_id;
        };
        expression_id = *inner_expression_id;
    }
}

/// Return whether one switch case expression body is one explicit block expression.
fn switch_case_expression_body_is_explicit_block(
    context: &DestackFormatContext<'_>,
    body_expression_id: LocalNodeId<Expression>,
) -> bool {
    let body_expression_id =
        switch_case_expression_without_statement_wrapper(context, body_expression_id);

    let Expression::Block(block_id) = context.tree.get(body_expression_id) else {
        return false;
    };

    let block = context.tree.get(*block_id);
    block.format == BlockFormat::Explicit
}

/// Return whether one expression is an empty statement block wrapper.
fn expression_is_empty_statement_block(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = switch_case_expression_without_statement_wrapper(context, expression_id);

    let Expression::Block(block_id) = context.tree.get(expression_id) else {
        return false;
    };

    let block = context.tree.get(*block_id);
    block.format == BlockFormat::Implicit && block.expressions.is_empty()
}

/// Return one explicit block expression for one switch case body when collapsible.
fn switch_case_expression_body_collapsed_explicit_block_expression(
    context: &DestackFormatContext<'_>,
    body_expression_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    let body_expression_id =
        switch_case_expression_without_statement_wrapper(context, body_expression_id);

    if switch_case_expression_body_is_explicit_block(context, body_expression_id) {
        return Some(body_expression_id);
    }

    let Expression::Block(block_id) = context.tree.get(body_expression_id) else {
        return None;
    };

    switch_case_implicit_body_single_explicit_block_expression(context, *block_id)
}

/// Return one explicit block expression when one implicit switch body only wraps that block.
fn switch_case_implicit_body_single_explicit_block_expression(
    context: &DestackFormatContext<'_>,
    implicit_block_id: LocalNodeId<Block>,
) -> Option<LocalNodeId<Expression>> {
    let implicit_block = context.tree.get(implicit_block_id);
    if implicit_block.format != BlockFormat::Implicit {
        return None;
    }

    let mut explicit_block_expression = None;
    for expression_id in implicit_block.expressions.iter().copied() {
        if expression_is_empty_statement_block(context, expression_id) {
            continue;
        }

        if explicit_block_expression.is_some() {
            return None;
        }

        if !switch_case_expression_body_is_explicit_block(context, expression_id) {
            return None;
        }

        explicit_block_expression = Some(expression_id);
    }

    explicit_block_expression
}

impl<'ast> FormatNode<'ast, MatchCase> for MatchCase {
    fn format_node(
        &self,
        node_id: LocalNodeId<MatchCase>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        format_match_case_with_style(f, node_id, false)
    }
}

/// Format a match expression.
pub(crate) fn format_match<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    include_prefix: bool,
) -> FormatResult<()> {
    let match_node = f.context().tree.get(node_id);
    let Expression::Match { kind, value, cases } = &match_node else {
        return Err(FormatError::SyntaxError {
            message: "invalid match expression",
        });
    };
    let kind = *kind;
    let is_switch_style = matches!(kind, MatchKind::Switch);

    if include_prefix {
        // match/switch <expression>
        let keyword = match kind {
            MatchKind::Match => Keyword::Match,
            MatchKind::Switch => Keyword::Switch,
        };
        write!(f, [keyword, space()])?;
    }

    write!(f, [token("("), value, token(")")])?;

    // empty match body
    if cases.is_empty() {
        write!(f, [space(), empty_block_with_infix_annotations(node_id)])?;
        write!(
            f,
            [crate::format::annotation::postfix_annotations(
                f.context(),
                node_id
            )]
        )?;
        return Ok(());
    }

    // match/switch cases
    write!(f, [space(), token("{"), hard_line_break()])?;
    write!(
        f,
        [group(&format_args![block_indent(&format_with(|f| {
            let mut first = true;
            for case_id in cases {
                if !first {
                    write!(f, [hard_line_break()])?;
                }
                first = false;
                format_match_case_with_style(f, *case_id, is_switch_style)?;
            }
            Ok(())
        })),])]
    )?;
    write!(
        f,
        [crate::format::annotation::block_infix_annotations(
            f.context(),
            node_id
        )]
    )?;
    write!(f, [hard_line_break(), token("}")])?;

    Ok(())
}
