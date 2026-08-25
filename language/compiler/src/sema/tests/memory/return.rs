use crate::tests::{DirRows, TestSession};

#[test]
fn test_tie_an_elided_return_borrow_to_the_single_input_region() {
    let session = TestSession::single(
        r#"
function first(values: &readonly [int32]): &readonly int32 {
    return &readonly values[0];
}

function inspect(values: &readonly [int32]): void {
    first(values) satisfies &readonly int32;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
function first<'a>(values: &'a readonly [int32]): &'a readonly int32 {
    return &readonly values[0];
}

function inspect<'a>(values: &'a readonly [int32]): void {
    first<P1>(values) satisfies &readonly int32;
}

=== dir ===
function first(values: &readonly [int32]): &readonly int32 {
/// @generic.template symbol=first parameters=('a, P1: Place)
/// @type.symbol symbol=first type=<first.'a, first.P1: Place>(&first.'a readonly Slice<int32>) => &first.'a readonly int32
/// @type.symbol symbol=first.values source="values: &readonly [int32]" type=&first.'a readonly Slice<int32>

    return &readonly values[0];
    /// @resolution.name source=values target=first.values
    /// @resolution.place source=values placement=first.P1 lifetime=first.'a access="readonly"
    /// @resolution.access source=values root=first.values
    /// @resolution.place source=values[0] placement=first.P1 lifetime=first.'a access="readonly"
    /// @resolution.access source=values[0] root=first.values keys=[0]
    /// @resolution.subscript source=values[0] type=int32 kind=call target="index#1(parameters=(isize), arguments=(provided(0) as isize), return=WithAccess<&first.'a int32, \"readonly\">)"
    /// @generic.instantiation id="index#1<int32, \"readonly\", first.P1>" template=index#1 arguments=(int32, "readonly", first.P1)

}

function inspect(values: &readonly [int32]): void {
/// @generic.template symbol=inspect parameters=('a, P1: Place)
/// @type.symbol symbol=inspect type=<inspect.'a, inspect.P1: Place>(&inspect.'a readonly Slice<int32>) => void
/// @type.symbol symbol=inspect.values source="values: &readonly [int32]" type=&inspect.'a readonly Slice<int32>

    first(values) satisfies &readonly int32;
    /// @resolution.name source=first target=first
    /// @resolution.call source=first(values) parameters=(&inspect.'a readonly Slice<int32>) arguments=(provided(values) as &inspect.'a readonly Slice<int32>) return=&inspect.'a readonly int32 kind=symbol target=first instance=first<inspect.P1>
    /// @generic.instantiation id=first<inspect.P1> template=first arguments=(inspect.P1)
    /// @resolution.name source=values target=inspect.values
    /// @resolution.place source=values placement=inspect.P1 lifetime=inspect.'a access="readonly"
    /// @resolution.access source=values root=inspect.values

}
"#, r#"
"#);
}

#[test]
fn test_join_the_input_regions_of_an_elided_return_borrow() {
    let session = TestSession::single(
        r#"
function pick(left: &readonly int32, right: &readonly int32, takeLeft: boolean): &readonly int32 {
    return takeLeft ? left : right;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
function pick<'a, 'b>(
    left: &'a readonly int32,
    right: &'b readonly int32,
    takeLeft: boolean,
): Borrowed<int32, 'a & P1 | 'b & P3, "readonly"> {
    return takeLeft ? left : right;
}

=== dir ===
function pick(left: &readonly int32, right: &readonly int32, takeLeft: boolean): &readonly int32 {
/// @generic.template symbol=pick parameters=('a, P1: Place, 'b, P3: Place)
/// @type.symbol symbol=pick type=<pick.'a, pick.P1: Place, pick.'b, pick.P3: Place>(&pick.'a readonly int32, &pick.'b readonly int32, boolean) => Borrowed<int32, pick.'a & pick.P1 | pick.'b & pick.P3, "readonly">
/// @type.symbol symbol=pick.left source="left: &readonly int32" type=&pick.'a readonly int32
/// @type.symbol symbol=pick.right source="right: &readonly int32" type=&pick.'b readonly int32
/// @type.symbol symbol=pick.takeLeft source="takeLeft: boolean" type=boolean

    return takeLeft ? left : right;
    /// @resolution.name source=takeLeft target=pick.takeLeft
    /// @resolution.place source=takeLeft placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=takeLeft root=pick.takeLeft
    /// @resolution.name source=left target=pick.left
    /// @resolution.place source=left placement=pick.P1 lifetime=pick.'a access="readonly"
    /// @resolution.access source=left root=pick.left
    /// @resolution.name source=right target=pick.right
    /// @resolution.place source=right placement=pick.P3 lifetime=pick.'b access="readonly"
    /// @resolution.access source=right root=pick.right

}
"#, r#"
"#);
}

#[test]
fn test_read_constant_storage_for_an_inputless_elided_return() {
    let session = TestSession::single(
        r#"
const shared: ^int32 = 1;

function fallback(): &readonly int32 {
    return &readonly shared;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
const shared: int32 = 1;

function fallback(): &'static readonly int32 {
    return &readonly shared;
}

=== dir ===
const shared: ^int32 = 1;
/// @type.symbol symbol=shared source=shared type=int32
/// @resolution.pattern source=shared kind=binding target=shared

function fallback(): &readonly int32 {
/// @type.symbol symbol=fallback type=() => &'static readonly constant int32

    return &readonly shared;
    /// @resolution.name source=shared target=shared
    /// @resolution.place source=shared placement="constant" lifetime="static" access="readonly"
    /// @resolution.access source=shared root=shared

}
"#, r#"
"#);
}

#[test]
fn test_reject_an_elided_return_borrow_from_a_pinned_receiver() {
    let session = TestSession::single(
        r#"
class Store {
    value: int32 = 0;

    view(this): &readonly int32 {
        return &readonly this.value;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
class Store {
    value: int32 = 0;

    view(this): &'static readonly int32 {
        return &readonly this.value;
    }
}

=== dir ===
class Store {
/// @type.symbol symbol=Store type=Store
/// @definition.class symbol=Store
/// @definition.field symbol=Store.value source="value: int32 = 0" key=value type=int32
/// @definition.method symbol=Store.view slot=view type=(this: this) => &'static readonly constant int32

    value: int32 = 0;
    /// @type.symbol symbol=Store.value source="value: int32 = 0" type=int32

    view(this): &readonly int32 {
    /// @type.symbol symbol=Store.view type=(this: this) => &'static readonly constant int32
    /// @type.symbol symbol=Store.view.this source=this type=this

        return &readonly this.value;
        /// @resolution.member source=this.value receiver=Store type=int32 kind=field target_receiver=Store key=value target=Store.value target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Store type=Store
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.value root=this keys=[value]

    }
}
"#, r#"
/// @diagnostic.error id=return-not-assignable message="type '&readonly local int32' is not assignable to the declared result type '&'static readonly constant int32'"
/// @diagnostic.label line=6 column=16 span="&" line_source="return &readonly this.value;"
"#);
}

#[test]
fn test_reject_an_escaping_elided_return_borrow() {
    let session = TestSession::single(
        r#"
function first(values: &readonly [int32]): &readonly int32 {
    return &readonly values[0];
}

function escape(): &readonly int32 {
    const values: ^[int32] = [1, 2];

    return first(&readonly values);
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
function first<'a>(values: &'a readonly [int32]): &'a readonly int32 {
    return &readonly values[0];
}

function escape(): &'static readonly int32 {
    const values: ^[int32] = [1, 2];

    return first(&readonly values);
}

=== dir ===
function first(values: &readonly [int32]): &readonly int32 {
/// @generic.template symbol=first parameters=('a, P1: Place)
/// @type.symbol symbol=first type=<first.'a, first.P1: Place>(&first.'a readonly Slice<int32>) => &first.'a readonly int32
/// @type.symbol symbol=first.values source="values: &readonly [int32]" type=&first.'a readonly Slice<int32>

    return &readonly values[0];
    /// @resolution.name source=values target=first.values
    /// @resolution.place source=values placement=first.P1 lifetime=first.'a access="readonly"
    /// @resolution.access source=values root=first.values
    /// @resolution.place source=values[0] placement=first.P1 lifetime=first.'a access="readonly"
    /// @resolution.access source=values[0] root=first.values keys=[0]
    /// @resolution.subscript source=values[0] type=int32 kind=call target="index#1(parameters=(isize), arguments=(provided(0) as isize), return=WithAccess<&first.'a int32, \"readonly\">)"
    /// @generic.instantiation id="index#1<int32, \"readonly\", first.P1>" template=index#1 arguments=(int32, "readonly", first.P1)

}

function escape(): &readonly int32 {
/// @type.symbol symbol=escape type=() => &'static readonly constant int32

    const values: ^[int32] = [1, 2];
    /// @type.symbol symbol=escape.values source=values type=Owned<Slice<int32>>
    /// @resolution.pattern source=values kind=binding target=escape.values
    /// @resolution.call source=[1, 2] parameters=(&arrayFromSlice.'a readonly Slice<arrayFromSlice.T>) arguments=(rest(1, 2) as int32) return=int32[] kind=symbol target=arrayFromSlice instance=arrayFromSlice<int32>
    /// @generic.instantiation id=arrayFromSlice<int32> template=arrayFromSlice arguments=(int32)

    return first(&readonly values);
    /// @resolution.name source=first target=first
    /// @resolution.call source="first(&readonly values)" parameters=(Borrowed<Slice<int32>, "frame" & <error>, "readonly">) arguments=(provided(&readonly values) as Borrowed<Slice<int32>, "frame" & <error>, "readonly">) return=Borrowed<int32, "frame" & <error>, "readonly"> kind=symbol target=first instance=first<<error>>
    /// @generic.instantiation id=first<<error>> template=first arguments=(<error>)
    /// @resolution.name source=values target=escape.values
    /// @resolution.place source=values placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=values root=escape.values

}
"#, r#"
/// @diagnostic.error id=not-assignable message="type '\"frame\"' is not assignable to type '\"static\"'"
/// @diagnostic.label line=9 column=12 span="first(&readonly values)" line_source="return first(&readonly values);"
"#);
}

#[test]
fn test_tie_an_elided_method_return_to_its_borrowed_receiver() {
    let session = TestSession::single(
        r#"
class Store {
    value: int32 = 0;

    view(&readonly this): &readonly int32 {
        return &readonly this.value;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
class Store {
    value: int32 = 0;

    view(&readonly this): &'a readonly int32 {
        return &readonly this.value;
    }
}

=== dir ===
class Store {
/// @type.symbol symbol=Store type=Store
/// @definition.class symbol=Store
/// @definition.field symbol=Store.value source="value: int32 = 0" key=value type=int32
/// @definition.method symbol=Store.view slot=view type=<Store.view.'a, Store.view.P1: Place>(this: &Store.view.'a readonly this) => &Store.view.'a readonly int32

    value: int32 = 0;
    /// @type.symbol symbol=Store.value source="value: int32 = 0" type=int32

    view(&readonly this): &readonly int32 {
    /// @generic.template symbol=Store.view parameters=('a, P1: Place)
    /// @type.symbol symbol=Store.view type=<Store.view.'a, Store.view.P1: Place>(this: &Store.view.'a readonly this) => &Store.view.'a readonly int32
    /// @type.symbol symbol=Store.view.this source="&readonly this" type=&Store.view.'a readonly this

        return &readonly this.value;
        /// @resolution.member source=this.value receiver=&Store.view.'a readonly Store type=int32 kind=field target_receiver=&Store.view.'a readonly Store key=value target=Store.value target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Store type=&Store.view.'a readonly Store
        /// @resolution.place source=this placement=Store.view.P1 lifetime=Store.view.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement=Store.view.P1 lifetime=Store.view.'a access="readonly"
        /// @resolution.access source=this.value root=this keys=[value]

    }
}
"#, r#"
"#);
}
