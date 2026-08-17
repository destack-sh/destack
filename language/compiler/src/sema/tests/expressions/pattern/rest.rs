use crate::tests::{DirRows, TestSession};

#[test]
fn test_sequence_pattern_rest_binds_array_tail() {
    let session = TestSession::single(
        r#"
declare const values: int32[];

let [head, ...tail] = values;

head satisfies int32;
tail satisfies ^int32[];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const values: int32[];

let [head, ...tail] = values;

head satisfies int32;
tail satisfies ^int32[];

=== dir ===
declare const values: int32[];
/// @type.symbol symbol=values source=values type=Array<int32>
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<int32> template=collections.array.Array arguments=(int32)
/// @generic.instance id=memory.init.MaybeUninit<int32> template=memory.init.MaybeUninit arguments=(int32)
/// @generic.instance id=memory.raw.dangling<memory.init.MaybeUninit<int32>> template=memory.raw.dangling arguments=(memory.init.MaybeUninit<int32>)
/// @generic.instance id=memory.unique.Unique<Slice<memory.init.MaybeUninit<int32>>> template=memory.unique.Unique arguments=(Slice<memory.init.MaybeUninit<int32>>)
/// @generic.instance id=memory.unique.empty<memory.init.MaybeUninit<int32>> template=memory.unique.empty arguments=(memory.init.MaybeUninit<int32>)
/// @generic.instance id=memory.unique.emptyUniqueSlice<memory.init.MaybeUninit<int32>> template=memory.unique.emptyUniqueSlice arguments=(memory.init.MaybeUninit<int32>)
/// @generic.instance id=memory.unique.uniqueSliceFromRaw<memory.init.MaybeUninit<int32>> template=memory.unique.uniqueSliceFromRaw arguments=(memory.init.MaybeUninit<int32>)

let [head, ...tail] = values;
/// @resolution.pattern source=[head, ...tail] kind=sequence element=int32 arity=1.. fields=(head) rest=...tail
/// @generic.instantiation id="collections.array.index#1<int32, \"exclusive\">" template=collections.array.index#1 arguments=(int32, "exclusive")
/// @generic.instantiation id=collections.array.rest#2<int32> template=collections.array.rest#2 arguments=(int32)
/// @generic.instance id="collections.array.index#1<int32, \"exclusive\">" template=collections.array.index#1 arguments=(int32, "exclusive")
/// @generic.instance id="memory.type.WithAccess<&'frame Array<int32>, \"exclusive\">" template=memory.type.WithAccess arguments=(&'frame Array<int32>, "exclusive")
/// @generic.instance id="memory.type.WithAccess<&'frame int32, \"exclusive\">" template=memory.type.WithAccess arguments=(&'frame int32, "exclusive")
/// @generic.instance id=collections.array.rest#2<int32> template=collections.array.rest#2 arguments=(int32)
/// @type.symbol symbol=head source=head type=int32
/// @resolution.pattern source=head kind=binding target=head
/// @type.symbol symbol=tail source=tail type=Owned<Array<int32>>
/// @resolution.pattern source=tail kind=binding target=tail
/// @type.node source=values type=Array<int32>
/// @resolution.name source=values target=values
/// @resolution.access source=values root=values

head satisfies int32;
/// @type.node source="head satisfies int32" type=int32
/// @type.node source=head type=int32
/// @resolution.name source=head target=head
/// @resolution.place source=head placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=head root=head

tail satisfies ^int32[];
/// @type.node source="tail satisfies ^int32[]" type=Owned<Array<int32>>
/// @type.node source=tail type=Owned<Array<int32>>
/// @resolution.name source=tail target=tail
/// @resolution.place source=tail placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=tail root=tail
"#,
    );
}

#[test]
fn test_rest_pattern_must_be_last() {
    let session = TestSession::single(
        r#"
let [...middle, last] = [1, 2, 3];
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let [...middle, last] = [1, 2, 3];

=== dir ===
let [...middle, last] = [1, 2, 3];
/// @resolution.rejected source=[...middle, last]
/// @type.symbol symbol=middle source=middle type=<error>
/// @type.symbol symbol=last source=last type=<error>
/// @type.node source=[1, 2, 3] type=Array<float64>
/// @resolution.call source=[1, 2, 3] parameters=(&collections.array.arrayFromSlice.'a readonly Slice<collections.array.arrayFromSlice.T>) arguments=(rest(1, 2, 3) as float64) return=Array<float64> kind=symbol target=collections.array.arrayFromSlice instance=collections.array.arrayFromSlice<float64>
/// @generic.instantiation id=collections.array.arrayFromSlice<float64> template=collections.array.arrayFromSlice arguments=(float64)
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3
"#,
        r#"
/// @diagnostic.error id=rest-pattern-not-last message="rest pattern must be last"
/// @diagnostic.label line=2 column=6 span="...middle" line_source="let [...middle, last] = [1, 2, 3];"
"#,
    );
}

#[test]
fn test_sequence_pattern_rejects_multiple_rest_patterns() {
    let session = TestSession::single(
        r#"
let [head, ...middle, ...tail] = [1, 2, 3];
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let [head, ...middle, ...tail] = [1, 2, 3];

=== dir ===
let [head, ...middle, ...tail] = [1, 2, 3];
/// @resolution.rejected source=[head, ...middle, ...tail]
/// @type.symbol symbol=head source=head type=<error>
/// @type.symbol symbol=middle source=middle type=<error>
/// @type.symbol symbol=tail source=tail type=<error>
/// @type.node source=[1, 2, 3] type=Array<float64>
/// @resolution.call source=[1, 2, 3] parameters=(&collections.array.arrayFromSlice.'a readonly Slice<collections.array.arrayFromSlice.T>) arguments=(rest(1, 2, 3) as float64) return=Array<float64> kind=symbol target=collections.array.arrayFromSlice instance=collections.array.arrayFromSlice<float64>
/// @generic.instantiation id=collections.array.arrayFromSlice<float64> template=collections.array.arrayFromSlice arguments=(float64)
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3
"#,
        r#"
/// @diagnostic.error id=multiple-rest-patterns message="pattern can contain at most one rest field"
/// @diagnostic.label line=2 column=23 span="...tail" line_source="let [head, ...middle, ...tail] = [1, 2, 3];"
"#,
    );
}
