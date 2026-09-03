use crate::tests::{DirRows, TestSession};

#[test]
fn test_object_pattern_parameter_binds_fields() {
    let session = TestSession::single(
        r#"
function label({ name, age }: { name: string; age: int32 }): string {
    name satisfies string;
    age satisfies int32;

    name
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function label({ name, age }: { name: string; age: int32 }): string {
    name satisfies string;
    age satisfies int32;

    name
}

=== dir ===
function label({ name, age }: { name: string; age: int32 }): string {
/// @type.symbol symbol=label type=({ name: string; age: int32 }) => string
/// @resolution.pattern source={ name, age } kind=object fields={ name, age }
/// @type.symbol symbol=label.name#2 source=name type=string
/// @type.symbol symbol=label.age#2 source=age type=int32
/// @type.symbol symbol=label.name#1 source="name: string" type=string
/// @type.symbol symbol=label.age#1 source="age: int32" type=int32

    name satisfies string;
    /// @type.node source="name satisfies string" type=string
    /// @type.node source=name type=string
    /// @resolution.name source=name target=label.name#2
    /// @resolution.place source=name placement="local" lifetime="managed" access="exclusive"
    /// @resolution.access source=name root=label.name#2

    age satisfies int32;
    /// @type.node source="age satisfies int32" type=int32
    /// @type.node source=age type=int32
    /// @resolution.name source=age target=label.age#2
    /// @resolution.place source=age placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=age root=label.age#2

    name
    /// @type.node source=name type=string
    /// @resolution.name source=name target=label.name#2
    /// @resolution.place source=name placement="local" lifetime="managed" access="exclusive"
    /// @resolution.access source=name root=label.name#2

}
"#,
    );
}

#[test]
fn test_sequence_pattern_parameter_binds_elements() {
    let session = TestSession::single(
        r#"
function first([head]: int32[]): int32 {
    head
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function first([head]: int32[]): int32 {
    head
}

=== dir ===
function first([head]: int32[]): int32 {
/// @type.symbol symbol=first type=(int32[]) => int32
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
/// @resolution.pattern source=[head] kind=sequence element=int32 arity=1 fields=(first.head)
/// @generic.instantiation id="index#1<int32, \"exclusive\">" template=index#1 arguments=(int32, "exclusive")
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="assumeInitReference<int32, \"exclusive\">" template=assumeInitReference arguments=(int32, "exclusive") evaluated=(<assumeInitReference.T, const assumeInitReference.A: Access = "mutable", assumeInitReference.'a>(WithAccess<&assumeInitReference.'a MaybeUninit<assumeInitReference.T>, assumeInitReference.A>) => WithAccess<&assumeInitReference.'a assumeInitReference.T, assumeInitReference.A> => <assumeInitReference.T, const assumeInitReference.A: Access = "mutable", assumeInitReference.'a>(&assumeInitReference.'a exclusive MaybeUninit<int32>) => &assumeInitReference.'a exclusive int32)
/// @generic.instance id="elementSlot<int32, \"exclusive\">" template=elementSlot arguments=(int32, "exclusive") evaluated=(<elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => <elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(&elementSlot.'a exclusive int32[], usize) => &elementSlot.'a exclusive MaybeUninit<int32>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<int32>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<int32>>, usize) => &elementSlot.'a exclusive MaybeUninit<int32>, WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<int32>>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<int32>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<int32>>, usize) => &elementSlot.'a exclusive MaybeUninit<int32>, WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A> => &elementSlot.'a exclusive int32[], WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<int32>>)
/// @generic.instance id="index#1<int32, \"exclusive\">" template=index#1 arguments=(int32, "exclusive") evaluated=(<const index.A: Access = "readonly", index#1.'a>(this: WithAccess<&index#1.'a T#6[], index.A>, isize) => WithAccess<&index#1.'a T#6, index.A> => <const index.A: Access = "readonly", index#1.'a>(this: &index#1.'a exclusive int32[], isize) => &index#1.'a exclusive int32, WithAccess<&index#1.'a T#6, index.A> => &index#1.'a exclusive int32, WithAccess<&index#1.'a T#6[], index.A> => &index#1.'a exclusive int32[], (WithAccess<&index#1.'a MaybeUninit<T#6>, index.A>) => WithAccess<&index#1.'a T#6, index.A> => (&index#1.'a exclusive MaybeUninit<int32>) => &index#1.'a exclusive int32, WithAccess<&index#1.'a MaybeUninit<T#6>, index.A> => &index#1.'a exclusive MaybeUninit<int32>, WithAccess<&index#1.'a T#6, index.A> => &index#1.'a exclusive int32, (WithAccess<&index#1.'a MaybeUninit<T#6>, index.A>) => WithAccess<&index#1.'a T#6, index.A> => (&index#1.'a exclusive MaybeUninit<int32>) => &index#1.'a exclusive int32, WithAccess<&index#1.'a MaybeUninit<T#6>, index.A> => &index#1.'a exclusive MaybeUninit<int32>, (WithAccess<&index#1.'a T#6[], index.A>, usize) => WithAccess<&index#1.'a MaybeUninit<T#6>, index.A> => (&index#1.'a exclusive int32[], usize) => &index#1.'a exclusive MaybeUninit<int32>, WithAccess<&index#1.'a T#6[], index.A> => &index#1.'a exclusive int32[])
/// @generic.instance id="sliceIndex<MaybeUninit<int32>, \"exclusive\">" template=sliceIndex arguments=(MaybeUninit<int32>, "exclusive") evaluated=(<sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(WithAccess<&sliceIndex.'a Slice<sliceIndex.T>, sliceIndex.A>, usize) => WithAccess<&sliceIndex.'a sliceIndex.T, sliceIndex.A> => <sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(&sliceIndex.'a exclusive Slice<MaybeUninit<int32>>, usize) => &sliceIndex.'a exclusive MaybeUninit<int32>)
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id=elementPosition<int32> template=elementPosition arguments=(int32)
/// @type.symbol symbol=first.head source=head type=int32
/// @resolution.pattern source=head kind=binding target=first.head

    head
    /// @type.node source=head type=int32
    /// @resolution.name source=head target=first.head
    /// @resolution.place source=head placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=head root=first.head

}
"#,
    );
}
