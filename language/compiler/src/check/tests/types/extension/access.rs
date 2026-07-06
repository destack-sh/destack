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
        this.view<L0, A>()
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
/// @definition.method symbol=peek slot=peek type=<comptime peek.L0: Lifetime>(this: WithAccess<Borrowed<Grid, peek.L0, "mutable">, A>) => int32
/// @definition.method symbol=view slot=view type=<comptime view.L0: Lifetime>(this: WithAccess<Borrowed<Grid, view.L0, "mutable">, A>) => int32
/// @type.symbol symbol=A source="comptime A: Access = \"readonly\"" type=A
/// @resolution.name source=Access target=memory.access.Access
/// @type.node source="\"readonly\"" type="readonly"
/// @resolution.name source=Grid target=Grid

    view(this: WithAccess<&Grid, A>): int32 {
    /// @generic.template symbol=view parent=template#0 parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=view type=<comptime view.L0: Lifetime>(this: WithAccess<Borrowed<Grid, view.L0, "mutable">, A>) => int32 reduced=<comptime view.L0: Lifetime>(this: Borrowed<Grid, view.L0, A>) => int32
    /// @type.symbol symbol=view.this source="this: WithAccess<&Grid, A>" type=WithAccess<Borrowed<Grid, view.L0, "mutable">, A> reduced=Borrowed<Grid, view.L0, A>
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=Grid target=Grid
    /// @resolution.name source=A target=A

        this.size
        /// @type.node source=this type=WithAccess<Borrowed<Grid, view.L0, "mutable">, A> reduced=Borrowed<Grid, view.L0, A>
        /// @type.node source=this.size type=int32
        /// @resolution.member source=this.size receiver=Borrowed<Grid, view.L0, A> kind=symbol target=Grid.size
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=WithAccess<Borrowed<Grid, view.L0, "mutable">, A>
        /// @generic.instance source=this id="WithAccess<Borrowed<Grid, view.L0, \"mutable\">, A>"

    }

    peek(this: WithAccess<&Grid, A>): int32 {
    /// @generic.template symbol=peek parent=template#0 parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=peek type=<comptime peek.L0: Lifetime>(this: WithAccess<Borrowed<Grid, peek.L0, "mutable">, A>) => int32 reduced=<comptime peek.L0: Lifetime>(this: Borrowed<Grid, peek.L0, A>) => int32
    /// @type.symbol symbol=peek.this source="this: WithAccess<&Grid, A>" type=WithAccess<Borrowed<Grid, peek.L0, "mutable">, A> reduced=Borrowed<Grid, peek.L0, A>
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=Grid target=Grid
    /// @resolution.name source=A target=A

        this.view()
        /// @type.node source=this type=WithAccess<Borrowed<Grid, peek.L0, "mutable">, A> reduced=Borrowed<Grid, peek.L0, A>
        /// @type.node source=this.view type=<comptime view.L0: Lifetime>(this: WithAccess<Borrowed<Grid, view.L0, "mutable">, A>) => int32 reduced=<comptime view.L0: Lifetime>(this: Borrowed<Grid, view.L0, A>) => int32
        /// @type.node source=this.view() type=int32
        /// @resolution.member source=this.view receiver=Borrowed<Grid, peek.L0, A> kind=symbol target=view
        /// @resolution.call source=this.view() parameters=() return=int32 kind=symbol target=view receiver=Borrowed<Grid, peek.L0, A> instance=view<peek.L0>
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=WithAccess<Borrowed<Grid, peek.L0, "mutable">, A>
        /// @generic.instance source=this id="WithAccess<Borrowed<Grid, peek.L0, \"mutable\">, A>"
        /// @generic.instance source=this.view id="WithAccess<Borrowed<Grid, view.L0, \"mutable\">, A>"
        /// @generic.instance source=this.view() id=view<peek.L0>

    }
}

/// @generic.instance id="WithAccess<Borrowed<Grid, peek.L0, \"mutable\">, A>" template=memory.type.WithAccess arguments=(Borrowed<Grid, peek.L0, "mutable">, A)
/// @generic.instance id="WithAccess<Borrowed<Grid, view.L0, \"mutable\">, A>" template=memory.type.WithAccess arguments=(Borrowed<Grid, view.L0, "mutable">, A)
/// @generic.instance id=view<peek.L0> template=view arguments=(peek.L0)
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

function read<comptime L0: Lifetime>(grid: Borrowed<Grid, L0, "readonly">): int32 {
    grid.view<L0, "readonly">()
}

function write<comptime L0: Lifetime>(grid: Borrowed<Grid, L0, "exclusive">): int32 {
    grid.view<L0, "exclusive">()
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
/// @definition.method symbol=view slot=view type=<comptime view.L0: Lifetime>(this: WithAccess<Borrowed<Grid, view.L0, "mutable">, A>) => int32
/// @type.symbol symbol=A source="comptime A: Access = \"readonly\"" type=A
/// @resolution.name source=Access target=memory.access.Access
/// @type.node source="\"readonly\"" type="readonly"
/// @resolution.name source=Grid target=Grid

    view(this: WithAccess<&Grid, A>): int32 {
    /// @generic.template symbol=view parent=template#0 parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=view type=<comptime view.L0: Lifetime>(this: WithAccess<Borrowed<Grid, view.L0, "mutable">, A>) => int32 reduced=<comptime view.L0: Lifetime>(this: Borrowed<Grid, view.L0, A>) => int32
    /// @type.symbol symbol=view.this source="this: WithAccess<&Grid, A>" type=WithAccess<Borrowed<Grid, view.L0, "mutable">, A> reduced=Borrowed<Grid, view.L0, A>
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=Grid target=Grid
    /// @resolution.name source=A target=A

        this.size
        /// @type.node source=this type=WithAccess<Borrowed<Grid, view.L0, "mutable">, A> reduced=Borrowed<Grid, view.L0, A>
        /// @type.node source=this.size type=int32
        /// @resolution.member source=this.size receiver=Borrowed<Grid, view.L0, A> kind=symbol target=Grid.size
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=WithAccess<Borrowed<Grid, view.L0, "mutable">, A>
        /// @generic.instance source=this id="WithAccess<Borrowed<Grid, view.L0, \"mutable\">, A>"

    }
}

function read(grid: &readonly Grid): int32 {
/// @generic.template symbol=read parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=read type=<comptime read.L0: Lifetime>(Borrowed<Grid, read.L0, "readonly">) => int32
/// @type.symbol symbol=read.grid source="grid: &readonly Grid" type=Borrowed<Grid, read.L0, "readonly">
/// @resolution.name source=Grid target=Grid

    grid.view()
    /// @type.node source=grid type=Borrowed<Grid, read.L0, "readonly">
    /// @type.node source=grid.view type=<comptime view.L0: Lifetime>(this: WithAccess<Borrowed<Grid, view.L0, "mutable">, A>) => int32 reduced=<comptime view.L0: Lifetime>(this: Borrowed<Grid, view.L0, A>) => int32
    /// @type.node source=grid.view() type=int32
    /// @resolution.name source=grid target=read.grid
    /// @resolution.member source=grid.view receiver=Borrowed<Grid, read.L0, "readonly"> kind=symbol target=view
    /// @resolution.call source=grid.view() parameters=() return=int32 kind=symbol target=view receiver=Borrowed<Grid, read.L0, "readonly"> instance=view<read.L0>
    /// @generic.instance source=grid.view id="WithAccess<Borrowed<Grid, view.L0, \"mutable\">, A>"
    /// @generic.instance source=grid.view() id=view<read.L0>

}

function write(grid: &exclusive Grid): int32 {
/// @generic.template symbol=write parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=write type=<comptime write.L0: Lifetime>(Borrowed<Grid, write.L0, "exclusive">) => int32
/// @type.symbol symbol=write.grid source="grid: &exclusive Grid" type=Borrowed<Grid, write.L0, "exclusive">
/// @resolution.name source=Grid target=Grid

    grid.view()
    /// @type.node source=grid type=Borrowed<Grid, write.L0, "exclusive">
    /// @type.node source=grid.view type=<comptime view.L0: Lifetime>(this: WithAccess<Borrowed<Grid, view.L0, "mutable">, A>) => int32 reduced=<comptime view.L0: Lifetime>(this: Borrowed<Grid, view.L0, A>) => int32
    /// @type.node source=grid.view() type=int32
    /// @resolution.name source=grid target=write.grid
    /// @resolution.member source=grid.view receiver=Borrowed<Grid, write.L0, "exclusive"> kind=symbol target=view
    /// @resolution.call source=grid.view() parameters=() return=int32 kind=symbol target=view receiver=Borrowed<Grid, write.L0, "exclusive"> instance=view<write.L0>
    /// @generic.instance source=grid.view id="WithAccess<Borrowed<Grid, view.L0, \"mutable\">, A>"
    /// @generic.instance source=grid.view() id=view<write.L0>

}

/// @generic.instance id="WithAccess<Borrowed<Grid, view.L0, \"mutable\">, A>" template=memory.type.WithAccess arguments=(Borrowed<Grid, view.L0, "mutable">, A)
/// @generic.instance id=view<read.L0> template=view arguments=(read.L0)
/// @generic.instance id=view<write.L0> template=view arguments=(write.L0)
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
        this.view<T, N, L0, A>()
    }
}

=== checked ===
export extension FixedArrayAccess<T, comptime N: number, comptime A: Access = "readonly"> of [T; N] {
/// @generic.template symbol=FixedArrayAccess parameters=(T, comptime N: float64, comptime A: Access = "readonly")
/// @definition.extension symbol=FixedArrayAccess form=exported target=FixedArray<T, N>
/// @definition.method symbol=FixedArrayAccess.peek slot=peek type=<comptime FixedArrayAccess.peek.L0: Lifetime>(this: WithAccess<Borrowed<FixedArray<T, N>, FixedArrayAccess.peek.L0, "mutable">, A>) => int32
/// @definition.method symbol=FixedArrayAccess.view slot=view type=<comptime FixedArrayAccess.view.L0: Lifetime>(this: WithAccess<Borrowed<FixedArray<T, N>, FixedArrayAccess.view.L0, "mutable">, A>) => int32
/// @type.symbol symbol=FixedArrayAccess.T source=T type=T
/// @type.symbol symbol=FixedArrayAccess.N source="comptime N: number" type=N
/// @type.symbol symbol=FixedArrayAccess.A source="comptime A: Access = \"readonly\"" type=A
/// @resolution.name source=Access target=memory.access.Access
/// @type.node source="\"readonly\"" type="readonly"
/// @resolution.name source=T target=FixedArrayAccess.T
/// @resolution.name source=N target=FixedArrayAccess.N

    view(this: WithAccess<&[T; N], A>): int32 {
    /// @generic.template symbol=FixedArrayAccess.view parent=template#0 parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=FixedArrayAccess.view type=<comptime FixedArrayAccess.view.L0: Lifetime>(this: WithAccess<Borrowed<FixedArray<T, N>, FixedArrayAccess.view.L0, "mutable">, A>) => int32 reduced=<comptime FixedArrayAccess.view.L0: Lifetime>(this: Borrowed<FixedArray<T, N>, FixedArrayAccess.view.L0, A>) => int32
    /// @type.symbol symbol=FixedArrayAccess.view.this source="this: WithAccess<&[T; N], A>" type=WithAccess<Borrowed<FixedArray<T, N>, FixedArrayAccess.view.L0, "mutable">, A> reduced=Borrowed<FixedArray<T, N>, FixedArrayAccess.view.L0, A>
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=T target=FixedArrayAccess.T
    /// @resolution.name source=N target=FixedArrayAccess.N
    /// @resolution.name source=A target=FixedArrayAccess.A

        1
        /// @type.node source=1 type=1

    }

    peek(this: WithAccess<&[T; N], A>): int32 {
    /// @generic.template symbol=FixedArrayAccess.peek parent=template#0 parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=FixedArrayAccess.peek type=<comptime FixedArrayAccess.peek.L0: Lifetime>(this: WithAccess<Borrowed<FixedArray<T, N>, FixedArrayAccess.peek.L0, "mutable">, A>) => int32 reduced=<comptime FixedArrayAccess.peek.L0: Lifetime>(this: Borrowed<FixedArray<T, N>, FixedArrayAccess.peek.L0, A>) => int32
    /// @type.symbol symbol=FixedArrayAccess.peek.this source="this: WithAccess<&[T; N], A>" type=WithAccess<Borrowed<FixedArray<T, N>, FixedArrayAccess.peek.L0, "mutable">, A> reduced=Borrowed<FixedArray<T, N>, FixedArrayAccess.peek.L0, A>
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=T target=FixedArrayAccess.T
    /// @resolution.name source=N target=FixedArrayAccess.N
    /// @resolution.name source=A target=FixedArrayAccess.A

        this.view()
        /// @type.node source=this type=WithAccess<Borrowed<FixedArray<T, N>, FixedArrayAccess.peek.L0, "mutable">, A> reduced=Borrowed<FixedArray<T, N>, FixedArrayAccess.peek.L0, A>
        /// @type.node source=this.view type=<comptime FixedArrayAccess.view.L0: Lifetime>(this: WithAccess<Borrowed<FixedArray<T, N>, FixedArrayAccess.view.L0, "mutable">, A>) => int32 | <comptime collections.array.view.A#1: Access = "readonly", comptime collections.array.view#1.L1: Lifetime>(this: WithAccess<Borrowed<FixedArray<T, N>, collections.array.view#1.L1, "mutable">, collections.array.view.A#1>, usize, usize | undefined) => WithAccess<Borrowed<Slice<T>, collections.array.view#1.L1, "mutable">, collections.array.view.A#1> reduced=<comptime FixedArrayAccess.view.L0: Lifetime>(this: Borrowed<FixedArray<T, N>, FixedArrayAccess.view.L0, A>) => int32 | <comptime collections.array.view.A#1: Access = "readonly", comptime collections.array.view#1.L1: Lifetime>(this: Borrowed<FixedArray<T, N>, collections.array.view#1.L1, collections.array.view.A#1>, usize, usize | undefined) => Borrowed<Slice<T>, collections.array.view#1.L1, collections.array.view.A#1>
        /// @type.node source=this.view() type=int32
        /// @resolution.member source=this.view receiver=Borrowed<FixedArray<T, N>, FixedArrayAccess.peek.L0, A> kind=existential targets=[FixedArrayAccess.view, collections.array.view#1]
        /// @resolution.call source=this.view() parameters=() return=int32 kind=symbol target=FixedArrayAccess.view receiver=Borrowed<FixedArray<T, N>, FixedArrayAccess.peek.L0, A> instance=FixedArrayAccess.view<FixedArrayAccess.peek.L0>
        /// @resolution.receiver source=this kind=this declaration=FixedArrayAccess type=WithAccess<Borrowed<FixedArray<T, N>, FixedArrayAccess.peek.L0, "mutable">, A>
        /// @generic.instance source=this id="WithAccess<Borrowed<FixedArray<T, N>, FixedArrayAccess.peek.L0, \"mutable\">, A>"
        /// @generic.instance source=this.view id="WithAccess<Borrowed<FixedArray<T, N>, FixedArrayAccess.view.L0, \"mutable\">, A>"
        /// @generic.instance source=this.view id="WithAccess<Borrowed<FixedArray<T, N>, collections.array.view#1.L1, \"mutable\">, collections.array.view.A#1>"
        /// @generic.instance source=this.view id="WithAccess<Borrowed<Slice<T>, collections.array.view#1.L1, \"mutable\">, collections.array.view.A#1>"
        /// @generic.instance source=this.view() id=FixedArrayAccess.view<FixedArrayAccess.peek.L0>

    }
}

/// @generic.instance id="WithAccess<Borrowed<FixedArray<T, N>, FixedArrayAccess.peek.L0, \"mutable\">, A>" template=memory.type.WithAccess arguments=(Borrowed<FixedArray<T, N>, FixedArrayAccess.peek.L0, "mutable">, A)
/// @generic.instance id="WithAccess<Borrowed<FixedArray<T, N>, FixedArrayAccess.view.L0, \"mutable\">, A>" template=memory.type.WithAccess arguments=(Borrowed<FixedArray<T, N>, FixedArrayAccess.view.L0, "mutable">, A)
/// @generic.instance id="WithAccess<Borrowed<FixedArray<T, N>, collections.array.view#1.L1, \"mutable\">, collections.array.view.A#1>" template=memory.type.WithAccess arguments=(Borrowed<FixedArray<T, N>, collections.array.view#1.L1, "mutable">, collections.array.view.A#1)
/// @generic.instance id="WithAccess<Borrowed<Slice<T>, collections.array.view#1.L1, \"mutable\">, collections.array.view.A#1>" template=memory.type.WithAccess arguments=(Borrowed<Slice<T>, collections.array.view#1.L1, "mutable">, collections.array.view.A#1)
/// @generic.instance id=FixedArrayAccess.view<FixedArrayAccess.peek.L0> template=FixedArrayAccess.view arguments=(FixedArrayAccess.peek.L0)
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
        this.view<T, L0, A>()
    }
}

=== checked ===
export extension ArrayAccess<T, comptime A: Access = "readonly"> of Array<T> {
/// @generic.template symbol=ArrayAccess parameters=(T, comptime A: Access = "readonly")
/// @definition.extension symbol=ArrayAccess form=exported target=Array<T>
/// @definition.method symbol=ArrayAccess.peek slot=peek type=<comptime ArrayAccess.peek.L0: Lifetime>(this: WithAccess<Borrowed<Array<T>, ArrayAccess.peek.L0, "mutable">, A>) => int32
/// @definition.method symbol=ArrayAccess.view slot=view type=<comptime ArrayAccess.view.L0: Lifetime>(this: WithAccess<Borrowed<Array<T>, ArrayAccess.view.L0, "mutable">, A>) => int32
/// @type.symbol symbol=ArrayAccess.T source=T type=T
/// @type.symbol symbol=ArrayAccess.A source="comptime A: Access = \"readonly\"" type=A
/// @resolution.name source=Access target=memory.access.Access
/// @type.node source="\"readonly\"" type="readonly"
/// @resolution.name source=Array target=collections.array.Array
/// @resolution.name source=T target=ArrayAccess.T

    view(this: WithAccess<&Array<T>, A>): int32 {
    /// @generic.template symbol=ArrayAccess.view parent=template#0 parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=ArrayAccess.view type=<comptime ArrayAccess.view.L0: Lifetime>(this: WithAccess<Borrowed<Array<T>, ArrayAccess.view.L0, "mutable">, A>) => int32 reduced=<comptime ArrayAccess.view.L0: Lifetime>(this: Borrowed<Array<T>, ArrayAccess.view.L0, A>) => int32
    /// @type.symbol symbol=ArrayAccess.view.this source="this: WithAccess<&Array<T>, A>" type=WithAccess<Borrowed<Array<T>, ArrayAccess.view.L0, "mutable">, A> reduced=Borrowed<Array<T>, ArrayAccess.view.L0, A>
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=Array target=collections.array.Array
    /// @resolution.name source=T target=ArrayAccess.T
    /// @resolution.name source=A target=ArrayAccess.A

        1
        /// @type.node source=1 type=1

    }

    peek(this: WithAccess<&Array<T>, A>): int32 {
    /// @generic.template symbol=ArrayAccess.peek parent=template#0 parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=ArrayAccess.peek type=<comptime ArrayAccess.peek.L0: Lifetime>(this: WithAccess<Borrowed<Array<T>, ArrayAccess.peek.L0, "mutable">, A>) => int32 reduced=<comptime ArrayAccess.peek.L0: Lifetime>(this: Borrowed<Array<T>, ArrayAccess.peek.L0, A>) => int32
    /// @type.symbol symbol=ArrayAccess.peek.this source="this: WithAccess<&Array<T>, A>" type=WithAccess<Borrowed<Array<T>, ArrayAccess.peek.L0, "mutable">, A> reduced=Borrowed<Array<T>, ArrayAccess.peek.L0, A>
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=Array target=collections.array.Array
    /// @resolution.name source=T target=ArrayAccess.T
    /// @resolution.name source=A target=ArrayAccess.A

        this.view()
        /// @type.node source=this type=WithAccess<Borrowed<Array<T>, ArrayAccess.peek.L0, "mutable">, A> reduced=Borrowed<Array<T>, ArrayAccess.peek.L0, A>
        /// @type.node source=this.view type=<comptime ArrayAccess.view.L0: Lifetime>(this: WithAccess<Borrowed<Array<T>, ArrayAccess.view.L0, "mutable">, A>) => int32 | <comptime collections.array.view.A#2: Access = "readonly", comptime collections.array.view#2.L1: Lifetime>(this: WithAccess<Borrowed<Array<T>, collections.array.view#2.L1, "mutable">, collections.array.view.A#2>, usize, usize | undefined) => WithAccess<Borrowed<Slice<T>, collections.array.view#2.L1, "mutable">, collections.array.view.A#2> reduced=<comptime ArrayAccess.view.L0: Lifetime>(this: Borrowed<Array<T>, ArrayAccess.view.L0, A>) => int32 | <comptime collections.array.view.A#2: Access = "readonly", comptime collections.array.view#2.L1: Lifetime>(this: Borrowed<Array<T>, collections.array.view#2.L1, collections.array.view.A#2>, usize, usize | undefined) => Borrowed<Slice<T>, collections.array.view#2.L1, collections.array.view.A#2>
        /// @type.node source=this.view() type=int32
        /// @resolution.member source=this.view receiver=Borrowed<Array<T>, ArrayAccess.peek.L0, A> kind=existential targets=[ArrayAccess.view, collections.array.view#2]
        /// @resolution.call source=this.view() parameters=() return=int32 kind=symbol target=ArrayAccess.view receiver=Borrowed<Array<T>, ArrayAccess.peek.L0, A> instance=ArrayAccess.view<ArrayAccess.peek.L0>
        /// @resolution.receiver source=this kind=this declaration=ArrayAccess type=WithAccess<Borrowed<Array<T>, ArrayAccess.peek.L0, "mutable">, A>
        /// @generic.instance source=this id="WithAccess<Borrowed<Array<T>, ArrayAccess.peek.L0, \"mutable\">, A>"
        /// @generic.instance source=this id=Array<T>
        /// @generic.instance source=this.view id="WithAccess<Borrowed<Array<T>, ArrayAccess.view.L0, \"mutable\">, A>"
        /// @generic.instance source=this.view id="WithAccess<Borrowed<Array<T>, collections.array.view#2.L1, \"mutable\">, collections.array.view.A#2>"
        /// @generic.instance source=this.view id="WithAccess<Borrowed<Slice<T>, collections.array.view#2.L1, \"mutable\">, collections.array.view.A#2>"
        /// @generic.instance source=this.view id=Array<T>
        /// @generic.instance source=this.view() id=ArrayAccess.view<ArrayAccess.peek.L0>

    }
}

/// @generic.instance id="WithAccess<Borrowed<Array<T>, ArrayAccess.peek.L0, \"mutable\">, A>" template=memory.type.WithAccess arguments=(Borrowed<Array<T>, ArrayAccess.peek.L0, "mutable">, A)
/// @generic.instance id="WithAccess<Borrowed<Array<T>, ArrayAccess.view.L0, \"mutable\">, A>" template=memory.type.WithAccess arguments=(Borrowed<Array<T>, ArrayAccess.view.L0, "mutable">, A)
/// @generic.instance id="WithAccess<Borrowed<Array<T>, collections.array.view#2.L1, \"mutable\">, collections.array.view.A#2>" template=memory.type.WithAccess arguments=(Borrowed<Array<T>, collections.array.view#2.L1, "mutable">, collections.array.view.A#2)
/// @generic.instance id="WithAccess<Borrowed<Slice<T>, collections.array.view#2.L1, \"mutable\">, collections.array.view.A#2>" template=memory.type.WithAccess arguments=(Borrowed<Slice<T>, collections.array.view#2.L1, "mutable">, collections.array.view.A#2)
/// @generic.instance id=Array<T> template=collections.array.Array arguments=(T)
/// @generic.instance id=ArrayAccess.view<ArrayAccess.peek.L0> template=ArrayAccess.view arguments=(ArrayAccess.peek.L0)
"#,
    );
}
