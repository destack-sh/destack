use crate::tests::{DirRows, TestSession};

#[test]
fn test_false_static_branch_is_not_checked() {
    let session = TestSession::single(
        r#"
@if(false)
const hidden: MissingType = missingValue;

const visible = 1;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
@if(false)
const hidden: MissingType = missingValue;

const visible: 1 = 1;

=== dir ===
@if(false)
const hidden: MissingType = missingValue;

const visible = 1;
/// @type.symbol symbol=visible source=visible type=1
/// @resolution.pattern source=visible kind=binding target=visible
/// @type.node source=1 type=1
"#,
    );
}

#[test]
fn test_true_static_branch_is_checked() {
    let session = TestSession::single(
        r#"
@if(true)
const value: int32 = "text";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
@if(true)
const value: int32 = "text";

=== dir ===
@if(true)
const value: int32 = "text";
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="\"text\"" type="text"
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"text\"' is not assignable to type 'int32'"
/// @diagnostic.label line=3 column=22 span="\"text\"" line_source="const value: int32 = \"text\";"
/// @diagnostic.related line=3 column=14 span="int32" line_source="const value: int32 = \"text\";" message="expected due to this annotation"
"#,
    );
}

#[test]
fn test_static_if_rejects_non_boolean_condition() {
    let session = TestSession::single(
        r#"
@if(1)
const value = 1;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
@if(1)
const value = 1;

=== dir ===
@if(1)
const value = 1;
"#,
        r#"
/// @diagnostic.error id=invalid-static-condition message="static condition must evaluate to a boolean"
/// @diagnostic.label line=2 column=5 span="1" line_source="@if(1)"
"#,
    );
}

#[test]
fn test_static_if_rejects_runtime_condition() {
    let session = TestSession::single(
        r#"
let enabled = true;

@if(enabled)
const value = 1;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let enabled: boolean = true;

@if(enabled)
const value = 1;

=== dir ===
let enabled = true;
/// @type.symbol symbol=enabled source=enabled type=boolean
/// @resolution.pattern source=enabled kind=binding target=enabled
/// @type.node source=true type=true

@if(enabled)
const value = 1;
"#,
        r#"
/// @diagnostic.error id=undecidable-static-condition message="static @if condition must be statically decidable"
/// @diagnostic.label line=4 column=5 span="enabled" line_source="@if(enabled)"
"#,
    );
}

#[test]
fn test_static_if_rejects_generic_invocation() {
    let session = TestSession::single(
        r#"
@if<boolean>(true)
const value = 1;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
@if<boolean>(true)
const value = 1;

=== dir ===
@if<boolean>(true)
const value = 1;
"#,
        r#"
/// @diagnostic.error id=invalid-static-if-invocation message="`@if` must be invoked as `@if(condition)`"
/// @diagnostic.label line=2 column=1 span="@if<boolean>(true)" line_source="@if<boolean>(true)"
"#,
    );
}
