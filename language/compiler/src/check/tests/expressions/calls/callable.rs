use crate::tests::{DirRows, TestSession};

#[test]
fn test_callable_value_call_uses_function_type() {
    let session = TestSession::single(
        r#"
declare const transform: Function<(int32,), string>;

const text = transform(1);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_node_types().without_reference_types(),
        r#"
declare const transform: Function<(int32,), string>;
/// @type.symbol symbol=transform type=(int32) => string
/// @generic.application source="Function<(int32,), string>" id="types.function.Function<(int32,), string>"
/// @resolution.name source=Function target=types.function.Function

const text = transform(1);
/// @type.symbol symbol=text type=string
/// @type.node source=transform(1) type=string
/// @resolution.name source=transform target=transform
/// @resolution.call source=transform(1) parameters=(int32) return=string kind=symbol target=transform
/// @type.node source=1 type=int32
/// @generic.application id="types.function.Function<(int32,), string>" symbol=types.function.Function arguments=[(int32,), string]
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
const value = 1;
/// @type.symbol symbol=value type=1
/// @type.node source=1 type=1

value();
/// @resolution.name source=value target=value

"#,
        r#"
/// @diagnostic.error code=EC301 message="value is not callable"
/// @diagnostic.label line=3 column=1 source="value();"
"#,
    );
}
