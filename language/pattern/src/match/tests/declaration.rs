use tspp_dir as dir;

use crate::tests::TestMatcher;

/// Match complete function declarations through their structural fields.
#[test]
fn test_match_function_declaration() {
    TestMatcher::context(
        "function $NAME<$$$GENERICS>($$$PARAMETERS): $RETURN { $BODY }",
        dir::NodeType::Declaration,
        r#"
function parse<T>(value: T): T { value }
function reset(): void {}
"#,
    )
    .assert(
        r#"
function parse<T>(value: T): T { value }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ match NAME.name="parse" GENERICS.nodes=["T"] PARAMETERS.nodes=["value: T"] RETURN.node="T" BODY.node="value"
function reset(): void {}
"#,
    );
}

/// Match enum fields selected from a declaration context.
#[test]
fn test_match_enum_field() {
    TestMatcher::context(
        "enum State { $NAME = $VALUE }",
        dir::NodeType::EnumField,
        r#"
enum State {
    Ready = 1;
    Waiting = 2
}
"#,
    )
    .assert(
        r#"
enum State {
    Ready = 1;
    ^^^^^^^^^ match NAME.name="Ready" VALUE.node="1"
    Waiting = 2
    ^^^^^^^^^^^ match NAME.name="Waiting" VALUE.node="2"
}
"#,
    );
}

/// Match decorators selected from their declaration context.
#[test]
fn test_match_decorator() {
    TestMatcher::context(
        "@trace($VALUE)\nfunction placeholder(): void {}",
        dir::NodeType::Decorator,
        r#"
@trace("first")
function first(): void {}

@other("second")
function second(): void {}
"#,
    )
    .assert(
        r#"
@trace("first")
^^^^^^^^^^^^^^^ match VALUE.node="\"first\""
function first(): void {}

@other("second")
function second(): void {}
"#,
    );
}

/// Constrain an opaque node metavariable with an authored decorator.
#[test]
fn test_match_decorated_metavariable() {
    TestMatcher::new(
        "@trace\n$NODE",
        r#"
@trace
first()

@other
second()
"#,
    )
    .assert(
        r#"
@trace
first()
^^^^^^^ match NODE.node="first()"

@other
second()
"#,
    );
}
