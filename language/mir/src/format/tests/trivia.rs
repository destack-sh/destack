use super::{
    assert_format, assert_format_output_eq, assert_format_to, format_tree_with_options,
    parse_fixture,
};
use crate::{ExecutionModel, Function, MirFormatOptions};

/// Preserves comments and normalizes declaration separators.
#[test]
fn test_format_roundtrip_declaration_comments() {
    assert_format_to(
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
}
"#,
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
}
"#,
    );
}

/// Preserves comments before type fields.
#[test]
fn test_format_roundtrip_type_field_comments() {
    assert_format(
        r#"
type Pair {
    // left
    left: int32;

    // right
    right: int32;
}
"#,
    );
}

/// Preserves comments between attributes and a function head.
#[test]
fn test_format_roundtrip_attribute_comments_before_function_head() {
    assert_format(
        r#"
@executionModel(kernel)
// detail
function kernel(): void {
entry0:
    return
}
"#,
    );
}

/// Preserves comments between attributes and an import function head.
#[test]
fn test_format_roundtrip_attribute_comments_before_import_function_head() {
    assert_format(
        r#"
@cold
// detail
extern function callee(int32): int32
"#,
    );
}

/// Preserves comments between attributes and an export function head.
#[test]
fn test_format_roundtrip_attribute_comments_before_export_function_head() {
    assert_format(
        r#"
@inline
// detail
export function callee(value0: int32): int32 {
entry0(value0: int32):
    return value0
}
"#,
    );
}

/// Preserves comments across multiple explicit attributes before an export function head.
#[test]
fn test_format_roundtrip_multiple_attribute_comments_before_export_function_head() {
    assert_format(
        r#"
@cold
// detail
@inline
// more
export function callee(value0: int32): int32 {
entry0(value0: int32):
    return value0
}
"#,
    );
}

/// Preserves comments between multiple attributes and a function head.
#[test]
fn test_format_roundtrip_multiple_attribute_comment_gaps() {
    assert_format(
        r#"
@executionModel(kernel)
// detail
@workgroupSize(8, 1, 1)
// more
function kernel(): void {
entry0:
    return
}
"#,
    );
}

/// Preserves comments between attributes and an import global head.
#[test]
fn test_format_roundtrip_attribute_comments_before_import_global_head() {
    assert_format(
        r#"
@section(".rodata")
// detail
extern global Count: int32, readonly
"#,
    );
}

/// Preserves comments across multiple explicit attributes before an import global head.
#[test]
fn test_format_roundtrip_multiple_attribute_comments_before_import_global_head() {
    assert_format(
        r#"
@section(".rodata")
// detail
@align(4)
// more
extern global Count: int32, readonly
"#,
    );
}

/// Preserves the function head comment when derived metadata adds one attribute line.
#[test]
fn test_format_output_with_derived_attribute_before_preserved_function_head_comment() {
    let (mut tree, strings) = parse_fixture(
        r#"
@custom
// detail
function kernel(): void {
entry0:
    return
}
"#,
    );

    let function_id = tree
        .iter_nodes::<Function>()
        .map(|(function_id, _)| function_id)
        .next()
        .unwrap_or_else(|| panic!("missing parsed function"));
    tree.get_mut(function_id).execution_model = Some(ExecutionModel::Kernel);

    let output = format_tree_with_options(&tree, &strings, MirFormatOptions::default());

    assert_format_output_eq(
        r#"
@custom
@executionModel(kernel)
// detail
function kernel(): void {
entry0:
    return
}
"#
        .trim(),
        output,
    );
}

/// Preserves comments between attributes and a type head.
#[test]
fn test_format_roundtrip_attribute_comments_before_type_head() {
    assert_format(
        r#"
@packed
// detail
type Pair {
    left: int32;
}
"#,
    );
}

/// Preserves comments between attributes and a global head.
#[test]
fn test_format_roundtrip_attribute_comments_before_global_head() {
    assert_format(
        r#"
@section(".rodata")
// detail
global Count: int32, readonly = 1int32
"#,
    );
}

/// Preserves comments between field attributes and a field head.
#[test]
fn test_format_roundtrip_field_attribute_comments_before_field_head() {
    assert_format(
        r#"
type Pair {
    @align(4)
    // left
    left: int32;
}
"#,
    );
}

/// Preserves comments across multiple attributed fields in one type declaration.
#[test]
fn test_format_roundtrip_multiple_type_field_comment_groups() {
    assert_format(
        r#"
type Pair {
    @align(4)
    // left
    left: int32;

    // middle gap

    @align(8)
    // right
    right: int64;
}
"#,
    );
}

/// Preserves inline comments on function parameters.
#[test]
fn test_format_roundtrip_inline_parameter_comments() {
    assert_format(
        r#"
function use(
    value0: int32, // left
    value1: int32
): int32 {
entry0(value0: int32, value1: int32):
    value2: int32 = int.add value0, value1
    return value2
}
"#,
    );
}

/// Preserves comments in multi-line import parameter lists.
#[test]
fn test_format_roundtrip_import_parameter_comments() {
    assert_format(
        r#"
extern function callee(
    // left
    int32,

    // right
    int32
): int32
"#,
    );
}

/// Preserves trailing instruction comments on the same line.
#[test]
fn test_format_roundtrip_instruction_trailing_comment() {
    assert_format(
        r#"
function use(): void {
entry0:
    value0: int32 = 1int32 // tail
    return
}
"#,
    );
}

/// Preserves comments before the first instruction in a block body.
#[test]
fn test_format_roundtrip_first_instruction_leading_comment() {
    assert_format(
        r#"
function use(): int32 {
entry0:
    // first
    value0: int32 = 1int32
    return value0
}
"#,
    );
}

/// Preserves comments before later block labels.
#[test]
fn test_format_roundtrip_block_leading_comment() {
    assert_format_to(
        r#"
function use(): void {
entry0:
    jump b1

// next
b1:
    return
}
"#,
        r#"
function use(): void {
entry0:
    jump block1

// next
block1:
    return
}
"#,
    );
}

/// Preserves comments between local declarations and the first block.
#[test]
fn test_format_roundtrip_comment_between_locals_and_entry_block() {
    assert_format_to(
        r#"
function use(): void {
    local local0: int32, owned

    // body
entry0:
    return
}
"#,
        r#"
function use(): void {
    local local0: int32, owned

// body
entry0:
    return
}
"#,
    );
}

/// Preserves comments in multi-line function parameter lists.
#[test]
fn test_format_roundtrip_parameter_comments() {
    assert_format(
        r#"
function use(
    // left
    value0: int32,

    // right
    value1: int32
): int32 {
entry0(value0: int32, value1: int32):
    value2: int32 = int.add value0, value1
    return value2
}
"#,
    );
}

/// Preserves trailing comments after the final item.
#[test]
fn test_format_roundtrip_end_of_file_trailing_comment() {
    assert_format(
        r#"
function use(): void {
entry0:
    return
}
// tail
"#,
    );
}

/// Preserves comments before local declarations.
#[test]
fn test_format_roundtrip_local_leading_comments() {
    assert_format(
        r#"
function use(): void {
    // scratch
    local local0: int32, owned

entry0:
    return
}
"#,
    );
}

/// Preserves trailing comments after local declarations.
#[test]
fn test_format_roundtrip_local_trailing_comment() {
    assert_format(
        r#"
function use(): void {
    local local0: int32, owned // scratch

entry0:
    return
}
"#,
    );
}

/// Preserves comments before terminators in otherwise empty blocks.
#[test]
fn test_format_roundtrip_terminator_leading_comment() {
    assert_format(
        r#"
function use(): void {
entry0:
    // tail
    return
}
"#,
    );
}
