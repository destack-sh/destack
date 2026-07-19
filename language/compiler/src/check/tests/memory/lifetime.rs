use crate::tests::{DirRows, TestSession};

#[test]
fn test_assign_module_and_function_borrows_static_and_frame_lifetimes() {
    let session = TestSession::single(
        r#"
struct Point { x: int32; }

const modulePoint = ^Point { x: 1 };
const moduleBorrow = &readonly modulePoint;

function inspectFrame(): void {
    const framePoint = ^Point { x: 2 };
    const frameBorrow = &readonly framePoint;

    moduleBorrow satisfies local Borrowed<Point, "static", "readonly">;
    frameBorrow satisfies local Borrowed<Point, "frame", "readonly">;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

const modulePoint: ^Point = ^Point { x: 1 };
const moduleBorrow: Borrowed<Point, "static", "readonly"> = &readonly modulePoint;

function inspectFrame(): void {
    const framePoint: ^Point = ^Point { x: 2 };
    const frameBorrow: Borrowed<Point, "frame", "readonly"> = &readonly framePoint;

    moduleBorrow satisfies local Borrowed<Point, "static", "readonly">;
    frameBorrow satisfies local Borrowed<Point, "frame", "readonly">;
}

=== checked ===
struct Point { x: int32; }
/// @type.symbol symbol=Point source="struct Point { x: int32; }" type=Point
/// @definition.struct symbol=Point source="struct Point { x: int32; }"
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @type.symbol symbol=Point.x source="x: int32" type=int32

const modulePoint = ^Point { x: 1 };
/// @type.symbol symbol=modulePoint source=modulePoint type=Owned<Point> reduced=Point
/// @resolution.name source=Point target=Point

const moduleBorrow = &readonly modulePoint;
/// @type.symbol symbol=moduleBorrow source=moduleBorrow type=Borrowed<Point, "static", "readonly">
/// @resolution.name source=modulePoint target=modulePoint

function inspectFrame(): void {
/// @type.symbol symbol=inspectFrame type=() => void

    const framePoint = ^Point { x: 2 };
    /// @type.symbol symbol=inspectFrame.framePoint source=framePoint type=Owned<Point> reduced=Point
    /// @resolution.name source=Point target=Point

    const frameBorrow = &readonly framePoint;
    /// @type.symbol symbol=inspectFrame.frameBorrow source=frameBorrow type=Borrowed<Point, "frame", "readonly">
    /// @resolution.name source=framePoint target=inspectFrame.framePoint

    moduleBorrow satisfies local Borrowed<Point, "static", "readonly">;
    /// @resolution.name source=moduleBorrow target=moduleBorrow
    /// @resolution.name source=Borrowed target=memory.borrow.Borrowed
    /// @resolution.name source=Point target=Point

    frameBorrow satisfies local Borrowed<Point, "frame", "readonly">;
    /// @resolution.name source=frameBorrow target=inspectFrame.frameBorrow
    /// @resolution.name source=Borrowed target=memory.borrow.Borrowed
    /// @resolution.name source=Point target=Point

}
"#,
    );
}

#[test]
fn test_infer_return_lifetime_from_function_body() {
    let session = TestSession::single(
        r#"
struct Node { id: int32; }

function first(a: &Node, b: &Node): &Node {
    return a;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
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
struct Node { id: int32; }
/// @type.symbol symbol=Node source="struct Node { id: int32; }" type=Node
/// @definition.struct symbol=Node source="struct Node { id: int32; }"
/// @definition.field symbol=Node.id source="id: int32" key=id type=int32
/// @type.symbol symbol=Node.id source="id: int32" type=int32

function first(a: &Node, b: &Node): &Node {
/// @generic.template symbol=first parameters=(comptime L0: Lifetime, comptime L1: Lifetime)
/// @type.symbol symbol=first type=<comptime first.L0: Lifetime, comptime first.L1: Lifetime>(Borrowed<Node, first.L0, "mutable">, Borrowed<Node, first.L1, "mutable">) => Borrowed<Node, first.L0, "mutable">
/// @type.symbol symbol=first.a source="a: &Node" type=Borrowed<Node, first.L0, "mutable">
/// @resolution.name source=Node target=Node
/// @type.symbol symbol=first.b source="b: &Node" type=Borrowed<Node, first.L1, "mutable">
/// @resolution.name source=Node target=Node
/// @resolution.name source=Node target=Node

    return a;
    /// @resolution.name source=a target=first.a

}
"#,
    );
}

#[test]
fn test_join_conditional_return_borrow_provenance() {
    let session = TestSession::single(
        r#"
struct Node { id: int32; }

function choose(a: &Node, b: &Node, flag: boolean): &Node {
    return flag ? a : b;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
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
struct Node { id: int32; }
/// @type.symbol symbol=Node source="struct Node { id: int32; }" type=Node
/// @definition.struct symbol=Node source="struct Node { id: int32; }"
/// @definition.field symbol=Node.id source="id: int32" key=id type=int32
/// @type.symbol symbol=Node.id source="id: int32" type=int32

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
    /// @resolution.name source=flag target=choose.flag
    /// @resolution.name source=a target=choose.a
    /// @resolution.name source=b target=choose.b

}
"#,
    );
}

#[test]
fn test_require_explicit_result_lifetime_in_bodyless_signature() {
    let session = TestSession::single(
        r#"
struct Node { id: int32; }

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
struct Node { id: int32; }
/// @type.symbol symbol=Node source="struct Node { id: int32; }" type=Node
/// @definition.struct symbol=Node source="struct Node { id: int32; }"
/// @definition.field symbol=Node.id source="id: int32" key=id type=int32
/// @type.symbol symbol=Node.id source="id: int32" type=int32

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
/// @diagnostic.error id=bodyless-lifetime-elided message="bodyless signatures must name result lifetimes explicitly"
/// @diagnostic.label line=4 column=18 span="choose" line_source="declare function choose<comptime L0: Lifetime, comptime L1: Lifetime>("
"#,
    );
}

#[test]
fn test_induce_lifetimes_for_stored_borrow_fields() {
    let session = TestSession::single(
        r#"
struct Engine { frame: uint64; }
struct AssetStore { count: uint32; }

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
struct Engine { frame: uint64; }
/// @type.symbol symbol=Engine source="struct Engine { frame: uint64; }" type=Engine
/// @definition.struct symbol=Engine source="struct Engine { frame: uint64; }"
/// @definition.field symbol=Engine.frame source="frame: uint64" key=frame type=uint64
/// @type.symbol symbol=Engine.frame source="frame: uint64" type=uint64

struct AssetStore { count: uint32; }
/// @type.symbol symbol=AssetStore source="struct AssetStore { count: uint32; }" type=AssetStore
/// @definition.struct symbol=AssetStore source="struct AssetStore { count: uint32; }"
/// @definition.field symbol=AssetStore.count source="count: uint32" key=count type=uint32
/// @type.symbol symbol=AssetStore.count source="count: uint32" type=uint32

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

#[test]
fn test_tie_elided_result_lifetime_and_place_to_receiver() {
    let session = TestSession::single(
        r#"
import { todo } from "destack:error";

struct Cell { value: int32; }

extension of Cell {
    peek(&readonly this): &readonly int32 {
        todo("peek")
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { todo } from "destack:error";

struct Cell {
    value: int32;
}

extension of Cell {
    peek(&readonly this): Borrowed<int32, L0, "readonly"> {
        todo("peek" as string | undefined)
    }
}

=== checked ===
import { todo } from "destack:error";

struct Cell { value: int32; }
/// @type.symbol symbol=Cell source="struct Cell { value: int32; }" type=Cell
/// @definition.struct symbol=Cell source="struct Cell { value: int32; }"
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32
/// @type.symbol symbol=Cell.value source="value: int32" type=int32

extension of Cell {
/// @definition.extension symbol=<module>#2 form=local target=Cell
/// @definition.method symbol=peek slot=peek type=<comptime peek.L0: Lifetime>(this: Borrowed<this, peek.L0, "readonly">) => Borrowed<int32, peek.L0, "readonly">
/// @resolution.name source=Cell target=Cell

    peek(&readonly this): &readonly int32 {
    /// @generic.template symbol=peek parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=peek type=<comptime peek.L0: Lifetime>(this: Borrowed<this, peek.L0, "readonly">) => Borrowed<int32, peek.L0, "readonly">
    /// @type.symbol symbol=peek.this source="&readonly this" type=Borrowed<this, peek.L0, "readonly">

        todo("peek")
        /// @resolution.name source=todo target=error.panic.todo
        /// @resolution.call source="todo(\"peek\")" parameters=(string | undefined) arguments=(provided("peek") as string | undefined) return=never kind=symbol target=error.panic.todo

    }
}
"#,
    );
}

#[test]
fn test_preserve_access_generic_on_receiver_borrow() {
    let session = TestSession::single(
        r#"
import { Access, WithAccess } from "destack:memory";

interface Viewing {
    type View;

    view<comptime A: Access = "readonly">(
        this: WithAccess<&this, A>,
    ): WithAccess<&this.View, A>;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Access, WithAccess } from "destack:memory";

interface Viewing {
    type View;

    view<comptime A: Access = "readonly">(this: WithAccess<&this, A>): WithAccess<&this.View, A>;
}

=== checked ===
import { Access, WithAccess } from "destack:memory";

interface Viewing {
/// @type.symbol symbol=Viewing type=Viewing
/// @definition.interface symbol=Viewing
/// @definition.associated.type symbol=Viewing.View source="type View" key=View
/// @definition.method symbol=Viewing.view slot=view type=<comptime A: memory.access.Access = "readonly", comptime Viewing.view.L1: Lifetime>(this: memory.type.WithAccess<Borrowed<this, Viewing.view.L1, "mutable">, A>) => memory.type.WithAccess<Borrowed<this.View, Viewing.view.L1, "mutable">, A>

    type View;

    view<comptime A: Access = "readonly">(
    /// @generic.template symbol=Viewing.view parent=template#0 parameters=(comptime A: memory.access.Access = "readonly", comptime L1: Lifetime)
    /// @type.symbol symbol=Viewing.view type=<comptime A: memory.access.Access = "readonly", comptime Viewing.view.L1: Lifetime>(this: memory.type.WithAccess<Borrowed<this, Viewing.view.L1, "mutable">, A>) => memory.type.WithAccess<Borrowed<this.View, Viewing.view.L1, "mutable">, A> reduced=<comptime A: memory.access.Access = "readonly", comptime Viewing.view.L1: Lifetime>(this: Borrowed<this, Viewing.view.L1, A>) => Borrowed<this.View, Viewing.view.L1, A>
    /// @type.symbol symbol=Viewing.view.A source="comptime A: Access = \"readonly\"" type=A
    /// @resolution.name source=Access target=memory.access.Access

        this: WithAccess<&this, A>,
        /// @type.symbol symbol=Viewing.view.this source="this: WithAccess<&this, A>" type=memory.type.WithAccess<Borrowed<this, Viewing.view.L1, "mutable">, A> reduced=Borrowed<this, Viewing.view.L1, A>
        /// @resolution.name source=WithAccess target=memory.type.WithAccess
        /// @resolution.name source=A target=Viewing.view.A

    ): WithAccess<&this.View, A>;
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=A target=Viewing.view.A

}

/// @generic.instance id="memory.type.WithAccess<Borrowed<this, Viewing.view.L1, \"mutable\">, A>" template=memory.type.WithAccess arguments=(Borrowed<this, Viewing.view.L1, "mutable">, A)
/// @generic.instance id="memory.type.WithAccess<Borrowed<this.View, Viewing.view.L1, \"mutable\">, A>" template=memory.type.WithAccess arguments=(Borrowed<this.View, Viewing.view.L1, "mutable">, A)
"#,
    );
}
