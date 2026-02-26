use super::render::annotation_precedes_separator;
use crate::{
    Annotation, DestackFormatArtifacts, DestackFormatContext, DestackFormatOptions, TestFormatter,
    assert_format, assert_format_program_idempotent_with_file_type, statement_list,
};
use destack_ast::{
    AnnotationPosition, DeclarationDescriptor, Expression, LocalNodeId, NodeParentIndex, NodeType,
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

/// Trailing array-comma line comments should attach to the array element owner.
#[test]
fn test_annotation_trailing_array_comma_line_comment_attaches_to_element_owner() {
    let source = "{
    const arr = [
        1,
        2,
        3, // trailing-array-marker
    ];
}";
    let (formatter, _) = TestFormatter::parse(source, |p| {
        p.eat_block(destack_ast::BlockContext::Expression)
    })
    .expect("parse trailing array comma comment source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "trailing-array-marker")
        .expect("expected trailing array marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected trailing array marker owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);

    assert_eq!(position, AnnotationPosition::LinePostfixBoundary);
    assert_eq!(owner_node_type, NodeType::Argument);
}

/// Declarator seam line comments after `=` should attach as leading comments to rhs values.
#[test]
fn test_annotation_declarator_assignment_line_comment_attaches_to_rhs_value() {
    let source = "{
    let value = // declarator-rhs-marker
    {
        key: 1,
    };
}";
    let (formatter, _) = TestFormatter::parse(source, |p| {
        p.eat_block(destack_ast::BlockContext::Expression)
    })
    .expect("parse declarator assignment seam comment source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "declarator-rhs-marker")
        .expect("expected declarator rhs marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected declarator rhs marker owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);

    assert_eq!(position, AnnotationPosition::LinePrefix);
    assert_eq!(owner_node_type, NodeType::Expression);
}

/// Declarator seam line comments before array rhs values should stay leading on the rhs.
#[test]
fn test_annotation_declarator_assignment_array_line_comment_attaches_to_rhs_value() {
    let source = "{
    let value =
        // declarator-array-rhs-marker
        [\"val\"];
}";
    let (formatter, _) = TestFormatter::parse(source, |p| {
        p.eat_block(destack_ast::BlockContext::Expression)
    })
    .expect("parse declarator assignment array seam comment source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "declarator-array-rhs-marker")
        .expect("expected declarator array rhs marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected declarator array rhs marker owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);

    assert_eq!(position, AnnotationPosition::BlockPrefix);
    assert_eq!(owner_node_type, NodeType::Expression);
}

/// Same-line `, // comment` seams in call argument lists should attach to argument owners.
#[test]
fn test_annotation_call_inline_separator_comment_attaches_to_argument_owner() {
    let source = "{
    call(
        overflowing ? \"absolute top-0\" : \"relative\", // inline-separator-marker
        parameter,
    );
}";
    let (formatter, _) = TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
        p.eat_block(destack_ast::BlockContext::Expression)
    })
    .expect("parse call inline separator marker source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "inline-separator-marker")
        .expect("expected inline separator marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected annotation owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);

    assert_eq!(owner_node_type, NodeType::Argument);
    assert_eq!(position, AnnotationPosition::LinePostfixBoundary);
}

/// Oxfmt conditional-argument separator line comments should attach to argument owners.
#[test]
fn test_annotation_call_conditional_separator_comment_matches_oxfmt_fixture() {
    let source = "{
    cb(
        overflowing ? 'absolute top-0' : 'relative', // inline-separator-marker
        parameter
    );
}";
    let (formatter, _) = TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
        p.eat_block(destack_ast::BlockContext::Expression)
    })
    .expect("parse oxfmt conditional separator marker source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "inline-separator-marker")
        .expect("expected inline separator marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected annotation owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);

    assert_eq!(owner_node_type, NodeType::Argument);
    assert_eq!(position, AnnotationPosition::LinePostfixBoundary);
}

/// Inline-block conditional separator comments should attach to argument owners.
#[test]
fn test_annotation_call_conditional_separator_comment_with_inline_block_matches_oxfmt_fixture() {
    let source = "{
    cb(
        overflowing ? 'absolute top-0' : 'relative' /* */, // inline-separator-marker
        parameter
    );
}";
    let (formatter, _) = TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
        p.eat_block(destack_ast::BlockContext::Expression)
    })
    .expect("parse oxfmt conditional separator marker with inline block source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "inline-separator-marker")
        .expect("expected inline separator marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected annotation owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);

    assert_eq!(owner_node_type, NodeType::Argument);
    assert_eq!(position, AnnotationPosition::LinePostfixBoundary);
}

/// Export item comments before separators should stay attached to dependency item seams.
#[test]
fn test_annotation_export_item_separator_line_comment_attaches_to_dependency_item() {
    let source = "const foo = \"\";
export {
  foo // export-item-separator-marker
  ,
}";
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| Ok(p.parse()))
            .expect("parse export item separator marker source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "export-item-separator-marker")
        .expect("expected export item separator marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected export item separator marker owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);
    let next_token_type = context.annotation_next_non_whitespace_token_type(annotation_id);

    assert_eq!(owner_node_type, NodeType::DependencyItem);
    assert_eq!(position, AnnotationPosition::LinePostfixBoundary);
    assert_eq!(next_token_type, Some(destack_ast::TokenType::Comma));
}

/// Import item comments after separators should stay attached to dependency item seams.
#[test]
fn test_annotation_import_item_separator_line_comment_attaches_to_dependency_item() {
    let source = "import {
  first, // import-item-separator-marker
  second,
} from \"pkg\";";
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| Ok(p.parse()))
            .expect("parse import item separator marker source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "import-item-separator-marker")
        .expect("expected import item separator marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected import item separator marker owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);
    let previous_token_type = context.annotation_previous_non_whitespace_token_type(annotation_id);
    let next_token_type = context.annotation_next_non_whitespace_token_type(annotation_id);

    assert_eq!(owner_node_type, NodeType::DependencyItem);
    assert_eq!(position, AnnotationPosition::LinePostfixBoundary);
    assert_eq!(previous_token_type, Some(destack_ast::TokenType::Comma));
    assert_eq!(next_token_type, Some(destack_ast::TokenType::Identifier));
}

/// Export alias line comments after `as` should attach as dependency-item prefixes.
#[test]
fn test_annotation_export_alias_line_comment_after_as_attaches_to_dependency_item_prefix() {
    let source = "const foo = \"\";
const bar = \"\";
export {
  foo,
  bar as // export-alias-marker
  baz,
} from \"pkg\";";
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| Ok(p.parse()))
            .expect("parse export alias seam source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "export-alias-marker")
        .expect("expected export alias marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected export alias marker owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);
    let previous_token_type = context.annotation_previous_non_whitespace_token_type(annotation_id);

    assert_eq!(owner_node_type, NodeType::DependencyItem);
    assert_eq!(position, AnnotationPosition::LinePrefix);
    assert_eq!(
        previous_token_type,
        Some(destack_ast::TokenType::Identifier)
    );
}

/// Export alias line comments after `as` should stay on dependency items with and without `from`.
#[test]
fn test_annotation_export_alias_line_comment_after_as_with_from_attaches_to_dependency_item_prefix()
{
    let source = "const foooo = \"\";
const barrr = \"\";
export {
  foooo,
  barrr as // export-alias-from-marker
  baz,
} from \"pkg\";

const fooooo = \"\";
const barrrr = \"\";
export {
  fooooo,
  barrrr as // export-alias-local-marker
  bazz,
};";
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| Ok(p.parse()))
            .expect("parse export alias source with and without from");
    let context = context_from_formatter(&formatter);

    for marker in ["export-alias-from-marker", "export-alias-local-marker"] {
        let annotation_id = find_annotation_by_marker(&context, marker)
            .expect("expected export alias marker annotation");
        let position = context.annotation(annotation_id).position();
        let owner_node = find_annotation_target_owner_node(&context, annotation_id)
            .expect("expected export alias marker owner node");
        let owner_node_type = context.tree.get_node_type(owner_node as u32);

        assert_eq!(owner_node_type, NodeType::DependencyItem);
        assert_eq!(position, AnnotationPosition::LinePrefix);
    }
}

/// Line comments between call callees and `(` should stay on the call expression.
#[test]
fn test_annotation_call_callee_line_comment_stays_on_call_expression() {
    let source = "{
    call // call-line-marker
    ();
}";
    let (formatter, _) = TestFormatter::parse(source, |p| {
        p.eat_block(destack_ast::BlockContext::Expression)
    })
    .expect("parse call callee line comment source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "call-line-marker")
        .expect("expected call line annotation");
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected call line annotation owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);
    let position = context.annotation(annotation_id).position();

    assert_eq!(owner_node_type, NodeType::Expression);
    assert_eq!(position, AnnotationPosition::LinePostfixBoundary);
}

/// Semicolon-guard own-line comments with indentation should stay on preceding boundaries.
#[test]
fn test_annotation_semicolon_guard_comment_before_parenthesized_call_uses_boundary_position() {
    let source = "declare const PAGE_PATH: string\n  //<- THIS spaces\n;(()=>{})()\n";
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| Ok(p.parse()))
            .expect("parse declaration semicolon guard source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "<- THIS spaces")
        .expect("expected semicolon guard marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected semicolon guard marker owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);

    assert_eq!(position, AnnotationPosition::LinePostfixBoundary);
    assert_eq!(owner_node_type, NodeType::Expression);
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

/// Union-head comments on assignment rhs type expressions should stay block-prefixed.
#[test]
fn test_annotation_type_union_assignment_head_block_comment_is_block_prefix() {
    let source =
        "{\n    const value =\n        /* union-head-marker */\n        input as Left | Right;\n}";
    let (formatter, _) = TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| {
        p.eat_block(destack_ast::BlockContext::Expression)
    })
    .expect("parse union-head assignment comment source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "union-head-marker")
        .expect("expected union-head marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected union-head marker owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);

    assert_eq!(position, AnnotationPosition::BlockPrefix);
    assert_eq!(owner_node_type, NodeType::Expression);
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
    let (formatter, _) = TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| {
        p.eat_block(destack_ast::BlockContext::Expression)
    })
    .expect("parse mapped remap comment source");
    let context = context_from_formatter(&formatter);
    let annotation_id =
        find_annotation_by_marker(&context, "remap-note").expect("expected remap-note annotation");
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected remap owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);
    let position = context.annotation(annotation_id).position();
    assert_eq!(owner_node_type, NodeType::Expression);
    assert_eq!(position, AnnotationPosition::LinePrefix);
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
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

/// Doc comments after `=` attach to assignment rhs values.
#[test]
fn test_assignment_rhs_docs_attach_to_rhs_owner() {
    let source = "x =
/** first-doc */
/** second-doc */
{
    value: 1,
};";
    let (formatter, expressions) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| Ok(p.parse()))
            .expect("parse assignment rhs docs source");
    let context = context_from_formatter(&formatter);

    let first_expression = expressions
        .first()
        .copied()
        .expect("expected one parsed expression");
    let assignment_expression = match context.tree.get(first_expression) {
        Expression::Statement(expression) => *expression,
        _ => first_expression,
    };
    let Expression::Assign { right, .. } = context.tree.get(assignment_expression) else {
        panic!("expected assignment expression");
    };

    let annotation_ids = context
        .annotations(*right)
        .expect("expected rhs annotation ids");
    let doc_count = annotation_ids
        .iter()
        .filter(|annotation_id| {
            matches!(context.annotation(**annotation_id), Annotation::Doc { .. })
        })
        .count();
    assert_eq!(doc_count, 2);
    assert!(context.has_prefix_annotation(*right));
}

/// Doc comments after declarator `=` attach to declarator rhs values.
#[test]
fn test_declarator_rhs_docs_attach_to_rhs_owner() {
    let source = "const issues =
/** first-doc */
/** second-doc */
{
    value: 1,
};";
    let (formatter, expressions) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| Ok(p.parse()))
            .expect("parse declarator rhs docs source");
    let context = context_from_formatter(&formatter);

    let first_expression = expressions
        .first()
        .copied()
        .expect("expected one parsed expression");
    let declaration_expression = match context.tree.get(first_expression) {
        Expression::Statement(expression) => *expression,
        _ => first_expression,
    };
    let Expression::Let { declarators, .. } = context.tree.get(declaration_expression) else {
        panic!("expected let expression");
    };
    let declarator_id = declarators
        .first()
        .copied()
        .expect("expected one declarator");
    let declarator = context.tree.get(declarator_id);
    let value_id = declarator.value.expect("expected declarator value");

    let annotation_ids = context
        .annotations(value_id)
        .expect("expected rhs annotation ids");
    let doc_count = annotation_ids
        .iter()
        .filter(|annotation_id| {
            matches!(context.annotation(**annotation_id), Annotation::Doc { .. })
        })
        .count();
    assert_eq!(doc_count, 2);
    assert!(context.has_prefix_annotation(value_id));
}

/// Complex nested jsdoc declarator seams keep docs on the rhs value owner.
#[test]
fn test_declarator_rhs_jsdoc_nestled_docs_attach_to_rhs_owner() {
    let source = r##"const issues =
/** Trailing comment 1 (not nestled as both comments should be multiline for that) */
/**
 * Trailing comment 2
 */
{
    see:
        /** Trailing nestled comment 1
         *//** Trailing nestled comment 2
         *//** Trailing nestled comment 3
         */
        "#7724 and #12653",
};"##;
    let (formatter, expressions) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| Ok(p.parse()))
            .expect("parse jsdoc nested assignment source");
    let context = context_from_formatter(&formatter);

    let first_expression = expressions
        .first()
        .copied()
        .expect("expected one parsed expression");
    let declaration_expression = match context.tree.get(first_expression) {
        Expression::Statement(expression) => *expression,
        _ => first_expression,
    };
    let Expression::Let { declarators, .. } = context.tree.get(declaration_expression) else {
        panic!("expected let expression");
    };
    let declarator_id = declarators
        .first()
        .copied()
        .expect("expected one declarator");
    let declarator = context.tree.get(declarator_id);
    let value_id = declarator.value.expect("expected declarator value");

    let annotation_ids = context
        .annotations(value_id)
        .expect("expected rhs annotation ids");
    let doc_count = annotation_ids
        .iter()
        .filter(|annotation_id| {
            matches!(context.annotation(**annotation_id), Annotation::Doc { .. })
        })
        .count();
    assert!(doc_count >= 2);
    assert!(context.has_prefix_annotation(value_id));
}

/// Doc comments before assignment-expression declarator values should stay idempotent.
#[test]
fn test_declarator_rhs_assignment_value_docs_are_idempotent() {
    let source = r#"const CONNECTION_STATUS =
/**
 * first-doc
 */
/**
 * second-doc
 */
exports.CONNECTION_STATUS = {
    CLOSED: Object.freeze({ kind: "CLOSED" }),
};"#;

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Fixture trailing jsdocs should stay idempotent.
#[test]
fn test_fixture_trailing_jsdocs_is_idempotent() {
    let source = include_str!(
        "../../../../test/fixtures/formatter/conformance/staging/prettier/tests/format/js/comments/trailing-jsdocs.js"
    );

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
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

/// File-level docs before decorated declarations should keep their declaration ownership.
#[test]
fn test_annotation_file_level_doc_before_decorator_keeps_following_declaration_owner() {
    let source = r#"@description("The status of a Meetup.")
export enum MeetupStatus {
    @description("The Meetup is a draft.")
    Draft,

    @description("The Meetup is upcoming.")
    Upcoming,
}

/// A Series of related Meetups (with a common prefix).
@entity
export class MeetupSeries {
    @description("The name of the Meetup Series.")
    name: string;
}
"#;

    let (formatter, expressions) =
        TestFormatter::parse_with_file_type(source, FileType::Destack, |p| Ok(p.parse()))
            .expect("parse file-level doc ownership source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(
        &context,
        "A Series of related Meetups (with a common prefix).",
    )
    .expect("expected class doc annotation");
    let _ = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected class doc owner node");

    let formatted = formatter.format(
        &statement_list(expressions.as_slice()),
        DestackFormatOptions::default(),
    );
    let expected = r#"@description("The status of a Meetup.")
export enum MeetupStatus {
    @description("The Meetup is a draft.")
    Draft,
    @description("The Meetup is upcoming.")
    Upcoming,
}

/// A Series of related Meetups (with a common prefix).
@entity
export class MeetupSeries {
    @description("The name of the Meetup Series.")
    name: string;
}
"#;
    assert_eq!(formatted, expected);
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

/// Decorator prefixed function return types should stay inline after a colon.
#[test]
fn test_format_function_return_type_decorator_annotation_stays_inline_after_colon() {
    let source =
        "{\n    function build(value: Buffer): @addrspace(\"shared\") &Buffer { return value; }\n}";
    let expected = "{\n    function build(value: Buffer): @addrspace(\"shared\") &Buffer {\n        return value;\n    }\n}";
    assert_format!(
        source,
        expected,
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
    let (formatter, _) = TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| {
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

/// Comments inside short object literals stay inline.
#[test]
fn test_format_comment_in_object() {
    assert_format!(
        "{ /* key */ a: 1, /* another */ b: 2 }",
        "{ /* key */ a: 1, /* another */ b: 2 }",
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
    let (first_formatter, first_block_id) =
        TestFormatter::parse_with_file_type(source, destack_source::FileType::JavaScript, |p| {
            p.eat_block(destack_ast::BlockContext::Expression)
        })
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

/// TypeScript class property decorators should keep `@name` adjacency and stay idempotent.
#[test]
fn test_format_typescript_property_decorators_keep_at_adjacency() {
    let source = r#"@Entity()
export class Board {
    @PrimaryGeneratedColumn()
    id: number;

    @Column()
    slug: string;
}
"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);

    let (first_formatter, first_expressions) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| Ok(p.parse()))
            .expect("parse decorator source");
    let first_output = first_formatter.format(
        &statement_list(first_expressions.as_slice()),
        options.clone(),
    );

    assert!(
        !first_output.contains("@\n"),
        "decorator head split after @:\n{first_output}"
    );

    let (second_formatter, second_expressions) =
        TestFormatter::parse_with_file_type(&first_output, FileType::TypeScript, |p| Ok(p.parse()))
            .expect("parse first pass output");
    let second_output =
        second_formatter.format(&statement_list(second_expressions.as_slice()), options);

    assert_eq!(first_output, second_output);
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

/// Own-line comments before block closers should stay attached to the preceding statement.
#[test]
fn test_format_own_line_comment_before_block_closer_stays_trailing() {
    assert_format!(
        r#"{
    compute();
    // marker-own-line-before-closer
}"#,
        r#"{
    compute();
    // marker-own-line-before-closer
}"#,
        |p| p.eat_block(destack_ast::BlockContext::Expression),
        DestackFormatOptions::default()
    );
}

/// End-of-line comments at statement tails should keep trailing ownership across passes.
#[test]
fn test_format_terminal_end_of_line_comment_stays_trailing() {
    assert_format!(
        r#"{
    const value = 1; // marker-terminal-end-of-line
}"#,
        r#"{
    const value = 1; // marker-terminal-end-of-line
}"#,
        |p| p.eat_block(destack_ast::BlockContext::Expression),
        DestackFormatOptions::default()
    );
}

/// TypeScript mapped-type prettier-ignore seams should stay idempotent.
#[test]
fn test_format_typescript_mapped_type_ignore_directives_are_idempotent() {
    let source = r#"
type a= {
    // prettier-ignore
    [A in B]: C  |  D
  }

type b= {
    [
      // prettier-ignore
      A in B
    ]: C  |  D
  }

type c= {
    [
      A in
      // prettier-ignore
      B
    ]: C  |  D
  }

type d= {
    [A in B]:
      // prettier-ignore
      C  |  D
  }
"#;

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Own-line comments before ASI guard semicolons should stay idempotent.
#[test]
fn test_format_semicolon_guard_own_line_comment_before_asi_array_is_idempotent() {
    let source = r#"{
    let foo = 42

    // keep with guarded statement
    ;[foo] = [1]
}
"#;

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Own-line comments before ASI guard semicolons for parenthesized calls should stay idempotent.
#[test]
fn test_format_semicolon_guard_own_line_comment_before_asi_call_is_idempotent() {
    let source = r#"{
    const fn = (value) => value

    // keep with guarded call
    ;(fn)(1)
}
"#;

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Own-line comments before non-guard semicolon starts should stay idempotent.
#[test]
fn test_format_own_line_comment_before_non_guard_semicolon_is_idempotent() {
    let source = r#"{
    run()

    // keep with following statement
    ;next()
}
"#;

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Inline comments between semicolons and guard heads should stay idempotent.
#[test]
fn test_format_inline_comment_between_semicolon_and_guard_head_is_idempotent() {
    let source = r#"{
    let values = [1]

    ;/* keep guard seam */[values[0]] = [2]
}
"#;

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Template interpolation prefix comments should stay idempotent across passes.
#[test]
fn test_format_template_interpolation_prefix_comments_are_idempotent() {
    let source = r#"`
${/* comment */ /* comment */ c}

${/* comment */ d};
`"#;

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Template interpolation call-member seams with line comments should stay idempotent.
#[test]
fn test_format_template_interpolation_member_comments_are_idempotent() {
    let source = r#"`
${// keep interpolation member seam
XRegExp.union([left, right], "", { conjunction: "or" }).source}
`"#;

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}
