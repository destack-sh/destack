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
declare const transform: Function<(int32,), string>;

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

/// @generic.instance id="Function<(int32,), string>" template=types.function.Function arguments=((int32,), string)
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
fn test_callable_union_uses_return_context_for_every_runtime_arm() {
    let session = TestSession::single(
        r#"
declare const transform:
    (Function<(int32,), "common"> & Function<(int32,), "left">) |
    (Function<(int32,), "common"> & Function<(int32,), "right">);

const result: "left" | "right" = transform(1);
"#,
    );

    session.assert_dir_checked(
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
/// @type.node source=transform(1) type="left" | "right"
/// @resolution.name source=transform target=transform
/// @resolution.call source=transform(1) return="left" | "right" kind=union arms=[expression(parameters=(int32), arguments=(provided(1) as int32), return="left"), expression(parameters=(int32), arguments=(provided(1) as int32), return="right")]
/// @resolution.place source=transform placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=transform root=transform
/// @type.node source=1 type=1

/// @generic.instance id="Function<(int32,), \"common\">" template=types.function.Function arguments=((int32,), "common")
/// @generic.instance id="Function<(int32,), \"left\">" template=types.function.Function arguments=((int32,), "left")
/// @generic.instance id="Function<(int32,), \"right\">" template=types.function.Function arguments=((int32,), "right")
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
"#,
        r#"
/// @diagnostic.error id=not-callable message="value of type '1' is not callable"
/// @diagnostic.label line=3 column=1 span="value()" line_source="value();"
"#,
    );
}
