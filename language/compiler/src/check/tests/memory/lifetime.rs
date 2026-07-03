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

#[test]
fn test_elided_result_lifetime_borrows_from_the_receiver() {
    let session = TestSession::single(
        r#"
import { todo } from "destack:error";

struct Cell {
    value: int32;
}

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
        todo("peek")
    }
}

=== checked ===
import { todo } from "destack:error";

struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

extension of Cell {
/// @definition.extension symbol=<module>#2 form=local target=Cell
/// @definition.method symbol=peek slot=peek type=<comptime peek.L0: Lifetime>(this: Borrowed<Cell, peek.L0, "readonly">) => Borrowed<int32, peek.L0, "readonly">
/// @resolution.name source=Cell target=Cell

    peek(&readonly this): &readonly int32 {
    /// @generic.template symbol=peek parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=peek type=<comptime peek.L0: Lifetime>(this: Borrowed<Cell, peek.L0, "readonly">) => Borrowed<int32, peek.L0, "readonly">
    /// @type.symbol symbol=peek.this source="&readonly this" type=Borrowed<this, peek.L0, "readonly">

        todo("peek")
        /// @resolution.name source=todo target=error.panic.todo
        /// @resolution.call source="todo(\"peek\")" parameters=(string) arguments=(provided("peek") as string) return=never kind=symbol target=error.panic.todo

    }
}
"#,
    );
}

#[test]
fn test_bodyless_borrowed_receiver_elides_view_lifetimes() {
    let session = TestSession::single(
        r#"
import { todo } from "destack:error";
import { WithAccess, Access } from "destack:memory";

interface Viewing {
    type View;

    view<comptime A: Access = "readonly">(this: WithAccess<&this, A>): WithAccess<&this.View, A>;
}

struct Buffer {
    value: int32;
}

extension of Buffer implements Viewing {
    type View = int32;

    view<comptime A: Access = "readonly">(this: WithAccess<&Buffer, A>): WithAccess<&int32, A> {
        todo("view")
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
import { Access, WithAccess } from "destack:memory";

interface Viewing {
    type View;

    view<comptime A: Access = "readonly">(this: WithAccess<&this, A>): WithAccess<&this.View, A>;
}

struct Buffer {
    value: int32;
}

extension of Buffer implements Viewing {
    type View = int32;

    view<comptime A: Access = "readonly">(
        this: WithAccess<&Buffer, A>,
    ): WithAccess<Borrowed<int32, L1, "mutable">, A> {
        todo("view")
    }
}

=== checked ===
import { todo } from "destack:error";
import { WithAccess, Access } from "destack:memory";

interface Viewing {
/// @type.symbol symbol=Viewing type=Viewing
/// @definition.interface symbol=Viewing
/// @definition.associated.type symbol=Viewing.View source="type View" key=View
/// @definition.method symbol=Viewing.view slot=view type=<comptime A#1: memory.access.Access = "readonly", comptime Viewing.view.L1: Lifetime>(this: Viewing) => memory.type.WithAccess<Borrowed<this.View, Viewing.view.L1, "mutable">, A#1>

    type View;

    view<comptime A: Access = "readonly">(this: WithAccess<&this, A>): WithAccess<&this.View, A>;
    /// @generic.template symbol=Viewing.view parameters=(comptime A#1: memory.access.Access = "readonly", comptime L1: Lifetime)
    /// @type.symbol symbol=Viewing.view type=<comptime A#1: memory.access.Access = "readonly", comptime Viewing.view.L1: Lifetime>(this: Viewing) => memory.type.WithAccess<Borrowed<this.View, Viewing.view.L1, "mutable">, A#1> reduced=<comptime A#1: memory.access.Access = "readonly", comptime Viewing.view.L1: Lifetime>(this: Viewing) => Borrowed<this.View, Viewing.view.L1, A#1>
    /// @type.symbol symbol=Viewing.view.A source="comptime A: Access = \"readonly\"" type=A#1
    /// @resolution.name source=Access target=memory.access.Access
    /// @type.symbol symbol=Viewing.view.this source="this: WithAccess<&this, A>" type=memory.type.WithAccess<Borrowed<this, Viewing.view.L1, "mutable">, A#1> reduced=Borrowed<this, Viewing.view.L1, A#1>
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=A target=Viewing.view.A
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=A target=Viewing.view.A

}

struct Buffer {
/// @type.symbol symbol=Buffer type=Buffer
/// @definition.struct symbol=Buffer
/// @definition.field symbol=Buffer.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Buffer.value source="value: int32" type=int32

}

extension of Buffer implements Viewing {
/// @definition.extension symbol=<module>#2 form=local target=Buffer
/// @definition.implements symbol=<module>#2 source=Viewing target=Viewing
/// @definition.associated.type symbol=View source="type View = int32" key=View value=int32
/// @definition.method symbol=view slot=view type=<comptime A#2: memory.access.Access = "readonly", comptime view.L1: Lifetime>(this: memory.type.WithAccess<Borrowed<Buffer, view.L1, "mutable">, A#2>) => memory.type.WithAccess<Borrowed<int32, view.L1, "mutable">, A#2>
/// @resolution.name source=Buffer target=Buffer
/// @resolution.name source=Viewing target=Viewing

    type View = int32;
    /// @type.symbol symbol=View source="type View = int32" type=int32

    view<comptime A: Access = "readonly">(this: WithAccess<&Buffer, A>): WithAccess<&int32, A> {
    /// @generic.template symbol=view parameters=(comptime A#2: memory.access.Access = "readonly", comptime L1: Lifetime)
    /// @type.symbol symbol=view type=<comptime A#2: memory.access.Access = "readonly", comptime view.L1: Lifetime>(this: memory.type.WithAccess<Borrowed<Buffer, view.L1, "mutable">, A#2>) => memory.type.WithAccess<Borrowed<int32, view.L1, "mutable">, A#2> reduced=<comptime A#2: memory.access.Access = "readonly", comptime view.L1: Lifetime>(this: Borrowed<Buffer, view.L1, A#2>) => Borrowed<int32, view.L1, A#2>
    /// @type.symbol symbol=view.A source="comptime A: Access = \"readonly\"" type=A#2
    /// @resolution.name source=Access target=memory.access.Access
    /// @type.symbol symbol=view.this source="this: WithAccess<&Buffer, A>" type=memory.type.WithAccess<Borrowed<Buffer, view.L1, "mutable">, A#2> reduced=Borrowed<Buffer, view.L1, A#2>
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=Buffer target=Buffer
    /// @resolution.name source=A target=view.A
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=A target=view.A

        todo("view")
        /// @resolution.name source=todo target=error.panic.todo
        /// @resolution.call source="todo(\"view\")" parameters=(string) arguments=(provided("view") as string) return=never kind=symbol target=error.panic.todo

    }
}

/// @generic.instance id="memory.type.WithAccess<Borrowed<Buffer, view.L1, \"mutable\">, A#2>" template=memory.type.WithAccess arguments=(Borrowed<Buffer, view.L1, "mutable">, A#2)
/// @generic.instance id="memory.type.WithAccess<Borrowed<int32, view.L1, \"mutable\">, A#2>" template=memory.type.WithAccess arguments=(Borrowed<int32, view.L1, "mutable">, A#2)
/// @generic.instance id="memory.type.WithAccess<Borrowed<this, Viewing.view.L1, \"mutable\">, A#1>" template=memory.type.WithAccess arguments=(Borrowed<this, Viewing.view.L1, "mutable">, A#1)
/// @generic.instance id="memory.type.WithAccess<Borrowed<this.View, Viewing.view.L1, \"mutable\">, A#1>" template=memory.type.WithAccess arguments=(Borrowed<this.View, Viewing.view.L1, "mutable">, A#1)
"#,
    );
}
