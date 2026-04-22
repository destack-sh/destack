use super::{assert_format, assert_format_eq, assert_output_eq, format_tree_with_options};
use crate::{
    Attribute, AttributeArgs, AttributeIdentifier, Copy, Field, MirFormatOptions, NodeTree, Type,
    TypeAlias,
};
use destack_core::StringPool;

/// Formats richer reference and builtin types canonically.
#[test]
fn test_format_reference_and_builtin_types() {
    assert_format(
        r#"
function pointerSized(value0: isize, value1: usize, value2: typeDescriptor, value3: typeId, value4: ref?<int32, managed>, value5: ref<int32, raw, space(shared)>, value6: ref<int32, raw, space(gpu)>, value7: ref<int32, owned, readonly>): ref?<int32, managed> {
entry0(value0: isize, value1: usize, value2: typeDescriptor, value3: typeId, value4: ref?<int32, managed>, value5: ref<int32, raw, space(shared)>, value6: ref<int32, raw, space(gpu)>, value7: ref<int32, owned, readonly>):
    return value4
}
"#,
    );
}

/// Formats aggregate and callable type forms canonically.
#[test]
fn test_format_aggregate_and_callable_types() {
    assert_format(
        r#"
function shapes(value0: (int32, float64, boolean), value1: int32[10], value2: (int32, int32) -> int64, value3: (int32) => int32, value4: { x: int32, y: float64 }): { x: int32, y: float64 } {
entry0(value0: (int32, float64, boolean), value1: int32[10], value2: (int32, int32) -> int64, value3: (int32) => int32, value4: { x: int32, y: float64 }):
    return value4
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

/// Formats attributed struct fields without parsed field spans.
#[test]
fn test_format_struct_fields_with_attributes_without_parsed_spans() {
    let mut tree = NodeTree::new();
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

    let strings = strings.into_immutable();
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
