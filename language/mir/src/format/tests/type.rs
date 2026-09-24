use super::{assert_format, assert_format_eq, assert_output_eq, format_tree_with_options};
use crate::{
    Attribute, AttributeArgs, AttributeIdentifier, Field, FormatOptions, Symbol, Tree, Type,
};
use destack_core::StringPool;

/// Formats pointer-sized builtin types canonically.
#[test]
fn test_format_pointer_sized_builtin_types() {
    assert_format(
        r#"
function pointerSized(v0: isize, v1: usize, v2: typeId): usize {
entry(v0: isize, v1: usize, v2: typeId):
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
function floats(v0: float32, v1: float64): void {
entry(v0: float32, v1: float64):
    v2: float32 = 1.5
    v3: float64 = 1.5
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

/// Formats reference kinds canonically.
#[test]
fn test_format_reference_kinds() {
    assert_format(
        r#"
function refs(v0: ref<int32, managed, mutable, local>, v1: ref<int32, unique, readonly>): ref<int32, managed, mutable, local> {
entry(v0: ref<int32, managed, mutable, local>, v1: ref<int32, unique, readonly>):
    return v0
}
"#,
    );
}

/// Formats process-local machine pointers canonically.
#[test]
fn test_format_pointer_access() {
    assert_format(
        r#"
function pointers(v0: ptr<int32, readonly>, v1: ptr<int32, mutable>, v2: ptr<int32, mutable>): ptr<int32, mutable> {
entry(v0: ptr<int32, readonly>, v1: ptr<int32, mutable>, v2: ptr<int32, mutable>):
    v3: ptr<int32, mutable> = null
    v4: ref<int32, managed, mutable, local> = cast.pointerToReference v1 -> ref<int32, managed, mutable, local>
    v5: ptr<int32, mutable> = cast.referenceToPointer v4 -> ptr<int32, mutable>
    return v1
}
"#,
    );
}

/// Formats the null type as its keyword, distinct from void.
#[test]
fn test_format_null_type() {
    assert_format(
        r#"
type Nullish = variant<uint2> { 0uint2 = void; 1uint2 = null; 2uint2 = int32; };

function nullish(v0: Nullish, v1: null): null {
entry(v0: Nullish, v1: null):
    return v1
}
"#,
    );
}

/// Formats parameter borrow lifetimes canonically.
#[test]
fn test_format_parameter_borrow_lifetime() {
    assert_format(
        r#"
function borrowParam<'a>(v0: ref<int32, borrowed, 'a, mutable>): ref<int32, borrowed, 'a, mutable> {
entry(v0: ref<int32, borrowed, 'a, mutable>):
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
function frameBorrow(v0: ref<int32, borrowed, 'frame, mutable>): ref<int32, borrowed, 'frame, mutable> {
entry(v0: ref<int32, borrowed, 'frame, mutable>):
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

/// Formats opaque declarations and their applications canonically.
#[test]
fn test_format_opaque_declarations() {
    assert_format(
        r#"
type Iterable<T> = void;

type Iterable.Iterator<T, Self>;

external function Iterable.iterator<T, U: Iterable<T>>(U): Iterable.Iterator<T, U>

function first<T, U: Iterable<T>>(v0: U): Iterable.Iterator<T, U> {
entry(v0: U):
    v1: Iterable.Iterator<T, U> = call.witness U, Iterable<T>, Iterable.iterator(v0): (U) => Iterable.Iterator<T, U>
    return v1
}
"#,
    );
}

/// Formats template applications with their generic and lifetime arguments.
#[test]
fn test_format_type_applications() {
    assert_format(
        r#"
type Box<T, 'a> {
    value: ref<T, borrowed, 'a, readonly>;
}

function borrow(v0: Box<int32, 'static>): Box<int32, 'static> {
entry(v0: Box<int32, 'static>):
    return v0
}
"#,
    );
}

/// Formats declared outlives bounds between lifetime parameters.
#[test]
fn test_format_lifetime_outlives_bounds() {
    assert_format(
        r#"
function pass<'LA, 'LC>(v0: ref<int32, borrowed, 'LA, mutable>): ref<int32, borrowed, 'LC, mutable> where 'LA: 'LC {
entry(v0: ref<int32, borrowed, 'LA, mutable>):
    return v0
}
"#,
    );
}

/// Formats borrowed slices canonically.
#[test]
fn test_format_borrowed_slices() {
    assert_format(
        r#"
function views<'a>(v0: slice<int32, borrowed, 'a, readonly>): void {
entry(v0: slice<int32, borrowed, 'a, readonly>):
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
function callbacks(v0: fn(int32, int32) => int64, v1: function<(int32) => int32, once, managed, mutable, local>): function<(int32) => int32, once, managed, mutable, local> {
entry(v0: fn(int32, int32) => int64, v1: function<(int32) => int32, once, managed, mutable, local>):
    return v1
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

function erased(v0: dynamic<Writer, managed, mutable, local>): dynamic<Writer, managed, mutable, local> {
entry(v0: dynamic<Writer, managed, mutable, local>):
    return v0
}

function sharedErased(v0: dynamic<Writer, managed, mutable, shared>): dynamic<Writer, managed, mutable, shared> {
entry(v0: dynamic<Writer, managed, mutable, shared>):
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
    next: ref<Node, managed, mutable, local>;
}

function usePoint(v0: ref<Point, managed, mutable, local>, v1: ref<Node, managed, mutable, local>): ref<Point, managed, mutable, local> {
entry(v0: ref<Point, managed, mutable, local>, v1: ref<Node, managed, mutable, local>):
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
    next: ref<Node, managed, mutable, local>;
}

function usePoint(v0: ref<Point, managed, mutable, local>, v1: ref<Node, managed, mutable, local>): ref<Point, managed, mutable, local> {
entry(v0: ref<Point, managed, mutable, local>, v1: ref<Node, managed, mutable, local>):
    return v0
}
"#,
    );
}

/// Formats direct nominal heritage canonically.
#[test]
fn test_format_type_heritage() {
    assert_format(
        r#"
type Parent { }

type Base { }

type Left { }

type Right { }

type Child extends Parent, Base implements Left, Right { }
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

    let left = tree.intern_field(Field {
        name: None,
        ty: int32_type,
        attributes: Vec::new(),
    });
    let right = tree.intern_field(Field {
        name: None,
        ty: int32_type,
        attributes: Vec::new(),
    });
    let struct_type = tree.intern_type(Type::Struct {
        fields: vec![left, right],
    });
    let pair = tree.reserve_type(Symbol::named(crate::TEST_MODULE, declaration_name));
    let declaration = tree.get_mut(pair);
    declaration.definition = Some(struct_type);
    declaration.name = Some(declaration_name);

    let output = format_tree_with_options(&tree, &strings, FormatOptions::default());

    assert_output_eq(
        r#"
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

    let field_id = tree.intern_field(Field {
        name: Some(field_name),
        ty: int32_type,
        attributes: vec![Attribute {
            name: AttributeIdentifier::identifier(attribute_name),
            args: AttributeArgs::None,
        }],
    });

    let struct_type = tree.intern_type(Type::Struct {
        fields: vec![field_id],
    });
    let point = tree.reserve_type(Symbol::named(crate::TEST_MODULE, declaration_name));
    let declaration = tree.get_mut(point);
    declaration.definition = Some(struct_type);
    declaration.name = Some(declaration_name);

    let output = format_tree_with_options(&tree, &strings, FormatOptions::default());

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

/// Preserve access modes on references, slices, and generic applications.
#[test]
fn test_format_borrow_qualifiers() {
    assert_format(
        r#"
type Borrows<T, 'a, A: Access> {
    mutable: ref<T, borrowed, 'a, mutable>;
    readonly: ref<T, borrowed, 'a, readonly>;
    exclusive: ref<T, borrowed, 'a, exclusive>;
    immutable: ref<T, borrowed, 'a, immutable>;
    generic: ref<T, borrowed, 'a, A>;
    raw: ref<T, raw, mutable>;
    rawSlice: slice<T, raw, readonly>;
    slice: slice<T, borrowed, 'a, immutable>;
    erased: ref<T, borrowed, '_, immutable>;
    @held: ref<T, borrowed, 'a, readonly>;
}

type Applied = newtype<Borrows<int32, 'static, immutable>>;
"#,
    );
}

/// Preserve forward recursive declarations and explicit variant tags in source order.
#[test]
fn test_format_recursive_variant_declarations() {
    assert_format(
        r#"
type Node<T> {
    next: ref<Node<T>, managed, mutable, local>;
    value: T;
}

type Choice = variant<uint8> { 7uint8 = ref<Node<int32>, unique, mutable>; 2uint8 = int32; };
"#,
    );
}

/// Preserve captured lifetimes, their spaces, and bounds across nested binders.
#[test]
fn test_format_nested_lifetime_binders() {
    assert_format(
        "type Callback = <'a>(<'b>(ref<int32, borrowed, 'a, readonly>, ref<int32, borrowed, 'b, readonly>) => ref<int32, borrowed, 'a, readonly> where 'b: 'a) => void;",
    );
    assert_format_eq(
        "type Callback = <'a>(<'a>(ref<int32, borrowed, 'a, readonly>) => void) => void;",
        "type Callback = <'a>(<'a_1>(ref<int32, borrowed, 'a_1, readonly>) => void) => void;",
    );
}
