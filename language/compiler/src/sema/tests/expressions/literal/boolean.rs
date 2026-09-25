use crate::tests::{DirRows, TestSession};

#[test]
fn test_const_boolean_preserves_literal_type() {
    let session = TestSession::single(
        r#"
const value = true;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: true = true;

=== dir ===
const value = true;
/// @type.symbol symbol=value source=value type=true
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=true type=true
"#,
    );
}

#[test]
fn test_let_boolean_widens_binding_type() {
    let session = TestSession::single(
        r#"
let value = true;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let value: boolean = true;

=== dir ===
let value = true;
/// @type.symbol symbol=value source=value type=boolean
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=true type=true
"#,
    );
}

#[test]
fn test_boolean_annotation_sets_binding_type() {
    let session = TestSession::single(
        r#"
const value: boolean = false;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: boolean = false;

=== dir ===
const value: boolean = false;
/// @type.symbol symbol=value source=value type=boolean
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=false type=false
"#,
    );
}

#[test]
fn test_false_const_preserves_literal_type() {
    let session = TestSession::single(
        r#"
const value = false;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: false = false;

=== dir ===
const value = false;
/// @type.symbol symbol=value source=value type=false
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=false type=false
"#,
    );
}

#[test]
fn test_boolean_literal_rejects_string_context() {
    let session = TestSession::single(
        r#"
const value: string = true;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: string = true;

=== dir ===
const value: string = true;
/// @type.symbol symbol=value source=value type=string
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=true type=true
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'true' is not assignable to type 'string'"
/// @diagnostic.label line=2 column=23 span="true" line_source="const value: string = true;"
/// @diagnostic.related line=2 column=14 span="string" line_source="const value: string = true;" message="expected due to this annotation"
"#,
    );
}

#[test]
fn test_boolean_literal_rejects_number_context() {
    let session = TestSession::single(
        r#"
const value: number = false;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: float64 = false;

=== dir ===
const value: number = false;
/// @type.symbol symbol=value source=value type=float64
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=false type=false
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'false' is not assignable to type 'float64'"
/// @diagnostic.label line=2 column=23 span="false" line_source="const value: number = false;"
/// @diagnostic.related line=2 column=14 span="number" line_source="const value: number = false;" message="expected due to this annotation"
"#,
    );
}
