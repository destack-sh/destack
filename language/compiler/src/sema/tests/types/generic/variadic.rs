use crate::tests::{DirRows, TestSession};

/// A variadic generic pack spreads into call arguments.
#[test]
fn test_spread_a_variadic_generic_into_call_arguments() {
    let session = TestSession::single(
        r#"
newtype Axis = intrinsic;

newtype Sharding<...Axes: Axis[]> = intrinsic;

newtype Grid<T, P> = intrinsic;

declare function mesh<T, ...Axes: Axis[]>(
    grid: &readonly Grid<T, Sharding<...Axes>>,
): int32;

export extension<T, ...Axes: Axis[]> of Grid<T, Sharding<...Axes>> {
    /// Return the mesh id.
    get mesh(): int32 {
        return mesh<T, ...Axes>(this);
    }
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype Axis = intrinsic;

newtype Sharding<in out ...Axes: Axis[]> = intrinsic;

newtype Grid<in out T, in out P> = intrinsic;

declare function mesh<T, ...Axes: Axis[], 'a>(grid: &'a readonly Grid<T, Sharding<Axes>>): int32;

export extension<T, ...Axes: Axis[]> of Grid<T, Sharding<...Axes>> {
    /// Return the mesh id.
    get mesh(): int32 {
        return mesh<T, ...Axes>(this);
    }
}

=== dir ===
newtype Axis = intrinsic;
/// @type.symbol symbol=Axis source="newtype Axis = intrinsic" type=Axis
/// @definition.newtype symbol=Axis source="newtype Axis = intrinsic" backing=intrinsic constructors=[(intrinsic) => Axis]

newtype Sharding<...Axes: Axis[]> = intrinsic;
/// @generic.template symbol=Sharding parameters=(in out ...Axes#1: Axis[])
/// @type.symbol symbol=Sharding source="newtype Sharding<...Axes: Axis[]> = intrinsic" type=Sharding
/// @definition.newtype symbol=Sharding source="newtype Sharding<...Axes: Axis[]> = intrinsic" template=(in out ...Axes#1: Axis[]) backing=intrinsic constructors=[<...Axes#1: Axis[]>(intrinsic) => Sharding<Axes#1>]
/// @type.symbol symbol=Sharding.Axes source="...Axes: Axis[]" type=Axes#1
/// @resolution.name source=Axis target=Axis
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="elementSlot<Axis, \"exclusive\">" template=elementSlot arguments=(Axis, "exclusive") evaluated=(<elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => <elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(&elementSlot.'a exclusive Axis[], usize) => &elementSlot.'a exclusive MaybeUninit<Axis>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<Axis>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<Axis>>, usize) => &elementSlot.'a exclusive MaybeUninit<Axis>, WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<Axis>>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<Axis>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<Axis>>, usize) => &elementSlot.'a exclusive MaybeUninit<Axis>, WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A> => &elementSlot.'a exclusive Axis[], WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<Axis>>)
/// @generic.instance id="initAsPointer<Axis, \"exclusive\">" template=initAsPointer arguments=(Axis, "exclusive") evaluated=(<initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(WithAccess<&initAsPointer.'a MaybeUninit<initAsPointer.T>, initAsPointer.A>) => Raw<initAsPointer.T> => <initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(&initAsPointer.'a exclusive MaybeUninit<Axis>) => Raw<Axis>)
/// @generic.instance id="sliceIndex<MaybeUninit<Axis>, \"exclusive\">" template=sliceIndex arguments=(MaybeUninit<Axis>, "exclusive") evaluated=(<sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(WithAccess<&sliceIndex.'a Slice<sliceIndex.T>, sliceIndex.A>, usize) => WithAccess<&sliceIndex.'a sliceIndex.T, sliceIndex.A> => <sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(&sliceIndex.'a exclusive Slice<MaybeUninit<Axis>>, usize) => &sliceIndex.'a exclusive MaybeUninit<Axis>)
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id=Array<Axis> template=Array arguments=(Axis)
/// @generic.instance id=MaybeUninit<Axis> template=MaybeUninit arguments=(Axis)
/// @generic.instance id=MaybeUninit<MaybeUninit<Axis>> template=MaybeUninit arguments=(MaybeUninit<Axis>)
/// @generic.instance id=assumeInitDrop#1<Axis> template=assumeInitDrop#1 arguments=(Axis)
/// @generic.instance id=assumeInitDrop<Axis> template=assumeInitDrop arguments=(Axis) evaluated=((WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive">) => Raw<assumeInitDrop.T> => (&assumeInitDrop.'a exclusive MaybeUninit<Axis>) => Raw<Axis>, WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive"> => &assumeInitDrop.'a exclusive MaybeUninit<Axis>)
/// @generic.instance id=clear<Axis> template=clear arguments=(Axis)
/// @generic.instance id=drop<Axis> template=drop arguments=(Axis)
/// @generic.instance id=dropInPlace<Axis> template=dropInPlace arguments=(Axis)
/// @generic.instance id=new<MaybeUninit<Axis>> template=new arguments=(MaybeUninit<Axis>)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<Axis>> template=sliceAssumeInit arguments=(MaybeUninit<Axis>)
/// @generic.instance id=sliceUninit<MaybeUninit<Axis>> template=sliceUninit arguments=(MaybeUninit<Axis>)
/// @generic.instance id=truncate<Axis> template=truncate arguments=(Axis) evaluated=(WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => &truncate.'a exclusive MaybeUninit<Axis>, (WithAccess<&truncate.'a T#6[], "exclusive">, usize) => WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => (&truncate.'a exclusive Axis[], usize) => &truncate.'a exclusive MaybeUninit<Axis>, WithAccess<&truncate.'a T#6[], "exclusive"> => &truncate.'a exclusive Axis[])

newtype Grid<T, P> = intrinsic;
/// @generic.template symbol=Grid parameters=(in out T#1, in out P)
/// @type.symbol symbol=Grid source="newtype Grid<T, P> = intrinsic" type=Grid
/// @definition.newtype symbol=Grid source="newtype Grid<T, P> = intrinsic" template=(in out T#1, in out P) backing=intrinsic constructors=[<T#1, P>(intrinsic) => Grid<T#1, P>]
/// @type.symbol symbol=Grid.T source=T type=T#1
/// @type.symbol symbol=Grid.P source=P type=P

declare function mesh<T, ...Axes: Axis[]>(
/// @generic.template symbol=mesh parameters=(T#2, ...Axes#2: Axis[], 'a)
/// @type.symbol symbol=mesh type=<T#2, ...Axes#2: Axis[], mesh.'a>(&mesh.'a readonly Grid<T#2, Sharding<Axes#2>>) => int32
/// @type.symbol symbol=mesh.T source=T type=T#2
/// @type.symbol symbol=mesh.Axes source="...Axes: Axis[]" type=Axes#2
/// @resolution.name source=Axis target=Axis

    grid: &readonly Grid<T, Sharding<...Axes>>,
    /// @type.symbol symbol=mesh.grid source="grid: &readonly Grid<T, Sharding<...Axes>>" type=&mesh.'a readonly Grid<T#2, Sharding<Axes#2>>
    /// @resolution.name source=Grid target=Grid
    /// @resolution.name source=T target=mesh.T
    /// @resolution.name source=Sharding target=Sharding
    /// @resolution.name source=Axes target=mesh.Axes

): int32;

export extension<T, ...Axes: Axis[]> of Grid<T, Sharding<...Axes>> {
/// @generic.template symbol=<module>#2 parameters=(T#3, ...Axes#3: Axis[])
/// @definition.extension symbol=<module>#2 form=exported target=Grid<T#3, Sharding<Axes#3>>
/// @definition.method symbol=mesh#1 slot=mesh role=getter type=<mesh#1.'a>(this: &mesh#1.'a readonly this) => int32
/// @type.symbol symbol=T source=T type=T#3
/// @type.symbol symbol=Axes source="...Axes: Axis[]" type=Axes#3
/// @resolution.name source=Axis target=Axis
/// @resolution.name source=Grid target=Grid
/// @resolution.name source=T target=T
/// @resolution.name source=Sharding target=Sharding
/// @resolution.name source=Axes target=Axes

    /// Return the mesh id.
    get mesh(): int32 {
    /// @generic.template symbol=mesh#1 parent=template#3 parameters=('a)
    /// @type.symbol symbol=mesh#1 type=<mesh#1.'a>(this: &mesh#1.'a readonly this) => int32
    /// @type.symbol symbol=mesh.this type=&mesh#1.'a readonly Grid<T#3, Sharding<Axes#3>>

        return mesh<T, ...Axes>(this);
        /// @resolution.name source=mesh target=mesh
        /// @resolution.call source="mesh<T, ...Axes>(this)" parameters=(&mesh#1.'a readonly Grid<T#3, Sharding<Axes#3>>) arguments=(provided(this) as &mesh#1.'a readonly Grid<T#3, Sharding<Axes#3>>) return=int32 kind=symbol target=mesh instance="mesh<T#3, Axes#3>"
        /// @generic.instantiation id="mesh<T#3, Axes#3>" template=mesh arguments=(T#3, Axes#3) owner=mesh#1
        /// @resolution.name source=T target=T
        /// @resolution.name source=Axes target=Axes
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&mesh#1.'a readonly Grid<T#3, Sharding<Axes#3>>
        /// @resolution.place source=this placement=mesh#1.'a lifetime=mesh#1.'a access="readonly"
        /// @resolution.access source=this root=this

    }
}
"#,
    );
}

/// Spreading a mismatched pack into a const variadic reports a diagnostic.
#[test]
fn test_diagnose_spreading_a_mismatched_pack_into_a_const_variadic() {
    let session = TestSession::builder()
        .module(
            "sharding.ds",
            r#"
export newtype interface Marker {}

export newtype interface Placed {}

export newtype Wrap<const ...Xs: Marker> = intrinsic;

export extension<const ...Xs: Marker> of Wrap<...Xs> implements Placed {}

export newtype Grid<T, P: Placed> = intrinsic;

export newtype Axis = intrinsic;
"#,
        )
        .module(
            "main.ds",
            r#"
import { Axis, Grid, Marker, Wrap } from "./sharding.ds";

declare function mesh<T, ...Xs: Axis[]>(
    grid: &readonly Grid<T, Wrap<...Xs>>,
): int32;

extension<T, const ...Xs: Marker> of Grid<T, Wrap<...Xs>> {
    /// Return the mesh id.
    get mesh(): int32 {
        return mesh<T, ...Xs>(this);
    }
}
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Axis, Grid, Marker, Wrap } from "./sharding.ds";

declare function mesh<T, ...Xs: Axis[], 'a>(grid: &'a readonly Grid<T, Wrap<Xs>>): int32;

extension<T, const ...Xs: Marker> of Grid<T, Wrap<...Xs>> {
    /// Return the mesh id.
    get mesh(): int32 {
        return mesh<T, ...Xs>(this);
    }
}

=== dir ===
import { Axis, Grid, Marker, Wrap } from "./sharding.ds";

declare function mesh<T, ...Xs: Axis[]>(
/// @generic.template symbol=mesh parameters=(T#1, ...Xs#1: sharding.Axis[], 'a)
/// @type.symbol symbol=mesh type=<T#1, ...Xs#1: sharding.Axis[], mesh.'a>(&mesh.'a readonly sharding.Grid<T#1, sharding.Wrap<Xs#1>>) => int32
/// @type.symbol symbol=mesh.T source=T type=T#1
/// @type.symbol symbol=mesh.Xs source="...Xs: Axis[]" type=Xs#1
/// @resolution.name source=Axis target=sharding.Axis

    grid: &readonly Grid<T, Wrap<...Xs>>,
    /// @type.symbol symbol=mesh.grid source="grid: &readonly Grid<T, Wrap<...Xs>>" type=&mesh.'a readonly sharding.Grid<T#1, sharding.Wrap<Xs#1>>
    /// @resolution.name source=Grid target=sharding.Grid
    /// @resolution.name source=T target=mesh.T
    /// @resolution.name source=Wrap target=sharding.Wrap
    /// @resolution.name source=Xs target=mesh.Xs

): int32;

extension<T, const ...Xs: Marker> of Grid<T, Wrap<...Xs>> {
/// @generic.template symbol=<module>#2 parameters=(T#2, const ...Xs#2: sharding.Marker)
/// @definition.extension symbol=<module>#2 form=local target=sharding.Grid<T#2, sharding.Wrap<Xs#2>>
/// @definition.method symbol=mesh#1 slot=mesh role=getter type=<mesh#1.'a>(this: &mesh#1.'a readonly this) => int32
/// @type.symbol symbol=T source=T type=T#2
/// @type.symbol symbol=Xs source="const ...Xs: Marker" type=Xs#2
/// @resolution.name source=Marker target=sharding.Marker
/// @resolution.name source=Grid target=sharding.Grid
/// @resolution.name source=T target=T
/// @resolution.name source=Wrap target=sharding.Wrap
/// @resolution.name source=Xs target=Xs

    /// Return the mesh id.
    get mesh(): int32 {
    /// @generic.template symbol=mesh#1 parent=template#1 parameters=('a)
    /// @type.symbol symbol=mesh#1 type=<mesh#1.'a>(this: &mesh#1.'a readonly this) => int32
    /// @type.symbol symbol=mesh.this type=&mesh#1.'a readonly sharding.Grid<T#2, sharding.Wrap<Xs#2>>

        return mesh<T, ...Xs>(this);
        /// @resolution.name source=mesh target=mesh
        /// @resolution.call source="mesh<T, ...Xs>(this)" parameters=(&'frame readonly sharding.Grid<T#2, sharding.Wrap<Xs#2>>) arguments=(provided(this) as &'frame readonly sharding.Grid<T#2, sharding.Wrap<Xs#2>>) return=int32 kind=symbol target=mesh instance="mesh<T#2, Xs#2>"
        /// @generic.instantiation id="mesh<T#2, Xs#2>" template=mesh arguments=(T#2, Xs#2) owner=mesh#1
        /// @resolution.name source=T target=T
        /// @resolution.name source=Xs target=Xs
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&mesh#1.'a readonly sharding.Grid<T#2, sharding.Wrap<Xs#2>>
        /// @resolution.place source=this placement=mesh#1.'a lifetime=mesh#1.'a access="readonly"
        /// @resolution.access source=this root=this

    }
}
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'sharding.Wrap<Xs>' does not satisfy 'sharding.Placed'"
/// @diagnostic.label line=5 column=29 span="Wrap<...Xs>" line_source="grid: &readonly Grid<T, Wrap<...Xs>>,"
/// @diagnostic.related file="sharding.ds" line=10 column=24 span="P" line_source="export newtype Grid<T, P: Placed> = intrinsic;" message="required by this bound on 'P'"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Xs' does not satisfy 'sharding.Marker'"
/// @diagnostic.label line=5 column=34 span="...Xs" line_source="grid: &readonly Grid<T, Wrap<...Xs>>,"
/// @diagnostic.related file="sharding.ds" line=6 column=30 span="Xs" line_source="export newtype Wrap<const ...Xs: Marker> = intrinsic;" message="required by this bound on 'Xs'"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Xs' does not satisfy 'sharding.Axis[]'"
/// @diagnostic.label line=11 column=16 span="mesh<T, ...Xs>(this)" line_source="return mesh<T, ...Xs>(this);"
/// @diagnostic.related line=4 column=29 span="Xs" line_source="declare function mesh<T, ...Xs: Axis[]>(" message="required by this bound on 'Xs'"
"#,
    );
}

/// An unresolved variadic bound reports a diagnostic.
#[test]
fn test_diagnose_an_unresolved_variadic_bound() {
    let session = TestSession::single(
        r#"
declare function mesh<...Axes: Missing[]>(value: int32): int32;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function mesh<...Axes: Missing[]>(value: int32): int32;

=== dir ===
declare function mesh<...Axes: Missing[]>(value: int32): int32;
/// @generic.template symbol=mesh parameters=(...Axes: <error>[])
/// @type.symbol symbol=mesh source="declare function mesh<...Axes: Missing[]>(value: int32): int32" type=<...Axes: <error>[]>(int32) => int32
/// @type.symbol symbol=mesh.Axes source="...Axes: Missing[]" type=Axes
/// @resolution.unresolved source=Missing path=Missing
/// @type.symbol symbol=mesh.value source="value: int32" type=int32
"#,
        r#"
/// @diagnostic.error id=unresolved-reference message="cannot find 'Missing'"
/// @diagnostic.label line=2 column=32 span="Missing" line_source="declare function mesh<...Axes: Missing[]>(value: int32): int32;"
"#,
    );
}
