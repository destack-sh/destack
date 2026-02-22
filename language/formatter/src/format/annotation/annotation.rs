use destack_fir::format::{FormatResult, hard_line_break};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{Annotation, DestackFormatter, FormatNode};
use destack_ast::{
    Blank, Comment, CommentStyle, Decorator, Doc, DocStyle, Expression, LocalNodeId, NodeTree,
};

impl<'ast> FormatNode<'ast, Annotation> for Annotation {
    fn format_node(
        &self,
        node_id: LocalNodeId<Annotation>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Annotation::Blank { node, .. } => {
                // skip trailing blanks at the end of the source
                let annotation_span = f.context().annotation_span(node_id);
                if annotation_span.end >= f.context().file.len.saturating_sub(1) {
                    return Ok(());
                }

                node.format(f)
            }
            Annotation::Doc { node, .. } => node.format(f),
            Annotation::Comment { node, .. } => node.format(f),
            Annotation::Decorator { node, .. } => node.format(f),
        }
    }
}

impl<'ast> FormatNode<'ast, Blank> for Blank {
    fn format_node(
        &self,
        _node_id: LocalNodeId<Blank>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // reduce any number of blank lines to a single one
        write!(f, [empty_line()])?;
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Doc> for Doc {
    fn format_node(
        &self,
        _node_id: LocalNodeId<Doc>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let string = f.context().strings.get(self.string);
        let is_multi_line = string.contains('\n');
        match self.style {
            DocStyle::Star => {
                if is_multi_line {
                    let total_lines = string.lines().count();
                    for (i, line) in string.lines().enumerate() {
                        let is_last_line = i == total_lines - 1;
                        let is_last_blank_line = is_last_line && line.trim().is_empty();

                        if is_last_blank_line {
                            // keep a trailing empty line compact as plain closing delimiter
                        } else if i == 0 {
                            write!(f, [token("/**")])?;
                        } else {
                            write!(f, [token(" *")])?;
                        }

                        if !line.is_empty() && !is_last_blank_line {
                            write!(f, [space(), text(line)])?;
                        } else if i == 0 {
                            write!(f, [space()])?;
                        }

                        if !is_last_line {
                            write!(f, [hard_line_break()])?;
                        }
                    }
                    if string.ends_with('\n') {
                        write!(f, [hard_line_break()])?;
                    }
                    write!(f, [token(" */")])?;
                } else {
                    let content = normalize_inline_block_comment_content(string);
                    if content.is_empty() {
                        write!(f, [token("/**/")])?;
                    } else if inline_block_comment_prefers_spaced_form(content) {
                        write!(
                            f,
                            [token("/**"), space(), text(content), space(), token("*/")]
                        )?;
                    } else {
                        write!(f, [token("/**"), text(content), token("*/")])?;
                    }
                }
            }
            DocStyle::Slash => {
                format_line_comment_lines(f, "///", string)?;
            }
        }
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Comment> for Comment {
    fn format_node(
        &self,
        node_id: LocalNodeId<Comment>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let string = f.context().comment_text(node_id);
        let raw_comment = f.context().comment_raw_text(node_id);
        let block_open_token = if raw_comment.trim_start().starts_with("/**") {
            "/**"
        } else {
            "/*"
        };
        let is_multi_line = string.contains('\n');
        match self.style {
            CommentStyle::Star => {
                if is_multi_line {
                    let prefers_star_lines = block_comment_prefers_star_lines(raw_comment);
                    let lines: Vec<&str> = string.lines().collect();
                    if prefers_star_lines {
                        for (i, line) in lines.iter().enumerate() {
                            if i == 0 {
                                write!(f, [token(block_open_token)])?;
                            } else {
                                write!(f, [token(" *")])?;
                            }
                            if !line.is_empty() {
                                write!(f, [space(), text(line.trim_end_matches('\r'))])?;
                            }
                            if i != lines.len() - 1 {
                                write!(f, [hard_line_break()])?;
                            }
                        }
                        if string.ends_with('\n') {
                            write!(f, [hard_line_break()])?;
                        }
                        write!(f, [token(" */")])?;
                    } else {
                        for (i, line) in lines.iter().enumerate() {
                            if i == 0 {
                                write!(f, [token(block_open_token)])?;
                            } else {
                                write!(f, [hard_line_break()])?;
                            }
                            if !line.is_empty() {
                                if i == 0 {
                                    write!(f, [space(), text(line.trim_end_matches('\r'))])?;
                                } else {
                                    write!(f, [text(line.trim_end_matches('\r'))])?;
                                }
                            }
                        }
                        if string.ends_with('\n') {
                            write!(f, [hard_line_break(), token("*/")])?;
                        } else {
                            write!(f, [space(), token("*/")])?;
                        }
                    }
                } else {
                    // preserve single-line block comments as parsed to avoid rewriting inline spacing
                    write!(f, [text(raw_comment)])?;
                }
            }
            CommentStyle::Slash => {
                if raw_comment.contains('\n') {
                    format_line_comment_lines(f, "//", string.as_ref())?;
                } else if let Some(payload) = raw_comment.strip_prefix("//") {
                    write!(f, [token("//"), text(payload)])?;
                } else {
                    format_line_comment_lines(f, "//", string.as_ref())?;
                }
            }
        }
        Ok(())
    }
}

/// Format slash style comment lines, preserving empty comment lines.
fn format_line_comment_lines<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    prefix: &'static str,
    content: &str,
) -> FormatResult<()> {
    if content.is_empty() {
        write!(f, [token(prefix)])?;
        return Ok(());
    }

    let mut lines = content.split('\n').peekable();
    while let Some(line) = lines.next() {
        if line.is_empty() {
            write!(f, [token(prefix)])?;
        } else {
            let first_character = line.chars().next();
            let should_insert_space = first_character
                .is_some_and(|character| !character.is_ascii_digit() && !line.starts_with("<-"));
            if should_insert_space {
                write!(f, [token(prefix), space(), text(line)])?;
            } else {
                write!(f, [token(prefix), text(line)])?;
            }
        }

        if lines.peek().is_some() {
            write!(f, [hard_line_break()])?;
        }
    }

    Ok(())
}

/// Normalize inline block comment content for stable output.
fn normalize_inline_block_comment_content(content: &str) -> &str {
    let content = content.trim();

    if let Some(stripped_doc) = content.strip_prefix("/**")
        && let Some(inner) = stripped_doc.strip_suffix("*/")
    {
        return inner.trim();
    }

    if let Some(stripped_comment) = content.strip_prefix("/*")
        && let Some(inner) = stripped_comment.strip_suffix("*/")
    {
        return inner.trim();
    }

    content
}

/// Return whether one raw block comment uses `*`-prefixed continuation lines.
fn block_comment_prefers_star_lines(raw: &str) -> bool {
    let Some(inner) = raw
        .strip_prefix("/*")
        .and_then(|inner| inner.strip_suffix("*/"))
    else {
        return false;
    };

    let mut has_non_empty_continuation_line = false;
    for line in inner.lines().skip(1) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        has_non_empty_continuation_line = true;
        if !line.trim_start().starts_with('*') {
            return false;
        }
    }

    has_non_empty_continuation_line
}

/// Return whether one inline block comment should use surrounding spaces.
fn inline_block_comment_prefers_spaced_form(content: &str) -> bool {
    if content.starts_with("@__") || content.starts_with("#__") {
        return false;
    }

    if content.starts_with('@')
        && content
            .chars()
            .nth(1)
            .is_some_and(|character| character.is_ascii_alphabetic())
    {
        return true;
    }

    let mut characters = content.chars();
    let Some(first_character) = characters.next() else {
        return false;
    };
    let Some(last_character) = content.chars().last() else {
        return false;
    };

    first_character.is_ascii_alphanumeric() && last_character.is_ascii_alphanumeric()
}

impl<'ast> FormatNode<'ast, Decorator> for Decorator {
    fn format_node(
        &self,
        _node_id: LocalNodeId<Decorator>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let tree = f.context().tree;
        let needs_parentheses = decorator_needs_parentheses(tree, self.expression);
        write!(f, [token("@")])?;
        if needs_parentheses {
            write!(f, [token("(")])?;
        }
        write!(f, [self.expression])?;
        if needs_parentheses {
            write!(f, [token(")")])?;
        }
        Ok(())
    }
}

/// Return whether a decorator expression requires parentheses.
fn decorator_needs_parentheses(tree: &NodeTree, expression_id: LocalNodeId<Expression>) -> bool {
    match tree.get(expression_id) {
        Expression::Parenthesized { .. } => false,
        Expression::Path {
            static_arguments, ..
        } => static_arguments.is_some(),
        Expression::Call { left, .. } => !is_identifier_or_static_member_only(tree, *left),
        Expression::Member {
            left,
            static_arguments,
            ..
        } => static_arguments.is_some() || !is_identifier_or_static_member_only(tree, *left),
        _ => true,
    }
}

/// Return whether an expression is an identifier or static-member-only path.
fn is_identifier_or_static_member_only(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match tree.get(expression_id) {
        Expression::Path {
            static_arguments, ..
        } => static_arguments.is_none(),
        Expression::Member {
            left,
            static_arguments,
            ..
        } => static_arguments.is_none() && is_identifier_or_static_member_only(tree, *left),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use crate::format::annotation::render::annotation_precedes_separator;
    use crate::{
        Annotation, DestackFormatArtifacts, DestackFormatContext, DestackFormatOptions,
        TestFormatter, assert_format,
    };
    use destack_ast::{
        AnnotationPosition, DeclarationDescriptor, LocalNodeId, NodeParentIndex, NodeType,
    };
    use destack_source::FileType;

    /// Build a formatter context for annotation routing assertions.
    fn context_from_formatter(formatter: &TestFormatter) -> DestackFormatContext<'_> {
        DestackFormatContext::new(
            DestackFormatOptions::default(),
            DestackFormatArtifacts {
                file: &formatter.file,
                tree: &formatter.tree,
                tokens: &formatter.tokens,
                side_tokens: &formatter.side_tokens,
                side_span: &formatter.side_span,
                strings: &formatter.strings,
                parents: NodeParentIndex::from_tree(&formatter.tree),
            },
        )
    }

    /// Find an annotation node by source marker text.
    fn find_annotation_by_marker(
        context: &DestackFormatContext<'_>,
        marker: &str,
    ) -> Option<LocalNodeId<Annotation>> {
        let annotation_matches_marker =
            |annotation_id: LocalNodeId<Annotation>, marker: &str| match context
                .annotation(annotation_id)
            {
                Annotation::Comment { node, .. } => context.comment_text(node).trim() == marker,
                Annotation::Doc { node, .. } => {
                    let document = context.tree.get(node);
                    context.strings.get(document.string).trim() == marker
                }
                Annotation::Blank { .. } | Annotation::Decorator { .. } => false,
            };

        for (entry_index, _) in context.formatter_annotation_entries.iter().enumerate() {
            let annotation_id = LocalNodeId::<Annotation>::new(entry_index as u32);
            if annotation_matches_marker(annotation_id, marker) {
                return Some(annotation_id);
            }
        }

        None
    }

    /// Find the target owner node for one annotation id.
    fn find_annotation_target_owner_node(
        context: &DestackFormatContext<'_>,
        annotation_id: LocalNodeId<Annotation>,
    ) -> Option<usize> {
        context
            .formatter_annotation_ids_by_node_id
            .iter()
            .enumerate()
            .find_map(|(node_index, annotation_ids)| {
                annotation_ids
                    .iter()
                    .any(|candidate| candidate.id == annotation_id.id)
                    .then_some(node_index)
            })
    }

    /// Single call argument trailing line comments stay discoverable with stable positions.
    #[test]
    fn test_annotation_single_call_argument_trailing_line_comment_attachment() {
        let source = "{
    someFunction(
        value,
        // trailing-argument-marker
    );
}";
        let (formatter, _) = TestFormatter::parse(source, |p| {
            p.eat_block(destack_ast::BlockContext::Expression)
        })
        .expect("parse trailing call argument marker source");
        let context = context_from_formatter(&formatter);

        let annotation_id = find_annotation_by_marker(&context, "trailing-argument-marker")
            .expect("expected trailing marker annotation");
        let position = context.annotation(annotation_id).position();

        assert!(matches!(
            position,
            AnnotationPosition::LinePrefix
                | AnnotationPosition::LinePostfixBoundary
                | AnnotationPosition::BlockPostfix
        ));
    }

    /// Trailing `, // comment )` seams should attach to call arguments.
    #[test]
    fn test_annotation_call_trailing_separator_comment_attaches_to_argument_owner() {
        let source = "{
    call(
        function () {
            var a = 1;
            // one
        },
        // trailing-separator-marker
    );
}";
        let (formatter, _) = TestFormatter::parse(source, |p| {
            p.eat_block(destack_ast::BlockContext::Expression)
        })
        .expect("parse call trailing separator marker source");
        let context = context_from_formatter(&formatter);

        let annotation_id = find_annotation_by_marker(&context, "trailing-separator-marker")
            .expect("expected trailing separator marker annotation");
        let position = context.annotation(annotation_id).position();
        let owner_node = find_annotation_target_owner_node(&context, annotation_id)
            .expect("expected annotation owner node");
        let owner_node_type = context.tree.get_node_type(owner_node as u32);

        assert_eq!(position, AnnotationPosition::LinePostfixBoundary);
        assert_eq!(owner_node_type, NodeType::Argument);
    }

    /// Trailing `, // comment )` seams on multi-argument calls should attach to the last argument.
    #[test]
    fn test_annotation_call_trailing_separator_comment_multi_argument_attaches_to_argument_owner() {
        let source = "{
    call(
        first,
        function () {
            var a = 1;
            // one
        },
        // trailing-separator-multi-marker
    );
}";
        let (formatter, _) = TestFormatter::parse(source, |p| {
            p.eat_block(destack_ast::BlockContext::Expression)
        })
        .expect("parse call trailing separator marker source");
        let context = context_from_formatter(&formatter);

        let annotation_id = find_annotation_by_marker(&context, "trailing-separator-multi-marker")
            .expect("expected trailing separator marker annotation");
        let position = context.annotation(annotation_id).position();
        let owner_node = find_annotation_target_owner_node(&context, annotation_id)
            .expect("expected annotation owner node");
        let owner_node_type = context.tree.get_node_type(owner_node as u32);

        assert_eq!(position, AnnotationPosition::LinePostfixBoundary);
        assert_eq!(owner_node_type, NodeType::Argument);
    }

    /// Inline block comments between call callees and `(` should stay on the call expression.
    #[test]
    fn test_annotation_call_callee_block_comment_stays_on_call_expression() {
        let source = "{
    call/* call-marker */();
    call/* optional-marker */?.();
}";
        let expected = "{\n    call /* call-marker */();\n    call /* optional-marker */?.();\n}";
        let (formatter, block_id) = TestFormatter::parse(source, |p| {
            p.eat_block(destack_ast::BlockContext::Expression)
        })
        .expect("parse call callee block comment source");
        let context = context_from_formatter(&formatter);

        let first_annotation_id =
            find_annotation_by_marker(&context, "call-marker").expect("expected call annotation");
        let first_owner_node = find_annotation_target_owner_node(&context, first_annotation_id)
            .expect("expected first annotation owner node");
        let first_owner_node_type = context.tree.get_node_type(first_owner_node as u32);
        let first_position = context.annotation(first_annotation_id).position();
        assert_eq!(first_owner_node_type, NodeType::Expression);
        assert_eq!(first_position, AnnotationPosition::LinePostfix);

        let second_annotation_id = find_annotation_by_marker(&context, "optional-marker")
            .expect("expected optional annotation");
        let second_owner_node = find_annotation_target_owner_node(&context, second_annotation_id)
            .expect("expected second annotation owner node");
        let second_owner_node_type = context.tree.get_node_type(second_owner_node as u32);
        let second_position = context.annotation(second_annotation_id).position();
        assert_eq!(second_owner_node_type, NodeType::Expression);
        assert_eq!(second_position, AnnotationPosition::LinePostfix);

        let formatted = formatter.format(&block_id, DestackFormatOptions::default());
        assert_eq!(formatted, expected);
    }

    /// Type-binary block comments between operator and right type must stay attached and render.
    #[test]
    fn test_type_binary_block_comment_between_operator_and_right_type_renders() {
        let source = "{\n    const value = left as /* between */ Foo;\n}";
        let expected = "{\n    const value = left as /* between */ Foo;\n}";
        let (formatter, block_id) =
            TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| {
                p.eat_block(destack_ast::BlockContext::Expression)
            })
            .expect("parse type-binary block seam source");
        let context = context_from_formatter(&formatter);
        let annotation_id =
            find_annotation_by_marker(&context, "between").expect("expected between annotation");
        let annotation = context.annotation(annotation_id);
        assert_eq!(annotation.position(), AnnotationPosition::LinePrefix);

        let formatted = formatter.format(&block_id, DestackFormatOptions::default());
        assert_eq!(formatted, expected);
    }

    /// Type-binary block seam comments on expression statements must not be dropped.
    #[test]
    fn test_type_binary_block_comment_between_operator_and_right_type_expression_statement() {
        let source = "{\n    1 as /* between */ Foo;\n    1 satisfies /* sat-between */ Foo;\n}";
        let expected = "{\n    1 as /* between */ Foo;\n    1 satisfies /* sat-between */ Foo;\n}";
        let (formatter, block_id) =
            TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| {
                p.eat_block(destack_ast::BlockContext::Expression)
            })
            .expect("parse type-binary block seam statement source");
        let context = context_from_formatter(&formatter);
        let first_annotation_id =
            find_annotation_by_marker(&context, "between").expect("expected between annotation");
        let second_annotation_id = find_annotation_by_marker(&context, "sat-between")
            .expect("expected sat-between annotation");
        assert_eq!(
            context.annotation(first_annotation_id).position(),
            AnnotationPosition::LinePrefix
        );
        assert_eq!(
            context.annotation(second_annotation_id).position(),
            AnnotationPosition::LinePrefix
        );

        let formatted = formatter.format(&block_id, DestackFormatOptions::default());
        assert_eq!(formatted, expected);
    }

    #[test]
    fn test_type_mapped_remap_line_comment_attachment() {
        let source = "{\n    type Paths<T> = {\n      [K in keyof T as // remap-note\n        `get${Capitalize<K & string>}`]: () => T[K]\n    }\n}";
        let expected = "{\n    type Paths<T> = {\n        [K in keyof T as `get${Capitalize<K & string> // remap-note\n        }`]: () => T[K],\n    };\n}";
        let (formatter, block_id) =
            TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| {
                p.eat_block(destack_ast::BlockContext::Expression)
            })
            .expect("parse mapped remap comment source");
        let context = context_from_formatter(&formatter);
        let annotation_id = find_annotation_by_marker(&context, "remap-note")
            .expect("expected remap-note annotation");
        let owner_node = find_annotation_target_owner_node(&context, annotation_id)
            .expect("expected remap owner node");
        let owner_node_type = context.tree.get_node_type(owner_node as u32);
        let position = context.annotation(annotation_id).position();
        assert_eq!(owner_node_type, NodeType::Expression);
        assert_eq!(position, AnnotationPosition::LinePostfix);

        let formatted = formatter.format(&block_id, DestackFormatOptions::default());
        assert_eq!(formatted, expected);
    }

    /// Prefix cast comments before parenthesized values should keep one separating space.
    #[test]
    fn test_prefix_cast_comment_keeps_space_before_parenthesized_value() {
        let source = "{\n    target(/** @type {{id: string}} */ (entry), second);\n}";
        let expected = "{\n    target(/** @type {{id: string}} */ (entry), second);\n}";
        let (formatter, block_id) =
            TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| {
                p.eat_block(destack_ast::BlockContext::Expression)
            })
            .expect("parse parenthesized cast argument source");

        let formatted = formatter.format(&block_id, DestackFormatOptions::default());
        assert_eq!(formatted, expected);
    }

    /// Condition boundary comments should detect closing delimiter separators.
    #[test]
    fn test_annotation_render_info_condition_comment_precedes_separator() {
        let source = "{
    if (true /* separator-marker */ ) {}
}";
        let (formatter, _) = TestFormatter::parse(source, |p| {
            p.eat_block(destack_ast::BlockContext::Expression)
        })
        .expect("parse separator marker source");
        let context = context_from_formatter(&formatter);
        let annotation_id = find_annotation_by_marker(&context, "separator-marker")
            .expect("expected marker-tagged separator annotation");
        let precedes_separator = annotation_precedes_separator(&context, annotation_id);
        assert!(precedes_separator);
    }

    /// Block comments should retain all their newlines (including leading and trailing newlines).
    #[test]
    fn test_format_block_comment_retain_newlines() {
        let source = r#"{
    /*
     * Comment 1
     */
    let x;

    /*
     * Comment 2.1
     * Comment 2.2
     * Comment 2.3
     */
    let y;
}"#;
        assert_format!(
            source,
            source,
            |p| p.eat_block(destack_ast::BlockContext::Expression),
            DestackFormatOptions::default()
        );
    }

    /// Decorators should be preserved in order with other annotations.
    #[test]
    fn test_format_decorators_on_struct() {
        let source = r#"{
    // comment before entity
    @entity
    // comment after entity
    // comment before foo
    @foo(1, 2, 3)
    // comment after foo
    struct Entity {}
}"#;
        assert_format!(
            source,
            source,
            |p| p.eat_block(destack_ast::BlockContext::Expression),
            DestackFormatOptions::default()
        );
    }

    /// Decorator expressions should not grow extra parentheses across formatting.
    #[test]
    fn test_format_decorator_parentheses_are_stable() {
        let source = r#"{
    @(chain.first().second())
    function chained() {}
}"#;
        assert_format!(
            source,
            source,
            |p| p.eat_block(destack_ast::BlockContext::Expression),
            DestackFormatOptions::default()
        );
    }

    /// Decorator prefixed type annotations should stay inline after a colon when simple.
    #[test]
    fn test_format_decorator_type_annotation_stays_inline_after_colon() {
        assert_format!(
            "{\n    const buffer: @addrspace(\"shared\") &Buffer = value;\n}",
            "{\n    const buffer: @addrspace(\"shared\") &Buffer = value;\n}",
            |p| p.eat_block(destack_ast::BlockContext::Expression),
            DestackFormatOptions::default()
        );
    }

    /// Decorator call object member trailing comments should stay attached to the object member.
    #[test]
    fn test_annotation_decorator_call_object_member_trailing_comment_attachment() {
        let source = "{
    @Component({
        selector: \"my-component\", // decorator-call-marker
    })
    class AppMyComponent {}
}";
        let (formatter, _) =
            TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| {
                p.eat_block(destack_ast::BlockContext::Expression)
            })
            .expect("parse decorator call trailing comment source");
        let context = context_from_formatter(&formatter);

        let annotation_id = find_annotation_by_marker(&context, "decorator-call-marker")
            .expect("expected decorator call marker annotation");
        let position = context.annotation(annotation_id).position();
        let owner_node = find_annotation_target_owner_node(&context, annotation_id)
            .expect("expected annotation owner node");
        let owner_node_type = context.tree.get_node_type(owner_node as u32);

        assert_eq!(owner_node_type, NodeType::Property);
        assert_eq!(position, AnnotationPosition::LinePostfixBoundary);
    }

    /// Multiple comments around an expression should retain their order.
    #[test]
    fn test_format_multiple_comments_around_expression() {
        let source = "{
    // comment part 1
    // comment part 2
    const A = 1;
    // comment part 3
    // comment part 4
}";
        assert_format!(
            source,
            source,
            |p| p.eat_block(destack_ast::BlockContext::Expression),
            DestackFormatOptions::default()
        );
    }

    /// Multiple comments around an expression should retain their order across successive blocks.
    #[test]
    fn test_format_multiple_comments_around_expression_in_successive_blocks() {
        let source = "{
    // comment part 0
    a: {
        // comment part 1
        // comment part 2
        const A = 1;
        // comment part 3
        // comment part 4
    }
    // comment part 5
    // comment part 6
    b: {
        // comment part 7
        // comment part 8
        const B = 2;
        // comment part 9
        // comment part 10
    }
    // comment part 11
}";
        assert_format!(
            source,
            source,
            |p| p.eat_block(destack_ast::BlockContext::Expression),
            DestackFormatOptions::default()
        );
    }

    /// Inline expression comments should be preserved with proper spacing.
    #[test]
    fn test_format_inline_expression_comment() {
        assert_format!(
            "/* Pre-X comment */const X=/* Pre-A comment */A/* A comment */&&B/* B comment */",
            "/* Pre-X comment */ const X = /* Pre-A comment */ A /* A comment */ && B /* B comment */",
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default_with_line_width(200)
        );
    }

    /// Keep multiline block doc comments as block comments.
    #[test]
    fn test_format_multi_line_block_doc_comment_stays_block() {
        assert_format!(
            "{
    /** some multiline
     * doc comment
     * over multiple lines */
    const X = 1 
}",
            "{
    /** some multiline
     * doc comment
     * over multiple lines */
    const X = 1;
}",
            |p| p.eat_block(destack_ast::BlockContext::Expression),
            DestackFormatOptions::default()
        );
    }

    /// Keep multiline postfix comments as block comments.
    #[test]
    fn test_format_multi_line_block_comment_stays_block() {
        assert_format!(
            "{
    const X = 1 /* some comment
    * over multiple lines yo       */
}",
            "{
    const X = 1;
    /* some comment
     * over multiple lines yo */
}",
            |p| p.eat_block(destack_ast::BlockContext::Expression),
            DestackFormatOptions::default()
        );
    }

    /// Excessive whitespace in line comments should be preserved.
    #[test]
    fn test_format_excessive_whitespace_in_line_comment() {
        let source = r"{
    // /// An Identity is globally unique identifier for an Entity.
    // struct Identity {
    //     /// The universally unique identifier of this Entity.
    //     id: Uuid
    // }
    const X = 1;
}";
        assert_format!(
            source,
            source,
            |p| p.eat_block(destack_ast::BlockContext::Expression),
            DestackFormatOptions::default()
        );
    }

    /// Comments inside function call arguments cause expansion.
    #[test]
    fn test_format_comment_in_call_arguments() {
        assert_format!(
            "foo(/* first */ a, /* second */ b)",
            "foo(/* first */ a, /* second */ b)",
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default()
        );
    }

    /// Inline array comments stay inline when the array still fits.
    #[test]
    fn test_format_comment_in_array() {
        assert_format!(
            "[/* first */ 1, /* second */ 2, /* third */ 3]",
            "[/* first */ 1, /* second */ 2, /* third */ 3]",
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default()
        );
    }

    /// Comments inside object literals cause expansion.
    #[test]
    fn test_format_comment_in_object() {
        assert_format!(
            "{ /* key */ a: 1, /* another */ b: 2 }",
            "{
    /* key */ a: 1,
    /* another */ b: 2,
}",
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default()
        );
    }

    /// Format trailing comments on array elements to stay with the comma.
    #[test]
    fn test_format_trailing_comment_array() {
        assert_format!(
            "{
    const arr = [
        1,
        2,
        3, // last element
    ];
}",
            "{
    const arr = [
        1,
        2,
        3, // last element
    ];
}",
            |p| p.eat_block(destack_ast::BlockContext::Expression),
            DestackFormatOptions::default()
        );
    }

    /// Comment inside function body.
    #[test]
    fn test_format_comment_in_function_body() {
        assert_format!(
            "function foo() { /* empty */ }",
            "function foo() {\n    /* empty */\n}",
            |p| p.eat_function(&p.mark(), DeclarationDescriptor::default(), false, false),
            DestackFormatOptions::default()
        );
    }

    /// Empty slash star doc comments should stay stable on arrows.
    #[test]
    fn test_format_empty_doc_comment_on_arrow() {
        assert_format!(
            "() /**/ => 1",
            "() /**/ => 1",
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default()
        );
    }

    /// Pure hint comments should keep compact style.
    #[test]
    fn test_format_compact_pure_hint_comment() {
        assert_format!(
            "/*#__PURE__*/factory()",
            "/*#__PURE__*/ factory()",
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default()
        );
    }

    /// Blank lines between array elements should be preserved.
    #[test]
    fn test_format_blank_in_array() {
        assert_format!(
            "[
    1,

    2,
]",
            "[
    1,

    2,
]",
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default()
        );
    }

    /// Inline block comments before arrow bodies should keep one separating space and remain idempotent.
    #[test]
    fn test_format_inline_block_comment_before_arrow_body_call_is_idempotent() {
        let source = "{
    const fn = () =>
        /* event, data */doSomething();

    const fn2 = () =>
        /* event, data */doSomething(anything);
}";
        let (first_formatter, first_block_id) = TestFormatter::parse_with_file_type(
            source,
            destack_source::FileType::JavaScript,
            |p| p.eat_block(destack_ast::BlockContext::Expression),
        )
        .expect("parse first arrow comment statement");
        let first = first_formatter.format(&first_block_id, DestackFormatOptions::default());

        let (second_formatter, second_block_id) = TestFormatter::parse_with_file_type(
            first.as_str(),
            destack_source::FileType::JavaScript,
            |p| p.eat_block(destack_ast::BlockContext::Expression),
        )
        .expect("parse second arrow comment statement");
        let second = second_formatter.format(&second_block_id, DestackFormatOptions::default());

        assert_eq!(first, second);
    }

    /// Member-chain inline block comments should stay attached at the original chain seam.
    #[test]
    fn test_format_member_chain_inline_block_comments_stay_on_chain_seams() {
        assert_format!(
            "{
    wow /** marker-one */
      .omg! /** marker-two */
      .map((x) => x.name) /** marker-three */
      .filter((x) => x.length > 3)
      .sort((a, b) => a.length - b.length);
}",
            "{
    wow /** marker-one */
        .omg! /** marker-two */
        .map((x) => x.name) /** marker-three */
        .filter((x) => x.length > 3)
        .sort((a, b) => a.length - b.length);
}",
            |p| p.eat_block(destack_ast::BlockContext::Expression),
            DestackFormatOptions::default()
        );
    }

    /// Inline class-head block comments before `{` should stay in the class header.
    #[test]
    fn test_format_class_head_block_comment_stays_before_open_brace() {
        assert_format!(
            "{
    export class Cls /* marker-class */ {
        // body
    }
}",
            "{
    export class Cls /* marker-class */ {
        // body
    }
}",
            |p| p.eat_block(destack_ast::BlockContext::Expression),
            DestackFormatOptions::default()
        );
    }

    /// Decorators should remain grouped when separated by a line comment.
    #[test]
    fn test_format_decorator_with_leading_line_comment_stays_grouped() {
        assert_format!(
            "{
    class A {
        // marker-decorator
        @memoize onContextMenu() {}
    }
}",
            "{
    class A {
        // marker-decorator
        @memoize onContextMenu() {}
    }
}",
            |p| p.eat_block(destack_ast::BlockContext::Expression),
            DestackFormatOptions::default()
        );
    }

    /// Inline member decorators should keep spans ending before the member head.
    #[test]
    fn test_decorator_annotation_span_stops_before_inline_member_head() {
        let source = "{
    class A {
        // marker-decorator
        @memoize onContextMenu() {}
    }
}";
        let (formatter, _) = TestFormatter::parse(source, |p| {
            p.eat_block(destack_ast::BlockContext::Expression)
        })
        .expect("parse inline member decorator source");
        let context = context_from_formatter(&formatter);

        let (annotation_id, owner_node) = context
            .formatter_annotation_entries
            .iter()
            .enumerate()
            .find_map(|(index, _)| {
                let annotation_id = LocalNodeId::<Annotation>::new(index as u32);
                let is_decorator = matches!(
                    context.annotation(annotation_id),
                    Annotation::Decorator { .. }
                );
                if !is_decorator {
                    return None;
                }
                let owner_node = find_annotation_target_owner_node(&context, annotation_id)?;
                Some((annotation_id, owner_node))
            })
            .expect("expected decorator annotation");

        let owner_node_id = owner_node as u32;
        assert_eq!(context.tree.get_node_type(owner_node_id), NodeType::Member);
        assert_eq!(
            context.annotation(annotation_id).position(),
            AnnotationPosition::BlockPrefix
        );

        let annotation_span = context.annotation_span(annotation_id);
        let owner_span = context.tree.get_span_by_id(owner_node_id);
        assert!(
            annotation_span.end <= owner_span.start,
            "decorator span should end before member head: annotation_span={annotation_span:?} owner_span={owner_span:?} annotation={:?} owner={:?}",
            context.span_str(annotation_span),
            context.span_str(owner_span),
        );
    }

    /// Own-line chain boundary comments after yield should keep member-call shape.
    #[test]
    fn test_format_yield_chain_boundary_comment_keeps_call_chain_shape() {
        assert_format!(
            "{
    function* a() {
        yield task
            // marker-yield
            .run();
    }
}",
            "{
    function* a() {
        yield task
            // marker-yield
            .run();
    }
}",
            |p| p.eat_block(destack_ast::BlockContext::Expression),
            DestackFormatOptions::default()
        );
    }
}
