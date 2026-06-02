use super::{
    assert_format, assert_format_eq, assert_output_eq, format_tree_with_options, parse_fixture,
};
use crate::{
    Access, Function, Lifetime, MirFormatOptions, Nullability, ReferenceKind, Space, Type,
    TypeReference,
};

/// Preserves declaration comments while normalizing canonical separators and names.
#[test]
fn test_format_declaration_comments() {
    assert_format_eq(
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
}
"#,
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
}
"#,
    );
}

/// Preserves comments between attributes and item heads across declaration kinds.
#[test]
fn test_format_item_attribute_comments() {
    assert_format(
        r#"
@packed
// type detail
type Pair {
    left: int32;
}

@section(".rodata")
// global detail
readonly global Count: int32 = 1int32

@section(".rodata")
// import global detail
@align(4)
// more import global detail
external readonly global Imported: int32

@cold
// import detail
external function callee(int32): int32

@cold
// function detail
@inline
// more function detail
function kernel(): void {
entry0:
    return
}

@cold
// export detail
@inline
// more export detail
export function exported(value0: int32): int32 {
entry0(value0: int32):
    return value0
}
"#,
    );
}

/// Preserves the function head comment when derived environment metadata adds one attribute line.
#[test]
fn test_format_derived_attribute_keeps_head_comment() {
    // parse
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

    // derived environment metadata
    let function_id = tree
        .iter_nodes::<Function>()
        .map(|(function_id, _)| function_id)
        .next()
        .unwrap_or_else(|| panic!("missing parsed function"));
    let int32 = tree.insert_type(Type::Int {
        width: 32,
        is_signed: true,
    });
    let environment = tree.insert_type(Type::Reference {
        kind: ReferenceKind::Managed,
        lifetime: Lifetime::empty(),
        space: Space::Local,
        access: Access::Mutable,
        pointee: TypeReference::from(int32),
        nullability: Nullability::None,
    });
    tree.get_mut(function_id).environment = Some(TypeReference::from(environment));

    // // detail
    let output = format_tree_with_options(&tree, &strings, MirFormatOptions::default());

    assert_output_eq(
        r#"
@custom
@environment(ref<int32, managed>)
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

/// Preserves comments around attributed and unattributed fields in one type declaration.
#[test]
fn test_format_field_comments() {
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

/// Preserves comments in import and function parameter lists.
#[test]
fn test_format_parameter_comments() {
    assert_format(
        r#"
external function callee(
    // left
    int32,

    // right
    int32
): int32

function use(
    // left
    value0: int32,
    value1: int32 // right
): int32 {
entry0(value0: int32, value1: int32):
    value2: int32 = int.add value0, value1
    return value2
}
"#,
    );
}

/// Preserves comments through local declarations, blocks, instructions, terminators, and EOF.
#[test]
fn test_format_body_comments() {
    assert_format_eq(
        r#"
function use(): void {
    // scratch
    local local0: int32, owned
    local local1: int32, owned // scratch

    // body
entry0:
    // first
    value0: int32 = 1int32 // tail
    jump b1

// next
b1:
    // tail
    return
}
// eof
"#,
        r#"
function use(): void {
    // scratch
    local local0: int32, owned
    local local1: int32, owned // scratch

// body
entry0:
    // first
    value0: int32 = 1int32 // tail
    jump block1

// next
block1:
    // tail
    return
}
// eof
"#,
    );
}
