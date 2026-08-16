use crate::tests::{DirRows, TestSession};

#[test]
fn test_step_literal_range_endpoints_in_the_integer_family() {
    let session = TestSession::single(
        r#"
const counted = 0..10;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const counted: Range<int64> = 0..10;

=== dir ===
const counted = 0..10;
/// @type.symbol symbol=counted source=counted type=Range<int64>
/// @resolution.pattern source=counted kind=binding target=counted
/// @type.node source=0 type=0
/// @type.node source=0..10 type=Range<int64>
/// @type.node source=10 type=10
"#,
    );
}

#[test]
fn test_iterate_the_integer_elements_of_a_literal_range() {
    let session = TestSession::single(
        r#"
for (const value of 0..10) {
    value satisfies int64;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
for (const value of 0..10) {
    value satisfies int64;
}

=== dir ===
for (const value of 0..10) {
/// @type.symbol symbol=value source=value type=int64
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=0 type=0
/// @type.node source=0..10 type=Range<int64>
/// @type.node source=10 type=10

    value satisfies int64;
    /// @type.node source="value satisfies int64" type=int64
    /// @type.node source=value type=int64
    /// @resolution.name source=value target=value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=value

}
"#,
    );
}

#[test]
fn test_iterate_the_integer_elements_of_a_written_scalar_range() {
    let session = TestSession::single(
        r#"
declare const limit: int32;

for (const value of 0..limit) {
    value satisfies int32;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const limit: int32;

for (const value of 0..limit) {
    value satisfies int32;
}

=== dir ===
declare const limit: int32;
/// @type.symbol symbol=limit source=limit type=int32
/// @resolution.pattern source=limit kind=binding target=limit

for (const value of 0..limit) {
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=0 type=0
/// @type.node source=0..limit type=Range<int32>
/// @type.node source=limit type=int32
/// @resolution.name source=limit target=limit
/// @resolution.place source=limit placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=limit root=limit

    value satisfies int32;
    /// @type.node source="value satisfies int32" type=int32
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=value

}
"#,
    );
}

#[test]
fn test_adopt_the_written_scalar_at_a_literal_range_endpoint() {
    let session = TestSession::single(
        r#"
declare const limit: int32;

const counted = 0..limit;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const limit: int32;

const counted: Range<int32> = 0..limit;

=== dir ===
declare const limit: int32;
/// @type.symbol symbol=limit source=limit type=int32
/// @resolution.pattern source=limit kind=binding target=limit

const counted = 0..limit;
/// @type.symbol symbol=counted source=counted type=Range<int32>
/// @resolution.pattern source=counted kind=binding target=counted
/// @type.node source=0 type=0
/// @type.node source=0..limit type=Range<int32>
/// @type.node source=limit type=int32
/// @resolution.name source=limit target=limit
/// @resolution.place source=limit placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=limit root=limit
"#,
    );
}

#[test]
fn test_reject_range_endpoints_of_different_scalars() {
    let session = TestSession::single(
        r#"
declare const low: float64;
declare const high: int32;

const counted = low..high;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const low: float64;
declare const high: int32;

const counted: Range<float64 | int32> = low..high;

=== dir ===
declare const low: float64;
/// @type.symbol symbol=low source=low type=float64
/// @resolution.pattern source=low kind=binding target=low

declare const high: int32;
/// @type.symbol symbol=high source=high type=int32
/// @resolution.pattern source=high kind=binding target=high

const counted = low..high;
/// @type.symbol symbol=counted source=counted type=Range<float64 | int32>
/// @resolution.pattern source=counted kind=binding target=counted
/// @type.node source=low type=float64
/// @type.node source=low..high type=Range<float64 | int32>
/// @resolution.name source=low target=low
/// @resolution.place source=low placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=low root=low
/// @type.node source=high type=int32
/// @resolution.name source=high target=high
/// @resolution.place source=high placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=high root=high
"#,
        r#"
/// @diagnostic.error id=incompatible-range-endpoints message="range endpoints do not share one type: 'float64 | int32'"
/// @diagnostic.label line=5 column=20 span=".." line_source="const counted = low..high;"
"#,
    );
}
