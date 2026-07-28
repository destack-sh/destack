use crate::tests::{DirRows, TestSession};

#[test]
fn test_access_generic_receiver_selects_a_sibling_method() {
    let session = TestSession::single(
        r#"
struct Grid {
    size: int32;
}

export extension<comptime A: Access = "readonly"> of Grid {
    view(this: WithAccess<&Grid, A>): int32 {
        this.size
    }

    peek(this: WithAccess<&Grid, A>): int32 {
        this.view()
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Grid {
    size: int32;
}

export extension<comptime A: Access = "readonly"> of Grid {
    view(this: WithAccess<&Grid, A>): int32 {
        this.size
    }

    peek(this: WithAccess<&Grid, A>): int32 {
        this.view<A>()
    }
}

=== checked ===
struct Grid {
/// @type.symbol symbol=Grid type=Grid
/// @definition.struct symbol=Grid
/// @definition.field symbol=Grid.size source="size: int32" key=size type=int32

    size: int32;
    /// @type.symbol symbol=Grid.size source="size: int32" type=int32

}

export extension<comptime A: Access = "readonly"> of Grid {
/// @generic.template symbol=<module>#2 parameters=(comptime A: Access = "readonly")
/// @definition.extension symbol=<module>#2 form=exported target=Grid
/// @definition.method symbol=peek slot=peek type=<peek.'a>(this: WithAccess<&peek.'a Grid, A>) => int32
/// @definition.method symbol=view slot=view type=<view.'a>(this: WithAccess<&view.'a Grid, A>) => int32
/// @type.symbol symbol=A source="comptime A: Access = \"readonly\"" type=A
/// @resolution.name source=Access target=memory.access.Access
/// @type.node source="\"readonly\"" type="readonly"
/// @resolution.name source=Grid target=Grid

    view(this: WithAccess<&Grid, A>): int32 {
    /// @generic.template symbol=view parent=template#0 parameters=('a)
    /// @type.symbol symbol=view type=<view.'a>(this: WithAccess<&view.'a Grid, A>) => int32 reduced=<view.'a>(this: Borrowed<Grid, view.'a, A>) => int32
    /// @type.symbol symbol=view.this source="this: WithAccess<&Grid, A>" type=WithAccess<&view.'a Grid, A> reduced=Borrowed<Grid, view.'a, A>
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=Grid target=Grid
    /// @resolution.name source=A target=A

        this.size
        /// @type.node source=this type=WithAccess<&view.'a Grid, A> reduced=Borrowed<Grid, view.'a, A>
        /// @type.node source=this.size type=int32
        /// @resolution.member source=this.size receiver=Borrowed<Grid, view.'a, A> type=int32 kind=field target_receiver=Borrowed<Grid, view.'a, A> key=size target=Grid.size target_type=int32
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=WithAccess<&view.'a Grid, A>
        /// @resolution.place source=this placement="local" lifetime=view.'a access=A
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.size placement="local" lifetime=view.'a access=A
        /// @resolution.access source=this.size root=this keys=[size]
        /// @generic.instance source=this id="WithAccess<&view.'a Grid, A>"

    }

    peek(this: WithAccess<&Grid, A>): int32 {
    /// @generic.template symbol=peek parent=template#0 parameters=('a)
    /// @type.symbol symbol=peek type=<peek.'a>(this: WithAccess<&peek.'a Grid, A>) => int32 reduced=<peek.'a>(this: Borrowed<Grid, peek.'a, A>) => int32
    /// @type.symbol symbol=peek.this source="this: WithAccess<&Grid, A>" type=WithAccess<&peek.'a Grid, A> reduced=Borrowed<Grid, peek.'a, A>
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=Grid target=Grid
    /// @resolution.name source=A target=A

        this.view()
        /// @type.node source=this type=WithAccess<&peek.'a Grid, A> reduced=Borrowed<Grid, peek.'a, A>
        /// @type.node source=this.view type=<view.'a>(this: WithAccess<&view.'a Grid, A>) => int32 reduced=<view.'a>(this: Borrowed<Grid, view.'a, A>) => int32
        /// @type.node source=this.view() type=int32
        /// @resolution.member source=this.view receiver=Borrowed<Grid, peek.'a, A> type=<view.'a>(this: WithAccess<&view.'a Grid, A>) => int32 kind=symbol target_receiver=Borrowed<Grid, peek.'a, A> target=view
        /// @resolution.call source=this.view() parameters=() return=int32 kind=symbol target=view receiver=Borrowed<Grid, peek.'a, A> instance=Grid.<extension#1>.view
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=WithAccess<&peek.'a Grid, A>
        /// @resolution.place source=this placement="local" lifetime=peek.'a access=A
        /// @resolution.access source=this root=this
        /// @generic.instance source=this id="WithAccess<&peek.'a Grid, A>"
        /// @generic.instance source=this.view id="WithAccess<&view.'a Grid, A>"
        /// @generic.instance source=this.view() id=Grid.<extension#1>.view

    }
}

/// @generic.instance id="WithAccess<&peek.'a Grid, A>" template=memory.type.WithAccess arguments=(&peek.'a Grid, A)
/// @generic.instance id="WithAccess<&view.'a Grid, A>" template=memory.type.WithAccess arguments=(&view.'a Grid, A)
/// @generic.instance id=Grid.<extension#1>.view template=view arguments=(A)
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

export extension<comptime A: Access = "readonly"> of Grid {
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Grid {
    size: int32;
}

export extension<comptime A: Access = "readonly"> of Grid {
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

=== checked ===
struct Grid {
/// @type.symbol symbol=Grid type=Grid
/// @definition.struct symbol=Grid
/// @definition.field symbol=Grid.size source="size: int32" key=size type=int32

    size: int32;
    /// @type.symbol symbol=Grid.size source="size: int32" type=int32

}

export extension<comptime A: Access = "readonly"> of Grid {
/// @generic.template symbol=<module>#2 parameters=(comptime A: Access = "readonly")
/// @definition.extension symbol=<module>#2 form=exported target=Grid
/// @definition.method symbol=view slot=view type=<view.'a>(this: WithAccess<&view.'a Grid, A>) => int32
/// @type.symbol symbol=A source="comptime A: Access = \"readonly\"" type=A
/// @resolution.name source=Access target=memory.access.Access
/// @type.node source="\"readonly\"" type="readonly"
/// @resolution.name source=Grid target=Grid

    view(this: WithAccess<&Grid, A>): int32 {
    /// @generic.template symbol=view parent=template#0 parameters=('a)
    /// @type.symbol symbol=view type=<view.'a>(this: WithAccess<&view.'a Grid, A>) => int32 reduced=<view.'a>(this: Borrowed<Grid, view.'a, A>) => int32
    /// @type.symbol symbol=view.this source="this: WithAccess<&Grid, A>" type=WithAccess<&view.'a Grid, A> reduced=Borrowed<Grid, view.'a, A>
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=Grid target=Grid
    /// @resolution.name source=A target=A

        this.size
        /// @type.node source=this type=WithAccess<&view.'a Grid, A> reduced=Borrowed<Grid, view.'a, A>
        /// @type.node source=this.size type=int32
        /// @resolution.member source=this.size receiver=Borrowed<Grid, view.'a, A> type=int32 kind=field target_receiver=Borrowed<Grid, view.'a, A> key=size target=Grid.size target_type=int32
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=WithAccess<&view.'a Grid, A>
        /// @resolution.place source=this placement="local" lifetime=view.'a access=A
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.size placement="local" lifetime=view.'a access=A
        /// @resolution.access source=this.size root=this keys=[size]
        /// @generic.instance source=this id="WithAccess<&view.'a Grid, A>"

    }
}

function read(grid: &readonly Grid): int32 {
/// @generic.template symbol=read parameters=('a)
/// @type.symbol symbol=read type=<read.'a>(&read.'a readonly Grid) => int32
/// @type.symbol symbol=read.grid source="grid: &readonly Grid" type=&read.'a readonly Grid
/// @resolution.name source=Grid target=Grid

    grid.view()
    /// @type.node source=grid type=&read.'a readonly Grid
    /// @type.node source=grid.view type=<view.'a>(this: WithAccess<&view.'a Grid, A>) => int32 reduced=<view.'a>(this: Borrowed<Grid, view.'a, A>) => int32
    /// @type.node source=grid.view() type=int32
    /// @resolution.name source=grid target=read.grid
    /// @resolution.member source=grid.view receiver=&read.'a readonly Grid type=<view.'a>(this: WithAccess<&view.'a Grid, A>) => int32 kind=symbol target_receiver=&read.'a readonly Grid target=view
    /// @resolution.call source=grid.view() parameters=() return=int32 kind=symbol target=view receiver=&read.'a readonly Grid instance=Grid.<extension#1>.view
    /// @resolution.place source=grid placement="local" lifetime=read.'a access="readonly"
    /// @resolution.access source=grid root=read.grid
    /// @generic.instance source=grid.view id="WithAccess<&view.'a Grid, A>"
    /// @generic.instance source=grid.view() id=Grid.<extension#1>.view

}

function write(grid: &exclusive Grid): int32 {
/// @generic.template symbol=write parameters=('a)
/// @type.symbol symbol=write type=<write.'a>(&write.'a exclusive Grid) => int32
/// @type.symbol symbol=write.grid source="grid: &exclusive Grid" type=&write.'a exclusive Grid
/// @resolution.name source=Grid target=Grid

    grid.view()
    /// @type.node source=grid type=&write.'a exclusive Grid
    /// @type.node source=grid.view type=<view.'a>(this: WithAccess<&view.'a Grid, A>) => int32 reduced=<view.'a>(this: Borrowed<Grid, view.'a, A>) => int32
    /// @type.node source=grid.view() type=int32
    /// @resolution.name source=grid target=write.grid
    /// @resolution.member source=grid.view receiver=&write.'a exclusive Grid type=<view.'a>(this: WithAccess<&view.'a Grid, A>) => int32 kind=symbol target_receiver=&write.'a exclusive Grid target=view
    /// @resolution.call source=grid.view() parameters=() return=int32 kind=symbol target=view receiver=&write.'a exclusive Grid instance=Grid.<extension#1>.view
    /// @resolution.place source=grid placement="local" lifetime=write.'a access="exclusive"
    /// @resolution.access source=grid root=write.grid
    /// @generic.instance source=grid.view id="WithAccess<&view.'a Grid, A>"
    /// @generic.instance source=grid.view() id=Grid.<extension#1>.view

}

/// @generic.instance id="WithAccess<&view.'a Grid, A>" template=memory.type.WithAccess arguments=(&view.'a Grid, A)
/// @generic.instance id=Grid.<extension#1>.view template=view arguments=("exclusive")
"#,
    );
}

#[test]
fn test_access_generic_receiver_selects_fixed_array_sibling_method() {
    let session = TestSession::single(
        r#"
export extension FixedArrayAccess<T, comptime N: number, comptime A: Access = "readonly"> of [T; N] {
    view(this: WithAccess<&[T; N], A>): int32 {
        1
    }

    peek(this: WithAccess<&[T; N], A>): int32 {
        this.view()
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
export extension FixedArrayAccess<T, comptime N: number, comptime A: Access = "readonly"> of
    [T; N] {
    view(this: WithAccess<&[T; N], A>): int32 {
        1
    }

    peek(this: WithAccess<&[T; N], A>): int32 {
        this.view<T, N, A>()
    }
}

=== checked ===
export extension FixedArrayAccess<T, comptime N: number, comptime A: Access = "readonly"> of [T; N] {
/// @generic.template symbol=FixedArrayAccess parameters=(T, comptime N: float64, comptime A: Access = "readonly")
/// @definition.extension symbol=FixedArrayAccess form=exported target=FixedArray<T, N>
/// @definition.method symbol=FixedArrayAccess.peek slot=peek type=<FixedArrayAccess.peek.'a>(this: WithAccess<&FixedArrayAccess.peek.'a FixedArray<T, N>, A>) => int32
/// @definition.method symbol=FixedArrayAccess.view slot=view type=<FixedArrayAccess.view.'a>(this: WithAccess<&FixedArrayAccess.view.'a FixedArray<T, N>, A>) => int32
/// @type.symbol symbol=FixedArrayAccess.T source=T type=T
/// @type.symbol symbol=FixedArrayAccess.N source="comptime N: number" type=N
/// @type.symbol symbol=FixedArrayAccess.A source="comptime A: Access = \"readonly\"" type=A
/// @resolution.name source=Access target=memory.access.Access
/// @type.node source="\"readonly\"" type="readonly"
/// @resolution.name source=T target=FixedArrayAccess.T
/// @resolution.name source=N target=FixedArrayAccess.N

    view(this: WithAccess<&[T; N], A>): int32 {
    /// @generic.template symbol=FixedArrayAccess.view parent=template#0 parameters=('a)
    /// @type.symbol symbol=FixedArrayAccess.view type=<FixedArrayAccess.view.'a>(this: WithAccess<&FixedArrayAccess.view.'a FixedArray<T, N>, A>) => int32 reduced=<FixedArrayAccess.view.'a>(this: Borrowed<FixedArray<T, N>, FixedArrayAccess.view.'a, A>) => int32
    /// @type.symbol symbol=FixedArrayAccess.view.this source="this: WithAccess<&[T; N], A>" type=WithAccess<&FixedArrayAccess.view.'a FixedArray<T, N>, A> reduced=Borrowed<FixedArray<T, N>, FixedArrayAccess.view.'a, A>
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=T target=FixedArrayAccess.T
    /// @resolution.name source=N target=FixedArrayAccess.N
    /// @resolution.name source=A target=FixedArrayAccess.A

        1
        /// @type.node source=1 type=1

    }

    peek(this: WithAccess<&[T; N], A>): int32 {
    /// @generic.template symbol=FixedArrayAccess.peek parent=template#0 parameters=('a)
    /// @type.symbol symbol=FixedArrayAccess.peek type=<FixedArrayAccess.peek.'a>(this: WithAccess<&FixedArrayAccess.peek.'a FixedArray<T, N>, A>) => int32 reduced=<FixedArrayAccess.peek.'a>(this: Borrowed<FixedArray<T, N>, FixedArrayAccess.peek.'a, A>) => int32
    /// @type.symbol symbol=FixedArrayAccess.peek.this source="this: WithAccess<&[T; N], A>" type=WithAccess<&FixedArrayAccess.peek.'a FixedArray<T, N>, A> reduced=Borrowed<FixedArray<T, N>, FixedArrayAccess.peek.'a, A>
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=T target=FixedArrayAccess.T
    /// @resolution.name source=N target=FixedArrayAccess.N
    /// @resolution.name source=A target=FixedArrayAccess.A

        this.view()
        /// @type.node source=this type=WithAccess<&FixedArrayAccess.peek.'a FixedArray<T, N>, A> reduced=Borrowed<FixedArray<T, N>, FixedArrayAccess.peek.'a, A>
        /// @type.node source=this.view type=<FixedArrayAccess.view.'a>(this: WithAccess<&FixedArrayAccess.view.'a FixedArray<T, N>, A>) => int32 & <comptime collections.array.view.A#1: Access = "readonly", collections.array.view#1.'a>(this: WithAccess<&collections.array.view#1.'a FixedArray<T, N>, collections.array.view.A#1>, usize, usize | undefined?) => WithAccess<&collections.array.view#1.'a Slice<T>, collections.array.view.A#1> reduced=<FixedArrayAccess.view.'a>(this: Borrowed<FixedArray<T, N>, FixedArrayAccess.view.'a, A>) => int32 & <comptime collections.array.view.A#1: Access = "readonly", collections.array.view#1.'a>(this: Borrowed<FixedArray<T, N>, collections.array.view#1.'a, collections.array.view.A#1>, usize, usize | undefined?) => Borrowed<Slice<T>, collections.array.view#1.'a, collections.array.view.A#1>
        /// @type.node source=this.view() type=int32
        /// @resolution.member source=this.view receiver=Borrowed<FixedArray<T, N>, FixedArrayAccess.peek.'a, A> type=<FixedArrayAccess.view.'a>(this: WithAccess<&FixedArrayAccess.view.'a FixedArray<T, N>, A>) => int32 & <comptime collections.array.view.A#1: Access = "readonly", collections.array.view#1.'a>(this: WithAccess<&collections.array.view#1.'a FixedArray<T, N>, collections.array.view.A#1>, usize, usize | undefined?) => WithAccess<&collections.array.view#1.'a Slice<T>, collections.array.view.A#1> kind=existential targets=[FixedArrayAccess.view, collections.array.view#1]
        /// @resolution.call source=this.view() parameters=() return=int32 kind=symbol target=FixedArrayAccess.view receiver=Borrowed<FixedArray<T, N>, FixedArrayAccess.peek.'a, A> instance="FixedArrayAccess<T, N, A>.view"
        /// @resolution.receiver source=this kind=this declaration=FixedArrayAccess type=WithAccess<&FixedArrayAccess.peek.'a FixedArray<T, N>, A>
        /// @resolution.place source=this placement="local" lifetime=FixedArrayAccess.peek.'a access=A
        /// @resolution.access source=this root=this
        /// @generic.instance source=this id="WithAccess<&FixedArrayAccess.peek.'a FixedArray<T, N>, A>"
        /// @generic.instance source=this.view id="WithAccess<&FixedArrayAccess.view.'a FixedArray<T, N>, A>"
        /// @generic.instance source=this.view id="WithAccess<&collections.array.view#1.'a FixedArray<T, N>, collections.array.view.A#1>"
        /// @generic.instance source=this.view id="WithAccess<&collections.array.view#1.'a Slice<T>, collections.array.view.A#1>"
        /// @generic.instance source=this.view() id="FixedArrayAccess<T, N, A>.view"

    }
}

/// @generic.instance id="FixedArrayAccess<T, N, A>.view" template=FixedArrayAccess.view arguments=(T, N, A)
/// @generic.instance id="WithAccess<&FixedArrayAccess.peek.'a FixedArray<T, N>, A>" template=memory.type.WithAccess arguments=(&FixedArrayAccess.peek.'a FixedArray<T, N>, A)
/// @generic.instance id="WithAccess<&FixedArrayAccess.view.'a FixedArray<T, N>, A>" template=memory.type.WithAccess arguments=(&FixedArrayAccess.view.'a FixedArray<T, N>, A)
/// @generic.instance id="WithAccess<&collections.array.view#1.'a FixedArray<T, N>, collections.array.view.A#1>" template=memory.type.WithAccess arguments=(&collections.array.view#1.'a FixedArray<T, N>, collections.array.view.A#1)
/// @generic.instance id="WithAccess<&collections.array.view#1.'a Slice<T>, collections.array.view.A#1>" template=memory.type.WithAccess arguments=(&collections.array.view#1.'a Slice<T>, collections.array.view.A#1)
"#,
    );
}

#[test]
fn test_access_generic_receiver_selects_array_sibling_method() {
    let session = TestSession::single(
        r#"
export extension ArrayAccess<T, comptime A: Access = "readonly"> of Array<T> {
    view(this: WithAccess<&Array<T>, A>): int32 {
        1
    }

    peek(this: WithAccess<&Array<T>, A>): int32 {
        this.view()
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
export extension ArrayAccess<T, comptime A: Access = "readonly"> of Array<T> {
    view(this: WithAccess<&Array<T>, A>): int32 {
        1
    }

    peek(this: WithAccess<&Array<T>, A>): int32 {
        this.view<T, A>()
    }
}

=== checked ===
export extension ArrayAccess<T, comptime A: Access = "readonly"> of Array<T> {
/// @generic.template symbol=ArrayAccess parameters=(T, comptime A: Access = "readonly")
/// @definition.extension symbol=ArrayAccess form=exported target=Array<T>
/// @definition.method symbol=ArrayAccess.peek slot=peek type=<ArrayAccess.peek.'a>(this: WithAccess<&ArrayAccess.peek.'a Array<T>, A>) => int32
/// @definition.method symbol=ArrayAccess.view slot=view type=<ArrayAccess.view.'a>(this: WithAccess<&ArrayAccess.view.'a Array<T>, A>) => int32
/// @type.symbol symbol=ArrayAccess.T source=T type=T
/// @type.symbol symbol=ArrayAccess.A source="comptime A: Access = \"readonly\"" type=A
/// @resolution.name source=Access target=memory.access.Access
/// @type.node source="\"readonly\"" type="readonly"
/// @resolution.name source=Array target=collections.array.Array
/// @resolution.name source=T target=ArrayAccess.T

    view(this: WithAccess<&Array<T>, A>): int32 {
    /// @generic.template symbol=ArrayAccess.view parent=template#0 parameters=('a)
    /// @type.symbol symbol=ArrayAccess.view type=<ArrayAccess.view.'a>(this: WithAccess<&ArrayAccess.view.'a Array<T>, A>) => int32 reduced=<ArrayAccess.view.'a>(this: Borrowed<Array<T>, ArrayAccess.view.'a, A>) => int32
    /// @type.symbol symbol=ArrayAccess.view.this source="this: WithAccess<&Array<T>, A>" type=WithAccess<&ArrayAccess.view.'a Array<T>, A> reduced=Borrowed<Array<T>, ArrayAccess.view.'a, A>
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=Array target=collections.array.Array
    /// @resolution.name source=T target=ArrayAccess.T
    /// @resolution.name source=A target=ArrayAccess.A

        1
        /// @type.node source=1 type=1

    }

    peek(this: WithAccess<&Array<T>, A>): int32 {
    /// @generic.template symbol=ArrayAccess.peek parent=template#0 parameters=('a)
    /// @type.symbol symbol=ArrayAccess.peek type=<ArrayAccess.peek.'a>(this: WithAccess<&ArrayAccess.peek.'a Array<T>, A>) => int32 reduced=<ArrayAccess.peek.'a>(this: Borrowed<Array<T>, ArrayAccess.peek.'a, A>) => int32
    /// @type.symbol symbol=ArrayAccess.peek.this source="this: WithAccess<&Array<T>, A>" type=WithAccess<&ArrayAccess.peek.'a Array<T>, A> reduced=Borrowed<Array<T>, ArrayAccess.peek.'a, A>
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=Array target=collections.array.Array
    /// @resolution.name source=T target=ArrayAccess.T
    /// @resolution.name source=A target=ArrayAccess.A

        this.view()
        /// @type.node source=this type=WithAccess<&ArrayAccess.peek.'a Array<T>, A> reduced=Borrowed<Array<T>, ArrayAccess.peek.'a, A>
        /// @type.node source=this.view type=<ArrayAccess.view.'a>(this: WithAccess<&ArrayAccess.view.'a Array<T>, A>) => int32 & <comptime collections.array.view.A#2: Access = "readonly", collections.array.view#2.'a>(this: WithAccess<&collections.array.view#2.'a Array<T>, collections.array.view.A#2>, usize, usize | undefined?) => WithAccess<&collections.array.view#2.'a Slice<T>, collections.array.view.A#2> reduced=<ArrayAccess.view.'a>(this: Borrowed<Array<T>, ArrayAccess.view.'a, A>) => int32 & <comptime collections.array.view.A#2: Access = "readonly", collections.array.view#2.'a>(this: Borrowed<Array<T>, collections.array.view#2.'a, collections.array.view.A#2>, usize, usize | undefined?) => Borrowed<Slice<T>, collections.array.view#2.'a, collections.array.view.A#2>
        /// @type.node source=this.view() type=int32
        /// @resolution.member source=this.view receiver=Borrowed<Array<T>, ArrayAccess.peek.'a, A> type=<ArrayAccess.view.'a>(this: WithAccess<&ArrayAccess.view.'a Array<T>, A>) => int32 & <comptime collections.array.view.A#2: Access = "readonly", collections.array.view#2.'a>(this: WithAccess<&collections.array.view#2.'a Array<T>, collections.array.view.A#2>, usize, usize | undefined?) => WithAccess<&collections.array.view#2.'a Slice<T>, collections.array.view.A#2> kind=existential targets=[ArrayAccess.view, collections.array.view#2]
        /// @resolution.call source=this.view() parameters=() return=int32 kind=symbol target=ArrayAccess.view receiver=Borrowed<Array<T>, ArrayAccess.peek.'a, A> instance="ArrayAccess<T, A>.view"
        /// @resolution.receiver source=this kind=this declaration=ArrayAccess type=WithAccess<&ArrayAccess.peek.'a Array<T>, A>
        /// @resolution.place source=this placement="local" lifetime=ArrayAccess.peek.'a access=A
        /// @resolution.access source=this root=this
        /// @generic.instance source=this id="WithAccess<&ArrayAccess.peek.'a Array<T>, A>"
        /// @generic.instance source=this id=Array<T>
        /// @generic.instance source=this.view id="WithAccess<&ArrayAccess.view.'a Array<T>, A>"
        /// @generic.instance source=this.view id="WithAccess<&collections.array.view#2.'a Array<T>, collections.array.view.A#2>"
        /// @generic.instance source=this.view id="WithAccess<&collections.array.view#2.'a Slice<T>, collections.array.view.A#2>"
        /// @generic.instance source=this.view id=Array<T>
        /// @generic.instance source=this.view() id="ArrayAccess<T, A>.view"

    }
}

/// @generic.instance id="ArrayAccess<T, A>.view" template=ArrayAccess.view arguments=(T, A)
/// @generic.instance id="WithAccess<&ArrayAccess.peek.'a Array<T>, A>" template=memory.type.WithAccess arguments=(&ArrayAccess.peek.'a Array<T>, A)
/// @generic.instance id="WithAccess<&ArrayAccess.view.'a Array<T>, A>" template=memory.type.WithAccess arguments=(&ArrayAccess.view.'a Array<T>, A)
/// @generic.instance id="WithAccess<&collections.array.view#2.'a Array<T>, collections.array.view.A#2>" template=memory.type.WithAccess arguments=(&collections.array.view#2.'a Array<T>, collections.array.view.A#2)
/// @generic.instance id="WithAccess<&collections.array.view#2.'a Slice<T>, collections.array.view.A#2>" template=memory.type.WithAccess arguments=(&collections.array.view#2.'a Slice<T>, collections.array.view.A#2)
/// @generic.instance id=Array<T> template=collections.array.Array arguments=(T)
"#,
    );
}
