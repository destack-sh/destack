use crate::Function;

use super::tests::{comment_texts, parse_and_format, parse_fixture, parse_fixture_source};

#[test]
fn test_parse_declaration_semicolons_and_comments() {
    parse_and_format(
        r#"
// aliases
type Callable = closure(int32) -> int32;
// imports
extern function callee(int32): int32;

// globals
global Count: int32, readonly = 1int32;

function use(v0: Callable): int32 {
b0(v0: Callable):
    v1: int32 = global.const Count
    v2: int32 = call.indirect v0(v1): (int32) -> int32
    return v2
}"#,
        r#"
// aliases
type Callable = closure(int32) -> int32;

// imports
extern function callee(int32): int32

// globals
global Count: int32, readonly = 1int32

function use(value0: Callable): int32 {
entry0(value0: Callable):
    value1: int32 = global.const Count
    value2: int32 = call.indirect value0(value1): (int32) -> int32
    return value2
}"#,
    );
}

#[test]
fn test_parse_type_alias_field_comments() {
    parse_and_format(
        r#"
type Pair {
    // left
    left: int32;

    // right
    right: int32;
}
"#,
        r#"
type Pair {
    // left
    left: int32;

    // right
    right: int32;
}"#,
    );
}

#[test]
fn test_parse_attribute_comments_before_function_head() {
    parse_and_format(
        r#"
@executionModel(kernel)
// detail
function kernel(): void {
entry0:
    return
}
"#,
        r#"
@executionModel(kernel)
// detail
function kernel(): void {
entry0:
    return
}"#,
    );
}

#[test]
fn test_parse_field_attribute_comments_before_field_head() {
    parse_and_format(
        r#"
type Pair {
    @align(4)
    // left
    left: int32;
}
"#,
        r#"
type Pair {
    @align(4)
    // left
    left: int32;
}"#,
    );
}

/// Parsed MIR keeps comments around parsed items.
#[test]
fn test_parse_keeps_comments_around_parsed_items() {
    let source = r#"
// head
function use(): void {
entry0:
    return // tail
}
"#;

    let tree = parse_fixture(source);
    let (function_id, function) = tree.iter_nodes::<Function>().next().unwrap();
    let block = tree.get(function.blocks[0]);
    let terminator_span = tree.get_span(block.terminator).unwrap();

    let leading_comments = tree.leading_comments(function_id);
    let trailing_comments = tree.comments_between(terminator_span.end, u32::MAX);

    assert_eq!(comment_texts(&leading_comments), vec!["// head"]);
    assert_eq!(comment_texts(&trailing_comments), vec!["// tail"]);
}

/// Recovering parse still keeps comments around broken syntax.
#[test]
fn test_parse_source_keeps_comments_around_broken_items() {
    let source = r#"
// before
global Broken int32 = 0int32
// after
function later(): void {
entry0:
    return
}
"#;

    let (tree, diagnostics) = parse_fixture_source(source);
    let comments = tree.comments_between(0, u32::MAX);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(comment_texts(&comments), vec!["// before", "// after"]);
}

/// Parsed MIR assigns leading comments to the next top-level item.
#[test]
fn test_parse_assigns_item_leading_comments() {
    let source = r#"
// head

function use(): void {
entry0:
    return
}
"#;

    let tree = parse_fixture(source);
    let (function_id, _) = tree.iter_nodes::<Function>().next().unwrap();
    let leading = tree.leading_comments(function_id);

    assert_eq!(comment_texts(&leading), vec!["// head"]);
}

/// Parsed MIR keeps inline trailing comments out of the next node's leading comments.
#[test]
fn test_parse_keeps_instruction_inline_comments_out_of_next_leading_comments() {
    let source = r#"
function use(): void {
entry0:
    value0: int32 = 1int32 // tail
    return
}
"#;

    let tree = parse_fixture(source);
    let (_, function) = tree.iter_nodes::<Function>().next().unwrap();
    let block = tree.get(function.blocks[0]);
    let leading = tree.leading_comments(block.terminator);

    assert!(leading.is_empty());
}

/// Parsed MIR assigns line-leading comments to the following block.
#[test]
fn test_parse_assigns_block_leading_comments() {
    let source = r#"
function use(): void {
entry0:
    return

// next
b1:
    return
}
"#;

    let tree = parse_fixture(source);
    let (_, function) = tree.iter_nodes::<Function>().next().unwrap();
    let leading = tree.leading_comments(function.blocks[1]);

    assert_eq!(comment_texts(&leading), vec!["// next"]);
}

/// Parsed MIR assigns body comments before the first block to that block.
#[test]
fn test_parse_assigns_first_block_body_leading_comments() {
    let source = r#"
function use(): void {
// body
entry0:
    return
}
"#;

    let tree = parse_fixture(source);
    let (_, function) = tree.iter_nodes::<Function>().next().unwrap();
    let leading = tree.leading_comments(function.blocks[0]);

    assert_eq!(comment_texts(&leading), vec!["// body"]);
}

/// Parsed MIR assigns label-to-terminator comments to the terminator when a block is otherwise empty.
#[test]
fn test_parse_assigns_terminator_leading_comments() {
    let source = r#"
function use(): void {
entry0:
    // tail
    return
}
"#;

    let tree = parse_fixture(source);
    let (_, function) = tree.iter_nodes::<Function>().next().unwrap();
    let block = tree.get(function.blocks[0]);
    let leading = tree.leading_comments(block.terminator);

    assert_eq!(comment_texts(&leading), vec!["// tail"]);
}

/// Parsed MIR keeps comments after the final terminator.
#[test]
fn test_parse_keeps_block_end_comments() {
    let source = r#"
function use(): void {
entry0:
    return // tail
}
"#;

    let tree = parse_fixture(source);
    let (_, function) = tree.iter_nodes::<Function>().next().unwrap();
    let block = tree.get(function.blocks[0]);
    let terminator_span = tree.get_span(block.terminator).unwrap();
    let comments = tree.comments_between(terminator_span.end, u32::MAX);

    assert_eq!(comment_texts(&comments), vec!["// tail"]);
}

/// Parsed MIR keeps final top-level comments.
#[test]
fn test_parse_keeps_final_item_comments() {
    let source = r#"
function use(): void {
entry0:
    return
}
// tail
"#;

    let tree = parse_fixture(source);
    let (function_id, _) = tree.iter_nodes::<Function>().next().unwrap();
    let function_span = tree.get_span(function_id).unwrap();
    let comments = tree.comments_between(function_span.end, u32::MAX);

    assert_eq!(comment_texts(&comments), vec!["// tail"]);
}
