use crate::tests::{DirRows, TestSession};

#[test]
fn test_access_generic_receiver_selects_a_sibling_method() {
    let session = TestSession::single(
        r#"
struct Grid {
    size: int32;
}

export extension<const A: Access = "readonly"> of Grid {
    view(this: WithAccess<&Grid, A>): int32 {
        this.size
    }

    peek(this: WithAccess<&Grid, A>): int32 {
        this.view()
    }
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Grid {
    size: int32;
}

export extension<const A: Access = "readonly"> of Grid {
    view(this: WithAccess<&Grid, A>): int32 {
        this.size
    }

    peek(this: WithAccess<&Grid, A>): int32 {
        this.view<A, 'a>()
    }
}

=== dir ===
struct Grid {
/// @type.symbol symbol=Grid type=Grid
/// @definition.struct symbol=Grid
/// @definition.field symbol=Grid.size source="size: int32" key=size type=int32

    size: int32;
    /// @type.symbol symbol=Grid.size source="size: int32" type=int32

}

export extension<const A: Access = "readonly"> of Grid {
/// @generic.template symbol=<module>#2 parameters=(const A: Access = "readonly")
/// @definition.extension symbol=<module>#2 form=exported target=Grid
/// @definition.method symbol=peek slot=peek type=<peek.'a>(this: WithAccess<&peek.'a Grid, A>) => int32
/// @definition.method symbol=view slot=view type=<view.'a>(this: WithAccess<&view.'a Grid, A>) => int32
/// @type.symbol symbol=A source="const A: Access = \"readonly\"" type=A
/// @resolution.name source=Access target=Access
/// @resolution.name source=Grid target=Grid

    view(this: WithAccess<&Grid, A>): int32 {
    /// @generic.template symbol=view parent=template#0 parameters=('a)
    /// @type.symbol symbol=view type=<view.'a>(this: WithAccess<&view.'a Grid, A>) => int32
    /// @type.symbol symbol=view.this source="this: WithAccess<&Grid, A>" type=WithAccess<&view.'a Grid, A>
    /// @resolution.name source=WithAccess target=WithAccess
    /// @resolution.name source=Grid target=Grid
    /// @resolution.name source=A target=A

        this.size
        /// @type.node source=this type=WithAccess<&view.'a Grid, A>
        /// @type.node source=this.size type=int32
        /// @resolution.member source=this.size receiver=WithAccess<&view.'a Grid, A> type=int32 kind=field target_receiver=WithAccess<&view.'a Grid, A> key=size target=Grid.size target_type=int32
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=WithAccess<&view.'a Grid, A>
        /// @resolution.place source=this placement=view.'a lifetime=view.'a access=A
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.size placement=view.'a lifetime=view.'a access=A
        /// @resolution.access source=this.size root=this keys=[size]

    }

    peek(this: WithAccess<&Grid, A>): int32 {
    /// @generic.template symbol=peek parent=template#0 parameters=('a)
    /// @type.symbol symbol=peek type=<peek.'a>(this: WithAccess<&peek.'a Grid, A>) => int32
    /// @type.symbol symbol=peek.this source="this: WithAccess<&Grid, A>" type=WithAccess<&peek.'a Grid, A>
    /// @resolution.name source=WithAccess target=WithAccess
    /// @resolution.name source=Grid target=Grid
    /// @resolution.name source=A target=A

        this.view()
        /// @type.node source=this type=WithAccess<&peek.'a Grid, A>
        /// @type.node source=this.view type=<view.'a>(this: WithAccess<&view.'a Grid, A>) => int32
        /// @type.node source=this.view() type=int32
        /// @resolution.member source=this.view receiver=WithAccess<&peek.'a Grid, A> type=<view.'a>(this: WithAccess<&view.'a Grid, A>) => int32 kind=symbol target_receiver=WithAccess<&peek.'a Grid, A> target=view
        /// @resolution.call source=this.view() parameters=() return=int32 regions=(peek.'a) kind=symbol target=view receiver=WithAccess<&peek.'a Grid, A> instance=Grid.<extension#1>.view<peek.'a>
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=WithAccess<&peek.'a Grid, A>
        /// @resolution.place source=this placement=peek.'a lifetime=peek.'a access=A
        /// @resolution.access source=this root=this
        /// @generic.instantiation id="view<A, peek.'a>" template=view arguments=(A, peek.'a) owner=peek
        /// @generic.instantiation id=view<A> template=view arguments=(A) owner=peek
        /// @generic.instance id="view<A, peek.'a>" template=view arguments=(A, peek.'a)

    }
}
"#,
    );
}

#[test]
fn test_access_generic_extension_infers_access_from_the_call_site() {
    let session = TestSession::single(
        r#"
struct Grid {
    size: int32;
}

export extension<const A: Access = "readonly"> of Grid {
    view(this: WithAccess<&Grid, A>): int32 {
        this.size
    }
}

function read(grid: &readonly Grid): int32 {
    grid.view()
}

function write(grid: &Grid): int32 {
    grid.view()
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Grid {
    size: int32;
}

export extension<const A: Access = "readonly"> of Grid {
    view(this: WithAccess<&Grid, A>): int32 {
        this.size
    }
}

function read<'a>(grid: &'a readonly Grid): int32 {
    grid.view<"readonly", 'a>()
}

function write<'a>(grid: &'a Grid): int32 {
    grid.view<"mutable", 'a>()
}

=== dir ===
struct Grid {
/// @type.symbol symbol=Grid type=Grid
/// @definition.struct symbol=Grid
/// @definition.field symbol=Grid.size source="size: int32" key=size type=int32

    size: int32;
    /// @type.symbol symbol=Grid.size source="size: int32" type=int32

}

export extension<const A: Access = "readonly"> of Grid {
/// @generic.template symbol=<module>#2 parameters=(const A: Access = "readonly")
/// @definition.extension symbol=<module>#2 form=exported target=Grid
/// @definition.method symbol=view slot=view type=<view.'a>(this: WithAccess<&view.'a Grid, A>) => int32
/// @type.symbol symbol=A source="const A: Access = \"readonly\"" type=A
/// @resolution.name source=Access target=Access
/// @resolution.name source=Grid target=Grid

    view(this: WithAccess<&Grid, A>): int32 {
    /// @generic.template symbol=view parent=template#0 parameters=('a)
    /// @type.symbol symbol=view type=<view.'a>(this: WithAccess<&view.'a Grid, A>) => int32
    /// @type.symbol symbol=view.this source="this: WithAccess<&Grid, A>" type=WithAccess<&view.'a Grid, A>
    /// @resolution.name source=WithAccess target=WithAccess
    /// @resolution.name source=Grid target=Grid
    /// @resolution.name source=A target=A

        this.size
        /// @type.node source=this type=WithAccess<&view.'a Grid, A>
        /// @type.node source=this.size type=int32
        /// @resolution.member source=this.size receiver=WithAccess<&view.'a Grid, A> type=int32 kind=field target_receiver=WithAccess<&view.'a Grid, A> key=size target=Grid.size target_type=int32
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=WithAccess<&view.'a Grid, A>
        /// @resolution.place source=this placement=view.'a lifetime=view.'a access=A
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.size placement=view.'a lifetime=view.'a access=A
        /// @resolution.access source=this.size root=this keys=[size]

    }
}

function read(grid: &readonly Grid): int32 {
/// @generic.template symbol=read parameters=('a)
/// @type.symbol symbol=read type=<read.'a>(&read.'a readonly Grid) => int32
/// @type.symbol symbol=read.grid source="grid: &readonly Grid" type=&read.'a readonly Grid
/// @resolution.name source=Grid target=Grid

    grid.view()
    /// @type.node source=grid type=&read.'a readonly Grid
    /// @type.node source=grid.view type=<view.'a>(this: &view.'a readonly Grid) => int32
    /// @type.node source=grid.view() type=int32
    /// @resolution.name source=grid target=read.grid
    /// @resolution.member source=grid.view receiver=&read.'a readonly Grid type=<view.'a>(this: &view.'a readonly Grid) => int32 kind=symbol target_receiver=&read.'a readonly Grid target=view
    /// @resolution.call source=grid.view() parameters=() return=int32 regions=(read.'a) kind=symbol target=view receiver=&read.'a readonly Grid instance=Grid.<extension#1>.view<read.'a>
    /// @resolution.place source=grid placement=read.'a lifetime=read.'a access="readonly"
    /// @resolution.access source=grid root=read.grid
    /// @generic.instantiation id="view<\"readonly\", read.'a>" template=view arguments=("readonly", read.'a)
    /// @generic.instantiation id="view<\"readonly\">" template=view arguments=("readonly")
    /// @generic.instance id="view<\"readonly\", read.'a>" template=view arguments=("readonly", read.'a)

}

function write(grid: &Grid): int32 {
/// @generic.template symbol=write parameters=('a)
/// @type.symbol symbol=write type=<write.'a>(&write.'a Grid) => int32
/// @type.symbol symbol=write.grid source="grid: &Grid" type=&write.'a Grid
/// @resolution.name source=Grid target=Grid

    grid.view()
    /// @type.node source=grid type=&write.'a Grid
    /// @type.node source=grid.view type=<view.'a>(this: &view.'a Grid) => int32
    /// @type.node source=grid.view() type=int32
    /// @resolution.name source=grid target=write.grid
    /// @resolution.member source=grid.view receiver=&write.'a Grid type=<view.'a>(this: &view.'a Grid) => int32 kind=symbol target_receiver=&write.'a Grid target=view
    /// @resolution.call source=grid.view() parameters=() return=int32 regions=(write.'a) kind=symbol target=view receiver=&write.'a Grid instance=Grid.<extension#1>.view<write.'a>
    /// @resolution.place source=grid placement=write.'a lifetime=write.'a access="mutable"
    /// @resolution.access source=grid root=write.grid
    /// @generic.instantiation id="view<\"mutable\", write.'a>" template=view arguments=("mutable", write.'a)
    /// @generic.instantiation id="view<\"mutable\">" template=view arguments=("mutable")
    /// @generic.instance id="view<\"mutable\", write.'a>" template=view arguments=("mutable", write.'a)

}
"#,
    );
}

#[test]
fn test_access_generic_receiver_selects_fixed_array_sibling_method() {
    let session = TestSession::single(
        r#"
export extension FixedArrayAccess<T, const N: usize, const A: Access = "readonly"> of [T; N] {
    inspect(this: WithAccess<&[T; N], A>): int32 {
        1
    }

    probe(this: WithAccess<&[T; N], A>): int32 {
        this.inspect()
    }
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
export extension FixedArrayAccess<T, const N: usize, const A: Access = "readonly"> of [T; N] {
    inspect(this: WithAccess<&[T; N], A>): int32 {
        1
    }

    probe(this: WithAccess<&[T; N], A>): int32 {
        this.inspect<T, N, A, 'a>()
    }
}

=== dir ===
export extension FixedArrayAccess<T, const N: usize, const A: Access = "readonly"> of [T; N] {
/// @generic.template symbol=FixedArrayAccess parameters=(T, const N: usize, const A: Access = "readonly")
/// @definition.extension symbol=FixedArrayAccess form=exported target=FixedArray<T, N>
/// @definition.method symbol=FixedArrayAccess.inspect slot=inspect type=<FixedArrayAccess.inspect.'a>(this: WithAccess<&FixedArrayAccess.inspect.'a FixedArray<T, N>, A>) => int32
/// @definition.method symbol=FixedArrayAccess.probe slot=probe type=<FixedArrayAccess.probe.'a>(this: WithAccess<&FixedArrayAccess.probe.'a FixedArray<T, N>, A>) => int32
/// @type.symbol symbol=FixedArrayAccess.T source=T type=T
/// @type.symbol symbol=FixedArrayAccess.N source="const N: usize" type=N
/// @type.symbol symbol=FixedArrayAccess.A source="const A: Access = \"readonly\"" type=A
/// @resolution.name source=Access target=Access
/// @resolution.name source=T target=FixedArrayAccess.T
/// @resolution.name source=N target=FixedArrayAccess.N

    inspect(this: WithAccess<&[T; N], A>): int32 {
    /// @generic.template symbol=FixedArrayAccess.inspect parent=template#0 parameters=('a)
    /// @type.symbol symbol=FixedArrayAccess.inspect type=<FixedArrayAccess.inspect.'a>(this: WithAccess<&FixedArrayAccess.inspect.'a FixedArray<T, N>, A>) => int32
    /// @type.symbol symbol=FixedArrayAccess.inspect.this source="this: WithAccess<&[T; N], A>" type=WithAccess<&FixedArrayAccess.inspect.'a FixedArray<T, N>, A>
    /// @resolution.name source=WithAccess target=WithAccess
    /// @resolution.name source=T target=FixedArrayAccess.T
    /// @resolution.name source=N target=FixedArrayAccess.N
    /// @resolution.name source=A target=FixedArrayAccess.A

        1
        /// @type.node source=1 type=1

    }

    probe(this: WithAccess<&[T; N], A>): int32 {
    /// @generic.template symbol=FixedArrayAccess.probe parent=template#0 parameters=('a)
    /// @type.symbol symbol=FixedArrayAccess.probe type=<FixedArrayAccess.probe.'a>(this: WithAccess<&FixedArrayAccess.probe.'a FixedArray<T, N>, A>) => int32
    /// @type.symbol symbol=FixedArrayAccess.probe.this source="this: WithAccess<&[T; N], A>" type=WithAccess<&FixedArrayAccess.probe.'a FixedArray<T, N>, A>
    /// @resolution.name source=WithAccess target=WithAccess
    /// @resolution.name source=T target=FixedArrayAccess.T
    /// @resolution.name source=N target=FixedArrayAccess.N
    /// @resolution.name source=A target=FixedArrayAccess.A

        this.inspect()
        /// @type.node source=this type=WithAccess<&FixedArrayAccess.probe.'a FixedArray<T, N>, A>
        /// @type.node source=this.inspect type=<FixedArrayAccess.inspect.'a>(this: WithAccess<&FixedArrayAccess.inspect.'a FixedArray<T, N>, A>) => int32
        /// @type.node source=this.inspect() type=int32
        /// @resolution.member source=this.inspect receiver=WithAccess<&FixedArrayAccess.probe.'a FixedArray<T, N>, A> type=<FixedArrayAccess.inspect.'a>(this: WithAccess<&FixedArrayAccess.inspect.'a FixedArray<T, N>, A>) => int32 kind=symbol target_receiver=WithAccess<&FixedArrayAccess.probe.'a FixedArray<T, N>, A> target=FixedArrayAccess.inspect
        /// @resolution.call source=this.inspect() parameters=() return=int32 regions=(FixedArrayAccess.probe.'a) kind=symbol target=FixedArrayAccess.inspect receiver=WithAccess<&FixedArrayAccess.probe.'a FixedArray<T, N>, A> instance="FixedArrayAccess<T, N, A>.inspect<FixedArrayAccess.probe.'a>"
        /// @resolution.receiver source=this kind=this declaration=FixedArrayAccess type=WithAccess<&FixedArrayAccess.probe.'a FixedArray<T, N>, A>
        /// @resolution.place source=this placement=FixedArrayAccess.probe.'a lifetime=FixedArrayAccess.probe.'a access=A
        /// @resolution.access source=this root=this
        /// @generic.instantiation id="FixedArrayAccess.inspect<T, N, A, FixedArrayAccess.probe.'a>" template=FixedArrayAccess.inspect arguments=(T, N, A, FixedArrayAccess.probe.'a) owner=FixedArrayAccess.probe
        /// @generic.instantiation id="FixedArrayAccess.inspect<T, N, A>" template=FixedArrayAccess.inspect arguments=(T, N, A) owner=FixedArrayAccess.probe
        /// @generic.instance id="FixedArrayAccess.inspect<T, N, A, FixedArrayAccess.probe.'a>" template=FixedArrayAccess.inspect arguments=(T, N, A, FixedArrayAccess.probe.'a)

    }
}
"#,
    );
}

#[test]
fn test_access_generic_receiver_selects_array_sibling_method() {
    let session = TestSession::single(
        r#"
export extension ArrayAccess<T, const A: Access = "readonly"> of Array<T> {
    inspect(this: WithAccess<&Array<T>, A>): int32 {
        1
    }

    probe(this: WithAccess<&Array<T>, A>): int32 {
        this.inspect()
    }
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
export extension ArrayAccess<T, const A: Access = "readonly"> of Array<T> {
    inspect(this: WithAccess<&Array<T>, A>): int32 {
        1
    }

    probe(this: WithAccess<&Array<T>, A>): int32 {
        this.inspect<T, A, 'a>()
    }
}

=== dir ===
export extension ArrayAccess<T, const A: Access = "readonly"> of Array<T> {
/// @generic.template symbol=ArrayAccess parameters=(T, const A: Access = "readonly")
/// @generic.instance id=Array<T> template=Array arguments=(T)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<T>> template=sliceAssumeInit arguments=(MaybeUninit<T>)
/// @generic.instance id=sliceUninit<MaybeUninit<T>> template=sliceUninit arguments=(MaybeUninit<T>)
/// @definition.extension symbol=ArrayAccess form=exported target=T[]
/// @definition.method symbol=ArrayAccess.inspect slot=inspect type=<ArrayAccess.inspect.'a>(this: WithAccess<&ArrayAccess.inspect.'a T[], A>) => int32
/// @definition.method symbol=ArrayAccess.probe slot=probe type=<ArrayAccess.probe.'a>(this: WithAccess<&ArrayAccess.probe.'a T[], A>) => int32
/// @type.symbol symbol=ArrayAccess.T source=T type=T
/// @type.symbol symbol=ArrayAccess.A source="const A: Access = \"readonly\"" type=A
/// @resolution.name source=Access target=Access
/// @resolution.name source=Array target=Array
/// @resolution.name source=T target=ArrayAccess.T

    inspect(this: WithAccess<&Array<T>, A>): int32 {
    /// @generic.template symbol=ArrayAccess.inspect parent=template#0 parameters=('a)
    /// @type.symbol symbol=ArrayAccess.inspect type=<ArrayAccess.inspect.'a>(this: WithAccess<&ArrayAccess.inspect.'a T[], A>) => int32
    /// @type.symbol symbol=ArrayAccess.inspect.this source="this: WithAccess<&Array<T>, A>" type=WithAccess<&ArrayAccess.inspect.'a T[], A>
    /// @resolution.name source=WithAccess target=WithAccess
    /// @resolution.name source=Array target=Array
    /// @resolution.name source=T target=ArrayAccess.T
    /// @resolution.name source=A target=ArrayAccess.A

        1
        /// @type.node source=1 type=1

    }

    probe(this: WithAccess<&Array<T>, A>): int32 {
    /// @generic.template symbol=ArrayAccess.probe parent=template#0 parameters=('a)
    /// @type.symbol symbol=ArrayAccess.probe type=<ArrayAccess.probe.'a>(this: WithAccess<&ArrayAccess.probe.'a T[], A>) => int32
    /// @type.symbol symbol=ArrayAccess.probe.this source="this: WithAccess<&Array<T>, A>" type=WithAccess<&ArrayAccess.probe.'a T[], A>
    /// @resolution.name source=WithAccess target=WithAccess
    /// @resolution.name source=Array target=Array
    /// @resolution.name source=T target=ArrayAccess.T
    /// @resolution.name source=A target=ArrayAccess.A

        this.inspect()
        /// @type.node source=this type=WithAccess<&ArrayAccess.probe.'a T[], A>
        /// @type.node source=this.inspect type=<ArrayAccess.inspect.'a>(this: WithAccess<&ArrayAccess.inspect.'a T[], A>) => int32
        /// @type.node source=this.inspect() type=int32
        /// @resolution.member source=this.inspect receiver=WithAccess<&ArrayAccess.probe.'a T[], A> type=<ArrayAccess.inspect.'a>(this: WithAccess<&ArrayAccess.inspect.'a T[], A>) => int32 kind=symbol target_receiver=WithAccess<&ArrayAccess.probe.'a T[], A> target=ArrayAccess.inspect
        /// @resolution.call source=this.inspect() parameters=() return=int32 regions=(ArrayAccess.probe.'a) kind=symbol target=ArrayAccess.inspect receiver=WithAccess<&ArrayAccess.probe.'a T[], A> instance="ArrayAccess<T, A>.inspect<ArrayAccess.probe.'a>"
        /// @resolution.receiver source=this kind=this declaration=ArrayAccess type=WithAccess<&ArrayAccess.probe.'a T[], A>
        /// @resolution.place source=this placement=ArrayAccess.probe.'a lifetime=ArrayAccess.probe.'a access=A
        /// @resolution.access source=this root=this
        /// @generic.instantiation id="ArrayAccess.inspect<T, A, ArrayAccess.probe.'a>" template=ArrayAccess.inspect arguments=(T, A, ArrayAccess.probe.'a) owner=ArrayAccess.probe
        /// @generic.instantiation id="ArrayAccess.inspect<T, A>" template=ArrayAccess.inspect arguments=(T, A) owner=ArrayAccess.probe
        /// @generic.instance id="ArrayAccess.inspect<T, A, ArrayAccess.probe.'a>" template=ArrayAccess.inspect arguments=(T, A, ArrayAccess.probe.'a)

    }
}
"#,
    );
}

#[test]
fn test_missing_member_through_access_generic_receiver_suggests_field() {
    let session = TestSession::single(
        r#"
struct Box<Value> {
    value: Value;
}

extension<Value, const A: Access = "readonly"> of Box<Value> {
    read(this: WithAccess<&Box<Value>, A>): Value {
        return this.val;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Box<out Value> {
    value: Value;
}

extension<Value, const A: Access = "readonly"> of Box<Value> {
    read(this: WithAccess<&Box<Value>, A>): Value {
        return this.val;
    }
}

=== dir ===
struct Box<Value> {
/// @generic.template symbol=Box parameters=(out Value#1)
/// @type.symbol symbol=Box type=Box
/// @definition.struct symbol=Box template=(out Value#1)
/// @definition.field symbol=Box.value source="value: Value" key=value type=Value#1
/// @type.symbol symbol=Box.Value source=Value type=Value#1

    value: Value;
    /// @type.symbol symbol=Box.value source="value: Value" type=Value#1
    /// @resolution.name source=Value target=Box.Value

}

extension<Value, const A: Access = "readonly"> of Box<Value> {
/// @generic.template symbol=<module>#2 parameters=(Value#2, const A: Access = "readonly")
/// @definition.extension symbol=<module>#2 form=local target=Box<Value#2>
/// @definition.method symbol=read slot=read type=<read.'a>(this: WithAccess<&read.'a Box<Value#2>, A>) => Value#2
/// @type.symbol symbol=Value source=Value type=Value#2
/// @type.symbol symbol=A source="const A: Access = \"readonly\"" type=A
/// @resolution.name source=Access target=Access
/// @resolution.name source=Box target=Box
/// @resolution.name source=Value target=Value

    read(this: WithAccess<&Box<Value>, A>): Value {
    /// @generic.template symbol=read parent=template#1 parameters=('a)
    /// @type.symbol symbol=read type=<read.'a>(this: WithAccess<&read.'a Box<Value#2>, A>) => Value#2
    /// @type.symbol symbol=read.this source="this: WithAccess<&Box<Value>, A>" type=WithAccess<&read.'a Box<Value#2>, A>
    /// @resolution.name source=WithAccess target=WithAccess
    /// @resolution.name source=Box target=Box
    /// @resolution.name source=Value target=Value
    /// @resolution.name source=A target=A
    /// @resolution.name source=Value target=Value

        return this.val;
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=WithAccess<&read.'a Box<Value#2>, A>
        /// @resolution.place source=this placement=read.'a lifetime=read.'a access=A
        /// @resolution.access source=this root=this
        /// @resolution.rejected source=this.val

    }
}
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'val' does not exist on type 'WithAccess<&'a Box<Value>, A>'; did you mean 'value'?"
/// @diagnostic.label line=8 column=21 span="val" line_source="return this.val;"
/// @diagnostic.suggestion message="rename to 'value'" applicability=dangerous patched="return this.value;"
"#,
    );
}

#[test]
fn test_apply_conditional_extensions_through_bound_only_parameters() {
    let session = TestSession::single(
        r#"
newtype interface It<T, R = void> {
    next(this): R {
        todo("next")
    }

    find(this): T | undefined {
        todo("find")
    }

    map<U>(this, transform: (value: T) => U): Wrap<this, T, U> {
        todo("map")
    }
}

struct Wrap<I, T, U> {
    source: I;
    transform: (value: T) => U;
}

extension<T, U, R, I: It<T, R>> of Wrap<I, T, U> implements It<U> {
    next(): void {
        todo("next")
    }
}

function firstDefined(values: It<int32>): int32 | undefined {
    return values.map((value) => value).find();
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked().with_node_types(), r#"
=== annotated ===
newtype interface It<in out T, out R = void> {
    next(this): R {
        todo("next" as string | undefined)
    }

    find(this): T | undefined {
        todo("find" as string | undefined)
    }

    map<U>(this, transform: (value: T) => U): Wrap<this, T, U> {
        todo("map" as string | undefined)
    }
}

struct Wrap<out I, in T, out U> {
    source: I;
    transform: (value: T) => U;
}

extension<T, U, R, I: It<T, R>> of Wrap<I, T, U> implements It<U> {
    next(): void {
        todo("next" as string | undefined);
    }
}

function firstDefined(values: It<int32>): int32 | undefined {
    return values.map<int32, void, int32>((value: int32): int32 => value).find<
        int32,
        void,
        int32,
        int32,
        void,
        It<int32>
    >();
}

=== dir ===
newtype interface It<T, R = void> {
/// @generic.template symbol=It parameters=(in out T#1, out R#1 = void, this: It<T#1, R#1>)
/// @type.symbol symbol=It type=It
/// @definition.interface symbol=It template=(in out T#1, out R#1 = void, this: It<T#1, R#1>) nominal=true
/// @definition.where symbol=It relation=satisfies left=this right=It<T#1, R#1>
/// @definition.method symbol=It.find slot=find type=(this: this) => T#1 | undefined
/// @definition.method symbol=It.map slot=map type=<U#1>(this: this, (T#1) => U#1) => Wrap<this, T#1, U#1>
/// @definition.method symbol=It.next slot=next type=(this: this) => R#1
/// @type.symbol symbol=It.T source=T type=T#1
/// @type.symbol symbol=It.R source="R = void" type=R#1

    next(this): R {
    /// @type.symbol symbol=It.next type=(this: this) => R#1
    /// @type.symbol symbol=It.next.this source=this type=this
    /// @resolution.name source=R target=It.R

        todo("next")
        /// @type.node source="todo(\"next\")" type=never
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"next\")" parameters=(string | undefined) arguments=(provided("next") as string | undefined) return=never kind=symbol target=todo
        /// @type.node source="\"next\"" type="next"

    }

    find(this): T | undefined {
    /// @type.symbol symbol=It.find type=(this: this) => T#1 | undefined
    /// @type.symbol symbol=It.find.this source=this type=this
    /// @resolution.name source=T target=It.T

        todo("find")
        /// @type.node source="todo(\"find\")" type=never
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"find\")" parameters=(string | undefined) arguments=(provided("find") as string | undefined) return=never kind=symbol target=todo
        /// @type.node source="\"find\"" type="find"

    }

    map<U>(this, transform: (value: T) => U): Wrap<this, T, U> {
    /// @generic.template symbol=It.map parent=template#0 parameters=(U#1)
    /// @type.symbol symbol=It.map type=<U#1>(this: this, (T#1) => U#1) => Wrap<this, T#1, U#1>
    /// @generic.instance id="Wrap<this, T#1, U#1>" template=Wrap arguments=(this, T#1, U#1)
    /// @type.symbol symbol=It.map.U source=U type=U#1
    /// @type.symbol symbol=It.map.this source=this type=this
    /// @type.symbol symbol=It.map.transform source="transform: (value: T) => U" type=(T#1) => U#1
    /// @type.symbol symbol=It.map.value source="value: T" type=T#1
    /// @resolution.name source=T target=It.T
    /// @resolution.name source=U target=It.map.U
    /// @resolution.name source=Wrap target=Wrap
    /// @resolution.name source=T target=It.T
    /// @resolution.name source=U target=It.map.U

        todo("map")
        /// @type.node source="todo(\"map\")" type=never
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"map\")" parameters=(string | undefined) arguments=(provided("map") as string | undefined) return=never kind=symbol target=todo
        /// @type.node source="\"map\"" type="map"

    }
}

struct Wrap<I, T, U> {
/// @generic.template symbol=Wrap parameters=(out I#1, in T#2, out U#2)
/// @type.symbol symbol=Wrap type=Wrap
/// @definition.struct symbol=Wrap template=(out I#1, in T#2, out U#2)
/// @definition.field symbol=Wrap.source source="source: I" key=source type=I#1
/// @definition.field symbol=Wrap.transform source="transform: (value: T) => U" key=transform type=(T#2) => U#2
/// @type.symbol symbol=Wrap.I source=I type=I#1
/// @type.symbol symbol=Wrap.T source=T type=T#2
/// @type.symbol symbol=Wrap.U source=U type=U#2

    source: I;
    /// @type.symbol symbol=Wrap.source source="source: I" type=I#1
    /// @resolution.name source=I target=Wrap.I

    transform: (value: T) => U;
    /// @type.symbol symbol=Wrap.transform source="transform: (value: T) => U" type=(T#2) => U#2
    /// @type.symbol symbol=Wrap.value source="value: T" type=T#2
    /// @resolution.name source=T target=Wrap.T
    /// @resolution.name source=U target=Wrap.U

}

extension<T, U, R, I: It<T, R>> of Wrap<I, T, U> implements It<U> {
/// @generic.template symbol=<module>#2 parameters=(T#3, U#3, R#2, I#2: It<T#3, R#2>)
/// @generic.instance id="It<U#3, void>" template=It arguments=(U#3, void)
/// @generic.instance id="Wrap<I#2, T#3, U#3>" template=Wrap arguments=(I#2, T#3, U#3)
/// @definition.extension symbol=<module>#2 form=local target=Wrap<I#2, T#3, U#3>
/// @definition.implements symbol=<module>#2 source=It<U> target="It<U#3, void>"
/// @definition.method symbol=next slot=next type=<next.'a>(this: &next.'a readonly Wrap<I#2, T#3, U#3>) => void
/// @definition.conformance symbol=<module>#2 member=It.find requirement=It.find
/// @definition.conformance symbol=<module>#2 member=It.map requirement=It.map
/// @definition.conformance symbol=<module>#2 member=next requirement=It.next
/// @type.symbol symbol=T source=T type=T#3
/// @type.symbol symbol=U source=U type=U#3
/// @type.symbol symbol=R source=R type=R#2
/// @type.symbol symbol=I source="I: It<T, R>" type=I#2
/// @resolution.name source=It target=It
/// @generic.instance id="It<T#3, R#2>" template=It arguments=(T#3, R#2)
/// @resolution.name source=T target=T
/// @resolution.name source=R target=R
/// @resolution.name source=Wrap target=Wrap
/// @resolution.name source=I target=I
/// @resolution.name source=T target=T
/// @resolution.name source=U target=U
/// @resolution.name source=It target=It
/// @resolution.name source=U target=U

    next(): void {
    /// @generic.template symbol=next parent=template#2 parameters=('a)
    /// @type.symbol symbol=next type=<next.'a>(this: &next.'a readonly Wrap<I#2, T#3, U#3>) => void
    /// @type.symbol symbol=next.this type=&next.'a readonly Wrap<I#2, T#3, U#3>

        todo("next")
        /// @type.node source="todo(\"next\")" type=never
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"next\")" parameters=(string | undefined) arguments=(provided("next") as string | undefined) return=never kind=symbol target=todo
        /// @type.node source="\"next\"" type="next"

    }
}

function firstDefined(values: It<int32>): int32 | undefined {
/// @type.symbol symbol=firstDefined type=(It<int32, void>) => int32 | undefined
/// @generic.instance id="It<int32, void>" template=It arguments=(int32, void)
/// @type.symbol symbol=firstDefined.values source="values: It<int32>" type=It<int32, void>
/// @resolution.name source=It target=It

    return values.map((value) => value).find();
    /// @type.node source="values.map((value) => value)" type=Wrap<It<int32, void>, int32, int32>
    /// @type.node source="values.map((value) => value).find" type=(this: Wrap<It<int32, void>, int32, int32>) => int32 | undefined
    /// @type.node source="values.map((value) => value).find()" type=int32 | undefined
    /// @type.node source=values.map type=<U#1>(this: It<int32, void>, (int32) => U#1) => Wrap<It<int32, void>, int32, U#1>
    /// @resolution.name source=values target=firstDefined.values
    /// @resolution.member source="values.map((value) => value).find" receiver=Wrap<It<int32, void>, int32, int32> type=(this: Wrap<It<int32, void>, int32, int32>) => int32 | undefined kind=symbol target_receiver=Wrap<It<int32, void>, int32, int32> target=It.find
    /// @resolution.member source=values.map receiver=It<int32, void> type=<U#1>(this: It<int32, void>, (int32) => U#1) => Wrap<It<int32, void>, int32, U#1> kind=symbol target_receiver=It<int32, void> dispatch=dynamic constraint=It<int32, void> target=It.map
    /// @resolution.call source="values.map((value) => value)" parameters=((int32) => int32) arguments=(provided((value) => value) as (int32) => int32) return=Wrap<It<int32, void>, int32, int32> kind=dynamic target=It.map receiver=It<int32, void> constraint=It<int32, void> generic_arguments=(int32, void, int32)
    /// @resolution.call source="values.map((value) => value).find()" parameters=() return=int32 | undefined kind=symbol target=It.find receiver=Wrap<It<int32, void>, int32, int32> instance="Wrap<It<int32, void>, int32, int32>.<extension#1>.find"
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=firstDefined.values
    /// @generic.instantiation id="It.find<int32, void, int32, int32, void, It<int32, void>>" template=It.find arguments=(int32, void, int32, int32, void, It<int32, void>)
    /// @generic.instantiation id="It.map<int32, void>" template=It.map arguments=(int32, void)
    /// @generic.instance id="Wrap<It<int32, void>, int32, int32>" template=Wrap arguments=(It<int32, void>, int32, int32)
    /// @type.symbol symbol=firstDefined.symbol34 source="(value) => value" type=Function<(int32,), int32, "readonly">
    /// @type.node source="(value) => value" type=Function<(int32,), int32, "readonly">
    /// @type.symbol symbol=firstDefined.symbol34.value source=value type=int32
    /// @resolution.name source=value target=firstDefined.symbol34.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=firstDefined.symbol34.value

}
"#);
}
