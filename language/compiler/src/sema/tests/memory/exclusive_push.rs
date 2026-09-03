use crate::tests::{DirRows, TestSession};

/// An exclusive borrow of an array binding reaches the array's freeing methods.
#[test]
fn test_push_through_an_exclusive_borrow_of_an_array_binding() {
    let session = TestSession::single(
        r#"
function grow(): void {
    let x = [1, 2];
    let x1 = &exclusive x;
    x1.push(3);
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function grow(): void {
    let x: int64[] = [1, 2];
    let x1: &'frame exclusive int64[] = &exclusive x;
    x1.push<int64>(3);
}

=== dir ===
function grow(): void {
/// @type.symbol symbol=grow type=() => void

    let x = [1, 2];
    /// @type.symbol symbol=grow.x source=x type=int64[]
    /// @resolution.pattern source=x kind=binding target=grow.x
    /// @generic.instance id="initAsPointer<int64, \"exclusive\">" template=initAsPointer arguments=(int64, "exclusive") evaluated=(<initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(WithAccess<&initAsPointer.'a MaybeUninit<initAsPointer.T>, initAsPointer.A>) => Raw<initAsPointer.T> => <initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(&initAsPointer.'a exclusive MaybeUninit<int64>) => Raw<int64>)
    /// @generic.instance id=Array<int64> template=Array arguments=(int64)
    /// @generic.instance id=MaybeUninit<MaybeUninit<int64>> template=MaybeUninit arguments=(MaybeUninit<int64>)
    /// @generic.instance id=MaybeUninit<int64> template=MaybeUninit arguments=(int64)
    /// @generic.instance id=assumeInitDrop#1<int64> template=assumeInitDrop#1 arguments=(int64)
    /// @generic.instance id=assumeInitDrop<int64> template=assumeInitDrop arguments=(int64) evaluated=((WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive">) => Raw<assumeInitDrop.T> => (&assumeInitDrop.'a exclusive MaybeUninit<int64>) => Raw<int64>, WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive"> => &assumeInitDrop.'a exclusive MaybeUninit<int64>)
    /// @generic.instance id=clear<int64> template=clear arguments=(int64)
    /// @generic.instance id=drop<int64> template=drop arguments=(int64)
    /// @generic.instance id=dropInPlace<int64> template=dropInPlace arguments=(int64)
    /// @generic.instance id=new<MaybeUninit<int64>> template=new arguments=(MaybeUninit<int64>)
    /// @generic.instance id=sliceAssumeInit<MaybeUninit<int64>> template=sliceAssumeInit arguments=(MaybeUninit<int64>)
    /// @generic.instance id=sliceUninit<MaybeUninit<int64>> template=sliceUninit arguments=(MaybeUninit<int64>)
    /// @generic.instance id=truncate<int64> template=truncate arguments=(int64) evaluated=(WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => &truncate.'a exclusive MaybeUninit<int64>, (WithAccess<&truncate.'a T#6[], "exclusive">, usize) => WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => (&truncate.'a exclusive int64[], usize) => &truncate.'a exclusive MaybeUninit<int64>, WithAccess<&truncate.'a T#6[], "exclusive"> => &truncate.'a exclusive int64[])
    /// @resolution.call source=[1, 2] parameters=(^Slice<arrayFromOwnedSlice.T>) arguments=(rest(1, 2) as int64) return=int64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int64>

    let x1 = &exclusive x;
    /// @type.symbol symbol=grow.x1 source=x1 type=&'frame exclusive int64[]
    /// @resolution.pattern source=x1 kind=binding target=grow.x1
    /// @resolution.name source=x target=grow.x
    /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=x root=grow.x

    x1.push(3);
    /// @resolution.name source=x1 target=grow.x1
    /// @resolution.member source=x1.push receiver=&'frame exclusive int64[] type=<push.'a>(this: &push.'a exclusive int64[], ...int64[]) => isize kind=symbol target_receiver=&'frame exclusive int64[] target=push
    /// @resolution.call source=x1.push(3) parameters=(int64[]) arguments=(rest(3) pack=arrayFromOwnedSlice as int64) return=isize kind=symbol target=push receiver=&'frame exclusive int64[] instance=Array<int64>.<extension#6>.push
    /// @resolution.place source=x1 placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=x1 root=grow.x1
    /// @generic.instantiation id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
    /// @generic.instantiation id=push<int64> template=push arguments=(int64)
    /// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
    /// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
    /// @generic.instance id="elementSlot<int64, \"exclusive\">" template=elementSlot arguments=(int64, "exclusive") evaluated=(<elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => <elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(&elementSlot.'a exclusive int64[], usize) => &elementSlot.'a exclusive MaybeUninit<int64>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<int64>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<int64>>, usize) => &elementSlot.'a exclusive MaybeUninit<int64>, WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<int64>>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<int64>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<int64>>, usize) => &elementSlot.'a exclusive MaybeUninit<int64>, WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A> => &elementSlot.'a exclusive int64[], WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<int64>>)
    /// @generic.instance id="sliceIndex<MaybeUninit<int64>, \"exclusive\">" template=sliceIndex arguments=(MaybeUninit<int64>, "exclusive") evaluated=(<sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(WithAccess<&sliceIndex.'a Slice<sliceIndex.T>, sliceIndex.A>, usize) => WithAccess<&sliceIndex.'a sliceIndex.T, sliceIndex.A> => <sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(&sliceIndex.'a exclusive Slice<MaybeUninit<int64>>, usize) => &sliceIndex.'a exclusive MaybeUninit<int64>)
    /// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
    /// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
    /// @generic.instance id=Slice<int64> template=Slice arguments=(int64)
    /// @generic.instance id=append<int64> template=append arguments=(int64) evaluated=(WithAccess<&append.'b MaybeUninit<T#6>, "exclusive"> => &append.'b exclusive MaybeUninit<int64>, (WithAccess<&append.'b T#6[], "exclusive">, usize) => WithAccess<&append.'b MaybeUninit<T#6>, "exclusive"> => (&append.'b exclusive int64[], usize) => &append.'b exclusive MaybeUninit<int64>, WithAccess<&append.'b T#6[], "exclusive"> => &append.'b exclusive int64[], WithAccess<&append.'a MaybeUninit<T#6>, "exclusive"> => &append.'a exclusive MaybeUninit<int64>, (WithAccess<&append.'a T#6[], "exclusive">, usize) => WithAccess<&append.'a MaybeUninit<T#6>, "exclusive"> => (&append.'a exclusive int64[], usize) => &append.'a exclusive MaybeUninit<int64>, WithAccess<&append.'a T#6[], "exclusive"> => &append.'a exclusive int64[])
    /// @generic.instance id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
    /// @generic.instance id=assumeInitRead#1<int64> template=assumeInitRead#1 arguments=(int64)
    /// @generic.instance id=assumeInitRead<int64> template=assumeInitRead arguments=(int64)
    /// @generic.instance id=fromOwnedSlice<int64> template=fromOwnedSlice arguments=(int64)
    /// @generic.instance id=initWrite<int64> template=initWrite arguments=(int64)
    /// @generic.instance id=intoUninit<int64> template=intoUninit arguments=(int64)
    /// @generic.instance id=push<int64> template=push arguments=(int64)
    /// @generic.instance id=reserve<int64> template=reserve arguments=(int64) evaluated=(WithAccess<&reserve.'a MaybeUninit<T#6>, "exclusive"> => &reserve.'a exclusive MaybeUninit<int64>, (WithAccess<&reserve.'a T#6[], "exclusive">, usize) => WithAccess<&reserve.'a MaybeUninit<T#6>, "exclusive"> => (&reserve.'a exclusive int64[], usize) => &reserve.'a exclusive MaybeUninit<int64>, WithAccess<&reserve.'a T#6[], "exclusive"> => &reserve.'a exclusive int64[], (WithAccess<&'frame Slice<MaybeUninit<T#6>>, "exclusive">, usize) => WithAccess<&'frame MaybeUninit<T#6>, "exclusive"> => (&'frame exclusive Slice<MaybeUninit<int64>>, usize) => &'frame exclusive MaybeUninit<int64>, WithAccess<&'frame Slice<MaybeUninit<T#6>>, "exclusive"> => &'frame exclusive Slice<MaybeUninit<int64>>, (WithAccess<&'frame Slice<MaybeUninit<T#6>>, "exclusive">, usize) => WithAccess<&'frame MaybeUninit<T#6>, "exclusive"> => (&'frame exclusive Slice<MaybeUninit<int64>>, usize) => &'frame exclusive MaybeUninit<int64>, WithAccess<&'frame Slice<MaybeUninit<T#6>>, "exclusive"> => &'frame exclusive Slice<MaybeUninit<int64>>)
    /// @generic.instance id=size<int64> template=size arguments=(int64)
    /// @generic.instance id=sliceIntoUninit<int64> template=sliceIntoUninit arguments=(int64)
    /// @generic.instance id=sliceLength<int64> template=sliceLength arguments=(int64)
    /// @generic.instance id=sliceUninit<int64> template=sliceUninit arguments=(int64)
    /// @generic.instance id=uninit<int64> template=uninit arguments=(int64)
    /// @generic.instance id=write<int64> template=write arguments=(int64)

}
"#,
    );
}
