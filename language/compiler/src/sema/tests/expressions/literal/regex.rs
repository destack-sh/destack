use crate::tests::{DirRows, TestSession};

#[test]
fn test_regex_literal_has_regexp_type() {
    let session = TestSession::single(
        r#"
const value: RegExp = /abc/;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: RegExp = /abc/;

=== dir ===
const value: RegExp = /abc/;
/// @type.symbol symbol=value source=value type=RegExp
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=RegExp target=RegExp
/// @type.node source=/abc/ type=RegExp
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: string = /abc/;

=== dir ===
const value: string = /abc/;
/// @type.symbol symbol=value source=value type=string
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=/abc/ type=RegExp
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'RegExp' is not assignable to type 'string'"
/// @diagnostic.label line=2 column=23 span="/abc/" line_source="const value: string = /abc/;"
/// @diagnostic.related line=2 column=14 span="string" line_source="const value: string = /abc/;" message="expected due to this annotation"
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: RegExp | int32 = /abc/ as RegExp | int32;

=== dir ===
const value: RegExp | int32 = /abc/;
/// @type.symbol symbol=value source=value type=RegExp | int32
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=RegExp target=RegExp
/// @type.node source=/abc/ type=RegExp
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: boolean = /abc/;

=== dir ===
const value: boolean = /abc/;
/// @type.symbol symbol=value source=value type=boolean
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=/abc/ type=RegExp
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'RegExp' is not assignable to type 'boolean'"
/// @diagnostic.label line=2 column=24 span="/abc/" line_source="const value: boolean = /abc/;"
/// @diagnostic.related line=2 column=14 span="boolean" line_source="const value: boolean = /abc/;" message="expected due to this annotation"
"#,
    );
}
