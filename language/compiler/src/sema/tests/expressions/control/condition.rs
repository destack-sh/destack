use crate::tests::{DirRows, TestSession};

#[test]
fn test_literal_condition_reports_warning() {
    let session = TestSession::single(
        r#"
if (true) {
    const value = 1;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
if (true) {
    const value: 1 = 1;
}

=== dir ===
if (true) {
    const value = 1;
    /// @type.symbol symbol=value source=value type=1
    /// @resolution.pattern source=value kind=binding target=value

}
"#,
        r#"
/// @diagnostic.warning id=constant-condition message="condition is always true"
/// @diagnostic.label line=2 column=5 span="true" line_source="if (true) {"
"#,
    );
}

#[test]
fn test_warn_about_an_endless_while_condition() {
    let session = TestSession::single(
        r#"
while (true) {
    break;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
while (true) {
    break;
}

=== dir ===
while (true) {
    break;
    /// @resolution.transfer source=break target=while

}
"#,
        r#"
/// @diagnostic.warning id=constant-condition message="condition is always true"
/// @diagnostic.label line=2 column=8 span="true" line_source="while (true) {"
"#,
    );
}

#[test]
fn test_warn_on_a_constant_while_condition() {
    let session = TestSession::single(
        r#"
const always = true;

while (always) {
    break;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const always: true = true;

while (always) {
    break;
}

=== dir ===
const always = true;
/// @type.symbol symbol=always source=always type=true
/// @resolution.pattern source=always kind=binding target=always

while (always) {
/// @resolution.name source=always target=always
/// @resolution.place source=always placement="local" lifetime="static" access="immutable"
/// @resolution.access source=always root=always

    break;
    /// @resolution.transfer source=break target=while

}
"#,
        r#"
/// @diagnostic.warning id=constant-condition message="condition is always true"
/// @diagnostic.label line=4 column=8 span="always" line_source="while (always) {"
"#,
    );
}
