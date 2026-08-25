use crate::tests::{DirRows, TestSession};

/// Materializing closes the instance a call to a generic function creates.
#[test]
fn test_materialize_closes_a_called_function_instance() {
    let session = TestSession::single(
        r#"
function pick<T>(value: T): T {
    return value;
}

const chosen = pick(1.5);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function pick<T>(value: T): T {
    return value;
}

const chosen: float64 = pick<float64>(1.5);

=== dir ===
function pick<T>(value: T): T {
/// @generic.template symbol=pick parameters=(T)
/// @type.symbol symbol=pick type=<T>(T) => T
/// @type.symbol symbol=pick.T source=T type=T
/// @type.symbol symbol=pick.value source="value: T" type=T
/// @resolution.name source=T target=pick.T
/// @resolution.name source=T target=pick.T

    return value;
    /// @resolution.name source=value target=pick.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=pick.value

}

const chosen = pick(1.5);
/// @type.symbol symbol=chosen source=chosen type=float64
/// @resolution.pattern source=chosen kind=binding target=chosen
/// @resolution.name source=pick target=pick
/// @resolution.call source=pick(1.5) parameters=(float64) arguments=(provided(1.5) as float64) return=float64 kind=symbol target=pick instance=pick<float64>
/// @generic.instantiation id=pick<float64> template=pick arguments=(float64)
/// @generic.instance id=pick<float64> template=pick arguments=(float64)
"#,
    );
}

/// Materializing emits one instance per distinct argument list.
#[test]
fn test_materialize_deduplicates_repeated_instance_arguments() {
    let session = TestSession::single(
        r#"
function pick<T>(value: T): T {
    return value;
}

const first = pick(1);
const second = pick(2);
const other = pick("text");
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function pick<T>(value: T): T {
    return value;
}

const first: int64 = pick<int64>(1);
const second: int64 = pick<int64>(2);
const other: "text" = pick<"text">("text");

=== dir ===
function pick<T>(value: T): T {
/// @generic.template symbol=pick parameters=(T)
/// @type.symbol symbol=pick type=<T>(T) => T
/// @type.symbol symbol=pick.T source=T type=T
/// @type.symbol symbol=pick.value source="value: T" type=T
/// @resolution.name source=T target=pick.T
/// @resolution.name source=T target=pick.T

    return value;
    /// @resolution.name source=value target=pick.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=pick.value

}

const first = pick(1);
/// @type.symbol symbol=first source=first type=int64
/// @resolution.pattern source=first kind=binding target=first
/// @resolution.name source=pick target=pick
/// @resolution.call source=pick(1) parameters=(int64) arguments=(provided(1) as int64) return=int64 kind=symbol target=pick instance=pick<int64>
/// @generic.instantiation id=pick<int64> template=pick arguments=(int64)
/// @generic.instance id=pick<int64> template=pick arguments=(int64)

const second = pick(2);
/// @type.symbol symbol=second source=second type=int64
/// @resolution.pattern source=second kind=binding target=second
/// @resolution.name source=pick target=pick
/// @resolution.call source=pick(2) parameters=(int64) arguments=(provided(2) as int64) return=int64 kind=symbol target=pick instance=pick<int64>

const other = pick("text");
/// @type.symbol symbol=other source=other type="text"
/// @resolution.pattern source=other kind=binding target=other
/// @resolution.name source=pick target=pick
/// @resolution.call source="pick(\"text\")" parameters=("text") arguments=(provided("text") as "text") return="text" kind=symbol target=pick instance="pick<\"text\">"
/// @generic.instantiation id="pick<\"text\">" template=pick arguments=("text")
/// @generic.instance id="pick<\"text\">" template=pick arguments=("text")
"#,
    );
}

/// Materializing closes the instances a template body reaches transitively.
#[test]
fn test_materialize_closes_transitive_instances_through_a_template_body() {
    let session = TestSession::single(
        r#"
function inner<T>(value: T): T {
    return value;
}

function outer<T>(value: T): T {
    return inner(value);
}

const chosen = outer(true);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function inner<T>(value: T): T {
    return value;
}

function outer<T>(value: T): T {
    return inner<T>(value);
}

const chosen: true = outer<true>(true);

=== dir ===
function inner<T>(value: T): T {
/// @generic.template symbol=inner parameters=(T#1)
/// @type.symbol symbol=inner type=<T#1>(T#1) => T#1
/// @type.symbol symbol=inner.T source=T type=T#1
/// @type.symbol symbol=inner.value source="value: T" type=T#1
/// @resolution.name source=T target=inner.T
/// @resolution.name source=T target=inner.T

    return value;
    /// @resolution.name source=value target=inner.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=inner.value

}

function outer<T>(value: T): T {
/// @generic.template symbol=outer parameters=(T#2)
/// @type.symbol symbol=outer type=<T#2>(T#2) => T#2
/// @type.symbol symbol=outer.T source=T type=T#2
/// @type.symbol symbol=outer.value source="value: T" type=T#2
/// @resolution.name source=T target=outer.T
/// @resolution.name source=T target=outer.T

    return inner(value);
    /// @resolution.name source=inner target=inner
    /// @resolution.call source=inner(value) parameters=(T#2) arguments=(provided(value) as T#2) return=T#2 kind=symbol target=inner instance=inner<T#2>
    /// @generic.instantiation id=inner<T#2> template=inner arguments=(T#2) owner=outer
    /// @resolution.name source=value target=outer.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=outer.value

}

const chosen = outer(true);
/// @type.symbol symbol=chosen source=chosen type=true
/// @resolution.pattern source=chosen kind=binding target=chosen
/// @resolution.name source=outer target=outer
/// @resolution.call source=outer(true) parameters=(true) arguments=(provided(true) as true) return=true kind=symbol target=outer instance=outer<true>
/// @generic.instantiation id=outer<true> template=outer arguments=(true)
/// @generic.instance id=inner<true> template=inner arguments=(true)
/// @generic.instance id=outer<true> template=outer arguments=(true)
"#,
    );
}

/// Materializing closes instances of templates imported from another module.
#[test]
fn test_materialize_closes_an_imported_template_instance() {
    let session = TestSession::single(
        r#"
function positive(values: int32[]): int32[] {
    return values.map((value) => value + 1);
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked(), r#"
=== annotated ===
function positive(values: int32[]): int32[] {
    return values.map<int32, int32, "local">((value: int32): int32 => value + 1) as int32[];
}

=== dir ===
function positive(values: int32[]): int32[] {
/// @type.symbol symbol=positive type=(int32[]) => int32[]
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=MaybeUninit<int32> template=MaybeUninit arguments=(int32)
/// @generic.instance id=new<MaybeUninit<int32>> template=new arguments=(MaybeUninit<int32>)
/// @type.symbol symbol=positive.values source="values: int32[]" type=int32[]

    return values.map((value) => value + 1);
    /// @resolution.name source=values target=positive.values
    /// @resolution.member source=values.map receiver=int32[] type=<map.U#2, map#2.P1: Place>(this: Managed<int32[], map#2.P1>, Function<(int32, isize), map.U#2>) => Owned<map.U#2[]> kind=symbol target_receiver=int32[] target=map#2
    /// @resolution.call source="values.map((value) => value + 1)" parameters=(Function<(int32, isize), int32>) arguments=(provided((value) => value + 1) as Function<(int32, isize), int32>) return=Owned<int32[]> kind=symbol target=map#2 receiver=int32[] instance="Array<int32>.<extension#3>.map#2<int32, \"local\">"
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=positive.values
    /// @generic.instantiation id="map#2<int32, int32, \"local\">" template=map#2 arguments=(int32, int32, "local")
    /// @generic.instantiation id=map#2<int32> template=map#2 arguments=(int32)
    /// @generic.instance id="map#2<int32, int32, \"local\">" template=map#2 arguments=(int32, int32, "local")
    /// @type.symbol symbol=positive.symbol3 source="(value) => value + 1" type=Function<(int32,), int32>
    /// @type.symbol symbol=positive.symbol3.value source=value type=int32
    /// @resolution.name source=value target=positive.symbol3.value
    /// @resolution.operator source="value + 1" type=int32 operator="+" kind=builtin operands=[value as int32 families=(integer), 1 as int32 families=(integer)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=positive.symbol3.value

}
"#);
}

/// Materializing evaluates a computed type inside a closed instance.
#[test]
fn test_materialize_evaluates_a_computed_template_type() {
    let session = TestSession::single(
        r#"
type Choice<T> = T extends string ? int32 : boolean;

declare function choose<T>(): Choice<T>;

function tag<T>(value: T): Choice<T> {
    return choose<T>();
}

const chosen = tag("name");
"#,
    );

    session.assert_dir("main.ds", DirRows::checked(), r#"
=== annotated ===
type Choice<T> = T extends string ? int32 : boolean;

declare function choose<T>(): Choice<T>;

function tag<T>(value: T): Choice<T> {
    return choose<T>();
}

const chosen: int32 = tag<string>("name");

=== dir ===
type Choice<T> = T extends string ? int32 : boolean;
/// @generic.template symbol=Choice parameters=(T#1)
/// @type.symbol symbol=Choice source="type Choice<T> = T extends string ? int32 : boolean" type=T#1 extends string ? int32 : boolean
/// @definition.type symbol=Choice source="type Choice<T> = T extends string ? int32 : boolean" template=(T#1) value=T#1 extends string ? int32 : boolean
/// @type.symbol symbol=Choice.T source=T type=T#1
/// @resolution.name source=T target=Choice.T

declare function choose<T>(): Choice<T>;
/// @generic.template symbol=choose parameters=(T#2)
/// @type.symbol symbol=choose source="declare function choose<T>(): Choice<T>" type=<T#2>() => Choice<T#2>
/// @type.symbol symbol=choose.T source=T type=T#2
/// @resolution.name source=Choice target=Choice
/// @resolution.name source=T target=choose.T

function tag<T>(value: T): Choice<T> {
/// @generic.template symbol=tag parameters=(T#3)
/// @type.symbol symbol=tag type=<T#3>(T#3) => Choice<T#3>
/// @type.symbol symbol=tag.T source=T type=T#3
/// @type.symbol symbol=tag.value source="value: T" type=T#3
/// @resolution.name source=T target=tag.T
/// @resolution.name source=Choice target=Choice
/// @resolution.name source=T target=tag.T

    return choose<T>();
    /// @resolution.name source=choose target=choose
    /// @resolution.call source=choose<T>() parameters=() return=Choice<T#3> kind=symbol target=choose instance=choose<T#3>
    /// @generic.instantiation id=choose<T#3> template=choose arguments=(T#3) owner=tag
    /// @resolution.name source=T target=tag.T

}

const chosen = tag("name");
/// @type.symbol symbol=chosen source=chosen type=int32
/// @resolution.pattern source=chosen kind=binding target=chosen
/// @resolution.name source=tag target=tag
/// @resolution.call source="tag(\"name\")" parameters=(string) arguments=(provided("name") as string) return=int32 kind=symbol target=tag instance=tag<string>
/// @generic.instantiation id=tag<string> template=tag arguments=(string)
/// @generic.instance id=Choice<string> template=Choice arguments=(string) evaluated=(T#1 extends string ? int32 : boolean => int32)
/// @generic.instance id=choose<string> template=choose arguments=(string)
/// @generic.instance id=tag<string> template=tag arguments=(string) evaluated=(Choice<T#3> => int32)
"#);
}

/// A monomorphic module closes no instances.
#[test]
fn test_materialize_adds_no_rows_to_a_monomorphic_module() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

const origin = Point { x: 0 };
"#,
    );

    // a module without generics closes no instances, so the tail renders bare
    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

const origin: Point = Point { x: 0 };

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

const origin = Point { x: 0 };
/// @type.symbol symbol=origin source=origin type=Point
/// @resolution.pattern source=origin kind=binding target=origin
/// @resolution.name source=Point target=Point
"#,
    );
}

/// Structurally identical calls instantiate their generics identically across bodies.
#[test]
fn test_identical_calls_instantiate_identically_across_bodies() {
    let session = TestSession::single(
        r#"
import * as assert from "destack:assert";

function checkLeft(value: int32): void {
    assert.assertEqual(value, 1);
    assert.assertEqual(value, 2);
}

function checkRight(input: int32): void {
    assert.assertEqual(input, 1);
    assert.assertEqual(input, 2);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import * as assert from "destack:assert";

function checkLeft(value: int32): void {
    assert.assertEqual<int32, int32, "local", "local">(
        value as &'frame readonly int32,
        1 as &'frame readonly int32,
    );
    assert.assertEqual<int32, int32, "local", "local">(
        value as &'frame readonly int32,
        2 as &'frame readonly int32,
    );
}

function checkRight(input: int32): void {
    assert.assertEqual<int32, int32, "local", "local">(
        input as &'frame readonly int32,
        1 as &'frame readonly int32,
    );
    assert.assertEqual<int32, int32, "local", "local">(
        input as &'frame readonly int32,
        2 as &'frame readonly int32,
    );
}

=== dir ===
import * as assert from "destack:assert";

function checkLeft(value: int32): void {
/// @type.symbol symbol=checkLeft type=(int32) => void
/// @type.symbol symbol=checkLeft.value source="value: int32" type=int32

    assert.assertEqual(value, 1);
    /// @resolution.name source=assert.assertEqual target=assertEqual
    /// @resolution.call source="assert.assertEqual(value, 1)" parameters=(&'frame readonly int32, &'frame readonly int32, AssertionMessage | undefined) arguments=(provided(value) as &'frame readonly int32, provided(1) as &'frame readonly int32, omitted as AssertionMessage | undefined) return=void kind=symbol target=assertEqual instance="assertEqual<int32, int32, \"local\", \"local\">"
    /// @generic.instantiation id="assertEqual<int32, int32, \"local\", \"local\">" template=assertEqual arguments=(int32, int32, "local", "local")
    /// @resolution.name source=value target=checkLeft.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=checkLeft.value

    assert.assertEqual(value, 2);
    /// @resolution.name source=assert.assertEqual target=assertEqual
    /// @resolution.call source="assert.assertEqual(value, 2)" parameters=(&'frame readonly int32, &'frame readonly int32, AssertionMessage | undefined) arguments=(provided(value) as &'frame readonly int32, provided(2) as &'frame readonly int32, omitted as AssertionMessage | undefined) return=void kind=symbol target=assertEqual instance="assertEqual<int32, int32, \"local\", \"local\">"
    /// @resolution.name source=value target=checkLeft.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=checkLeft.value

}

function checkRight(input: int32): void {
/// @type.symbol symbol=checkRight type=(int32) => void
/// @type.symbol symbol=checkRight.input source="input: int32" type=int32

    assert.assertEqual(input, 1);
    /// @resolution.name source=assert.assertEqual target=assertEqual
    /// @resolution.call source="assert.assertEqual(input, 1)" parameters=(&'frame readonly int32, &'frame readonly int32, AssertionMessage | undefined) arguments=(provided(input) as &'frame readonly int32, provided(1) as &'frame readonly int32, omitted as AssertionMessage | undefined) return=void kind=symbol target=assertEqual instance="assertEqual<int32, int32, \"local\", \"local\">"
    /// @resolution.name source=input target=checkRight.input
    /// @resolution.place source=input placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=input root=checkRight.input

    assert.assertEqual(input, 2);
    /// @resolution.name source=assert.assertEqual target=assertEqual
    /// @resolution.call source="assert.assertEqual(input, 2)" parameters=(&'frame readonly int32, &'frame readonly int32, AssertionMessage | undefined) arguments=(provided(input) as &'frame readonly int32, provided(2) as &'frame readonly int32, omitted as AssertionMessage | undefined) return=void kind=symbol target=assertEqual instance="assertEqual<int32, int32, \"local\", \"local\">"
    /// @resolution.name source=input target=checkRight.input
    /// @resolution.place source=input placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=input root=checkRight.input

}
"#,
        r#"
"#,
    );
}
