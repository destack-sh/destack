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
function pointerSized(v0: isize, v1: usize, v2: typeDescriptor, v3: typeId): usize {
entry(v0: isize, v1: usize, v2: typeDescriptor, v3: typeId):
    return v1
}
"#,
    );
}

/// Formats concrete float types canonically.
#[test]
fn test_format_concrete_float_types() {
    assert_format(
        r#"
function floats(v0: float16, v1: bfloat16, v2: float32, v3: float64): void {
entry(v0: float16, v1: bfloat16, v2: float32, v3: float64):
    v4: float16 = 1.5
    v5: bfloat16 = 1.5
    return
}
"#,
    );
}

/// Formats managed and unique reference kinds canonically.
#[test]
fn test_format_managed_and_unique_references() {
    assert_format(
        r#"
function refs(v0: ref<int32, managed, nullable>, v1: ref<int32, unique, readonly>): ref<int32, managed, nullable> {
entry(v0: ref<int32, managed, nullable>, v1: ref<int32, unique, readonly>):
    return v0
}
"#,
    );
}

/// Formats raw spaces canonically.
#[test]
fn test_format_raw_spaces() {
    assert_format(
        r#"
function rawSpaces(v0: ref<int32, raw, space(shared)>, v1: ref<int32, raw, space(static)>): ref<int32, raw, space(shared)> {
entry(v0: ref<int32, raw, space(shared)>, v1: ref<int32, raw, space(static)>):
    return v0
}
"#,
    );
}

/// Formats parameter borrow lifetimes canonically.
#[test]
fn test_format_parameter_borrow_lifetime() {
    assert_format(
        r#"
function borrowParam(v0: ref<int32, borrowed, lifetime(0)>): ref<int32, borrowed, lifetime(0)> {
entry(v0: ref<int32, borrowed, lifetime(0)>):
    return v0
}
"#,
    );
}

/// Formats static borrow lifetimes canonically.
#[test]
fn test_format_static_borrow_lifetime() {
    assert_format(
        r#"
function staticBorrow(v0: ref<int32, borrowed, lifetime(static)>): ref<int32, borrowed, lifetime(static)> {
entry(v0: ref<int32, borrowed, lifetime(static)>):
    return v0
}
"#,
    );
}

/// Formats named lifetime parameters and applied lifetime arguments.
#[test]
fn test_format_named_lifetimes() {
    assert_format(
        r#"
type Player<LWorld: lifetime, LMesh: lifetime> {
    world: ref<int32, borrowed, lifetime(LWorld)>;
    mesh: ref<float64, borrowed, lifetime(LMesh)>;
}

function tickPlayer<LPlayer: lifetime, LWorld: lifetime, LMesh: lifetime>(v0: ref<Player<lifetime(LWorld), lifetime(LMesh)>, borrowed, lifetime(LPlayer)>): void {
entry(v0: ref<Player<lifetime(LWorld), lifetime(LMesh)>, borrowed, lifetime(LPlayer)>):
    return
}
"#,
    );
}

/// Formats callable suspension contracts canonically.
#[test]
fn test_format_callable_suspension_contract() {
    assert_format(
        r#"
function callContract(v0: (ref<int32, borrowed, readonly> @suspensionSafe(0)) => int32, v1: ref<int32, borrowed, readonly>): int32 {
entry(v0: (ref<int32, borrowed, readonly> @suspensionSafe(0)) => int32, v1: ref<int32, borrowed, readonly>):
    v2: int32 = call.indirect v0(v1): (ref<int32, borrowed, readonly> @suspensionSafe(0)) => int32
    return v2
}
"#,
    );
}

/// Formats borrowed shaped views canonically.
#[test]
fn test_format_borrowed_shaped_views() {
    assert_format(
        r#"
function views(v0: slice<int32, borrowed, lifetime(0), readonly>, v1: tensorView<int32, borrowed, lifetime(0), (4, 4)>): void {
entry(v0: slice<int32, borrowed, lifetime(0), readonly>, v1: tensorView<int32, borrowed, lifetime(0), (4, 4)>):
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
function tensors(v0: tensor<float32, (batch, dynamic, 64), layout(dense(columnMajor))>, v1: tensorView<float32, borrowed, lifetime(0), readonly, (batch, dynamic, 64), layout(strided)>): void {
entry(v0: tensor<float32, (batch, dynamic, 64), layout(dense(columnMajor))>, v1: tensorView<float32, borrowed, lifetime(0), readonly, (batch, dynamic, 64), layout(strided)>):
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
function sequences(v0: (int32, float64, boolean), v1: [int32; 10]): (int32, float64, boolean) {
entry(v0: (int32, float64, boolean), v1: [int32; 10]):
    return v0
}
"#,
    );
}

/// Formats callable type forms canonically.
#[test]
fn test_format_callable_types() {
    assert_format(
        r#"
function callbacks(v0: fn(int32, int32) => int64, v1: (int32) => int32): (int32) => int32 {
entry(v0: fn(int32, int32) => int64, v1: (int32) => int32):
    return v1
}
"#,
    );
}

/// Formats structural type forms canonically.
#[test]
fn test_format_structural_types() {
    assert_format(
        r#"
function point(v0: { x: int32, y: float64 }): { x: int32, y: float64 } {
entry(v0: { x: int32, y: float64 }):
    return v0
}
"#,
    );
}

/// Formats erased dynamic type forms canonically.
#[test]
fn test_format_dynamic_types() {
    assert_format(
        r#"
type Writer {
    write: fn() => uint32;
}

function erased(v0: dynamic<Writer>): dynamic<Writer> {
entry(v0: dynamic<Writer>):
    return v0
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
entry(v0: ref<Point, managed>, v1: ref<Node, managed>):
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

function usePoint(v0: ref<Point, managed>, v1: ref<Node, managed>): ref<Point, managed> {
entry(v0: ref<Point, managed>, v1: ref<Node, managed>):
    return v0
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
        ty: int32_type,
    });
    let right = tree.insert(Field {
        name: None,
        ty: int32_type,
    });
    let struct_type = tree.insert_type(Type::Struct {
        fields: vec![left, right],
        copy: Copy::No,
    });
    tree.insert(TypeAlias {
        name: alias_name,
        lifetimes: Vec::new(),
        ty: struct_type,
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
        ty: int32_type,
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
        lifetimes: Vec::new(),
        ty: struct_type,
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
