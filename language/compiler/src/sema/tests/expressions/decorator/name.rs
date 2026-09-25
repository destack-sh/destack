use crate::tests::{DirRows, TestSession};

#[test]
fn test_unresolved_decorator_reports_error() {
    let session = TestSession::single(
        r#"
@missing
const value = 1;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
@missing
const value: 1 = 1;

=== dir ===
@missing
/// @resolution.unresolved source=missing path=missing

const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @resolution.pattern source=value kind=binding target=value
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
        .module("first.tspp", "export const mark = 1;\n")
        .module("second.tspp", "export const mark = 1;\n")
        .module(
            "main.tspp",
            r#"
import { mark } from "./first.tspp";
import { mark } from "./second.tspp";

@mark
const value = 1;
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { mark } from "./first.tspp";
import { mark } from "./second.tspp";

@mark
const value: 1 = 1;

=== dir ===
import { mark } from "./first.tspp";
import { mark } from "./second.tspp";

@mark
const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1
"#,
        r#"
/// @diagnostic.error id=ambiguous-reference message="ambiguous reference 'mark'"
/// @diagnostic.label line=5 column=2 span="mark" line_source="@mark"
/// @diagnostic.related file="first.tspp" line=1 column=14 span="mark" line_source="export const mark = 1;" message="one candidate is declared here"
/// @diagnostic.related file="second.tspp" line=1 column=14 span="mark" line_source="export const mark = 1;" message="one candidate is declared here"
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const mark: 1 = 1;

@mark
const value: 1 = 1;

=== dir ===
const mark = 1;
/// @type.symbol symbol=mark source=mark type=1
/// @resolution.pattern source=mark kind=binding target=mark
/// @type.node source=1 type=1

@mark
/// @resolution.name source=mark target=mark

const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @resolution.pattern source=value kind=binding target=value
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
        .module("marks.tspp", "export newtype mark = ();\n")
        .module(
            "main.tspp",
            r#"
import * as marks from "./marks.tspp";

@marks.mark
const value = 1;
"#,
        )
        .build();

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types().with_decorators(),
        r#"
=== annotated ===
import * as marks from "./marks.tspp";

@marks.mark
const value: 1 = 1;

=== dir ===
import * as marks from "./marks.tspp";

@marks.mark
/// @decorator.node source=@marks.mark owner="const value = 1" expression=marks.mark target=marks.mark type=marks.mark kind=newtype parameters=() newtype=marks.mark backing=() value=marks.mark()
/// @type.node source=marks.mark type=marks.mark
/// @resolution.name source=marks.mark target=marks.mark

const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1
"#,
    );
}
