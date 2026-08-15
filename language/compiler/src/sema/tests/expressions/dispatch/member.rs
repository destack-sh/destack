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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

const point: Point = Point { x: 1 };
const x: int32 = point.x;

=== checked ===
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
/// @resolution.place source=point placement="local" lifetime="static" access="exclusive"
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

    length(): int32 {
        return this.x;
    }
}

const point = Point { x: 1 };
const length = point.length();
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;

    length(): int32 {
        return this.x;
    }
}

const point: Point = Point { x: 1 };
const length: int32 = point.length();

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.method symbol=Point.length slot=length type=<Point.length.'a>(this: &Point.length.'a exclusive this) => int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    length(): int32 {
    /// @generic.template symbol=Point.length parameters=('a)
    /// @type.symbol symbol=Point.length type=<Point.length.'a>(this: &Point.length.'a exclusive this) => int32

        return this.x;
        /// @type.node source=this type=&Point.length.'a exclusive Point
        /// @type.node source=this.x type=int32
        /// @resolution.member source=this.x receiver=&Point.length.'a exclusive Point type=int32 kind=field target_receiver=&Point.length.'a exclusive Point key=x target=Point.x target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Point type=&Point.length.'a exclusive Point
        /// @resolution.place source=this placement="local" lifetime=Point.length.'a access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.x placement="local" lifetime=Point.length.'a access="exclusive"
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
/// @type.node source=point.length type=<Point.length.'a>(this: &Point.length.'a exclusive Point) => int32
/// @type.node source=point.length() type=int32
/// @resolution.name source=point target=point
/// @resolution.member source=point.length receiver=Point type=<Point.length.'a>(this: &Point.length.'a exclusive Point) => int32 kind=symbol target_receiver=Point target=Point.length
/// @resolution.call source=point.length() parameters=() return=int32 kind=symbol target=Point.length receiver=Point adjustments=(borrow(&'static exclusive Point))
/// @resolution.place source=point placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=point root=point
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

    compiler.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Point } from "./geometry.ds";

const point: Point = Point { x: 1 };
const x: int32 = point.x;

=== checked ===
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
/// @resolution.place source=point placement="local" lifetime="static" access="exclusive"
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let values: int32[] = [];
const length: isize = values.length;

=== checked ===
let values: int32[] = [];
/// @type.symbol symbol=values source=values type=Array<int32>
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source=[] type=Array<int32>

const length = values.length;
/// @type.symbol symbol=length source=length type=isize
/// @resolution.pattern source=length kind=binding target=length
/// @type.node source=values type=Array<int32>
/// @type.node source=values.length type=isize
/// @resolution.name source=values target=values
/// @resolution.member source=values.length receiver=Array<int32> type=isize kind=call target="collections.array.length#2(parameters=(), arguments=(), return=isize)"
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @generic.instance source=values.length id=Array<int32>.<extension#3>.length#2

/// @generic.instance id=Array<int32>.<extension#3>.length#2 template=collections.array.length#2 arguments=(int32)
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { values } from "./values.ds";

const length: isize = values.length;

=== checked ===
import { values } from "./values.ds";

const length = values.length;
/// @type.symbol symbol=length source=length type=isize
/// @resolution.pattern source=length kind=binding target=length
/// @type.node source=values type=Array<int32>
/// @type.node source=values.length type=isize
/// @resolution.name source=values target=values.values
/// @resolution.member source=values.length receiver=Array<int32> type=isize kind=call target="collections.array.length#2(parameters=(), arguments=(), return=isize)"
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values.values
/// @generic.instance source=values.length id=Array<int32>.<extension#3>.length#2

/// @generic.instance id=Array<int32>.<extension#3>.length#2 template=collections.array.length#2 arguments=(int32)
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

    session.assert_dir_checked(
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

=== checked ===
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
    /// @resolution.place source=state placement="local" lifetime="static" access="exclusive"
    /// @resolution.access source=state root=state
    /// @resolution.place source=state.value placement="local" lifetime="static" access="exclusive"
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let values: int32[] = [];
values.push<int32>(1);

=== checked ===
let values: int32[] = [];
/// @type.symbol symbol=values source=values type=Array<int32>
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source=[] type=Array<int32>

values.push(1);
/// @type.node source=values type=Array<int32>
/// @type.node source=values.push type=<collections.array.push.'a>(this: &collections.array.push.'a exclusive Array<int32>, ...int32[]) => isize
/// @type.node source=values.push(1) type=isize
/// @resolution.name source=values target=values
/// @resolution.member source=values.push receiver=Array<int32> type=<collections.array.push.'a>(this: &collections.array.push.'a exclusive Array<int32>, ...int32[]) => isize kind=symbol target_receiver=Array<int32> target=collections.array.push
/// @resolution.call source=values.push(1) parameters=(Array<int32>) arguments=(rest(1) as int32) return=isize kind=symbol target=collections.array.push receiver=Array<int32> adjustments=(borrow(&'static exclusive Array<int32>)) instance=Array<int32>.<extension#5>.push
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @generic.instance source=values.push(1) id=Array<int32>.<extension#5>.push
/// @type.node source=1 type=1

/// @generic.instance id=Array<int32>.<extension#5>.push template=collections.array.push arguments=(int32)
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { todo } from "destack:error";

function pending(): int32 {
    let value: never = todo("later" as string | undefined);
    return value.field;
}

=== checked ===
import { todo } from "destack:error";

function pending(): int32 {
/// @type.symbol symbol=pending type=() => int32

    let value = todo("later");
    /// @type.symbol symbol=pending.value source=value type=never
    /// @resolution.pattern source=value kind=binding target=pending.value
    /// @resolution.name source=todo target=error.panic.todo
    /// @resolution.call source="todo(\"later\")" parameters=(string | undefined) arguments=(provided("later") as string | undefined) return=never kind=symbol target=error.panic.todo

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

    session.assert_dir_checked(
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

=== checked ===
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
/// @resolution.place source=handler placement="local" lifetime="static" access="exclusive"
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
/// @resolution.place source=handler placement="local" lifetime="static" access="exclusive"
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

const point: Point = Point { x: 1 };
const x: int32 = point.x;

=== checked ===
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
/// @resolution.place source=point placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=point root=point
/// @resolution.access source=point.x root=point keys=[x]
"#,
    );
}
