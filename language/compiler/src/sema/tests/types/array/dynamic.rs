use crate::tests::{DirRows, TestSession};

#[test]
fn test_dynamic_array_subscript_selects_element_type() {
    let session = TestSession::single(
        r#"
declare const bytes: uint8[];
declare const index: isize;
const byte = bytes[index];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const bytes: uint8[];
declare const index: isize;
const byte: uint8 = bytes[index];

=== dir ===
declare const bytes: uint8[];
/// @type.symbol symbol=bytes source=bytes type=uint8[]
/// @resolution.pattern source=bytes kind=binding target=bytes
/// @generic.instance id="initAsPointer<uint8, \"exclusive\">" template=initAsPointer arguments=(uint8, "exclusive") evaluated=(<initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(WithAccess<&initAsPointer.'a MaybeUninit<initAsPointer.T>, initAsPointer.A>) => Raw<initAsPointer.T> => <initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(&initAsPointer.'a exclusive MaybeUninit<uint8>) => Raw<uint8>)
/// @generic.instance id=Array<uint8> template=Array arguments=(uint8)
/// @generic.instance id=MaybeUninit<MaybeUninit<uint8>> template=MaybeUninit arguments=(MaybeUninit<uint8>)
/// @generic.instance id=MaybeUninit<uint8> template=MaybeUninit arguments=(uint8)
/// @generic.instance id=assumeInitDrop#1<uint8> template=assumeInitDrop#1 arguments=(uint8)
/// @generic.instance id=assumeInitDrop<uint8> template=assumeInitDrop arguments=(uint8) evaluated=((WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive">) => Raw<assumeInitDrop.T> => (&assumeInitDrop.'a exclusive MaybeUninit<uint8>) => Raw<uint8>, WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive"> => &assumeInitDrop.'a exclusive MaybeUninit<uint8>)
/// @generic.instance id=clear<uint8> template=clear arguments=(uint8)
/// @generic.instance id=drop<uint8> template=drop arguments=(uint8)
/// @generic.instance id=dropInPlace<uint8> template=dropInPlace arguments=(uint8)
/// @generic.instance id=new<MaybeUninit<uint8>> template=new arguments=(MaybeUninit<uint8>)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<uint8>> template=sliceAssumeInit arguments=(MaybeUninit<uint8>)
/// @generic.instance id=sliceUninit<MaybeUninit<uint8>> template=sliceUninit arguments=(MaybeUninit<uint8>)
/// @generic.instance id=truncate<uint8> template=truncate arguments=(uint8) evaluated=(WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => &truncate.'a exclusive MaybeUninit<uint8>, (WithAccess<&truncate.'a T#6[], "exclusive">, usize) => WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => (&truncate.'a exclusive uint8[], usize) => &truncate.'a exclusive MaybeUninit<uint8>, WithAccess<&truncate.'a T#6[], "exclusive"> => &truncate.'a exclusive uint8[])

declare const index: isize;
/// @type.symbol symbol=index source=index type=isize
/// @resolution.pattern source=index kind=binding target=index

const byte = bytes[index];
/// @type.symbol symbol=byte source=byte type=uint8
/// @resolution.pattern source=byte kind=binding target=byte
/// @resolution.name source=bytes target=bytes
/// @resolution.place source=bytes placement="local" lifetime="managed" access="exclusive"
/// @resolution.access source=bytes root=bytes
/// @resolution.subscript source=bytes[index] type=uint8 kind=call target="index#1(parameters=(isize), arguments=(provided(index) as isize), return=WithAccess<Borrowed<uint8, \"managed\" & \"local\", \"mutable\">, \"exclusive\">)"
/// @generic.instantiation id="index#1<uint8, \"exclusive\">" template=index#1 arguments=(uint8, "exclusive")
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="assumeInitReference<uint8, \"exclusive\">" template=assumeInitReference arguments=(uint8, "exclusive") evaluated=(<assumeInitReference.T, const assumeInitReference.A: Access = "mutable", assumeInitReference.'a>(WithAccess<&assumeInitReference.'a MaybeUninit<assumeInitReference.T>, assumeInitReference.A>) => WithAccess<&assumeInitReference.'a assumeInitReference.T, assumeInitReference.A> => <assumeInitReference.T, const assumeInitReference.A: Access = "mutable", assumeInitReference.'a>(&assumeInitReference.'a exclusive MaybeUninit<uint8>) => &assumeInitReference.'a exclusive uint8)
/// @generic.instance id="elementSlot<uint8, \"exclusive\">" template=elementSlot arguments=(uint8, "exclusive") evaluated=(<elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => <elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(&elementSlot.'a exclusive uint8[], usize) => &elementSlot.'a exclusive MaybeUninit<uint8>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<uint8>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<uint8>>, usize) => &elementSlot.'a exclusive MaybeUninit<uint8>, WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<uint8>>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<uint8>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<uint8>>, usize) => &elementSlot.'a exclusive MaybeUninit<uint8>, WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A> => &elementSlot.'a exclusive uint8[], WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<uint8>>)
/// @generic.instance id="index#1<uint8, \"exclusive\">" template=index#1 arguments=(uint8, "exclusive") evaluated=(<const index.A: Access = "readonly", index#1.'a>(this: WithAccess<&index#1.'a T#6[], index.A>, isize) => WithAccess<&index#1.'a T#6, index.A> => <const index.A: Access = "readonly", index#1.'a>(this: &index#1.'a exclusive uint8[], isize) => &index#1.'a exclusive uint8, WithAccess<&index#1.'a T#6, index.A> => &index#1.'a exclusive uint8, WithAccess<&index#1.'a T#6[], index.A> => &index#1.'a exclusive uint8[], (WithAccess<&index#1.'a MaybeUninit<T#6>, index.A>) => WithAccess<&index#1.'a T#6, index.A> => (&index#1.'a exclusive MaybeUninit<uint8>) => &index#1.'a exclusive uint8, WithAccess<&index#1.'a MaybeUninit<T#6>, index.A> => &index#1.'a exclusive MaybeUninit<uint8>, WithAccess<&index#1.'a T#6, index.A> => &index#1.'a exclusive uint8, (WithAccess<&index#1.'a MaybeUninit<T#6>, index.A>) => WithAccess<&index#1.'a T#6, index.A> => (&index#1.'a exclusive MaybeUninit<uint8>) => &index#1.'a exclusive uint8, WithAccess<&index#1.'a MaybeUninit<T#6>, index.A> => &index#1.'a exclusive MaybeUninit<uint8>, (WithAccess<&index#1.'a T#6[], index.A>, usize) => WithAccess<&index#1.'a MaybeUninit<T#6>, index.A> => (&index#1.'a exclusive uint8[], usize) => &index#1.'a exclusive MaybeUninit<uint8>, WithAccess<&index#1.'a T#6[], index.A> => &index#1.'a exclusive uint8[])
/// @generic.instance id="sliceIndex<MaybeUninit<uint8>, \"exclusive\">" template=sliceIndex arguments=(MaybeUninit<uint8>, "exclusive") evaluated=(<sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(WithAccess<&sliceIndex.'a Slice<sliceIndex.T>, sliceIndex.A>, usize) => WithAccess<&sliceIndex.'a sliceIndex.T, sliceIndex.A> => <sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(&sliceIndex.'a exclusive Slice<MaybeUninit<uint8>>, usize) => &sliceIndex.'a exclusive MaybeUninit<uint8>)
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id=elementPosition<uint8> template=elementPosition arguments=(uint8)
/// @resolution.name source=index target=index
/// @resolution.place source=index placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=index root=index
"#,
    );
}

#[test]
fn test_dynamic_array_subscript_write_selects_index_set() {
    let session = TestSession::single(
        r#"
declare const bytes: uint8[];
declare const index: isize;
bytes[index] = 255;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const bytes: uint8[];
declare const index: isize;
bytes[index] = 255;

=== dir ===
declare const bytes: uint8[];
/// @type.symbol symbol=bytes source=bytes type=uint8[]
/// @resolution.pattern source=bytes kind=binding target=bytes
/// @generic.instance id="initAsPointer<uint8, \"exclusive\">" template=initAsPointer arguments=(uint8, "exclusive") evaluated=(<initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(WithAccess<&initAsPointer.'a MaybeUninit<initAsPointer.T>, initAsPointer.A>) => Raw<initAsPointer.T> => <initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(&initAsPointer.'a exclusive MaybeUninit<uint8>) => Raw<uint8>)
/// @generic.instance id=Array<uint8> template=Array arguments=(uint8)
/// @generic.instance id=MaybeUninit<MaybeUninit<uint8>> template=MaybeUninit arguments=(MaybeUninit<uint8>)
/// @generic.instance id=MaybeUninit<uint8> template=MaybeUninit arguments=(uint8)
/// @generic.instance id=assumeInitDrop#1<uint8> template=assumeInitDrop#1 arguments=(uint8)
/// @generic.instance id=assumeInitDrop<uint8> template=assumeInitDrop arguments=(uint8) evaluated=((WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive">) => Raw<assumeInitDrop.T> => (&assumeInitDrop.'a exclusive MaybeUninit<uint8>) => Raw<uint8>, WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive"> => &assumeInitDrop.'a exclusive MaybeUninit<uint8>)
/// @generic.instance id=clear<uint8> template=clear arguments=(uint8)
/// @generic.instance id=drop<uint8> template=drop arguments=(uint8)
/// @generic.instance id=dropInPlace<uint8> template=dropInPlace arguments=(uint8)
/// @generic.instance id=new<MaybeUninit<uint8>> template=new arguments=(MaybeUninit<uint8>)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<uint8>> template=sliceAssumeInit arguments=(MaybeUninit<uint8>)
/// @generic.instance id=sliceUninit<MaybeUninit<uint8>> template=sliceUninit arguments=(MaybeUninit<uint8>)
/// @generic.instance id=truncate<uint8> template=truncate arguments=(uint8) evaluated=(WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => &truncate.'a exclusive MaybeUninit<uint8>, (WithAccess<&truncate.'a T#6[], "exclusive">, usize) => WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => (&truncate.'a exclusive uint8[], usize) => &truncate.'a exclusive MaybeUninit<uint8>, WithAccess<&truncate.'a T#6[], "exclusive"> => &truncate.'a exclusive uint8[])

declare const index: isize;
/// @type.symbol symbol=index source=index type=isize
/// @resolution.pattern source=index kind=binding target=index

bytes[index] = 255;
/// @resolution.name source=bytes target=bytes
/// @resolution.place source=bytes placement="local" lifetime="managed" access="exclusive"
/// @resolution.access source=bytes root=bytes
/// @resolution.pattern.assign source=bytes[index] kind=place
/// @resolution.assignment source=bytes[index] write="indexSet#1(parameters=(isize, uint8), arguments=(provided(index) as isize, supplied as uint8), return=void)" type=uint8
/// @generic.instantiation id=indexSet#1<uint8> template=indexSet#1 arguments=(uint8)
/// @generic.instance id="Bound<&'bound0 readonly isize>" template=Bound arguments=(&'bound0 readonly isize)
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
/// @generic.instance id="RangeBounds.endBound<RangeBounds<isize>, isize>" template=RangeBounds.endBound arguments=(isize)
/// @generic.instance id="RangeBounds.startBound<RangeBounds<isize>, isize>" template=RangeBounds.startBound arguments=(isize)
/// @generic.instance id="as<uint8, \"exclusive\" | \"mutable\" | \"readonly\">" template=as arguments=(uint8, "exclusive" | "mutable" | "readonly") evaluated=(<as.'a>(this: WithAccess<&as.'a T#7[], A#2>) => WithAccess<&as.'a Slice<T#7>, A#2> => <as.'a>(this: Borrowed<uint8[], as.'a, "exclusive" | "mutable" | "readonly">) => Borrowed<Slice<uint8>, as.'a, "exclusive" | "mutable" | "readonly">)
/// @generic.instance id="assumeInitReference<uint8, \"exclusive\">" template=assumeInitReference arguments=(uint8, "exclusive") evaluated=(<assumeInitReference.T, const assumeInitReference.A: Access = "mutable", assumeInitReference.'a>(WithAccess<&assumeInitReference.'a MaybeUninit<assumeInitReference.T>, assumeInitReference.A>) => WithAccess<&assumeInitReference.'a assumeInitReference.T, assumeInitReference.A> => <assumeInitReference.T, const assumeInitReference.A: Access = "mutable", assumeInitReference.'a>(&assumeInitReference.'a exclusive MaybeUninit<uint8>) => &assumeInitReference.'a exclusive uint8)
/// @generic.instance id="elementSlot<uint8, \"exclusive\">" template=elementSlot arguments=(uint8, "exclusive") evaluated=(<elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => <elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(&elementSlot.'a exclusive uint8[], usize) => &elementSlot.'a exclusive MaybeUninit<uint8>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<uint8>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<uint8>>, usize) => &elementSlot.'a exclusive MaybeUninit<uint8>, WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<uint8>>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<uint8>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<uint8>>, usize) => &elementSlot.'a exclusive MaybeUninit<uint8>, WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A> => &elementSlot.'a exclusive uint8[], WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<uint8>>)
/// @generic.instance id="index#1<uint8, \"exclusive\">" template=index#1 arguments=(uint8, "exclusive") evaluated=(<const index.A: Access = "readonly", index#1.'a>(this: WithAccess<&index#1.'a T#6[], index.A>, isize) => WithAccess<&index#1.'a T#6, index.A> => <const index.A: Access = "readonly", index#1.'a>(this: &index#1.'a exclusive uint8[], isize) => &index#1.'a exclusive uint8, WithAccess<&index#1.'a T#6, index.A> => &index#1.'a exclusive uint8, WithAccess<&index#1.'a T#6[], index.A> => &index#1.'a exclusive uint8[], (WithAccess<&index#1.'a MaybeUninit<T#6>, index.A>) => WithAccess<&index#1.'a T#6, index.A> => (&index#1.'a exclusive MaybeUninit<uint8>) => &index#1.'a exclusive uint8, WithAccess<&index#1.'a MaybeUninit<T#6>, index.A> => &index#1.'a exclusive MaybeUninit<uint8>, WithAccess<&index#1.'a T#6, index.A> => &index#1.'a exclusive uint8, (WithAccess<&index#1.'a MaybeUninit<T#6>, index.A>) => WithAccess<&index#1.'a T#6, index.A> => (&index#1.'a exclusive MaybeUninit<uint8>) => &index#1.'a exclusive uint8, WithAccess<&index#1.'a MaybeUninit<T#6>, index.A> => &index#1.'a exclusive MaybeUninit<uint8>, (WithAccess<&index#1.'a T#6[], index.A>, usize) => WithAccess<&index#1.'a MaybeUninit<T#6>, index.A> => (&index#1.'a exclusive uint8[], usize) => &index#1.'a exclusive MaybeUninit<uint8>, WithAccess<&index#1.'a T#6[], index.A> => &index#1.'a exclusive uint8[])
/// @generic.instance id="index#2<uint8, RangeBounds<isize>, \"exclusive\" | \"mutable\" | \"readonly\">" template=index#2 arguments=(uint8, RangeBounds<isize>, "exclusive" | "mutable" | "readonly") evaluated=(<index#2.'a>(this: WithAccess<&index#2.'a Slice<T#6>, A#2>, R#1) => WithAccess<&index#2.'a Slice<T#6>, A#2> => <index#2.'a>(this: Borrowed<Slice<uint8>, index#2.'a, "exclusive" | "mutable" | "readonly">, RangeBounds<isize>) => Borrowed<Slice<uint8>, index#2.'a, "exclusive" | "mutable" | "readonly">, WithAccess<&index#2.'a Slice<T#6>, A#2> => Borrowed<Slice<uint8>, index#2.'a, "exclusive" | "mutable" | "readonly">, WithAccess<&index#2.'a Slice<T#6>, A#2> => Borrowed<Slice<uint8>, index#2.'a, "exclusive" | "mutable" | "readonly">, (this: WithAccess<&index#2.'a Slice<T#6>, A#2>, usize, usize) => WithAccess<&index#2.'a Slice<T#6>, A#2> => (this: Borrowed<Slice<uint8>, index#2.'a, "exclusive" | "mutable" | "readonly">, usize, usize) => Borrowed<Slice<uint8>, index#2.'a, "exclusive" | "mutable" | "readonly">)
/// @generic.instance id="rangeSpan<uint8, RangeBounds<isize>, \"exclusive\" | \"mutable\" | \"readonly\">" template=rangeSpan arguments=(uint8, RangeBounds<isize>, "exclusive" | "mutable" | "readonly")
/// @generic.instance id="sliceIndex<MaybeUninit<uint8>, \"exclusive\">" template=sliceIndex arguments=(MaybeUninit<uint8>, "exclusive") evaluated=(<sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(WithAccess<&sliceIndex.'a Slice<sliceIndex.T>, sliceIndex.A>, usize) => WithAccess<&sliceIndex.'a sliceIndex.T, sliceIndex.A> => <sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(&sliceIndex.'a exclusive Slice<MaybeUninit<uint8>>, usize) => &sliceIndex.'a exclusive MaybeUninit<uint8>)
/// @generic.instance id="sliceView<uint8, \"exclusive\" | \"mutable\" | \"readonly\">" template=sliceView arguments=(uint8, "exclusive" | "mutable" | "readonly") evaluated=(<sliceView.T, const sliceView.A: Access = "readonly", sliceView.'a>(WithAccess<&sliceView.'a Slice<sliceView.T>, sliceView.A>, usize, usize) => WithAccess<&sliceView.'a Slice<sliceView.T>, sliceView.A> => <sliceView.T, const sliceView.A: Access = "readonly", sliceView.'a>(Borrowed<Slice<uint8>, sliceView.'a, "exclusive" | "mutable" | "readonly">, usize, usize) => Borrowed<Slice<uint8>, sliceView.'a, "exclusive" | "mutable" | "readonly">)
/// @generic.instance id="subslice<uint8, \"exclusive\" | \"mutable\" | \"readonly\">" template=subslice arguments=(uint8, "exclusive" | "mutable" | "readonly") evaluated=(<const subslice.A: Access = "readonly", subslice.'a>(this: WithAccess<&subslice.'a Slice<T#1>, subslice.A>, usize, usize) => WithAccess<&subslice.'a Slice<T#1>, subslice.A> => <const subslice.A: Access = "readonly", subslice.'a>(this: Borrowed<Slice<uint8>, subslice.'a, "exclusive" | "mutable" | "readonly">, usize, usize) => Borrowed<Slice<uint8>, subslice.'a, "exclusive" | "mutable" | "readonly">, WithAccess<&subslice.'a Slice<T#1>, subslice.A> => Borrowed<Slice<uint8>, subslice.'a, "exclusive" | "mutable" | "readonly">, (WithAccess<&subslice.'a Slice<T#1>, subslice.A>, usize, usize) => WithAccess<&subslice.'a Slice<T#1>, subslice.A> => (Borrowed<Slice<uint8>, subslice.'a, "exclusive" | "mutable" | "readonly">, usize, usize) => Borrowed<Slice<uint8>, subslice.'a, "exclusive" | "mutable" | "readonly">, WithAccess<&subslice.'a Slice<T#1>, subslice.A> => Borrowed<Slice<uint8>, subslice.'a, "exclusive" | "mutable" | "readonly">)
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
/// @generic.instance id=RangeBounds<isize> template=RangeBounds arguments=(isize)
/// @generic.instance id=Slice<uint8> template=Slice arguments=(uint8)
/// @generic.instance id=elementPosition<uint8> template=elementPosition arguments=(uint8)
/// @generic.instance id=indexSet#1<uint8> template=indexSet#1 arguments=(uint8) evaluated=(WithAccess<&indexSet#1.'a T#6, "exclusive"> => &indexSet#1.'a exclusive uint8, (this: WithAccess<&indexSet#1.'a T#6[], "exclusive">, isize) => WithAccess<&indexSet#1.'a T#6, "exclusive"> => (this: &indexSet#1.'a exclusive uint8[], isize) => &indexSet#1.'a exclusive uint8, <const index.A: Access = "readonly", index#1.'a>(this: WithAccess<&index#1.'a T#6[], index.A>, isize) => WithAccess<&index#1.'a T#6, index.A> & <index#2.'a>(this: WithAccess<&index#2.'a T#6[], "readonly" | "exclusive" | "mutable">, RangeBounds<isize>) => WithAccess<&index#2.'a Slice<T#6>, "readonly" | "exclusive" | "mutable"> => <const index.A: Access = "readonly", index#1.'a>(this: WithAccess<&index#1.'a uint8[], index.A>, isize) => WithAccess<&index#1.'a uint8, index.A> & <index#2.'a>(this: Borrowed<uint8[], index#2.'a, "readonly" | "exclusive" | "mutable">, RangeBounds<isize>) => Borrowed<Slice<uint8>, index#2.'a, "readonly" | "exclusive" | "mutable">, <index#2.'a>(this: WithAccess<&index#2.'a T#6[], "readonly" | "mutable" | "exclusive">, RangeBounds<isize>) => WithAccess<&index#2.'a Slice<T#6>, "readonly" | "mutable" | "exclusive"> => <index#2.'a>(this: Borrowed<uint8[], index#2.'a, "readonly" | "mutable" | "exclusive">, RangeBounds<isize>) => Borrowed<Slice<uint8>, index#2.'a, "readonly" | "mutable" | "exclusive">, <const index.A: Access = "readonly", index#1.'a>(this: WithAccess<&index#1.'a T#6[], index.A>, isize) => WithAccess<&index#1.'a T#6, index.A> & <index#2.'a>(this: WithAccess<&index#2.'a T#6[], "readonly" | "mutable" | "exclusive">, RangeBounds<isize>) => WithAccess<&index#2.'a Slice<T#6>, "readonly" | "mutable" | "exclusive"> => <const index.A: Access = "readonly", index#1.'a>(this: WithAccess<&index#1.'a uint8[], index.A>, isize) => WithAccess<&index#1.'a uint8, index.A> & <index#2.'a>(this: Borrowed<uint8[], index#2.'a, "readonly" | "mutable" | "exclusive">, RangeBounds<isize>) => Borrowed<Slice<uint8>, index#2.'a, "readonly" | "mutable" | "exclusive">)
/// @generic.instance id=size<uint8> template=size arguments=(uint8)
/// @generic.instance id=sliceLength<uint8> template=sliceLength arguments=(uint8)
/// @generic.instance id=symbol2<uint8> template=symbol2 arguments=(uint8)
/// @resolution.name source=index target=index
/// @resolution.place source=index placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=index root=index
"#,
    );
}

#[test]
fn test_dynamic_array_compound_subscript_resolves_read_and_write() {
    let session = TestSession::single(
        r#"
declare const bytes: uint8[];
declare const index: isize;
bytes[index] += 1;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const bytes: uint8[];
declare const index: isize;
bytes[index] += 1;

=== dir ===
declare const bytes: uint8[];
/// @type.symbol symbol=bytes source=bytes type=uint8[]
/// @resolution.pattern source=bytes kind=binding target=bytes
/// @generic.instance id="initAsPointer<uint8, \"exclusive\">" template=initAsPointer arguments=(uint8, "exclusive") evaluated=(<initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(WithAccess<&initAsPointer.'a MaybeUninit<initAsPointer.T>, initAsPointer.A>) => Raw<initAsPointer.T> => <initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(&initAsPointer.'a exclusive MaybeUninit<uint8>) => Raw<uint8>)
/// @generic.instance id=Array<uint8> template=Array arguments=(uint8)
/// @generic.instance id=MaybeUninit<MaybeUninit<uint8>> template=MaybeUninit arguments=(MaybeUninit<uint8>)
/// @generic.instance id=MaybeUninit<uint8> template=MaybeUninit arguments=(uint8)
/// @generic.instance id=assumeInitDrop#1<uint8> template=assumeInitDrop#1 arguments=(uint8)
/// @generic.instance id=assumeInitDrop<uint8> template=assumeInitDrop arguments=(uint8) evaluated=((WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive">) => Raw<assumeInitDrop.T> => (&assumeInitDrop.'a exclusive MaybeUninit<uint8>) => Raw<uint8>, WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive"> => &assumeInitDrop.'a exclusive MaybeUninit<uint8>)
/// @generic.instance id=clear<uint8> template=clear arguments=(uint8)
/// @generic.instance id=drop<uint8> template=drop arguments=(uint8)
/// @generic.instance id=dropInPlace<uint8> template=dropInPlace arguments=(uint8)
/// @generic.instance id=new<MaybeUninit<uint8>> template=new arguments=(MaybeUninit<uint8>)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<uint8>> template=sliceAssumeInit arguments=(MaybeUninit<uint8>)
/// @generic.instance id=sliceUninit<MaybeUninit<uint8>> template=sliceUninit arguments=(MaybeUninit<uint8>)
/// @generic.instance id=truncate<uint8> template=truncate arguments=(uint8) evaluated=(WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => &truncate.'a exclusive MaybeUninit<uint8>, (WithAccess<&truncate.'a T#6[], "exclusive">, usize) => WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => (&truncate.'a exclusive uint8[], usize) => &truncate.'a exclusive MaybeUninit<uint8>, WithAccess<&truncate.'a T#6[], "exclusive"> => &truncate.'a exclusive uint8[])

declare const index: isize;
/// @type.symbol symbol=index source=index type=isize
/// @resolution.pattern source=index kind=binding target=index

bytes[index] += 1;
/// @resolution.name source=bytes target=bytes
/// @resolution.operator source="bytes[index] += 1" type=uint8 operator="+" kind=builtin operands=[bytes[index] as uint8 families=(integer), 1 as uint8 families=(integer)]
/// @resolution.place source=bytes placement="local" lifetime="managed" access="exclusive"
/// @resolution.access source=bytes root=bytes
/// @resolution.pattern.assign source=bytes[index] kind=place
/// @resolution.assignment source=bytes[index] read="index#1(parameters=(isize), arguments=(provided(index) as isize), return=WithAccess<Borrowed<uint8, \"managed\" & \"local\", \"mutable\">, \"exclusive\">)" write="indexSet#1(parameters=(isize, uint8), arguments=(provided(index) as isize, supplied as uint8), return=void)" type=uint8
/// @generic.instantiation id="index#1<uint8, \"exclusive\">" template=index#1 arguments=(uint8, "exclusive")
/// @generic.instantiation id=indexSet#1<uint8> template=indexSet#1 arguments=(uint8)
/// @generic.instance id="Bound<&'bound0 readonly isize>" template=Bound arguments=(&'bound0 readonly isize)
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
/// @generic.instance id="RangeBounds.endBound<RangeBounds<isize>, isize>" template=RangeBounds.endBound arguments=(isize)
/// @generic.instance id="RangeBounds.startBound<RangeBounds<isize>, isize>" template=RangeBounds.startBound arguments=(isize)
/// @generic.instance id="as<uint8, \"exclusive\" | \"mutable\" | \"readonly\">" template=as arguments=(uint8, "exclusive" | "mutable" | "readonly") evaluated=(<as.'a>(this: WithAccess<&as.'a T#7[], A#2>) => WithAccess<&as.'a Slice<T#7>, A#2> => <as.'a>(this: Borrowed<uint8[], as.'a, "exclusive" | "mutable" | "readonly">) => Borrowed<Slice<uint8>, as.'a, "exclusive" | "mutable" | "readonly">)
/// @generic.instance id="assumeInitReference<uint8, \"exclusive\">" template=assumeInitReference arguments=(uint8, "exclusive") evaluated=(<assumeInitReference.T, const assumeInitReference.A: Access = "mutable", assumeInitReference.'a>(WithAccess<&assumeInitReference.'a MaybeUninit<assumeInitReference.T>, assumeInitReference.A>) => WithAccess<&assumeInitReference.'a assumeInitReference.T, assumeInitReference.A> => <assumeInitReference.T, const assumeInitReference.A: Access = "mutable", assumeInitReference.'a>(&assumeInitReference.'a exclusive MaybeUninit<uint8>) => &assumeInitReference.'a exclusive uint8)
/// @generic.instance id="elementSlot<uint8, \"exclusive\">" template=elementSlot arguments=(uint8, "exclusive") evaluated=(<elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => <elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(&elementSlot.'a exclusive uint8[], usize) => &elementSlot.'a exclusive MaybeUninit<uint8>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<uint8>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<uint8>>, usize) => &elementSlot.'a exclusive MaybeUninit<uint8>, WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<uint8>>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<uint8>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<uint8>>, usize) => &elementSlot.'a exclusive MaybeUninit<uint8>, WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A> => &elementSlot.'a exclusive uint8[], WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<uint8>>)
/// @generic.instance id="index#1<uint8, \"exclusive\">" template=index#1 arguments=(uint8, "exclusive") evaluated=(<const index.A: Access = "readonly", index#1.'a>(this: WithAccess<&index#1.'a T#6[], index.A>, isize) => WithAccess<&index#1.'a T#6, index.A> => <const index.A: Access = "readonly", index#1.'a>(this: &index#1.'a exclusive uint8[], isize) => &index#1.'a exclusive uint8, WithAccess<&index#1.'a T#6, index.A> => &index#1.'a exclusive uint8, WithAccess<&index#1.'a T#6[], index.A> => &index#1.'a exclusive uint8[], (WithAccess<&index#1.'a MaybeUninit<T#6>, index.A>) => WithAccess<&index#1.'a T#6, index.A> => (&index#1.'a exclusive MaybeUninit<uint8>) => &index#1.'a exclusive uint8, WithAccess<&index#1.'a MaybeUninit<T#6>, index.A> => &index#1.'a exclusive MaybeUninit<uint8>, WithAccess<&index#1.'a T#6, index.A> => &index#1.'a exclusive uint8, (WithAccess<&index#1.'a MaybeUninit<T#6>, index.A>) => WithAccess<&index#1.'a T#6, index.A> => (&index#1.'a exclusive MaybeUninit<uint8>) => &index#1.'a exclusive uint8, WithAccess<&index#1.'a MaybeUninit<T#6>, index.A> => &index#1.'a exclusive MaybeUninit<uint8>, (WithAccess<&index#1.'a T#6[], index.A>, usize) => WithAccess<&index#1.'a MaybeUninit<T#6>, index.A> => (&index#1.'a exclusive uint8[], usize) => &index#1.'a exclusive MaybeUninit<uint8>, WithAccess<&index#1.'a T#6[], index.A> => &index#1.'a exclusive uint8[])
/// @generic.instance id="index#2<uint8, RangeBounds<isize>, \"exclusive\" | \"mutable\" | \"readonly\">" template=index#2 arguments=(uint8, RangeBounds<isize>, "exclusive" | "mutable" | "readonly") evaluated=(<index#2.'a>(this: WithAccess<&index#2.'a Slice<T#6>, A#2>, R#1) => WithAccess<&index#2.'a Slice<T#6>, A#2> => <index#2.'a>(this: Borrowed<Slice<uint8>, index#2.'a, "exclusive" | "mutable" | "readonly">, RangeBounds<isize>) => Borrowed<Slice<uint8>, index#2.'a, "exclusive" | "mutable" | "readonly">, WithAccess<&index#2.'a Slice<T#6>, A#2> => Borrowed<Slice<uint8>, index#2.'a, "exclusive" | "mutable" | "readonly">, WithAccess<&index#2.'a Slice<T#6>, A#2> => Borrowed<Slice<uint8>, index#2.'a, "exclusive" | "mutable" | "readonly">, (this: WithAccess<&index#2.'a Slice<T#6>, A#2>, usize, usize) => WithAccess<&index#2.'a Slice<T#6>, A#2> => (this: Borrowed<Slice<uint8>, index#2.'a, "exclusive" | "mutable" | "readonly">, usize, usize) => Borrowed<Slice<uint8>, index#2.'a, "exclusive" | "mutable" | "readonly">)
/// @generic.instance id="rangeSpan<uint8, RangeBounds<isize>, \"exclusive\" | \"mutable\" | \"readonly\">" template=rangeSpan arguments=(uint8, RangeBounds<isize>, "exclusive" | "mutable" | "readonly")
/// @generic.instance id="sliceIndex<MaybeUninit<uint8>, \"exclusive\">" template=sliceIndex arguments=(MaybeUninit<uint8>, "exclusive") evaluated=(<sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(WithAccess<&sliceIndex.'a Slice<sliceIndex.T>, sliceIndex.A>, usize) => WithAccess<&sliceIndex.'a sliceIndex.T, sliceIndex.A> => <sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(&sliceIndex.'a exclusive Slice<MaybeUninit<uint8>>, usize) => &sliceIndex.'a exclusive MaybeUninit<uint8>)
/// @generic.instance id="sliceView<uint8, \"exclusive\" | \"mutable\" | \"readonly\">" template=sliceView arguments=(uint8, "exclusive" | "mutable" | "readonly") evaluated=(<sliceView.T, const sliceView.A: Access = "readonly", sliceView.'a>(WithAccess<&sliceView.'a Slice<sliceView.T>, sliceView.A>, usize, usize) => WithAccess<&sliceView.'a Slice<sliceView.T>, sliceView.A> => <sliceView.T, const sliceView.A: Access = "readonly", sliceView.'a>(Borrowed<Slice<uint8>, sliceView.'a, "exclusive" | "mutable" | "readonly">, usize, usize) => Borrowed<Slice<uint8>, sliceView.'a, "exclusive" | "mutable" | "readonly">)
/// @generic.instance id="subslice<uint8, \"exclusive\" | \"mutable\" | \"readonly\">" template=subslice arguments=(uint8, "exclusive" | "mutable" | "readonly") evaluated=(<const subslice.A: Access = "readonly", subslice.'a>(this: WithAccess<&subslice.'a Slice<T#1>, subslice.A>, usize, usize) => WithAccess<&subslice.'a Slice<T#1>, subslice.A> => <const subslice.A: Access = "readonly", subslice.'a>(this: Borrowed<Slice<uint8>, subslice.'a, "exclusive" | "mutable" | "readonly">, usize, usize) => Borrowed<Slice<uint8>, subslice.'a, "exclusive" | "mutable" | "readonly">, WithAccess<&subslice.'a Slice<T#1>, subslice.A> => Borrowed<Slice<uint8>, subslice.'a, "exclusive" | "mutable" | "readonly">, (WithAccess<&subslice.'a Slice<T#1>, subslice.A>, usize, usize) => WithAccess<&subslice.'a Slice<T#1>, subslice.A> => (Borrowed<Slice<uint8>, subslice.'a, "exclusive" | "mutable" | "readonly">, usize, usize) => Borrowed<Slice<uint8>, subslice.'a, "exclusive" | "mutable" | "readonly">, WithAccess<&subslice.'a Slice<T#1>, subslice.A> => Borrowed<Slice<uint8>, subslice.'a, "exclusive" | "mutable" | "readonly">)
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
/// @generic.instance id=RangeBounds<isize> template=RangeBounds arguments=(isize)
/// @generic.instance id=Slice<uint8> template=Slice arguments=(uint8)
/// @generic.instance id=elementPosition<uint8> template=elementPosition arguments=(uint8)
/// @generic.instance id=indexSet#1<uint8> template=indexSet#1 arguments=(uint8) evaluated=(WithAccess<&indexSet#1.'a T#6, "exclusive"> => &indexSet#1.'a exclusive uint8, (this: WithAccess<&indexSet#1.'a T#6[], "exclusive">, isize) => WithAccess<&indexSet#1.'a T#6, "exclusive"> => (this: &indexSet#1.'a exclusive uint8[], isize) => &indexSet#1.'a exclusive uint8, <const index.A: Access = "readonly", index#1.'a>(this: WithAccess<&index#1.'a T#6[], index.A>, isize) => WithAccess<&index#1.'a T#6, index.A> & <index#2.'a>(this: WithAccess<&index#2.'a T#6[], "readonly" | "exclusive" | "mutable">, RangeBounds<isize>) => WithAccess<&index#2.'a Slice<T#6>, "readonly" | "exclusive" | "mutable"> => <const index.A: Access = "readonly", index#1.'a>(this: WithAccess<&index#1.'a uint8[], index.A>, isize) => WithAccess<&index#1.'a uint8, index.A> & <index#2.'a>(this: Borrowed<uint8[], index#2.'a, "readonly" | "exclusive" | "mutable">, RangeBounds<isize>) => Borrowed<Slice<uint8>, index#2.'a, "readonly" | "exclusive" | "mutable">, <index#2.'a>(this: WithAccess<&index#2.'a T#6[], "readonly" | "mutable" | "exclusive">, RangeBounds<isize>) => WithAccess<&index#2.'a Slice<T#6>, "readonly" | "mutable" | "exclusive"> => <index#2.'a>(this: Borrowed<uint8[], index#2.'a, "readonly" | "mutable" | "exclusive">, RangeBounds<isize>) => Borrowed<Slice<uint8>, index#2.'a, "readonly" | "mutable" | "exclusive">, <const index.A: Access = "readonly", index#1.'a>(this: WithAccess<&index#1.'a T#6[], index.A>, isize) => WithAccess<&index#1.'a T#6, index.A> & <index#2.'a>(this: WithAccess<&index#2.'a T#6[], "readonly" | "mutable" | "exclusive">, RangeBounds<isize>) => WithAccess<&index#2.'a Slice<T#6>, "readonly" | "mutable" | "exclusive"> => <const index.A: Access = "readonly", index#1.'a>(this: WithAccess<&index#1.'a uint8[], index.A>, isize) => WithAccess<&index#1.'a uint8, index.A> & <index#2.'a>(this: Borrowed<uint8[], index#2.'a, "readonly" | "mutable" | "exclusive">, RangeBounds<isize>) => Borrowed<Slice<uint8>, index#2.'a, "readonly" | "mutable" | "exclusive">)
/// @generic.instance id=size<uint8> template=size arguments=(uint8)
/// @generic.instance id=sliceLength<uint8> template=sliceLength arguments=(uint8)
/// @generic.instance id=symbol2<uint8> template=symbol2 arguments=(uint8)
/// @resolution.name source=index target=index
/// @resolution.place source=index placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=index root=index
"#,
    );
}

/// A scalar written into an `unknown` element erases behind the element's handle.
#[test]
fn test_write_a_scalar_into_an_unknown_array_element() {
    let session = TestSession::single(
        r#"
function copy(target: &exclusive unknown[], source: &readonly int32[]): void {
    for (let index: isize = 0; index < source.length; index++) {
        target[index] = source[index];
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function copy<'a, 'b>(target: &'a exclusive unknown[], source: &'b readonly int32[]): void {
    for (let index: isize = 0; index < source.length; index++) {
        target[index] = source[index] as Managed<unknown, 'a>;
    }
}

=== dir ===
function copy(target: &exclusive unknown[], source: &readonly int32[]): void {
/// @generic.template symbol=copy parameters=('a, 'b)
/// @type.symbol symbol=copy type=<copy.'a, copy.'b>(&copy.'a exclusive unknown[], &copy.'b readonly int32[]) => void
/// @type.symbol symbol=copy.target source="target: &exclusive unknown[]" type=&copy.'a exclusive unknown[]
/// @type.symbol symbol=copy.source source="source: &readonly int32[]" type=&copy.'b readonly int32[]

    for (let index: isize = 0; index < source.length; index++) {
    /// @type.symbol symbol=copy.index source=index type=isize
    /// @resolution.pattern source=index kind=binding target=copy.index
    /// @resolution.name source=index target=copy.index
    /// @resolution.operator source="index < source.length" type=boolean operator="<" kind=builtin operands=[index as isize families=(integer), source.length as isize families=(integer)]
    /// @resolution.place source=index placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=index root=copy.index
    /// @resolution.name source=source target=copy.source
    /// @resolution.member source=source.length receiver=&copy.'b readonly int32[] type=isize kind=call target="length(parameters=(), arguments=(), return=isize)"
    /// @resolution.place source=source placement=copy.'b lifetime=copy.'b access="readonly"
    /// @resolution.access source=source root=copy.source
    /// @generic.instantiation id=length<int32> template=length arguments=(int32)
    /// @resolution.name source=index target=copy.index
    /// @resolution.assignment source=index read=binding(copy.index) write=binding(copy.index) type=isize
    /// @resolution.access source=index root=copy.index
    /// @resolution.operator source=index++ type=isize operator="++" kind=builtin operands=[index as isize families=(integer)]

        target[index] = source[index];
        /// @resolution.name source=target target=copy.target
        /// @resolution.place source=target placement=copy.'a lifetime=copy.'a access="exclusive"
        /// @resolution.access source=target root=copy.target
        /// @resolution.pattern.assign source=target[index] kind=place
        /// @resolution.assignment source=target[index] write="indexSet#1(parameters=(isize, Managed<unknown, copy.'a>), arguments=(provided(index) as isize, supplied as Managed<unknown, copy.'a>), return=void)" type=Managed<unknown, copy.'a>
        /// @generic.instantiation id=indexSet#1<unknown> template=indexSet#1 arguments=(unknown)
        /// @resolution.name source=index target=copy.index
        /// @resolution.place source=index placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=index root=copy.index
        /// @resolution.name source=source target=copy.source
        /// @resolution.place source=source placement=copy.'b lifetime=copy.'b access="readonly"
        /// @resolution.access source=source root=copy.source
        /// @resolution.place source=source[index] placement=copy.'b lifetime=copy.'b access="readonly"
        /// @resolution.subscript source=source[index] type=int32 kind=call target="index#1(parameters=(isize), arguments=(provided(index) as isize), return=WithAccess<&copy.'b int32, \"readonly\">)"
        /// @generic.instantiation id="index#1<int32, \"readonly\">" template=index#1 arguments=(int32, "readonly")
        /// @resolution.name source=index target=copy.index
        /// @resolution.place source=index placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=index root=copy.index

    }
}
"#,
        r#"
"#,
    );
}
