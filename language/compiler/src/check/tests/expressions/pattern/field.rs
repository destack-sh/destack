use crate::tests::{DirRows, TestSession};

#[test]
fn test_object_pattern_reports_missing_field() {
    let session = TestSession::single(
        r#"
declare const point: { x: int32 };

let { y } = point;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const point: { x: int32 };

let { y } = point;

=== checked ===
declare const point: { x: int32 };
/// @type.symbol symbol=point source=point type={ x: int32 }

let { y } = point;
/// @type.symbol symbol=y source=y type=<error>
/// @resolution.pattern source="{ y }" kind=object fields={ y }
/// @resolution.pattern source=y kind=binding target=y
/// @type.node source=point type={ x: int32 }
/// @resolution.name source=point target=point
"#,
        r#"
/// @diagnostic.error code=EC426 message="pattern field 'y' does not exist on type '{ x: int32 }'"
/// @diagnostic.label line=4 column=7 source=y
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const point: { x: int32 };

let { x, x: other } = point;

=== checked ===
declare const point: { x: int32 };
/// @type.symbol symbol=point source=point type={ x: int32 }

let { x, x: other } = point;
/// @type.symbol symbol=x source=x type=int32
/// @type.symbol symbol=other source=other type=int32
/// @resolution.pattern source="{ x, x: other }" kind=object fields={ x, x: other }
/// @resolution.pattern source=x kind=binding target=x
/// @resolution.pattern source=other kind=binding target=other
/// @type.node source=point type={ x: int32 }
/// @resolution.name source=point target=point
"#,
        r#"
/// @diagnostic.error code=EC428 message="field 'x' appears more than once in pattern"
/// @diagnostic.label line=4 column=10 source=x
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const pair: { left: int32; right: int32 };

let { left: value, right: value } = pair;

=== checked ===
declare const pair: { left: int32; right: int32 };
/// @type.symbol symbol=pair source=pair type={ left: int32; right: int32 }

let { left: value, right: value } = pair;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source="{ left: value, right: value }" kind=object fields={ left: value, right: value }
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=pair type={ left: int32; right: int32 }
/// @resolution.name source=pair target=pair
"#,
        r#"
/// @diagnostic.error code=EC429 message="binding 'value' appears more than once in pattern"
/// @diagnostic.label line=4 column=27 source=value
"#,
    );
}
