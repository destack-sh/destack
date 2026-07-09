use crate::tests::{DirRows, TestSession};

#[test]
fn test_unresolved_decorator_reports_error() {
    let session = TestSession::single(
        r#"
@missing
const value = 1;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
@missing
const value: 1 = 1;

=== checked ===
@missing
const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @type.node source=1 type=1
"#,
        r#"
/// @diagnostic.error code=EC308 message="cannot find 'missing'"
/// @diagnostic.label line=2 column=2 span="missing" line_source="@missing"
"#,
    );
}

#[test]
fn test_ambiguous_decorator_reports_error() {
    let session = TestSession::builder()
        .module("first.ds", "export const mark = 1;\n")
        .module("second.ds", "export const mark = 1;\n")
        .module(
            "main.ds",
            r#"
import { mark } from "./first.ds";
import { mark } from "./second.ds";

@mark
const value = 1;
"#,
        )
        .build();

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { mark } from "./first.ds";
import { mark } from "./second.ds";

@mark
const value: 1 = 1;

=== checked ===
import { mark } from "./first.ds";
import { mark } from "./second.ds";

@mark
const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @type.node source=1 type=1
"#,
        r#"
/// @diagnostic.error code=EC309 message="ambiguous reference 'mark'"
/// @diagnostic.label line=5 column=2 span="mark" line_source="@mark"
"#,
    );
}

#[test]
fn test_namespace_member_decorator_resolves() {
    let session = TestSession::builder()
        .module("marks.ds", "export const mark = 1;\n")
        .module(
            "main.ds",
            r#"
import * as marks from "./marks.ds";

@marks.mark
const value = 1;
"#,
        )
        .build();

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import * as marks from "./marks.ds";

@marks.mark
const value: 1 = 1;

=== checked ===
import * as marks from "./marks.ds";

@marks.mark
/// @resolution.name source=marks.mark target=marks.mark

const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @type.node source=1 type=1
"#,
    );
}
