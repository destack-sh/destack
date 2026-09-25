use crate::tests::{DirRows, TestSession};

#[test]
fn test_let_array_widens_element_literals() {
    let session = TestSession::single(
        r#"
let values = [1, 2];
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types().with_coercion(),
        r#"
=== annotated ===
let values: int64[] = [1, 2];

=== dir ===
let values = [1, 2];
/// @type.symbol symbol=values source=values type=int64[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<int64> template=Array arguments=(int64)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int64>> template=sliceAssumeInit arguments=(MaybeUninit<int64>)
/// @generic.instance id=sliceUninit<MaybeUninit<int64>> template=sliceUninit arguments=(MaybeUninit<int64>)
/// @type.node source=[1, 2] type=int64[]
/// @resolution.call source=[1, 2] parameters=(^Slice<int64>) arguments=(rest(provided(1) as int64, provided(2) as int64) as int64) return=int64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int64>
/// @generic.instantiation id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
/// @generic.instance id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 adjustments=[{ kind: materialize, target: int64 }] origin=implicit
/// @type.node source=2 type=2
/// @coercion.node source=2 from=2 adjustments=[{ kind: materialize, target: int64 }] origin=implicit
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
        "main.tspp",
        DirRows::checked().with_reference_types().with_coercion(),
        r#"
=== annotated ===
const values: int64[] = [1, 2];

=== dir ===
const values = [1, 2];
/// @type.symbol symbol=values source=values type=int64[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<int64> template=Array arguments=(int64)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int64>> template=sliceAssumeInit arguments=(MaybeUninit<int64>)
/// @generic.instance id=sliceUninit<MaybeUninit<int64>> template=sliceUninit arguments=(MaybeUninit<int64>)
/// @type.node source=[1, 2] type=int64[]
/// @resolution.call source=[1, 2] parameters=(^Slice<int64>) arguments=(rest(provided(1) as int64, provided(2) as int64) as int64) return=int64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int64>
/// @generic.instantiation id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
/// @generic.instance id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 adjustments=[{ kind: materialize, target: int64 }] origin=implicit
/// @type.node source=2 type=2
/// @coercion.node source=2 from=2 adjustments=[{ kind: materialize, target: int64 }] origin=implicit
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
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const values: readonly (1 | 2)[] = [1, 2] as const;
const first: 1 | 2 = values[0];

=== dir ===
const values = [1, 2] as const;
/// @type.symbol symbol=values source=values type=readonly 1 | 2[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id="Array<1 | 2>" template=Array arguments=(1 | 2)
/// @generic.instance id="sliceAssumeInit<MaybeUninit<1 | 2>>" template=sliceAssumeInit arguments=(MaybeUninit<1 | 2>)
/// @generic.instance id="sliceUninit<MaybeUninit<1 | 2>>" template=sliceUninit arguments=(MaybeUninit<1 | 2>)
/// @type.node source="[1, 2] as const" type=readonly 1 | 2[]
/// @type.node source=[1, 2] type=readonly 1 | 2[]
/// @resolution.call source=[1, 2] parameters=(^Slice<1 | 2>) arguments=(rest(provided(1) as 1 | 2, provided(2) as 1 | 2) as 1 | 2) return=1 | 2[] kind=symbol target=arrayFromOwnedSlice instance="arrayFromOwnedSlice<1 | 2>"
/// @generic.instantiation id="arrayFromOwnedSlice<1 | 2>" template=arrayFromOwnedSlice arguments=(1 | 2)
/// @generic.instance id="arrayFromOwnedSlice<1 | 2>" template=arrayFromOwnedSlice arguments=(1 | 2)
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const first = values[0];
/// @type.symbol symbol=first source=first type=1 | 2
/// @resolution.pattern source=first kind=binding target=first
/// @type.node source=values type=readonly 1 | 2[]
/// @type.node source=values[0] type=1 | 2
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
/// @resolution.subscript source=values[0] type=1 | 2 kind=call target="index#2(parameters=(isize), arguments=(provided(0) as isize), return=1 | 2, regions=(\"managed\" & \"local\"))"
/// @generic.instantiation id="index#2<1 | 2, \"managed\" & \"local\">" template=index#2 arguments=(1 | 2, "managed" & "local")
/// @generic.instance id="index#2<1 | 2, \"bound0\" & \"local\">" template=index#2 arguments=(1 | 2, "bound0" & "local")
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
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const values: never[] = [];

=== dir ===
const values = [];
/// @type.symbol symbol=values source=values type=never[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<never> template=Array arguments=(never)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<never>> template=sliceAssumeInit arguments=(MaybeUninit<never>)
/// @generic.instance id=sliceUninit<MaybeUninit<never>> template=sliceUninit arguments=(MaybeUninit<never>)
/// @type.node source=[] type=never[]
/// @resolution.call source=[] parameters=(^Slice<never>) arguments=(rest() as never) return=never[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<never>
/// @generic.instantiation id=arrayFromOwnedSlice<never> template=arrayFromOwnedSlice arguments=(never)
/// @generic.instance id=arrayFromOwnedSlice<never> template=arrayFromOwnedSlice arguments=(never)
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
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const values: int32[] = [];

=== dir ===
const values: int32[] = [];
/// @type.symbol symbol=values source=values type=int32[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @type.node source=[] type=int32[]
/// @resolution.call source=[] parameters=(^Slice<int32>) arguments=(rest() as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
/// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.instance id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
"#,
    );
}

#[test]
fn test_join_mixed_element_families_into_a_union() {
    let session = TestSession::single(
        r#"
let values = [1, "two", true];
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types().with_coercion(),
        r#"
=== annotated ===
let values: (string | int64 | boolean)[] = [
    1 as string | int64 | boolean,
    "two" as string | int64 | boolean,
    true as string | int64 | boolean,
];

=== dir ===
let values = [1, "two", true];
/// @type.symbol symbol=values source=values type=string | int64 | boolean[]
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source=[1, "two", true] type=string | int64 | boolean[]
/// @resolution.call source=[1, "two", true] parameters=(^Slice<string | int64 | boolean>) arguments=(rest(provided(1) as string | int64 | boolean, provided("two") as string | int64 | boolean, provided(true) as string | int64 | boolean) as string | int64 | boolean) return=string | int64 | boolean[] kind=symbol target=arrayFromOwnedSlice instance="arrayFromOwnedSlice<string | int64 | boolean>"
/// @generic.instantiation id="arrayFromOwnedSlice<string | int64 | boolean>" template=arrayFromOwnedSlice arguments=(string | int64 | boolean)
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 adjustments=[{ kind: union, target: string | int64 | boolean, cases: ({ source: 1, target: int64, adjustments: [{ kind: materialize, target: int64 }] }) }] origin=implicit
/// @type.node source="\"two\"" type="two"
/// @coercion.node source="\"two\"" from="two" adjustments=[{ kind: union, target: string | int64 | boolean, cases: ({ source: "two", target: string, adjustments: [{ kind: materialize, target: string }] }) }] origin=implicit
/// @type.node source=true type=true
/// @coercion.node source=true from=true adjustments=[{ kind: union, target: string | int64 | boolean, cases: ({ source: true, target: boolean, adjustments: [{ kind: materialize, target: boolean }] }) }] origin=implicit
"#,
        r#"

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
        "main.tspp",
        DirRows::checked().with_reference_types().with_coercion(),
        r#"
=== annotated ===
const values: float64[] = [1, 2, 3];

=== dir ===
const values: number[] = [1, 2, 3];
/// @type.symbol symbol=values source=values type=float64[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<float64> template=Array arguments=(float64)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<float64>> template=sliceAssumeInit arguments=(MaybeUninit<float64>)
/// @generic.instance id=sliceUninit<MaybeUninit<float64>> template=sliceUninit arguments=(MaybeUninit<float64>)
/// @type.node source=[1, 2, 3] type=float64[]
/// @resolution.call source=[1, 2, 3] parameters=(^Slice<float64>) arguments=(rest(provided(1) as float64, provided(2) as float64, provided(3) as float64) as float64) return=float64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<float64>
/// @generic.instantiation id=arrayFromOwnedSlice<float64> template=arrayFromOwnedSlice arguments=(float64)
/// @generic.instance id=arrayFromOwnedSlice<float64> template=arrayFromOwnedSlice arguments=(float64)
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 adjustments=[{ kind: materialize, target: float64 }] origin=implicit
/// @type.node source=2 type=2
/// @coercion.node source=2 from=2 adjustments=[{ kind: materialize, target: float64 }] origin=implicit
/// @type.node source=3 type=3
/// @coercion.node source=3 from=3 adjustments=[{ kind: materialize, target: float64 }] origin=implicit
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
        "main.tspp",
        DirRows::checked().with_reference_types().with_coercion(),
        r#"
=== annotated ===
const values: float64[] = [1, 2] satisfies readonly number[];

=== dir ===
const values = [1, 2] satisfies readonly number[];
/// @type.symbol symbol=values source=values type=float64[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<float64> template=Array arguments=(float64)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<float64>> template=sliceAssumeInit arguments=(MaybeUninit<float64>)
/// @generic.instance id=sliceUninit<MaybeUninit<float64>> template=sliceUninit arguments=(MaybeUninit<float64>)
/// @type.node source=[1, 2] satisfies readonly number[] type=float64[]
/// @type.node source=[1, 2] type=float64[]
/// @resolution.call source=[1, 2] parameters=(^Slice<float64>) arguments=(rest(provided(1) as float64, provided(2) as float64) as float64) return=float64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<float64>
/// @generic.instantiation id=arrayFromOwnedSlice<float64> template=arrayFromOwnedSlice arguments=(float64)
/// @generic.instance id=arrayFromOwnedSlice<float64> template=arrayFromOwnedSlice arguments=(float64)
/// @type.node source=1 type=1
/// @type.node source=2 type=2
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
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const values: float64[] = [1, "two"];

=== dir ===
const values: number[] = [1, "two"];
/// @type.symbol symbol=values source=values type=float64[]
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source=[1, "two"] type=float64[]
/// @resolution.call source=[1, "two"] parameters=(^Slice<float64>) arguments=(rest(provided(1) as float64, provided("two") as float64) as float64) return=float64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<float64>
/// @generic.instantiation id=arrayFromOwnedSlice<float64> template=arrayFromOwnedSlice arguments=(float64)
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

/// A fresh spread source takes the element context of the array it spreads into.
#[test]
fn test_spread_a_fresh_array_literal_under_the_element_context() {
    let session = TestSession::single(
        r#"
function values(): (int32 | undefined)[] {
    return [0, ...[1, , 2], 3];
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function values(): (int32 | undefined)[] {
    return [
        0 as int32 | undefined,
        ...[1 as int32 | undefined, , 2 as int32 | undefined],
        3 as int32 | undefined,
    ];
}

=== dir ===
function values(): (int32 | undefined)[] {
/// @type.symbol symbol=values type=() => int32 | undefined[]

    return [0, ...[1, , 2], 3];
    /// @resolution.call source=[0, ...[1, , 2], 3] parameters=(^Slice<int32 | undefined>) arguments=(rest(provided(0) as int32 | undefined, spread(provided(...[1, , 2]) as ^int32 | undefined[], iterator=iterator#1(parameters=(), arguments=(), return=Iterator<int32 | undefined>), next=dynamic(Iterator<int32 | undefined> as Iterator<int32 | undefined>, Iterator.next)(parameters=(), arguments=(), return=IteratorResult<int32 | undefined, void>, regions=("managed" & "local"))) as int32 | undefined, provided(3) as int32 | undefined) as int32 | undefined) return=int32 | undefined[] kind=symbol target=arrayFromOwnedSlice instance="arrayFromOwnedSlice<int32 | undefined>"
    /// @generic.instantiation id="iterator#1<int32 | undefined>" template=iterator#1 arguments=(int32 | undefined)
    /// @resolution.call source=[1, , 2] parameters=(^Slice<int32 | undefined>) arguments=(rest(provided(1) as int32 | undefined, omitted as int32 | undefined, provided(2) as int32 | undefined) as int32 | undefined) return=int32 | undefined[] kind=symbol target=arrayFromOwnedSlice instance="arrayFromOwnedSlice<int32 | undefined>"
    /// @generic.instantiation id="arrayFromOwnedSlice<int32 | undefined>" template=arrayFromOwnedSlice arguments=(int32 | undefined)

}
"#,
        r#"

"#,
    );
}
