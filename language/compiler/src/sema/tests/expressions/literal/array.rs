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
let values: float64[] = [1, 2];

=== dir ===
let values = [1, 2];
/// @type.symbol symbol=values source=values type=Array<float64>
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<float64> template=collections.array.Array arguments=(float64)
/// @generic.instance id=memory.init.MaybeUninit<float64> template=memory.init.MaybeUninit arguments=(float64)
/// @generic.instance id=memory.unique.Unique<Slice<memory.init.MaybeUninit<float64>>> template=memory.unique.Unique arguments=(Slice<memory.init.MaybeUninit<float64>>)
/// @generic.instance id=memory.unique.empty<memory.init.MaybeUninit<float64>> template=memory.unique.empty arguments=(memory.init.MaybeUninit<float64>)
/// @type.node source=[1, 2] type=Array<float64>
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 adjustments=[{ kind: widen, target: float64 }] origin=implicit
/// @type.node source=2 type=2
/// @coercion.node source=2 from=2 adjustments=[{ kind: widen, target: float64 }] origin=implicit
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
const values: float64[] = [1, 2];

=== dir ===
const values = [1, 2];
/// @type.symbol symbol=values source=values type=Array<float64>
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<float64> template=collections.array.Array arguments=(float64)
/// @generic.instance id=memory.init.MaybeUninit<float64> template=memory.init.MaybeUninit arguments=(float64)
/// @generic.instance id=memory.unique.Unique<Slice<memory.init.MaybeUninit<float64>>> template=memory.unique.Unique arguments=(Slice<memory.init.MaybeUninit<float64>>)
/// @generic.instance id=memory.unique.empty<memory.init.MaybeUninit<float64>> template=memory.unique.empty arguments=(memory.init.MaybeUninit<float64>)
/// @type.node source=[1, 2] type=Array<float64>
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 adjustments=[{ kind: widen, target: float64 }] origin=implicit
/// @type.node source=2 type=2
/// @coercion.node source=2 from=2 adjustments=[{ kind: widen, target: float64 }] origin=implicit
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
/// @resolution.place source=values placement="local" lifetime="static" access="readonly"
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
/// @type.symbol symbol=values source=values type=Array<never>
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<never> template=collections.array.Array arguments=(never)
/// @generic.instance id=memory.init.MaybeUninit<never> template=memory.init.MaybeUninit arguments=(never)
/// @generic.instance id=memory.unique.Unique<Slice<memory.init.MaybeUninit<never>>> template=memory.unique.Unique arguments=(Slice<memory.init.MaybeUninit<never>>)
/// @generic.instance id=memory.unique.empty<memory.init.MaybeUninit<never>> template=memory.unique.empty arguments=(memory.init.MaybeUninit<never>)
/// @type.node source=[] type=Array<never>
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
/// @type.symbol symbol=values source=values type=Array<int32>
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<int32> template=collections.array.Array arguments=(int32)
/// @generic.instance id=memory.init.MaybeUninit<int32> template=memory.init.MaybeUninit arguments=(int32)
/// @generic.instance id=memory.unique.Unique<Slice<memory.init.MaybeUninit<int32>>> template=memory.unique.Unique arguments=(Slice<memory.init.MaybeUninit<int32>>)
/// @generic.instance id=memory.unique.empty<memory.init.MaybeUninit<int32>> template=memory.unique.empty arguments=(memory.init.MaybeUninit<int32>)
/// @type.node source=[] type=Array<int32>
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

    session.assert_dir(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            .with_coercion()
            ,
        r#"
=== annotated ===
let values: (float64 | string | boolean)[] = [
    1 as float64 | string | boolean,
    "two" as float64 | string | boolean,
    true as float64 | string | boolean,
];

=== dir ===
let values = [1, "two", true];
/// @type.symbol symbol=values source=values type=Array<float64 | string | boolean>
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id="Array<float64 | string | boolean>" template=collections.array.Array arguments=(float64 | string | boolean)
/// @generic.instance id="memory.init.MaybeUninit<float64 | string | boolean>" template=memory.init.MaybeUninit arguments=(float64 | string | boolean)
/// @generic.instance id="memory.unique.Unique<Slice<memory.init.MaybeUninit<float64 | string | boolean>>>" template=memory.unique.Unique arguments=(Slice<memory.init.MaybeUninit<float64 | string | boolean>>)
/// @generic.instance id="memory.unique.empty<memory.init.MaybeUninit<float64 | string | boolean>>" template=memory.unique.empty arguments=(memory.init.MaybeUninit<float64 | string | boolean>)
/// @type.node source=[1, "two", true] type=Array<float64 | string | boolean>
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 adjustments=[{ kind: union, target: float64 | string | boolean, cases: ({ source: 1, target: float64, adjustments: [{ kind: widen, target: float64 }] }) }] origin=implicit
/// @type.node source="\"two\"" type="two"
/// @coercion.node source="\"two\"" from="two" adjustments=[{ kind: union, target: float64 | string | boolean, cases: ({ source: "two", target: string, adjustments: [{ kind: widen, target: string }] }) }] origin=implicit
/// @type.node source=true type=true
/// @coercion.node source=true from=true adjustments=[{ kind: union, target: float64 | string | boolean, cases: ({ source: true, target: boolean, adjustments: [{ kind: widen, target: boolean }] }) }] origin=implicit
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
/// @type.symbol symbol=values source=values type=Array<float64>
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<float64> template=collections.array.Array arguments=(float64)
/// @generic.instance id=memory.init.MaybeUninit<float64> template=memory.init.MaybeUninit arguments=(float64)
/// @generic.instance id=memory.unique.Unique<Slice<memory.init.MaybeUninit<float64>>> template=memory.unique.Unique arguments=(Slice<memory.init.MaybeUninit<float64>>)
/// @generic.instance id=memory.unique.empty<memory.init.MaybeUninit<float64>> template=memory.unique.empty arguments=(memory.init.MaybeUninit<float64>)
/// @type.node source=[1, 2, 3] type=Array<float64>
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
/// @type.symbol symbol=values source=values type=Array<1 | 2>
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id="Array<1 | 2>" template=collections.array.Array arguments=(1 | 2)
/// @generic.instance id="memory.init.MaybeUninit<1 | 2>" template=memory.init.MaybeUninit arguments=(1 | 2)
/// @generic.instance id="memory.unique.Unique<Slice<memory.init.MaybeUninit<1 | 2>>>" template=memory.unique.Unique arguments=(Slice<memory.init.MaybeUninit<1 | 2>>)
/// @generic.instance id="memory.unique.empty<memory.init.MaybeUninit<1 | 2>>" template=memory.unique.empty arguments=(memory.init.MaybeUninit<1 | 2>)
/// @type.node source=[1, 2] satisfies readonly number[] type=Array<1 | 2>
/// @type.node source=[1, 2] type=Array<1 | 2>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @generic.instance id=Array<float64> template=collections.array.Array arguments=(float64)
/// @generic.instance id=memory.init.MaybeUninit<float64> template=memory.init.MaybeUninit arguments=(float64)
/// @generic.instance id=memory.unique.Unique<Slice<memory.init.MaybeUninit<float64>>> template=memory.unique.Unique arguments=(Slice<memory.init.MaybeUninit<float64>>)
/// @generic.instance id=memory.unique.empty<memory.init.MaybeUninit<float64>> template=memory.unique.empty arguments=(memory.init.MaybeUninit<float64>)
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
/// @type.symbol symbol=values source=values type=Array<float64>
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source=[1, "two"] type=Array<float64>
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
