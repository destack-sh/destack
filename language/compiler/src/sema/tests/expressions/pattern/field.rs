use crate::tests::{DirRows, TestSession};

#[test]
fn test_object_pattern_reports_missing_field() {
    let session = TestSession::single(
        r#"
declare const point: { x: int32 };

let { y } = point;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const point: { x: int32 };

let { y } = point;

=== dir ===
declare const point: { x: int32 };
/// @type.symbol symbol=point source=point type={ x: int32 }
/// @resolution.pattern source=point kind=binding target=point
/// @type.symbol symbol=x source="x: int32" type=int32

let { y } = point;
/// @resolution.pattern source={ y } kind=object fields={}
/// @type.symbol symbol=y source=y type=<error>
/// @type.node source=point type={ x: int32 }
/// @resolution.name source=point target=point
/// @resolution.access source=point root=point
"#,
        r#"
/// @diagnostic.error id=pattern-field-missing message="pattern field 'y' does not exist on type '{ x: int32 }'"
/// @diagnostic.label line=4 column=7 span="y" line_source="let { y } = point;"
"#,
    );
}

#[test]
fn test_object_pattern_reports_duplicate_field() {
    let session = TestSession::single(
        r#"
declare const point: { x: int32 };

let { x, x: other } = point;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const point: { x: int32 };

let { x, x: other } = point;

=== dir ===
declare const point: { x: int32 };
/// @type.symbol symbol=point source=point type={ x: int32 }
/// @resolution.pattern source=point kind=binding target=point
/// @type.symbol symbol=x#1 source="x: int32" type=int32

let { x, x: other } = point;
/// @resolution.pattern source={ x, x: other } kind=object fields={ x, x: other }
/// @type.symbol symbol=x#2 source=x type=int32
/// @type.symbol symbol=other source=other type=int32
/// @resolution.pattern source=other kind=binding target=other
/// @type.node source=point type={ x: int32 }
/// @resolution.name source=point target=point
/// @resolution.access source=point root=point
"#,
        r#"
/// @diagnostic.error id=duplicate-pattern-field message="field 'x' appears more than once in pattern"
/// @diagnostic.label line=4 column=10 span="x" line_source="let { x, x: other } = point;"
/// @diagnostic.related line=4 column=7 span="x" line_source="let { x, x: other } = point;" message="first matched here"
"#,
    );
}

#[test]
fn test_object_pattern_reports_duplicate_binding_name() {
    let session = TestSession::single(
        r#"
declare const pair: { left: int32; right: int32 };

let { left: value, right: value } = pair;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const pair: { left: int32; right: int32 };

let { left: value, right: value } = pair;

=== dir ===
declare const pair: { left: int32; right: int32 };
/// @type.symbol symbol=pair source=pair type={ left: int32; right: int32 }
/// @resolution.pattern source=pair kind=binding target=pair
/// @type.symbol symbol=left source="left: int32" type=int32
/// @type.symbol symbol=right source="right: int32" type=int32

let { left: value, right: value } = pair;
/// @resolution.pattern source={ left: value, right: value } kind=object fields={ left: value#1, right: value#2 }
/// @type.symbol symbol=value#1 source=value type=int32
/// @resolution.pattern source=value kind=binding target=value#1
/// @type.symbol symbol=value#2 source=value type=int32
/// @resolution.pattern source=value kind=binding target=value#2
/// @type.node source=pair type={ left: int32; right: int32 }
/// @resolution.name source=pair target=pair
/// @resolution.access source=pair root=pair
"#,
        r#"
/// @diagnostic.error id=duplicate-pattern-binding message="binding 'value' appears more than once in pattern"
/// @diagnostic.label line=4 column=27 span="value" line_source="let { left: value, right: value } = pair;"
"#,
    );
}
