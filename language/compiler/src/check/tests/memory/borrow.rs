use crate::tests::{DirRows, TestSession};

#[test]
fn test_borrow_parameter_shorthands_expand_to_borrowed_forms() {
    let session = TestSession::single(
        r#"
struct Node {
    id: int32;
}

function access(read: &readonly Node, write: &Node, exclusive: &exclusive Node): void {
    read.id;
    write.id;
    exclusive.id;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Node {
    id: int32;
}

function access<comptime L0: Lifetime, comptime L1: Lifetime, comptime L2: Lifetime>(
    read: Borrowed<Node, L0, "readonly">,
    write: Borrowed<Node, L1, "mutable">,
    exclusive: Borrowed<Node, L2, "exclusive">,
): void {
    read.id;
    write.id;
    exclusive.id;
}

=== checked ===
struct Node {
/// @type.symbol symbol=Node type=Node
/// @definition.struct symbol=Node
/// @definition.field symbol=Node.id source="id: int32" key=id type=int32

    id: int32;
    /// @type.symbol symbol=Node.id source="id: int32" type=int32

}

function access(read: &readonly Node, write: &Node, exclusive: &exclusive Node): void {
/// @generic.template symbol=access parameters=(comptime L0: Lifetime, comptime L1: Lifetime, comptime L2: Lifetime)
/// @type.symbol symbol=access type=<comptime access.L0: Lifetime, comptime access.L1: Lifetime, comptime access.L2: Lifetime>(Borrowed<Node, access.L0, "readonly">, Borrowed<Node, access.L1, "mutable">, Borrowed<Node, access.L2, "exclusive">) => void
/// @type.symbol symbol=access.read source="read: &readonly Node" type=Borrowed<Node, access.L0, "readonly">
/// @resolution.name source=Node target=Node
/// @type.symbol symbol=access.write source="write: &Node" type=Borrowed<Node, access.L1, "mutable">
/// @resolution.name source=Node target=Node
/// @type.symbol symbol=access.exclusive source="exclusive: &exclusive Node" type=Borrowed<Node, access.L2, "exclusive">
/// @resolution.name source=Node target=Node

    read.id;
    /// @type.node source=read type=Borrowed<Node, access.L0, "readonly">
    /// @type.node source=read.id type=int32
    /// @resolution.name source=read target=access.read
    /// @resolution.member source=read.id receiver=Borrowed<Node, access.L0, "readonly"> kind=symbol target=Node.id

    write.id;
    /// @type.node source=write type=Borrowed<Node, access.L1, "mutable">
    /// @type.node source=write.id type=int32
    /// @resolution.name source=write target=access.write
    /// @resolution.member source=write.id receiver=Borrowed<Node, access.L1, "mutable"> kind=symbol target=Node.id

    exclusive.id;
    /// @type.node source=exclusive type=Borrowed<Node, access.L2, "exclusive">
    /// @type.node source=exclusive.id type=int32
    /// @resolution.name source=exclusive target=access.exclusive
    /// @resolution.member source=exclusive.id receiver=Borrowed<Node, access.L2, "exclusive"> kind=symbol target=Node.id

}
"#);
}

#[test]
fn test_borrow_expression_preserves_field_path() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

let point = ^Point { x: 1 };
let x = &readonly point.x;

x satisfies &readonly int32;
point.x satisfies int32;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

let point: ^Point = ^Point { x: 1 };
let x: Borrowed<int32, "static", "readonly"> = &readonly point.x;

x satisfies &readonly int32;
point.x satisfies int32;

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

let point = ^Point { x: 1 };
/// @type.symbol symbol=point source=point type=Owned<Point> reduced=Point
/// @type.node source="^Point { x: 1 }" type=Owned<Point> reduced=Point
/// @type.node source="Point { x: 1 }" type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=1 type=1

let x = &readonly point.x;
/// @type.symbol symbol=x source=x type=Borrowed<int32, "static", "readonly">
/// @type.node source="&readonly point.x" type=Borrowed<int32, "static", "readonly">
/// @type.node source=point type=Owned<Point> reduced=Point
/// @type.node source=point.x type=int32
/// @resolution.name source=point target=point
/// @resolution.member source=point.x receiver=Point kind=symbol target=Point.x

x satisfies &readonly int32;
/// @type.node source="x satisfies &readonly int32" type=Borrowed<int32, "static", "readonly">
/// @type.node source=x type=Borrowed<int32, "static", "readonly">
/// @resolution.name source=x target=x

point.x satisfies int32;
/// @type.node source="point.x satisfies int32" type=int32
/// @type.node source=point type=Owned<Point> reduced=Point
/// @type.node source=point.x type=int32
/// @resolution.name source=point target=point
/// @resolution.member source=point.x receiver=Point kind=symbol target=Point.x
"#,
    );
}

#[test]
fn test_borrowed_fixed_array_preserves_length() {
    let session = TestSession::single(
        r#"
let values: [int32; 3] = [1, 2, 3];
let borrow = &readonly values;

borrow satisfies &readonly [int32; 3];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let values: [int32; 3] = [1, 2, 3];
let borrow: Borrowed<[int32; 3], "static", "readonly"> = &readonly values;

borrow satisfies &readonly [int32; 3];

=== checked ===
let values: [int32; 3] = [1, 2, 3];
/// @type.symbol symbol=values source=values type=FixedArray<int32, 3>
/// @type.node source=[1, 2, 3] type=FixedArray<int32, 3>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3

let borrow = &readonly values;
/// @type.symbol symbol=borrow source=borrow type=Borrowed<FixedArray<int32, 3>, "static", "readonly">
/// @type.node source="&readonly values" type=Borrowed<FixedArray<int32, 3>, "static", "readonly">
/// @type.node source=values type=FixedArray<int32, 3>
/// @resolution.name source=values target=values

borrow satisfies &readonly [int32; 3];
/// @type.node source="borrow satisfies &readonly [int32; 3]" type=Borrowed<FixedArray<int32, 3>, "static", "readonly">
/// @type.node source=borrow type=Borrowed<FixedArray<int32, 3>, "static", "readonly">
/// @resolution.name source=borrow target=borrow
"#,
    );
}
