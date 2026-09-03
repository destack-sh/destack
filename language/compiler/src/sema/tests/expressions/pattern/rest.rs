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
/// @type.symbol symbol=values source=values type=int32[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id="initAsPointer<int32, \"exclusive\">" template=initAsPointer arguments=(int32, "exclusive") evaluated=(<initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(WithAccess<&initAsPointer.'a MaybeUninit<initAsPointer.T>, initAsPointer.A>) => Raw<initAsPointer.T> => <initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(&initAsPointer.'a exclusive MaybeUninit<int32>) => Raw<int32>)
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=MaybeUninit<MaybeUninit<int32>> template=MaybeUninit arguments=(MaybeUninit<int32>)
/// @generic.instance id=MaybeUninit<int32> template=MaybeUninit arguments=(int32)
/// @generic.instance id=assumeInitDrop#1<int32> template=assumeInitDrop#1 arguments=(int32)
/// @generic.instance id=assumeInitDrop<int32> template=assumeInitDrop arguments=(int32) evaluated=((WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive">) => Raw<assumeInitDrop.T> => (&assumeInitDrop.'a exclusive MaybeUninit<int32>) => Raw<int32>, WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive"> => &assumeInitDrop.'a exclusive MaybeUninit<int32>)
/// @generic.instance id=clear<int32> template=clear arguments=(int32)
/// @generic.instance id=drop<int32> template=drop arguments=(int32)
/// @generic.instance id=dropInPlace<int32> template=dropInPlace arguments=(int32)
/// @generic.instance id=new<MaybeUninit<int32>> template=new arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @generic.instance id=truncate<int32> template=truncate arguments=(int32) evaluated=(WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => &truncate.'a exclusive MaybeUninit<int32>, (WithAccess<&truncate.'a T#6[], "exclusive">, usize) => WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => (&truncate.'a exclusive int32[], usize) => &truncate.'a exclusive MaybeUninit<int32>, WithAccess<&truncate.'a T#6[], "exclusive"> => &truncate.'a exclusive int32[])

let [head, ...tail] = values;
/// @resolution.pattern source=[head, ...tail] kind=sequence element=int32 arity=1.. fields=(head) rest=...tail
/// @generic.instantiation id="index#1<int32, \"exclusive\">" template=index#1 arguments=(int32, "exclusive")
/// @generic.instantiation id="rest#2<int32, \"local\">" template=rest#2 arguments=(int32, "local")
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="assumeInitReference<int32, \"exclusive\">" template=assumeInitReference arguments=(int32, "exclusive") evaluated=(<assumeInitReference.T, const assumeInitReference.A: Access = "mutable", assumeInitReference.'a>(WithAccess<&assumeInitReference.'a MaybeUninit<assumeInitReference.T>, assumeInitReference.A>) => WithAccess<&assumeInitReference.'a assumeInitReference.T, assumeInitReference.A> => <assumeInitReference.T, const assumeInitReference.A: Access = "mutable", assumeInitReference.'a>(&assumeInitReference.'a exclusive MaybeUninit<int32>) => &assumeInitReference.'a exclusive int32)
/// @generic.instance id="elementSlot<int32, \"exclusive\">" template=elementSlot arguments=(int32, "exclusive") evaluated=(<elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => <elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(&elementSlot.'a exclusive int32[], usize) => &elementSlot.'a exclusive MaybeUninit<int32>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<int32>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<int32>>, usize) => &elementSlot.'a exclusive MaybeUninit<int32>, WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<int32>>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<int32>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<int32>>, usize) => &elementSlot.'a exclusive MaybeUninit<int32>, WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A> => &elementSlot.'a exclusive int32[], WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<int32>>)
/// @generic.instance id="index#1<int32, \"exclusive\">" template=index#1 arguments=(int32, "exclusive") evaluated=(<const index.A: Access = "readonly", index#1.'a>(this: WithAccess<&index#1.'a T#6[], index.A>, isize) => WithAccess<&index#1.'a T#6, index.A> => <const index.A: Access = "readonly", index#1.'a>(this: &index#1.'a exclusive int32[], isize) => &index#1.'a exclusive int32, WithAccess<&index#1.'a T#6, index.A> => &index#1.'a exclusive int32, WithAccess<&index#1.'a T#6[], index.A> => &index#1.'a exclusive int32[], (WithAccess<&index#1.'a MaybeUninit<T#6>, index.A>) => WithAccess<&index#1.'a T#6, index.A> => (&index#1.'a exclusive MaybeUninit<int32>) => &index#1.'a exclusive int32, WithAccess<&index#1.'a MaybeUninit<T#6>, index.A> => &index#1.'a exclusive MaybeUninit<int32>, WithAccess<&index#1.'a T#6, index.A> => &index#1.'a exclusive int32, (WithAccess<&index#1.'a MaybeUninit<T#6>, index.A>) => WithAccess<&index#1.'a T#6, index.A> => (&index#1.'a exclusive MaybeUninit<int32>) => &index#1.'a exclusive int32, WithAccess<&index#1.'a MaybeUninit<T#6>, index.A> => &index#1.'a exclusive MaybeUninit<int32>, (WithAccess<&index#1.'a T#6[], index.A>, usize) => WithAccess<&index#1.'a MaybeUninit<T#6>, index.A> => (&index#1.'a exclusive int32[], usize) => &index#1.'a exclusive MaybeUninit<int32>, WithAccess<&index#1.'a T#6[], index.A> => &index#1.'a exclusive int32[])
/// @generic.instance id="rest#2<int32, \"local\">" template=rest#2 arguments=(int32, "local")
/// @generic.instance id="sliceIndex<MaybeUninit<int32>, \"exclusive\">" template=sliceIndex arguments=(MaybeUninit<int32>, "exclusive") evaluated=(<sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(WithAccess<&sliceIndex.'a Slice<sliceIndex.T>, sliceIndex.A>, usize) => WithAccess<&sliceIndex.'a sliceIndex.T, sliceIndex.A> => <sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(&sliceIndex.'a exclusive Slice<MaybeUninit<int32>>, usize) => &sliceIndex.'a exclusive MaybeUninit<int32>)
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id=elementPosition<int32> template=elementPosition arguments=(int32)
/// @type.symbol symbol=head source=head type=int32
/// @resolution.pattern source=head kind=binding target=head
/// @type.symbol symbol=tail source=tail type=^int32[]
/// @resolution.pattern source=tail kind=binding target=tail
/// @type.node source=values type=int32[]
/// @resolution.name source=values target=values
/// @resolution.access source=values root=values

head satisfies int32;
/// @type.node source="head satisfies int32" type=int32
/// @type.node source=head type=int32
/// @resolution.name source=head target=head
/// @resolution.place source=head placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=head root=head

tail satisfies ^int32[];
/// @type.node source="tail satisfies ^int32[]" type=^int32[]
/// @type.node source=tail type=^int32[]
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
/// @type.node source=[1, 2, 3] type=int64[]
/// @resolution.call source=[1, 2, 3] parameters=(^Slice<arrayFromOwnedSlice.T>) arguments=(rest(1, 2, 3) as int64) return=int64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int64>
/// @generic.instantiation id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
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
/// @type.node source=[1, 2, 3] type=int64[]
/// @resolution.call source=[1, 2, 3] parameters=(^Slice<arrayFromOwnedSlice.T>) arguments=(rest(1, 2, 3) as int64) return=int64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int64>
/// @generic.instantiation id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
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
