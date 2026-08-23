use crate::tests::{DirRows, TestSession};

#[test]
fn test_assign_module_and_function_borrows_static_and_frame_lifetimes() {
    let session = TestSession::single(
        r#"
struct Point { x: int32; }

const modulePoint: ^Point = Point { x: 1 };
const moduleBorrow = &readonly modulePoint;

function inspectFrame(): void {
    const framePoint: ^Point = Point { x: 2 };
    const frameBorrow = &readonly framePoint;

    moduleBorrow satisfies local Borrowed<Point, "static", "readonly">;
    frameBorrow satisfies local Borrowed<Point, "frame", "readonly">;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

const modulePoint: Point = Point { x: 1 };
const moduleBorrow: &'static readonly Point = &readonly modulePoint;

function inspectFrame(): void {
    const framePoint: Point = Point { x: 2 };
    const frameBorrow: &'frame readonly Point = &readonly framePoint;

    moduleBorrow satisfies local Borrowed<Point, "static", "readonly">;
    frameBorrow satisfies local Borrowed<Point, "frame", "readonly">;
}

=== dir ===
struct Point { x: int32; }
/// @type.symbol symbol=Point source="struct Point { x: int32; }" type=Point
/// @definition.struct symbol=Point source="struct Point { x: int32; }"
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @type.symbol symbol=Point.x source="x: int32" type=int32

const modulePoint: ^Point = Point { x: 1 };
/// @type.symbol symbol=modulePoint source=modulePoint type=Point
/// @resolution.pattern source=modulePoint kind=binding target=modulePoint
/// @resolution.name source=Point target=Point
/// @resolution.name source=Point target=Point

const moduleBorrow = &readonly modulePoint;
/// @type.symbol symbol=moduleBorrow source=moduleBorrow type=&'static readonly Point
/// @resolution.pattern source=moduleBorrow kind=binding target=moduleBorrow
/// @resolution.name source=modulePoint target=modulePoint
/// @resolution.place source=modulePoint placement="local" lifetime="static" access="readonly"
/// @resolution.access source=modulePoint root=modulePoint

function inspectFrame(): void {
/// @type.symbol symbol=inspectFrame type=() => void

    const framePoint: ^Point = Point { x: 2 };
    /// @type.symbol symbol=inspectFrame.framePoint source=framePoint type=Point
    /// @resolution.pattern source=framePoint kind=binding target=inspectFrame.framePoint
    /// @resolution.name source=Point target=Point
    /// @resolution.name source=Point target=Point

    const frameBorrow = &readonly framePoint;
    /// @type.symbol symbol=inspectFrame.frameBorrow source=frameBorrow type=&'frame readonly Point
    /// @resolution.pattern source=frameBorrow kind=binding target=inspectFrame.frameBorrow
    /// @resolution.name source=framePoint target=inspectFrame.framePoint
    /// @resolution.place source=framePoint placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=framePoint root=inspectFrame.framePoint

    moduleBorrow satisfies local Borrowed<Point, "static", "readonly">;
    /// @resolution.name source=moduleBorrow target=moduleBorrow
    /// @resolution.place source=moduleBorrow placement="local" lifetime="static" access="readonly"
    /// @resolution.access source=moduleBorrow root=moduleBorrow
    /// @resolution.name source=Borrowed target=memory.borrow.Borrowed
    /// @resolution.name source=Point target=Point

    frameBorrow satisfies local Borrowed<Point, "frame", "readonly">;
    /// @resolution.name source=frameBorrow target=inspectFrame.frameBorrow
    /// @resolution.place source=frameBorrow placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=frameBorrow root=inspectFrame.frameBorrow
    /// @resolution.name source=Borrowed target=memory.borrow.Borrowed
    /// @resolution.name source=Point target=Point

}
"#,
    );
}

#[test]
fn test_elide_ambiguous_return_lifetime_to_input_union() {
    let session = TestSession::single(
        r#"
struct Node { id: int32; }

function first(a: &Node, b: &Node): &Node {
    return a;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Node {
    id: int32;
}

function first<'a, 'b>(a: &'a Node, b: &'b Node): Borrowed<Node, 'a | 'b, "mutable"> {
    return a;
}

=== dir ===
struct Node { id: int32; }
/// @type.symbol symbol=Node source="struct Node { id: int32; }" type=Node
/// @definition.struct symbol=Node source="struct Node { id: int32; }"
/// @definition.field symbol=Node.id source="id: int32" key=id type=int32
/// @type.symbol symbol=Node.id source="id: int32" type=int32

function first(a: &Node, b: &Node): &Node {
/// @generic.template symbol=first parameters=('a, 'b)
/// @type.symbol symbol=first type=<first.'a, first.'b>(&first.'a Node, &first.'b Node) => &first.'a | first.'b Node
/// @type.symbol symbol=first.a source="a: &Node" type=&first.'a Node
/// @resolution.name source=Node target=Node
/// @type.symbol symbol=first.b source="b: &Node" type=&first.'b Node
/// @resolution.name source=Node target=Node
/// @resolution.name source=Node target=Node

    return a;
    /// @resolution.name source=a target=first.a
    /// @resolution.place source=a placement="local" lifetime=first.'a access="mutable"
    /// @resolution.access source=a root=first.a

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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Node {
    id: int32;
}

function choose<'a, 'b>(
    a: &'a Node,
    b: &'b Node,
    flag: boolean,
): Borrowed<Node, 'a | 'b, "mutable"> {
    return flag ? a : b;
}

=== dir ===
struct Node { id: int32; }
/// @type.symbol symbol=Node source="struct Node { id: int32; }" type=Node
/// @definition.struct symbol=Node source="struct Node { id: int32; }"
/// @definition.field symbol=Node.id source="id: int32" key=id type=int32
/// @type.symbol symbol=Node.id source="id: int32" type=int32

function choose(a: &Node, b: &Node, flag: boolean): &Node {
/// @generic.template symbol=choose parameters=('a, 'b)
/// @type.symbol symbol=choose type=<choose.'a, choose.'b>(&choose.'a Node, &choose.'b Node, boolean) => &choose.'a | choose.'b Node
/// @type.symbol symbol=choose.a source="a: &Node" type=&choose.'a Node
/// @resolution.name source=Node target=Node
/// @type.symbol symbol=choose.b source="b: &Node" type=&choose.'b Node
/// @resolution.name source=Node target=Node
/// @type.symbol symbol=choose.flag source="flag: boolean" type=boolean
/// @resolution.name source=Node target=Node

    return flag ? a : b;
    /// @resolution.name source=flag target=choose.flag
    /// @resolution.place source=flag placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=flag root=choose.flag
    /// @resolution.name source=a target=choose.a
    /// @resolution.place source=a placement="local" lifetime=choose.'a access="mutable"
    /// @resolution.access source=a root=choose.a
    /// @resolution.name source=b target=choose.b
    /// @resolution.place source=b placement="local" lifetime=choose.'b access="mutable"
    /// @resolution.access source=b root=choose.b

}
"#,
    );
}

#[test]
fn test_elide_bodyless_result_lifetime_to_input_union() {
    let session = TestSession::single(
        r#"
struct Node { id: int32; }

declare function choose<'a, 'b>(
    a: Borrowed<Node, 'a, "mutable">,
    b: Borrowed<Node, 'b, "mutable">,
): &Node;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Node {
    id: int32;
}

declare function choose<'a, 'b>(a: &'a Node, b: &'b Node): &Node;

=== dir ===
struct Node { id: int32; }
/// @type.symbol symbol=Node source="struct Node { id: int32; }" type=Node
/// @definition.struct symbol=Node source="struct Node { id: int32; }"
/// @definition.field symbol=Node.id source="id: int32" key=id type=int32
/// @type.symbol symbol=Node.id source="id: int32" type=int32

declare function choose<'a, 'b>(
/// @generic.template symbol=choose parameters=('a, 'b)
/// @type.symbol symbol=choose type=<'a, 'b>(&'a Node, &'b Node) => &'a | 'b Node
/// @type.symbol symbol=choose.'a source='a type='a
/// @type.symbol symbol=choose.'b source='b type='b

    a: Borrowed<Node, 'a, "mutable">,
    /// @type.symbol symbol=choose.a source="a: Borrowed<Node, 'a, \"mutable\">" type=&'a Node
    /// @resolution.name source=Borrowed target=memory.borrow.Borrowed
    /// @resolution.name source=Node target=Node
    /// @resolution.name source='a target=choose.'a

    b: Borrowed<Node, 'b, "mutable">,
    /// @type.symbol symbol=choose.b source="b: Borrowed<Node, 'b, \"mutable\">" type=&'b Node
    /// @resolution.name source=Borrowed target=memory.borrow.Borrowed
    /// @resolution.name source=Node target=Node
    /// @resolution.name source='b target=choose.'b

): &Node;
/// @resolution.name source=Node target=Node
"#,
        r#"
"#,
    );
}

#[test]
fn test_write_lifetimes_on_stored_borrow_fields() {
    let session = TestSession::single(
        r#"
struct Engine { frame: uint64; }
struct AssetStore { count: uint32; }

struct WorldView<'a, 'b> {
    engine: &'a Engine;
    assets: &'b AssetStore;
}
"#,
    );

    session.assert_dir(
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

struct WorldView<'a, 'b> {
    engine: &'a Engine;
    assets: &'b AssetStore;
}

=== dir ===
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

struct WorldView<'a, 'b> {
/// @generic.template symbol=WorldView parameters=('a, 'b)
/// @type.symbol symbol=WorldView type=WorldView
/// @definition.struct symbol=WorldView template=('a, 'b)
/// @definition.field symbol=WorldView.assets source="assets: &'b AssetStore" key=assets type=&'b AssetStore
/// @definition.field symbol=WorldView.engine source="engine: &'a Engine" key=engine type=&'a Engine
/// @type.symbol symbol=WorldView.'a source='a type='a
/// @type.symbol symbol=WorldView.'b source='b type='b

    engine: &'a Engine;
    /// @type.symbol symbol=WorldView.engine source="engine: &'a Engine" type=&'a Engine
    /// @resolution.name source='a target=WorldView.'a
    /// @resolution.name source=Engine target=Engine

    assets: &'b AssetStore;
    /// @type.symbol symbol=WorldView.assets source="assets: &'b AssetStore" type=&'b AssetStore
    /// @resolution.name source='b target=WorldView.'b
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { todo } from "destack:error";

struct Cell {
    value: int32;
}

extension of Cell {
    peek(&readonly this): &'a readonly int32 {
        todo("peek" as string | undefined)
    }
}

=== dir ===
import { todo } from "destack:error";

struct Cell { value: int32; }
/// @type.symbol symbol=Cell source="struct Cell { value: int32; }" type=Cell
/// @definition.struct symbol=Cell source="struct Cell { value: int32; }"
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32
/// @type.symbol symbol=Cell.value source="value: int32" type=int32

extension of Cell {
/// @definition.extension symbol=<module>#2 form=local target=Cell
/// @definition.method symbol=peek slot=peek type=<peek.'a>(this: &peek.'a readonly Cell) => &peek.'a readonly int32
/// @resolution.name source=Cell target=Cell

    peek(&readonly this): &readonly int32 {
    /// @generic.template symbol=peek parameters=('a)
    /// @type.symbol symbol=peek type=<peek.'a>(this: &peek.'a readonly Cell) => &peek.'a readonly int32
    /// @type.symbol symbol=peek.this source="&readonly this" type=&peek.'a readonly this

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

    view<const A: Access = "readonly">(
        this: WithAccess<&this, A>,
    ): WithAccess<&this.View, A>;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Access, WithAccess } from "destack:memory";

interface Viewing {
    type View;

    view<const A: Access = "readonly">(this: WithAccess<&this, A>): WithAccess<&this.View, A>;
}

=== dir ===
import { Access, WithAccess } from "destack:memory";

interface Viewing {
/// @type.symbol symbol=Viewing type=Viewing
/// @definition.interface symbol=Viewing
/// @definition.associated.type symbol=Viewing.View source="type View" key=View
/// @definition.method symbol=Viewing.view slot=view type=<const A: memory.access.Access = "readonly", Viewing.view.'a>(this: memory.type.WithAccess<&Viewing.view.'a this, A>) => memory.type.WithAccess<&Viewing.view.'a this.View, A>

    type View;

    view<const A: Access = "readonly">(
    /// @generic.template symbol=Viewing.view parent=template#0 parameters=(const A: memory.access.Access = "readonly", 'a)
    /// @type.symbol symbol=Viewing.view type=<const A: memory.access.Access = "readonly", Viewing.view.'a>(this: memory.type.WithAccess<&Viewing.view.'a this, A>) => memory.type.WithAccess<&Viewing.view.'a this.View, A>
    /// @type.symbol symbol=Viewing.view.A source="const A: Access = \"readonly\"" type=A
    /// @resolution.name source=Access target=memory.access.Access

        this: WithAccess<&this, A>,
        /// @type.symbol symbol=Viewing.view.this source="this: WithAccess<&this, A>" type=memory.type.WithAccess<&Viewing.view.'a this, A>
        /// @resolution.name source=WithAccess target=memory.type.WithAccess
        /// @resolution.name source=A target=Viewing.view.A

    ): WithAccess<&this.View, A>;
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=A target=Viewing.view.A

}
"#,
    );
}

#[test]
fn test_default_unconstrained_call_lifetimes_to_frame() {
    let session = TestSession::single(
        r#"
type Options<'a> = {
    count?: int32 | undefined;
    message?: &'a readonly string;
    error?: unknown;
};

function log(options?: Options): void {}

function warn(count?: int32, cause?: unknown): void {
    log({ count, error: cause });
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Options<'a> = {
    count?: int32 | undefined;
    message?: &'a readonly string;
    error?: unknown;
};

function log<'a>(options?: Options<'a>): void {}

function warn(count?: int32, cause?: unknown): void {
    log({ count, error: cause as unknown } as Options<"frame"> | undefined);
}

=== dir ===
type Options<'a> = {
/// @generic.template symbol=Options parameters=('a)
/// @type.symbol symbol=Options type={ count?: int32 | undefined; message?: &'a readonly string; error?: unknown }
/// @definition.type symbol=Options template=('a) value={ count?: int32 | undefined; message?: &'a readonly string; error?: unknown }
/// @type.symbol symbol=Options.'a source='a type='a

    count?: int32 | undefined;
    message?: &'a readonly string;
    /// @resolution.name source='a target=Options.'a

    error?: unknown;
};

function log(options?: Options): void {}
/// @generic.template symbol=log parameters=('a)
/// @type.symbol symbol=log source="function log(options?: Options): void {}" type=<log.'a>(Options<log.'a> | undefined?) => void
/// @type.symbol symbol=log.options source="options?: Options" type=Options<log.'a> | undefined
/// @resolution.name source=Options target=Options

function warn(count?: int32, cause?: unknown): void {
/// @type.symbol symbol=warn type=(int32 | undefined?, unknown | undefined?) => void
/// @type.symbol symbol=warn.count source="count?: int32" type=int32 | undefined
/// @type.symbol symbol=warn.cause source="cause?: unknown" type=unknown | undefined

    log({ count, error: cause });
    /// @resolution.name source=log target=log
    /// @resolution.call source="log({ count, error: cause })" parameters=(Options<"frame"> | undefined) arguments=(provided({ count, error: cause }) as Options<"frame"> | undefined) return=void kind=symbol target=log
    /// @resolution.name source=count target=warn.count
    /// @resolution.place source=count placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=count root=warn.count
    /// @resolution.name source=cause target=warn.cause
    /// @resolution.place source=cause placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=cause root=warn.cause

}
"#,
    );
}

#[test]
fn test_write_lifetimes_through_forward_nominal_references() {
    let session = TestSession::single(
        r#"
struct Holder<'a> {
    view: View<'a>;
}

struct View<'a> {
    user: &'a readonly User;
}

struct User {
    id: int32;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Holder<'a> {
    view: View<'a>;
}

struct View<'a> {
    user: &'a readonly User;
}

struct User {
    id: int32;
}

=== dir ===
struct Holder<'a> {
/// @generic.template symbol=Holder parameters=('a#1)
/// @type.symbol symbol=Holder type=Holder
/// @definition.struct symbol=Holder template=('a#1)
/// @definition.field symbol=Holder.view source="view: View<'a>" key=view type=View<'a#1>
/// @type.symbol symbol=Holder.'a source='a type='a#1

    view: View<'a>;
    /// @type.symbol symbol=Holder.view source="view: View<'a>" type=View<'a#1>
    /// @resolution.name source=View target=View
    /// @resolution.name source='a target=Holder.'a

}

struct View<'a> {
/// @generic.template symbol=View parameters=('a#2)
/// @type.symbol symbol=View type=View
/// @definition.struct symbol=View template=('a#2)
/// @definition.field symbol=View.user source="user: &'a readonly User" key=user type=&'a#2 readonly User
/// @type.symbol symbol=View.'a source='a type='a#2

    user: &'a readonly User;
    /// @type.symbol symbol=View.user source="user: &'a readonly User" type=&'a#2 readonly User
    /// @resolution.name source='a target=View.'a
    /// @resolution.name source=User target=User

}

struct User {
/// @type.symbol symbol=User type=User
/// @definition.struct symbol=User
/// @definition.field symbol=User.id source="id: int32" key=id type=int32

    id: int32;
    /// @type.symbol symbol=User.id source="id: int32" type=int32

}
"#,
    );
}

#[test]
fn test_reject_elided_lifetimes_in_cyclic_borrowed_fields() {
    let session = TestSession::single(
        r#"
struct Ping {
    pong: &readonly Pong;
}

struct Pong {
    ping: &readonly Ping;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
struct Ping {
    pong: &readonly Pong;
}

struct Pong {
    ping: &readonly Ping;
}

=== dir ===
struct Ping {
    pong: &readonly Pong;
}

struct Pong {
    ping: &readonly Ping;
}
"#,
        r#"
/// @diagnostic.error id=elided-declaration-lifetime message="type declaration 'Ping' writes its lifetimes"
/// @diagnostic.label line=3 column=11 span="&" line_source="pong: &readonly Pong;"
/// @diagnostic.help message="declare the lifetime parameter and name it, like &'a"
/// @diagnostic.error id=elided-declaration-lifetime message="type declaration 'Pong' writes its lifetimes"
/// @diagnostic.label line=7 column=11 span="&" line_source="ping: &readonly Ping;"
/// @diagnostic.help message="declare the lifetime parameter and name it, like &'a"
"#,
    );
}

#[test]
fn test_elide_body_binding_lifetimes_to_frame() {
    let session = TestSession::single(
        r#"
struct User {
    id: int32;
}

struct View<'a> {
    user: &'a readonly User;
}

function inspect(user: &readonly User): int32 {
    const view: View = View { user };

    return view.user.id;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct User {
    id: int32;
}

struct View<'a> {
    user: &'a readonly User;
}

function inspect<'a>(user: &'a readonly User): int32 {
    const view: View<'a> = View<'a> { user };

    return view.user.id;
}

=== dir ===
struct User {
/// @type.symbol symbol=User type=User
/// @definition.struct symbol=User
/// @definition.field symbol=User.id source="id: int32" key=id type=int32

    id: int32;
    /// @type.symbol symbol=User.id source="id: int32" type=int32

}

struct View<'a> {
/// @generic.template symbol=View parameters=('a)
/// @type.symbol symbol=View type=View
/// @definition.struct symbol=View template=('a)
/// @definition.field symbol=View.user source="user: &'a readonly User" key=user type=&'a readonly User
/// @type.symbol symbol=View.'a source='a type='a

    user: &'a readonly User;
    /// @type.symbol symbol=View.user source="user: &'a readonly User" type=&'a readonly User
    /// @resolution.name source='a target=View.'a
    /// @resolution.name source=User target=User

}

function inspect(user: &readonly User): int32 {
/// @generic.template symbol=inspect parameters=('a)
/// @type.symbol symbol=inspect type=<inspect.'a>(&inspect.'a readonly User) => int32
/// @type.symbol symbol=inspect.user source="user: &readonly User" type=&inspect.'a readonly User
/// @resolution.name source=User target=User

    const view: View = View { user };
    /// @type.symbol symbol=inspect.view source=view type=View<inspect.'a>
    /// @resolution.pattern source=view kind=binding target=inspect.view
    /// @resolution.name source=View target=View
    /// @resolution.name source=View target=View
    /// @resolution.name source=user target=inspect.user
    /// @resolution.place source=user placement="local" lifetime=inspect.'a access="readonly"
    /// @resolution.access source=user root=inspect.user

    return view.user.id;
    /// @resolution.name source=view target=inspect.view
    /// @resolution.member source=view.user receiver=View<inspect.'a> type=&inspect.'a readonly User kind=field target_receiver=View<inspect.'a> key=user target=View.user target_type=&inspect.'a readonly User
    /// @resolution.member source=view.user.id receiver=&inspect.'a readonly User type=int32 kind=field target_receiver=&inspect.'a readonly User key=id target=User.id target_type=int32
    /// @resolution.place source=view placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=view root=inspect.view
    /// @resolution.place source=view.user placement="local" lifetime=inspect.'a access="readonly"
    /// @resolution.access source=view.user root=inspect.view keys=[user]
    /// @resolution.place source=view.user.id placement="local" lifetime=inspect.'a access="readonly"
    /// @resolution.access source=view.user.id root=inspect.view keys=[user, id]

}
"#,
    );
}

#[test]
fn test_desugar_tick_parameters_to_const_lifetimes() {
    let session = TestSession::single(
        r#"
struct Node { id: int32; }

function first<'a>(a: &'a Node, b: &Node): &'a Node {
    return a;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Node {
    id: int32;
}

function first<'a, 'b>(a: &'a Node, b: &'b Node): &'a Node {
    return a;
}

=== dir ===
struct Node { id: int32; }
/// @type.symbol symbol=Node source="struct Node { id: int32; }" type=Node
/// @definition.struct symbol=Node source="struct Node { id: int32; }"
/// @definition.field symbol=Node.id source="id: int32" key=id type=int32
/// @type.symbol symbol=Node.id source="id: int32" type=int32

function first<'a>(a: &'a Node, b: &Node): &'a Node {
/// @generic.template symbol=first parameters=('a, 'b)
/// @type.symbol symbol=first type=<'a, first.'b>(&'a Node, &first.'b Node) => &'a Node
/// @type.symbol symbol=first.'a source='a type='a
/// @type.symbol symbol=first.a source="a: &'a Node" type=&'a Node
/// @resolution.name source='a target=first.'a
/// @resolution.name source=Node target=Node
/// @type.symbol symbol=first.b source="b: &Node" type=&first.'b Node
/// @resolution.name source=Node target=Node
/// @resolution.name source='a target=first.'a
/// @resolution.name source=Node target=Node

    return a;
    /// @resolution.name source=a target=first.a
    /// @resolution.place source=a placement="local" lifetime='a access="mutable"
    /// @resolution.access source=a root=first.a

}
"#,
    );
}

#[test]
fn test_walk_static_tick_literal_in_borrow() {
    let session = TestSession::single(
        r#"
struct Node { id: int32; }

declare const shared: &'static Node;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Node {
    id: int32;
}

declare const shared: &'static Node;

=== dir ===
struct Node { id: int32; }
/// @type.symbol symbol=Node source="struct Node { id: int32; }" type=Node
/// @definition.struct symbol=Node source="struct Node { id: int32; }"
/// @definition.field symbol=Node.id source="id: int32" key=id type=int32
/// @type.symbol symbol=Node.id source="id: int32" type=int32

declare const shared: &'static Node;
/// @type.symbol symbol=shared source=shared type=&'static Node
/// @resolution.pattern source=shared kind=binding target=shared
/// @resolution.name source=Node target=Node
"#,
    );
}
