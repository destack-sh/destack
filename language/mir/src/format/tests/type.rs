use super::{assert_format, assert_format_eq, assert_output_eq, format_tree_with_options};
use crate::{
    Attribute, AttributeArgs, AttributeIdentifier, Copy, Field, MirFormatOptions, Tree, Type,
    TypeAlias,
};
use destack_core::StringPool;

/// Formats pointer-sized builtin types canonically.
#[test]
fn test_format_pointer_sized_builtin_types() {
    assert_format(
        r#"
function pointerSized(value0: isize, value1: usize, value2: typeDescriptor, value3: typeId): usize {
entry0(value0: isize, value1: usize, value2: typeDescriptor, value3: typeId):
    return value1
}
"#,
    );
}

/// Formats managed and unique reference kinds canonically.
#[test]
fn test_format_managed_and_unique_references() {
    assert_format(
        r#"
function refs(value0: ref<int32, managed, nullable>, value1: ref<int32, unique, readonly>): ref<int32, managed, nullable> {
entry0(value0: ref<int32, managed, nullable>, value1: ref<int32, unique, readonly>):
    return value0
}
"#,
    );
}

/// Formats raw address spaces canonically.
#[test]
fn test_format_raw_address_spaces() {
    assert_format(
        r#"
function rawSpaces(value0: ref<int32, raw, space(shared)>, value1: ref<int32, raw, space(gpu)>): ref<int32, raw, space(shared)> {
entry0(value0: ref<int32, raw, space(shared)>, value1: ref<int32, raw, space(gpu)>):
    return value0
}
"#,
    );
}

/// Formats parameter borrow lifetimes canonically.
#[test]
fn test_format_parameter_borrow_lifetime() {
    assert_format(
        r#"
function borrowParam(value0: ref<int32, borrowed, lifetime(0)>): ref<int32, borrowed, lifetime(0)> {
entry0(value0: ref<int32, borrowed, lifetime(0)>):
    return value0
}
"#,
    );
}

/// Formats static borrow lifetimes canonically.
#[test]
fn test_format_static_borrow_lifetime() {
    assert_format(
        r#"
function staticBorrow(value0: ref<int32, borrowed, lifetime(static)>): ref<int32, borrowed, lifetime(static)> {
entry0(value0: ref<int32, borrowed, lifetime(static)>):
    return value0
}
"#,
    );
}

/// Formats borrowed shaped views canonically.
#[test]
fn test_format_borrowed_shaped_views() {
    assert_format(
        r#"
function views(value0: slice<int32, borrowed, lifetime(0), readonly>, value1: tensorView<int32, borrowed, lifetime(0), (4, 4)>): void {
entry0(value0: slice<int32, borrowed, lifetime(0), readonly>, value1: tensorView<int32, borrowed, lifetime(0), (4, 4)>):
    return
}
"#,
    );
}

/// Formats tensor shapes and layouts canonically.
#[test]
fn test_format_tensor_shapes_and_layouts() {
    assert_format(
        r#"
function tensors(value0: tensor<float32, (batch, dynamic, 64), layout(dense(columnMajor))>, value1: tensorView<float32, borrowed, lifetime(0), readonly, (batch, dynamic, 64), layout(strided)>): void {
entry0(value0: tensor<float32, (batch, dynamic, 64), layout(dense(columnMajor))>, value1: tensorView<float32, borrowed, lifetime(0), readonly, (batch, dynamic, 64), layout(strided)>):
    return
}
"#,
    );
}

/// Formats tuple and array type forms canonically.
#[test]
fn test_format_tuple_and_array_types() {
    assert_format(
        r#"
function sequences(value0: (int32, float64, boolean), value1: [int32; 10]): (int32, float64, boolean) {
entry0(value0: (int32, float64, boolean), value1: [int32; 10]):
    return value0
}
"#,
    );
}

/// Formats callable type forms canonically.
#[test]
fn test_format_callable_types() {
    assert_format(
        r#"
function callbacks(value0: (int32, int32) -> int64, value1: (int32) => int32): (int32) => int32 {
entry0(value0: (int32, int32) -> int64, value1: (int32) => int32):
    return value1
}
"#,
    );
}

/// Formats structural type forms canonically.
#[test]
fn test_format_structural_types() {
    assert_format(
        r#"
function point(value0: { x: int32, y: float64 }): { x: int32, y: float64 } {
entry0(value0: { x: int32, y: float64 }):
    return value0
}
"#,
    );
}

/// Formats erased Any type forms canonically.
#[test]
fn test_format_any_types() {
    assert_format(
        r#"
type Writer {
    write: () -> uint32;
}

function erased(value0: any<Writer>): any<Writer> {
entry0(value0: any<Writer>):
    return value0
}
"#,
    );
}

/// Formats recursive and named type declarations canonically.
#[test]
fn test_format_type_declarations() {
    assert_format_eq(
        r#"
type Point {
    x: int32;
    y: float64;
}

type Node {
    value: int64;
    next: ref<Node, managed>;
}

function usePoint(v0: ref<Point, managed>, v1: ref<Node, managed>): ref<Point, managed> {
b0(v0: ref<Point, managed>, v1: ref<Node, managed>):
    return v0
}
"#,
        r#"
type Point {
    x: int32;
    y: float64;
}

type Node {
    value: int64;
    next: ref<Node, managed>;
}

function usePoint(value0: ref<Point, managed>, value1: ref<Node, managed>): ref<Point, managed> {
entry0(value0: ref<Point, managed>, value1: ref<Node, managed>):
    return value0
}
"#,
    );
}

/// Formats copy markers canonically.
#[test]
fn test_format_type_copy_markers() {
    assert_format(
        r#"
@moveOnly
type OwnedPair {
    ref<int32, unique>;
    ref<int32, unique>;
}

@copy
type CopyPair {
    int32;
    int32;
}
"#,
    );
}

/// Formats synthetic move-only markers for built types.
#[test]
fn test_format_synthetic_move_only_marker() {
    let mut tree = Tree::new();
    let strings = StringPool::new();

    let int32_type = tree.insert_type(Type::Int {
        width: 32,
        is_signed: true,
    });
    let alias_name = strings.intern("Pair");

    let left = tree.insert(Field {
        name: None,
        ty: int32_type.into(),
    });
    let right = tree.insert(Field {
        name: None,
        ty: int32_type.into(),
    });
    let struct_type = tree.insert_type(Type::Struct {
        fields: vec![left, right],
        copy: Copy::No,
    });
    tree.insert(TypeAlias {
        name: alias_name,
        ty: struct_type.into(),
    });

    let output = format_tree_with_options(&tree, &strings, MirFormatOptions::default());

    assert_output_eq(
        r#"
@moveOnly
type Pair {
    int32;
    int32;
}
"#
        .trim(),
        output,
    );
}

/// Formats attributed struct fields without parsed field spans.
#[test]
fn test_format_struct_fields_with_attributes_without_parsed_spans() {
    let mut tree = Tree::new();
    let strings = StringPool::new();

    let int32_type = tree.insert_type(Type::Int {
        width: 32,
        is_signed: true,
    });
    let attribute_name = strings.intern("packed");
    let alias_name = strings.intern("Point");
    let field_name = strings.intern("x");

    let field_id = tree.insert(Field {
        name: Some(field_name),
        ty: int32_type.into(),
    });
    tree.set_attributes(
        field_id,
        vec![Attribute {
            name: AttributeIdentifier::identifier(attribute_name),
            args: AttributeArgs::None,
        }],
    );

    let struct_type = tree.insert_type(Type::Struct {
        fields: vec![field_id],
        copy: Copy::Yes,
    });
    tree.insert(TypeAlias {
        name: alias_name,
        ty: struct_type.into(),
    });

    let output = format_tree_with_options(&tree, &strings, MirFormatOptions::default());

    assert_output_eq(
        r#"
type Point {
    @packed
    x: int32;
}
"#
        .trim(),
        output,
    );
}
