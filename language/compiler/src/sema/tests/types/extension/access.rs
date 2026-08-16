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
        this.view<A>()
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
/// @resolution.name source=Access target=memory.access.Access
/// @resolution.name source=Grid target=Grid

    view(this: WithAccess<&Grid, A>): int32 {
    /// @generic.template symbol=view parent=template#0 parameters=('a)
    /// @type.symbol symbol=view type=<view.'a>(this: WithAccess<&view.'a Grid, A>) => int32
    /// @type.symbol symbol=view.this source="this: WithAccess<&Grid, A>" type=WithAccess<&view.'a Grid, A>
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=Grid target=Grid
    /// @resolution.name source=A target=A

        this.size
        /// @type.node source=this type=WithAccess<&view.'a Grid, A>
        /// @type.node source=this.size type=int32
        /// @resolution.member source=this.size receiver=WithAccess<&view.'a Grid, A> type=int32 kind=field target_receiver=WithAccess<&view.'a Grid, A> key=size target=Grid.size target_type=int32
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=WithAccess<&view.'a Grid, A>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.size placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.size root=this keys=[size]

    }

    peek(this: WithAccess<&Grid, A>): int32 {
    /// @generic.template symbol=peek parent=template#0 parameters=('a)
    /// @type.symbol symbol=peek type=<peek.'a>(this: WithAccess<&peek.'a Grid, A>) => int32
    /// @type.symbol symbol=peek.this source="this: WithAccess<&Grid, A>" type=WithAccess<&peek.'a Grid, A>
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=Grid target=Grid
    /// @resolution.name source=A target=A

        this.view()
        /// @type.node source=this type=WithAccess<&peek.'a Grid, A>
        /// @type.node source=this.view type=<view.'a>(this: WithAccess<&view.'a Grid, A>) => int32
        /// @type.node source=this.view() type=int32
        /// @resolution.member source=this.view receiver=WithAccess<&peek.'a Grid, A> type=<view.'a>(this: WithAccess<&view.'a Grid, A>) => int32 kind=symbol target_receiver=WithAccess<&peek.'a Grid, A> target=view
        /// @resolution.call source=this.view() parameters=() return=int32 kind=symbol target=view receiver=WithAccess<&peek.'a Grid, A> instance=Grid.<extension#1>.view
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=WithAccess<&peek.'a Grid, A>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id=view<A> template=view arguments=(A) owner=<module>#2

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

function write(grid: &exclusive Grid): int32 {
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
    grid.view<"readonly">()
}

function write<'a>(grid: &'a exclusive Grid): int32 {
    grid.view<"exclusive">()
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
/// @resolution.name source=Access target=memory.access.Access
/// @resolution.name source=Grid target=Grid

    view(this: WithAccess<&Grid, A>): int32 {
    /// @generic.template symbol=view parent=template#0 parameters=('a)
    /// @type.symbol symbol=view type=<view.'a>(this: WithAccess<&view.'a Grid, A>) => int32
    /// @type.symbol symbol=view.this source="this: WithAccess<&Grid, A>" type=WithAccess<&view.'a Grid, A>
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=Grid target=Grid
    /// @resolution.name source=A target=A

        this.size
        /// @type.node source=this type=WithAccess<&view.'a Grid, A>
        /// @type.node source=this.size type=int32
        /// @resolution.member source=this.size receiver=WithAccess<&view.'a Grid, A> type=int32 kind=field target_receiver=WithAccess<&view.'a Grid, A> key=size target=Grid.size target_type=int32
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=WithAccess<&view.'a Grid, A>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.size placement="local" lifetime="frame" access="exclusive"
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
    /// @type.node source=grid.view type=<view.'a>(this: WithAccess<&view.'a Grid, A>) => int32
    /// @type.node source=grid.view() type=int32
    /// @resolution.name source=grid target=read.grid
    /// @resolution.member source=grid.view receiver=&read.'a readonly Grid type=<view.'a>(this: WithAccess<&view.'a Grid, A>) => int32 kind=symbol target_receiver=&read.'a readonly Grid target=view
    /// @resolution.call source=grid.view() parameters=() return=int32 kind=symbol target=view receiver=&read.'a readonly Grid instance=Grid.<extension#1>.view
    /// @resolution.place source=grid placement="local" lifetime=read.'a access="readonly"
    /// @resolution.access source=grid root=read.grid
    /// @generic.instantiation id="view<\"readonly\">" template=view arguments=("readonly")
    /// @generic.instance id="WithAccess<&'frame Grid, \"readonly\">" template=memory.type.WithAccess arguments=(&'frame Grid, "readonly")
    /// @generic.instance id="view<\"readonly\">" template=view arguments=("readonly")

}

function write(grid: &exclusive Grid): int32 {
/// @generic.template symbol=write parameters=('a)
/// @type.symbol symbol=write type=<write.'a>(&write.'a exclusive Grid) => int32
/// @type.symbol symbol=write.grid source="grid: &exclusive Grid" type=&write.'a exclusive Grid
/// @resolution.name source=Grid target=Grid

    grid.view()
    /// @type.node source=grid type=&write.'a exclusive Grid
    /// @type.node source=grid.view type=<view.'a>(this: WithAccess<&view.'a Grid, A>) => int32
    /// @type.node source=grid.view() type=int32
    /// @resolution.name source=grid target=write.grid
    /// @resolution.member source=grid.view receiver=&write.'a exclusive Grid type=<view.'a>(this: WithAccess<&view.'a Grid, A>) => int32 kind=symbol target_receiver=&write.'a exclusive Grid target=view
    /// @resolution.call source=grid.view() parameters=() return=int32 kind=symbol target=view receiver=&write.'a exclusive Grid instance=Grid.<extension#1>.view
    /// @resolution.place source=grid placement="local" lifetime=write.'a access="exclusive"
    /// @resolution.access source=grid root=write.grid
    /// @generic.instantiation id="view<\"exclusive\">" template=view arguments=("exclusive")
    /// @generic.instance id="WithAccess<&'frame Grid, \"exclusive\">" template=memory.type.WithAccess arguments=(&'frame Grid, "exclusive")
    /// @generic.instance id="view<\"exclusive\">" template=view arguments=("exclusive")

}
"#,
    );
}

#[test]
fn test_access_generic_receiver_selects_fixed_array_sibling_method() {
    let session = TestSession::single(
        r#"
export extension FixedArrayAccess<T, const N: usize, const A: Access = "readonly"> of [T; N] {
    view(this: WithAccess<&[T; N], A>): int32 {
        1
    }

    peek(this: WithAccess<&[T; N], A>): int32 {
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
export extension FixedArrayAccess<T, const N: usize, const A: Access = "readonly"> of [T; N] {
    view(this: WithAccess<&[T; N], A>): int32 {
        1
    }

    peek(this: WithAccess<&[T; N], A>): int32 {
        this.view<T, N, A>()
    }
}

=== dir ===
export extension FixedArrayAccess<T, const N: usize, const A: Access = "readonly"> of [T; N] {
/// @generic.template symbol=FixedArrayAccess parameters=(T, const N: usize, const A: Access = "readonly")
/// @definition.extension symbol=FixedArrayAccess form=exported target=FixedArray<T, N>
/// @definition.method symbol=FixedArrayAccess.peek slot=peek type=<FixedArrayAccess.peek.'a>(this: WithAccess<&FixedArrayAccess.peek.'a FixedArray<T, N>, A>) => int32
/// @definition.method symbol=FixedArrayAccess.view slot=view type=<FixedArrayAccess.view.'a>(this: WithAccess<&FixedArrayAccess.view.'a FixedArray<T, N>, A>) => int32
/// @type.symbol symbol=FixedArrayAccess.T source=T type=T
/// @type.symbol symbol=FixedArrayAccess.N source="const N: usize" type=N
/// @type.symbol symbol=FixedArrayAccess.A source="const A: Access = \"readonly\"" type=A
/// @resolution.name source=Access target=memory.access.Access
/// @resolution.name source=T target=FixedArrayAccess.T
/// @resolution.name source=N target=FixedArrayAccess.N

    view(this: WithAccess<&[T; N], A>): int32 {
    /// @generic.template symbol=FixedArrayAccess.view parent=template#0 parameters=('a)
    /// @type.symbol symbol=FixedArrayAccess.view type=<FixedArrayAccess.view.'a>(this: WithAccess<&FixedArrayAccess.view.'a FixedArray<T, N>, A>) => int32
    /// @type.symbol symbol=FixedArrayAccess.view.this source="this: WithAccess<&[T; N], A>" type=WithAccess<&FixedArrayAccess.view.'a FixedArray<T, N>, A>
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=T target=FixedArrayAccess.T
    /// @resolution.name source=N target=FixedArrayAccess.N
    /// @resolution.name source=A target=FixedArrayAccess.A

        1
        /// @type.node source=1 type=1

    }

    peek(this: WithAccess<&[T; N], A>): int32 {
    /// @generic.template symbol=FixedArrayAccess.peek parent=template#0 parameters=('a)
    /// @type.symbol symbol=FixedArrayAccess.peek type=<FixedArrayAccess.peek.'a>(this: WithAccess<&FixedArrayAccess.peek.'a FixedArray<T, N>, A>) => int32
    /// @type.symbol symbol=FixedArrayAccess.peek.this source="this: WithAccess<&[T; N], A>" type=WithAccess<&FixedArrayAccess.peek.'a FixedArray<T, N>, A>
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=T target=FixedArrayAccess.T
    /// @resolution.name source=N target=FixedArrayAccess.N
    /// @resolution.name source=A target=FixedArrayAccess.A

        this.view()
        /// @type.node source=this type=WithAccess<&FixedArrayAccess.peek.'a FixedArray<T, N>, A>
        /// @type.node source=this.view type=<FixedArrayAccess.view.'a>(this: WithAccess<&FixedArrayAccess.view.'a FixedArray<T, N>, A>) => int32 & <const collections.fixed-array.view.A: Access = "readonly", collections.fixed-array.view.'a>(this: WithAccess<&collections.fixed-array.view.'a FixedArray<T, N>, collections.fixed-array.view.A>, isize, isize | undefined?) => WithAccess<&collections.fixed-array.view.'a Slice<T>, collections.fixed-array.view.A>
        /// @type.node source=this.view() type=int32
        /// @resolution.member source=this.view receiver=WithAccess<&FixedArrayAccess.peek.'a FixedArray<T, N>, A> type=<FixedArrayAccess.view.'a>(this: WithAccess<&FixedArrayAccess.view.'a FixedArray<T, N>, A>) => int32 & <const collections.fixed-array.view.A: Access = "readonly", collections.fixed-array.view.'a>(this: WithAccess<&collections.fixed-array.view.'a FixedArray<T, N>, collections.fixed-array.view.A>, isize, isize | undefined?) => WithAccess<&collections.fixed-array.view.'a Slice<T>, collections.fixed-array.view.A> kind=existential targets=[FixedArrayAccess.view, collections.fixed-array.view]
        /// @resolution.call source=this.view() parameters=() return=int32 kind=symbol target=FixedArrayAccess.view receiver=WithAccess<&FixedArrayAccess.peek.'a FixedArray<T, N>, A> instance="FixedArrayAccess<T, N, A>.view"
        /// @resolution.receiver source=this kind=this declaration=FixedArrayAccess type=WithAccess<&FixedArrayAccess.peek.'a FixedArray<T, N>, A>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id="FixedArrayAccess.view<T, N, A>" template=FixedArrayAccess.view arguments=(T, N, A) owner=FixedArrayAccess
        /// @generic.instantiation id="FixedArrayAccess.view<T, N>" template=FixedArrayAccess.view arguments=(T, N) owner=FixedArrayAccess
        /// @generic.instantiation id="collections.fixed-array.view<T, N>" template=collections.fixed-array.view arguments=(T, N) owner=FixedArrayAccess

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
    view(this: WithAccess<&Array<T>, A>): int32 {
        1
    }

    peek(this: WithAccess<&Array<T>, A>): int32 {
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
export extension ArrayAccess<T, const A: Access = "readonly"> of Array<T> {
    view(this: WithAccess<&Array<T>, A>): int32 {
        1
    }

    peek(this: WithAccess<&Array<T>, A>): int32 {
        this.view<T, A>()
    }
}

=== dir ===
export extension ArrayAccess<T, const A: Access = "readonly"> of Array<T> {
/// @generic.template symbol=ArrayAccess parameters=(T, const A: Access = "readonly")
/// @definition.extension symbol=ArrayAccess form=exported target=Array<T>
/// @definition.method symbol=ArrayAccess.peek slot=peek type=<ArrayAccess.peek.'a>(this: WithAccess<&ArrayAccess.peek.'a Array<T>, A>) => int32
/// @definition.method symbol=ArrayAccess.view slot=view type=<ArrayAccess.view.'a>(this: WithAccess<&ArrayAccess.view.'a Array<T>, A>) => int32
/// @type.symbol symbol=ArrayAccess.T source=T type=T
/// @type.symbol symbol=ArrayAccess.A source="const A: Access = \"readonly\"" type=A
/// @resolution.name source=Access target=memory.access.Access
/// @resolution.name source=Array target=collections.array.Array
/// @resolution.name source=T target=ArrayAccess.T

    view(this: WithAccess<&Array<T>, A>): int32 {
    /// @generic.template symbol=ArrayAccess.view parent=template#0 parameters=('a)
    /// @type.symbol symbol=ArrayAccess.view type=<ArrayAccess.view.'a>(this: WithAccess<&ArrayAccess.view.'a Array<T>, A>) => int32
    /// @type.symbol symbol=ArrayAccess.view.this source="this: WithAccess<&Array<T>, A>" type=WithAccess<&ArrayAccess.view.'a Array<T>, A>
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=Array target=collections.array.Array
    /// @resolution.name source=T target=ArrayAccess.T
    /// @resolution.name source=A target=ArrayAccess.A

        1
        /// @type.node source=1 type=1

    }

    peek(this: WithAccess<&Array<T>, A>): int32 {
    /// @generic.template symbol=ArrayAccess.peek parent=template#0 parameters=('a)
    /// @type.symbol symbol=ArrayAccess.peek type=<ArrayAccess.peek.'a>(this: WithAccess<&ArrayAccess.peek.'a Array<T>, A>) => int32
    /// @type.symbol symbol=ArrayAccess.peek.this source="this: WithAccess<&Array<T>, A>" type=WithAccess<&ArrayAccess.peek.'a Array<T>, A>
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=Array target=collections.array.Array
    /// @resolution.name source=T target=ArrayAccess.T
    /// @resolution.name source=A target=ArrayAccess.A

        this.view()
        /// @type.node source=this type=WithAccess<&ArrayAccess.peek.'a Array<T>, A>
        /// @type.node source=this.view type=<ArrayAccess.view.'a>(this: WithAccess<&ArrayAccess.view.'a Array<T>, A>) => int32 & <const collections.array.view.A: Access = "readonly", collections.array.view.'a>(this: WithAccess<&collections.array.view.'a Array<T>, collections.array.view.A>, isize, isize | undefined?) => WithAccess<&collections.array.view.'a Slice<T>, collections.array.view.A>
        /// @type.node source=this.view() type=int32
        /// @resolution.member source=this.view receiver=WithAccess<&ArrayAccess.peek.'a Array<T>, A> type=<ArrayAccess.view.'a>(this: WithAccess<&ArrayAccess.view.'a Array<T>, A>) => int32 & <const collections.array.view.A: Access = "readonly", collections.array.view.'a>(this: WithAccess<&collections.array.view.'a Array<T>, collections.array.view.A>, isize, isize | undefined?) => WithAccess<&collections.array.view.'a Slice<T>, collections.array.view.A> kind=existential targets=[ArrayAccess.view, collections.array.view]
        /// @resolution.call source=this.view() parameters=() return=int32 kind=symbol target=ArrayAccess.view receiver=WithAccess<&ArrayAccess.peek.'a Array<T>, A> instance="ArrayAccess<T, A>.view"
        /// @resolution.receiver source=this kind=this declaration=ArrayAccess type=WithAccess<&ArrayAccess.peek.'a Array<T>, A>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id="ArrayAccess.view<T, A>" template=ArrayAccess.view arguments=(T, A) owner=ArrayAccess
        /// @generic.instantiation id=ArrayAccess.view<T> template=ArrayAccess.view arguments=(T) owner=ArrayAccess
        /// @generic.instantiation id=collections.array.view<T> template=collections.array.view arguments=(T) owner=ArrayAccess

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
/// @resolution.name source=Access target=memory.access.Access
/// @resolution.name source=Box target=Box
/// @resolution.name source=Value target=Value

    read(this: WithAccess<&Box<Value>, A>): Value {
    /// @generic.template symbol=read parent=template#1 parameters=('a)
    /// @type.symbol symbol=read type=<read.'a>(this: WithAccess<&read.'a Box<Value#2>, A>) => Value#2
    /// @type.symbol symbol=read.this source="this: WithAccess<&Box<Value>, A>" type=WithAccess<&read.'a Box<Value#2>, A>
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=Box target=Box
    /// @resolution.name source=Value target=Value
    /// @resolution.name source=A target=A
    /// @resolution.name source=Value target=Value

        return this.val;
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=WithAccess<&read.'a Box<Value#2>, A>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
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
