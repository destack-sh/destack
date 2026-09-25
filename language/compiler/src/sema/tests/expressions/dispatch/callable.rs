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
        "main.tspp",
        DirRows::checked()
            .with_node_types()
            .without_reference_types(),
        r#"
=== annotated ===
declare const transform: (arg0: int32) => string;

const text: string = transform(1);

=== dir ===
declare const transform: Function<(int32,), string>;
/// @type.symbol symbol=transform source=transform type=(int32) => string
/// @resolution.pattern source=transform kind=binding target=transform
/// @resolution.name source=Function target=Function

const text = transform(1);
/// @type.symbol symbol=text source=text type=string
/// @resolution.pattern source=text kind=binding target=text
/// @type.node source=transform(1) type=string
/// @resolution.name source=transform target=transform
/// @resolution.call source=transform(1) parameters=(int32) arguments=(provided(1) as int32) return=string kind=expression target=expression
/// @resolution.place source=transform placement="local" lifetime="static" access="immutable"
/// @resolution.access source=transform root=transform
/// @type.node source=1 type=1
"#,
    );
}

/// Call a repeatable function through a readonly borrow of its handle, managed granting mutable.
#[test]
fn test_call_a_repeatable_function_through_a_readonly_borrow() {
    let session = TestSession::single(
        r#"
function invokeReadonly(run: &readonly Function<(), void>): void {
    run();
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
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
/// @generic.template symbol=invokeReadonly parameters=('a)
/// @type.symbol symbol=invokeReadonly type=<invokeReadonly.'a>(&invokeReadonly.'a readonly (() => void)) => void
/// @type.symbol symbol=invokeReadonly.run source="run: &readonly Function<(), void>" type=&invokeReadonly.'a readonly (() => void)
/// @resolution.name source=Function target=Function

    run();
    /// @type.node source=run() type=void
    /// @resolution.name source=run target=invokeReadonly.run
    /// @resolution.call source=run() parameters=() return=void kind=expression target=expression
    /// @resolution.place source=run placement=invokeReadonly.'a lifetime=invokeReadonly.'a access="readonly"
    /// @resolution.access source=run root=invokeReadonly.run

}
"#,
        r#"
/// @diagnostic.error id=receiver-not-assignable message="receiver type '&'a readonly (() => void)' is not assignable to the method's 'this' type '&(() => void)'"
/// @diagnostic.label line=3 column=5 span="run()" line_source="run();"
"#,
    );
}

/// Reject calling an owned-receiver callable through a managed handle or a borrow.
#[test]
fn test_reject_calling_an_owned_callable_through_a_handle() {
    let session = TestSession::single(
        r#"
declare const owned: ^Function<(), void, "once">;
declare const handle: Function<(), void, "once">;
declare const borrowed: &readonly Function<(), void, "once">;

owned();
handle();
borrowed();
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
declare const owned: ^(() => void);
declare const handle: () => void;
declare const borrowed: &'static readonly (() => void);

owned();
handle();
borrowed();

=== dir ===
declare const owned: ^Function<(), void, "once">;
/// @type.symbol symbol=owned source=owned type=^Function<(), void, "once">
/// @resolution.pattern source=owned kind=binding target=owned
/// @resolution.name source=Function target=Function

declare const handle: Function<(), void, "once">;
/// @type.symbol symbol=handle source=handle type=Function<(), void, "once">
/// @resolution.pattern source=handle kind=binding target=handle
/// @resolution.name source=Function target=Function

declare const borrowed: &readonly Function<(), void, "once">;
/// @type.symbol symbol=borrowed source=borrowed type=&'static readonly Function<(), void, "once">
/// @resolution.pattern source=borrowed kind=binding target=borrowed
/// @resolution.name source=Function target=Function

owned();
/// @resolution.name source=owned target=owned
/// @resolution.call source=owned() parameters=() return=void kind=expression target=expression
/// @resolution.place source=owned placement="local" lifetime="static" access="immutable"
/// @resolution.access source=owned root=owned

handle();
/// @resolution.name source=handle target=handle
/// @resolution.call source=handle() parameters=() return=void kind=expression target=expression
/// @resolution.place source=handle placement="local" lifetime="static" access="immutable"
/// @resolution.access source=handle root=handle

borrowed();
/// @resolution.name source=borrowed target=borrowed
/// @resolution.call source=borrowed() parameters=() return=void kind=expression target=expression
/// @resolution.place source=borrowed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=borrowed root=borrowed
"#, r#"
/// @diagnostic.error id=receiver-not-assignable message="receiver type 'Function<(), void, \"once\">' is not assignable to the method's 'this' type '^Function<(), void, \"once\">'"
/// @diagnostic.label line=7 column=1 span="handle()" line_source="handle();"
/// @diagnostic.error id=receiver-not-assignable message="receiver type '&'static readonly Function<(), void, \"once\">' is not assignable to the method's 'this' type '^Function<(), void, \"once\">'"
/// @diagnostic.label line=8 column=1 span="borrowed()" line_source="borrowed();"
"#);
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
        "main.tspp",
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
declare const affineKind: InvocationKind<^(() => void)>;

const copy: "copy" = copyKind();
const affine: "affine" = affineKind();

=== dir ===
interface InvocationKind<in out T> {
/// @generic.template symbol=InvocationKind parameters=(in out T, this: InvocationKind<T>)
/// @type.symbol symbol=InvocationKind type=InvocationKind
/// @definition.interface symbol=InvocationKind template=(in out T, this: InvocationKind<T>)
/// @definition.where symbol=InvocationKind relation=satisfies left=this right=InvocationKind<T>
/// @definition.signature kind=call source="(): \"affine\"" type=() => "affine"
/// @definition.signature kind=call source="(): \"copy\" where T: Copy" type=() => "copy"
/// @type.symbol symbol=InvocationKind.T source="in out T" type=T

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
/// @resolution.call source=copyKind() parameters=() return="copy" kind=dynamic target="call((): \"copy\" where T: Copy)" receiver=InvocationKind<int32> constraint=InvocationKind<int32>
/// @resolution.place source=copyKind placement="local" lifetime="static" access="immutable"
/// @resolution.access source=copyKind root=copyKind

const affine = affineKind();
/// @type.symbol symbol=affine source=affine type="affine"
/// @resolution.pattern source=affine kind=binding target=affine
/// @type.node source=affineKind() type="affine"
/// @resolution.name source=affineKind target=affineKind
/// @resolution.call source=affineKind() parameters=() return="affine" kind=dynamic target="call((): \"affine\")" receiver=InvocationKind<^Function<(), void, "once">> constraint=InvocationKind<^Function<(), void, "once">>
/// @resolution.place source=affineKind placement="local" lifetime="static" access="immutable"
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
    Function<(string,), "left", "readonly"> |
    Function<(string,), "right", "readonly">;

const result = transform("value");
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked()
            .with_node_types()
            .without_reference_types(),
        r#"
=== annotated ===
declare const transform: ((arg0: string) => "left") | ((arg0: string) => "right");

const result: "left" | "right" = transform("value");

=== dir ===
declare const transform:
/// @type.symbol symbol=transform source=transform type=Function<(string,), "left", "readonly"> | Function<(string,), "right", "readonly">
/// @resolution.pattern source=transform kind=binding target=transform

    Function<(string,), "left", "readonly"> |
    /// @resolution.name source=Function target=Function

    Function<(string,), "right", "readonly">;
    /// @resolution.name source=Function target=Function

const result = transform("value");
/// @type.symbol symbol=result source=result type="left" | "right"
/// @resolution.pattern source=result kind=binding target=result
/// @type.node source="transform(\"value\")" type="left" | "right"
/// @resolution.name source=transform target=transform
/// @resolution.call source="transform(\"value\")" return="left" | "right" kind=union arms=[expression(parameters=(string), arguments=(provided("value") as string), return="left"), expression(parameters=(string), arguments=(provided("value") as string), return="right")]
/// @resolution.place source=transform placement="local" lifetime="static" access="immutable"
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
    (Function<(int32,), "common", "readonly"> & Function<(int32,), "left", "readonly">) |
    (Function<(int32,), "common", "readonly"> & Function<(int32,), "right", "readonly">);

const result: "left" | "right" = transform(1);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked()
            .with_node_types()
            .without_reference_types(),
        r#"
=== annotated ===
declare const transform:
    | ((arg0: int32) => "common") & ((arg0: int32) => "left")
    | ((arg0: int32) => "common") & ((arg0: int32) => "right");

const result: "left" | "right" = transform(1);

=== dir ===
declare const transform:
/// @type.symbol symbol=transform source=transform type=Function<(int32,), "common", "readonly"> & Function<(int32,), "left", "readonly"> | Function<(int32,), "common", "readonly"> & Function<(int32,), "right", "readonly">
/// @resolution.pattern source=transform kind=binding target=transform

    (Function<(int32,), "common", "readonly"> & Function<(int32,), "left", "readonly">) |
    /// @resolution.name source=Function target=Function
    /// @resolution.name source=Function target=Function

    (Function<(int32,), "common", "readonly"> & Function<(int32,), "right", "readonly">);
    /// @resolution.name source=Function target=Function
    /// @resolution.name source=Function target=Function

const result: "left" | "right" = transform(1);
/// @type.symbol symbol=result source=result type="left" | "right"
/// @resolution.pattern source=result kind=binding target=result
/// @type.node source=transform(1) type="common"
/// @resolution.name source=transform target=transform
/// @resolution.call source=transform(1) parameters=(int32) arguments=(provided(1) as int32) return="common" kind=expression target=expression
/// @resolution.place source=transform placement="local" lifetime="static" access="immutable"
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
        "main.tspp",
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
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
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
        "main.tspp",
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
/// @generic.template symbol=Adder parameters=(this: Adder)
/// @type.symbol symbol=Adder type=Adder
/// @definition.interface symbol=Adder template=(this: Adder)
/// @definition.where symbol=Adder relation=satisfies left=this right=Adder
/// @definition.signature kind=call source="(left: int32, right: int32): int32" type=(int32, int32) => int32

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
/// @resolution.place source=add placement="local" lifetime="static" access="immutable"
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
        "main.tspp",
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
/// @type.symbol symbol=Counter type=typeof Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32 = 0" key=value type=int32

    value: int32 = 0;
    /// @type.symbol symbol=Counter.value source="value: int32 = 0" type=int32

}

interface Factory {
/// @generic.template symbol=Factory parameters=(this: Factory)
/// @type.symbol symbol=Factory type=Factory
/// @definition.interface symbol=Factory template=(this: Factory)
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
/// @resolution.call source="new factory(1)" parameters=(int32) arguments=(provided(1) as int32) return=Counter kind=dynamic target="construct(new (value: int32): Counter)" receiver=Factory constraint=Factory
/// @resolution.name source=factory target=factory
/// @resolution.place source=factory placement="local" lifetime="static" access="immutable"
/// @resolution.access source=factory root=factory
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
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
interface Adder {
    (left: int32, right: int32): int32;
}

const add: Adder = ((left: int32, right: int32): int32 => left + right) as Adder;

=== dir ===
interface Adder {
/// @generic.template symbol=Adder parameters=(this: Adder)
/// @type.symbol symbol=Adder type=Adder
/// @definition.interface symbol=Adder template=(this: Adder)
/// @definition.where symbol=Adder relation=satisfies left=this right=Adder
/// @definition.signature kind=call source="(left: int32, right: int32): int32" type=(int32, int32) => int32

    (left: int32, right: int32): int32;
    /// @type.symbol symbol=Adder.left source="left: int32" type=int32
    /// @type.symbol symbol=Adder.right source="right: int32" type=int32

}

const add: Adder = (left: int32, right: int32): int32 => left + right;
/// @type.symbol symbol=add source=add type=Adder
/// @resolution.pattern source=add kind=binding target=add
/// @resolution.name source=Adder target=Adder
/// @type.symbol symbol=symbol5 source="(left: int32, right: int32): int32 => left + right" type=Function<(int32, int32), int32, "readonly">
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

/// Reject a closure writing a capture where the callback slot takes its receiver readonly.
#[test]
fn test_reject_a_capture_writing_closure_in_a_readonly_callback_slot() {
    let session = TestSession::single(
        r#"
declare function visit(callback: Function<(int32,), void, "readonly">): void;

function total(values: int32[]): int32 {
    let sum = 0;
    visit((value) => {
        sum += value;
    });
    return sum;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
declare function visit(callback: Function<(int32,), void, "readonly">): void;

function total(values: int32[]): int32 {
    let sum: int32 = 0;
    visit((value: int32): void => {
        sum += value;
    });
    return sum;
}

=== dir ===
declare function visit(callback: Function<(int32,), void, "readonly">): void;
/// @type.symbol symbol=visit source="declare function visit(callback: Function<(int32,), void, \"readonly\">): void" type=(Function<(int32,), void, "readonly">) => void
/// @resolution.name source=Function target=Function

function total(values: int32[]): int32 {
/// @type.symbol symbol=total type=(int32[]) => int32
/// @type.symbol symbol=total.values source="values: int32[]" type=int32[]

    let sum = 0;
    /// @type.symbol symbol=total.sum source=sum type=int32
    /// @resolution.pattern source=sum kind=binding target=total.sum

    visit((value) => {
    /// @resolution.name source=visit target=visit
    /// @resolution.call parameters=(Function<(int32,), void, "readonly">) arguments=(provided(argument) as Function<(int32,), void, "readonly">) return=void kind=symbol target=visit
    /// @type.symbol symbol=total.symbol6 type=(int32) => void
    /// @type.symbol symbol=total.symbol6.value source=value type=int32

        sum += value;
        /// @resolution.name source=sum target=total.sum
        /// @resolution.operator source="sum += value" type=int32 operator="+" kind=builtin operands=[sum as int32 families=(integer), value as int32 families=(integer)]
        /// @resolution.pattern.assign source=sum kind=place
        /// @resolution.place source=sum placement="local" lifetime="frame" access="exclusive"
        /// @resolution.assignment source=sum read=binding(total.sum) write=binding(total.sum) type=int32
        /// @resolution.access source=sum root=total.sum
        /// @resolution.name source=value target=total.symbol6.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=total.symbol6.value

    });
    return sum;
    /// @resolution.name source=sum target=total.sum
    /// @resolution.place source=sum placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=sum root=total.sum

}
"#, r#"
/// @diagnostic.error id=receiver-access-not-granted message="the callable requires 'mutable' access to its receiver, and its slot takes it 'readonly'"
/// @diagnostic.label line=6 column=11 span="(value) => {\n        sum += value;\n    }" line_source="visit((value) => {"
"#);
}

/// Accept a closure writing a capture where the callback slot elides its receiver.
#[test]
fn test_accept_a_capture_writing_closure_in_an_elided_callback_slot() {
    let session = TestSession::single(
        r#"
declare function visit(callback: (value: int32) => void): void;

function total(values: int32[]): int32 {
    let sum = 0;
    visit((value) => {
        sum += value;
    });
    return sum;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
declare function visit(callback: (value: int32) => void): void;

function total(values: int32[]): int32 {
    let sum: int32 = 0;
    visit((value: int32): void => {
        sum += value;
    });
    return sum;
}

=== dir ===
declare function visit(callback: (value: int32) => void): void;
/// @type.symbol symbol=visit source="declare function visit(callback: (value: int32) => void): void" type=((int32) => void) => void
/// @type.symbol symbol=visit.value source="value: int32" type=int32

function total(values: int32[]): int32 {
/// @type.symbol symbol=total type=(int32[]) => int32
/// @type.symbol symbol=total.values source="values: int32[]" type=int32[]

    let sum = 0;
    /// @type.symbol symbol=total.sum source=sum type=int32
    /// @resolution.pattern source=sum kind=binding target=total.sum

    visit((value) => {
    /// @resolution.name source=visit target=visit
    /// @resolution.call parameters=((int32) => void) arguments=(provided(argument) as (int32) => void) return=void kind=symbol target=visit
    /// @type.symbol symbol=total.symbol7 type=(int32) => void
    /// @type.symbol symbol=total.symbol7.value source=value type=int32

        sum += value;
        /// @resolution.name source=sum target=total.sum
        /// @resolution.operator source="sum += value" type=int32 operator="+" kind=builtin operands=[sum as int32 families=(integer), value as int32 families=(integer)]
        /// @resolution.pattern.assign source=sum kind=place
        /// @resolution.place source=sum placement="local" lifetime="frame" access="exclusive"
        /// @resolution.assignment source=sum read=binding(total.sum) write=binding(total.sum) type=int32
        /// @resolution.access source=sum root=total.sum
        /// @resolution.name source=value target=total.symbol7.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=total.symbol7.value

    });
    return sum;
    /// @resolution.name source=sum target=total.sum
    /// @resolution.place source=sum placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=sum root=total.sum

}
"#, r#"
"#);
}

/// Reject a function reference stored where its return must widen into a union.
#[test]
fn test_reject_storing_a_function_reference_into_a_union_returning_slot() {
    let session = TestSession::single(
        r#"
function increment(value: int32): int32 {
    return value + 1;
}

const widened: (value: int32) => int32 | undefined = increment;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function increment(value: int32): int32 {
    return value + 1;
}

const widened: (value: int32) => int32 | undefined = increment;

=== dir ===
function increment(value: int32): int32 {
/// @type.symbol symbol=increment type=(int32) => int32
/// @type.symbol symbol=increment.value source="value: int32" type=int32

    return value + 1;
    /// @resolution.name source=value target=increment.value
    /// @resolution.operator source="value + 1" type=int32 operator="+" kind=builtin operands=[value as int32 families=(integer), 1 as int32 families=(integer)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=increment.value

}

const widened: (value: int32) => int32 | undefined = increment;
/// @type.symbol symbol=widened source=widened type=(int32) => int32 | undefined
/// @resolution.pattern source=widened kind=binding target=widened
/// @type.symbol symbol=value source="value: int32" type=int32
/// @resolution.name source=increment target=increment
/// @resolution.function source=increment type=Function<(int32,), int32, "readonly"> target=increment
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Function<(value: int32,), int32, \"readonly\">' is not assignable to type '(value: int32) => int32 | undefined'"
/// @diagnostic.label line=6 column=54 span="increment" line_source="const widened: (value: int32) => int32 | undefined = increment;"
/// @diagnostic.related line=6 column=16 span="(value: int32) => int32 | undefined" line_source="const widened: (value: int32) => int32 | undefined = increment;" message="expected due to this annotation"
"#,
    );
}

/// Reject a function reference argument whose return must widen into a union.
#[test]
fn test_reject_a_function_reference_argument_in_a_union_returning_callback_slot() {
    let session = TestSession::single(
        r#"
function increment(value: int32): int32 {
    return value + 1;
}

declare function applyOpen<U>(map: (value: int32) => U | undefined): U | undefined;

const out = applyOpen(increment);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function increment(value: int32): int32 {
    return value + 1;
}

declare function applyOpen<U>(map: (value: int32) => U | undefined): U | undefined;

const out: int32 | undefined = applyOpen<int32>(increment);

=== dir ===
function increment(value: int32): int32 {
/// @type.symbol symbol=increment type=(int32) => int32
/// @type.symbol symbol=increment.value source="value: int32" type=int32

    return value + 1;
    /// @resolution.name source=value target=increment.value
    /// @resolution.operator source="value + 1" type=int32 operator="+" kind=builtin operands=[value as int32 families=(integer), 1 as int32 families=(integer)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=increment.value

}

declare function applyOpen<U>(map: (value: int32) => U | undefined): U | undefined;
/// @generic.template symbol=applyOpen parameters=(U)
/// @type.symbol symbol=applyOpen type=<U>((int32) => U | undefined) => U | undefined
/// @type.symbol symbol=applyOpen.U source=U type=U
/// @type.symbol symbol=applyOpen.value source="value: int32" type=int32
/// @resolution.name source=U target=applyOpen.U
/// @resolution.name source=U target=applyOpen.U

const out = applyOpen(increment);
/// @type.symbol symbol=out source=out type=int32 | undefined
/// @resolution.pattern source=out kind=binding target=out
/// @resolution.name source=applyOpen target=applyOpen
/// @resolution.call source=applyOpen(increment) parameters=((int32) => int32 | undefined) arguments=(provided(increment) as (int32) => int32 | undefined) return=int32 | undefined kind=symbol target=applyOpen instance=applyOpen<int32>
/// @generic.instantiation id=applyOpen<int32> template=applyOpen arguments=(int32)
/// @resolution.name source=increment target=increment
/// @resolution.function source=increment type=Function<(int32,), int32, "readonly"> target=increment
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type 'Function<(value: int32,), int32, \"readonly\">' is not assignable to parameter of type '(value: int32) => int32 | undefined'"
/// @diagnostic.label line=8 column=23 span="increment" line_source="const out = applyOpen(increment);"
/// @diagnostic.related line=8 column=13 span="applyOpen(increment)" line_source="const out = applyOpen(increment);" message="in this call"
"#,
    );
}

/// Accept a lambda in a union-returning callback slot through contextual typing.
#[test]
fn test_accept_a_lambda_argument_in_a_union_returning_callback_slot() {
    let session = TestSession::single(
        r#"
declare function applyOpen<U>(map: (value: int32) => U | undefined): U | undefined;

const out: int32 | undefined = applyOpen((value: int32) => value + 1);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare function applyOpen<U>(map: (value: int32) => U | undefined): U | undefined;

const out: int32 | undefined = applyOpen<int32>(
    (value: int32): int32 | undefined => (value + 1) as int32 | undefined,
);

=== dir ===
declare function applyOpen<U>(map: (value: int32) => U | undefined): U | undefined;
/// @generic.template symbol=applyOpen parameters=(U)
/// @type.symbol symbol=applyOpen type=<U>((int32) => U | undefined) => U | undefined
/// @type.symbol symbol=applyOpen.U source=U type=U
/// @type.symbol symbol=applyOpen.value source="value: int32" type=int32
/// @resolution.name source=U target=applyOpen.U
/// @resolution.name source=U target=applyOpen.U

const out: int32 | undefined = applyOpen((value: int32) => value + 1);
/// @type.symbol symbol=out source=out type=int32 | undefined
/// @resolution.pattern source=out kind=binding target=out
/// @resolution.name source=applyOpen target=applyOpen
/// @resolution.call source="applyOpen((value: int32) => value + 1)" parameters=((int32) => int32 | undefined) arguments=(provided((value: int32) => value + 1) as (int32) => int32 | undefined) return=int32 | undefined kind=symbol target=applyOpen instance=applyOpen<int32>
/// @generic.instantiation id=applyOpen<int32> template=applyOpen arguments=(int32)
/// @type.symbol symbol=symbol5 source="(value: int32) => value + 1" type=Function<(int32,), int32 | undefined, "readonly">
/// @type.symbol symbol=symbol5.value source="value: int32" type=int32
/// @resolution.name source=value target=symbol5.value
/// @resolution.operator source="value + 1" type=int32 operator="+" kind=builtin operands=[value as int32 families=(integer), 1 as int32 families=(integer)]
/// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=value root=symbol5.value
"#,
        r#"
"#,
    );
}

/// Accept a function reference upcasting its class return in a stored slot.
#[test]
fn test_accept_a_function_reference_upcasting_its_class_return() {
    let session = TestSession::single(
        r#"
class Animal {}
class Dog extends Animal {}

declare function makeDog(): Dog;

const covariant: () => Animal = makeDog;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class Animal {}
class Dog extends Animal {}

declare function makeDog(): Dog;

const covariant: () => Animal = makeDog;

=== dir ===
class Animal {}
/// @type.symbol symbol=Animal source="class Animal {}" type=typeof Animal
/// @definition.class symbol=Animal source="class Animal {}"

class Dog extends Animal {}
/// @type.symbol symbol=Dog source="class Dog extends Animal {}" type=typeof Dog
/// @definition.class symbol=Dog source="class Dog extends Animal {}"
/// @definition.extends symbol=Dog source=Animal target=Animal
/// @resolution.name source=Animal target=Animal

declare function makeDog(): Dog;
/// @type.symbol symbol=makeDog source="declare function makeDog(): Dog" type=() => Dog
/// @resolution.name source=Dog target=Dog

const covariant: () => Animal = makeDog;
/// @type.symbol symbol=covariant source=covariant type=() => Animal
/// @resolution.pattern source=covariant kind=binding target=covariant
/// @resolution.name source=Animal target=Animal
/// @resolution.name source=makeDog target=makeDog
/// @resolution.function source=makeDog type=Function<(), Dog, "readonly"> target=makeDog
"#,
        r#"
"#,
    );
}

/// Accept a function reference narrowing its parameter in a stored slot.
#[test]
fn test_accept_a_function_reference_narrowing_its_parameter() {
    let session = TestSession::single(
        r#"
class Animal {}
class Dog extends Animal {}

declare function eatAnimal(animal: Animal): void;

const contravariant: (dog: Dog) => void = eatAnimal;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class Animal {}
class Dog extends Animal {}

declare function eatAnimal(animal: Animal): void;

const contravariant: (dog: Dog) => void = eatAnimal;

=== dir ===
class Animal {}
/// @type.symbol symbol=Animal source="class Animal {}" type=typeof Animal
/// @definition.class symbol=Animal source="class Animal {}"

class Dog extends Animal {}
/// @type.symbol symbol=Dog source="class Dog extends Animal {}" type=typeof Dog
/// @definition.class symbol=Dog source="class Dog extends Animal {}"
/// @definition.extends symbol=Dog source=Animal target=Animal
/// @resolution.name source=Animal target=Animal

declare function eatAnimal(animal: Animal): void;
/// @type.symbol symbol=eatAnimal source="declare function eatAnimal(animal: Animal): void" type=(Animal) => void
/// @resolution.name source=Animal target=Animal

const contravariant: (dog: Dog) => void = eatAnimal;
/// @type.symbol symbol=contravariant source=contravariant type=(Dog) => void
/// @resolution.pattern source=contravariant kind=binding target=contravariant
/// @type.symbol symbol=dog source="dog: Dog" type=Dog
/// @resolution.name source=Dog target=Dog
/// @resolution.name source=eatAnimal target=eatAnimal
/// @resolution.function source=eatAnimal type=Function<(Animal,), void, "readonly"> target=eatAnimal
"#,
        r#"
"#,
    );
}

/// Accept a type predicate function in a boolean-returning slot.
#[test]
fn test_accept_a_type_predicate_function_in_a_boolean_slot() {
    let session = TestSession::single(
        r#"
class Animal {}
class Dog extends Animal {}

declare function isDog(animal: Animal): animal is Dog;

const predicate: (animal: Animal) => boolean = isDog;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class Animal {}
class Dog extends Animal {}

declare function isDog(animal: Animal): animal; is Dog;

const predicate: (animal: Animal) => boolean = isDog;

=== dir ===
class Animal {}
/// @type.symbol symbol=Animal source="class Animal {}" type=typeof Animal
/// @definition.class symbol=Animal source="class Animal {}"

class Dog extends Animal {}
/// @type.symbol symbol=Dog source="class Dog extends Animal {}" type=typeof Dog
/// @definition.class symbol=Dog source="class Dog extends Animal {}"
/// @definition.extends symbol=Dog source=Animal target=Animal
/// @resolution.name source=Animal target=Animal

declare function isDog(animal: Animal): animal is Dog;
/// @resolution.guard source="declare function isDog(animal: Animal): animal is Dog" kind=is value=void target=Dog predicate="void is subtype(Dog)" narrowed=Narrow<void, Dog>
/// @resolution.name source=Dog target=Dog

const predicate: (animal: Animal) => boolean = isDog;
/// @type.symbol symbol=predicate source=predicate type=(Animal) => boolean
/// @resolution.pattern source=predicate kind=binding target=predicate
/// @type.symbol symbol=animal source="animal: Animal" type=Animal
/// @resolution.name source=Animal target=Animal
/// @resolution.name source=isDog target=isDog
/// @resolution.function source=isDog type=(Animal) => boolean target=isDog
"#,
        r#"

"#,
    );
}

/// Reject a function value stored where its return must change representation.
#[test]
fn test_reject_a_function_value_whose_return_changes_representation() {
    let session = TestSession::single(
        r#"
class Dog {}

declare const makeInt: () => int32;
declare function makeDog(): Dog;

const widened: () => int64 = makeInt;
const erased: () => unknown = makeDog;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class Dog {}

declare const makeInt: () => int32;
declare function makeDog(): Dog;

const widened: () => int64 = makeInt;
const erased: () => unknown = makeDog;

=== dir ===
class Dog {}
/// @type.symbol symbol=Dog source="class Dog {}" type=typeof Dog
/// @definition.class symbol=Dog source="class Dog {}"

declare const makeInt: () => int32;
/// @type.symbol symbol=makeInt source=makeInt type=() => int32
/// @resolution.pattern source=makeInt kind=binding target=makeInt

declare function makeDog(): Dog;
/// @type.symbol symbol=makeDog source="declare function makeDog(): Dog" type=() => Dog
/// @resolution.name source=Dog target=Dog

const widened: () => int64 = makeInt;
/// @type.symbol symbol=widened source=widened type=() => int64
/// @resolution.pattern source=widened kind=binding target=widened
/// @resolution.name source=makeInt target=makeInt
/// @resolution.place source=makeInt placement="local" lifetime="static" access="immutable"
/// @resolution.access source=makeInt root=makeInt

const erased: () => unknown = makeDog;
/// @type.symbol symbol=erased source=erased type=() => unknown
/// @resolution.pattern source=erased kind=binding target=erased
/// @resolution.name source=makeDog target=makeDog
/// @resolution.function source=makeDog type=Function<(), Dog, "readonly"> target=makeDog
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '() => int32' is not assignable to type '() => int64'"
/// @diagnostic.label line=7 column=30 span="makeInt" line_source="const widened: () => int64 = makeInt;"
/// @diagnostic.related line=7 column=16 span="() => int64" line_source="const widened: () => int64 = makeInt;" message="expected due to this annotation"
/// @diagnostic.error id=not-assignable message="type 'Function<(), Dog, \"readonly\">' is not assignable to type '() => unknown'"
/// @diagnostic.label line=8 column=31 span="makeDog" line_source="const erased: () => unknown = makeDog;"
/// @diagnostic.related line=8 column=15 span="() => unknown" line_source="const erased: () => unknown = makeDog;" message="expected due to this annotation"
"#,
    );
}
