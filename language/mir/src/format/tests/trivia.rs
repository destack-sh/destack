use super::{
    assert_format, assert_format_eq, assert_output_eq, format_tree_with_options, parse_fixture,
};
use crate::{Copy, Access, FormatOptions, Function, Lifetime, Reference, Space, Storage, Type};

/// Preserves declaration comments while normalizing canonical separators and names.
#[test]
fn test_format_declaration_comments() {
    assert_format(
        r#"
// declarations
type Callable = (int32) => int32;

// imports
external function callee(int32): int32

// globals
readonly global Count: int32 = 1

function use(v0: Callable): int32 {
entry(v0: Callable):
    v1: ref<int32, borrowed, 'static & local, readonly> = address @Count
    v2: int32 = load (*v1)
    v3: int32 = call.indirect v0(v2): (int32) => int32
    return v3
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
readonly global Count: int32 = 1
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
entry:
    return
}

@cold
// export detail
@inline
// more export detail
export function exported(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#,
    );
}

/// Preserves the function head comment when derived environment tables adds one attribute line.
#[test]
fn test_format_derived_attribute_keeps_head_comment() {
    // parse
    let (mut tree, strings) = parse_fixture(
        r#"
@custom
// detail
function kernel(): void {
entry:
    return
}
"#,
    );

    // derived environment tables
    let function_id = tree
        .iter_nodes::<Function>()
        .map(|(function_id, _)| function_id)
        .next()
        .unwrap_or_else(|| panic!("missing parsed function"));
    let int32 = tree.intern_type(Type::Int {
        width: 32,
        is_signed: true,
    }, Copy::Yes);
    let environment = tree.intern_type(Type::Reference {
        kind: Reference::Managed,
        lifetime: Lifetime::empty(),
        storage: Storage::Heap(Space::Local),
        access: Access::Mutable,
        pointee: int32,
    }, Copy::Yes);
    tree.get_mut(function_id).environment = Some(environment);

    // // detail
    let output = format_tree_with_options(&tree, &strings, FormatOptions::default());

    assert_output_eq(
        r#"
@custom
@environment(ref<int32, managed, mutable, local>)
// detail
function kernel(): void {
entry:
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
    v0: int32,
    v1: int32 // right
): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = add v0, v1
    return v2
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
    local l0: int32
    local l1: int32 // scratch

// body
entry:
    // first
    v0: int32 = 1 // tail
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
    local l0: int32
    local l1: int32 // scratch

// body
entry:
    // first
    v0: int32 = 1 // tail
    jump b1

// next
b1:
    // tail
    return
}
// eof
"#,
    );
}
