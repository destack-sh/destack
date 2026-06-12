use crate::tests::{DirRows, TestSession};

#[test]
fn test_const_character_preserves_literal_type() {
    let session = TestSession::single(
        r#"
const value = 'a';
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: 'a' = 'a';

=== checked ===
const value = 'a';
/// @type.symbol symbol=value source=value type='a'
/// @type.node source='a' type='a'

/// @check.stats.solve variables=0 types=2 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
let value: char = 'a';

=== checked ===
let value = 'a';
/// @type.symbol symbol=value source=value type=char
/// @type.node source='a' type='a'

/// @check.stats.solve variables=0 types=3 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_annotation_sets_binding_type() {
    let session = TestSession::single(
        r#"
const value: char = 'a';
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: char = 'a';

=== checked ===
const value: char = 'a';
/// @type.symbol symbol=value source=value type=char
/// @type.node source='a' type='a'

/// @check.stats.solve variables=0 types=3 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: string = 'a';

=== checked ===
const value: string = 'a';
/// @type.symbol symbol=value source=value type=string
/// @type.node source='a' type='a'

/// @check.stats.solve variables=0 types=3 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0

"#,
        r#"

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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: int32 = 'a';

=== checked ===
const value: int32 = 'a';
/// @type.symbol symbol=value source=value type=int32
/// @type.node source='a' type='a'

/// @check.stats.solve variables=0 types=3 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0

"#,
        r#"
/// @diagnostic.error code=EC200 message="type ''a'' is not assignable to type 'int32'"
/// @diagnostic.label line=2 column=22 source="const value: int32 = 'a';"
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: char | string = 'a';

=== checked ===
const value: char | string = 'a';
/// @type.symbol symbol=value source=value type=char | string
/// @type.node source='a' type='a'

/// @check.stats.solve variables=0 types=5 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}
