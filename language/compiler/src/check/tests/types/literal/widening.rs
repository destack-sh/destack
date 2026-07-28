use crate::tests::{DirRows, TestSession};

#[test]
fn test_let_array_widens_element_literals() {
    let session = TestSession::single(
        r#"
let values = [1, 2];
const first = values[0];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
let values: float64[] = [1, 2];
const first: float64 = values[0];

=== checked ===
let values = [1, 2];
/// @type.symbol symbol=values source=values type=Array<float64>
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source=[1, 2] type=Array<float64>
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const first = values[0];
/// @type.symbol symbol=first source=first type=float64
/// @resolution.pattern source=first kind=binding target=first
/// @type.node source=values type=Array<float64>
/// @type.node source=values[0] type=float64
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @resolution.access source=values[0] root=values keys=[0]
/// @resolution.subscript source=values[0] type=float64 kind=call target="collections.array.index#3(parameters=(usize), arguments=(provided(0) as usize), return=memory.type.WithAccess<&'static float64, \"exclusive\">)"
/// @generic.instance source=values[0] id="Array<float64>.<extension#6>.index#3<\"exclusive\">"
/// @type.node source=0 type=0

/// @generic.instance id="Array<float64>.<extension#6>.index#3<\"exclusive\">" template=collections.array.index#3 arguments=(float64, "exclusive")

/// @check.stats.solve variables=13 types=74 constraints=4 obligations=2 solutions=13 bounds=9 decisions=4
"#,
    );
}

#[test]
fn test_contextual_array_preserves_union_element_type() {
    let session = TestSession::single(
        r#"
const values: (1 | 2)[] = [1, 2];
const first = values[0];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const values: (1 | 2)[] = [1 as 1 | 2, 2 as 1 | 2];
const first: 1 | 2 = values[0];

=== checked ===
const values: (1 | 2)[] = [1, 2];
/// @type.symbol symbol=values source=values type=Array<1 | 2>
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source=[1, 2] type=Array<1 | 2>
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const first = values[0];
/// @type.symbol symbol=first source=first type=1 | 2
/// @resolution.pattern source=first kind=binding target=first
/// @type.node source=values type=Array<1 | 2>
/// @type.node source=values[0] type=1 | 2
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @resolution.access source=values[0] root=values keys=[0]
/// @resolution.subscript source=values[0] type=1 | 2 kind=call target="collections.array.index#3(parameters=(usize), arguments=(provided(0) as usize), return=memory.type.WithAccess<&'static 1 | 2, \"exclusive\">)"
/// @generic.instance source=values[0] id="Array<1 | 2>.<extension#6>.index#3<\"exclusive\">"
/// @type.node source=0 type=0

/// @generic.instance id="Array<1 | 2>.<extension#6>.index#3<\"exclusive\">" template=collections.array.index#3 arguments=(1 | 2, "exclusive")

/// @check.stats.solve variables=13 types=72 constraints=4 obligations=2 solutions=13 bounds=9 decisions=4
"#,
    );
}

#[test]
fn test_contextual_literal_requires_coercion_for_mixed_union() {
    let session = TestSession::single(
        r#"
const value: number | boolean = 1;
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
const value: float64 | boolean = 1 as float64 | boolean;

=== checked ===
const value: number | boolean = 1;
/// @type.symbol symbol=value source=value type=float64 | boolean
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 adjustments=[{ kind: union, target: float64 | boolean, cases: ({ source: 1, target: float64, adjustments: [{ kind: widen, target: float64 }] }) }] origin=implicit

/// @check.stats.solve variables=1 types=6 constraints=0 obligations=1 solutions=1 bounds=0 decisions=1
"#,
    );
}

#[test]
fn test_union_coercion_records_case_adjustments() {
    let session = TestSession::single(
        r#"
newtype Flag = boolean;

function widen(value: 1 | Flag): int32 | Flag {
    return value;
}
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
newtype Flag = boolean;

function widen(value: 1 | Flag): int32 | Flag {
    return value as int32 | Flag;
}

=== checked ===
newtype Flag = boolean;
/// @type.symbol symbol=Flag source="newtype Flag = boolean" type=Flag
/// @definition.newtype symbol=Flag source="newtype Flag = boolean" backing=boolean

function widen(value: 1 | Flag): int32 | Flag {
/// @type.symbol symbol=widen type=(1 | Flag) => int32 | Flag
/// @type.symbol symbol=widen.value source="value: 1 | Flag" type=1 | Flag
/// @resolution.name source=Flag target=Flag
/// @resolution.name source=Flag target=Flag

    return value;
    /// @type.node source=value type=1 | Flag
    /// @resolution.name source=value target=widen.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=widen.value
    /// @coercion.node source=value from=1 | Flag adjustments=[{ kind: union, target: int32 | Flag, cases: ({ source: 1, target: int32, adjustments: [{ kind: widen, target: int32 }] }, { source: Flag, target: Flag }) }] origin=implicit

}

/// @check.stats.solve variables=0 types=13 constraints=0 obligations=0 solutions=0 bounds=0 decisions=3
"#,
    );
}

#[test]
fn test_union_members_coerce_to_common_target() {
    let session = TestSession::single(
        r#"
function widen(value: 1 | 2): int32 {
    return value;
}
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
function widen(value: 1 | 2): int32 {
    return value as int32;
}

=== checked ===
function widen(value: 1 | 2): int32 {
/// @type.symbol symbol=widen type=(1 | 2) => int32
/// @type.symbol symbol=widen.value source="value: 1 | 2" type=1 | 2

    return value;
    /// @type.node source=value type=1 | 2
    /// @resolution.name source=value target=widen.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=widen.value
    /// @coercion.node source=value from=1 | 2 adjustments=[{ kind: union, target: int32, cases: ({ source: 1, target: int32, adjustments: [{ kind: widen, target: int32 }] }, { source: 2, target: int32, adjustments: [{ kind: widen, target: int32 }] }) }] origin=implicit

}

/// @check.stats.solve variables=0 types=10 constraints=0 obligations=0 solutions=0 bounds=0 decisions=1
"#,
    );
}

#[test]
fn test_const_conditional_preserves_literal_union() {
    let session = TestSession::single(
        r#"
const value = true ? 1 : 2;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: 1 | 2 = true ? 1 : 2;

=== checked ===
const value = true ? 1 : 2;
/// @type.symbol symbol=value source=value type=1 | 2
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="true ? 1 : 2" type=1 | 2
/// @type.node source=true type=true
/// @type.node source=1 type=1
/// @type.node source=2 type=2

/// @check.stats.solve variables=1 types=7 constraints=0 obligations=1 solutions=1 bounds=0 decisions=1
"#,
        r#"
/// @diagnostic.warning id=constant-condition message="condition is always true"
/// @diagnostic.label line=2 column=15 span="true" line_source="const value = true ? 1 : 2;"
"#,
    );
}

#[test]
fn test_let_conditional_widens_literal_union() {
    let session = TestSession::single(
        r#"
let value = true ? 1 : 2;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
let value: float64 = true ? 1 : 2;

=== checked ===
let value = true ? 1 : 2;
/// @type.symbol symbol=value source=value type=float64
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="true ? 1 : 2" type=1 | 2
/// @type.node source=true type=true
/// @type.node source=1 type=1
/// @type.node source=2 type=2

/// @check.stats.solve variables=1 types=8 constraints=0 obligations=1 solutions=1 bounds=0 decisions=1
"#,
        r#"
/// @diagnostic.warning id=constant-condition message="condition is always true"
/// @diagnostic.label line=2 column=13 span="true" line_source="let value = true ? 1 : 2;"
"#,
    );
}
