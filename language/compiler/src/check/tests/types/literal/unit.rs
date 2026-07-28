use crate::tests::{DirRows, TestSession};

#[test]
fn test_unit_value_has_unit_type() {
    let session = TestSession::single(
        r#"
const value = ();
value satisfies ();
value satisfies void;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: () = ();
value satisfies ();
value satisfies void;

=== checked ===
const value = ();
/// @type.symbol symbol=value source=value type=()
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=() type=()

value satisfies ();
/// @type.node source="value satisfies ()" type=()
/// @type.node source=value type=()
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value

value satisfies void;
/// @type.node source="value satisfies void" type=()
/// @type.node source=value type=()
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
"#,
    );
}

#[test]
fn test_void_annotation_accepts_unit_value() {
    let session = TestSession::single(
        r#"
const value: void = ();
value satisfies ();
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: void = ();
value satisfies ();

=== checked ===
const value: void = ();
/// @type.symbol symbol=value source=value type=void
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=() type=()

value satisfies ();
/// @type.node source="value satisfies ()" type=void
/// @type.node source=value type=void
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
"#,
    );
}
