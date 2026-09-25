use crate::tests::{DirRows, TestSession};

/// Recursive automatic dereferencing reports its depth limit at the member access.
#[test]
fn test_recursive_member_dereference() {
    let session = TestSession::single(
        r#"
import { Dereference } from "tspp:ops";

struct Recursive {}

extension of Recursive implements Dereference {
    type Output = Recursive;

    dereference(&readonly this): &readonly Recursive {
        this
    }
}

declare const value: Recursive;

const missing = value.missing;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Dereference } from "tspp:ops";

struct Recursive {}

extension of Recursive implements Dereference {
    type Output = Recursive;

    dereference(&readonly this): &'a readonly Recursive {
        this
    }
}

declare const value: Recursive;

const missing = value.missing;

=== dir ===
import { Dereference } from "tspp:ops";

struct Recursive {}
/// @type.symbol symbol=Recursive source="struct Recursive {}" type=Recursive
/// @definition.struct symbol=Recursive source="struct Recursive {}"

extension of Recursive implements Dereference {
/// @definition.extension symbol=<module>#2 form=local target=Recursive
/// @definition.implements symbol=<module>#2 source=Dereference target=Dereference
/// @definition.associated.type symbol=Output source="type Output = Recursive" key=Output value=Recursive
/// @definition.method symbol=dereference slot=dereference type=<dereference.'a>(this: &dereference.'a readonly Recursive) => &dereference.'a readonly Recursive
/// @resolution.name source=Recursive target=Recursive
/// @resolution.name source=Dereference target=Dereference

    type Output = Recursive;
    /// @type.symbol symbol=Output source="type Output = Recursive" type=Recursive
    /// @resolution.name source=Recursive target=Recursive

    dereference(&readonly this): &readonly Recursive {
    /// @generic.template symbol=dereference parent=template#0 parameters=('a)
    /// @type.symbol symbol=dereference type=<dereference.'a>(this: &dereference.'a readonly Recursive) => &dereference.'a readonly Recursive
    /// @type.symbol symbol=dereference.this source="&readonly this" type=&dereference.'a readonly Recursive
    /// @resolution.name source=Recursive target=Recursive

        this
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&dereference.'a readonly Recursive
        /// @resolution.place source=this placement=dereference.'a lifetime=dereference.'a access="readonly"
        /// @resolution.access source=this root=this

    }
}

declare const value: Recursive;
/// @type.symbol symbol=value source=value type=Recursive
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Recursive target=Recursive

const missing = value.missing;
/// @type.symbol symbol=missing source=missing type=<error>
/// @resolution.pattern source=missing kind=binding target=missing
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
/// @resolution.rejected source=value.missing
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'missing' does not exist on type 'Recursive'"
/// @diagnostic.label line=16 column=23 span="missing" line_source="const missing = value.missing;"
/// @diagnostic.error id=interface-not-implemented message="type 'Recursive' does not implement interface 'Dereference'"
/// @diagnostic.label line=6 column=35 span="Dereference" line_source="extension of Recursive implements Dereference {"
"#,
    );
}

#[test]
fn test_struct_member_access_selects_field_symbol() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

const point = Point { x: 1 };
const x = point.x;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

const point: Point = Point { x: 1 };
const x: int32 = point.x;

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

const point = Point { x: 1 };
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @type.node source="Point { x: 1 }" type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=1 type=1

const x = point.x;
/// @type.symbol symbol=x source=x type=int32
/// @resolution.pattern source=x kind=binding target=x
/// @type.node source=point type=Point
/// @type.node source=point.x type=int32
/// @resolution.name source=point target=point
/// @resolution.member source=point.x receiver=Point type=int32 kind=field target_receiver=Point key=x target=Point.x target_type=int32
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
/// @resolution.access source=point root=point
/// @resolution.access source=point.x root=point keys=[x]
"#,
    );
}

#[test]
fn test_struct_member_call_selects_method_symbol() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;

    length(&readonly this): int32 {
        return this.x;
    }
}

const point = Point { x: 1 };
const length = point.length();
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;

    length(&readonly this): int32 {
        return this.x;
    }
}

const point: Point = Point { x: 1 };
const length: int32 = point.length<"static">();

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.method symbol=Point.length slot=length type=<Point.length.'a>(this: &Point.length.'a readonly Point) => int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    length(&readonly this): int32 {
    /// @generic.template symbol=Point.length parameters=('a)
    /// @type.symbol symbol=Point.length type=<Point.length.'a>(this: &Point.length.'a readonly Point) => int32
    /// @type.symbol symbol=Point.length.this source="&readonly this" type=&Point.length.'a readonly Point

        return this.x;
        /// @type.node source=this type=&Point.length.'a readonly Point
        /// @type.node source=this.x type=int32
        /// @resolution.member source=this.x receiver=&Point.length.'a readonly Point type=int32 kind=field target_receiver=&Point.length.'a readonly Point key=x target=Point.x target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Point type=&Point.length.'a readonly Point
        /// @resolution.place source=this placement=Point.length.'a lifetime=Point.length.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.x placement=Point.length.'a lifetime=Point.length.'a access="readonly"
        /// @resolution.access source=this.x root=this keys=[x]

    }
}

const point = Point { x: 1 };
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @type.node source="Point { x: 1 }" type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=1 type=1

const length = point.length();
/// @type.symbol symbol=length source=length type=int32
/// @resolution.pattern source=length kind=binding target=length
/// @type.node source=point type=Point
/// @type.node source=point.length type=<Point.length.'a>(this: &Point.length.'a readonly Point) => int32
/// @type.node source=point.length() type=int32
/// @resolution.name source=point target=point
/// @resolution.member source=point.length receiver=Point type=<Point.length.'a>(this: &Point.length.'a readonly Point) => int32 kind=symbol target_receiver=Point target=Point.length
/// @resolution.call source=point.length() parameters=() return=int32 regions=("static" & "local") kind=symbol target=Point.length receiver=Point adjustments=(borrow(&'static readonly Point)) instance="Point.length<\"static\" & \"local\">"
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
/// @resolution.access source=point root=point
/// @generic.instantiation id="Point.length<\"static\" & \"local\">" template=Point.length arguments=("static" & "local")
/// @generic.instance id="Point.length<\"bound0\" & \"local\">" template=Point.length arguments=("bound0" & "local")
"#,
    );
}

#[test]
fn test_imported_struct_member_access_selects_exported_field() {
    let compiler = TestSession::builder()
        .module(
            "geometry.tspp",
            r#"
export struct Point {
    x: int32;
}
"#,
        )
        .module(
            "main.tspp",
            r#"
import { Point } from "./geometry.tspp";

const point = Point { x: 1 };
const x = point.x;
"#,
        )
        .build();

    compiler.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Point } from "./geometry.tspp";

const point: Point = Point { x: 1 };
const x: int32 = point.x;

=== dir ===
import { Point } from "./geometry.tspp";

const point = Point { x: 1 };
/// @type.symbol symbol=point source=point type=geometry.Point
/// @resolution.pattern source=point kind=binding target=point
/// @type.node source="Point { x: 1 }" type=geometry.Point
/// @resolution.name source=Point target=geometry.Point
/// @type.node source=1 type=1

const x = point.x;
/// @type.symbol symbol=x source=x type=int32
/// @resolution.pattern source=x kind=binding target=x
/// @type.node source=point type=geometry.Point
/// @type.node source=point.x type=int32
/// @resolution.name source=point target=point
/// @resolution.member source=point.x receiver=geometry.Point type=int32 kind=field target_receiver=geometry.Point key=x target=geometry.Point.x target_type=int32
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
/// @resolution.access source=point root=point
/// @resolution.access source=point.x root=point keys=[x]
"#,
    );
}

#[test]
fn test_array_member_access_selects_length() {
    let session = TestSession::single(
        r#"
let values: int32[] = [];
const length = values.length;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let values: int32[] = [];
const length: isize = values.length;

=== dir ===
let values: int32[] = [];
/// @type.symbol symbol=values source=values type=int32[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @type.node source=[] type=int32[]
/// @resolution.call source=[] parameters=(^Slice<int32>) arguments=(rest() as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
/// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.instance id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)

const length = values.length;
/// @type.symbol symbol=length source=length type=isize
/// @resolution.pattern source=length kind=binding target=length
/// @type.node source=values type=int32[]
/// @type.node source=values.length type=isize
/// @resolution.name source=values target=values
/// @resolution.member source=values.length receiver=int32[] type=isize kind=call target="length(parameters=(), arguments=(), return=isize, regions=(\"managed\" & \"local\"))"
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @generic.instantiation id="length<int32, \"managed\" & \"local\">" template=length arguments=(int32, "managed" & "local")
/// @generic.instance id="length<int32, \"bound0\" & \"local\">" template=length arguments=(int32, "bound0" & "local")
"#,
    );
}

#[test]
fn test_imported_array_member_access_selects_length() {
    let session = TestSession::builder()
        .module(
            "values.tspp",
            r#"
export const values: int32[] = [];
"#,
        )
        .module(
            "main.tspp",
            r#"
import { values } from "./values.tspp";

const length = values.length;
"#,
        )
        .build();

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { values } from "./values.tspp";

const length: isize = values.length;

=== dir ===
import { values } from "./values.tspp";

const length = values.length;
/// @type.symbol symbol=length source=length type=isize
/// @resolution.pattern source=length kind=binding target=length
/// @type.node source=values type=int32[]
/// @type.node source=values.length type=isize
/// @resolution.name source=values target=values.values
/// @resolution.member source=values.length receiver=int32[] type=isize kind=call target="length(parameters=(), arguments=(), return=isize, regions=(\"managed\" & \"local\"))"
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values.values
/// @generic.instantiation id="length<int32, \"managed\" & \"local\">" template=length arguments=(int32, "managed" & "local")
/// @generic.instance id="length<int32, \"bound0\" & \"local\">" template=length arguments=(int32, "bound0" & "local")
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
"#,
    );
}

#[test]
fn test_member_access_uses_later_inferred_binding() {
    let session = TestSession::single(
        r#"
struct State {
    value: int32;
}

function read(): int32 {
    return state.value;
}

const state = State { value: 1 };
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct State {
    value: int32;
}

function read(): int32 {
    return state.value;
}

const state: State = State { value: 1 };

=== dir ===
struct State {
/// @type.symbol symbol=State type=State
/// @definition.struct symbol=State
/// @definition.field symbol=State.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=State.value source="value: int32" type=int32

}

function read(): int32 {
/// @type.symbol symbol=read type=() => int32

    return state.value;
    /// @type.node source=state type=State
    /// @type.node source=state.value type=int32
    /// @resolution.name source=state target=state
    /// @resolution.member source=state.value receiver=State type=int32 kind=field target_receiver=State key=value target=State.value target_type=int32
    /// @resolution.place source=state placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=state root=state
    /// @resolution.place source=state.value placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=state.value root=state keys=[value]

}

const state = State { value: 1 };
/// @type.symbol symbol=state source=state type=State
/// @resolution.pattern source=state kind=binding target=state
/// @type.node source="State { value: 1 }" type=State
/// @resolution.name source=State target=State
/// @type.node source=1 type=1
"#,
    );
}

#[test]
fn test_array_member_call_selects_push_overload() {
    let session = TestSession::single(
        r#"
let values: int32[] = [];
values.push(1);
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let values: int32[] = [];
values.push<int32, "managed">(1);

=== dir ===
let values: int32[] = [];
/// @type.symbol symbol=values source=values type=int32[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @type.node source=[] type=int32[]
/// @resolution.call source=[] parameters=(^Slice<int32>) arguments=(rest() as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
/// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.instance id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)

values.push(1);
/// @type.node source=values type=int32[]
/// @type.node source=values.push type=<push.'a>(this: &push.'a int32[], ...int32[]) => isize
/// @type.node source=values.push(1) type=isize
/// @resolution.name source=values target=values
/// @resolution.member source=values.push receiver=int32[] type=<push.'a>(this: &push.'a int32[], ...int32[]) => isize kind=symbol target_receiver=int32[] target=push
/// @resolution.call source=values.push(1) parameters=(int32[]) arguments=(rest(provided(1) as int32) pack=arrayFromOwnedSlice as int32) return=isize regions=("managed" & "local") kind=symbol target=push receiver=int32[] adjustments=(borrow(&'managed int32[])) instance="Array<int32>.<extension#6>.push<\"managed\" & \"local\">"
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @generic.instantiation id="push<int32, \"managed\" & \"local\">" template=push arguments=(int32, "managed" & "local")
/// @generic.instantiation id=push<int32> template=push arguments=(int32)
/// @generic.instance id="push<int32, \"bound0\" & \"local\">" template=push arguments=(int32, "bound0" & "local")
/// @type.node source=1 type=1
"#,
    );
}

#[test]
fn test_member_on_never_reports_missing_member() {
    let session = TestSession::single(
        r#"
import { todo } from "tspp:error";

function pending(): int32 {
    let value = todo("later");
    return value.field;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { todo } from "tspp:error";

function pending(): int32 {
    let value: never = todo("later" as string | undefined);
    return value.field;
}

=== dir ===
import { todo } from "tspp:error";

function pending(): int32 {
/// @type.symbol symbol=pending type=() => int32

    let value = todo("later");
    /// @type.symbol symbol=pending.value source=value type=never
    /// @resolution.pattern source=value kind=binding target=pending.value
    /// @resolution.name source=todo target=todo
    /// @resolution.call source="todo(\"later\")" parameters=(string | undefined) arguments=(provided("later") as string | undefined) return=never kind=symbol target=todo

    return value.field;
    /// @resolution.name source=value target=pending.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=pending.value
    /// @resolution.rejected source=value.field

}
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'field' does not exist on type 'never'"
/// @diagnostic.label line=6 column=18 span="field" line_source="return value.field;"
"#,
    );
}

#[test]
fn test_function_valued_field_supports_repeated_calls() {
    let session = TestSession::single(
        r#"
struct Handler {
    readonly run: Function<(int32,), int32, "readonly">;
}

const handler = Handler { run: (value) => value };
const first = handler.run(1);
const second = handler.run(2);
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Handler {
    readonly run: (arg0: int32) => int32;
}

const handler: Handler = Handler { run: (value: int32): int32 => value };
const first: int32 = handler.run(1);
const second: int32 = handler.run(2);

=== dir ===
struct Handler {
/// @type.symbol symbol=Handler type=Handler
/// @definition.struct symbol=Handler
/// @definition.field symbol=Handler.run source="readonly run: Function<(int32,), int32, \"readonly\">" key=run type=Function<(int32,), int32, "readonly">

    readonly run: Function<(int32,), int32, "readonly">;
    /// @type.symbol symbol=Handler.run source="readonly run: Function<(int32,), int32, \"readonly\">" type=Function<(int32,), int32, "readonly">
    /// @resolution.name source=Function target=Function

}

const handler = Handler { run: (value) => value };
/// @type.symbol symbol=handler source=handler type=Handler
/// @resolution.pattern source=handler kind=binding target=handler
/// @type.node source="Handler { run: (value) => value }" type=Handler
/// @resolution.name source=Handler target=Handler
/// @type.symbol symbol=symbol4 source="(value) => value" type=Function<(int32,), int32, "readonly">
/// @type.node source="(value) => value" type=Function<(int32,), int32, "readonly">
/// @type.symbol symbol=symbol4.value source=value type=int32
/// @type.node source=value type=int32
/// @resolution.name source=value target=symbol4.value
/// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=value root=symbol4.value

const first = handler.run(1);
/// @type.symbol symbol=first source=first type=int32
/// @resolution.pattern source=first kind=binding target=first
/// @type.node source=handler type=Handler
/// @type.node source=handler.run type=Function<(int32,), int32, "readonly">
/// @type.node source=handler.run(1) type=int32
/// @resolution.name source=handler target=handler
/// @resolution.member source=handler.run receiver=Handler type=Function<(int32,), int32, "readonly"> kind=field target_receiver=Handler key=run target=Handler.run target_type=Function<(int32,), int32, "readonly">
/// @resolution.call source=handler.run(1) parameters=(int32) arguments=(provided(1) as int32) return=int32 kind=expression target=expression
/// @resolution.place source=handler placement="local" lifetime="static" access="immutable"
/// @resolution.access source=handler root=handler
/// @resolution.place source=handler.run placement="local" lifetime="static" access="immutable"
/// @resolution.access source=handler.run root=handler keys=[run]
/// @type.node source=1 type=1

const second = handler.run(2);
/// @type.symbol symbol=second source=second type=int32
/// @resolution.pattern source=second kind=binding target=second
/// @type.node source=handler type=Handler
/// @type.node source=handler.run type=Function<(int32,), int32, "readonly">
/// @type.node source=handler.run(2) type=int32
/// @resolution.name source=handler target=handler
/// @resolution.member source=handler.run receiver=Handler type=Function<(int32,), int32, "readonly"> kind=field target_receiver=Handler key=run target=Handler.run target_type=Function<(int32,), int32, "readonly">
/// @resolution.call source=handler.run(2) parameters=(int32) arguments=(provided(2) as int32) return=int32 kind=expression target=expression
/// @resolution.place source=handler placement="local" lifetime="static" access="immutable"
/// @resolution.access source=handler root=handler
/// @resolution.place source=handler.run placement="local" lifetime="static" access="immutable"
/// @resolution.access source=handler.run root=handler keys=[run]
/// @type.node source=2 type=2
"#,
    );
}

#[test]
fn test_reject_instance_method_read_outside_call_position() {
    let session = TestSession::single(
        r#"
class Logger {
    log(message: string): void {}
}

declare const logger: Logger;

const log = logger.log;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class Logger {
    log(message: string): void {}
}

declare const logger: Logger;

const log = logger.log;

=== dir ===
class Logger {
/// @type.symbol symbol=Logger type=typeof Logger
/// @definition.class symbol=Logger
/// @definition.method symbol=Logger.log source="log(message: string): void {}" slot=log type=(this: Logger, string) => void

    log(message: string): void {}
    /// @type.symbol symbol=Logger.log source="log(message: string): void {}" type=(this: Logger, string) => void
    /// @type.symbol symbol=Logger.log.this type=Logger
    /// @type.symbol symbol=Logger.log.message source="message: string" type=string

}

declare const logger: Logger;
/// @type.symbol symbol=logger source=logger type=Logger
/// @resolution.pattern source=logger kind=binding target=logger
/// @resolution.name source=Logger target=Logger

const log = logger.log;
/// @type.symbol symbol=log source=log type=<error>
/// @resolution.pattern source=log kind=binding target=log
/// @resolution.name source=logger target=logger
/// @resolution.place source=logger placement="local" lifetime="static" access="immutable"
/// @resolution.access source=logger root=logger
/// @resolution.rejected source=logger.log
"#,
        r#"
/// @diagnostic.error id=cannot-extract-bound-method message="method 'log' cannot be read as a value"
/// @diagnostic.label line=8 column=20 span="log" line_source="const log = logger.log;"
/// @diagnostic.help message="wrap the read in a closure to make its receiver capture explicit"
"#,
    );
}

#[test]
fn test_instance_method_call_through_explicit_application_selects_method() {
    let session = TestSession::single(
        r#"
class Store {
    pick<T>(value: T): T {
        return value;
    }
}

declare const store: Store;

const chosen = store.pick<int32>(3);
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class Store {
    pick<T>(value: T): T {
        return value;
    }
}

declare const store: Store;

const chosen: int32 = store.pick<int32>(3);

=== dir ===
class Store {
/// @type.symbol symbol=Store type=typeof Store
/// @definition.class symbol=Store
/// @definition.method symbol=Store.pick slot=pick type=<T>(this: Store, T) => T

    pick<T>(value: T): T {
    /// @generic.template symbol=Store.pick parameters=(T)
    /// @type.symbol symbol=Store.pick type=<T>(this: Store, T) => T
    /// @type.symbol symbol=Store.pick.this type=Store
    /// @type.symbol symbol=Store.pick.T source=T type=T
    /// @type.symbol symbol=Store.pick.value source="value: T" type=T
    /// @resolution.name source=T target=Store.pick.T
    /// @resolution.name source=T target=Store.pick.T

        return value;
        /// @resolution.name source=value target=Store.pick.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Store.pick.value

    }
}

declare const store: Store;
/// @type.symbol symbol=store source=store type=Store
/// @resolution.pattern source=store kind=binding target=store
/// @resolution.name source=Store target=Store

const chosen = store.pick<int32>(3);
/// @type.symbol symbol=chosen source=chosen type=int32
/// @resolution.pattern source=chosen kind=binding target=chosen
/// @resolution.name source=store target=store
/// @resolution.member source=store.pick receiver=Store type=<T>(this: Store, T) => T kind=symbol target_receiver=Store target=Store.pick
/// @resolution.call source=store.pick<int32>(3) parameters=(int32) arguments=(provided(3) as int32) return=int32 kind=symbol target=Store.pick receiver=Store instance=Store.pick<int32>
/// @resolution.place source=store placement="local" lifetime="static" access="immutable"
/// @resolution.access source=store root=store
/// @generic.instantiation id=Store.pick<int32> template=Store.pick arguments=(int32)
/// @generic.instance id=Store.pick<int32> template=Store.pick arguments=(int32)
"#,
    );
}

/// A member observation settles a widened integer receiver at its family form.
#[test]
fn test_default_an_integer_binding_to_int64_at_a_method_call() {
    let session = TestSession::single(
        r#"
function render(): void {
    let x = 5;
    x.toFixed(1);
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_node_types().without_reference_types(),
        r#"
=== annotated ===
function render(): void {
    let x: int64 = 5;
    x.toFixed<int64>(1 as float64 | undefined);
}

=== dir ===
function render(): void {
/// @type.symbol symbol=render type=() => void

    let x = 5;
    /// @type.symbol symbol=render.x source=x type=int64
    /// @resolution.pattern source=x kind=binding target=render.x
    /// @type.node source=5 type=5

    x.toFixed(1);
    /// @type.node source=x.toFixed type=(this: int64, float64 | undefined?) => ^string
    /// @type.node source=x.toFixed(1) type=^string
    /// @resolution.name source=x target=render.x
    /// @resolution.member source=x.toFixed receiver=int64 type=(this: int64, float64 | undefined?) => ^string kind=symbol target_receiver=int64 target=toFixed
    /// @resolution.call source=x.toFixed(1) parameters=(float64 | undefined) arguments=(provided(1) as float64 | undefined) return=^string kind=symbol target=toFixed receiver=int64 instance=int64.<extension#1>.toFixed
    /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=x root=render.x
    /// @generic.instantiation id=toFixed<int64> template=toFixed arguments=(int64)
    /// @generic.instance id=toFixed<int64> template=toFixed arguments=(int64)
    /// @type.node source=1 type=1

}
"#,
    );
}

/// A member observation settles a widened float receiver at its family form.
#[test]
fn test_default_a_float_binding_to_float64_at_a_method_call() {
    let session = TestSession::single(
        r#"
function render(): void {
    let x = 1.5;
    x.toFixed(1);
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_node_types().without_reference_types(),
        r#"
=== annotated ===
function render(): void {
    let x: float64 = 1.5;
    x.toFixed<float64>(1 as float64 | undefined);
}

=== dir ===
function render(): void {
/// @type.symbol symbol=render type=() => void

    let x = 1.5;
    /// @type.symbol symbol=render.x source=x type=float64
    /// @resolution.pattern source=x kind=binding target=render.x
    /// @type.node source=1.5 type=1.5

    x.toFixed(1);
    /// @type.node source=x.toFixed type=(this: float64, float64 | undefined?) => ^string
    /// @type.node source=x.toFixed(1) type=^string
    /// @resolution.name source=x target=render.x
    /// @resolution.member source=x.toFixed receiver=float64 type=(this: float64, float64 | undefined?) => ^string kind=symbol target_receiver=float64 target=toFixed
    /// @resolution.call source=x.toFixed(1) parameters=(float64 | undefined) arguments=(provided(1) as float64 | undefined) return=^string kind=symbol target=toFixed receiver=float64 instance=float64.<extension#1>.toFixed
    /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=x root=render.x
    /// @generic.instantiation id=toFixed<float64> template=toFixed arguments=(float64)
    /// @generic.instance id=toFixed<float64> template=toFixed arguments=(float64)
    /// @type.node source=1 type=1

}
"#,
    );
}

/// A widened integer stays open for wider uses without a member observation.
#[test]
fn test_infer_an_integer_binding_from_a_wider_parameter() {
    let session = TestSession::single(
        r#"
function wide(value: int64): int64 {
    return value;
}

function feed(): int64 {
    let x = 5;
    return wide(x);
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_node_types().without_reference_types(),
        r#"
=== annotated ===
function wide(value: int64): int64 {
    return value;
}

function feed(): int64 {
    let x: int64 = 5;
    return wide(x);
}

=== dir ===
function wide(value: int64): int64 {
/// @type.symbol symbol=wide type=(int64) => int64
/// @type.symbol symbol=wide.value source="value: int64" type=int64

    return value;
    /// @resolution.name source=value target=wide.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=wide.value

}

function feed(): int64 {
/// @type.symbol symbol=feed type=() => int64

    let x = 5;
    /// @type.symbol symbol=feed.x source=x type=int64
    /// @resolution.pattern source=x kind=binding target=feed.x
    /// @type.node source=5 type=5

    return wide(x);
    /// @type.node source=wide(x) type=int64
    /// @resolution.name source=wide target=wide
    /// @resolution.call source=wide(x) parameters=(int64) arguments=(provided(x) as int64) return=int64 kind=symbol target=wide
    /// @resolution.name source=x target=feed.x
    /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=x root=feed.x

}
"#,
    );
}

/// A misspelled member on a widened receiver reports with the closest key.
#[test]
fn test_suggest_the_closest_member_on_an_integer_binding() {
    let session = TestSession::single(
        r#"
function render(): void {
    let x = 5;
    x.toFixd(1);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function render(): void {
    let x: int64 = 5;
    x.toFixd(1);
}

=== dir ===
function render(): void {
/// @type.symbol symbol=render type=() => void

    let x = 5;
    /// @type.symbol symbol=render.x source=x type=int64
    /// @resolution.pattern source=x kind=binding target=render.x

    x.toFixd(1);
    /// @resolution.name source=x target=render.x
    /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=x root=render.x
    /// @resolution.rejected source=x.toFixd
    /// @resolution.rejected source=x.toFixd(1)

}
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'toFixd' does not exist on type 'int64'; did you mean 'toFixed'?"
/// @diagnostic.label line=4 column=7 span="toFixd" line_source="x.toFixd(1);"
/// @diagnostic.suggestion message="rename to 'toFixed'" applicability=dangerous patched="x.toFixed(1);"
"#,
    );
}
