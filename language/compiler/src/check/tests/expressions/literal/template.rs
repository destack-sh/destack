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
/// @resolution.pattern source=name kind=binding target=name
/// @type.node source="\"Ada\"" type="Ada"

const greeting = `hello ${name}`;
/// @type.symbol symbol=greeting source=greeting type=string
/// @resolution.pattern source=greeting kind=binding target=greeting
/// @type.node source="`hello ${name}`" type=string
/// @type.node source=name type="Ada"
/// @resolution.name source=name target=name

/// @check.stats.solve variables=2 types=5 constraints=0 obligations=2 solutions=2 bounds=0 decisions=3
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
/// @resolution.pattern source=greeting kind=binding target=greeting
/// @type.node source=`hello` type=string

/// @check.stats.solve variables=1 types=3 constraints=1 obligations=1 solutions=1 bounds=0 decisions=1
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
const value: float64 = `hello`;

=== checked ===
const value: number = `hello`;
/// @type.symbol symbol=value source=value type=float64
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=`hello` type=string

/// @check.stats.solve variables=1 types=4 constraints=1 obligations=1 solutions=1 bounds=0 decisions=1
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'string' is not assignable to type 'float64'"
/// @diagnostic.label line=2 column=23 span="`hello`" line_source="const value: number = `hello`;"
/// @diagnostic.related line=2 column=14 span="number" line_source="const value: number = `hello`;" message="expected due to this annotation"
"#,
    );
}
