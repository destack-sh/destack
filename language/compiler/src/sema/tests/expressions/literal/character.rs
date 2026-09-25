use crate::tests::{DirRows, TestSession};

#[test]
fn test_const_character_preserves_literal_type() {
    let session = TestSession::single(
        r#"
const value = 'a';
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: 'a' = 'a';

=== dir ===
const value = 'a';
/// @type.symbol symbol=value source=value type='a'
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source='a' type='a'
"#,
    );
}

#[test]
fn test_let_character_widens_binding_type() {
    let session = TestSession::single(
        r#"
let value = 'a';
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let value: char = 'a';

=== dir ===
let value = 'a';
/// @type.symbol symbol=value source=value type=char
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source='a' type='a'
"#,
    );
}

#[test]
fn test_character_annotation_sets_binding_type() {
    let session = TestSession::single(
        r#"
const value: char = 'a';
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: char = 'a';

=== dir ===
const value: char = 'a';
/// @type.symbol symbol=value source=value type=char
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source='a' type='a'
"#,
    );
}

#[test]
fn test_character_literal_rejects_string_context() {
    let session = TestSession::single(
        r#"
const value: string = 'a';
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: string = 'a';

=== dir ===
const value: string = 'a';
/// @type.symbol symbol=value source=value type=string
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source='a' type='a'
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type ''a'' is not assignable to type 'string'"
/// @diagnostic.label line=2 column=23 span="'a'" line_source="const value: string = 'a';"
/// @diagnostic.related line=2 column=14 span="string" line_source="const value: string = 'a';" message="expected due to this annotation"
"#,
    );
}

#[test]
fn test_character_literal_rejects_integer_context() {
    let session = TestSession::single(
        r#"
const value: int32 = 'a';
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: int32 = 'a';

=== dir ===
const value: int32 = 'a';
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source='a' type='a'
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type ''a'' is not assignable to type 'int32'"
/// @diagnostic.label line=2 column=22 span="'a'" line_source="const value: int32 = 'a';"
/// @diagnostic.related line=2 column=14 span="int32" line_source="const value: int32 = 'a';" message="expected due to this annotation"
"#,
    );
}

#[test]
fn test_character_literal_flows_into_character_union() {
    let session = TestSession::single(
        r#"
const value: char | string = 'a';
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: char | string = 'a' as char | string;

=== dir ===
const value: char | string = 'a';
/// @type.symbol symbol=value source=value type=char | string
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source='a' type='a'
"#,
    );
}
