use crate::tests::{DirRows, TestSession};

#[test]
fn test_callable_value_invokes_function_type() {
    let session = TestSession::single(
        r#"
declare const transform: Function<(int32,), string>;

const text = transform(1);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked()
            .with_node_types()
            .without_reference_types(),
        r#"
=== annotated ===
declare const transform: (arg0: int32) => string;

const text: string = transform(1);

=== dir ===
declare const transform: Function<(int32,), string>;
/// @type.symbol symbol=transform source=transform type=Function<(int32,), string>
/// @resolution.pattern source=transform kind=binding target=transform
/// @resolution.name source=Function target=Function

const text = transform(1);
/// @type.symbol symbol=text source=text type=string
/// @resolution.pattern source=text kind=binding target=text
/// @type.node source=transform(1) type=string
/// @resolution.name source=transform target=transform
/// @resolution.call source=transform(1) parameters=(int32) arguments=(provided(1) as int32) return=string kind=expression target=expression
/// @resolution.place source=transform placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=transform root=transform
/// @type.node source=1 type=1
"#,
    );
}

/// Reject calling a repeatable function through readonly access.
#[test]
fn test_reject_calling_repeatable_function_through_readonly_borrow() {
    let session = TestSession::single(
        r#"
function invokeReadonly(run: &readonly Function<(), void>): void {
    run();
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked()
            .with_node_types()
            .without_reference_types(),
        r#"
=== annotated ===
function invokeReadonly<'a>(run: &'a readonly (() => void)): void {
    run();
}

=== dir ===
function invokeReadonly(run: &readonly Function<(), void>): void {
/// @generic.template symbol=invokeReadonly parameters=('a, P1: Place)
/// @type.symbol symbol=invokeReadonly type=<invokeReadonly.'a, invokeReadonly.P1: Place>(&invokeReadonly.'a readonly Function<(), void>) => void
/// @type.symbol symbol=invokeReadonly.run source="run: &readonly Function<(), void>" type=&invokeReadonly.'a readonly Function<(), void>
/// @resolution.name source=Function target=Function

    run();
    /// @type.node source=run() type=void
    /// @resolution.name source=run target=invokeReadonly.run
    /// @resolution.call source=run() parameters=() return=void kind=expression target=expression receiver=&invokeReadonly.'a readonly Function<(), void>
    /// @resolution.place source=run placement=invokeReadonly.P1 lifetime=invokeReadonly.'a access="readonly"
    /// @resolution.access source=run root=invokeReadonly.run

}
"#,
        r#"
/// @diagnostic.error id=receiver-not-assignable message="receiver type '&'a readonly Function<(), void>' is not assignable to the callable's required access '&Function<(), void>'"
/// @diagnostic.label line=3 column=5 span="run()" line_source="run();"
"#,
    );
}

/// Reject a once function without an owned environment.
#[test]
fn test_reject_managed_once_function_type() {
    let session = TestSession::single(
        r#"
type Invalid = Function<(), void, "once">;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Invalid = Function<(), void, "once">;

=== dir ===
type Invalid = Function<(), void, "once">;
/// @type.symbol symbol=Invalid source="type Invalid = Function<(), void, \"once\">" type=Function<(), void, "once">
/// @definition.type symbol=Invalid source="type Invalid = Function<(), void, \"once\">" value=Function<(), void, "once">
/// @resolution.name source=Function target=Function
"#,
        r#"
/// @diagnostic.error id=once-function-requires-owned message="a once Function requires an owned environment"
/// @diagnostic.label line=2 column=16 span="Function<(), void, \"once\">" line_source="type Invalid = Function<(), void, \"once\">;"
/// @diagnostic.help message="use '^Function<(), void, \"once\">'"
"#,
    );
}

/// Select the first callable member whose where clauses hold.
#[test]
fn test_call_signature_where_clause_selects_a_callable_member() {
    let session = TestSession::single(
        r#"
interface InvocationKind<in out T> {
    (): "copy" where T: Copy;
    (): "affine";
}

declare const copyKind: InvocationKind<int32>;
declare const affineKind: InvocationKind<^Function<(), void, "once">>;

const copy = copyKind();
const affine = affineKind();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked()
            .with_node_types()
            .without_reference_types(),
        r#"
=== annotated ===
interface InvocationKind<in out T> {
    (): "copy" where T: Copy;
    (): "affine";
}

declare const copyKind: InvocationKind<int32>;
declare const affineKind: InvocationKind<^Function<(), void, "once">>;

const copy: "copy" = copyKind();
const affine: "affine" = affineKind();

=== dir ===
interface InvocationKind<in out T> {
/// @generic.template symbol=InvocationKind parameters=(in out T)
/// @type.symbol symbol=InvocationKind type=InvocationKind
/// @definition.interface symbol=InvocationKind template=(in out T)
/// @definition.where symbol=InvocationKind relation=satisfies left=this right=InvocationKind<T>
/// @definition.signature kind=call source="(): \"copy\" where T: Copy" type=Function<(), "copy">
/// @definition.signature kind=call source="(): \"affine\"" type=Function<(), "affine">
/// @type.symbol symbol=InvocationKind.T source=T type=T

    (): "copy" where T: Copy;
    /// @resolution.name source=T target=InvocationKind.T
    /// @resolution.name source=Copy target=Copy

    (): "affine";
}

declare const copyKind: InvocationKind<int32>;
/// @type.symbol symbol=copyKind source=copyKind type=InvocationKind<int32>
/// @resolution.pattern source=copyKind kind=binding target=copyKind
/// @resolution.name source=InvocationKind target=InvocationKind

declare const affineKind: InvocationKind<^Function<(), void, "once">>;
/// @type.symbol symbol=affineKind source=affineKind type=InvocationKind<^Function<(), void, "once">>
/// @resolution.pattern source=affineKind kind=binding target=affineKind
/// @resolution.name source=InvocationKind target=InvocationKind
/// @resolution.name source=Function target=Function

const copy = copyKind();
/// @type.symbol symbol=copy source=copy type="copy"
/// @resolution.pattern source=copy kind=binding target=copy
/// @type.node source=copyKind() type="copy"
/// @resolution.name source=copyKind target=copyKind
/// @resolution.call source=copyKind() parameters=() return="copy" kind=dynamic target="call((): \"copy\")" receiver=InvocationKind<int32> constraint=InvocationKind<int32>
/// @resolution.place source=copyKind placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=copyKind root=copyKind

const affine = affineKind();
/// @type.symbol symbol=affine source=affine type="affine"
/// @resolution.pattern source=affine kind=binding target=affine
/// @type.node source=affineKind() type="affine"
/// @resolution.name source=affineKind target=affineKind
/// @resolution.call source=affineKind() parameters=() return="affine" kind=dynamic target="call((): \"affine\")" receiver=InvocationKind<^Function<(), void, "once">> constraint=InvocationKind<^Function<(), void, "once">>
/// @resolution.place source=affineKind placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=affineKind root=affineKind
"#,
        r#"
"#,
    );
}

#[test]
fn test_callable_union_invokes_every_runtime_arm() {
    let session = TestSession::single(
        r#"
declare const transform:
    Function<(string,), "left"> |
    Function<(string,), "right">;

const result = transform("value");
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked()
            .with_node_types()
            .without_reference_types(),
        r#"
=== annotated ===
declare const transform: Function<(string,), "left"> | Function<(string,), "right">;

const result: "left" | "right" = transform("value");

=== dir ===
declare const transform:
/// @type.symbol symbol=transform source=transform type=Function<(string,), "left"> | Function<(string,), "right">
/// @resolution.pattern source=transform kind=binding target=transform
/// @generic.instance id="Function<(string,), \"left\", \"repeatable\">" template=Function arguments=((string,), "left", "repeatable")
/// @generic.instance id="Function<(string,), \"right\", \"repeatable\">" template=Function arguments=((string,), "right", "repeatable")

    Function<(string,), "left"> |
    /// @resolution.name source=Function target=Function

    Function<(string,), "right">;
    /// @resolution.name source=Function target=Function

const result = transform("value");
/// @type.symbol symbol=result source=result type="left" | "right"
/// @resolution.pattern source=result kind=binding target=result
/// @type.node source="transform(\"value\")" type="left" | "right"
/// @resolution.name source=transform target=transform
/// @resolution.call source="transform(\"value\")" return="left" | "right" kind=union arms=[expression(parameters=(string), arguments=(provided("value") as string), return="left"), expression(parameters=(string), arguments=(provided("value") as string), return="right")]
/// @resolution.place source=transform placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=transform root=transform
/// @type.node source="\"value\"" type="value"
"#,
    );
}

#[test]
fn test_callable_union_ignores_expected_return_in_every_runtime_arm() {
    let session = TestSession::single(
        r#"
declare const transform:
    (Function<(int32,), "common"> & Function<(int32,), "left">) |
    (Function<(int32,), "common"> & Function<(int32,), "right">);

const result: "left" | "right" = transform(1);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked()
            .with_node_types()
            .without_reference_types(),
        r#"
=== annotated ===
declare const transform:
    | Function<(int32,), "common"> & Function<(int32,), "left">
    | Function<(int32,), "common"> & Function<(int32,), "right">;

const result: "left" | "right" = transform(1);

=== dir ===
declare const transform:
/// @type.symbol symbol=transform source=transform type=Function<(int32,), "common"> & Function<(int32,), "left"> | Function<(int32,), "common"> & Function<(int32,), "right">
/// @resolution.pattern source=transform kind=binding target=transform

    (Function<(int32,), "common"> & Function<(int32,), "left">) |
    /// @resolution.name source=Function target=Function
    /// @resolution.name source=Function target=Function

    (Function<(int32,), "common"> & Function<(int32,), "right">);
    /// @resolution.name source=Function target=Function
    /// @resolution.name source=Function target=Function

const result: "left" | "right" = transform(1);
/// @type.symbol symbol=result source=result type="left" | "right"
/// @resolution.pattern source=result kind=binding target=result
/// @type.node source=transform(1) type="common"
/// @resolution.name source=transform target=transform
/// @resolution.call source=transform(1) return="common" kind=union arms=[expression(parameters=(int32), arguments=(provided(1) as int32), return="common"), expression(parameters=(int32), arguments=(provided(1) as int32), return="common")]
/// @resolution.place source=transform placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=transform root=transform
/// @type.node source=1 type=1
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"common\"' is not assignable to type '\"left\" | \"right\"'"
/// @diagnostic.label line=6 column=34 span="transform(1)" line_source="const result: \"left\" | \"right\" = transform(1);"
/// @diagnostic.related line=6 column=22 span="|" line_source="const result: \"left\" | \"right\" = transform(1);" message="expected due to this annotation"
"#,
    );
}

#[test]
fn test_non_callable_call_reports_error() {
    let session = TestSession::single(
        r#"
const value = 1;
value();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked()
            .with_node_types()
            .without_reference_types(),
        r#"
=== annotated ===
const value: 1 = 1;
value();

=== dir ===
const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1

value();
/// @type.node source=value() type=<error>
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=value root=value
/// @resolution.rejected source=value()
"#,
        r#"
/// @diagnostic.error id=not-callable message="value of type '1' is not callable"
/// @diagnostic.label line=3 column=1 span="value()" line_source="value();"
"#,
    );
}

#[test]
fn test_call_an_interface_value_through_its_call_signature() {
    let session = TestSession::single(
        r#"
interface Adder {
    (left: int32, right: int32): int32;
}

declare const add: Adder;
const sum = add(1, 2);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Adder {
    (left: int32, right: int32): int32;
}

declare const add: Adder;
const sum: int32 = add(1, 2);

=== dir ===
interface Adder {
/// @type.symbol symbol=Adder type=Adder
/// @definition.interface symbol=Adder
/// @definition.where symbol=Adder relation=satisfies left=this right=Adder
/// @definition.signature kind=call source="(left: int32, right: int32): int32" type=Function<(int32, int32), int32>

    (left: int32, right: int32): int32;
    /// @type.symbol symbol=Adder.left source="left: int32" type=int32
    /// @type.symbol symbol=Adder.right source="right: int32" type=int32

}

declare const add: Adder;
/// @type.symbol symbol=add source=add type=Adder
/// @resolution.pattern source=add kind=binding target=add
/// @resolution.name source=Adder target=Adder

const sum = add(1, 2);
/// @type.symbol symbol=sum source=sum type=int32
/// @resolution.pattern source=sum kind=binding target=sum
/// @resolution.name source=add target=add
/// @resolution.call source="add(1, 2)" parameters=(int32, int32) arguments=(provided(1) as int32, provided(2) as int32) return=int32 kind=dynamic target="call((left: int32, right: int32): int32)" receiver=Adder constraint=Adder
/// @resolution.place source=add placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=add root=add
"#,
    );
}

#[test]
fn test_construct_through_an_interface_construct_signature() {
    let session = TestSession::single(
        r#"
class Counter {
    value: int32 = 0;
}

interface Factory {
    new (value: int32): Counter;
}

declare const factory: Factory;
const counter = new factory(1);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Counter {
    value: int32 = 0;
}

interface Factory {
    new (value: int32): Counter;
}

declare const factory: Factory;
const counter: Counter = new factory(1);

=== dir ===
class Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32 = 0" key=value type=int32

    value: int32 = 0;
    /// @type.symbol symbol=Counter.value source="value: int32 = 0" type=int32

}

interface Factory {
/// @type.symbol symbol=Factory type=Factory
/// @definition.interface symbol=Factory
/// @definition.where symbol=Factory relation=satisfies left=this right=Factory
/// @definition.signature kind=construct source="new (value: int32): Counter" type=new (int32) => Counter

    new (value: int32): Counter;
    /// @type.symbol symbol=Factory.value source="value: int32" type=int32
    /// @resolution.name source=Counter target=Counter

}

declare const factory: Factory;
/// @type.symbol symbol=factory source=factory type=Factory
/// @resolution.pattern source=factory kind=binding target=factory
/// @resolution.name source=Factory target=Factory

const counter = new factory(1);
/// @type.symbol symbol=counter source=counter type=Counter
/// @resolution.pattern source=counter kind=binding target=counter
/// @resolution.construct source="new factory(1)" parameters=(int32) arguments=(provided(1) as int32) return=Counter kind=dynamic target="construct(new (value: int32): Counter)" receiver=Factory constraint=Factory
/// @resolution.name source=factory target=factory
"#,
    );
}

#[test]
fn test_assign_a_function_to_a_call_signature_interface() {
    let session = TestSession::single(
        r#"
interface Adder {
    (left: int32, right: int32): int32;
}

const add: Adder = (left: int32, right: int32): int32 => left + right;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Adder {
    (left: int32, right: int32): int32;
}

const add: Adder = ((left: int32, right: int32): int32 => left + right) as Adder;

=== dir ===
interface Adder {
/// @type.symbol symbol=Adder type=Adder
/// @definition.interface symbol=Adder
/// @definition.where symbol=Adder relation=satisfies left=this right=Adder
/// @definition.signature kind=call source="(left: int32, right: int32): int32" type=Function<(int32, int32), int32>

    (left: int32, right: int32): int32;
    /// @type.symbol symbol=Adder.left source="left: int32" type=int32
    /// @type.symbol symbol=Adder.right source="right: int32" type=int32

}

const add: Adder = (left: int32, right: int32): int32 => left + right;
/// @type.symbol symbol=add source=add type=Adder
/// @resolution.pattern source=add kind=binding target=add
/// @resolution.name source=Adder target=Adder
/// @type.symbol symbol=symbol5 source="(left: int32, right: int32): int32 => left + right" type=Function<(int32, int32), int32>
/// @type.symbol symbol=symbol5.left source="left: int32" type=int32
/// @type.symbol symbol=symbol5.right source="right: int32" type=int32
/// @resolution.name source=left target=symbol5.left
/// @resolution.operator source="left + right" type=int32 operator="+" kind=builtin operands=[left as int32 families=(integer), right as int32 families=(integer)]
/// @resolution.place source=left placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=left root=symbol5.left
/// @resolution.name source=right target=symbol5.right
/// @resolution.place source=right placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=right root=symbol5.right
"#,
    );
}
