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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Tag<T> {
    name: string;
}

=== dir ===
class Tag<T> {
/// @generic.template symbol=Tag parameters=(T)
/// @type.symbol symbol=Tag type=typeof Tag
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Tag<in out T> {
    name: string;
}

=== dir ===
class Tag<in out T> {
/// @generic.template symbol=Tag parameters=(in out T)
/// @type.symbol symbol=Tag type=typeof Tag
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

/// Report one const parameter that never occurs in its declaration.
#[test]
fn test_report_an_unused_const_parameter() {
    let session = TestSession::single(
        r#"
struct Fixed<const N: int> {
    name: string;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Fixed<const N: int> {
    name: string;
}

=== dir ===
struct Fixed<const N: int> {
/// @generic.template symbol=Fixed parameters=(const N: int64)
/// @type.symbol symbol=Fixed type=Fixed
/// @definition.struct symbol=Fixed template=(const N: int64)
/// @definition.field symbol=Fixed.name source="name: string" key=name type=string
/// @type.symbol symbol=Fixed.N source="const N: int" type=N

    name: string;
    /// @type.symbol symbol=Fixed.name source="name: string" type=string

}
"#,
        r#"
/// @diagnostic.error id=unused-generic-parameter message="generic parameter 'N' is never used"
/// @diagnostic.label line=2 column=20 span="N" line_source="struct Fixed<const N: int> {"
/// @diagnostic.help message="declare explicit variance like 'out T' to keep a marker parameter"
"#,
    );
}
