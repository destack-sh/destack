use crate::tests::{DirRows, TestSession};

#[test]
fn test_callable_value_invokes_function_type() {
    let session = TestSession::single(
        r#"
declare const transform: Function<(int32,), string>;

const text = transform(1);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked()
            .with_node_types()
            .without_reference_types(),
        r#"
=== annotated ===
declare const transform: (arg0: int32) => string;

const text: string = transform(1);

=== checked ===
declare const transform: Function<(int32,), string>;
/// @type.symbol symbol=transform source=transform type=Function<(int32,), string>
/// @resolution.pattern source=transform kind=binding target=transform
/// @resolution.name source=Function target=types.function.Function

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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked()
            .with_node_types()
            .without_reference_types(),
        r#"
=== annotated ===
declare const transform: Function<(string,), "left"> | Function<(string,), "right">;

const result: "left" | "right" = transform("value");

=== checked ===
declare const transform:
/// @type.symbol symbol=transform source=transform type=Function<(string,), "left"> | Function<(string,), "right">
/// @resolution.pattern source=transform kind=binding target=transform

    Function<(string,), "left"> |
    /// @resolution.name source=Function target=types.function.Function

    Function<(string,), "right">;
    /// @resolution.name source=Function target=types.function.Function

const result = transform("value");
/// @type.symbol symbol=result source=result type="left" | "right"
/// @resolution.pattern source=result kind=binding target=result
/// @type.node source="transform(\"value\")" type="left" | "right"
/// @resolution.name source=transform target=transform
/// @resolution.call source="transform(\"value\")" return="left" | "right" kind=union arms=[expression(parameters=(string), arguments=(provided("value") as string), return="left"), expression(parameters=(string), arguments=(provided("value") as string), return="right")]
/// @resolution.place source=transform placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=transform root=transform
/// @type.node source="\"value\"" type="value"

/// @generic.instance id="Function<(string,), \"left\">" template=types.function.Function arguments=((string,), "left")
/// @generic.instance id="Function<(string,), \"right\">" template=types.function.Function arguments=((string,), "right")
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

    session.assert_dir_checked_and_diagnostics(
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

=== checked ===
declare const transform:
/// @type.symbol symbol=transform source=transform type=Function<(int32,), "common"> & Function<(int32,), "left"> | Function<(int32,), "common"> & Function<(int32,), "right">
/// @resolution.pattern source=transform kind=binding target=transform

    (Function<(int32,), "common"> & Function<(int32,), "left">) |
    /// @resolution.name source=Function target=types.function.Function
    /// @resolution.name source=Function target=types.function.Function

    (Function<(int32,), "common"> & Function<(int32,), "right">);
    /// @resolution.name source=Function target=types.function.Function
    /// @resolution.name source=Function target=types.function.Function

const result: "left" | "right" = transform(1);
/// @type.symbol symbol=result source=result type="left" | "right"
/// @resolution.pattern source=result kind=binding target=result
/// @type.node source=transform(1) type="common"
/// @resolution.name source=transform target=transform
/// @resolution.call source=transform(1) return="common" kind=union arms=[expression(parameters=(int32), arguments=(provided(1) as int32), return="common"), expression(parameters=(int32), arguments=(provided(1) as int32), return="common")]
/// @resolution.place source=transform placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=transform root=transform
/// @type.node source=1 type=1

/// @generic.instance id="Function<(int32,), \"common\">" template=types.function.Function arguments=((int32,), "common")
/// @generic.instance id="Function<(int32,), \"left\">" template=types.function.Function arguments=((int32,), "left")
/// @generic.instance id="Function<(int32,), \"right\">" template=types.function.Function arguments=((int32,), "right")
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked()
            .with_node_types()
            .without_reference_types(),
        r#"
=== annotated ===
const value: 1 = 1;
value();

=== checked ===
const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1

value();
/// @type.node source=value() type=<error>
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Adder {
    (left: int32, right: int32): int32;
}

declare const add: Dynamic<Adder>;
const sum: int32 = add(1, 2);

=== checked ===
interface Adder {
/// @type.symbol symbol=Adder type=Adder
/// @definition.interface symbol=Adder
/// @definition.signature kind=call source="(left: int32, right: int32): int32" type=Function<(int32, int32), int32>

    (left: int32, right: int32): int32;
    /// @type.symbol symbol=Adder.left source="left: int32" type=int32
    /// @type.symbol symbol=Adder.right source="right: int32" type=int32

}

declare const add: Adder;
/// @type.symbol symbol=add source=add type=Dynamic<Adder>
/// @resolution.pattern source=add kind=binding target=add
/// @resolution.name source=Adder target=Adder

const sum = add(1, 2);
/// @type.symbol symbol=sum source=sum type=int32
/// @resolution.pattern source=sum kind=binding target=sum
/// @resolution.name source=add target=add
/// @resolution.call source="add(1, 2)" parameters=(int32, int32) arguments=(provided(1) as int32, provided(2) as int32) return=int32 kind=dynamic target="call((left: int32, right: int32): int32)" receiver=Dynamic<Adder> constraint=Adder
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

    session.assert_dir_checked(
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

declare const factory: Dynamic<Factory>;
const counter: Counter = new factory(1);

=== checked ===
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
/// @definition.signature kind=construct source="new (value: int32): Counter" type=new (int32) => Counter

    new (value: int32): Counter;
    /// @type.symbol symbol=Factory.value source="value: int32" type=int32
    /// @resolution.name source=Counter target=Counter

}

declare const factory: Factory;
/// @type.symbol symbol=factory source=factory type=Dynamic<Factory>
/// @resolution.pattern source=factory kind=binding target=factory
/// @resolution.name source=Factory target=Factory

const counter = new factory(1);
/// @type.symbol symbol=counter source=counter type=Counter
/// @resolution.pattern source=counter kind=binding target=counter
/// @resolution.construct source="new factory(1)" parameters=(int32) arguments=(provided(1) as int32) return=Counter kind=dynamic target="construct(new (value: int32): Counter)" receiver=Dynamic<Factory> constraint=Factory
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Adder {
    (left: int32, right: int32): int32;
}

const add: Dynamic<Adder> = ((left: int32, right: int32): int32 => left + right) as Dynamic<Adder>;

=== checked ===
interface Adder {
/// @type.symbol symbol=Adder type=Adder
/// @definition.interface symbol=Adder
/// @definition.signature kind=call source="(left: int32, right: int32): int32" type=Function<(int32, int32), int32>

    (left: int32, right: int32): int32;
    /// @type.symbol symbol=Adder.left source="left: int32" type=int32
    /// @type.symbol symbol=Adder.right source="right: int32" type=int32

}

const add: Adder = (left: int32, right: int32): int32 => left + right;
/// @type.symbol symbol=add source=add type=Dynamic<Adder>
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
