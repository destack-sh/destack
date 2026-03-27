use destack_ast::{AnnotationPosition, Block, NodeParentIndex};
use destack_source::FileType;

use crate::{
    Annotation, DestackFormatContext, DestackFormatOptions, TestFormatter, assert_format,
    statement_list,
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

/// Parser annotations should keep loop-to-let spacing as one block-prefix blank seam.
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
        &test.file,
        &test.tree,
        &test.tokens,
        &test.side_tokens,
        &test.side_span,
        &test.strings,
        NodeParentIndex::from_tree(&test.tree),
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
    assert_eq!(loop_blank_block_prefix, 0);

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
    assert_eq!(let_blank_annotations, 1);

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
    assert_eq!(let_blank_block_prefix, 1);

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
        &test.file,
        &test.tree,
        &test.tokens,
        &test.side_tokens,
        &test.side_span,
        &test.strings,
        NodeParentIndex::from_tree(&test.tree),
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
        &statement_list(expressions.as_slice()),
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
        &statement_list(expressions.as_slice()),
        DestackFormatOptions::default(),
    );
    assert_eq!(formatted.trim_end_matches('\n'), expected);
}
