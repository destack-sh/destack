use crate::tests::{DirRows, TestSession};

/// The unit value has the unit type and satisfies void.
#[test]
fn test_unit_value_has_unit_type() {
    let session = TestSession::single(
        r#"
const value = ();
value satisfies ();
value satisfies void;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: () = ();
value satisfies ();
value satisfies void;

=== dir ===
const value = ();
/// @type.symbol symbol=value source=value type=()
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=() type=()

value satisfies ();
/// @type.node source="value satisfies ()" type=()
/// @type.node source=value type=()
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value

value satisfies void;
/// @type.node source="value satisfies void" type=()
/// @type.node source=value type=()
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
"#,
    );
}

/// A void annotation accepts the unit value.
#[test]
fn test_void_annotation_accepts_unit_value() {
    let session = TestSession::single(
        r#"
const value: void = ();
value satisfies ();
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: void = ();
value satisfies ();

=== dir ===
const value: void = ();
/// @type.symbol symbol=value source=value type=void
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=() type=()

value satisfies ();
/// @type.node source="value satisfies ()" type=void
/// @type.node source=value type=void
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
"#,
    );
}

/// String escapes decode into the literal type of the decoded text.
#[test]
fn test_decode_string_escapes_into_literal_types() {
    let session = TestSession::single(
        r#"
const letter: "A" = "\u0041";
const tab: "a\tb" = "a\tb";
const slash: "\\w+" = "\\w+";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const letter: "A" = "\u0041";
const tab: "a\tb" = "a\tb";
const slash: "\\w+" = "\\w+";

=== dir ===
const letter: "A" = "\u0041";
/// @type.symbol symbol=letter source=letter type="A"
/// @resolution.pattern source=letter kind=binding target=letter

const tab: "a\tb" = "a\tb";
/// @type.symbol symbol=tab source=tab type="a\tb"
/// @resolution.pattern source=tab kind=binding target=tab

const slash: "\\w+" = "\\w+";
/// @type.symbol symbol=slash source=slash type="\\w+"
/// @resolution.pattern source=slash kind=binding target=slash
"#,
        r#"
"#,
    );
}
