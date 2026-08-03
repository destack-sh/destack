use super::{assert_format, assert_format_eq, assert_output_eq, format_tree_with_options};
use crate::{
    Attribute, AttributeArgs, AttributeIdentifier, Copy, Field, FormatOptions, Symbol, Tree, Type,
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

/// Formats the character type and constants without integer erasure.
#[test]
fn test_format_character_type() {
    assert_format(
        r#"
function character(): char {
entry:
    v0: char = 'A'
    return v0
}
"#,
    );
}

/// Formats reference kinds and nullability canonically.
#[test]
fn test_format_reference_kinds_and_nullability() {
    assert_format(
        r#"
function refs(v0: ref<int32, managed, mutable, nullable>, v1: ref<int32, unique, readonly>): ref<int32, managed, mutable, nullable> {
entry(v0: ref<int32, managed, mutable, nullable>, v1: ref<int32, unique, readonly>):
    v2: ref<int32, managed, mutable, nullable> = null
    v3: ref<int32, managed, mutable, undefined> = undefined
    v4: slice<int32, managed, mutable, nullish> = undefined
    v5: tensorView<int32, managed, mutable, nullish, (2, 2)> = null
    return v2
}
"#,
    );
}

/// Formats each reference storage canonically.
#[test]
fn test_format_reference_storage() {
    assert_format(
        r#"
function storage(v0: ref<int32, raw, mutable, shared>, v1: ref<int32, raw, mutable, frame>, v2: ref<int32, raw, mutable, constant>, v3: ref<int32, raw, mutable, global>, v4: ref<int32, raw, mutable, shared global>): ref<int32, raw, mutable, shared> {
entry(v0: ref<int32, raw, mutable, shared>, v1: ref<int32, raw, mutable, frame>, v2: ref<int32, raw, mutable, constant>, v3: ref<int32, raw, mutable, global>, v4: ref<int32, raw, mutable, shared global>):
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
function borrowParam<'L0>(v0: ref<int32, borrowed, 'L0, mutable>): ref<int32, borrowed, 'L0, mutable> {
entry(v0: ref<int32, borrowed, 'L0, mutable>):
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
function staticBorrow(v0: ref<int32, borrowed, 'static, mutable>): ref<int32, borrowed, 'static, mutable> {
entry(v0: ref<int32, borrowed, 'static, mutable>):
    return v0
}
"#,
    );
}

/// Formats frame borrow lifetimes independently from frame storage.
#[test]
fn test_format_frame_borrow_lifetime() {
    assert_format(
        r#"
function frameBorrow(v0: ref<int32, borrowed, 'frame, mutable, frame>): ref<int32, borrowed, 'frame, mutable, frame> {
entry(v0: ref<int32, borrowed, 'frame, mutable, frame>):
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
type Player<'LWorld, 'LMesh> {
    world: ref<int32, borrowed, 'LWorld, mutable>;
    mesh: ref<float64, borrowed, 'LMesh, mutable>;
}

function tickPlayer<'LPlayer, 'LWorld, 'LMesh>(v0: ref<Player<'LWorld, 'LMesh>, borrowed, 'LPlayer, mutable>): void {
entry(v0: ref<Player<'LWorld, 'LMesh>, borrowed, 'LPlayer, mutable>):
    return
}
"#,
    );
}

/// Formats concrete type instances with applied lifetime arguments.
#[test]
fn test_format_type_instances() {
    assert_format(
        r#"
type Box<int32, 'L0> {
    value: ref<int32, borrowed, 'L0, readonly>;
}

function borrow(v0: Box<int32, 'static>): Box<int32, 'static> {
entry(v0: Box<int32, 'static>):
    return v0
}
"#,
    );
}

/// Formats declared outlives rows between lifetime parameters.
#[test]
fn test_format_lifetime_outlives_rows() {
    assert_format(
        r#"
function pass<'LA, 'LC>(v0: ref<int32, borrowed, 'LA, mutable>): ref<int32, borrowed, 'LC, mutable> where 'LA: 'LC {
entry(v0: ref<int32, borrowed, 'LA, mutable>):
    return v0
}
"#,
    );
}

/// Formats borrowed shaped views canonically.
#[test]
fn test_format_borrowed_shaped_views() {
    assert_format(
        r#"
function views<'L0>(v0: slice<int32, borrowed, 'L0, readonly>, v1: tensorView<int32, borrowed, 'L0, mutable, (4, 4)>): void {
entry(v0: slice<int32, borrowed, 'L0, readonly>, v1: tensorView<int32, borrowed, 'L0, mutable, (4, 4)>):
    return
}
"#,
    );
}

/// Formats tensor shapes and formats canonically.
#[test]
fn test_format_tensor_shapes_and_formats() {
    assert_format(
        r#"
function tensors<'L0>(v0: tensor<float32, managed, mutable, shared, (batch, dynamic, 64), format(dense(columnMajor))>, v1: tensorView<float32, borrowed, 'L0, readonly, (batch, dynamic, 64), format(strided)>): void {
entry(v0: tensor<float32, managed, mutable, shared, (batch, dynamic, 64), format(dense(columnMajor))>, v1: tensorView<float32, borrowed, 'L0, readonly, (batch, dynamic, 64), format(strided)>):
    return
}
"#,
    );
}

/// Formats tuple and fixed array type forms canonically.
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
function callbacks(v0: fn(int32, int32) => int64, v1: function<(int32) => int32, once, managed, mutable, nullish>): function<(int32) => int32, once, managed, mutable, nullish> {
entry(v0: fn(int32, int32) => int64, v1: function<(int32) => int32, once, managed, mutable, nullish>):
    return v1
}
"#,
    );
}

/// Formats uninhabited and execution handle types canonically.
#[test]
fn test_format_execution_types() {
    assert_format(
        r#"
function executionTypes(v0: continuation<void, never, int32>, v1: waiter<int32>): continuation<void, never, int32> {
entry(v0: continuation<void, never, int32>, v1: waiter<int32>):
    return v0
}
"#,
    );
}

/// Formats explicit initialization and destruction storage forms canonically.
#[test]
fn test_format_storage_forms() {
    assert_format(
        r#"
function storageForms(v0: uninit<int32>, v1: manual<int32>): manual<int32> {
entry(v0: uninit<int32>, v1: manual<int32>):
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

function erased(v0: dynamic<Writer, managed, mutable>): dynamic<Writer, managed, mutable> {
entry(v0: dynamic<Writer, managed, mutable>):
    return v0
}

function sharedErased(v0: dynamic<Writer, managed, mutable, nullish, shared>): dynamic<Writer, managed, mutable, nullish, shared> {
entry(v0: dynamic<Writer, managed, mutable, nullish, shared>):
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
    next: ref<Node, managed, mutable>;
}

function usePoint(v0: ref<Point, managed, mutable>, v1: ref<Node, managed, mutable>): ref<Point, managed, mutable> {
entry(v0: ref<Point, managed, mutable>, v1: ref<Node, managed, mutable>):
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
    next: ref<Node, managed, mutable>;
}

function usePoint(v0: ref<Point, managed, mutable>, v1: ref<Node, managed, mutable>): ref<Point, managed, mutable> {
entry(v0: ref<Point, managed, mutable>, v1: ref<Node, managed, mutable>):
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
type OwnedPair {
    ref<int32, unique, mutable>;
    ref<int32, unique, mutable>;
}

@copy
type CopyPair {
    int32;
    int32;
}
"#,
    );
}

/// Formats synthetic copy markers for built types.
#[test]
fn test_format_synthetic_copy_marker() {
    let mut tree = Tree::new();
    let strings = StringPool::new();

    let int32_type = tree.intern_type(Type::Int {
        width: 32,
        is_signed: true,
    });
    let declaration_name = strings.intern("Pair");

    let left = tree.intern_field(
        Field {
            name: None,
            ty: int32_type,
        },
        Vec::new(),
    );
    let right = tree.intern_field(
        Field {
            name: None,
            ty: int32_type,
        },
        Vec::new(),
    );
    let struct_type = tree.intern_type(Type::Struct {
        fields: vec![left, right],
        copy: Copy::Yes,
    });
    let representation = tree.get(struct_type).clone();
    let pair = tree.reserve_type(Symbol::named(declaration_name));
    tree.define_type(pair, representation);
    tree.insert_type_declaration(declaration_name, Vec::new(), Vec::new(), pair);

    let output = format_tree_with_options(&tree, &strings, FormatOptions::default());

    assert_output_eq(
        r#"
@copy
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

    let int32_type = tree.intern_type(Type::Int {
        width: 32,
        is_signed: true,
    });
    let attribute_name = strings.intern("packed");
    let declaration_name = strings.intern("Point");
    let field_name = strings.intern("x");

    let field_id = tree.intern_field(
        Field {
            name: Some(field_name),
            ty: int32_type,
        },
        vec![Attribute {
            name: AttributeIdentifier::identifier(attribute_name),
            args: AttributeArgs::None,
        }],
    );

    let struct_type = tree.intern_type(Type::Struct {
        fields: vec![field_id],
        copy: Copy::Yes,
    });
    let representation = tree.get(struct_type).clone();
    let point = tree.reserve_type(Symbol::named(declaration_name));
    tree.define_type(point, representation);
    tree.insert_type_declaration(declaration_name, Vec::new(), Vec::new(), point);

    let output = format_tree_with_options(&tree, &strings, FormatOptions::default());

    assert_output_eq(
        r#"
@copy
type Point {
    @packed
    x: int32;
}
"#
        .trim(),
        output,
    );
}
