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
/// @resolution.name source=Function target=types.function.Function

const text = transform(1);
/// @type.symbol symbol=text source=text type=string
/// @type.node source=transform(1) type=string
/// @resolution.name source=transform target=transform
/// @resolution.call source=transform(1) parameters=(int32) arguments=(provided(1) as int32) return=string kind=expression
/// @type.node source=1 type=1

/// @generic.instance id="Function<(int32,), string>" template=types.function.Function arguments=((int32,), string)
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
/// @type.node source=1 type=1

value();
/// @type.node source=value() type=<error>
/// @resolution.name source=value target=value
"#,
        r#"
/// @diagnostic.error code=EC301 message="value of type '1' is not callable"
/// @diagnostic.label line=3 column=1 span="value()" line_source="value();"
"#,
    );
}
