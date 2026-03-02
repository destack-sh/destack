use super::ownership::find_smallest_owner_enclosing_range;
use super::render::annotation_precedes_separator;
use crate::{
    Annotation, DestackFormatArtifacts, DestackFormatContext, DestackFormatOptions, TestFormatter,
    assert_format, assert_format_output_eq, assert_format_program_idempotent_with_file_type,
    assert_format_program_roundtrip_with_file_type, statement_list,
};
use destack_ast::{
    AnnotationPosition, Declaration, DeclarationDescriptor, Expression, LocalNodeId,
    NodeParentIndex, NodeType,
};
use destack_source::{FileType, LanguageType};
use destack_workspace::FormatterOptions;

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

/// Return default formatter options for TypeScript mode.
fn typescript_format_options() -> DestackFormatOptions {
    DestackFormatOptions::from_formatter_options(
        FormatterOptions::default(),
        LanguageType::TypeScript,
    )
}

/// Return default formatter options for JavaScript mode.
fn javascript_format_options() -> DestackFormatOptions {
    DestackFormatOptions::from_formatter_options(
        FormatterOptions::default(),
        LanguageType::JavaScript,
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

/// Find an annotation node by marker fragment in comment or doc text.
fn find_annotation_by_marker_fragment(
    context: &DestackFormatContext<'_>,
    marker: &str,
) -> Option<LocalNodeId<Annotation>> {
    for (entry_index, _) in context.formatter_annotation_entries.iter().enumerate() {
        let annotation_id = LocalNodeId::<Annotation>::new(entry_index as u32);
        let matches_marker = match context.annotation(annotation_id) {
            Annotation::Comment { node, .. } => context.comment_text(node).contains(marker),
            Annotation::Doc { node, .. } => {
                let document = context.tree.get(node);
                context.strings.get(document.string).contains(marker)
            }
            Annotation::Blank { .. } | Annotation::Decorator { .. } => false,
        };
        if matches_marker {
            return Some(annotation_id);
        }
    }

    None
}

/// Find all comment annotation ids by source marker text.
fn find_annotations_by_marker(
    context: &DestackFormatContext<'_>,
    marker: &str,
) -> Vec<LocalNodeId<Annotation>> {
    let mut annotation_ids = Vec::new();

    for (entry_index, _) in context.formatter_annotation_entries.iter().enumerate() {
        let annotation_id = LocalNodeId::<Annotation>::new(entry_index as u32);
        let annotation = context.annotation(annotation_id);
        let matches_marker = matches!(
            annotation,
            Annotation::Comment { node, .. } if context.comment_text(node).trim() == marker
        );
        if matches_marker {
            annotation_ids.push(annotation_id);
        }
    }

    annotation_ids
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

/// Closing-tag seam comments should attach to the tree-expression boundary owner.
#[test]
fn test_annotation_jsx_closing_tag_seam_comments_attach_to_tree_expression_boundary() {
    let source = r#"
const beforeSlash = <a><// jsx-before-slash-line
/a>;
const beforeSlashBlock = <a></* jsx-before-slash-block */
/a>;
const afterSlash = <a></ // jsx-after-slash-line
a>;
const afterSlashBlock = <a></ /* jsx-after-slash-block */
a>;
const afterName = <a></a // jsx-after-name-line
>;
const afterNameBlock = <a></a /* jsx-after-name-block */
>;
const fragmentAfterSlash = <></ // jsx-fragment-after-slash-line
>;
const fragmentAfterSlashBlock = <></ /* jsx-fragment-after-slash-block */
>;
"#;
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScriptXml, |p| Ok(p.parse()))
            .expect("parse jsx closing-tag seam source");
    let context = context_from_formatter(&formatter);

    for (marker, expected_position) in [
        (
            "jsx-before-slash-line",
            AnnotationPosition::LinePostfixBoundary,
        ),
        ("jsx-before-slash-block", AnnotationPosition::BlockPostfix),
        (
            "jsx-after-slash-line",
            AnnotationPosition::LinePostfixBoundary,
        ),
        ("jsx-after-slash-block", AnnotationPosition::BlockPostfix),
        (
            "jsx-after-name-line",
            AnnotationPosition::LinePostfixBoundary,
        ),
        ("jsx-after-name-block", AnnotationPosition::BlockPostfix),
        (
            "jsx-fragment-after-slash-line",
            AnnotationPosition::LinePostfixBoundary,
        ),
        (
            "jsx-fragment-after-slash-block",
            AnnotationPosition::BlockPostfix,
        ),
    ] {
        let annotation_id =
            find_annotation_by_marker(&context, marker).expect("expected closing-tag annotation");
        let annotation = context.annotation(annotation_id);
        let position = annotation.position();
        let owner_node = find_annotation_target_owner_node(&context, annotation_id)
            .expect("expected closing-tag annotation owner node");
        let owner_node_id = owner_node as u32;

        assert_eq!(
            position, expected_position,
            "marker={marker}, owner={owner_node}"
        );
        assert_eq!(
            context.tree.get_node_type(owner_node_id),
            NodeType::Expression,
            "marker={marker}, owner={owner_node}"
        );
        assert!(
            matches!(
                context
                    .tree
                    .get(LocalNodeId::<Expression>::new(owner_node_id)),
                Expression::TreeExpression { .. }
            ),
            "marker={marker}, owner={owner_node}"
        );
    }
}

/// Parenthesized assignment JSX closing-tag seam comments should stay on the tree-expression owner.
#[test]
fn test_annotation_jsx_closing_tag_seam_comments_stay_on_parenthesized_assignment_tree_owner() {
    let source = r#"
const element = (
    <a></ // jsx-element-after-slash
a>
);
const fragment = (
    <></ // jsx-fragment-after-slash
>
);
"#;
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScriptXml, |p| Ok(p.parse()))
            .expect("parse parenthesized jsx closing-tag seam source");
    let context = context_from_formatter(&formatter);

    for marker in ["jsx-element-after-slash", "jsx-fragment-after-slash"] {
        let annotation_id =
            find_annotation_by_marker(&context, marker).expect("expected closing-tag annotation");
        let owner_node = find_annotation_target_owner_node(&context, annotation_id)
            .expect("expected closing-tag annotation owner node");
        let owner_node_id = owner_node as u32;

        assert_eq!(
            context.tree.get_node_type(owner_node_id),
            NodeType::Expression,
            "marker={marker}, owner={owner_node}"
        );
        assert!(
            matches!(
                context
                    .tree
                    .get(LocalNodeId::<Expression>::new(owner_node_id)),
                Expression::TreeExpression { .. }
            ),
            "marker={marker}, owner={owner_node}"
        );
    }
}

/// Own-line slash comments after closing-tag block seams should stay idempotent.
#[test]
fn test_format_jsx_closing_tag_block_then_line_comment_is_idempotent() {
    let source = r#"
<a></a>;
/* block */
// line
<a></a>;
"#;

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScriptXml,
        DestackFormatOptions::default(),
    );
}

/// Closing-tag slash seam line comments should remain stable across format passes.
#[test]
fn test_format_jsx_closing_tag_after_slash_line_comments_are_idempotent() {
    let source = r#"
<a></ // element-close
a>;
<></ // fragment-close
>;
"#;

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScriptXml,
        DestackFormatOptions::default(),
    );
}

/// Closing-tag slash seam comments in assignment expressions should stay idempotent.
#[test]
fn test_format_jsx_closing_tag_after_slash_line_comments_in_assignments_are_idempotent() {
    let source = r#"
const element = <a></ // element-close
a>;
const fragment = <></ // fragment-close
>;
"#;

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScriptXml,
        DestackFormatOptions::default(),
    );
}

/// Heading block comments after line-postfix JSX statements should keep blank-line stability.
#[test]
fn test_format_jsx_line_postfix_then_heading_block_comment_is_idempotent() {
    let source = r#"
<></>;
<></>; // line

/* heading */
<a></a>;
"#;

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScriptXml,
        DestackFormatOptions::default(),
    );
}

/// Prettier in-end-tag JSX comment shapes should remain idempotent.
#[test]
fn test_format_jsx_closing_tag_prettier_in_end_tag_fixture_is_idempotent() {
    let source = r#"
/* =========== before slash =========== */
<a><// line
/a>;
<a></* block */
/a>;

<><// line
/>;
<></* block */
/>;

/* =========== after slash =========== */
<a></ // line
a>;
<a></ /* block */
a>;

<></ // line
>;
<></ /* block */
>;

/* =========== after name =========== */
<a></a // line
>;
<a></a /* block */
>;


/* =========== block =========== */
<a></a /* block */>;
<></ /* block */>;

/* =========== multiple ===========  */
<a><// line 1
// line 2
/a>;
<a></* block1 */ /* block2 */
/a>;
<a></* block */ // line
/a>;

<><// line 1
// line 2
/>;
<></* block1 */ /* block2 */
/>;
<></* block */ // line
/>;"#;

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScriptXml,
        DestackFormatOptions::default(),
    );
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

    assert!(matches!(
        position,
        AnnotationPosition::LinePostfixBoundary | AnnotationPosition::BlockPrefix
    ));
    assert_eq!(owner_node_type, NodeType::Argument);
}

/// Own-line comments before array elements should attach to following element prefixes.
#[test]
fn test_annotation_array_element_own_line_comment_attaches_to_following_element() {
    let source = "{
    const rows = [
        // array-element-prefix-marker-a
        [1],
        // array-element-prefix-marker-b
        [2],
    ];
}";
    let (formatter, _) = TestFormatter::parse(source, |p| {
        p.eat_block(destack_ast::BlockContext::Expression)
    })
    .expect("parse own-line array element comment source");
    let context = context_from_formatter(&formatter);

    for marker in [
        "array-element-prefix-marker-a",
        "array-element-prefix-marker-b",
    ] {
        let annotation_id =
            find_annotation_by_marker(&context, marker).expect("expected array element marker");
        let position = context.annotation(annotation_id).position();
        let owner_node = find_annotation_target_owner_node(&context, annotation_id)
            .expect("expected array element marker owner node");
        let owner_node_type = context.tree.get_node_type(owner_node as u32);

        assert_eq!(
            position,
            AnnotationPosition::LinePrefix,
            "marker={marker}, owner={owner_node}, owner_type={owner_node_type:?}"
        );
        assert!(
            matches!(owner_node_type, NodeType::Argument | NodeType::Expression),
            "marker={marker}, owner={owner_node}, owner_type={owner_node_type:?}"
        );
    }
}

/// Own-line comments inside empty array literals should stay on the array as infix annotations.
#[test]
fn test_annotation_empty_array_own_line_comment_attaches_as_array_infix() {
    let source = r#"expect(() => {}).toTriggerReadyStateChanges([
    // empty-array-dangling-marker
]);
"#;
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| Ok(p.parse()))
            .expect("parse empty array dangling comment source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "empty-array-dangling-marker")
        .expect("expected empty array dangling marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected empty array dangling marker owner node");
    let owner_node_id = owner_node as u32;

    assert_eq!(position, AnnotationPosition::BlockInfix);
    assert_eq!(
        context.tree.get_node_type(owner_node_id),
        NodeType::Expression,
        "expected expression owner for empty array dangling comment, got node {owner_node}"
    );
    assert!(
        matches!(
            context
                .tree
                .get(LocalNodeId::<Expression>::new(owner_node_id)),
            Expression::ArrayExpression { .. }
        ),
        "expected array expression owner for empty array dangling comment, got {owner_node}"
    );
}

/// Own-line comments inside empty call-argument arrays should keep one stable idempotent shape.
#[test]
fn test_format_empty_array_own_line_comment_in_call_argument_is_idempotent() {
    let source = r#"expect(() => {}).toTriggerReadyStateChanges([
  // Nothing.
]);

[1 /* first comment */, 2 /* second comment */, 3];
"#;
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        javascript_format_options(),
    );
}

/// Inline comments between closing delimiters and semicolons should stay on the preceding boundary.
#[test]
fn test_annotation_inline_comment_between_closing_paren_and_semicolon_is_boundary_postfix() {
    let source = "const value = !(() => 3) /* closing-paren-semicolon-marker */;";
    let (formatter, roots) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| Ok(p.parse()))
            .expect("parse closing-paren semicolon marker source");
    let context = context_from_formatter(&formatter);
    let _ = roots;

    let annotation_id = find_annotation_by_marker(&context, "closing-paren-semicolon-marker")
        .expect("expected closing-paren semicolon marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node_ids = context
        .formatter_annotation_ids_by_node_id
        .iter()
        .enumerate()
        .filter_map(|(node_index, annotation_ids)| {
            annotation_ids
                .iter()
                .any(|candidate| candidate.id == annotation_id.id)
                .then_some(node_index as u32)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        owner_node_ids.len(),
        1,
        "expected one owner for closing-paren semicolon marker, got {owner_node_ids:?}"
    );
    let owner_node_id = owner_node_ids[0];
    let owner_node_type = context.tree.get_node_type(owner_node_id);

    assert_eq!(
        position,
        AnnotationPosition::LinePostfixBoundary,
        "marker owner id={owner_node_id} owner type={owner_node_type:?}"
    );
    assert_eq!(owner_node_type, NodeType::Expression);
}

/// Own-line comments before list elements should stay as line prefixes in mixed list layouts.
#[test]
fn test_annotation_mixed_array_element_own_line_comments_attach_as_line_prefix() {
    let source = "{
    const test_cases = [
        // mixed-array-prefix-marker-a
        [\"--check\", \"!**/err.js\"],
        // mixed-array-prefix-marker-b
        [\"--check\", \"--ignore-path\", \"ignore1\"],
        // mixed-array-prefix-marker-c
        [\"--check\", \"--ignore-path\", \"ignore1\", \"should_format/ok.js\"],
        // mixed-array-prefix-marker-d
        [
            \"--check\",
            \"--ignore-path\",
            \"ignore1\",
            \"should_format/ok.js\",
            \"--no-error-on-unmatched-pattern\",
        ],
    ];
}";
    let (formatter, _) = TestFormatter::parse(source, |p| {
        p.eat_block(destack_ast::BlockContext::Expression)
    })
    .expect("parse mixed array element own-line comments source");
    let context = context_from_formatter(&formatter);

    for marker in [
        "mixed-array-prefix-marker-a",
        "mixed-array-prefix-marker-b",
        "mixed-array-prefix-marker-c",
        "mixed-array-prefix-marker-d",
    ] {
        let annotation_id =
            find_annotation_by_marker(&context, marker).expect("expected mixed array marker");
        let position = context.annotation(annotation_id).position();
        let owner_node = find_annotation_target_owner_node(&context, annotation_id)
            .expect("expected mixed array marker owner node");
        let owner_node_type = context.tree.get_node_type(owner_node as u32);

        assert_eq!(
            position,
            AnnotationPosition::LinePrefix,
            "marker={marker}, owner={owner_node}, owner_type={owner_node_type:?}"
        );
    }
}

/// Optional-call seam line comments should attach once.
#[test]
fn test_annotation_optional_call_line_boundary_comment_attaches_once() {
    let source = "const value = call // keep-line
?.()";
    let (formatter, _) = TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| {
        p.eat_expression(Default::default())
    })
    .expect("parse optional-call boundary comment source");
    let context = context_from_formatter(&formatter);

    let annotation_ids = find_annotations_by_marker(&context, "keep-line");
    assert_eq!(
        annotation_ids.len(),
        1,
        "optional-call line boundary comment should attach exactly once",
    );

    let annotation_id = annotation_ids[0];
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("optional-call boundary comment should have one owner");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);
    let owner_expression_kind = if owner_node_type == NodeType::Expression {
        let owner_expression_id = LocalNodeId::<Expression>::new(owner_node as u32);
        match context.tree.get(owner_expression_id) {
            Expression::Path { .. } => "Path",
            Expression::Maybe { .. } => "Maybe",
            Expression::Call { .. } => "Call",
            Expression::Member { .. } => "Member",
            _ => "Other",
        }
    } else {
        "NonExpression"
    };
    let owner_annotation_count = context.formatter_annotation_ids_by_node_id[owner_node]
        .iter()
        .filter(|candidate_id| candidate_id.id == annotation_id.id)
        .count();
    assert_eq!(
        owner_annotation_count, 1,
        "optional-call boundary comment should appear once in the owner annotation list",
    );
    assert_eq!(
        owner_expression_kind, "Call",
        "optional-call line boundary comment owner kind should be Call, found {owner_expression_kind}",
    );

    let total_owner_occurrences = context
        .formatter_annotation_ids_by_node_id
        .iter()
        .flat_map(|annotation_ids| annotation_ids.iter())
        .filter(|candidate_id| candidate_id.id == annotation_id.id)
        .count();
    assert_eq!(
        total_owner_occurrences, 1,
        "optional-call boundary comment should not be attached to multiple owners",
    );
}

/// Declarator seam line comments after `=` should stay inline with the rhs seam.
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
    assert_eq!(
        owner_node_type,
        NodeType::Expression,
        "position={position:?}, owner={owner_node}"
    );
}

/// Type declaration head comments before `=` should attach to the declaration value.
#[test]
fn test_annotation_type_declaration_value_seam_line_comment_attaches_to_rhs_value() {
    let source = "type UploadState<E, EM, D>
  // type-value-seam-marker
  = A | B;";
    let expected = "type UploadState<E, EM, D> =
    // type-value-seam-marker
    A | B;
";
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| Ok(p.parse()))
            .expect("parse type declaration value seam source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "type-value-seam-marker")
        .expect("expected type value seam marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected type value seam marker owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);

    assert_eq!(position, AnnotationPosition::LinePrefix);
    assert_eq!(
        owner_node_type,
        NodeType::Expression,
        "position={position:?}, owner={owner_node}"
    );
    assert_format_program_roundtrip_with_file_type(
        source,
        expected,
        FileType::TypeScript,
        typescript_format_options(),
    );
}

/// Type declaration comments after `=` should stay attached to the value expression.
#[test]
fn test_annotation_type_declaration_assignment_line_comment_attaches_to_rhs_value() {
    let source = "type UploadState<E, EM, D> =
// type-assignment-seam-marker
| A
| B;";
    let expected = "type UploadState<E, EM, D> =
    // type-assignment-seam-marker
    A | B;
";
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| Ok(p.parse()))
            .expect("parse type declaration assignment seam source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "type-assignment-seam-marker")
        .expect("expected type assignment seam marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected type assignment seam marker owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);

    assert_eq!(position, AnnotationPosition::BlockPrefix);
    assert_eq!(owner_node_type, NodeType::Expression);
    assert_format_program_roundtrip_with_file_type(
        source,
        expected,
        FileType::TypeScript,
        typescript_format_options(),
    );
}

/// Type value-seam comments should stay on their own declaration across omitted-semicolon boundaries.
#[test]
fn test_annotation_type_declaration_value_seam_keeps_local_declaration_ownership() {
    let source = r#"type First =
  // first-type-value-seam-marker
  FirstValue

type Second =
  // second-type-value-seam-marker
  SecondValue
"#;
    let expected = r#"type First =
    // first-type-value-seam-marker
    FirstValue;

type Second =
    // second-type-value-seam-marker
    SecondValue;
"#;
    assert_format_program_roundtrip_with_file_type(
        source,
        expected,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
    assert_format_program_roundtrip_with_file_type(
        source,
        expected,
        FileType::TypeScript,
        typescript_format_options(),
    );
}

/// Type parameter seam comments before closers should keep separator comments on the comma seam.
#[test]
fn test_annotation_formats_type_parameter_comment_with_virtual_trailing_comma() {
    let source = r#"interface Foo {
    <
        A // type-parameter-seam-marker
    >(arg): any;
}"#;
    let expected = r#"interface Foo {
    <
        A, // type-parameter-seam-marker
    >(arg): any;
}
"#;
    let options = DestackFormatOptions::default();

    assert_format_program_roundtrip_with_file_type(source, expected, FileType::TypeScript, options);
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

/// Import alias comments before separator commas should stay on dependency-item boundaries.
#[test]
fn test_annotation_import_alias_comment_before_separator_comma_attaches_to_dependency_item() {
    let source = "import {
  a as b // import-alias-separator-marker
  ,
} from \"pkg\";";
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| Ok(p.parse()))
            .expect("parse import alias separator marker source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "import-alias-separator-marker")
        .expect("expected import alias separator marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected import alias separator marker owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);
    let next_token_type = context.annotation_next_non_whitespace_token_type(annotation_id);

    assert_eq!(owner_node_type, NodeType::DependencyItem);
    assert_eq!(position, AnnotationPosition::LinePostfixBoundary);
    assert_eq!(next_token_type, Some(destack_ast::TokenType::Comma));
}

/// Import block comments before separator commas should stay on dependency-item boundaries.
#[test]
fn test_annotation_import_block_comment_before_separator_comma_attaches_to_dependency_item() {
    let source = "import {
  SelectionSetNode /* import-block-separator-marker */
  ,
} from \"graphql\";";
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| Ok(p.parse()))
            .expect("parse import block separator marker source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "import-block-separator-marker")
        .expect("expected import block separator marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected import block separator marker owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);
    let next_token_type = context.annotation_next_non_whitespace_token_type(annotation_id);

    assert_eq!(owner_node_type, NodeType::DependencyItem);
    assert_eq!(position, AnnotationPosition::LinePostfixBoundary);
    assert_eq!(next_token_type, Some(destack_ast::TokenType::Comma));
}

/// Own-line block comments before `)` in parameter lists should stay on parameter boundaries.
#[test]
fn test_annotation_parameter_own_line_block_comment_before_close_paren_attaches_to_parameter() {
    let source = "function f(
  value: T
  /* parameter-separator-block-marker */
): R {}";
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| Ok(p.parse()))
            .expect("parse parameter separator block marker source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "parameter-separator-block-marker")
        .expect("expected parameter separator block marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected parameter separator block marker owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);

    assert_eq!(owner_node_type, NodeType::Parameter);
    assert_eq!(position, AnnotationPosition::LinePostfixBoundary);
}

/// Multiline own-line block comments before `)` in parameter lists should stay on parameter boundaries.
#[test]
fn test_annotation_parameter_multiline_own_line_block_comment_before_close_paren_attaches_to_parameter()
 {
    let source = r#"function f(
  value: T
  /* parameter-separator-multiline-block-marker
   * marker-body
   */
): R {}"#;
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| Ok(p.parse()))
            .expect("parse multiline parameter separator block marker source");
    let context = context_from_formatter(&formatter);

    let annotation_id =
        find_annotation_by_marker_fragment(&context, "parameter-separator-multiline-block-marker")
            .expect("expected multiline parameter separator block marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected multiline parameter separator block marker owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);

    assert_eq!(owner_node_type, NodeType::Parameter);
    assert_eq!(position, AnnotationPosition::LinePostfixBoundary);
}

/// Method parameters with default values should keep own-line separator block comments on boundaries.
#[test]
fn test_annotation_method_parameter_default_own_line_block_comment_before_close_paren_attaches_to_parameter()
 {
    let source = r#"class X {
  getSectionMode(
    pageMetaData: PageMetaData,
    sectionMetaData: SectionMetaData = ["unknown"]
    /* method-parameter-default-separator-block-marker
     * marker-body
     */
  ): R {}
}"#;
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| Ok(p.parse()))
            .expect("parse method parameter separator block marker source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker_fragment(
        &context,
        "method-parameter-default-separator-block-marker",
    )
    .expect("expected method parameter separator block marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected method parameter separator block marker owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);

    assert_eq!(owner_node_type, NodeType::Parameter);
    assert_eq!(position, AnnotationPosition::LinePostfixBoundary);
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

/// Heritage-head comments after declaration type parameters should stay on declaration heads.
#[test]
fn test_annotation_type_parameter_head_comment_before_extends_attaches_to_declaration() {
    let source = r#"interface ReallyReallyLongName<
  TypeArgumentNumberOne,
  TypeArgumentNumberTwo,
  TypeArgumentNumberThree
> // heritage-head-seam-marker
extends BaseInterface {}"#;
    let expected = r#"interface ReallyReallyLongName<
    TypeArgumentNumberOne,
    TypeArgumentNumberTwo,
    TypeArgumentNumberThree,
> // heritage-head-seam-marker
    extends BaseInterface {}
"#;
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| Ok(p.parse()))
            .expect("parse declaration heritage head seam source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "heritage-head-seam-marker")
        .expect("expected declaration heritage head seam marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected declaration heritage head seam marker owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);

    assert_eq!(position, AnnotationPosition::LinePrefix);
    assert_eq!(owner_node_type, NodeType::Declaration);
    assert_format_program_roundtrip_with_file_type(
        source,
        expected,
        FileType::TypeScript,
        typescript_format_options(),
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

/// Line comments between call callees and argument parentheses should normalize to call boundaries.
#[test]
fn test_format_call_callee_line_comment_before_empty_arguments_is_boundary_postfix() {
    let source = "call // call-tail-marker
()
call // optional-call-tail-marker
?.()
";
    let expected = "call(); // call-tail-marker
call?.(); // optional-call-tail-marker
";

    assert_format_program_roundtrip_with_file_type(
        source,
        expected,
        FileType::JavaScript,
        javascript_format_options(),
    );
}

/// Line comments between instantiation callees and `(` should stay stable across non-empty and empty calls.
#[test]
fn test_format_instantiation_callee_line_comment_before_arguments_is_idempotent() {
    let source = r#"foo<string>// instantiation-call-marker-13
(1)
foo<string>// instantiation-call-marker-23
()
"#;
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        typescript_format_options(),
    );
}

/// Type-argument close-to-call line comments should follow call-boundary and argument-prefix ownership.
#[test]
fn test_format_call_type_argument_close_line_comments_are_stable() {
    let source = r#"foo<string>// marker-13
(1)
foo<string>
// marker-14
(1)
foo<string>// marker-23
()
foo<string>
// marker-24
()
"#;
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        typescript_format_options(),
    );
}

/// Block comments before newline-prefixed call type arguments should keep one stable call shape.
#[test]
fn test_format_call_newline_block_comment_before_type_arguments_is_stable() {
    let source = r#"foo/* marker-31 */
<string>(1)
foo/* marker-41 */
<string>()
"#;
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        typescript_format_options(),
    );
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

/// Own-line comments between control heads and empty-statement semicolons stay on statement seams.
#[test]
fn test_annotation_control_head_comment_before_empty_statement_semicolon_stays_on_boundary() {
    let source = "if (a)\n// control-head-marker\n;\n";
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| Ok(p.parse()))
            .expect("parse control-head empty statement source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "control-head-marker")
        .expect("expected control-head marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected control-head marker owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);

    assert_eq!(position, AnnotationPosition::LinePostfixBoundary);
    assert_eq!(owner_node_type, NodeType::Expression);
}

/// Inline control-head comments before empty-statement semicolons should stay idempotent.
#[test]
fn test_format_control_head_inline_comment_before_empty_statement_semicolon_is_idempotent() {
    let source = r#"for(;;) // marker-14
;
if(a) // marker-15
;
while(a) // marker-16
;
"#;

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        javascript_format_options(),
    );
}

/// Own-line comments before semicolon-guarded array heads after `do...while` stay on the guarded expression.
#[test]
fn test_annotation_do_while_semicolon_guard_comment_stays_on_guarded_expression() {
    let source = "do;while(1)\n\n// do-while-guard-marker\n;[]\n";
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| Ok(p.parse()))
            .expect("parse do-while semicolon guard source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "do-while-guard-marker")
        .expect("expected do-while guard marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected do-while guard marker owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);

    assert_eq!(position, AnnotationPosition::BlockPrefix);
    assert_eq!(owner_node_type, NodeType::Expression);
}

/// Standalone semicolon tail comments should stay as leading comments of the following statement.
#[test]
fn test_format_line_leading_semicolon_tail_comment_stays_with_following_statement() {
    let source = r#"function x() {
} // first
; // second
const y = 1;
"#;
    let expected = r#"function x() {} // first
// second
const y = 1;
"#;

    assert_format_program_roundtrip_with_file_type(
        source,
        expected,
        FileType::JavaScript,
        javascript_format_options(),
    );
}

/// Standalone semicolon block-tail comments should stay as leading comments of the following statement.
#[test]
fn test_format_line_leading_semicolon_block_tail_comment_stays_with_following_statement() {
    let source = r#"a;
; /* marker */
foo();
"#;
    let expected = r#"a;
/* marker */
foo();
"#;

    assert_format_program_roundtrip_with_file_type(
        source,
        expected,
        FileType::JavaScript,
        javascript_format_options(),
    );
}

/// Trailing block comments after import semicolons should stay on the import statement boundary.
#[test]
fn test_annotation_import_trailing_block_comment_after_semicolon_uses_boundary_position() {
    let source = "import \"a\"; /* import-tail-marker */\n";
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| Ok(p.parse()))
            .expect("parse import trailing block comment source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "import-tail-marker")
        .expect("expected import trailing block comment marker");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected import trailing block comment owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);

    assert_eq!(position, AnnotationPosition::LinePostfixBoundary);
    assert_eq!(owner_node_type, NodeType::Expression);
}

/// JavaScript import tails should keep one space before inline block postfix comments.
#[test]
fn test_format_import_trailing_block_comment_after_semicolon_keeps_space_javascript() {
    let source = "import \"a\";/* import-tail-marker */\n";
    let expected = "import \"a\"; /* import-tail-marker */\n";
    assert_format_program_roundtrip_with_file_type(
        source,
        expected,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// TypeScript import tails should keep one space before inline block postfix comments.
#[test]
fn test_format_import_trailing_block_comment_after_semicolon_keeps_space_typescript() {
    let source = "import type {} from \"a\";/* import-type-tail-marker */\n";
    let expected = "import type {} from \"a\"; /* import-type-tail-marker */\n";
    assert_format_program_roundtrip_with_file_type(
        source,
        expected,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Empty import-assertion comments should stay idempotent when lowered to postfix tails.
#[test]
fn test_format_import_assertion_empty_comment_tail_is_idempotent() {
    let source = "export * as baz from \"baz.json\" assert { /* comment */ }\nimport * as baz from \"baz.json\" assert { /* comment */ }\n";
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
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

/// Type alias trailing line comments should stay on the union value expression seam.
#[test]
fn test_annotation_type_alias_union_trailing_line_comment_stays_on_expression() {
    let source = r#"
type Value =
  | First
  | Second // second-tail
"#;
    let (formatter, expressions) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| Ok(p.parse()))
            .expect("parse trailing type alias comment source");
    let [first_expression] = expressions.as_slice() else {
        panic!("expected one top-level expression");
    };
    let Expression::Declaration(declaration_id) = formatter.tree.get(*first_expression) else {
        panic!("expected declaration expression");
    };
    let Declaration::Type { value, .. } = formatter.tree.get(*declaration_id) else {
        panic!("expected type declaration");
    };
    let _ = formatter.tree.get(*value);

    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "second-tail")
        .expect("expected trailing type alias marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected trailing type alias annotation owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);

    assert_eq!(position, AnnotationPosition::LinePostfixBoundary);
    assert_eq!(owner_node_type, NodeType::Expression);
}

/// Union fixture C1 and C2 comments should keep stable annotation metadata.
#[test]
fn test_annotation_union_c1_c2_comment_attachment_metadata() {
    let source = r#"
type C1 = | (
  /* c1a */ /* c1b */ | (
    | (
          | A
          // c1-tail
          | B
        )
  )
  );

type C2 = | (
  /* c2a */ /* c2b */
  /* c2c */ | (
    | (
          | A
          // c2-tail
          | B
        )
  )
  );
"#;
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| Ok(p.parse()))
            .expect("parse c1 c2 metadata source");
    let context = context_from_formatter(&formatter);

    let expected_positions = [
        ("c1a", AnnotationPosition::LinePrefix),
        ("c1b", AnnotationPosition::LinePrefix),
        ("c1-tail", AnnotationPosition::LinePostfixBoundary),
        ("c2a", AnnotationPosition::BlockPrefix),
        ("c2b", AnnotationPosition::BlockPrefix),
        ("c2c", AnnotationPosition::LinePrefix),
        ("c2-tail", AnnotationPosition::LinePostfixBoundary),
    ];

    for (marker, expected_position) in expected_positions {
        let annotation_id = find_annotation_by_marker(&context, marker)
            .unwrap_or_else(|| panic!("expected marker annotation: {marker}"));
        let position = context.annotation(annotation_id).position();
        let owner_node = find_annotation_target_owner_node(&context, annotation_id)
            .unwrap_or_else(|| panic!("expected owner for marker annotation: {marker}"));
        let owner_node_type = context.tree.get_node_type(owner_node as u32);
        let next_token = context.annotation_next_non_whitespace_token_type(annotation_id);
        assert_eq!(owner_node_type, NodeType::Expression, "marker={marker}");
        assert_eq!(position, expected_position, "marker={marker}");
        assert_eq!(next_token, Some(destack_ast::TokenType::ElementwiseOr));
    }
}

/// Conformance C2 comment before `|` should stay line-prefix even with trailing spaces.
#[test]
fn test_annotation_union_c2_comment_before_pipe_stays_line_prefix() {
    let source = r#"
type C2 = | (
  /* c2a */ /* c2b */ 
  /* c2c */ | (
    | (
          | A
          // c2-tail
          | B
        )
  )
  );
"#;
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| Ok(p.parse()))
            .expect("parse c2 line-prefix source");
    let context = context_from_formatter(&formatter);
    let annotation_id =
        find_annotation_by_marker(&context, "c2c").expect("expected c2c annotation");
    let position = context.annotation(annotation_id).position();
    let next_token = context.annotation_next_non_whitespace_token_type(annotation_id);
    assert_eq!(position, AnnotationPosition::LinePrefix);
    assert_eq!(next_token, Some(destack_ast::TokenType::ElementwiseOr));
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

/// Own-line comments after `as` should attach to the rhs type seam.
#[test]
fn test_annotation_type_binary_own_line_comment_after_as_attaches_to_rhs() {
    let source = "{\n    value = a as\n        // own-line-as-marker\n        Foo | Bar;\n}";
    let (formatter, _) = TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| {
        p.eat_block(destack_ast::BlockContext::Expression)
    })
    .expect("parse own-line as comment source");
    let context = context_from_formatter(&formatter);
    let annotation_id = find_annotation_by_marker(&context, "own-line-as-marker")
        .expect("expected own-line as marker annotation");
    let position = context.annotation(annotation_id).position();
    let next_token = context.annotation_next_non_whitespace_token_type(annotation_id);
    assert_eq!(
        position,
        AnnotationPosition::BlockPrefix,
        "position={position:?}, next_token={next_token:?}"
    );
}

/// Own-line comments after `as` in long unions should keep rhs-leading ownership.
#[test]
fn test_annotation_type_binary_own_line_comment_after_as_with_union_attaches_to_rhs() {
    let source = "{\n    functionArg = a as\n        // own-line-as-union-marker\n        TSESTree.ArrowFunctionExpression | TSESTree.ArrowFunctionExpression | TSESTree.FunctionExpression | undefined;\n}";
    let (formatter, _) = TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| {
        p.eat_block(destack_ast::BlockContext::Expression)
    })
    .expect("parse own-line as union comment source");
    let context = context_from_formatter(&formatter);
    let annotation_id = find_annotation_by_marker(&context, "own-line-as-union-marker")
        .expect("expected own-line as union marker annotation");
    let position = context.annotation(annotation_id).position();
    let next_token = context.annotation_next_non_whitespace_token_type(annotation_id);
    assert_eq!(
        position,
        AnnotationPosition::BlockPrefix,
        "position={position:?}, next_token={next_token:?}"
    );
}

/// Re-parsed output should keep own-line `as` seam comments as rhs-leading annotations.
#[test]
fn test_annotation_type_binary_own_line_comment_after_as_with_union_reparse_keeps_rhs() {
    let source = "{\n    functionArg = a as\n        // own-line-as-union-reparse-marker\n        TSESTree.ArrowFunctionExpression | TSESTree.ArrowFunctionExpression | TSESTree.FunctionExpression | undefined;\n}";
    let (formatter, block_id) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| {
            p.eat_block(destack_ast::BlockContext::Expression)
        })
        .expect("parse own-line as union reparse source");
    let formatted = formatter.format(&block_id, typescript_format_options());

    let (reparsed_formatter, _) =
        TestFormatter::parse_with_file_type(&formatted, FileType::TypeScript, |p| {
            p.eat_block(destack_ast::BlockContext::Expression)
        })
        .expect("reparse own-line as union reparse formatted source");
    let reparsed_context = context_from_formatter(&reparsed_formatter);
    let annotation_id =
        find_annotation_by_marker(&reparsed_context, "own-line-as-union-reparse-marker")
            .expect("expected own-line as union reparse marker annotation");
    let position = reparsed_context.annotation(annotation_id).position();
    let next_token = reparsed_context.annotation_next_non_whitespace_token_type(annotation_id);
    assert_eq!(
        position,
        AnnotationPosition::BlockPrefix,
        "position={position:?}, next_token={next_token:?}, formatted={formatted}"
    );
}

/// Parenthesized union trailing arm comments should stay attached to the trailing arm.
#[test]
fn test_annotation_parenthesized_union_trailing_arm_comment_stays_on_arm() {
    let source = "type Result = (
  | \"a\" // arm-a
  | \"b\" // arm-b
)[]; // final-tail
";
    let expected = "type Result = (
    | \"a\" // arm-a
    | \"b\" // arm-b
)[]; // final-tail
";
    let (formatter, expressions) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| Ok(p.parse()))
            .expect("parse parenthesized union comment source");
    let context = context_from_formatter(&formatter);

    let annotation_id =
        find_annotation_by_marker(&context, "arm-b").expect("expected arm-b annotation");
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected arm-b owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);
    let position = context.annotation(annotation_id).position();
    let owner_expression = if owner_node_type == NodeType::Expression {
        Some(
            context
                .tree
                .get(LocalNodeId::<Expression>::new(owner_node as u32)),
        )
    } else {
        None
    };

    assert_eq!(
        owner_node_type,
        NodeType::Expression,
        "owner={owner_node}, position={position:?}"
    );
    assert!(
        matches!(owner_expression, Some(Expression::ScalarLiteral(_))),
        "owner={owner_node}, owner_expression={owner_expression:?}, position={position:?}"
    );
    assert_eq!(position, AnnotationPosition::LinePostfixBoundary);

    let formatted = formatter.format(
        &statement_list(expressions.as_slice()),
        typescript_format_options(),
    );
    assert_format_output_eq(expected, &formatted);
}

/// TSX alternate branch block comments should stay inside the alternate branch body.
#[test]
fn test_annotation_tsx_alternate_block_comment_stays_in_alternate_branch() {
    let source = "const Component = () => (
  <div>
    {\"error\" ? (
      <Error />
    ) : (
      <Success />
      /* keep-inside-branch */
    )}
  </div>
)
";
    let expected = "const Component = () => (
    <div>
        {\"error\" ? (
            <Error />
        ) : (
            <Success />
            /* keep-inside-branch */
        )}
    </div>
);
";

    let (formatter, expressions) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScriptXml, |p| Ok(p.parse()))
            .expect("parse tsx alternate block comment source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "keep-inside-branch")
        .expect("expected tsx alternate block annotation");
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected tsx alternate block owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);
    let position = context.annotation(annotation_id).position();

    assert_eq!(
        owner_node_type,
        NodeType::Expression,
        "owner={owner_node}, position={position:?}"
    );
    assert_eq!(position, AnnotationPosition::BlockPostfix);

    let formatted = formatter.format(
        &statement_list(expressions.as_slice()),
        DestackFormatOptions::default(),
    );
    assert_format_output_eq(expected, &formatted);
}

/// Neighboring closure-cast comments should preserve source order.
#[test]
fn test_annotation_closure_cast_neighboring_block_comment_order() {
    let source = "(/* 2 */ /** @type {{bar: string[]}} */ {}).bar.forEach(doStuff)
(/** @type {{bar: string[]}} */ /* 2 */ {}).bar.forEach(doStuff)
";
    let expected = "/* 2 */ /** @type {{bar: string[]}} */ ({}).bar
    .forEach(doStuff)(/** @type {{bar: string[]}} */ /* 2 */ {})
    .bar.forEach(doStuff);
";

    assert_format_program_roundtrip_with_file_type(
        source,
        expected,
        FileType::JavaScript,
        javascript_format_options(),
    );
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
    assert_eq!(position, AnnotationPosition::LinePostfix);
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
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker_fragment(&context, "@type")
        .expect("expected cast doc annotation");
    let annotation_position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected cast doc owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);
    let owner_expression = (owner_node_type == NodeType::Expression).then(|| {
        let owner_expression_id = LocalNodeId::<Expression>::new(owner_node as u32);
        context.tree.get(owner_expression_id)
    });

    assert!(matches!(
        context.annotation(annotation_id),
        Annotation::Doc { .. }
    ));
    assert_eq!(annotation_position, AnnotationPosition::LinePrefix);
    assert!(matches!(
        owner_expression,
        Some(Expression::Parenthesized { .. })
    ));

    let formatted = formatter.format(&block_id, DestackFormatOptions::default());
    assert_format_output_eq(expected, &formatted);
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

/// Comments between declaration decorators should stay declaration block-prefix comments.
#[test]
fn test_annotation_decorator_between_comments_attach_to_declaration_prefix() {
    let source = r#"{
    // comment before entity
    @entity
    // comment after entity
    // comment before foo
    @foo(1, 2, 3)
    // comment after foo
    struct Entity {}
}"#;
    let (formatter, _) = TestFormatter::parse(source, |p| {
        p.eat_block(destack_ast::BlockContext::Expression)
    })
    .expect("parse decorator comment ownership source");
    let context = context_from_formatter(&formatter);

    let comment_after_entity_id = find_annotation_by_marker(&context, "comment after entity")
        .expect("expected comment after entity annotation");
    let comment_before_foo_id = find_annotation_by_marker(&context, "comment before foo")
        .expect("expected comment before foo annotation");

    let comment_after_entity_owner =
        find_annotation_target_owner_node(&context, comment_after_entity_id)
            .expect("expected comment after entity owner");
    let comment_before_foo_owner =
        find_annotation_target_owner_node(&context, comment_before_foo_id)
            .expect("expected comment before foo owner");

    assert_eq!(
        context.annotation(comment_after_entity_id).position(),
        AnnotationPosition::BlockPrefix
    );
    assert_eq!(
        context.annotation(comment_before_foo_id).position(),
        AnnotationPosition::BlockPrefix
    );
    assert_eq!(
        context
            .tree
            .get_node_type(comment_after_entity_owner as u32),
        NodeType::Declaration
    );
    assert_eq!(
        context.tree.get_node_type(comment_before_foo_owner as u32),
        NodeType::Declaration
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

/// Own-line array element comments should be preserved across formatting.
#[test]
fn test_format_own_line_array_element_comments_are_preserved() {
    let source = r#"
const test_cases = [
    // first-marker
    ["--check", "*", "!**/error.js"],
    // second-marker
    ["--check", "foo/**/*.js", "!**/bar/*"],
];
"#;

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Ignore-range comments inside arrays should be preserved across formatting.
#[test]
fn test_format_ignore_range_array_comments_are_preserved() {
    let source = r#"
const values = [
    1,
    // format-ignore-start
    foo(1, 2),
    bar(3),
    // format-ignore-end
    baz(4),
];
"#;

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Own-line comments between nested array elements should be preserved.
#[test]
fn test_format_nested_array_element_comments_are_preserved() {
    let source = r#"
const test_cases = [
    // nested-array-comment-a
    ["--check", "*", "!**/error.js"],
    // nested-array-comment-b
    ["--check", "foo/**/*.js", "!**/bar/*"],
];
"#;

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Real-world roundtrip fixture should preserve nested list comments.
#[test]
fn test_format_roundtrip_fixture_0052_preserves_comments() {
    let source = include_str!("../../../../test/fixtures/formatter/roundtrip/formatter-0052.ts");

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Roundtrip fixture comment before the final nested array element should stay as a line prefix.
#[test]
fn test_annotation_roundtrip_fixture_0052_second_comment_stays_line_prefix() {
    let source = include_str!("../../../../test/fixtures/formatter/roundtrip/formatter-0052.ts");
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| Ok(p.parse()))
            .expect("parse roundtrip fixture 0052 source");
    let context = context_from_formatter(&formatter);

    let marker = "Glob include foo/**/*.js, glob exclude bar directory";
    let annotation_id = find_annotation_by_marker(&context, marker)
        .expect("expected second glob include marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected second glob include marker owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);

    assert_eq!(
        position,
        AnnotationPosition::LinePrefix,
        "owner={owner_node}, owner_type={owner_node_type:?}, position={position:?}"
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

/// Own-line comments inside empty object literals should stay object infix comments.
#[test]
fn test_annotation_empty_object_line_comment_stays_object_infix() {
    let source = r#"func({
  // empty-object-line-marker
});"#;
    let (formatter, _) = TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
        p.eat_expression(Default::default())
    })
    .expect("parse empty object line comment source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "empty-object-line-marker")
        .expect("expected empty object line marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected empty object line marker owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);
    assert_eq!(
        position,
        AnnotationPosition::BlockInfix,
        "owner={owner_node}, owner_type={owner_node_type:?}, position={position:?}"
    );
    assert!(
        matches!(owner_node_type, NodeType::Argument | NodeType::Expression),
        "expected empty object line marker owner to be argument or expression, got owner={owner_node}, owner_type={owner_node_type:?}"
    );

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2),
    );
}

/// Own-line comments between a closing delimiter and comma should stay trailing boundary comments.
#[test]
fn test_annotation_closing_delimiter_comma_own_line_comment_is_line_postfix_boundary() {
    let source = r#"foo(
  {}
  // closing-delimiter-comma-marker
  ,
)"#;
    let (formatter, _) = TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
        p.eat_expression(Default::default())
    })
    .expect("parse closing delimiter comma comment source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "closing-delimiter-comma-marker")
        .expect("expected closing delimiter comma marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected closing delimiter comma marker owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);

    assert_eq!(
        position,
        AnnotationPosition::LinePostfixBoundary,
        "owner={owner_node}, owner_type={owner_node_type:?}, position={position:?}"
    );
    assert!(
        matches!(owner_node_type, NodeType::Argument | NodeType::Expression),
        "expected closing delimiter comma marker owner to be argument or expression, got owner={owner_node}, owner_type={owner_node_type:?}"
    );

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2),
    );
}

/// Own-line comments before a closing call delimiter should keep stable ownership.
#[test]
fn test_annotation_closing_delimiter_own_line_comment_has_stable_owner() {
    let source = r#"import(
  // comment1
  `../alias/${base}.js`
  // closing-delimiter-marker
)"#;
    let (formatter, _) = TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
        p.eat_expression(Default::default())
    })
    .expect("parse closing delimiter comment source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "closing-delimiter-marker")
        .expect("expected closing delimiter marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected closing delimiter marker owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);

    assert_ne!(
        position,
        AnnotationPosition::BlockInfix,
        "owner={owner_node}, owner_type={owner_node_type:?}, position={position:?}"
    );
    assert!(
        matches!(owner_node_type, NodeType::Argument | NodeType::Expression),
        "expected closing delimiter marker owner to be argument or expression, got owner={owner_node}, owner_type={owner_node_type:?}"
    );

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2),
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

/// Nested parenthesized arrow postfix comments should keep stable owner layering.
#[test]
fn test_format_nested_parenthesized_arrow_postfix_comments_are_idempotent() {
    let source = "{
    f((a) => ((b) => ((c) => (1, 2, 3)/* b */ /* c */)/* a */) /* b */);
}";
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        javascript_format_options(),
    );
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

#[test]
fn test_format_class_head_and_body_seam_comments_are_idempotent() {
    let source = r#"class A2 /* marker-head-inline */ extends B
// marker-before-body-1
// marker-before-body-2
{
  // marker-body
}
class A3 extends B /* marker-a3-before-body */ {}"#;
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        javascript_format_options(),
    );
}

#[test]
fn test_format_class_implements_comment_indentation_is_idempotent() {
    let source = r#"class a3 extends b
implements
// marker-implements-comment
z,
y {
  constructor() {}
}"#;
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        typescript_format_options(),
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

/// Own-line comments before member dots should stay on the left seam boundary.
#[test]
fn test_annotation_member_dot_comment_attaches_as_left_boundary() {
    let source = r#"
verylongidentifierthatwillwrap123123123123123(
  a.b
    // prettier-ignore
    // issue-10661-marker
    .c
);
"#;
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| Ok(p.parse()))
            .expect("parse member-dot own-line comment source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "issue-10661-marker")
        .expect("expected member-dot marker annotation");
    let ignore_annotation_id = find_annotation_by_marker(&context, "prettier-ignore")
        .expect("expected member-dot ignore annotation");
    let position = context.annotation(annotation_id).position();
    let ignore_position = context.annotation(ignore_annotation_id).position();
    let previous_token_type = context.annotation_previous_non_whitespace_token_type(annotation_id);
    let next_token_type = context.annotation_next_non_whitespace_token_type(annotation_id);
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected member-dot marker owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);

    assert_eq!(
        position,
        AnnotationPosition::LinePostfixBoundary,
        "expected member-dot marker to stay on the left seam boundary, got {position:?}, prev={previous_token_type:?}, next={next_token_type:?}"
    );
    assert_eq!(
        ignore_position,
        AnnotationPosition::LinePostfixBoundary,
        "expected member-dot ignore directive to stay on the left seam boundary, got {ignore_position:?}"
    );
    assert_eq!(owner_node_type, NodeType::Expression);
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

/// Mapped-type value ignore directives should attach as line-prefix comments on the value owner.
#[test]
fn test_annotation_mapped_type_value_ignore_directive_attaches_line_prefix() {
    let source = r#"
type T = {
    [A in B]:
        // prettier-ignore
        C  |  D
}
"#;
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| Ok(p.parse()))
            .expect("parse mapped type value ignore source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "prettier-ignore")
        .expect("expected mapped type value ignore annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected mapped type value ignore owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);

    assert_eq!(position, AnnotationPosition::LinePrefix);
    assert_eq!(owner_node_type, NodeType::Expression);
}

/// Intersection rhs ignore directives should attach as line-prefix comments on rhs union owners.
#[test]
fn test_annotation_intersection_rhs_ignore_directive_attaches_line_prefix() {
    let source = r#"
type T = (A | B) & (
    // prettier-ignore
    A  |  B
);
"#;
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| Ok(p.parse()))
            .expect("parse intersection rhs ignore source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "prettier-ignore")
        .expect("expected intersection rhs ignore annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected intersection rhs ignore owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);

    assert_eq!(position, AnnotationPosition::LinePrefix);
    assert_eq!(owner_node_type, NodeType::Expression);
}

/// Union separator line comments should stay on the left operand boundary.
#[test]
fn test_annotation_union_separator_line_comment_attaches_left_operand_boundary() {
    let source = r#"
type Value = First | // union-seam-marker
Second
"#;
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| Ok(p.parse()))
            .expect("parse union separator seam source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "union-seam-marker")
        .expect("expected union seam marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected union seam marker owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);

    assert_eq!(position, AnnotationPosition::LinePostfixBoundary);
    assert_eq!(owner_node_type, NodeType::Expression);
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// JSX spread-value ignore comments should stay owned by the spread value expression.
#[test]
fn test_annotation_jsx_spread_value_ignore_comment_stays_on_value_expression() {
    let source = "a = <div {...{/* prettier-ignore */}}/>;";
    let (formatter, roots) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScriptXml, |p| Ok(p.parse()))
            .expect("parse jsx spread value ignore source");
    let context = context_from_formatter(&formatter);

    let annotation_ids = find_annotations_by_marker(&context, "prettier-ignore");
    assert_eq!(
        annotation_ids.len(),
        1,
        "expected exactly one prettier-ignore annotation, got {}",
        annotation_ids.len()
    );
    let annotation_id = annotation_ids[0];
    let position = context.annotation(annotation_id).position();
    let annotation = context.annotation(annotation_id);
    let comment_node = match annotation {
        Annotation::Comment { node, .. } => node,
        _ => panic!("expected comment annotation for prettier-ignore"),
    };
    let comment_span = context.span(comment_node);
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected prettier-ignore owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);
    let owner_expression = context
        .tree
        .get(LocalNodeId::<Expression>::new(owner_node as u32));
    let smallest_owner =
        find_smallest_owner_enclosing_range(context.tree, comment_span.start, comment_span.end);
    let previous_token_type = context.annotation_previous_non_whitespace_token_type(annotation_id);
    let next_token_type = context
        .annotation_next_non_whitespace_token(annotation_id)
        .map(|token| token.token.ty);

    assert!(
        !roots.is_empty(),
        "expected parsed roots for jsx spread value ignore source"
    );
    assert_eq!(
        owner_node_type,
        NodeType::Expression,
        "expected spread-value ignore comment owner to be expression, got owner={owner_node}, owner_type={owner_node_type:?}"
    );
    assert!(
        matches!(owner_expression, Expression::ObjectExpression { .. }),
        "expected spread-value ignore comment owner to be object expression, got owner={owner_node}, expression={owner_expression:?}, smallest_owner={smallest_owner:?}, prev={previous_token_type:?}, next={next_token_type:?}"
    );
    assert_eq!(
        position,
        AnnotationPosition::BlockInfix,
        "expected spread-value ignore comment position to be block infix, got {position:?}, smallest_owner={smallest_owner:?}"
    );
}

/// Leading-pipe union line comments should stay on the following declaration value.
#[test]
fn test_annotation_union_leading_pipe_line_comment_stays_on_following_union_value() {
    let source = r#"
type A2 =
  | A
  | B

type A3 =
  | // leading-union-marker
  C
  |
  D;
"#;
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| Ok(p.parse()))
            .expect("parse leading-pipe union comment source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "leading-union-marker")
        .expect("expected leading-pipe union annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected leading-pipe union annotation owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);

    assert_eq!(position, AnnotationPosition::LinePrefix);
    assert_eq!(owner_node_type, NodeType::Expression);
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Union fixture ignore comments should have stable seam ownership and positions.
#[test]
fn test_annotation_union_fixture_18379_ignore_comment_positions() {
    let source = r#"
type A2 =
  (
    A | B // prettier-ignore
  ) & (
    // prettier-ignore
    A | B
  )
"#;
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| Ok(p.parse()))
            .expect("parse union fixture 18379 ignore source");
    let context = context_from_formatter(&formatter);

    let ignore_annotations = find_annotations_by_marker(&context, "prettier-ignore");
    assert_eq!(ignore_annotations.len(), 2);

    let positions: Vec<AnnotationPosition> = ignore_annotations
        .iter()
        .map(|annotation_id| context.annotation(*annotation_id).position())
        .collect();

    assert!(positions.contains(&AnnotationPosition::LinePostfixBoundary));
    assert!(positions.contains(&AnnotationPosition::LinePrefix));
}

/// Mapped-type fixture ignore comments should all keep line-prefix ownership.
#[test]
fn test_annotation_mapped_type_ignore_comment_positions() {
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
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| Ok(p.parse()))
            .expect("parse mapped type ignore source");
    let context = context_from_formatter(&formatter);

    let ignore_annotations = find_annotations_by_marker(&context, "prettier-ignore");
    assert_eq!(ignore_annotations.len(), 4);
    for annotation_id in ignore_annotations {
        assert_eq!(
            context.annotation(annotation_id).position(),
            AnnotationPosition::LinePrefix
        );
    }
}

/// Own-line comments before mapped-type entries should stay on the mapped head seam.
#[test]
fn test_format_typescript_mapped_type_entry_head_comment_is_idempotent() {
    let source = r#"
type M6  = {
  /* 61 */ [b in long_long_long_long_long_long_type|long_long_long_long_long_long_type|long_long_long_long_long_long_type]: string
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

/// Own-line comments after `(` before nested unary heads should stay on the inner unary operand.
#[test]
fn test_annotation_if_nested_unary_own_line_comment_attaches_to_inner_unary() {
    let source = r#"if (!(
  // if-nested-unary-marker
  !(node.type === "ImportExpression" ||
    node.type === "TSImportType" ||
    node.type === "TSExternalModuleReference")
));"#;
    let (formatter, _) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| Ok(p.parse()))
            .expect("parse nested unary own-line marker source");
    let context = context_from_formatter(&formatter);

    let annotation_id = find_annotation_by_marker(&context, "if-nested-unary-marker")
        .expect("expected nested unary own-line marker annotation");
    let position = context.annotation(annotation_id).position();
    let owner_node = find_annotation_target_owner_node(&context, annotation_id)
        .expect("expected nested unary own-line marker owner node");
    let owner_node_type = context.tree.get_node_type(owner_node as u32);
    let owner_expression_kind = if owner_node_type == NodeType::Expression {
        match context
            .tree
            .get(LocalNodeId::<Expression>::new(owner_node as u32))
        {
            Expression::Unary { .. } => "Unary",
            Expression::Parenthesized { .. } => "Parenthesized",
            Expression::If { .. } => "If",
            _ => "Other",
        }
    } else {
        "NonExpression"
    };

    assert_eq!(
        position,
        AnnotationPosition::LinePrefix,
        "owner={owner_node}, owner_type={owner_node_type:?}, owner_expression_kind={owner_expression_kind}",
    );
    assert_eq!(
        owner_expression_kind, "Unary",
        "owner={owner_node}, owner_type={owner_node_type:?}",
    );
}

/// Inline block comments before `)` after callback bodies stay idempotent with adjacent line comments.
#[test]
fn test_format_callback_tail_block_comment_before_close_paren_is_idempotent() {
    let source = r#"x2 = (a) => ((askTrovenaBeenaDependsRowans1, askTrovenaBeenaDependsRowans2, askTrovenaBeenaDependsRowans3) => {
  c();
} /* ! */ // KABOOM
)
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

/// Inline block comments before semicolons should stay on preceding expression boundaries.
#[test]
fn test_format_inline_block_comment_before_semicolon_is_idempotent() {
    let source = r#"{
    const value = run() /* keep before semicolon */;
}
"#;

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Own-line comments between `for..of` heads and non-block bodies should stay idempotent.
#[test]
fn test_format_for_of_head_body_own_line_comment_is_idempotent() {
    let source = r#"
for (a of b)
// marker
foo();
"#;

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Own-line comments between control heads and non-block bodies should stay idempotent.
#[test]
fn test_format_control_head_body_own_line_comments_are_idempotent() {
    let source = r#"
do
// 21
foo(); while(1)
for(a in b)
// 22
foo();
for(a of b)
// 23
foo();
for(;;)
// 24
foo();
if(a)
// 25
foo();
else
// 252
foo();
while(a)
// 26
foo();
with(a)
// 27
foo();
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

/// Template interpolation boundaries should keep line comments on their interpolation owner.
#[test]
fn test_format_template_interpolation_line_suffix_boundary_comments_are_idempotent() {
    let source = r#"`${
a +  // a
  a
}

${a // comment
}

${b /* comment */}

${/* comment */ c /* comment */}

${// comment
d //comment
}

${// $FlowFixMe found when converting React.createClass to ES6
ExampleStory.getFragment('story')}
`;
"#;

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Call trailing-comma comment seams before `)` should stay idempotent.
#[test]
fn test_format_call_trailing_comma_comment_before_close_paren_is_idempotent() {
    let source = r#"doThing(
  someOtherStuff,

  // This is important
  true,
  {
    decline: (creditCard) => takeMoney(creditCard),
  },

);
"#;

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}
