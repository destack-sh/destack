use crate::tests::{DirRows, TestSession};

#[test]
fn test_let_array_widens_element_literals() {
    let session = TestSession::single(
        r#"
let values = [1, 2];
const first = values[0];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let values: float64[] = [1, 2];
const first: float64 = values[0];

=== dir ===
let values = [1, 2];
/// @type.symbol symbol=values source=values type=float64[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<float64> template=collections.array.Array arguments=(float64)
/// @generic.instance id=collections.slice.new<memory.init.MaybeUninit<float64>> template=collections.slice.new arguments=(memory.init.MaybeUninit<float64>)
/// @generic.instance id=memory.init.MaybeUninit<float64> template=memory.init.MaybeUninit arguments=(float64)
/// @type.node source=[1, 2] type=float64[]
/// @resolution.call source=[1, 2] parameters=(&collections.array.arrayFromSlice.'a readonly Slice<collections.array.arrayFromSlice.T>) arguments=(rest(1, 2) as float64) return=float64[] kind=symbol target=collections.array.arrayFromSlice instance=collections.array.arrayFromSlice<float64>
/// @generic.instantiation id=collections.array.arrayFromSlice<float64> template=collections.array.arrayFromSlice arguments=(float64)
/// @generic.instance id=collections.array.arrayFromSlice<float64> template=collections.array.arrayFromSlice arguments=(float64)
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const first = values[0];
/// @type.symbol symbol=first source=first type=float64
/// @resolution.pattern source=first kind=binding target=first
/// @type.node source=values type=float64[]
/// @type.node source=values[0] type=float64
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @resolution.access source=values[0] root=values keys=[0]
/// @resolution.subscript source=values[0] type=float64 kind=call target="collections.array.index#1(parameters=(isize), arguments=(provided(0) as isize), return=memory.type.WithAccess<&'static float64, \"exclusive\">)"
/// @generic.instantiation id="collections.array.index#1<float64, \"exclusive\">" template=collections.array.index#1 arguments=(float64, "exclusive")
/// @generic.instance id="collections.array.index#1<float64, \"exclusive\">" template=collections.array.index#1 arguments=(float64, "exclusive")
/// @generic.instance id="memory.type.WithAccess<&'frame float64, \"exclusive\">" template=memory.type.WithAccess arguments=(&'frame float64, "exclusive")
/// @generic.instance id="memory.type.WithAccess<&'frame float64[], \"exclusive\">" template=memory.type.WithAccess arguments=(&'frame float64[], "exclusive")
/// @type.node source=0 type=0
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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const values: (1 | 2)[] = [1 as 1 | 2, 2 as 1 | 2];
const first: 1 | 2 = values[0];

=== dir ===
const values: (1 | 2)[] = [1, 2];
/// @type.symbol symbol=values source=values type=1 | 2[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id="Array<1 | 2>" template=collections.array.Array arguments=(1 | 2)
/// @generic.instance id="collections.slice.new<memory.init.MaybeUninit<1 | 2>>" template=collections.slice.new arguments=(memory.init.MaybeUninit<1 | 2>)
/// @generic.instance id="memory.init.MaybeUninit<1 | 2>" template=memory.init.MaybeUninit arguments=(1 | 2)
/// @type.node source=[1, 2] type=1 | 2[]
/// @resolution.call source=[1, 2] parameters=(&collections.array.arrayFromSlice.'a readonly Slice<collections.array.arrayFromSlice.T>) arguments=(rest(1, 2) as 1 | 2) return=1 | 2[] kind=symbol target=collections.array.arrayFromSlice instance="collections.array.arrayFromSlice<1 | 2>"
/// @generic.instantiation id="collections.array.arrayFromSlice<1 | 2>" template=collections.array.arrayFromSlice arguments=(1 | 2)
/// @generic.instance id="collections.array.arrayFromSlice<1 | 2>" template=collections.array.arrayFromSlice arguments=(1 | 2)
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const first = values[0];
/// @type.symbol symbol=first source=first type=1 | 2
/// @resolution.pattern source=first kind=binding target=first
/// @type.node source=values type=1 | 2[]
/// @type.node source=values[0] type=1 | 2
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @resolution.access source=values[0] root=values keys=[0]
/// @resolution.subscript source=values[0] type=1 | 2 kind=call target="collections.array.index#1(parameters=(isize), arguments=(provided(0) as isize), return=memory.type.WithAccess<&'static 1 | 2, \"exclusive\">)"
/// @generic.instantiation id="collections.array.index#1<1 | 2, \"exclusive\">" template=collections.array.index#1 arguments=(1 | 2, "exclusive")
/// @generic.instance id="collections.array.index#1<1 | 2, \"exclusive\">" template=collections.array.index#1 arguments=(1 | 2, "exclusive")
/// @generic.instance id="memory.type.WithAccess<&'frame 1 | 2, \"exclusive\">" template=memory.type.WithAccess arguments=(&'frame 1 | 2, "exclusive")
/// @generic.instance id="memory.type.WithAccess<&'frame 1 | 2[], \"exclusive\">" template=memory.type.WithAccess arguments=(&'frame 1 | 2[], "exclusive")
/// @type.node source=0 type=0
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

    session.assert_dir(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            .with_coercion()
            ,
        r#"
=== annotated ===
const value: float64 | boolean = 1 as float64 | boolean;

=== dir ===
const value: number | boolean = 1;
/// @type.symbol symbol=value source=value type=float64 | boolean
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 adjustments=[{ kind: union, target: float64 | boolean, cases: ({ source: 1, target: float64, adjustments: [{ kind: widen, target: float64 }] }) }] origin=implicit
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

    session.assert_dir(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            .with_coercion()
            ,
        r#"
=== annotated ===
newtype Flag = boolean;

function widen(value: 1 | Flag): int32 | Flag {
    return value as int32 | Flag;
}

=== dir ===
newtype Flag = boolean;
/// @type.symbol symbol=Flag source="newtype Flag = boolean" type=Flag
/// @definition.newtype symbol=Flag source="newtype Flag = boolean" backing=boolean constructors=[(boolean) => Flag]

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

    session.assert_dir(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            .with_coercion()
            ,
        r#"
=== annotated ===
function widen(value: 1 | 2): int32 {
    return value as int32;
}

=== dir ===
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: 1 | 2 = true ? 1 : 2;

=== dir ===
const value = true ? 1 : 2;
/// @type.symbol symbol=value source=value type=1 | 2
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="true ? 1 : 2" type=1 | 2
/// @type.node source=true type=true
/// @type.node source=1 type=1
/// @type.node source=2 type=2
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let value: float64 = true ? 1 : 2;

=== dir ===
let value = true ? 1 : 2;
/// @type.symbol symbol=value source=value type=float64
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="true ? 1 : 2" type=1 | 2
/// @type.node source=true type=true
/// @type.node source=1 type=1
/// @type.node source=2 type=2
"#,
        r#"
/// @diagnostic.warning id=constant-condition message="condition is always true"
/// @diagnostic.label line=2 column=13 span="true" line_source="let value = true ? 1 : 2;"
"#,
    );
}
