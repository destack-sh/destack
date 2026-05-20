use crate::tests::{DirRows, TestSession};

#[test]
fn test_check_infers_return_lifetime_from_body() {
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
        DirRows::checked(),
        r#"
struct Node {
/// @type.symbol symbol=Node type=Node

    id: int32;
    /// @type.symbol symbol=Node.id type=int32
}

function first(a: &Node, b: &Node): &Node {
/// @generic.slot symbol=first.L0 index=0 kind=static constraint=Lifetime
/// @generic.slot symbol=first.L1 index=1 kind=static constraint=Lifetime
/// @type.symbol symbol=first type=<first.L0: Lifetime, first.L1: Lifetime>(Borrowed<Node, first.L0, mutable>, Borrowed<Node, first.L1, mutable>) => Borrowed<Node, first.L0, mutable>

    return a;
    /// @resolution.name source=a target=a
    /// @type.node source=a type=Borrowed<Node, first.L0, mutable>
}
"#);
}

#[test]
fn test_check_infers_joined_return_lifetime_from_conditional_body() {
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
        DirRows::checked(),
        r#"
struct Node {
/// @type.symbol symbol=Node type=Node

    id: int32;
    /// @type.symbol symbol=Node.id type=int32
}

function choose(a: &Node, b: &Node, flag: boolean): &Node {
/// @generic.slot symbol=choose.L0 index=0 kind=static constraint=Lifetime
/// @generic.slot symbol=choose.L1 index=1 kind=static constraint=Lifetime
/// @type.symbol symbol=choose type=<choose.L0: Lifetime, choose.L1: Lifetime>(Borrowed<Node, choose.L0, mutable>, Borrowed<Node, choose.L1, mutable>, boolean) => Borrowed<Node, join(choose.L0 | choose.L1), mutable>

    return flag ? a : b;
    /// @resolution.name source=flag target=flag
    /// @resolution.name source=a target=a
    /// @resolution.name source=b target=b
    /// @type.node source="flag ? a : b" type=Borrowed<Node, join(choose.L0 | choose.L1), mutable>
}
"#);
}

#[test]
fn test_check_reports_ambiguous_declaration_return_lifetime() {
    let session = TestSession::single(
        r#"
struct Node {
    id: int32;
}

declare function choose(a: &Node, b: &Node): &Node;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
struct Node {
/// @type.symbol symbol=Node type=Node

    id: int32;
    /// @type.symbol symbol=Node.id type=int32
}

declare function choose(a: &Node, b: &Node): &Node;
/// @generic.slot symbol=choose.L0 index=0 kind=static constraint=Lifetime
/// @generic.slot symbol=choose.L1 index=1 kind=static constraint=Lifetime

"#,
        r#"
/// @diagnostic.error code=EC206 message="borrowed return lifetime is ambiguous"
/// @diagnostic.label line=6 column=43 source="declare function choose(a: &Node, b: &Node): &Node;"
"#,
    );
}

#[test]
fn test_check_infers_hidden_lifetime_for_stored_borrow_fields() {
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
/// @generic.slot symbol=WorldView.L0 index=0 kind=static constraint=Lifetime
/// @generic.slot symbol=WorldView.L1 index=1 kind=static constraint=Lifetime
/// @type.symbol symbol=WorldView type=WorldView<WorldView.L0, WorldView.L1>

    engine: &Engine;
    /// @type.symbol symbol=WorldView.engine type=Borrowed<Engine, WorldView.L0, mutable>

    assets: &AssetStore;
    /// @type.symbol symbol=WorldView.assets type=Borrowed<AssetStore, WorldView.L1, mutable>
}
"#,
    );
}
