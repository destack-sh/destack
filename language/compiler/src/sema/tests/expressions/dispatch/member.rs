use crate::tests::{DirRows, TestSession};

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
        "main.ds",
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
/// @resolution.place source=point placement="constant" lifetime="static" access="readonly"
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
        "main.ds",
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
const length: int32 = point.length<"constant">();

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.method symbol=Point.length slot=length type=<Point.length.'a, Point.length.P1: Place>(this: &Point.length.'a readonly Point) => int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    length(&readonly this): int32 {
    /// @generic.template symbol=Point.length parameters=('a, P1: Place)
    /// @type.symbol symbol=Point.length type=<Point.length.'a, Point.length.P1: Place>(this: &Point.length.'a readonly Point) => int32
    /// @type.symbol symbol=Point.length.this source="&readonly this" type=&Point.length.'a readonly this

        return this.x;
        /// @type.node source=this type=&Point.length.'a readonly Point
        /// @type.node source=this.x type=int32
        /// @resolution.member source=this.x receiver=&Point.length.'a readonly Point type=int32 kind=field target_receiver=&Point.length.'a readonly Point key=x target=Point.x target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Point type=&Point.length.'a readonly Point
        /// @resolution.place source=this placement=Point.length.P1 lifetime=Point.length.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.x placement=Point.length.P1 lifetime=Point.length.'a access="readonly"
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
/// @type.node source=point.length type=<Point.length.'a, Point.length.P1: Place>(this: &Point.length.'a readonly Point) => int32
/// @type.node source=point.length() type=int32
/// @resolution.name source=point target=point
/// @resolution.member source=point.length receiver=Point type=<Point.length.'a, Point.length.P1: Place>(this: &Point.length.'a readonly Point) => int32 kind=symbol target_receiver=Point target=Point.length
/// @resolution.call source=point.length() parameters=() return=int32 kind=symbol target=Point.length receiver=Point adjustments=(borrow(&'static readonly constant Point)) instance="Point.length<\"constant\">"
/// @resolution.place source=point placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=point root=point
/// @generic.instantiation id="Point.length<\"constant\">" template=Point.length arguments=("constant")
/// @generic.instance id="Point.length<\"constant\">" template=Point.length arguments=("constant")
"#,
    );
}

#[test]
fn test_imported_struct_member_access_selects_exported_field() {
    let compiler = TestSession::builder()
        .module(
            "geometry.ds",
            r#"
export struct Point {
    x: int32;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Point } from "./geometry.ds";

const point = Point { x: 1 };
const x = point.x;
"#,
        )
        .build();

    compiler.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Point } from "./geometry.ds";

const point: Point = Point { x: 1 };
const x: int32 = point.x;

=== dir ===
import { Point } from "./geometry.ds";

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
/// @resolution.place source=point placement="constant" lifetime="static" access="readonly"
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
        "main.ds",
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
/// @generic.instance id=MaybeUninit<int32> template=MaybeUninit arguments=(int32)
/// @generic.instance id=new<MaybeUninit<int32>> template=new arguments=(MaybeUninit<int32>)
/// @type.node source=[] type=int32[]
/// @resolution.call source=[] parameters=(&arrayFromSlice.'a readonly Slice<arrayFromSlice.T>) arguments=(rest() as int32) return=int32[] kind=symbol target=arrayFromSlice instance=arrayFromSlice<int32>
/// @generic.instantiation id=arrayFromSlice<int32> template=arrayFromSlice arguments=(int32)
/// @generic.instance id="arrayFromSlice<int32, \"local\">" template=arrayFromSlice arguments=(int32, "local")

const length = values.length;
/// @type.symbol symbol=length source=length type=isize
/// @resolution.pattern source=length kind=binding target=length
/// @type.node source=values type=int32[]
/// @type.node source=values.length type=isize
/// @resolution.name source=values target=values
/// @resolution.member source=values.length receiver=int32[] type=isize kind=call target="length(parameters=(), arguments=(), return=isize)"
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @generic.instantiation id="length<int32, \"local\">" template=length arguments=(int32, "local")
/// @generic.instance id="length<int32, \"local\">" template=length arguments=(int32, "local")
"#,
    );
}

#[test]
fn test_imported_array_member_access_selects_length() {
    let session = TestSession::builder()
        .module(
            "values.ds",
            r#"
export const values: int32[] = [];
"#,
        )
        .module(
            "main.ds",
            r#"
import { values } from "./values.ds";

const length = values.length;
"#,
        )
        .build();

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { values } from "./values.ds";

const length: isize = values.length;

=== dir ===
import { values } from "./values.ds";

const length = values.length;
/// @type.symbol symbol=length source=length type=isize
/// @resolution.pattern source=length kind=binding target=length
/// @type.node source=values type=int32[]
/// @type.node source=values.length type=isize
/// @resolution.name source=values target=values.values
/// @resolution.member source=values.length receiver=int32[] type=isize kind=call target="length(parameters=(), arguments=(), return=isize)"
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values.values
/// @generic.instantiation id="length<int32, \"local\">" template=length arguments=(int32, "local")
/// @generic.instance id="length<int32, \"local\">" template=length arguments=(int32, "local")
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=MaybeUninit<int32> template=MaybeUninit arguments=(int32)
/// @generic.instance id=new<MaybeUninit<int32>> template=new arguments=(MaybeUninit<int32>)
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
        "main.ds",
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
    /// @resolution.place source=state placement="constant" lifetime="static" access="readonly"
    /// @resolution.access source=state root=state
    /// @resolution.place source=state.value placement="constant" lifetime="static" access="readonly"
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
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let values: int32[] = [];
values.push<int32, "local">(1);

=== dir ===
let values: int32[] = [];
/// @type.symbol symbol=values source=values type=int32[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=MaybeUninit<int32> template=MaybeUninit arguments=(int32)
/// @generic.instance id=new<MaybeUninit<int32>> template=new arguments=(MaybeUninit<int32>)
/// @type.node source=[] type=int32[]
/// @resolution.call source=[] parameters=(&arrayFromSlice.'a readonly Slice<arrayFromSlice.T>) arguments=(rest() as int32) return=int32[] kind=symbol target=arrayFromSlice instance=arrayFromSlice<int32>
/// @generic.instantiation id=arrayFromSlice<int32> template=arrayFromSlice arguments=(int32)
/// @generic.instance id="arrayFromSlice<int32, \"local\">" template=arrayFromSlice arguments=(int32, "local")

values.push(1);
/// @type.node source=values type=int32[]
/// @type.node source=values.push type=<push.'a, push.P1: Place>(this: &push.'a exclusive int32[], ...int32[]) => isize
/// @type.node source=values.push(1) type=isize
/// @resolution.name source=values target=values
/// @resolution.member source=values.push receiver=int32[] type=<push.'a, push.P1: Place>(this: Borrowed<int32[], push.'a & push.P1, "exclusive">, ...int32[]) => isize kind=symbol target_receiver=int32[] target=push
/// @resolution.call source=values.push(1) parameters=(int32[]) arguments=(rest(1) pack=arrayFromSlice as int32) return=isize kind=symbol target=push receiver=int32[] adjustments=(borrow(&'static exclusive int32[])) instance="Array<int32>.<extension#5>.push<\"local\">"
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @generic.instantiation id="push<int32, \"local\">" template=push arguments=(int32, "local")
/// @generic.instantiation id=push<int32> template=push arguments=(int32)
/// @generic.instance id="push<int32, \"local\">" template=push arguments=(int32, "local")
/// @type.node source=1 type=1
"#,
    );
}

#[test]
fn test_member_on_never_reports_missing_member() {
    let session = TestSession::single(
        r#"
import { todo } from "destack:error";

function pending(): int32 {
    let value = todo("later");
    return value.field;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { todo } from "destack:error";

function pending(): int32 {
    let value: never = todo("later" as string | undefined);
    return value.field;
}

=== dir ===
import { todo } from "destack:error";

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
    readonly run: (value: int32) => int32;
}

const handler = Handler { run: (value) => value };
const first = handler.run(1);
const second = handler.run(2);
"#,
    );

    session.assert_dir(
        "main.ds",
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
/// @definition.field symbol=Handler.run source="readonly run: (value: int32) => int32" key=run type=Function<(int32,), int32>

    readonly run: (value: int32) => int32;
    /// @type.symbol symbol=Handler.run source="readonly run: (value: int32) => int32" type=Function<(int32,), int32>
    /// @type.symbol symbol=Handler.value source="value: int32" type=int32

}

const handler = Handler { run: (value) => value };
/// @type.symbol symbol=handler source=handler type=Handler
/// @resolution.pattern source=handler kind=binding target=handler
/// @type.node source="Handler { run: (value) => value }" type=Handler
/// @resolution.name source=Handler target=Handler
/// @type.symbol symbol=symbol5 source="(value) => value" type=Function<(int32,), int32>
/// @type.node source="(value) => value" type=Function<(int32,), int32>
/// @type.symbol symbol=symbol5.value source=value type=int32
/// @type.node source=value type=int32
/// @resolution.name source=value target=symbol5.value
/// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=value root=symbol5.value

const first = handler.run(1);
/// @type.symbol symbol=first source=first type=int32
/// @resolution.pattern source=first kind=binding target=first
/// @type.node source=handler type=Handler
/// @type.node source=handler.run type=Function<(int32,), int32>
/// @type.node source=handler.run(1) type=int32
/// @resolution.name source=handler target=handler
/// @resolution.member source=handler.run receiver=Handler type=Function<(int32,), int32> kind=field target_receiver=Handler key=run target=Handler.run target_type=Function<(int32,), int32>
/// @resolution.call source=handler.run(1) parameters=(int32) arguments=(provided(1) as int32) return=int32 kind=expression target=expression
/// @resolution.place source=handler placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=handler root=handler
/// @resolution.access source=handler.run root=handler keys=[run]
/// @type.node source=1 type=1

const second = handler.run(2);
/// @type.symbol symbol=second source=second type=int32
/// @resolution.pattern source=second kind=binding target=second
/// @type.node source=handler type=Handler
/// @type.node source=handler.run type=Function<(int32,), int32>
/// @type.node source=handler.run(2) type=int32
/// @resolution.name source=handler target=handler
/// @resolution.member source=handler.run receiver=Handler type=Function<(int32,), int32> kind=field target_receiver=Handler key=run target=Handler.run target_type=Function<(int32,), int32>
/// @resolution.call source=handler.run(2) parameters=(int32) arguments=(provided(2) as int32) return=int32 kind=expression target=expression
/// @resolution.place source=handler placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=handler root=handler
/// @resolution.access source=handler.run root=handler keys=[run]
/// @type.node source=2 type=2
"#,
    );
}

#[test]
fn test_struct_member_access_selects_declared_field() {
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
        "main.ds",
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
/// @resolution.place source=point placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=point root=point
/// @resolution.access source=point.x root=point keys=[x]
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
        "main.ds",
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
/// @type.symbol symbol=Logger type=Logger
/// @definition.class symbol=Logger
/// @definition.method symbol=Logger.log source="log(message: string): void {}" slot=log type=<Logger.log.P0: Place>(this: Managed<this, Logger.log.P0>, string) => void

    log(message: string): void {}
    /// @generic.template symbol=Logger.log parameters=(P0: Place)
    /// @type.symbol symbol=Logger.log source="log(message: string): void {}" type=<Logger.log.P0: Place>(this: Managed<this, Logger.log.P0>, string) => void
    /// @type.symbol symbol=Logger.log.this type=Managed<Logger, Logger.log.P0>
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
/// @resolution.place source=logger placement="local" lifetime="static" access="exclusive"
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
        "main.ds",
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
/// @type.symbol symbol=Store type=Store
/// @definition.class symbol=Store
/// @definition.method symbol=Store.pick slot=pick type=<T, Store.pick.P1: Place>(this: Managed<this, Store.pick.P1>, T) => T

    pick<T>(value: T): T {
    /// @generic.template symbol=Store.pick parameters=(T, P1: Place)
    /// @type.symbol symbol=Store.pick type=<T, Store.pick.P1: Place>(this: Managed<this, Store.pick.P1>, T) => T
    /// @type.symbol symbol=Store.pick.this type=Managed<Store, Store.pick.P1>
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
/// @resolution.member source=store.pick receiver=Store type=<T, Store.pick.P1: Place>(this: Managed<Store, Store.pick.P1>, T) => T kind=symbol target_receiver=Store target=Store.pick
/// @resolution.call source=store.pick<int32>(3) parameters=(int32) arguments=(provided(3) as int32) return=int32 kind=symbol target=Store.pick receiver=Store instance="Store.pick<int32, \"local\">"
/// @resolution.place source=store placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=store root=store
/// @generic.instantiation id="Store.pick<int32, \"local\">" template=Store.pick arguments=(int32, "local")
/// @generic.instance id="Store.pick<int32, \"local\">" template=Store.pick arguments=(int32, "local")
"#,
    );
}
