use crate::tests::{DirRows, TestSession};

#[test]
fn test_unused_generic_parameter_reports_error() {
    let session = TestSession::single(
        r#"
class Tag<T> {
    name: string;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Tag<T> {
    name: string;
}

=== checked ===
class Tag<T> {
/// @generic.template symbol=Tag parameters=(T)
/// @type.symbol symbol=Tag type=Tag
/// @definition.class symbol=Tag template=(T)
/// @definition.field symbol=Tag.name source="name: string" key=name type=string
/// @type.symbol symbol=Tag.T source=T type=T

    name: string;
    /// @type.symbol symbol=Tag.name source="name: string" type=string

}
"#,
        r#"
/// @diagnostic.error id=field-not-definitely-initialized message="field 'name' is not initialized on every constructor path"
/// @diagnostic.label line=3 column=5 span="name" line_source="name: string;"
/// @diagnostic.error id=unused-generic-parameter message="generic parameter 'T' is never used"
/// @diagnostic.label line=2 column=11 span="T" line_source="class Tag<T> {"
/// @diagnostic.help message="declare explicit variance like 'out T' to keep a marker parameter"
"#,
    );
}

#[test]
fn test_explicit_variance_keeps_a_marker_parameter() {
    let session = TestSession::single(
        r#"
class Tag<in out T> {
    name: string;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Tag<in out T> {
    name: string;
}

=== checked ===
class Tag<in out T> {
/// @generic.template symbol=Tag parameters=(in out T)
/// @type.symbol symbol=Tag type=Tag
/// @definition.class symbol=Tag template=(in out T)
/// @definition.field symbol=Tag.name source="name: string" key=name type=string
/// @type.symbol symbol=Tag.T source="in out T" type=T

    name: string;
    /// @type.symbol symbol=Tag.name source="name: string" type=string

}
"#,
        r#"
/// @diagnostic.error id=field-not-definitely-initialized message="field 'name' is not initialized on every constructor path"
/// @diagnostic.label line=3 column=5 span="name" line_source="name: string;"
"#,
    );
}

#[test]
fn test_comptime_parameter_needs_no_occurrence() {
    let session = TestSession::single(
        r#"
struct Fixed<comptime N: int> {
    name: string;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Fixed<comptime N: int> {
    name: string;
}

=== checked ===
struct Fixed<comptime N: int> {
/// @generic.template symbol=Fixed parameters=(comptime N: int64)
/// @type.symbol symbol=Fixed type=Fixed
/// @definition.struct symbol=Fixed template=(comptime N: int64)
/// @definition.field symbol=Fixed.name source="name: string" key=name type=string
/// @type.symbol symbol=Fixed.N source="comptime N: int" type=N

    name: string;
    /// @type.symbol symbol=Fixed.name source="name: string" type=string

}
"#,
        r#"
"#,
    );
}
