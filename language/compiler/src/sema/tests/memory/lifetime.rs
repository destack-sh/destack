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

    moduleBorrow satisfies Borrowed<Point, "static" & "constant", "readonly">;
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

    moduleBorrow satisfies Borrowed<Point, "static" & "constant", "readonly">;
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
/// @type.symbol symbol=moduleBorrow source=moduleBorrow type=&'static readonly constant Point
/// @resolution.pattern source=moduleBorrow kind=binding target=moduleBorrow
/// @resolution.name source=modulePoint target=modulePoint
/// @resolution.place source=modulePoint placement="constant" lifetime="static" access="readonly"
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

    moduleBorrow satisfies Borrowed<Point, "static" & "constant", "readonly">;
    /// @resolution.name source=moduleBorrow target=moduleBorrow
    /// @resolution.place source=moduleBorrow placement="constant" lifetime="static" access="readonly"
    /// @resolution.access source=moduleBorrow root=moduleBorrow
    /// @resolution.name source=Borrowed target=Borrowed
    /// @resolution.name source=Point target=Point

    frameBorrow satisfies local Borrowed<Point, "frame", "readonly">;
    /// @resolution.name source=frameBorrow target=inspectFrame.frameBorrow
    /// @resolution.place source=frameBorrow placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=frameBorrow root=inspectFrame.frameBorrow
    /// @resolution.name source=Borrowed target=Borrowed
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Node {
    id: int32;
}

function first<'a, 'b>(a: &'a Node, b: &'b Node): Borrowed<Node, 'a & P1 | 'b & P3, "mutable"> {
    return a;
}

=== dir ===
struct Node { id: int32; }
/// @type.symbol symbol=Node source="struct Node { id: int32; }" type=Node
/// @definition.struct symbol=Node source="struct Node { id: int32; }"
/// @definition.field symbol=Node.id source="id: int32" key=id type=int32
/// @type.symbol symbol=Node.id source="id: int32" type=int32

function first(a: &Node, b: &Node): &Node {
/// @generic.template symbol=first parameters=('a, P1: Place, 'b, P3: Place)
/// @type.symbol symbol=first type=<first.'a, first.P1: Place, first.'b, first.P3: Place>(&first.'a Node, &first.'b Node) => Borrowed<Node, first.'a & first.P1 | first.'b & first.P3, "mutable">
/// @type.symbol symbol=first.a source="a: &Node" type=&first.'a Node
/// @resolution.name source=Node target=Node
/// @type.symbol symbol=first.b source="b: &Node" type=&first.'b Node
/// @resolution.name source=Node target=Node
/// @resolution.name source=Node target=Node

    return a;
    /// @resolution.name source=a target=first.a
    /// @resolution.place source=a placement=first.P1 lifetime=first.'a access="mutable"
    /// @resolution.access source=a root=first.a

}
"#,
        r#"

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

    session.assert_dir_and_diagnostics(
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
): Borrowed<Node, 'a & P1 | 'b & P3, "mutable"> {
    return flag ? a : b;
}

=== dir ===
struct Node { id: int32; }
/// @type.symbol symbol=Node source="struct Node { id: int32; }" type=Node
/// @definition.struct symbol=Node source="struct Node { id: int32; }"
/// @definition.field symbol=Node.id source="id: int32" key=id type=int32
/// @type.symbol symbol=Node.id source="id: int32" type=int32

function choose(a: &Node, b: &Node, flag: boolean): &Node {
/// @generic.template symbol=choose parameters=('a, P1: Place, 'b, P3: Place)
/// @type.symbol symbol=choose type=<choose.'a, choose.P1: Place, choose.'b, choose.P3: Place>(&choose.'a Node, &choose.'b Node, boolean) => Borrowed<Node, choose.'a & choose.P1 | choose.'b & choose.P3, "mutable">
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
    /// @resolution.place source=a placement=choose.P1 lifetime=choose.'a access="mutable"
    /// @resolution.access source=a root=choose.a
    /// @resolution.name source=b target=choose.b
    /// @resolution.place source=b placement=choose.P3 lifetime=choose.'b access="mutable"
    /// @resolution.access source=b root=choose.b

}
"#,
        r#"

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

declare function choose<'a, 'b>(
    a: Borrowed<Node, 'a, "mutable">,
    b: Borrowed<Node, 'b, "mutable">,
): &Node;

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
    /// @resolution.name source=Borrowed target=Borrowed
    /// @resolution.name source=Node target=Node
    /// @resolution.name source='a target=choose.'a

    b: Borrowed<Node, 'b, "mutable">,
    /// @type.symbol symbol=choose.b source="b: Borrowed<Node, 'b, \"mutable\">" type=&'b Node
    /// @resolution.name source=Borrowed target=Borrowed
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
    engine: Borrowed<Engine, 'a, "mutable">;
    assets: Borrowed<AssetStore, 'b, "mutable">;
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
/// @definition.method symbol=peek slot=peek type=<peek.'a, peek.P1: Place>(this: &peek.'a readonly Cell) => &peek.'a readonly int32
/// @resolution.name source=Cell target=Cell

    peek(&readonly this): &readonly int32 {
    /// @generic.template symbol=peek parameters=('a, P1: Place)
    /// @type.symbol symbol=peek type=<peek.'a, peek.P1: Place>(this: &peek.'a readonly Cell) => &peek.'a readonly int32
    /// @type.symbol symbol=peek.this source="&readonly this" type=&peek.'a readonly this

        todo("peek")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"peek\")" parameters=(string | undefined) arguments=(provided("peek") as string | undefined) return=never kind=symbol target=todo

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
/// @definition.where symbol=Viewing relation=satisfies left=this right=Viewing
/// @definition.associated.type symbol=Viewing.View source="type View" key=View
/// @definition.method symbol=Viewing.view slot=view type=<const A: Access = "readonly", Viewing.view.'a, Viewing.view.P2: Place>(this: WithAccess<&Viewing.view.'a Viewing, A>) => WithAccess<&Viewing.view.'a Viewing.View, A>

    type View;

    view<const A: Access = "readonly">(
    /// @generic.template symbol=Viewing.view parent=template#0 parameters=(const A: Access = "readonly", 'a, P2: Place)
    /// @type.symbol symbol=Viewing.view type=<const A: Access = "readonly", Viewing.view.'a, Viewing.view.P2: Place>(this: WithAccess<&Viewing.view.'a Viewing, A>) => WithAccess<&Viewing.view.'a Viewing.View, A>
    /// @type.symbol symbol=Viewing.view.A source="const A: Access = \"readonly\"" type=A
    /// @resolution.name source=Access target=Access

        this: WithAccess<&this, A>,
        /// @type.symbol symbol=Viewing.view.this source="this: WithAccess<&this, A>" type=WithAccess<&Viewing.view.'a this, A>
        /// @resolution.name source=WithAccess target=WithAccess
        /// @resolution.name source=A target=Viewing.view.A

    ): WithAccess<&this.View, A>;
    /// @resolution.name source=WithAccess target=WithAccess
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

function log<'a>(options?: Options<'a & P1>): void {}

function warn(count?: int32, cause?: unknown): void {
    log<"local">({ count, error: cause as unknown } as Options<"frame" & "local"> | undefined);
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
/// @generic.template symbol=log parameters=('a, P1: Place)
/// @type.symbol symbol=log source="function log(options?: Options): void {}" type=<log.'a, log.P1: Place>(Options<log.'a & log.P1> | undefined?) => void
/// @type.symbol symbol=log.options source="options?: Options" type=Options<log.'a & log.P1> | undefined
/// @resolution.name source=Options target=Options

function warn(count?: int32, cause?: unknown): void {
/// @type.symbol symbol=warn type=(int32 | undefined?, unknown | undefined?) => void
/// @type.symbol symbol=warn.count source="count?: int32" type=int32 | undefined
/// @type.symbol symbol=warn.cause source="cause?: unknown" type=unknown | undefined

    log({ count, error: cause });
    /// @resolution.name source=log target=log
    /// @resolution.call source="log({ count, error: cause })" parameters=(Options<"frame" & "local"> | undefined) arguments=(provided({ count, error: cause }) as Options<"frame" & "local"> | undefined) return=void kind=symbol target=log instance="log<\"local\">"
    /// @generic.instantiation id="log<\"local\">" template=log arguments=("local")
    /// @generic.instance id="log<\"local\">" template=log arguments=("local")
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
    user: Borrowed<User, 'a, "readonly">;
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
    user: Borrowed<User, 'a, "readonly">;
}

function inspect<'a>(user: &'a readonly User): int32 {
    const view: View<'a & P1> = View<'a & P1> { user };

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
/// @generic.template symbol=inspect parameters=('a, P1: Place)
/// @type.symbol symbol=inspect type=<inspect.'a, inspect.P1: Place>(&inspect.'a readonly User) => int32
/// @type.symbol symbol=inspect.user source="user: &readonly User" type=&inspect.'a readonly User
/// @resolution.name source=User target=User

    const view: View = View { user };
    /// @type.symbol symbol=inspect.view source=view type=View<inspect.'a & inspect.P1>
    /// @resolution.pattern source=view kind=binding target=inspect.view
    /// @resolution.name source=View target=View
    /// @resolution.name source=View target=View
    /// @resolution.name source=user target=inspect.user
    /// @resolution.place source=user placement=inspect.P1 lifetime=inspect.'a access="readonly"
    /// @resolution.access source=user root=inspect.user

    return view.user.id;
    /// @resolution.name source=view target=inspect.view
    /// @resolution.member source=view.user receiver=View<inspect.'a & inspect.P1> type=&inspect.'a readonly User kind=field target_receiver=View<inspect.'a & inspect.P1> key=user target=View.user target_type=&inspect.'a readonly User
    /// @resolution.member source=view.user.id receiver=&inspect.'a readonly User type=int32 kind=field target_receiver=&inspect.'a readonly User key=id target=User.id target_type=int32
    /// @resolution.place source=view placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=view root=inspect.view
    /// @resolution.place source=view.user placement=inspect.P1 lifetime=inspect.'a access="readonly"
    /// @resolution.access source=view.user root=inspect.view keys=[user]
    /// @resolution.place source=view.user.id placement=inspect.P1 lifetime=inspect.'a access="readonly"
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Node {
    id: int32;
}

function first<'a, 'b>(
    a: Borrowed<Node, 'a, "mutable">,
    b: &'b Node,
): Borrowed<Node, 'a, "mutable"> {
    return a;
}

=== dir ===
struct Node { id: int32; }
/// @type.symbol symbol=Node source="struct Node { id: int32; }" type=Node
/// @definition.struct symbol=Node source="struct Node { id: int32; }"
/// @definition.field symbol=Node.id source="id: int32" key=id type=int32
/// @type.symbol symbol=Node.id source="id: int32" type=int32

function first<'a>(a: &'a Node, b: &Node): &'a Node {
/// @generic.template symbol=first parameters=('a, 'b, P2: Place)
/// @type.symbol symbol=first type=<'a, first.'b, first.P2: Place>(&'a Node, &first.'b Node) => &'a Node
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
    /// @resolution.place source=a placement='a lifetime='a access="mutable"
    /// @resolution.access source=a root=first.a

}
"#,
        r#"

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

declare const shared: Borrowed<Node, "static", "mutable">;

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

#[test]
fn test_reject_mixed_space_borrow_arguments_for_one_region_parameter() {
    let session = TestSession::single(
        r#"
struct Label { size: int32; }

declare shared const stored: ^Label;

function longest<'a, P: Place>(
    left: Borrowed<Label, 'a & P, "readonly">,
    right: Borrowed<Label, 'a & P, "readonly">,
): Borrowed<Label, 'a & P, "readonly"> {
    return left.size >= right.size ? left : right;
}

function pick(): void {
    const near: ^Label = Label { size: 2 };
    const widest = longest(&readonly near, &readonly stored);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Label {
    size: int32;
}

declare shared const stored: Label;

function longest<'a, P: Place>(
    left: Borrowed<Label, 'a & P, "readonly">,
    right: Borrowed<Label, 'a & P, "readonly">,
): Borrowed<Label, 'a & P, "readonly"> {
    return left.size >= right.size ? left : right;
}

function pick(): void {
    const near: Label = Label { size: 2 };
    const widest = longest(&readonly near, &readonly stored);
}

=== dir ===
struct Label { size: int32; }
/// @type.symbol symbol=Label source="struct Label { size: int32; }" type=Label
/// @definition.struct symbol=Label source="struct Label { size: int32; }"
/// @definition.field symbol=Label.size source="size: int32" key=size type=int32
/// @type.symbol symbol=Label.size source="size: int32" type=int32

declare shared const stored: ^Label;
/// @type.symbol symbol=stored source=stored type=Label
/// @resolution.pattern source=stored kind=binding target=stored
/// @resolution.name source=Label target=Label

function longest<'a, P: Place>(
/// @generic.template symbol=longest parameters=('a, P: Place)
/// @type.symbol symbol=longest type=<'a, P: Place>(Borrowed<Label, 'a & P, "readonly">, Borrowed<Label, 'a & P, "readonly">) => Borrowed<Label, 'a & P, "readonly">
/// @type.symbol symbol=longest.'a source='a type='a
/// @type.symbol symbol=longest.P source="P: Place" type=P
/// @resolution.name source=Place target=Place

    left: Borrowed<Label, 'a & P, "readonly">,
    /// @type.symbol symbol=longest.left source="left: Borrowed<Label, 'a & P, \"readonly\">" type=Borrowed<Label, 'a & P, "readonly">
    /// @resolution.name source=Borrowed target=Borrowed
    /// @resolution.name source=Label target=Label
    /// @resolution.name source='a target=longest.'a
    /// @resolution.name source=P target=longest.P

    right: Borrowed<Label, 'a & P, "readonly">,
    /// @type.symbol symbol=longest.right source="right: Borrowed<Label, 'a & P, \"readonly\">" type=Borrowed<Label, 'a & P, "readonly">
    /// @resolution.name source=Borrowed target=Borrowed
    /// @resolution.name source=Label target=Label
    /// @resolution.name source='a target=longest.'a
    /// @resolution.name source=P target=longest.P

): Borrowed<Label, 'a & P, "readonly"> {
/// @resolution.name source=Borrowed target=Borrowed
/// @resolution.name source=Label target=Label
/// @resolution.name source='a target=longest.'a
/// @resolution.name source=P target=longest.P

    return left.size >= right.size ? left : right;
    /// @resolution.name source=left target=longest.left
    /// @resolution.member source=left.size receiver=Borrowed<Label, 'a & P, "readonly"> type=int32 kind=field target_receiver=Borrowed<Label, 'a & P, "readonly"> key=size target=Label.size target_type=int32
    /// @resolution.operator source="left.size >= right.size" type=boolean operator=">=" kind=builtin operands=[left.size as int32 families=(integer), right.size as int32 families=(integer)]
    /// @resolution.place source=left placement=P lifetime='a access="readonly"
    /// @resolution.access source=left root=longest.left
    /// @resolution.place source=left.size placement=P lifetime='a access="readonly"
    /// @resolution.access source=left.size root=longest.left keys=[size]
    /// @resolution.name source=right target=longest.right
    /// @resolution.member source=right.size receiver=Borrowed<Label, 'a & P, "readonly"> type=int32 kind=field target_receiver=Borrowed<Label, 'a & P, "readonly"> key=size target=Label.size target_type=int32
    /// @resolution.place source=right placement=P lifetime='a access="readonly"
    /// @resolution.access source=right root=longest.right
    /// @resolution.place source=right.size placement=P lifetime='a access="readonly"
    /// @resolution.access source=right.size root=longest.right keys=[size]
    /// @resolution.name source=left target=longest.left
    /// @resolution.place source=left placement=P lifetime='a access="readonly"
    /// @resolution.access source=left root=longest.left
    /// @resolution.name source=right target=longest.right
    /// @resolution.place source=right placement=P lifetime='a access="readonly"
    /// @resolution.access source=right root=longest.right

}

function pick(): void {
/// @type.symbol symbol=pick type=() => void

    const near: ^Label = Label { size: 2 };
    /// @type.symbol symbol=pick.near source=near type=Label
    /// @resolution.pattern source=near kind=binding target=pick.near
    /// @resolution.name source=Label target=Label
    /// @resolution.name source=Label target=Label

    const widest = longest(&readonly near, &readonly stored);
    /// @type.symbol symbol=pick.widest source=widest type=Borrowed<Label, "frame" & <error>, "readonly">
    /// @resolution.pattern source=widest kind=binding target=pick.widest
    /// @resolution.name source=longest target=longest
    /// @resolution.call source="longest(&readonly near, &readonly stored)" parameters=(Borrowed<Label, "frame" & <error>, "readonly">, Borrowed<Label, "frame" & <error>, "readonly">) arguments=(provided(&readonly near) as Borrowed<Label, "frame" & <error>, "readonly">, provided(&readonly stored) as Borrowed<Label, "frame" & <error>, "readonly">) return=Borrowed<Label, "frame" & <error>, "readonly"> kind=symbol target=longest instance=longest<<error>>
    /// @generic.instantiation id=longest<<error>> template=longest arguments=(<error>)
    /// @resolution.name source=near target=pick.near
    /// @resolution.place source=near placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=near root=pick.near
    /// @resolution.name source=stored target=stored
    /// @resolution.place source=stored placement="shared" lifetime="static" access="readonly"
    /// @resolution.access source=stored root=stored

}
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"shared\"' is not assignable to type '\"local\"'"
/// @diagnostic.label line=15 column=44 span="&" line_source="const widest = longest(&readonly near, &readonly stored);"
/// @diagnostic.related line=15 column=20 span="longest(&readonly near, &readonly stored)" line_source="const widest = longest(&readonly near, &readonly stored);" message="in this call"
"#,
    );
}

#[test]
fn test_join_mixed_space_borrows_through_a_written_union() {
    let session = TestSession::single(
        r#"
struct Label { size: int32; }

declare shared const stored: ^Label;
declare const wide: boolean;

function pick(): void {
    const near: ^Label = Label { size: 2 };
    const widest: Borrowed<Label, "local", "readonly"> | Borrowed<Label, "shared", "readonly"> =
        wide ? &readonly stored : &readonly near;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Label {
    size: int32;
}

declare shared const stored: Label;
declare const wide: boolean;

function pick(): void {
    const near: Label = Label { size: 2 };
    const widest: &'frame readonly Label | &'static readonly shared Label = wide
        ? &readonly stored
        : &readonly near;
}

=== dir ===
struct Label { size: int32; }
/// @type.symbol symbol=Label source="struct Label { size: int32; }" type=Label
/// @definition.struct symbol=Label source="struct Label { size: int32; }"
/// @definition.field symbol=Label.size source="size: int32" key=size type=int32
/// @type.symbol symbol=Label.size source="size: int32" type=int32

declare shared const stored: ^Label;
/// @type.symbol symbol=stored source=stored type=Label
/// @resolution.pattern source=stored kind=binding target=stored
/// @resolution.name source=Label target=Label

declare const wide: boolean;
/// @type.symbol symbol=wide source=wide type=boolean
/// @resolution.pattern source=wide kind=binding target=wide

function pick(): void {
/// @type.symbol symbol=pick type=() => void

    const near: ^Label = Label { size: 2 };
    /// @type.symbol symbol=pick.near source=near type=Label
    /// @resolution.pattern source=near kind=binding target=pick.near
    /// @resolution.name source=Label target=Label
    /// @resolution.name source=Label target=Label

    const widest: Borrowed<Label, "local", "readonly"> | Borrowed<Label, "shared", "readonly"> =
    /// @type.symbol symbol=pick.widest source=widest type=&'frame readonly Label | &'static readonly shared Label
    /// @resolution.pattern source=widest kind=binding target=pick.widest
    /// @resolution.name source=Borrowed target=Borrowed
    /// @resolution.name source=Label target=Label
    /// @resolution.name source=Borrowed target=Borrowed
    /// @resolution.name source=Label target=Label

        wide ? &readonly stored : &readonly near;
        /// @resolution.name source=wide target=wide
        /// @resolution.place source=wide placement="constant" lifetime="static" access="readonly"
        /// @resolution.access source=wide root=wide
        /// @resolution.name source=stored target=stored
        /// @resolution.place source=stored placement="shared" lifetime="static" access="readonly"
        /// @resolution.access source=stored root=stored
        /// @resolution.name source=near target=pick.near
        /// @resolution.place source=near placement="local" lifetime="frame" access="readonly"
        /// @resolution.access source=near root=pick.near

}
"#,
    );
}

#[test]
fn test_reject_an_elided_heritage_borrow_region() {
    let session = TestSession::single(
        r#"
struct Buffer { size: int32; }

interface Source<T> {
    read(): T;
}

declare class Reader implements Source<Borrowed<Buffer, "readonly">> {
    read(): Borrowed<Buffer, "readonly">;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Buffer {
    size: int32;
}

interface Source<out T> {
    read(): T;
}

declare class Reader implements Source<Borrowed<Buffer, "readonly">> {
    read(): Borrowed<Buffer, "readonly">;
}

=== dir ===
struct Buffer { size: int32; }
/// @type.symbol symbol=Buffer source="struct Buffer { size: int32; }" type=Buffer
/// @definition.struct symbol=Buffer source="struct Buffer { size: int32; }"
/// @definition.field symbol=Buffer.size source="size: int32" key=size type=int32
/// @type.symbol symbol=Buffer.size source="size: int32" type=int32

interface Source<T> {
/// @generic.template symbol=Source parameters=(out T)
/// @type.symbol symbol=Source type=Source
/// @definition.interface symbol=Source template=(out T)
/// @definition.where symbol=Source relation=satisfies left=this right=Source<T>
/// @definition.method symbol=Source.read source="read(): T" slot=read type=(this: this) => T
/// @type.symbol symbol=Source.T source=T type=T

    read(): T;
    /// @type.symbol symbol=Source.read source="read(): T" type=(this: this) => T
    /// @resolution.name source=T target=Source.T

}

declare class Reader implements Source<Borrowed<Buffer, "readonly">> {
/// @generic.template symbol=Reader parameters=(P0: Place)
/// @type.symbol symbol=Reader type=Reader
/// @definition.class symbol=Reader template=(P0: Place)
/// @definition.where symbol=Reader source="Source<Borrowed<Buffer, \"readonly\">>" relation=satisfies left=this right=Source<Borrowed<Buffer, <error> & Reader.P0, "readonly">>
/// @definition.implements symbol=Reader source="Source<Borrowed<Buffer, \"readonly\">>" target="Source<Borrowed<Buffer, <error> & Reader.P0, \"readonly\">>"
/// @definition.method symbol=Reader.read source="read(): Borrowed<Buffer, \"readonly\">" slot=read type=<Reader.read.P0: Place, Reader.read.'a, Reader.read.P2: Place>(this: &Reader.read.'a readonly Managed<this, Reader.read.P0>) => &Reader.read.'a readonly Buffer
/// @definition.conformance symbol=Reader member=Reader.read requirement=Source.read
/// @resolution.name source=Source target=Source
/// @resolution.name source=Borrowed target=Borrowed
/// @resolution.name source=Buffer target=Buffer

    read(): Borrowed<Buffer, "readonly">;
    /// @generic.template symbol=Reader.read parent=template#1 parameters=(P0: Place, 'a, P2: Place)
    /// @type.symbol symbol=Reader.read source="read(): Borrowed<Buffer, \"readonly\">" type=<Reader.read.P0: Place, Reader.read.'a, Reader.read.P2: Place>(this: &Reader.read.'a readonly Managed<this, Reader.read.P0>) => &Reader.read.'a readonly Buffer
    /// @resolution.name source=Borrowed target=Borrowed
    /// @resolution.name source=Buffer target=Buffer

}
"#,
        r#"
/// @diagnostic.error id=elided-declaration-lifetime message="type declaration 'Reader' writes its lifetimes"
/// @diagnostic.label line=8 column=40 span="Borrowed" line_source="declare class Reader implements Source<Borrowed<Buffer, \"readonly\">> {"
/// @diagnostic.help message="declare the lifetime parameter and name it, like &'a"
"#,
    );
}

#[test]
fn test_elide_the_region_of_an_access_only_borrowed_application() {
    let session = TestSession::single(
        r#"
struct Buffer { size: int32; }

declare function read(borrow: Borrowed<Buffer, "readonly">): int32;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Buffer {
    size: int32;
}

declare function read<'a>(borrow: &'a readonly Buffer): int32;

=== dir ===
struct Buffer { size: int32; }
/// @type.symbol symbol=Buffer source="struct Buffer { size: int32; }" type=Buffer
/// @definition.struct symbol=Buffer source="struct Buffer { size: int32; }"
/// @definition.field symbol=Buffer.size source="size: int32" key=size type=int32
/// @type.symbol symbol=Buffer.size source="size: int32" type=int32

declare function read(borrow: Borrowed<Buffer, "readonly">): int32;
/// @generic.template symbol=read parameters=('a, P1: Place)
/// @type.symbol symbol=read source="declare function read(borrow: Borrowed<Buffer, \"readonly\">): int32" type=<read.'a, read.P1: Place>(&read.'a readonly Buffer) => int32
/// @type.symbol symbol=read.borrow source="borrow: Borrowed<Buffer, \"readonly\">" type=&read.'a readonly Buffer
/// @resolution.name source=Borrowed target=Borrowed
/// @resolution.name source=Buffer target=Buffer
"#,
    );
}

#[test]
fn test_slide_access_arguments_past_const_region_parameters() {
    let session = TestSession::single(
        r#"
import { Region, Access } from "destack:memory";

struct Pair<const R: Region, const A: Access = "mutable"> {
    size: int32;
}

declare function read(pair: Pair<"readonly">): int32;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Access, Region } from "destack:memory";

struct Pair<const R: Region, const A: Access = "mutable"> {
    size: int32;
}

declare function read<'a>(pair: Pair<'a & P1, "readonly">): int32;

=== dir ===
import { Region, Access } from "destack:memory";

struct Pair<const R: Region, const A: Access = "mutable"> {
/// @generic.template symbol=Pair parameters=(const R: Region, const A: Access = "mutable")
/// @type.symbol symbol=Pair type=Pair
/// @definition.struct symbol=Pair template=(const R: Region, const A: Access = "mutable")
/// @definition.field symbol=Pair.size source="size: int32" key=size type=int32
/// @type.symbol symbol=Pair.R source="const R: Region" type=R
/// @resolution.name source=Region target=Region
/// @type.symbol symbol=Pair.A source="const A: Access = \"mutable\"" type=A
/// @resolution.name source=Access target=Access

    size: int32;
    /// @type.symbol symbol=Pair.size source="size: int32" type=int32

}

declare function read(pair: Pair<"readonly">): int32;
/// @generic.template symbol=read parameters=('a, P1: Place)
/// @type.symbol symbol=read source="declare function read(pair: Pair<\"readonly\">): int32" type=<read.'a, read.P1: Place>(Pair<read.'a & read.P1, "readonly">) => int32
/// @type.symbol symbol=read.pair source="pair: Pair<\"readonly\">" type=Pair<read.'a & read.P1, "readonly">
/// @resolution.name source=Pair target=Pair
"#,
    );
}
