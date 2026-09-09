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
    first(values) satisfies &readonly int32;
}

=== dir ===
function first(values: &readonly [int32]): &readonly int32 {
/// @generic.template symbol=first parameters=('a)
/// @type.symbol symbol=first type=<first.'a>(&first.'a readonly Slice<int32>) => &first.'a readonly int32
/// @type.symbol symbol=first.values source="values: &readonly [int32]" type=&first.'a readonly Slice<int32>

    return &readonly values[0];
    /// @resolution.name source=values target=first.values
    /// @resolution.place source=values placement=first.'a lifetime=first.'a access="readonly"
    /// @resolution.access source=values root=first.values
    /// @resolution.place source=values[0] placement=first.'a lifetime=first.'a access="readonly"
    /// @resolution.access source=values[0] root=first.values keys=[0]
    /// @resolution.subscript source=values[0] type=int32 kind=call target="index#1(parameters=(isize), arguments=(provided(0) as isize), return=WithAccess<&first.'a int32, \"readonly\">, regions=(first.'a))"
    /// @generic.instantiation id="index#1<int32, \"readonly\">" template=index#1 arguments=(int32, "readonly")

}

function inspect(values: &readonly [int32]): void {
/// @generic.template symbol=inspect parameters=('a)
/// @type.symbol symbol=inspect type=<inspect.'a>(&inspect.'a readonly Slice<int32>) => void
/// @type.symbol symbol=inspect.values source="values: &readonly [int32]" type=&inspect.'a readonly Slice<int32>

    first(values) satisfies &readonly int32;
    /// @resolution.name source=first target=first
    /// @resolution.call source=first(values) parameters=(&inspect.'a readonly Slice<int32>) arguments=(provided(values) as &inspect.'a readonly Slice<int32>) return=&inspect.'a readonly int32 regions=(inspect.'a) kind=symbol target=first
    /// @resolution.name source=values target=inspect.values
    /// @resolution.place source=values placement=inspect.'a lifetime=inspect.'a access="readonly"
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
): Borrowed<int32, 'a | 'b, "readonly"> {
    return takeLeft ? left : right;
}

=== dir ===
function pick(left: &readonly int32, right: &readonly int32, takeLeft: boolean): &readonly int32 {
/// @generic.template symbol=pick parameters=('a, 'b)
/// @type.symbol symbol=pick type=<pick.'a, pick.'b>(&pick.'a readonly int32, &pick.'b readonly int32, boolean) => &pick.'a | pick.'b readonly int32
/// @type.symbol symbol=pick.left source="left: &readonly int32" type=&pick.'a readonly int32
/// @type.symbol symbol=pick.right source="right: &readonly int32" type=&pick.'b readonly int32
/// @type.symbol symbol=pick.takeLeft source="takeLeft: boolean" type=boolean

    return takeLeft ? left : right;
    /// @resolution.name source=takeLeft target=pick.takeLeft
    /// @resolution.place source=takeLeft placement="local" lifetime="frame" access="mutable"
    /// @resolution.access source=takeLeft root=pick.takeLeft
    /// @resolution.name source=left target=pick.left
    /// @resolution.place source=left placement=pick.'a lifetime=pick.'a access="readonly"
    /// @resolution.access source=left root=pick.left
    /// @resolution.name source=right target=pick.right
    /// @resolution.place source=right placement=pick.'b lifetime=pick.'b access="readonly"
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const shared: int32 = 1;

function fallback(): &'managed readonly int32 {
    return &readonly shared;
}

=== dir ===
const shared: ^int32 = 1;
/// @type.symbol symbol=shared source=shared type=int32
/// @resolution.pattern source=shared kind=binding target=shared

function fallback(): &readonly int32 {
/// @type.symbol symbol=fallback type=() => Borrowed<int32, "managed" & "local", "readonly">

    return &readonly shared;
    /// @resolution.name source=shared target=shared
    /// @resolution.place source=shared placement="constant" lifetime="static" access="readonly"
    /// @resolution.access source=shared root=shared

}
"#,
        r#"

"#,
    );
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

    view(this): &'managed readonly int32 {
        return &readonly this.value;
    }
}

=== dir ===
class Store {
/// @type.symbol symbol=Store type=Store
/// @definition.class symbol=Store
/// @definition.field symbol=Store.value source="value: int32 = 0" key=value type=int32
/// @definition.method symbol=Store.view slot=view type=(this: this) => Borrowed<int32, "managed" & "local", "readonly">

    value: int32 = 0;
    /// @type.symbol symbol=Store.value source="value: int32 = 0" type=int32

    view(this): &readonly int32 {
    /// @type.symbol symbol=Store.view type=(this: this) => Borrowed<int32, "managed" & "local", "readonly">
    /// @type.symbol symbol=Store.view.this source=this type=this

        return &readonly this.value;
        /// @resolution.member source=this.value receiver=Store type=int32 kind=field target_receiver=Store key=value target=Store.value target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Store type=Store
        /// @resolution.place source=this placement="local" lifetime="frame" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime="frame" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]

    }
}
"#, r#"

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

function escape(): &'managed readonly int32 {
    const values: ^[int32] = [1, 2];

    return first(&readonly values);
}

=== dir ===
function first(values: &readonly [int32]): &readonly int32 {
/// @generic.template symbol=first parameters=('a)
/// @type.symbol symbol=first type=<first.'a>(&first.'a readonly Slice<int32>) => &first.'a readonly int32
/// @type.symbol symbol=first.values source="values: &readonly [int32]" type=&first.'a readonly Slice<int32>

    return &readonly values[0];
    /// @resolution.name source=values target=first.values
    /// @resolution.place source=values placement=first.'a lifetime=first.'a access="readonly"
    /// @resolution.access source=values root=first.values
    /// @resolution.place source=values[0] placement=first.'a lifetime=first.'a access="readonly"
    /// @resolution.access source=values[0] root=first.values keys=[0]
    /// @resolution.subscript source=values[0] type=int32 kind=call target="index#1(parameters=(isize), arguments=(provided(0) as isize), return=WithAccess<&first.'a int32, \"readonly\">, regions=(first.'a))"
    /// @generic.instantiation id="index#1<int32, \"readonly\">" template=index#1 arguments=(int32, "readonly")

}

function escape(): &readonly int32 {
/// @type.symbol symbol=escape type=() => Borrowed<int32, "managed" & "local", "readonly">

    const values: ^[int32] = [1, 2];
    /// @type.symbol symbol=escape.values source=values type=^Slice<int32>
    /// @resolution.pattern source=values kind=binding target=escape.values
    /// @resolution.call source=[1, 2] parameters=(^Slice<arrayFromOwnedSlice.T>) arguments=(rest(1, 2) as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
    /// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)

    return first(&readonly values);
    /// @resolution.name source=first target=first
    /// @resolution.call source="first(&readonly values)" parameters=(&'frame readonly Slice<int32>) arguments=(provided(&readonly values) as &'frame readonly Slice<int32>) return=&'frame readonly int32 regions=("frame" & "local") kind=symbol target=first
    /// @resolution.name source=values target=escape.values
    /// @resolution.place source=values placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=values root=escape.values

}
"#, r#"
/// @diagnostic.error id=not-assignable message="type '^int32[]' is not assignable to type '^Slice<int32>'"
/// @diagnostic.label line=7 column=30 span="[1, 2]" line_source="const values: ^[int32] = [1, 2];"
/// @diagnostic.related line=7 column=19 span="^" line_source="const values: ^[int32] = [1, 2];" message="expected due to this annotation"
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
/// @definition.method symbol=Store.view slot=view type=<Store.view.'a>(this: &Store.view.'a readonly this) => &Store.view.'a readonly int32

    value: int32 = 0;
    /// @type.symbol symbol=Store.value source="value: int32 = 0" type=int32

    view(&readonly this): &readonly int32 {
    /// @generic.template symbol=Store.view parameters=('a)
    /// @type.symbol symbol=Store.view type=<Store.view.'a>(this: &Store.view.'a readonly this) => &Store.view.'a readonly int32
    /// @type.symbol symbol=Store.view.this source="&readonly this" type=&Store.view.'a readonly this

        return &readonly this.value;
        /// @resolution.member source=this.value receiver=&Store.view.'a readonly Store type=int32 kind=field target_receiver=&Store.view.'a readonly Store key=value target=Store.value target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Store type=&Store.view.'a readonly Store
        /// @resolution.place source=this placement=Store.view.'a lifetime=Store.view.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement=Store.view.'a lifetime=Store.view.'a access="readonly"
        /// @resolution.access source=this.value root=this keys=[value]

    }
}
"#, r#"
"#);
}

#[test]
fn test_tie_an_elided_method_return_to_its_implicit_receiver() {
    let session = TestSession::single(
        r#"
struct Own {
    message: string;

    inherent(): &readonly string {
        return this.message;
    }
}

export extension of Own {
    extended(): &readonly string {
        return this.message;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
struct Own {
    message: string;

    inherent(): &'a readonly string {
        return this.message as &'a readonly string;
    }
}

export extension of Own {
    extended(): &'a readonly string {
        return this.message as &'a readonly string;
    }
}

=== dir ===
struct Own {
/// @type.symbol symbol=Own type=Own
/// @definition.struct symbol=Own
/// @definition.field symbol=Own.message source="message: string" key=message type=string
/// @definition.method symbol=Own.inherent slot=inherent type=<Own.inherent.'a>(this: &Own.inherent.'a readonly this) => &Own.inherent.'a readonly string

    message: string;
    /// @type.symbol symbol=Own.message source="message: string" type=string

    inherent(): &readonly string {
    /// @generic.template symbol=Own.inherent parameters=('a)
    /// @type.symbol symbol=Own.inherent type=<Own.inherent.'a>(this: &Own.inherent.'a readonly this) => &Own.inherent.'a readonly string
    /// @type.symbol symbol=Own.inherent.this type=&Own.inherent.'a readonly Own

        return this.message;
        /// @resolution.member source=this.message receiver=&Own.inherent.'a readonly Own type=Readonly<string> kind=field target_receiver=&Own.inherent.'a readonly Own key=message target=Own.message target_type=Readonly<string>
        /// @resolution.receiver source=this kind=this declaration=Own type=&Own.inherent.'a readonly Own
        /// @resolution.place source=this placement=Own.inherent.'a lifetime=Own.inherent.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.message placement=Own.inherent.'a lifetime="managed" access="readonly"
        /// @resolution.access source=this.message root=this keys=[message]

    }
}

export extension of Own {
/// @definition.extension symbol=<module>#2 form=exported target=Own
/// @definition.method symbol=extended slot=extended type=<extended.'a>(this: &extended.'a readonly this) => &extended.'a readonly string
/// @resolution.name source=Own target=Own

    extended(): &readonly string {
    /// @generic.template symbol=extended parameters=('a)
    /// @type.symbol symbol=extended type=<extended.'a>(this: &extended.'a readonly this) => &extended.'a readonly string
    /// @type.symbol symbol=extended.this type=&extended.'a readonly Own

        return this.message;
        /// @resolution.member source=this.message receiver=&extended.'a readonly Own type=Readonly<string> kind=field target_receiver=&extended.'a readonly Own key=message target=Own.message target_type=Readonly<string>
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&extended.'a readonly Own
        /// @resolution.place source=this placement=extended.'a lifetime=extended.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.message placement=extended.'a lifetime="managed" access="readonly"
        /// @resolution.access source=this.message root=this keys=[message]

    }
}
"#, "");
}
