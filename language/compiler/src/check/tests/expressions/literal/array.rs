use crate::tests::{DirRows, TestSession};

#[test]
fn test_let_array_widens_element_literals() {
    let session = TestSession::single(
        r#"
let values = [1, 2];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            .with_coercion()
            .with_check_stats(),
        r#"
=== annotated ===
let values: float64[] = [1, 2];

=== checked ===
let values = [1, 2];
/// @type.symbol symbol=values source=values type=Array<float64>
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source=[1, 2] type=Array<float64>
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 adjustments=[{ kind: widen, target: float64 }] origin=implicit
/// @type.node source=2 type=2
/// @coercion.node source=2 from=2 adjustments=[{ kind: widen, target: float64 }] origin=implicit

/// @check.stats.solve variables=1 types=8 constraints=0 obligations=1 solutions=1 bounds=0 decisions=1
"#,
    );
}

#[test]
fn test_const_array_widens_without_const_assertion() {
    let session = TestSession::single(
        r#"
const values = [1, 2];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            .with_coercion()
            .with_check_stats(),
        r#"
=== annotated ===
const values: float64[] = [1, 2];

=== checked ===
const values = [1, 2];
/// @type.symbol symbol=values source=values type=Array<float64>
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source=[1, 2] type=Array<float64>
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 adjustments=[{ kind: widen, target: float64 }] origin=implicit
/// @type.node source=2 type=2
/// @coercion.node source=2 from=2 adjustments=[{ kind: widen, target: float64 }] origin=implicit

/// @check.stats.solve variables=1 types=8 constraints=0 obligations=1 solutions=1 bounds=0 decisions=1
"#,
    );
}

#[test]
fn test_const_asserted_array_becomes_readonly_tuple() {
    let session = TestSession::single(
        r#"
const values = [1, 2] as const;
const first = values[0];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const values: readonly [1, 2] = [1, 2] as const;
const first: 1 = values[0];

=== checked ===
const values = [1, 2] as const;
/// @type.symbol symbol=values source=values type=readonly [1, 2] reduced=[1, 2]
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source="[1, 2] as const" type=readonly [1, 2] reduced=[1, 2]
/// @type.node source=[1, 2] type=readonly [1, 2] reduced=[1, 2]
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const first = values[0];
/// @type.symbol symbol=first source=first type=1
/// @resolution.pattern source=first kind=binding target=first
/// @type.node source=values type=readonly [1, 2] reduced=[1, 2]
/// @type.node source=values[0] type=1
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @resolution.access source=values[0] root=values keys=[0]
/// @resolution.subscript source=values[0] type=1 kind=member target="receiver=[1, 2], target=field(receiver=[1, 2], target=0, type=1), type=1"
/// @type.node source=0 type=0

/// @check.stats.solve variables=2 types=11 constraints=0 obligations=2 solutions=2 bounds=0 decisions=4
"#,
    );
}

#[test]
fn test_empty_array_infers_never_elements() {
    let session = TestSession::single(
        r#"
const values = [];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const values: never[] = [];

=== checked ===
const values = [];
/// @type.symbol symbol=values source=values type=Array<never>
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source=[] type=Array<never>

/// @check.stats.solve variables=1 types=4 constraints=0 obligations=1 solutions=1 bounds=0 decisions=1
"#,
    );
}

#[test]
fn test_contextual_empty_array_uses_element_type() {
    let session = TestSession::single(
        r#"
const values: int32[] = [];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const values: int32[] = [];

=== checked ===
const values: int32[] = [];
/// @type.symbol symbol=values source=values type=Array<int32>
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source=[] type=Array<int32>

/// @check.stats.solve variables=1 types=4 constraints=0 obligations=1 solutions=1 bounds=0 decisions=1
"#,
    );
}

#[test]
fn test_mixed_array_infers_union_element_type() {
    let session = TestSession::single(
        r#"
let values = [1, "two", true];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            .with_coercion()
            .with_check_stats(),
        r#"
=== annotated ===
let values: (float64 | string | boolean)[] = [
    1 as float64 | string | boolean,
    "two" as float64 | string | boolean,
    true as float64 | string | boolean,
];

=== checked ===
let values = [1, "two", true];
/// @type.symbol symbol=values source=values type=Array<float64 | string | boolean>
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source=[1, "two", true] type=Array<float64 | string | boolean>
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 adjustments=[{ kind: union, target: float64 | string | boolean, cases: ({ source: 1, target: float64, adjustments: [{ kind: widen, target: float64 }] }) }] origin=implicit
/// @type.node source="\"two\"" type="two"
/// @coercion.node source="\"two\"" from="two" adjustments=[{ kind: union, target: float64 | string | boolean, cases: ({ source: "two", target: string }) }] origin=implicit
/// @type.node source=true type=true
/// @coercion.node source=true from=true adjustments=[{ kind: union, target: float64 | string | boolean, cases: ({ source: true, target: boolean }) }] origin=implicit

/// @check.stats.solve variables=1 types=12 constraints=0 obligations=1 solutions=1 bounds=0 decisions=1
"#,
    );
}

#[test]
fn test_contextual_array_literal_uses_element_type() {
    let session = TestSession::single(
        r#"
const values: number[] = [1, 2, 3];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            .with_coercion()
            .with_check_stats(),
        r#"
=== annotated ===
const values: float64[] = [1, 2, 3];

=== checked ===
const values: number[] = [1, 2, 3];
/// @type.symbol symbol=values source=values type=Array<float64>
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source=[1, 2, 3] type=Array<float64>
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 adjustments=[{ kind: widen, target: float64 }] origin=implicit
/// @type.node source=2 type=2
/// @coercion.node source=2 from=2 adjustments=[{ kind: widen, target: float64 }] origin=implicit
/// @type.node source=3 type=3
/// @coercion.node source=3 from=3 adjustments=[{ kind: widen, target: float64 }] origin=implicit

/// @check.stats.solve variables=1 types=7 constraints=0 obligations=1 solutions=1 bounds=0 decisions=1
"#,
    );
}

#[test]
fn test_satisfies_contextualizes_array_elements() {
    let session = TestSession::single(
        r#"
const values = [1, 2] satisfies readonly number[];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            .with_coercion()
            .with_check_stats(),
        r#"
=== annotated ===
const values: (1 | 2)[] = [1, 2] satisfies readonly number[];

=== checked ===
const values = [1, 2] satisfies readonly number[];
/// @type.symbol symbol=values source=values type=Array<1 | 2>
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source=[1, 2] satisfies readonly number[] type=Array<1 | 2>
/// @type.node source=[1, 2] type=Array<1 | 2>
/// @type.node source=1 type=1
/// @type.node source=2 type=2

/// @check.stats.solve variables=1 types=9 constraints=0 obligations=1 solutions=1 bounds=0 decisions=1
"#,
    );
}

#[test]
fn test_contextual_array_literal_rejects_element_mismatch() {
    let session = TestSession::single(
        r#"
const values: number[] = [1, "two"];
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const values: float64[] = [1, "two"];

=== checked ===
const values: number[] = [1, "two"];
/// @type.symbol symbol=values source=values type=Array<float64>
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source=[1, "two"] type=Array<float64>
/// @type.node source=1 type=1
/// @type.node source="\"two\"" type="two"

/// @check.stats.solve variables=1 types=6 constraints=0 obligations=1 solutions=1 bounds=0 decisions=1
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"two\"' is not assignable to type 'float64'"
/// @diagnostic.label line=2 column=30 span="\"two\"" line_source="const values: number[] = [1, \"two\"];"
/// @diagnostic.related line=2 column=15 span="number[]" line_source="const values: number[] = [1, \"two\"];" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in element 1"
"#,
    );
}
