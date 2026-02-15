use std::borrow::Cow;

use destack_ast::{
    Annotation, AnnotationPosition, Block, Declaration, Expression, FunctionKind, LocalNodeId,
    Node, NodeTree, NodeTreeImpl, NodeType, ScalarLiteral,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_source::{FileId, Span};

use super::imports;
use super::timing::tags;
use crate::directive::{
    FormatterDirective, FormatterDirectiveKind, FormatterDirectivePosition,
    collect_ignore_ranges_for_nodes, directive_for_node, has_file_ignore_directive,
    ignored_span_source, write_ignored_span,
};
use crate::expression::format_expression;
use crate::{DestackFormatContext, DestackFormatter, FormatNode};

/// Check whether an expression is a directive prologue string literal.
fn is_directive_expression(tree: &NodeTree, expression_id: LocalNodeId<Expression>) -> bool {
    match tree.get(expression_id) {
        Expression::Statement(inner_id) => is_directive_expression(tree, *inner_id),
        Expression::Parenthesized { expression } => is_directive_expression(tree, *expression),
        Expression::ScalarLiteral(ScalarLiteral::String(_)) => true,
        _ => false,
    }
}

/// A formatted list of expression statements.
#[derive(Debug, Clone, Copy)]
pub struct StatementList<'a> {
    expressions: &'a [LocalNodeId<Expression>],
}

impl<'ast, 'a> Format<DestackFormatContext<'ast>> for StatementList<'a> {
    fn format(&self, f: &mut Formatter<'_, DestackFormatContext<'ast>>) -> FormatResult<()> {
        // respect file-level ignore directives for top-level formatting
        if f.context().options.respect_file_ignore
            && statement_list_is_file_root(f.context(), self.expressions)
            && has_file_ignore_directive(f.context())
        {
            let file_line_count = f.context().file.text().lines().count();
            f.context().mark_file_ignore_applied();
            f.context()
                .increment_counter("profile.file_ignore.files", 1);
            f.context()
                .increment_counter("profile.file_ignore.lines", file_line_count);
            let full_file_span = Span::new(f.context().file.id, 0, f.context().file.len);
            write_ignored_span(f, full_file_span)?;
            return Ok(());
        }

        let _timing = f.context().timing_scope(tags::FORMAT_STATEMENT_LIST);
        format_block_of_statements(f, self.expressions)?;
        if !self.expressions.is_empty() {
            write!(f, [hard_line_break()])?;
        }
        Ok(())
    }
}

/// Create a formatter for a list of expression statements.
pub fn statement_list(expressions: &[LocalNodeId<Expression>]) -> StatementList<'_> {
    StatementList { expressions }
}

/// Return whether this statement list is the file root expression list.
fn statement_list_is_file_root(
    context: &DestackFormatContext<'_>,
    expressions: &[LocalNodeId<Expression>],
) -> bool {
    expressions
        .first()
        .is_some_and(|expression_id| context.get_parent(*expression_id).is_none())
}

/// Empty block with infix annotations.
#[derive(Debug, Clone, PartialEq)]
pub struct EmptyBlockWithInfixAnnotations<T: Node> {
    node_id: LocalNodeId<T>,
}

impl<'ast, T> Format<DestackFormatContext<'ast>> for EmptyBlockWithInfixAnnotations<T>
where
    T: Node + Clone,
    NodeTree: NodeTreeImpl<T>,
{
    #[inline]
    fn format(&self, f: &mut Formatter<'_, DestackFormatContext<'ast>>) -> FormatResult<()> {
        // keep empty blocks compact unless they carry infix annotations
        if !f.context().has_infix_annotation(self.node_id) {
            return write!(f, [token("{"), token("}")]);
        }

        write!(
            f,
            [group(&format_args![
                token("{"),
                soft_block_indent(&format_args![
                    if_group_fits_on_line(&token("")),
                    &f.context().block_infix_annotations(self.node_id)
                ]),
                token("}")
            ])]
        )
    }
}

/// Format an empty block with infix annotations.
///
/// Example.
/// ```
/// {
///     // infix comment
/// }
/// ```
pub fn empty_block_with_infix_annotations<T: Node>(
    node_id: LocalNodeId<T>,
) -> EmptyBlockWithInfixAnnotations<T> {
    EmptyBlockWithInfixAnnotations { node_id }
}

/// Return whether source text between two expressions contains an explicit blank line.
fn expressions_have_blank_line_between(
    context: &DestackFormatContext<'_>,
    left_expression_id: LocalNodeId<Expression>,
    right_expression_id: LocalNodeId<Expression>,
) -> bool {
    let left_span = context.get_span(left_expression_id);
    let right_span = context.get_span(right_expression_id);
    if left_span.file != right_span.file || left_span.end >= right_span.start {
        return false;
    }

    let between_span = Span::new(left_span.file, left_span.end, right_span.start);
    if !context.has_newline(between_span) {
        return false;
    }

    let between = context.get_span_str(between_span);
    source_has_blank_line_in_trivia(between)
}

/// Return whether trivia between two offsets contains an explicit blank line.
fn source_has_blank_line_between_offsets(
    context: &DestackFormatContext<'_>,
    file: FileId,
    start: u32,
    end: u32,
) -> bool {
    if end <= start {
        return false;
    }

    let between_span = Span::new(file, start, end);
    if !context.has_newline(between_span) {
        return false;
    }

    let between = context.get_span_str(between_span);
    source_has_blank_line_in_trivia(between)
}

/// Return whether trivia text contains an interior blank line.
fn source_has_blank_line_in_trivia(source: &str) -> bool {
    let mut has_first_newline = false;
    let mut line_has_content = false;

    for byte in source.bytes() {
        match byte {
            b'\r' => {}
            b'\n' => {
                if has_first_newline && !line_has_content {
                    return true;
                }
                has_first_newline = true;
                line_has_content = false;
            }
            b' ' | b'\t' => {}
            _ => {
                if has_first_newline {
                    line_has_content = true;
                }
            }
        }
    }

    false
}

/// Return the earliest start offset for prefix comment annotations on an expression.
fn expression_prefix_start(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    fallback: u32,
) -> u32 {
    let Some(annotation_ids) = context.get_annotations(expression_id) else {
        return fallback;
    };

    let mut start = fallback;
    for annotation_id in annotation_ids {
        let Annotation::Comment { node, position } = context.tree.get::<Annotation>(annotation_id)
        else {
            continue;
        };
        if !matches!(
            position,
            AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
        ) {
            continue;
        }

        let comment_span = context.get_span(*node);
        start = start.min(comment_span.start);
    }

    start
}

/// Format a block inline with zero or one expression (including label and infix annotations).
/// Format block contents with compact inner spacing.
#[inline]
pub(crate) fn format_block_body_narrow<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    block_id: LocalNodeId<Block>,
) -> FormatResult<()> {
    let block = f.context().tree.get(block_id);
    debug_assert!(block.expressions.len() <= 1);

    // body
    if block.expressions.is_empty() {
        write!(f, [token("{"), token("}")])?;
    } else {
        write!(
            f,
            [
                token("{"),
                soft_line_break_or_space(),
                soft_block_indent(&format_args![
                    &block.expressions[0],
                    f.context().block_infix_annotations(block_id)
                ]),
                soft_line_break_or_space(),
                token("}")
            ]
        )?;
    }
    Ok(())
}

/// Format a block multiline with multiple expressions (including label and infix annotations).
/// Format block contents with expanded inner spacing.
#[inline]
pub(crate) fn format_block_body_wide<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    block_id: LocalNodeId<Block>,
) -> FormatResult<()> {
    let block = f.context().tree.get(block_id);
    // body
    write!(
        f,
        [
            token("{"),
            hard_line_break(),
            soft_block_indent(&format_with(|f| format_block_of_statements(
                f,
                &block.expressions
            ))),
            hard_line_break(),
            block_indent(&f.context().block_infix_annotations(block_id)),
            token("}"),
        ]
    )
}

/// Format a block statement body with statement-level spacing and ignore handling.
pub(crate) fn format_block_of_statements<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expressions: &[LocalNodeId<Expression>],
) -> FormatResult<()> {
    let _timing = f.context().timing_scope(tags::FORMAT_BLOCK_STATEMENTS);
    let organize = f.context().options.organize_imports.is_enabled();
    let tree = f.context().tree;
    let strings = f.context().strings;
    // ignore ranges: only compute when the file may contain ignore directives
    let ignore_ranges = if f.context().has_ignore_directive_markers() {
        let comment_tokens = f.context().comment_tokens();
        collect_ignore_ranges_for_nodes(f.context(), expressions, comment_tokens)
    } else {
        std::collections::HashMap::new()
    };
    let has_ignore_ranges = !ignore_ranges.is_empty();

    // find contiguous import section at the start
    let import_count = expressions
        .iter()
        .take_while(|&&expr_id| imports::is_import(expr_id, tree))
        .count();

    // prepare the expression list (potentially with sorted imports)
    let sorted_imports: Vec<LocalNodeId<Expression>>;
    let effective_expressions: Cow<'_, [LocalNodeId<Expression>]> =
        if organize && import_count > 1 && !has_ignore_ranges {
            sorted_imports = imports::sort_imports(&expressions[..import_count], tree, strings);
            Cow::Owned(
                sorted_imports
                    .iter()
                    .copied()
                    .chain(expressions[import_count..].iter().copied())
                    .collect(),
            )
        } else {
            Cow::Borrowed(expressions)
        };

    let directive_count = effective_expressions
        .iter()
        .take_while(|&&expr_id| is_directive_expression(tree, expr_id))
        .count();
    let insert_blank_after_directive_prologue = if directive_count == 0 {
        false
    } else {
        let last_directive_expression = effective_expressions[directive_count - 1];
        !f.context().has_prefix_annotation(last_directive_expression)
            && !f
                .context()
                .has_postfix_annotation(last_directive_expression)
    };

    let mut prev_was_import = false;
    let mut prev_import_id: Option<LocalNodeId<Expression>> = None;
    let mut previous_output_end: Option<(FileId, u32)> = None;
    let mut skip_until: Option<u32> = None;

    for (i, &expression_id) in effective_expressions.iter().enumerate() {
        let expression = f.context().tree.get(expression_id);
        let is_import_expr = imports::is_import(expression_id, tree);
        let ignore_range = ignore_ranges.get(&expression_id.id).copied();
        let has_ignore_range = ignore_range.is_some();

        let expression_span = f.context().get_span(expression_id);

        if let Some(skip_end) = skip_until {
            if expression_span.start < skip_end {
                continue;
            }
            skip_until = None;
        }

        // blank line between expressions
        if i > 0 {
            let previous_expression_id = effective_expressions[i - 1];
            let has_blank_prefix_annotation =
                f.context().has_blank_prefix_annotation(expression_id);
            let has_prefix_annotation = f.context().has_prefix_annotation(expression_id);
            let previous_has_postfix_annotation =
                f.context().has_postfix_annotation(previous_expression_id);
            let source_has_blank_line_between = if has_ignore_range {
                if let Some((previous_file, previous_end)) = previous_output_end {
                    if previous_file != expression_span.file {
                        false
                    } else {
                        let range_start =
                            ignore_range.map_or(expression_span.start, |span| span.start);
                        source_has_blank_line_between_offsets(
                            f.context(),
                            expression_span.file,
                            previous_end,
                            range_start,
                        )
                    }
                } else {
                    let previous_span = f.context().get_span(previous_expression_id);
                    if previous_span.file != expression_span.file {
                        false
                    } else {
                        let range_start =
                            ignore_range.map_or(expression_span.start, |span| span.start);
                        source_has_blank_line_between_offsets(
                            f.context(),
                            expression_span.file,
                            previous_span.end,
                            range_start,
                        )
                    }
                }
            } else {
                expressions_have_blank_line_between(
                    f.context(),
                    previous_expression_id,
                    expression_id,
                )
            };
            let uses_source_blank_line_without_leading_break = source_has_blank_line_between
                && !has_blank_prefix_annotation
                && !has_prefix_annotation
                && !previous_has_postfix_annotation
                && !has_ignore_range;
            if (!has_blank_prefix_annotation || has_ignore_range)
                && !uses_source_blank_line_without_leading_break
            {
                write!(f, [hard_line_break()])?;
            }

            // determine if we need an extra blank line
            let needs_blank = if directive_count > 0 && i == directive_count {
                insert_blank_after_directive_prologue && !has_blank_prefix_annotation
            } else if has_ignore_range && has_blank_prefix_annotation {
                true
            } else if organize && prev_was_import && is_import_expr {
                // check if different import groups
                prev_import_id.is_some_and(|prev_id| {
                    imports::should_insert_blank_between(
                        prev_id,
                        expression_id,
                        f.context().tree,
                        f.context().strings,
                    )
                })
            } else if prev_was_import && !is_import_expr {
                // blank line after import section (if not already present)
                !has_blank_prefix_annotation || has_ignore_range
            } else if source_has_blank_line_between {
                (!has_blank_prefix_annotation
                    && !has_prefix_annotation
                    && !previous_has_postfix_annotation)
                    || has_ignore_range
            } else {
                false
            };

            if needs_blank {
                write!(f, [empty_line()])?;
            }
        }

        if let Some(range_span) = ignore_range {
            let prefix_start =
                expression_prefix_start(f.context(), expression_id, expression_span.start);
            if prefix_start < range_span.start {
                let prefix_span = Span::new(expression_span.file, prefix_start, range_span.start);
                // preserve the final source newline between prefix trivia and ignored range
                let prefix_source = ignored_span_source(f.context(), prefix_span);
                let prefix_has_trailing_newline = prefix_source.ends_with('\n');
                write_ignored_span(f, prefix_span)?;
                if prefix_has_trailing_newline {
                    write!(f, [hard_line_break()])?;
                }
            }

            write_ignored_span(f, range_span)?;
            skip_until = Some(range_span.end);
            prev_was_import = false;
            prev_import_id = None;
            previous_output_end = Some((range_span.file, range_span.end));
            continue;
        }

        let directive = directive_for_node(f.context(), expression_id);

        // expression itself (with prefix annotations)
        // lambda declaration line prefix comments are handled in declaration formatting
        let is_lambda_declaration_expression = matches!(
            expression,
            Expression::Declaration(declaration_id)
                if matches!(
                    tree.get(*declaration_id),
                    Declaration::Function { signature, .. }
                        if signature.kind == FunctionKind::Lambda
                )
        );
        if is_lambda_declaration_expression {
            write!(f, [f.context().block_prefix_annotations(expression_id)])?;
        } else {
            write!(f, [f.context().any_prefix_annotations(expression_id)])?;
        }
        format_expression(f, expression_id, expression, directive)?;

        // add statement terminators for declaration like expression forms
        let needs_statement_terminator = matches!(
            expression,
            Expression::Import { .. } | Expression::Let { .. } | Expression::Using { .. }
        ) || matches!(
            expression,
            Expression::Declaration(declaration_id)
                if matches!(
                    tree.get(*declaration_id),
                    Declaration::Function {
                        descriptor,
                        signature,
                        ..
                    }
                    if descriptor.name.is_none() && signature.kind == FunctionKind::Lambda
                )
        );
        if needs_statement_terminator {
            write!(f, [token(";")])?;
        }

        // postfix annotations
        if !matches!(
            directive,
            Some(FormatterDirective {
                kind: FormatterDirectiveKind::IgnoreFormat,
                position: FormatterDirectivePosition::Postfix { .. },
            })
        ) {
            write!(
                f,
                [f.context().any_infix_or_postfix_annotations(expression_id)]
            )?;
        }

        prev_was_import = is_import_expr;
        if is_import_expr {
            prev_import_id = Some(expression_id);
        }
        previous_output_end = Some((expression_span.file, expression_span.end));
    }
    Ok(())
}

/// Return whether a block should stay inline.
#[inline]
pub(crate) fn should_inline_block<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    block_id: LocalNodeId<Block>,
) -> bool {
    let block = f.context().tree.get(block_id);
    let span = f.context().get_span(block_id);

    // can only inline if there is at most one expression
    if block.expressions.len() > 1 || f.context().has_infix_annotation(block_id) {
        return false;
    } else if block.expressions.is_empty() {
        // keep empty control flow blocks expanded
        if empty_block_prefers_multiline(f.context(), block_id) {
            return false;
        }

        return true;
    }

    // check whether the block is inlinable based on its contents
    // if any expression is not inline, then the entire block shouldn't be
    let is_body_inlinable = block.expressions.is_empty()
        || block
            .expressions
            .iter()
            .all(|expr_id| f.context().get_node(*expr_id).is_narrow());

    // container (default to self, mostly for testing)
    let (mut container_node_id, mut container_node_type) = f
        .context()
        .get_parent_by_id(block_id.id)
        .unwrap_or((block_id.id, NodeType::Block));
    if container_node_type == NodeType::Expression {
        (container_node_id, container_node_type) = f
            .context()
            .get_parent_by_id(container_node_id)
            .unwrap_or((container_node_id, NodeType::Block));
    }

    is_body_inlinable
        && !f.context().is_at_line_start(block_id.id)
        && !f.context().is_at_line_start(container_node_id)
        && !f.context().has_newline(span)
        && container_node_type != NodeType::Declaration
}

/// Return whether an empty block should stay multiline in control flow contexts.
fn empty_block_prefers_multiline<'ast>(
    context: &DestackFormatContext<'ast>,
    block_id: LocalNodeId<Block>,
) -> bool {
    let Some((parent_expression_id, parent_type)) = context.get_parent(block_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_expression_id);
    let Expression::Block(inner_block_id) = context.tree.get(parent_expression_id) else {
        return false;
    };
    if *inner_block_id != block_id {
        return false;
    }

    let Some((container_id, container_type)) = context.get_parent(parent_expression_id) else {
        return false;
    };
    if container_type != NodeType::Expression {
        return false;
    }

    let container_id = LocalNodeId::<Expression>::new(container_id);
    match context.tree.get(container_id) {
        Expression::Try { .. } => true,
        Expression::If {
            then_expression,
            else_expression,
            ..
        } => {
            then_expression.id == parent_expression_id.id
                || else_expression.is_some_and(|id| id.id == parent_expression_id.id)
        }
        _ => false,
    }
}

/// Format a block (without a nested group!).
/// Format a block with opening and closing braces.
pub fn format_block<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Block>,
) -> FormatResult<()> {
    write!(f, [f.context().any_prefix_annotations(node_id)])?;
    if should_inline_block(f, node_id) {
        format_block_body_narrow(f, node_id)?;
    } else {
        format_block_body_wide(f, node_id)?;
    }
    write!(f, [f.context().any_postfix_annotations(node_id)])?;
    Ok(())
}

impl<'ast> FormatNode<'ast, Block> for Block {
    fn format_node(
        &self,
        node_id: LocalNodeId<Block>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;
        if should_inline_block(f, node_id) {
            write!(
                f,
                [group(&format_with(|f| format_block_body_narrow(
                    f, node_id
                )))]
            )?;
        } else {
            write!(
                f,
                [group(&format_with(|f| format_block_body_wide(f, node_id)))]
            )?;
        }
        write!(f, [f.context().any_postfix_annotations(node_id)])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{Annotation, AnnotationPosition, Block, NodeParentIndex};

    use crate::{
        DestackFormatArtifacts, DestackFormatContext, DestackFormatOptions, TestFormatter,
        assert_format,
    };

    /// Semicolons should be automatically inserted for every value-ignored expression.
    /// Control flow forms like if and let only get semicolons if used as statements.
    #[test]
    fn test_format_block_insert_semicolon() {
        assert_format!(
            r#"{
    import "foo"
    import * as baz from "foo"

    let x = 1;
    let y = 2
    y

    if (x) {
        y
    } else {
        print("foo")
        z(x) 
    }

    loop {
       break;
    }

    let x = z()
    let x = if (let y = 1) {
        z()
    } else {
        w()
    };

    return 5;
}"#,
            r#"{
    import "foo";
    import * as baz from "foo";

    let x = 1;
    let y = 2;
    y;

    if (x) {
        y
    } else {
        print("foo");
        z(x)
    }

    loop {
        break;
    }

    let x = z();
    let x = if (let y = 1) {
        z()
    } else {
        w()
    };

    return 5;
}"#,
            |p| p.eat_block(),
            DestackFormatOptions::default()
        );
    }

    /// Parser annotations should keep one blank prefix between loop and following let statement.
    #[test]
    fn test_block_insert_semicolon_annotation_contract_after_loop() {
        let source = r#"{
    import "foo"
    import * as baz from "foo"

    let x = 1;
    let y = 2
    y

    if (x) {
        y
    } else {
        print("foo")
        z(x) 
    }

    loop {
       break;
    }

    let x = z()
    let x = if (let y = 1) {
        z()
    } else {
        w()
    };

    return 5;
}"#;
        let (test, block_id) = TestFormatter::parse(source, |p| p.eat_block()).unwrap();
        let block = test.tree.get(block_id);
        let Block { expressions, .. } = block;

        let mut loop_statement_id = None;
        let mut let_after_loop_id = None;
        let mut loop_statement_index = None;
        let mut let_after_loop_index = None;
        for (index, expression_id) in expressions.iter().enumerate() {
            let span = test.tree.get_span(*expression_id);
            let source = test.file.span_str(span);
            if source.starts_with("loop {") {
                loop_statement_id = Some(*expression_id);
                loop_statement_index = Some(index);
            }
            if source.starts_with("let x = z()") {
                let_after_loop_id = Some(*expression_id);
                let_after_loop_index = Some(index);
            }
        }

        let loop_statement_id = loop_statement_id.expect("expected loop statement");
        let let_after_loop_id = let_after_loop_id.expect("expected let statement after loop");
        let loop_statement_index = loop_statement_index.expect("expected loop statement index");
        let let_after_loop_index =
            let_after_loop_index.expect("expected let statement after loop index");
        assert_eq!(let_after_loop_index, loop_statement_index + 1);

        let loop_annotations = test.tree.get_annotations(loop_statement_id.id);
        let loop_blank_block_prefix = loop_annotations
            .iter()
            .filter(|annotation_id| {
                matches!(
                    test.tree.get::<Annotation>(**annotation_id),
                    Annotation::Blank {
                        position: AnnotationPosition::BlockPrefix,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(loop_blank_block_prefix, 1);

        let loop_blank_postfix = loop_annotations
            .iter()
            .filter(|annotation_id| {
                matches!(
                    test.tree.get::<Annotation>(**annotation_id),
                    Annotation::Blank {
                        position: AnnotationPosition::BlockPostfix
                            | AnnotationPosition::LinePostfix
                            | AnnotationPosition::LinePostfixBoundary,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(loop_blank_postfix, 0);

        let let_annotations = test.tree.get_annotations(let_after_loop_id.id);
        let let_blank_annotations = let_annotations
            .iter()
            .filter(|annotation_id| {
                matches!(
                    test.tree.get::<Annotation>(**annotation_id),
                    Annotation::Blank { .. }
                )
            })
            .count();
        assert_eq!(let_blank_annotations, 1);

        let let_blank_block_prefix = let_annotations
            .iter()
            .filter(|annotation_id| {
                matches!(
                    test.tree.get::<Annotation>(**annotation_id),
                    Annotation::Blank {
                        position: AnnotationPosition::BlockPrefix,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(let_blank_block_prefix, 1);

        let let_blank_line_prefix = let_annotations
            .iter()
            .filter(|annotation_id| {
                matches!(
                    test.tree.get::<Annotation>(**annotation_id),
                    Annotation::Blank {
                        position: AnnotationPosition::LinePrefix,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(let_blank_line_prefix, 0);

        let context = {
            DestackFormatContext::new(
                DestackFormatOptions::default(),
                DestackFormatArtifacts {
                    file: &test.file,
                    tree: &test.tree,
                    tokens: &test.tokens,
                    side_tokens: &test.side_tokens,
                    side_span: &test.side_span,
                    strings: &test.strings,
                    parents: NodeParentIndex::from_tree(&test.tree),
                },
            )
        };
        assert!(context.has_blank_prefix_annotation(let_after_loop_id));
    }

    /// One blank line between loop and following let should stay one blank line.
    #[test]
    fn test_format_block_loop_blank_line_before_let() {
        assert_format!(
            r#"{
    loop {
        break;
    }

    let x = z()
}"#,
            r#"{
    loop {
        break;
    }

    let x = z();
}"#,
            |p| p.eat_block(),
            DestackFormatOptions::default()
        );
    }

    /// Minimal loop-to-let blank line contract should attach one blank prefix to the let node.
    #[test]
    fn test_block_loop_blank_line_annotation_contract_minimal() {
        let source = r#"{
    loop {
        break;
    }

    let x = z()
}"#;
        let (test, block_id) = TestFormatter::parse(source, |p| p.eat_block()).unwrap();
        let block = test.tree.get(block_id);
        let Block { expressions, .. } = block;
        assert_eq!(expressions.len(), 2);

        let loop_statement_id = expressions[0];
        let let_statement_id = expressions[1];

        let loop_blank_postfix = test
            .tree
            .get_annotations(loop_statement_id.id)
            .iter()
            .filter(|annotation_id| {
                matches!(
                    test.tree.get::<Annotation>(**annotation_id),
                    Annotation::Blank {
                        position: AnnotationPosition::BlockPostfix
                            | AnnotationPosition::LinePostfix
                            | AnnotationPosition::LinePostfixBoundary,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(loop_blank_postfix, 0);

        let let_blank_block_prefix = test
            .tree
            .get_annotations(let_statement_id.id)
            .iter()
            .filter(|annotation_id| {
                matches!(
                    test.tree.get::<Annotation>(**annotation_id),
                    Annotation::Blank {
                        position: AnnotationPosition::BlockPrefix,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(let_blank_block_prefix, 1);
    }

    #[test]
    fn test_format_empty_block_with_comment() {
        let source = "{
    // infix comment
}";
        assert_format!(
            source,
            source,
            |p| p.eat_block(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_mixed_block_with_prefix_postfix_comment() {
        let source = "{
    // prefix comment
    const X = 1; // suffix comment
    // postfix comment
}";
        assert_format!(
            source,
            source,
            |p| p.eat_block(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_mixed_block_with_postfix_comment() {
        let source = "{
    const X = 1; // this is my X
    const Y = 2; // this is my Y
    const Z = 3; // this is my Z
}";
        assert_format!(
            source,
            source,
            |p| p.eat_block(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_mixed_block_with_postfix_annotations_mixed() {
        let source = "{
    const X = 1; // this is my X
    const Y = 2; // this is my Y
    const Z = 3; // this is my Z
}";
        assert_format!(
            source,
            source,
            |p| p.eat_block(),
            DestackFormatOptions::default()
        );
    }

    /// Block shouldn't break if the expression is used inline.
    #[test]
    fn test_format_block_inline() {
        let source = "const x = if (y) { z } else { w }";
        assert_format!(
            source,
            source,
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_block_statement_like() {
        assert_format!(
            "if (y) { z } else { w; }",
            "if (y) {\n\tz\n} else {\n\tw;\n}",
            |p| p.eat_if(),
            DestackFormatOptions::default_tab()
        );
    }

    /// Block should retain the explicit newline.
    #[test]
    fn test_format_block_statement_retain_newline() {
        let source = "{\n\tconst X = 1;\n\tconst Y = 2;\n\tconst Z = 3;\n}";
        assert_format!(
            source,
            source,
            |p| p.eat_block(),
            DestackFormatOptions::default_tab()
        );
    }

    /// Declarations after expressions should not force an extra blank line.
    #[test]
    fn test_format_block_declaration_after_expression_no_forced_blank_line() {
        assert_format!(
            "{ x = 1; class A {} }",
            "{\n\tx = 1;\n\tclass A {}\n}",
            |p| p.eat_block(),
            DestackFormatOptions::default_tab()
        );
    }

    /// Import statements should be sorted when organize_imports is enabled.
    #[test]
    fn test_format_block_import_sorting() {
        use destack_workspace::OrganizeImports;
        let options = DestackFormatOptions {
            organize_imports: OrganizeImports::On,
            ..DestackFormatOptions::default()
        };
        assert_format!(
            r#"{
    import lodash from "lodash"
    import fs from "node:fs"
    import path from "node:path"
}"#,
            r#"{
    import fs from "node:fs";
    import path from "node:path";

    import lodash from "lodash";
}"#,
            |p| p.eat_block(),
            options
        );
    }

    /// Side-effect imports should keep source order and stay above regular imports.
    #[test]
    fn test_format_block_import_sorting_preserves_side_effect_order() {
        use destack_workspace::OrganizeImports;
        let options = DestackFormatOptions {
            organize_imports: OrganizeImports::On,
            ..DestackFormatOptions::default()
        };
        assert_format!(
            r#"{
    import "zeta"
    import thing from "pkg"
    import "alpha"
    import fs from "node:fs"
}"#,
            r#"{
    import "zeta";
    import "alpha";

    import fs from "node:fs";

    import thing from "pkg";
}"#,
            |p| p.eat_block(),
            options
        );
    }

    /// Organized imports should insert blank lines between import groups.
    #[test]
    fn test_format_block_import_sorting_inserts_group_blank_lines() {
        use destack_workspace::OrganizeImports;
        let options = DestackFormatOptions {
            organize_imports: OrganizeImports::On,
            ..DestackFormatOptions::default()
        };
        assert_format!(
            r#"{
    import rel from "./rel"
    import pkg from "react"
    import alias from "~/core"
    import fs from "node:fs"
}"#,
            r#"{
    import fs from "node:fs";

    import pkg from "react";

    import alias from "~/core";

    import rel from "./rel";
}"#,
            |p| p.eat_block(),
            options
        );
    }
}
