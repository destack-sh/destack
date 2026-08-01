use crate::tests::{DirRows, TestSession};

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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype Axis = intrinsic;

newtype Sharding<...Axes: Axis[]> = intrinsic;

newtype Grid<T, P> = intrinsic;

declare function mesh<T, ...Axes: Axis[], 'a>(grid: &'a readonly Grid<T, Sharding<Axes>>): int32;

export extension<T, ...Axes: Axis[]> of Grid<T, Sharding<...Axes>> {
    /// Return the mesh id.
    get mesh(): int32 {
        return mesh<T, ...Axes>(this as &'frame readonly Grid<T, Sharding<Axes>>);
    }
}

=== checked ===
newtype Axis = intrinsic;
/// @type.symbol symbol=Axis source="newtype Axis = intrinsic" type=Axis
/// @definition.newtype symbol=Axis source="newtype Axis = intrinsic" backing=intrinsic

newtype Sharding<...Axes: Axis[]> = intrinsic;
/// @generic.template symbol=Sharding parameters=(...Axes#1: Array<Axis>)
/// @type.symbol symbol=Sharding source="newtype Sharding<...Axes: Axis[]> = intrinsic" type=Sharding
/// @definition.newtype symbol=Sharding source="newtype Sharding<...Axes: Axis[]> = intrinsic" template=(...Axes#1: Array<Axis>) backing=intrinsic
/// @type.symbol symbol=Sharding.Axes source="...Axes: Axis[]" type=Axes#1
/// @resolution.name source=Axis target=Axis

newtype Grid<T, P> = intrinsic;
/// @generic.template symbol=Grid parameters=(T#1, P)
/// @type.symbol symbol=Grid source="newtype Grid<T, P> = intrinsic" type=Grid
/// @definition.newtype symbol=Grid source="newtype Grid<T, P> = intrinsic" template=(T#1, P) backing=intrinsic
/// @type.symbol symbol=Grid.T source=T type=T#1
/// @type.symbol symbol=Grid.P source=P type=P

declare function mesh<T, ...Axes: Axis[]>(
/// @generic.template symbol=mesh#1 parameters=(T#2, ...Axes#2: Array<Axis>, 'a)
/// @type.symbol symbol=mesh#1 type=<T#2, ...Axes#2: Array<Axis>, mesh#1.'a>(&mesh#1.'a readonly Grid<T#2, Sharding<Axes#2>>) => int32
/// @type.symbol symbol=mesh.T source=T type=T#2
/// @type.symbol symbol=mesh.Axes source="...Axes: Axis[]" type=Axes#2
/// @resolution.name source=Axis target=Axis

    grid: &readonly Grid<T, Sharding<...Axes>>,
    /// @type.symbol symbol=mesh.grid source="grid: &readonly Grid<T, Sharding<...Axes>>" type=&mesh#1.'a readonly Grid<T#2, Sharding<Axes#2>>
    /// @resolution.name source=Grid target=Grid
    /// @resolution.name source=T target=mesh.T
    /// @resolution.name source=Sharding target=Sharding
    /// @resolution.name source=Axes target=mesh.Axes

): int32;

export extension<T, ...Axes: Axis[]> of Grid<T, Sharding<...Axes>> {
/// @generic.template symbol=<module>#2 parameters=(T#3, ...Axes#3: Array<Axis>)
/// @definition.extension symbol=<module>#2 form=exported target=Grid<T#3, Sharding<Axes#3>>
/// @definition.method symbol=mesh#2 slot=mesh role=getter type=(this: this) => int32
/// @type.symbol symbol=T source=T type=T#3
/// @type.symbol symbol=Axes source="...Axes: Axis[]" type=Axes#3
/// @resolution.name source=Axis target=Axis
/// @resolution.name source=Grid target=Grid
/// @resolution.name source=T target=T
/// @resolution.name source=Sharding target=Sharding
/// @resolution.name source=Axes target=Axes

    /// Return the mesh id.
    get mesh(): int32 {
    /// @type.symbol symbol=mesh#2 type=(this: this) => int32

        return mesh<T, ...Axes>(this);
        /// @resolution.name source=mesh target=mesh#1
        /// @resolution.call source="mesh<T, ...Axes>(this)" parameters=(&'frame readonly Grid<T#3, Sharding<Axes#3>>) arguments=(provided(this) as &'frame readonly Grid<T#3, Sharding<Axes#3>>) return=int32 kind=symbol target=mesh#1 instance="mesh#1<T#3, Axes#3>"
        /// @generic.instance source="mesh<T, ...Axes>(this)" id="mesh#1<T#3, Axes#3>"
        /// @resolution.name source=T target=T
        /// @resolution.name source=Axes target=Axes
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Grid<T#3, Sharding<Axes#3>>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this

    }
}

/// @generic.instance id="Grid<T#2, Sharding<Axes#2>>" template=Grid arguments=(T#2, Sharding<Axes#2>)
/// @generic.instance id="mesh#1<T#3, Axes#3>" template=mesh#1 arguments=(T#3, Axes#3)
/// @generic.instance id=Sharding<Axes#2> template=Sharding arguments=(Axes#2)
"#,
    );
}

#[test]
fn test_diagnose_spreading_a_mismatched_pack_into_a_comptime_variadic() {
    let session = TestSession::builder()
        .module(
            "sharding.ds",
            r#"
export newtype interface Marker {}

export newtype interface Placed {}

export newtype Wrap<comptime ...Xs: Marker> = intrinsic;

export extension<comptime ...Xs: Marker> of Wrap<...Xs> implements Placed {}

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

extension<T, comptime ...Xs: Marker> of Grid<T, Wrap<...Xs>> {
    /// Return the mesh id.
    get mesh(): int32 {
        return mesh<T, ...Xs>(this);
    }
}
"#,
        )
        .build();

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Axis, Grid, Marker, Wrap } from "./sharding.ds";

declare function mesh<T, ...Xs: Axis[], 'a>(grid: &'a readonly Grid<T, Wrap<Xs>>): int32;

extension<T, comptime ...Xs: Marker> of Grid<T, Wrap<...Xs>> {
    /// Return the mesh id.
    get mesh(): int32 {
        return mesh<T, ...Xs>(this);
    }
}

=== checked ===
import { Axis, Grid, Marker, Wrap } from "./sharding.ds";

declare function mesh<T, ...Xs: Axis[]>(
/// @generic.template symbol=mesh#1 parameters=(T#1, ...Xs#1: Array<sharding.Axis>, 'a)
/// @type.symbol symbol=mesh#1 type=<T#1, ...Xs#1: Array<sharding.Axis>, mesh#1.'a>(&mesh#1.'a readonly <error>) => int32
/// @type.symbol symbol=mesh#1 type=<T#1, ...Xs#1: Array<sharding.Axis>, mesh#1.'a>(&mesh#1.'a readonly sharding.Grid<T#1, sharding.Wrap<Xs#1>>) => int32
/// @type.symbol symbol=mesh.T source=T type=T#1
/// @type.symbol symbol=mesh.Xs source="...Xs: Axis[]" type=Xs#1
/// @resolution.name source=Axis target=sharding.Axis

    grid: &readonly Grid<T, Wrap<...Xs>>,
    /// @type.symbol symbol=mesh.grid source="grid: &readonly Grid<T, Wrap<...Xs>>" type=&mesh#1.'a readonly <error>
    /// @resolution.name source=Grid target=sharding.Grid
    /// @resolution.name source=T target=mesh.T
    /// @resolution.name source=Wrap target=sharding.Wrap
    /// @resolution.name source=Xs target=mesh.Xs

): int32;

extension<T, comptime ...Xs: Marker> of Grid<T, Wrap<...Xs>> {
/// @generic.template symbol=<module>#2 parameters=(T#2, comptime ...Xs#2: sharding.Marker)
/// @definition.extension symbol=<module>#2 form=local target=sharding.Grid<T#2, sharding.Wrap<Xs#2>>
/// @definition.method symbol=mesh#2 slot=mesh role=getter type=(this: this) => int32
/// @type.symbol symbol=T source=T type=T#2
/// @type.symbol symbol=Xs source="comptime ...Xs: Marker" type=Xs#2
/// @resolution.name source=Marker target=sharding.Marker
/// @resolution.name source=Grid target=sharding.Grid
/// @resolution.name source=T target=T
/// @resolution.name source=Wrap target=sharding.Wrap
/// @resolution.name source=Xs target=Xs

    /// Return the mesh id.
    get mesh(): int32 {
    /// @type.symbol symbol=mesh#2 type=(this: this) => int32

        return mesh<T, ...Xs>(this);
        /// @resolution.name source=mesh target=mesh#1
        /// @resolution.call source="mesh<T, ...Xs>(this)" parameters=(&'frame readonly sharding.Grid<T#2, sharding.Wrap<Xs#2>>) arguments=(provided(this) as &'frame readonly sharding.Grid<T#2, sharding.Wrap<Xs#2>>) return=int32 kind=symbol target=mesh#1 instance="mesh#1<T#2, Xs#2>"
        /// @generic.instance source="mesh<T, ...Xs>(this)" id="mesh#1<T#2, Xs#2>"
        /// @resolution.name source=T target=T
        /// @resolution.name source=Xs target=Xs
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=sharding.Grid<T#2, sharding.Wrap<Xs#2>>
        /// @resolution.access source=this root=this

    }
}

/// @generic.instance id="mesh#1<T#2, Xs#2>" template=mesh#1 arguments=(T#2, Xs#2)
/// @generic.instance id="sharding.Grid<T#1, sharding.Wrap<Xs#1>>" template=sharding.Grid arguments=(T#1, sharding.Wrap<Xs#1>)
/// @generic.instance id=sharding.Wrap<Xs#1> template=sharding.Wrap arguments=(Xs#1)
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Xs' does not satisfy 'Array<sharding.Axis>'"
/// @diagnostic.label line=11 column=16 span="mesh<T, ...Xs>(this)" line_source="return mesh<T, ...Xs>(this);"
/// @diagnostic.related line=4 column=29 span="Xs" line_source="declare function mesh<T, ...Xs: Axis[]>(" message="required by this bound on 'Xs'"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Xs' does not satisfy 'sharding.Marker'"
/// @diagnostic.label line=5 column=34 span="...Xs" line_source="grid: &readonly Grid<T, Wrap<...Xs>>,"
/// @diagnostic.related file="sharding.ds" line=6 column=33 span="Xs" line_source="export newtype Wrap<comptime ...Xs: Marker> = intrinsic;" message="required by this bound on 'Xs'"
/// @diagnostic.error id=constraint-not-satisfied message="type 'sharding.Wrap<Xs>' does not satisfy 'sharding.Placed'"
/// @diagnostic.label line=5 column=29 span="Wrap<...Xs>" line_source="grid: &readonly Grid<T, Wrap<...Xs>>,"
/// @diagnostic.related file="sharding.ds" line=10 column=24 span="P" line_source="export newtype Grid<T, P: Placed> = intrinsic;" message="required by this bound on 'P'"
"#,
    );
}

#[test]
fn test_diagnose_an_unresolved_variadic_bound() {
    let session = TestSession::single(
        r#"
declare function mesh<...Axes: Missing[]>(value: int32): int32;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function mesh<...Axes: Missing[]>(value: int32): int32;

=== checked ===
declare function mesh<...Axes: Missing[]>(value: int32): int32;
/// @generic.template symbol=mesh parameters=(...Axes: Array<<error>>)
/// @type.symbol symbol=mesh source="declare function mesh<...Axes: Missing[]>(value: int32): int32" type=<...Axes: Array<<error>>>(int32) => int32
/// @type.symbol symbol=mesh.Axes source="...Axes: Missing[]" type=Axes
/// @type.symbol symbol=mesh.value source="value: int32" type=int32
"#,
        r#"
/// @diagnostic.error id=unresolved-reference message="cannot find 'Missing'"
/// @diagnostic.label line=2 column=32 span="Missing" line_source="declare function mesh<...Axes: Missing[]>(value: int32): int32;"
"#,
    );
}
