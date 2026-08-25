use crate::tests::{DirRows, TestSession};

#[test]
fn test_let_array_widens_element_literals() {
    let session = TestSession::single(
        r#"
let values = [1, 2];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types().with_coercion(),
        r#"
=== annotated ===
let values: int64[] = [1, 2];

=== dir ===
let values = [1, 2];
/// @type.symbol symbol=values source=values type=int64[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<int64> template=Array arguments=(int64)
/// @generic.instance id=MaybeUninit<int64> template=MaybeUninit arguments=(int64)
/// @generic.instance id=new<MaybeUninit<int64>> template=new arguments=(MaybeUninit<int64>)
/// @type.node source=[1, 2] type=int64[]
/// @resolution.call source=[1, 2] parameters=(&arrayFromSlice.'a readonly Slice<arrayFromSlice.T>) arguments=(rest(1, 2) as int64) return=int64[] kind=symbol target=arrayFromSlice instance=arrayFromSlice<int64>
/// @generic.instantiation id=arrayFromSlice<int64> template=arrayFromSlice arguments=(int64)
/// @generic.instance id="arrayFromSlice<int64, \"local\">" template=arrayFromSlice arguments=(int64, "local")
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 adjustments=[{ kind: widen, target: int64 }] origin=implicit
/// @type.node source=2 type=2
/// @coercion.node source=2 from=2 adjustments=[{ kind: widen, target: int64 }] origin=implicit
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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types().with_coercion(),
        r#"
=== annotated ===
const values: int64[] = [1, 2];

=== dir ===
const values = [1, 2];
/// @type.symbol symbol=values source=values type=int64[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<int64> template=Array arguments=(int64)
/// @generic.instance id=MaybeUninit<int64> template=MaybeUninit arguments=(int64)
/// @generic.instance id=new<MaybeUninit<int64>> template=new arguments=(MaybeUninit<int64>)
/// @type.node source=[1, 2] type=int64[]
/// @resolution.call source=[1, 2] parameters=(&arrayFromSlice.'a readonly Slice<arrayFromSlice.T>) arguments=(rest(1, 2) as int64) return=int64[] kind=symbol target=arrayFromSlice instance=arrayFromSlice<int64>
/// @generic.instantiation id=arrayFromSlice<int64> template=arrayFromSlice arguments=(int64)
/// @generic.instance id="arrayFromSlice<int64, \"local\">" template=arrayFromSlice arguments=(int64, "local")
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 adjustments=[{ kind: widen, target: int64 }] origin=implicit
/// @type.node source=2 type=2
/// @coercion.node source=2 from=2 adjustments=[{ kind: widen, target: int64 }] origin=implicit
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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const values: readonly [1, 2] = [1, 2] as const;
const first: 1 = values[0];

=== dir ===
const values = [1, 2] as const;
/// @type.symbol symbol=values source=values type=readonly [1, 2]
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source="[1, 2] as const" type=readonly [1, 2]
/// @type.node source=[1, 2] type=readonly [1, 2]
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const first = values[0];
/// @type.symbol symbol=first source=first type=1
/// @resolution.pattern source=first kind=binding target=first
/// @type.node source=values type=readonly [1, 2]
/// @type.node source=values[0] type=1
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=values root=values
/// @resolution.access source=values[0] root=values keys=[0]
/// @resolution.subscript source=values[0] type=1 kind=member target="receiver=readonly [1, 2], target=field(receiver=[1, 2], target=0, type=1), type=1"
/// @type.node source=0 type=0
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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const values: never[] = [];

=== dir ===
const values = [];
/// @type.symbol symbol=values source=values type=never[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<never> template=Array arguments=(never)
/// @generic.instance id=MaybeUninit<never> template=MaybeUninit arguments=(never)
/// @generic.instance id=new<MaybeUninit<never>> template=new arguments=(MaybeUninit<never>)
/// @type.node source=[] type=never[]
/// @resolution.call source=[] parameters=(&arrayFromSlice.'a readonly Slice<arrayFromSlice.T>) arguments=(rest() as never) return=never[] kind=symbol target=arrayFromSlice instance=arrayFromSlice<never>
/// @generic.instantiation id=arrayFromSlice<never> template=arrayFromSlice arguments=(never)
/// @generic.instance id="arrayFromSlice<never, \"local\">" template=arrayFromSlice arguments=(never, "local")
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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const values: int32[] = [];

=== dir ===
const values: int32[] = [];
/// @type.symbol symbol=values source=values type=int32[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=MaybeUninit<int32> template=MaybeUninit arguments=(int32)
/// @generic.instance id=new<MaybeUninit<int32>> template=new arguments=(MaybeUninit<int32>)
/// @type.node source=[] type=int32[]
/// @resolution.call source=[] parameters=(&arrayFromSlice.'a readonly Slice<arrayFromSlice.T>) arguments=(rest() as int32) return=int32[] kind=symbol target=arrayFromSlice instance=arrayFromSlice<int32>
/// @generic.instantiation id=arrayFromSlice<int32> template=arrayFromSlice arguments=(int32)
/// @generic.instance id="arrayFromSlice<int32, \"local\">" template=arrayFromSlice arguments=(int32, "local")
"#,
    );
}

#[test]
fn test_reject_an_integer_literal_beside_other_element_types() {
    let session = TestSession::single(
        r#"
let values = [1, "two", true];
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            .with_coercion(),
        r#"
=== annotated ===
let values: (string | boolean)[] = [1, "two" as string | boolean, true as string | boolean];

=== dir ===
let values = [1, "two", true];
/// @type.symbol symbol=values source=values type=string | boolean[]
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source=[1, "two", true] type=string | boolean[]
/// @resolution.call source=[1, "two", true] parameters=(&arrayFromSlice.'a readonly Slice<arrayFromSlice.T>) arguments=(rest(1, "two", true) as string | boolean) return=string | boolean[] kind=symbol target=arrayFromSlice instance="arrayFromSlice<string | boolean>"
/// @generic.instantiation id="arrayFromSlice<string | boolean>" template=arrayFromSlice arguments=(string | boolean)
/// @type.node source=1 type=1
/// @type.node source="\"two\"" type="two"
/// @coercion.node source="\"two\"" from="two" adjustments=[{ kind: union, target: string | boolean, cases: ({ source: "two", target: string, adjustments: [{ kind: widen, target: string }] }) }] origin=implicit
/// @type.node source=true type=true
/// @coercion.node source=true from=true adjustments=[{ kind: union, target: string | boolean, cases: ({ source: true, target: boolean, adjustments: [{ kind: widen, target: boolean }] }) }] origin=implicit
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '1' is not assignable to type 'string | boolean'"
/// @diagnostic.label line=2 column=15 span="1" line_source="let values = [1, \"two\", true];"
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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types().with_coercion(),
        r#"
=== annotated ===
const values: float64[] = [1, 2, 3];

=== dir ===
const values: number[] = [1, 2, 3];
/// @type.symbol symbol=values source=values type=float64[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<float64> template=Array arguments=(float64)
/// @generic.instance id=MaybeUninit<float64> template=MaybeUninit arguments=(float64)
/// @generic.instance id=new<MaybeUninit<float64>> template=new arguments=(MaybeUninit<float64>)
/// @type.node source=[1, 2, 3] type=float64[]
/// @resolution.call source=[1, 2, 3] parameters=(&arrayFromSlice.'a readonly Slice<arrayFromSlice.T>) arguments=(rest(1, 2, 3) as float64) return=float64[] kind=symbol target=arrayFromSlice instance=arrayFromSlice<float64>
/// @generic.instantiation id=arrayFromSlice<float64> template=arrayFromSlice arguments=(float64)
/// @generic.instance id="arrayFromSlice<float64, \"local\">" template=arrayFromSlice arguments=(float64, "local")
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 adjustments=[{ kind: widen, target: float64 }] origin=implicit
/// @type.node source=2 type=2
/// @coercion.node source=2 from=2 adjustments=[{ kind: widen, target: float64 }] origin=implicit
/// @type.node source=3 type=3
/// @coercion.node source=3 from=3 adjustments=[{ kind: widen, target: float64 }] origin=implicit
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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types().with_coercion(),
        r#"
=== annotated ===
const values: (1 | 2)[] = [1, 2] satisfies readonly number[];

=== dir ===
const values = [1, 2] satisfies readonly number[];
/// @type.symbol symbol=values source=values type=1 | 2[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id="Array<1 | 2>" template=Array arguments=(1 | 2)
/// @generic.instance id="MaybeUninit<1 | 2>" template=MaybeUninit arguments=(1 | 2)
/// @generic.instance id="new<MaybeUninit<1 | 2>>" template=new arguments=(MaybeUninit<1 | 2>)
/// @type.node source=[1, 2] satisfies readonly number[] type=1 | 2[]
/// @type.node source=[1, 2] type=1 | 2[]
/// @resolution.call source=[1, 2] parameters=(&arrayFromSlice.'a readonly Slice<arrayFromSlice.T>) arguments=(rest(1, 2) as 1 | 2) return=1 | 2[] kind=symbol target=arrayFromSlice instance="arrayFromSlice<1 | 2>"
/// @generic.instantiation id="arrayFromSlice<1 | 2>" template=arrayFromSlice arguments=(1 | 2)
/// @generic.instance id="arrayFromSlice<1 | 2, \"local\">" template=arrayFromSlice arguments=(1 | 2, "local")
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @generic.instance id=Array<float64> template=Array arguments=(float64)
/// @generic.instance id=MaybeUninit<float64> template=MaybeUninit arguments=(float64)
/// @generic.instance id=new<MaybeUninit<float64>> template=new arguments=(MaybeUninit<float64>)
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const values: float64[] = [1, "two"];

=== dir ===
const values: number[] = [1, "two"];
/// @type.symbol symbol=values source=values type=float64[]
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source=[1, "two"] type=float64[]
/// @resolution.call source=[1, "two"] parameters=(&arrayFromSlice.'a readonly Slice<arrayFromSlice.T>) arguments=(rest(1, "two") as float64) return=float64[] kind=symbol target=arrayFromSlice instance=arrayFromSlice<float64>
/// @generic.instantiation id=arrayFromSlice<float64> template=arrayFromSlice arguments=(float64)
/// @type.node source=1 type=1
/// @type.node source="\"two\"" type="two"
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"two\"' is not assignable to type 'float64'"
/// @diagnostic.label line=2 column=30 span="\"two\"" line_source="const values: number[] = [1, \"two\"];"
/// @diagnostic.related line=2 column=15 span="number[]" line_source="const values: number[] = [1, \"two\"];" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in element 1"
"#,
    );
}
