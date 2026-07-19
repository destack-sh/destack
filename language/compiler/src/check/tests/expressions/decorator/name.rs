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
/// @diagnostic.error id=unresolved-reference message="cannot find 'missing'"
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
/// @diagnostic.error id=ambiguous-reference message="ambiguous reference 'mark'"
/// @diagnostic.label line=5 column=2 span="mark" line_source="@mark"
"#,
    );
}

#[test]
fn test_value_decorator_reports_error() {
    let session = TestSession::single(
        r#"
const mark = 1;

@mark
const value = 1;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const mark: 1 = 1;

@mark
const value: 1 = 1;

=== checked ===
const mark = 1;
/// @type.symbol symbol=mark source=mark type=1
/// @type.node source=1 type=1

@mark
/// @resolution.name source=mark target=mark

const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @type.node source=1 type=1
"#,
        r#"
/// @diagnostic.error id=invalid-decorator-target message="decorator must name a newtype declaration"
/// @diagnostic.label line=4 column=2 span="mark" line_source="@mark"
"#,
    );
}

#[test]
fn test_namespace_member_decorator_resolves() {
    let session = TestSession::builder()
        .module("marks.ds", "export newtype mark = ();\n")
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
        DirRows::checked()
            .with_reference_types()
            .with_decorators(),
        r#"
=== annotated ===
import * as marks from "./marks.ds";

@marks.mark
const value: 1 = 1;

=== checked ===
import * as marks from "./marks.ds";

@marks.mark
/// @decorator.node source=@marks.mark owner="const value = 1" expression=marks.mark target=marks.mark type=marks.mark kind=newtype parameters=() newtype=marks.mark backing=() value=marks.mark()
/// @type.node source=marks.mark type=marks.mark
/// @resolution.name source=marks.mark target=marks.mark

const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @type.node source=1 type=1
"#,
    );
}
