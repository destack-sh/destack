use destack_ast::{
    Block, BlockFormat, Expression, LocalNodeId, Node, NodeTree, NodeTreeImpl, NodeType,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_source::Span;

use crate::format::analysis::timing::tags;
use crate::format::declaration::statement_list::{
    block_allows_value_tail, format_block_body_narrow, format_block_body_wide,
};
use crate::format::directive::{has_file_ignore_directive, write_ignored_span};
use crate::{DestackFormatContext, DestackFormatter, FormatNode};

pub(crate) use crate::format::declaration::statement_list::format_block_of_statements;

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
            let file_line_count = f.context().file_line_count();
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
        format_block_of_statements(f, self.expressions, false)?;
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
        .is_some_and(|expression_id| context.parent(*expression_id).is_none())
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

/// Return whether a block should stay inline.
#[inline]
pub(crate) fn should_inline_block<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    block_id: LocalNodeId<Block>,
) -> bool {
    let block = f.context().tree.get(block_id);
    let span = f.context().span(block_id);

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

    // explicit non-value blocks should stay expanded, except empty blocks above
    if block.format == BlockFormat::Explicit && !block_allows_value_tail(f.context(), block_id) {
        return false;
    }

    // check whether the block is inlinable based on its contents
    // if any expression is not inline, then the entire block shouldn't be
    let is_body_inlinable = block.expressions.is_empty()
        || block
            .expressions
            .iter()
            .all(|expr_id| f.context().node(*expr_id).is_narrow());

    // container (default to self, mostly for testing)
    let (mut container_node_id, mut container_node_type) = f
        .context()
        .parent_by_id(block_id.id)
        .unwrap_or((block_id.id, NodeType::Block));
    if container_node_type == NodeType::Expression {
        (container_node_id, container_node_type) = f
            .context()
            .parent_by_id(container_node_id)
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
    let Some((parent_expression_id, parent_type)) = context.parent(block_id) else {
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

    let Some((container_id, container_type)) = context.parent(parent_expression_id) else {
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
    use destack_ast::{AnnotationPosition, Block, NodeParentIndex};
    use destack_source::FileType;

    use crate::{
        Annotation, DestackFormatArtifacts, DestackFormatContext, DestackFormatOptions,
        TestFormatter, assert_format,
    };

    /// Semicolons should be inserted for non-tail statement expressions.
    /// Tail expressions in value-position blocks should stay semicolonless.
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
        y;
    } else {
        print("foo");
        z(x);
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
            |p| p.eat_block(destack_ast::BlockContext::Expression),
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
        let (test, block_id) = TestFormatter::parse(source, |p| {
            p.eat_block(destack_ast::BlockContext::Expression)
        })
        .unwrap();
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

        let context = DestackFormatContext::new(
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
        );

        let loop_annotations = context.annotations(loop_statement_id).unwrap_or_default();
        let loop_blank_block_prefix = loop_annotations
            .iter()
            .filter(|annotation_id| {
                matches!(
                    context.annotation(**annotation_id),
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
                    context.annotation(**annotation_id),
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

        let let_annotations = context.annotations(let_after_loop_id).unwrap_or_default();
        let let_blank_annotations = let_annotations
            .iter()
            .filter(|annotation_id| {
                matches!(
                    context.annotation(**annotation_id),
                    Annotation::Blank { .. }
                )
            })
            .count();
        assert_eq!(let_blank_annotations, 0);

        let let_blank_block_prefix = let_annotations
            .iter()
            .filter(|annotation_id| {
                matches!(
                    context.annotation(**annotation_id),
                    Annotation::Blank {
                        position: AnnotationPosition::BlockPrefix,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(let_blank_block_prefix, 0);

        let let_blank_line_prefix = let_annotations
            .iter()
            .filter(|annotation_id| {
                matches!(
                    context.annotation(**annotation_id),
                    Annotation::Blank {
                        position: AnnotationPosition::LinePrefix,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(let_blank_line_prefix, 0);

        assert!(!context.has_blank_prefix_annotation(let_after_loop_id));
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
            |p| p.eat_block(destack_ast::BlockContext::Expression),
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
        let (test, block_id) = TestFormatter::parse(source, |p| {
            p.eat_block(destack_ast::BlockContext::Expression)
        })
        .unwrap();
        let block = test.tree.get(block_id);
        let Block { expressions, .. } = block;
        assert_eq!(expressions.len(), 2);

        let loop_statement_id = expressions[0];
        let let_statement_id = expressions[1];

        let context = DestackFormatContext::new(
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
        );

        let loop_blank_postfix = context
            .annotations(loop_statement_id)
            .unwrap_or_default()
            .iter()
            .filter(|annotation_id| {
                matches!(
                    context.annotation(**annotation_id),
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

        let let_blank_block_prefix = context
            .annotations(let_statement_id)
            .unwrap_or_default()
            .iter()
            .filter(|annotation_id| {
                matches!(
                    context.annotation(**annotation_id),
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
            |p| p.eat_block(destack_ast::BlockContext::Expression),
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
            |p| p.eat_block(destack_ast::BlockContext::Expression),
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
            |p| p.eat_block(destack_ast::BlockContext::Expression),
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
            |p| p.eat_block(destack_ast::BlockContext::Expression),
            DestackFormatOptions::default()
        );
    }

    /// Block shouldn't break if the expression is used inline.
    #[test]
    fn test_format_block_inline() {
        let source = "const x = if (y) { z } else { w }";
        assert_format!(
            "const x = if (y) { z } else { w }",
            source,
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_block_statement_like() {
        assert_format!(
            "if (y) { z } else { w; }",
            "if (y) {\n\tz;\n} else {\n\tw;\n}",
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
            |p| p.eat_block(destack_ast::BlockContext::Expression),
            DestackFormatOptions::default_tab()
        );
    }

    /// Declarations after expressions should not force an extra blank line.
    #[test]
    fn test_format_block_declaration_after_expression_no_forced_blank_line() {
        assert_format!(
            "{ x = 1; class A {} }",
            "{\n\tx = 1;\n\tclass A {}\n}",
            |p| p.eat_block(destack_ast::BlockContext::Expression),
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
            |p| p.eat_block(destack_ast::BlockContext::Expression),
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
            |p| p.eat_block(destack_ast::BlockContext::Expression),
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
            |p| p.eat_block(destack_ast::BlockContext::Expression),
            options
        );
    }

    /// A directive prelude followed by one statement keeps one blank line before the next declaration.
    #[test]
    fn test_statement_list_directive_comment_adjacency_keeps_single_blank_line() {
        let source = r#"/******/ "use strict" /**/
/******/ a;

function func() {
  /******/ "use strict" //
  /******/ b;
}
"#;
        let expected = r#"/******/ "use strict"; /**/
/******/ a;

function func() {
    /******/ "use strict"; //
    /******/ b;
}
"#;

        let (test, expressions) =
            TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| Ok(p.parse()))
                .unwrap();
        let formatted = test.format(
            &super::statement_list(expressions.as_slice()),
            DestackFormatOptions::default(),
        );

        assert_eq!(formatted, expected);
    }

    /// Expression statements keep inline optional-call star boundary comments.
    #[test]
    fn test_statement_optional_call_keeps_inline_boundary_star_comment() {
        let source = r#"// Issue #18969 - comment between callee and optional chaining operator
alert/* comment */?.('value')"#;
        let expected = r#"// Issue #18969 - comment between callee and optional chaining operator
alert /* comment */?.("value");"#;
        let (test, expressions) =
            TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| Ok(p.parse()))
                .unwrap();
        let formatted = test.format(
            &super::statement_list(expressions.as_slice()),
            DestackFormatOptions::default(),
        );
        assert_eq!(formatted.trim_end_matches('\n'), expected);
    }
}
