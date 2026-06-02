use crate::tests::{DirRows, TestSession};

#[test]
fn test_regex_literal_has_regexp_type() {
    let session = TestSession::single(
        r#"
const value: RegExp = /abc/;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
const value: RegExp = /abc/;
/// @type.symbol symbol=value type=regexp.regexp.RegExp
/// @resolution.name source=RegExp target=regexp.regexp.RegExp
/// @type.node source=/abc/ type=regexp.regexp.RegExp

/// @check.stats.solve variables=0 terms=3 constraints=1 obligations=0 solutions=0 bounds=0 decisions=1
"#,
    );
}

#[test]
fn test_regex_literal_rejects_string_context() {
    let session = TestSession::single(
        r#"
const value: string = /abc/;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
const value: string = /abc/;
/// @type.symbol symbol=value type=string
/// @type.node source=/abc/ type=regexp.regexp.RegExp

/// @check.stats.solve variables=0 terms=3 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0

"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=2 column=23 source="const value: string = /abc/;"
"#,
    );
}

#[test]
fn test_regex_literal_flows_into_regexp_union() {
    let session = TestSession::single(
        r#"
const value: RegExp | int32 = /abc/;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
const value: RegExp | int32 = /abc/;
/// @type.symbol symbol=value type=regexp.regexp.RegExp | int32
/// @resolution.name source=RegExp target=regexp.regexp.RegExp
/// @type.node source=/abc/ type=regexp.regexp.RegExp

/// @check.stats.solve variables=0 terms=5 constraints=1 obligations=0 solutions=0 bounds=0 decisions=1
"#,
    );
}

#[test]
fn test_regex_literal_rejects_boolean_context() {
    let session = TestSession::single(
        r#"
const value: boolean = /abc/;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
const value: boolean = /abc/;
/// @type.symbol symbol=value type=boolean
/// @type.node source=/abc/ type=regexp.regexp.RegExp

/// @check.stats.solve variables=0 terms=3 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0

"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=2 column=24 source="const value: boolean = /abc/;"
"#,
    );
}
