use crate::{Block, Function, assert_node};

use super::tests::{TestParser, comment_texts};

#[test]
fn test_parse_declaration_comments() {
    TestParser::new(
        r#"
// aliases
type Callable = (int32) => int32;
// imports
external function callee(int32): int32;

// globals
readonly global Count: int32 = 1int32;

function use(v0: Callable): int32 {
b0(v0: Callable):
    v1: ref<int32, raw, readonly> = global.address Count
    v2: int32 = load v1
    v3: int32 = call.indirect v0(v2): (int32) -> int32
    return v3
}"#,
    )
    .assert_format(
        r#"
// aliases
type Callable = (int32) => int32;

// imports
external function callee(int32): int32

// globals
readonly global Count: int32 = 1int32

function use(value0: Callable): int32 {
entry0(value0: Callable):
    value1: ref<int32, raw, readonly> = global.address Count
    value2: int32 = load value1
    value3: int32 = call.indirect value0(value2): (int32) -> int32
    return value3
}"#,
    );
}

#[test]
fn test_parse_type_field_comments() {
    TestParser::new(
        r#"
type Pair {
    // left
    left: int32;

    // right
    right: int32;
}
"#,
    )
    .assert_format(
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
fn test_parse_function_attribute_comments() {
    TestParser::new(
        r#"
@cold
// detail
function kernel(): void {
entry0:
    return
}
"#,
    )
    .assert_format(
        r#"
@cold
// detail
function kernel(): void {
entry0:
    return
}"#,
    );
}

#[test]
fn test_parse_field_attribute_comments() {
    TestParser::new(
        r#"
type Pair {
    @align(4)
    // left
    left: int32;
}
"#,
    )
    .assert_format(
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
fn test_parse_item_comments() {
    let source = r#"
// head
function use(): void {
entry0:
    return // tail
}
"#;

    // parse
    let tree = TestParser::new(source).tree();
    let (function_id, function) = tree.iter_nodes::<Function>().next().unwrap();

    // // head and // tail
    assert_node!(tree, function.blocks[0], Block { terminator, .. } => {
        let leading_comments = tree.leading_comments(function_id);
        let terminator_span = tree.get_span(*terminator).unwrap();
        let trailing_comments = tree.comments_between(terminator_span.end, u32::MAX);

        assert_eq!(comment_texts(&leading_comments), vec!["// head"]);
        assert_eq!(comment_texts(&trailing_comments), vec!["// tail"]);
    });
}

/// Recovering parse still keeps comments around broken syntax.
#[test]
fn test_parse_broken_item_comments() {
    let source = r#"
// before
global Broken int32 = 0int32
// after
function later(): void {
entry0:
    return
}
"#;

    // parse
    let (tree, diagnostics) = TestParser::new(source).parse_with_diagnostics();
    let comments = tree.comments_between(0, u32::MAX);

    // // before and // after
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(comment_texts(&comments), vec!["// before", "// after"]);
}

/// Parsed MIR assigns leading comments to the next top-level item.
#[test]
fn test_parse_item_leading_comments() {
    let source = r#"
// head

function use(): void {
entry0:
    return
}
"#;

    // parse
    let tree = TestParser::new(source).tree();
    let (function_id, _) = tree.iter_nodes::<Function>().next().unwrap();
    let leading = tree.leading_comments(function_id);

    // // head
    assert_eq!(comment_texts(&leading), vec!["// head"]);
}

/// Parsed MIR keeps inline trailing comments out of the next node's leading comments.
#[test]
fn test_parse_instruction_inline_comments() {
    let source = r#"
function use(): void {
entry0:
    value0: int32 = 1int32 // tail
    return
}
"#;

    // parse
    let tree = TestParser::new(source).tree();
    let (_, function) = tree.iter_nodes::<Function>().next().unwrap();

    // no terminator leading comments
    assert_node!(tree, function.blocks[0], Block { terminator, .. } => {
        let leading = tree.leading_comments(*terminator);

        assert!(leading.is_empty());
    });
}

/// Parsed MIR assigns line-leading comments to the following block.
#[test]
fn test_parse_block_leading_comments() {
    let source = r#"
function use(): void {
entry0:
    return

// next
b1:
    return
}
"#;

    // parse
    let tree = TestParser::new(source).tree();
    let (_, function) = tree.iter_nodes::<Function>().next().unwrap();
    let leading = tree.leading_comments(function.blocks[1]);

    // // next
    assert_eq!(comment_texts(&leading), vec!["// next"]);
}

/// Parsed MIR assigns body comments before the first block to that block.
#[test]
fn test_parse_first_block_body_comments() {
    let source = r#"
function use(): void {
// body
entry0:
    return
}
"#;

    // parse
    let tree = TestParser::new(source).tree();
    let (_, function) = tree.iter_nodes::<Function>().next().unwrap();
    let leading = tree.leading_comments(function.blocks[0]);

    // // body
    assert_eq!(comment_texts(&leading), vec!["// body"]);
}

/// Parsed MIR assigns label-to-terminator comments to the terminator when a block is otherwise empty.
#[test]
fn test_parse_terminator_leading_comments() {
    let source = r#"
function use(): void {
entry0:
    // tail
    return
}
"#;

    // parse
    let tree = TestParser::new(source).tree();
    let (_, function) = tree.iter_nodes::<Function>().next().unwrap();

    // // tail
    assert_node!(tree, function.blocks[0], Block { terminator, .. } => {
        let leading = tree.leading_comments(*terminator);

        assert_eq!(comment_texts(&leading), vec!["// tail"]);
    });
}

/// Parsed MIR keeps comments after the final terminator.
#[test]
fn test_parse_block_end_comments() {
    let source = r#"
function use(): void {
entry0:
    return // tail
}
"#;

    // parse
    let tree = TestParser::new(source).tree();
    let (_, function) = tree.iter_nodes::<Function>().next().unwrap();

    // // tail
    assert_node!(tree, function.blocks[0], Block { terminator, .. } => {
        let terminator_span = tree.get_span(*terminator).unwrap();
        let comments = tree.comments_between(terminator_span.end, u32::MAX);

        assert_eq!(comment_texts(&comments), vec!["// tail"]);
    });
}

/// Parsed MIR keeps final top-level comments.
#[test]
fn test_parse_final_item_comments() {
    let source = r#"
function use(): void {
entry0:
    return
}
// tail
"#;

    // parse
    let tree = TestParser::new(source).tree();
    let (function_id, _) = tree.iter_nodes::<Function>().next().unwrap();
    let function_span = tree.get_span(function_id).unwrap();
    let comments = tree.comments_between(function_span.end, u32::MAX);

    // // tail
    assert_eq!(comment_texts(&comments), vec!["// tail"]);
}
