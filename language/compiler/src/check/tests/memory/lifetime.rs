use crate::tests::{DirRows, TestSession};

#[test]
fn test_return_lifetime_infers_from_body() {
    let session = TestSession::single(
        r#"
struct Node {
    id: int32;
}

function first(a: &Node, b: &Node): &Node {
    return a;
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

function first<comptime L0: Lifetime, comptime L1: Lifetime>(
    a: Borrowed<Node, L0, "mutable">,
    b: Borrowed<Node, L1, "mutable">,
): Borrowed<Node, L0, "mutable"> {
    return a;
}

=== checked ===
struct Node {
/// @type.symbol symbol=Node type=Node
/// @definition.struct symbol=Node
/// @definition.field symbol=Node.id source="id: int32" key=id type=int32

    id: int32;
    /// @type.symbol symbol=Node.id source="id: int32" type=int32

}

function first(a: &Node, b: &Node): &Node {
/// @generic.template symbol=first parameters=(comptime L0: Lifetime, comptime L1: Lifetime)
/// @type.symbol symbol=first type=<comptime first.L0: Lifetime, comptime first.L1: Lifetime>(Borrowed<Node, first.L0, "mutable">, Borrowed<Node, first.L1, "mutable">) => Borrowed<Node, first.L0, "mutable">
/// @type.symbol symbol=first.a source="a: &Node" type=Borrowed<Node, first.L0, "mutable">
/// @resolution.name source=Node target=Node
/// @type.symbol symbol=first.b source="b: &Node" type=Borrowed<Node, first.L1, "mutable">
/// @resolution.name source=Node target=Node
/// @resolution.name source=Node target=Node

    return a;
    /// @type.node source=a type=Borrowed<Node, first.L0, "mutable">
    /// @resolution.name source=a target=first.a

}
"#);
}

#[test]
fn test_conditional_return_lifetime_joins_branch_lifetimes() {
    let session = TestSession::single(
        r#"
struct Node {
    id: int32;
}

function choose(a: &Node, b: &Node, flag: boolean): &Node {
    return flag ? a : b;
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

function choose<comptime L0: Lifetime, comptime L1: Lifetime>(
    a: Borrowed<Node, L0, "mutable">,
    b: Borrowed<Node, L1, "mutable">,
    flag: boolean,
): Borrowed<Node, L0 | L1, "mutable"> {
    return flag ? a : b;
}

=== checked ===
struct Node {
/// @type.symbol symbol=Node type=Node
/// @definition.struct symbol=Node
/// @definition.field symbol=Node.id source="id: int32" key=id type=int32

    id: int32;
    /// @type.symbol symbol=Node.id source="id: int32" type=int32

}

function choose(a: &Node, b: &Node, flag: boolean): &Node {
/// @generic.template symbol=choose parameters=(comptime L0: Lifetime, comptime L1: Lifetime)
/// @type.symbol symbol=choose type=<comptime choose.L0: Lifetime, comptime choose.L1: Lifetime>(Borrowed<Node, choose.L0, "mutable">, Borrowed<Node, choose.L1, "mutable">, boolean) => Borrowed<Node, choose.L0 | choose.L1, "mutable">
/// @type.symbol symbol=choose.a source="a: &Node" type=Borrowed<Node, choose.L0, "mutable">
/// @resolution.name source=Node target=Node
/// @type.symbol symbol=choose.b source="b: &Node" type=Borrowed<Node, choose.L1, "mutable">
/// @resolution.name source=Node target=Node
/// @type.symbol symbol=choose.flag source="flag: boolean" type=boolean
/// @resolution.name source=Node target=Node

    return flag ? a : b;
    /// @type.node source="flag ? a : b" type=Borrowed<Node, choose.L0, "mutable"> | Borrowed<Node, choose.L1, "mutable">
    /// @type.node source=flag type=boolean
    /// @resolution.name source=flag target=choose.flag
    /// @type.node source=a type=Borrowed<Node, choose.L0, "mutable">
    /// @resolution.name source=a target=choose.a
    /// @type.node source=b type=Borrowed<Node, choose.L1, "mutable">
    /// @resolution.name source=b target=choose.b

}
"#);
}

#[test]
fn test_ambient_elided_return_lifetime_reports_error() {
    let session = TestSession::single(
        r#"
struct Node {
    id: int32;
}

declare function choose<comptime L0: Lifetime, comptime L1: Lifetime>(
    a: Borrowed<Node, L0, "mutable">,
    b: Borrowed<Node, L1, "mutable">,
): &Node;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Node {
    id: int32;
}

declare function choose<comptime L0: Lifetime, comptime L1: Lifetime>(
    a: Borrowed<Node, L0, "mutable">,
    b: Borrowed<Node, L1, "mutable">,
): &Node;

=== checked ===
struct Node {
/// @type.symbol symbol=Node type=Node
/// @definition.struct symbol=Node
/// @definition.field symbol=Node.id source="id: int32" key=id type=int32

    id: int32;
    /// @type.symbol symbol=Node.id source="id: int32" type=int32

}

declare function choose<comptime L0: Lifetime, comptime L1: Lifetime>(
/// @generic.template symbol=choose parameters=(comptime L0: Lifetime, comptime L1: Lifetime)
/// @type.symbol symbol=choose type=<comptime L0: Lifetime, comptime L1: Lifetime>(Borrowed<Node, L0, "mutable">, Borrowed<Node, L1, "mutable">) => Borrowed<Node, <error>, "mutable">
/// @type.symbol symbol=choose.L0 source="comptime L0: Lifetime" type=L0
/// @resolution.name source=Lifetime target=memory.lifetime.Lifetime
/// @type.symbol symbol=choose.L1 source="comptime L1: Lifetime" type=L1
/// @resolution.name source=Lifetime target=memory.lifetime.Lifetime

    a: Borrowed<Node, L0, "mutable">,
    /// @type.symbol symbol=choose.a source="a: Borrowed<Node, L0, \"mutable\">" type=Borrowed<Node, L0, "mutable">
    /// @resolution.name source=Borrowed target=memory.borrow.Borrowed
    /// @resolution.name source=Node target=Node
    /// @resolution.name source=L0 target=choose.L0

    b: Borrowed<Node, L1, "mutable">,
    /// @type.symbol symbol=choose.b source="b: Borrowed<Node, L1, \"mutable\">" type=Borrowed<Node, L1, "mutable">
    /// @resolution.name source=Borrowed target=memory.borrow.Borrowed
    /// @resolution.name source=Node target=Node
    /// @resolution.name source=L1 target=choose.L1

): &Node;
/// @resolution.name source=Node target=Node

/// @generic.instance id="Borrowed<Node, L0, \"mutable\">" template=memory.borrow.Borrowed arguments=(Node, L0, "mutable")
/// @generic.instance id="Borrowed<Node, L1, \"mutable\">" template=memory.borrow.Borrowed arguments=(Node, L1, "mutable")
"#,
        r#"
/// @diagnostic.error code=EC614 message="ambient signatures must spell result lifetimes explicitly"
/// @diagnostic.label line=6 column=18 span="choose" line_source="declare function choose<comptime L0: Lifetime, comptime L1: Lifetime>("
"#,
    );
}

#[test]
fn test_stored_borrow_field_infers_hidden_lifetime() {
    let session = TestSession::single(
        r#"
struct Engine {
    frame: uint64;
}

struct AssetStore {
    count: uint32;
}

struct WorldView {
    engine: &Engine;
    assets: &AssetStore;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Engine {
    frame: uint64;
}

struct AssetStore {
    count: uint32;
}

struct WorldView<comptime L0: Lifetime, comptime L1: Lifetime> {
    engine: Borrowed<Engine, L0, "mutable">;
    assets: Borrowed<AssetStore, L1, "mutable">;
}

=== checked ===
struct Engine {
/// @type.symbol symbol=Engine type=Engine
/// @definition.struct symbol=Engine
/// @definition.field symbol=Engine.frame source="frame: uint64" key=frame type=uint64

    frame: uint64;
    /// @type.symbol symbol=Engine.frame source="frame: uint64" type=uint64

}

struct AssetStore {
/// @type.symbol symbol=AssetStore type=AssetStore
/// @definition.struct symbol=AssetStore
/// @definition.field symbol=AssetStore.count source="count: uint32" key=count type=uint32

    count: uint32;
    /// @type.symbol symbol=AssetStore.count source="count: uint32" type=uint32

}

struct WorldView {
/// @generic.template symbol=WorldView parameters=(comptime L0: Lifetime, comptime L1: Lifetime)
/// @type.symbol symbol=WorldView type=WorldView
/// @definition.struct symbol=WorldView
/// @definition.field symbol=WorldView.assets source="assets: &AssetStore" key=assets type=Borrowed<AssetStore, WorldView.L1, "mutable">
/// @definition.field symbol=WorldView.engine source="engine: &Engine" key=engine type=Borrowed<Engine, WorldView.L0, "mutable">

    engine: &Engine;
    /// @type.symbol symbol=WorldView.engine source="engine: &Engine" type=Borrowed<Engine, WorldView.L0, "mutable">
    /// @resolution.name source=Engine target=Engine

    assets: &AssetStore;
    /// @type.symbol symbol=WorldView.assets source="assets: &AssetStore" type=Borrowed<AssetStore, WorldView.L1, "mutable">
    /// @resolution.name source=AssetStore target=AssetStore

}
"#,
    );
}
