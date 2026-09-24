use crate::tests::{DirRows, TestSession};

/// Report unannotated module and local bindings.
#[test]
fn test_report_unannotated_module_and_local_bindings() {
    let session = TestSession::single(
        r#"
const value;

function run(): void {
    let local;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const value;

function run(): void {
    let local;
}

=== dir ===
const value;
/// @type.symbol symbol=value source=value type=<error>
/// @resolution.pattern source=value kind=binding target=value

function run(): void {
/// @type.symbol symbol=run type=() => void

    let local;
    /// @type.symbol symbol=run.local source=local type=<error>
    /// @resolution.pattern source=local kind=binding target=run.local

}
"#,
        r#"
/// @diagnostic.error id=missing-type-annotation message="missing type annotation"
/// @diagnostic.label line=5 column=9 span="local" line_source="let local;"
/// @diagnostic.error id=missing-type-annotation message="missing type annotation"
/// @diagnostic.label line=2 column=7 span="value" line_source="const value;"
"#,
    );
}

/// Report an unannotated ambient binding.
#[test]
fn test_report_unannotated_ambient_binding() {
    let session = TestSession::single(
        r#"
declare const value;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const value;

=== dir ===
declare const value;
/// @type.symbol symbol=value source=value type=<error>
/// @resolution.pattern source=value kind=binding target=value
"#,
        r#"
/// @diagnostic.error id=missing-type-annotation message="missing type annotation"
/// @diagnostic.label line=2 column=15 span="value" line_source="declare const value;"
"#,
    );
}

/// Report a written type hole on a module binding.
#[test]
fn test_report_written_hole_on_module_binding() {
    let session = TestSession::single(
        r#"
const value: _ = 1;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const value: _ = 1;

=== dir ===
const value: _ = 1;
/// @type.symbol symbol=value source=value type=<error>
/// @resolution.pattern source=value kind=binding target=value
"#,
        r#"
/// @diagnostic.error id=cannot-infer-type message="cannot infer a type here"
/// @diagnostic.label line=2 column=14 span="_" line_source="const value: _ = 1;"
/// @diagnostic.help message="annotate the type explicitly"
"#,
    );
}

/// Report a written type hole on a member.
#[test]
fn test_report_written_hole_on_member() {
    let session = TestSession::single(
        r#"
class Box {
    value: _ = 1;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Box {
    value: _ = 1;
}

=== dir ===
class Box {
/// @type.symbol symbol=Box type=typeof Box
/// @definition.class symbol=Box
/// @definition.field symbol=Box.value source="value: _ = 1" key=value type=<error>

    value: _ = 1;
    /// @type.symbol symbol=Box.value source="value: _ = 1" type=<error>

}
"#,
        r#"
/// @diagnostic.error id=cannot-infer-type message="cannot infer a type here"
/// @diagnostic.label line=3 column=12 span="_" line_source="value: _ = 1;"
/// @diagnostic.help message="annotate the type explicitly"
"#,
    );
}

/// Infer a written type hole on a local binding.
#[test]
fn test_infer_written_hole_on_local_binding() {
    let session = TestSession::single(
        r#"
function build(): void {
    const value: _ = 1;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function build(): void {
    const value: 1 = 1;
}

=== dir ===
function build(): void {
/// @type.symbol symbol=build type=() => void

    const value: _ = 1;
    /// @type.symbol symbol=build.value source=value type=1
    /// @resolution.pattern source=value kind=binding target=build.value

}
"#,
        r#"
"#,
    );
}
