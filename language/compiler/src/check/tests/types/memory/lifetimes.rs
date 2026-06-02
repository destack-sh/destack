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
struct Node {
/// @type.symbol symbol=Node type=Node

    id: int32;
    /// @type.symbol symbol=Node.id type=int32

}

function first(a: &Node, b: &Node): &Node {
/// @type.symbol symbol=first type=(Borrowed<Node, first.L0, "mutable">, Borrowed<Node, first.L1, "mutable">) => Borrowed<Node, first.L0, "mutable">
/// @generic.template symbol=first parameters=[comptime L0: memory.lifetime.Lifetime, comptime L1: memory.lifetime.Lifetime, comptime L2: memory.lifetime.Lifetime]
/// @type.symbol symbol=a type=Borrowed<Node, first.L0, "mutable">
/// @type.symbol symbol=b type=Borrowed<Node, first.L1, "mutable">

    return a;
    /// @type.node source=a type=Borrowed<Node, first.L0, "mutable">
    /// @resolution.name source=a target=a

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
struct Node {
/// @type.symbol symbol=Node type=Node

    id: int32;
    /// @type.symbol symbol=Node.id type=int32

}

function choose(a: &Node, b: &Node, flag: boolean): &Node {
/// @type.symbol symbol=choose type=(Borrowed<Node, choose.L0, "mutable">, Borrowed<Node, choose.L1, "mutable">, boolean) => Borrowed<Node, choose.L0 | choose.L1, "mutable">
/// @generic.template symbol=choose parameters=[comptime L0: memory.lifetime.Lifetime, comptime L1: memory.lifetime.Lifetime, comptime L2: memory.lifetime.Lifetime]
/// @type.symbol symbol=a type=Borrowed<Node, choose.L0, "mutable">
/// @type.symbol symbol=b type=Borrowed<Node, choose.L1, "mutable">
/// @type.symbol symbol=flag type=boolean

    return flag ? a : b;
    /// @type.node source="flag ? a : b" type=Borrowed<Node, choose.L0 | choose.L1, "mutable">
    /// @type.node source=flag type=boolean
    /// @resolution.name source=flag target=flag
    /// @type.node source=a type=Borrowed<Node, choose.L0, "mutable">
    /// @resolution.name source=a target=a
    /// @type.node source=b type=Borrowed<Node, choose.L1, "mutable">
    /// @resolution.name source=b target=b

}
"#);
}

#[test]
fn test_ambient_return_type_uses_lifetime_slot() {
    let session = TestSession::single(
        r#"
struct Node {
    id: int32;
}

declare function choose(a: &Node, b: &Node): &Node;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
struct Node {
/// @type.symbol symbol=Node type=Node

    id: int32;
    /// @type.symbol symbol=Node.id type=int32

}

declare function choose(a: &Node, b: &Node): &Node;
/// @type.symbol symbol=choose type=(Borrowed<Node, choose.L0, "mutable">, Borrowed<Node, choose.L1, "mutable">) => Borrowed<Node, choose.L2, "mutable">
/// @generic.template symbol=choose parameters=[comptime L0: memory.lifetime.Lifetime, comptime L1: memory.lifetime.Lifetime, comptime L2: memory.lifetime.Lifetime]
/// @type.symbol symbol=a type=Borrowed<Node, choose.L0, "mutable">
/// @type.symbol symbol=b type=Borrowed<Node, choose.L1, "mutable">

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
struct Engine {
/// @type.symbol symbol=Engine type=Engine

    frame: uint64;
    /// @type.symbol symbol=Engine.frame type=uint64

}

struct AssetStore {
/// @type.symbol symbol=AssetStore type=AssetStore

    count: uint32;
    /// @type.symbol symbol=AssetStore.count type=uint32

}

struct WorldView {
/// @type.symbol symbol=WorldView type=WorldView<WorldView.L0, WorldView.L1>
/// @generic.template symbol=WorldView parameters=[comptime L0: memory.lifetime.Lifetime, comptime L1: memory.lifetime.Lifetime]

    engine: &Engine;
    /// @type.symbol symbol=WorldView.engine type=Borrowed<Engine, WorldView.L0, "mutable">

    assets: &AssetStore;
    /// @type.symbol symbol=WorldView.assets type=Borrowed<AssetStore, WorldView.L1, "mutable">

}
"#,
    );
}
