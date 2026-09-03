use crate::tests::{DirRows, TestSession};

#[test]
fn test_assign_a_tuple_to_a_target_with_a_trailing_optional_element() {
    let session = TestSession::single(
        r#"
declare const pair: (int32, int32);
const triple: (int32, int32, int32?) = pair;
const same: (int32, int32) = pair;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
declare const pair: (int32, int32);
const triple: (int32, int32, int32?) = pair as (int32, int32, int32?);
const same: (int32, int32) = pair;

=== dir ===
declare const pair: (int32, int32);
/// @type.symbol symbol=pair source=pair type=(int32, int32)
/// @resolution.pattern source=pair kind=binding target=pair

const triple: (int32, int32, int32?) = pair;
/// @type.symbol symbol=triple source=triple type=(int32, int32, int32?)
/// @resolution.pattern source=triple kind=binding target=triple
/// @resolution.name source=pair target=pair
/// @resolution.place source=pair placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=pair root=pair
/// @coercion.node source=pair from=(int32, int32) adjustments=[{ kind: tuple, target: (int32, int32, int32?) }] origin=implicit

const same: (int32, int32) = pair;
/// @type.symbol symbol=same source=same type=(int32, int32)
/// @resolution.pattern source=same kind=binding target=same
/// @resolution.name source=pair target=pair
/// @resolution.place source=pair placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=pair root=pair
"#,
    );
}

#[test]
fn test_tuple_subscript_selects_literal_element() {
    let session = TestSession::single(
        r#"
const tuple = ["id", 42] as const;
const name = tuple[0];
const count = tuple[1];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const tuple: readonly ("id" | 42)[] = ["id" as "id" | 42, 42 as "id" | 42] as const;
const name: "id" | 42 = tuple[0];
const count: "id" | 42 = tuple[1];

=== dir ===
const tuple = ["id", 42] as const;
/// @type.symbol symbol=tuple source=tuple type=readonly "id" | 42[]
/// @resolution.pattern source=tuple kind=binding target=tuple
/// @generic.instance id="Array<\"id\" | 42>" template=Array arguments=("id" | 42)
/// @generic.instance id="MaybeUninit<MaybeUninit<\"id\" | 42>>" template=MaybeUninit arguments=(MaybeUninit<"id" | 42>)
/// @generic.instance id="MaybeUninit<\"id\" | 42>" template=MaybeUninit arguments=("id" | 42)
/// @generic.instance id="assumeInitDrop#1<\"id\" | 42>" template=assumeInitDrop#1 arguments=("id" | 42)
/// @generic.instance id="assumeInitDrop<\"id\" | 42>" template=assumeInitDrop arguments=("id" | 42) evaluated=((WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive">) => Raw<assumeInitDrop.T> => (&assumeInitDrop.'a exclusive MaybeUninit<"id" | 42>) => Raw<"id" | 42>, WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive"> => &assumeInitDrop.'a exclusive MaybeUninit<"id" | 42>)
/// @generic.instance id="clear<\"id\" | 42>" template=clear arguments=("id" | 42)
/// @generic.instance id="drop<\"id\" | 42>" template=drop arguments=("id" | 42)
/// @generic.instance id="dropInPlace<\"id\" | 42>" template=dropInPlace arguments=("id" | 42)
/// @generic.instance id="elementSlot<\"id\" | 42, \"exclusive\">" template=elementSlot arguments=("id" | 42, "exclusive") evaluated=(<elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => <elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(&elementSlot.'a exclusive "id" | 42[], usize) => &elementSlot.'a exclusive MaybeUninit<"id" | 42>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<"id" | 42>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<"id" | 42>>, usize) => &elementSlot.'a exclusive MaybeUninit<"id" | 42>, WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<"id" | 42>>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<"id" | 42>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<"id" | 42>>, usize) => &elementSlot.'a exclusive MaybeUninit<"id" | 42>, WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A> => &elementSlot.'a exclusive "id" | 42[], WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<"id" | 42>>)
/// @generic.instance id="initAsPointer<\"id\" | 42, \"exclusive\">" template=initAsPointer arguments=("id" | 42, "exclusive") evaluated=(<initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(WithAccess<&initAsPointer.'a MaybeUninit<initAsPointer.T>, initAsPointer.A>) => Raw<initAsPointer.T> => <initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(&initAsPointer.'a exclusive MaybeUninit<"id" | 42>) => Raw<"id" | 42>)
/// @generic.instance id="new<MaybeUninit<\"id\" | 42>>" template=new arguments=(MaybeUninit<"id" | 42>)
/// @generic.instance id="sliceAssumeInit<MaybeUninit<\"id\" | 42>>" template=sliceAssumeInit arguments=(MaybeUninit<"id" | 42>)
/// @generic.instance id="sliceIndex<MaybeUninit<\"id\" | 42>, \"exclusive\">" template=sliceIndex arguments=(MaybeUninit<"id" | 42>, "exclusive") evaluated=(<sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(WithAccess<&sliceIndex.'a Slice<sliceIndex.T>, sliceIndex.A>, usize) => WithAccess<&sliceIndex.'a sliceIndex.T, sliceIndex.A> => <sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(&sliceIndex.'a exclusive Slice<MaybeUninit<"id" | 42>>, usize) => &sliceIndex.'a exclusive MaybeUninit<"id" | 42>)
/// @generic.instance id="sliceUninit<MaybeUninit<\"id\" | 42>>" template=sliceUninit arguments=(MaybeUninit<"id" | 42>)
/// @generic.instance id="truncate<\"id\" | 42>" template=truncate arguments=("id" | 42) evaluated=(WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => &truncate.'a exclusive MaybeUninit<"id" | 42>, (WithAccess<&truncate.'a T#6[], "exclusive">, usize) => WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => (&truncate.'a exclusive "id" | 42[], usize) => &truncate.'a exclusive MaybeUninit<"id" | 42>, WithAccess<&truncate.'a T#6[], "exclusive"> => &truncate.'a exclusive "id" | 42[])
/// @resolution.call source=["id", 42] parameters=(^Slice<arrayFromOwnedSlice.T>) arguments=(rest("id", 42) as "id" | 42) return="id" | 42[] kind=symbol target=arrayFromOwnedSlice instance="arrayFromOwnedSlice<\"id\" | 42>"
/// @generic.instantiation id="arrayFromOwnedSlice<\"id\" | 42>" template=arrayFromOwnedSlice arguments=("id" | 42)
/// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
/// @generic.instance id="Slice<\"id\" | 42>" template=Slice arguments=("id" | 42)
/// @generic.instance id="arrayFromOwnedSlice<\"id\" | 42>" template=arrayFromOwnedSlice arguments=("id" | 42)
/// @generic.instance id="fromOwnedSlice<\"id\" | 42>" template=fromOwnedSlice arguments=("id" | 42)
/// @generic.instance id="intoUninit<\"id\" | 42>" template=intoUninit arguments=("id" | 42)
/// @generic.instance id="size<\"id\" | 42>" template=size arguments=("id" | 42)
/// @generic.instance id="sliceIntoUninit<\"id\" | 42>" template=sliceIntoUninit arguments=("id" | 42)
/// @generic.instance id="sliceLength<\"id\" | 42>" template=sliceLength arguments=("id" | 42)
/// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)

const name = tuple[0];
/// @type.symbol symbol=name source=name type="id" | 42
/// @resolution.pattern source=name kind=binding target=name
/// @resolution.name source=tuple target=tuple
/// @resolution.place source=tuple placement="local" lifetime="managed" access="readonly"
/// @resolution.access source=tuple root=tuple
/// @resolution.access source=tuple[0] root=tuple keys=[0]
/// @resolution.subscript source=tuple[0] type="id" | 42 kind=call target="index#1(parameters=(isize), arguments=(provided(0) as isize), return=WithAccess<Borrowed<\"id\" | 42, \"managed\" & \"local\", \"mutable\">, \"readonly\">)"
/// @generic.instantiation id="index#1<\"id\" | 42, \"readonly\">" template=index#1 arguments=("id" | 42, "readonly")
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="assumeInitReference<\"id\" | 42, \"readonly\">" template=assumeInitReference arguments=("id" | 42, "readonly") evaluated=(<assumeInitReference.T, const assumeInitReference.A: Access = "mutable", assumeInitReference.'a>(WithAccess<&assumeInitReference.'a MaybeUninit<assumeInitReference.T>, assumeInitReference.A>) => WithAccess<&assumeInitReference.'a assumeInitReference.T, assumeInitReference.A> => <assumeInitReference.T, const assumeInitReference.A: Access = "mutable", assumeInitReference.'a>(&assumeInitReference.'a readonly MaybeUninit<"id" | 42>) => &assumeInitReference.'a readonly "id" | 42)
/// @generic.instance id="elementPosition<\"id\" | 42>" template=elementPosition arguments=("id" | 42)
/// @generic.instance id="elementSlot<\"id\" | 42, \"readonly\">" template=elementSlot arguments=("id" | 42, "readonly") evaluated=(<elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => <elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(&elementSlot.'a readonly "id" | 42[], usize) => &elementSlot.'a readonly MaybeUninit<"id" | 42>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a readonly MaybeUninit<"id" | 42>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a readonly Slice<MaybeUninit<"id" | 42>>, usize) => &elementSlot.'a readonly MaybeUninit<"id" | 42>, WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a readonly Slice<MaybeUninit<"id" | 42>>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a readonly MaybeUninit<"id" | 42>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a readonly Slice<MaybeUninit<"id" | 42>>, usize) => &elementSlot.'a readonly MaybeUninit<"id" | 42>, WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A> => &elementSlot.'a readonly "id" | 42[], WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a readonly Slice<MaybeUninit<"id" | 42>>)
/// @generic.instance id="index#1<\"id\" | 42, \"readonly\">" template=index#1 arguments=("id" | 42, "readonly") evaluated=(<const index.A: Access = "readonly", index#1.'a>(this: WithAccess<&index#1.'a T#6[], index.A>, isize) => WithAccess<&index#1.'a T#6, index.A> => <const index.A: Access = "readonly", index#1.'a>(this: &index#1.'a readonly "id" | 42[], isize) => &index#1.'a readonly "id" | 42, WithAccess<&index#1.'a T#6, index.A> => &index#1.'a readonly "id" | 42, WithAccess<&index#1.'a T#6[], index.A> => &index#1.'a readonly "id" | 42[], (WithAccess<&index#1.'a MaybeUninit<T#6>, index.A>) => WithAccess<&index#1.'a T#6, index.A> => (&index#1.'a readonly MaybeUninit<"id" | 42>) => &index#1.'a readonly "id" | 42, WithAccess<&index#1.'a MaybeUninit<T#6>, index.A> => &index#1.'a readonly MaybeUninit<"id" | 42>, WithAccess<&index#1.'a T#6, index.A> => &index#1.'a readonly "id" | 42, (WithAccess<&index#1.'a MaybeUninit<T#6>, index.A>) => WithAccess<&index#1.'a T#6, index.A> => (&index#1.'a readonly MaybeUninit<"id" | 42>) => &index#1.'a readonly "id" | 42, WithAccess<&index#1.'a MaybeUninit<T#6>, index.A> => &index#1.'a readonly MaybeUninit<"id" | 42>, (WithAccess<&index#1.'a T#6[], index.A>, usize) => WithAccess<&index#1.'a MaybeUninit<T#6>, index.A> => (&index#1.'a readonly "id" | 42[], usize) => &index#1.'a readonly MaybeUninit<"id" | 42>, WithAccess<&index#1.'a T#6[], index.A> => &index#1.'a readonly "id" | 42[])
/// @generic.instance id="sliceIndex<MaybeUninit<\"id\" | 42>, \"readonly\">" template=sliceIndex arguments=(MaybeUninit<"id" | 42>, "readonly") evaluated=(<sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(WithAccess<&sliceIndex.'a Slice<sliceIndex.T>, sliceIndex.A>, usize) => WithAccess<&sliceIndex.'a sliceIndex.T, sliceIndex.A> => <sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(&sliceIndex.'a readonly Slice<MaybeUninit<"id" | 42>>, usize) => &sliceIndex.'a readonly MaybeUninit<"id" | 42>)
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)

const count = tuple[1];
/// @type.symbol symbol=count source=count type="id" | 42
/// @resolution.pattern source=count kind=binding target=count
/// @resolution.name source=tuple target=tuple
/// @resolution.place source=tuple placement="local" lifetime="managed" access="readonly"
/// @resolution.access source=tuple root=tuple
/// @resolution.access source=tuple[1] root=tuple keys=[1]
/// @resolution.subscript source=tuple[1] type="id" | 42 kind=call target="index#1(parameters=(isize), arguments=(provided(1) as isize), return=WithAccess<Borrowed<\"id\" | 42, \"managed\" & \"local\", \"mutable\">, \"readonly\">)"
"#,
    );
}
