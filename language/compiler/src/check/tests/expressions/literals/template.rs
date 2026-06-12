use crate::tests::{DirRows, TestSession};

#[test]
fn test_template_literal_has_string_type() {
    let session = TestSession::single(
        r#"
const name = "Ada";
const greeting = `hello ${name}`;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const name: "Ada" = "Ada";
const greeting: string = `hello ${name}`;

=== checked ===
const name = "Ada";
/// @type.symbol symbol=name source=name type="Ada"
/// @type.node source="\"Ada\"" type="Ada"

const greeting = `hello ${name}`;
/// @type.symbol symbol=greeting source=greeting type=string
/// @type.node source="`hello ${name}`" type=string
/// @type.node source=name type="Ada"
/// @resolution.name source=name target=name

/// @check.stats.solve variables=0 types=4 constraints=0 obligations=0 solutions=0 bounds=0 decisions=1
"#,
    );
}

#[test]
fn test_template_literal_is_assignable_to_string() {
    let session = TestSession::single(
        r#"
const greeting: string = `hello`;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const greeting: string = `hello`;

=== checked ===
const greeting: string = `hello`;
/// @type.symbol symbol=greeting source=greeting type=string
/// @type.node source=`hello` type=string

/// @check.stats.solve variables=0 types=3 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_template_literal_rejects_number_context() {
    let session = TestSession::single(
        r#"
const value: number = `hello`;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: number = `hello`;

=== checked ===
const value: number = `hello`;
/// @type.symbol symbol=value source=value type=float64
/// @type.node source=`hello` type=string

/// @check.stats.solve variables=0 types=3 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0

"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'string' is not assignable to type 'float64'"
/// @diagnostic.label line=2 column=23 source="const value: number = `hello`;"
"#,
    );
}
