use crate::tests::{DirRows, TestSession};

#[test]
fn test_bind_an_elided_return_borrow_to_the_single_input_region() {
    let session = TestSession::single(
        r#"
function first(values: &immutable [int32]): &immutable int32 {
    return &immutable values[0];
}

function inspect(values: &immutable [int32]): void {
    first(values) satisfies &immutable int32;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
function first<'a>(values: &'a immutable [int32]): &'a immutable int32 {
    return &immutable values[0];
}

function inspect<'a>(values: &'a immutable [int32]): void {
    first<'a>(values) satisfies &immutable int32;
}

=== dir ===
function first(values: &immutable [int32]): &immutable int32 {
/// @generic.template symbol=first parameters=('a)
/// @type.symbol symbol=first type=<first.'a>(&first.'a immutable Slice<int32>) => &first.'a immutable int32
/// @type.symbol symbol=first.values source="values: &immutable [int32]" type=&first.'a immutable Slice<int32>

    return &immutable values[0];
    /// @resolution.name source=values target=first.values
    /// @resolution.place source=values placement=first.'a lifetime=first.'a access="immutable"
    /// @resolution.access source=values root=first.values
    /// @resolution.subscript source=values[0] type=int32 kind=call target="index#2(parameters=(isize), arguments=(provided(0) as isize), return=int32, regions=(first.'a))"
    /// @generic.instantiation id="index#2<int32, first.'a>" template=index#2 arguments=(int32, first.'a)

}

function inspect(values: &immutable [int32]): void {
/// @generic.template symbol=inspect parameters=('a)
/// @type.symbol symbol=inspect type=<inspect.'a>(&inspect.'a immutable Slice<int32>) => void
/// @type.symbol symbol=inspect.values source="values: &immutable [int32]" type=&inspect.'a immutable Slice<int32>

    first(values) satisfies &immutable int32;
    /// @resolution.name source=first target=first
    /// @resolution.call source=first(values) parameters=(&inspect.'a immutable Slice<int32>) arguments=(provided(values) as &inspect.'a immutable Slice<int32>) return=&inspect.'a immutable int32 regions=(inspect.'a) kind=symbol target=first instance=first<inspect.'a>
    /// @generic.instantiation id=first<inspect.'a> template=first arguments=(inspect.'a)
    /// @resolution.name source=values target=inspect.values
    /// @resolution.place source=values placement=inspect.'a lifetime=inspect.'a access="immutable"
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

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
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
/// @type.symbol symbol=pick type=<pick.'a, pick.'b>(&pick.'a readonly int32, &pick.'b readonly int32, boolean) => &readonly int32
/// @type.symbol symbol=pick.left source="left: &readonly int32" type=&pick.'a readonly int32
/// @type.symbol symbol=pick.right source="right: &readonly int32" type=&pick.'b readonly int32
/// @type.symbol symbol=pick.takeLeft source="takeLeft: boolean" type=boolean

    return takeLeft ? left : right;
    /// @resolution.name source=takeLeft target=pick.takeLeft
    /// @resolution.place source=takeLeft placement="local" lifetime="frame" access="exclusive"
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
        "main.tspp",
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
/// @type.symbol symbol=fallback type=() => &'managed readonly int32

    return &readonly shared;
    /// @resolution.name source=shared target=shared
    /// @resolution.place source=shared placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=shared root=shared

}
"#,
        r#"

"#,
    );
}

/// Elide a return borrow from a managed receiver.
#[test]
fn test_elide_a_return_borrow_from_a_managed_receiver() {
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

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
class Store {
    value: int32 = 0;

    view(this): &'managed readonly int32 {
        return &readonly this.value;
    }
}

=== dir ===
class Store {
/// @type.symbol symbol=Store type=typeof Store
/// @definition.class symbol=Store
/// @definition.field symbol=Store.value source="value: int32 = 0" key=value type=int32
/// @definition.method symbol=Store.view slot=view type=(this: Store) => &'managed readonly int32

    value: int32 = 0;
    /// @type.symbol symbol=Store.value source="value: int32 = 0" type=int32

    view(this): &readonly int32 {
    /// @type.symbol symbol=Store.view type=(this: Store) => &'managed readonly int32
    /// @type.symbol symbol=Store.view.this source=this type=Store

        return &readonly this.value;
        /// @resolution.member source=this.value receiver=Store type=int32 kind=field target_receiver=Store key=value target=Store.value target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Store type=Store
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]

    }
}
"#, r#"

"#);
}

/// Type an elided return borrow of a frame value, which verify rejects.
#[test]
fn test_type_an_elided_return_borrow_of_a_frame_value() {
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

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
function first<'a>(values: &'a readonly [int32]): &'a readonly int32 {
    return &readonly values[0];
}

function escape(): &'managed readonly int32 {
    const values: ^[int32] = [1, 2];

    return first<"frame">(&readonly values);
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
    /// @resolution.subscript source=values[0] type=int32 kind=call target="index#2(parameters=(isize), arguments=(provided(0) as isize), return=int32, regions=(first.'a))"
    /// @generic.instantiation id="index#2<int32, first.'a>" template=index#2 arguments=(int32, first.'a)

}

function escape(): &readonly int32 {
/// @type.symbol symbol=escape type=() => &'managed readonly int32

    const values: ^[int32] = [1, 2];
    /// @type.symbol symbol=escape.values source=values type=^Slice<int32>
    /// @resolution.pattern source=values kind=binding target=escape.values
    /// @resolution.call source=[1, 2] parameters=(^Slice<int32>) arguments=(rest(provided(1) as int32, provided(2) as int32) as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
    /// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)

    return first(&readonly values);
    /// @resolution.name source=first target=first
    /// @resolution.call source="first(&readonly values)" parameters=(&'frame readonly Slice<int32>) arguments=(provided(&readonly values) as &'frame readonly Slice<int32>) return=&'frame readonly int32 regions=("frame" & "local") kind=symbol target=first instance="first<\"frame\" & \"local\">"
    /// @generic.instantiation id="first<\"frame\" & \"local\">" template=first arguments=("frame" & "local")
    /// @resolution.name source=values target=escape.values
    /// @resolution.place source=values placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=values root=escape.values

}
"#, r#"

"#);
}

#[test]
fn test_bind_an_elided_method_return_to_its_borrowed_receiver() {
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

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
class Store {
    value: int32 = 0;

    view(&readonly this): &'a readonly int32 {
        return &readonly this.value;
    }
}

=== dir ===
class Store {
/// @type.symbol symbol=Store type=typeof Store
/// @definition.class symbol=Store
/// @definition.field symbol=Store.value source="value: int32 = 0" key=value type=int32
/// @definition.method symbol=Store.view slot=view type=<Store.view.'a>(this: &Store.view.'a readonly Store) => &Store.view.'a readonly int32

    value: int32 = 0;
    /// @type.symbol symbol=Store.value source="value: int32 = 0" type=int32

    view(&readonly this): &readonly int32 {
    /// @generic.template symbol=Store.view parameters=('a)
    /// @type.symbol symbol=Store.view type=<Store.view.'a>(this: &Store.view.'a readonly Store) => &Store.view.'a readonly int32
    /// @type.symbol symbol=Store.view.this source="&readonly this" type=&Store.view.'a readonly Store

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
fn test_bind_an_elided_method_return_to_its_implicit_receiver() {
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

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
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
/// @definition.method symbol=Own.inherent slot=inherent type=<Own.inherent.'a>(this: &Own.inherent.'a readonly Own) => &Own.inherent.'a readonly string

    message: string;
    /// @type.symbol symbol=Own.message source="message: string" type=string

    inherent(): &readonly string {
    /// @generic.template symbol=Own.inherent parameters=('a)
    /// @type.symbol symbol=Own.inherent type=<Own.inherent.'a>(this: &Own.inherent.'a readonly Own) => &Own.inherent.'a readonly string
    /// @type.symbol symbol=Own.inherent.this type=&Own.inherent.'a readonly Own

        return this.message;
        /// @resolution.member source=this.message receiver=&Own.inherent.'a readonly Own type=string kind=field target_receiver=&Own.inherent.'a readonly Own key=message target=Own.message target_type=string
        /// @resolution.receiver source=this kind=this declaration=Own type=&Own.inherent.'a readonly Own
        /// @resolution.place source=this placement=Own.inherent.'a lifetime=Own.inherent.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.message placement=Own.inherent.'a lifetime=Own.inherent.'a access="readonly"
        /// @resolution.access source=this.message root=this keys=[message]

    }
}

export extension of Own {
/// @definition.extension symbol=<module>#2 form=exported target=Own
/// @definition.method symbol=extended slot=extended type=<extended.'a>(this: &extended.'a readonly Own) => &extended.'a readonly string
/// @resolution.name source=Own target=Own

    extended(): &readonly string {
    /// @generic.template symbol=extended parameters=('a)
    /// @type.symbol symbol=extended type=<extended.'a>(this: &extended.'a readonly Own) => &extended.'a readonly string
    /// @type.symbol symbol=extended.this type=&extended.'a readonly Own

        return this.message;
        /// @resolution.member source=this.message receiver=&extended.'a readonly Own type=string kind=field target_receiver=&extended.'a readonly Own key=message target=Own.message target_type=string
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&extended.'a readonly Own
        /// @resolution.place source=this placement=extended.'a lifetime=extended.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.message placement=extended.'a lifetime=extended.'a access="readonly"
        /// @resolution.access source=this.message root=this keys=[message]

    }
}
"#, r#"
"#);
}

/// An elided return borrow inside a newtype application takes the receiver's region.
#[test]
fn test_bind_an_elided_return_borrow_inside_a_newtype_application_to_the_receiver() {
    let session = TestSession::single(
        r#"
struct Pair<T> {
    start: T;
    end: T;
}

newtype Bound<T> =
    | { kind: "included"; value: T }
    | { kind: "unbounded" };

extension<T> of Bound<T> {
    static included(value: T): Bound<T> {
        Bound({ kind: "included", value })
    }
}

extension<T: Copy> of Pair<T> {
    startBound(&immutable this): Bound<&immutable T> {
        Bound.included(&immutable this.start)
    }
}
"#,
    );

    session.assert_dir("main.tspp", DirRows::checked(), r#"
=== annotated ===
struct Pair<out T> {
    start: T;
    end: T;
}

newtype Bound<in out T> = { kind: "included"; value: T } | { kind: "unbounded" };

extension<T> of Bound<T> {
    static included(value: T): Bound<T> {
        Bound({ kind: "included", value })
    }
}

extension<T: Copy> of Pair<T> {
    startBound(&immutable this): Bound<&'a immutable T> {
        Bound.included<&'a immutable T>(&immutable this.start)
    }
}

=== dir ===
struct Pair<T> {
/// @generic.template symbol=Pair parameters=(out T#1)
/// @type.symbol symbol=Pair type=Pair
/// @definition.struct symbol=Pair template=(out T#1)
/// @definition.field symbol=Pair.end source="end: T" key=end type=T#1
/// @definition.field symbol=Pair.start source="start: T" key=start type=T#1
/// @type.symbol symbol=Pair.T source=T type=T#1

    start: T;
    /// @type.symbol symbol=Pair.start source="start: T" type=T#1
    /// @resolution.name source=T target=Pair.T

    end: T;
    /// @type.symbol symbol=Pair.end source="end: T" type=T#1
    /// @resolution.name source=T target=Pair.T

}

newtype Bound<T> =
/// @generic.template symbol=Bound parameters=(in out T#2)
/// @type.symbol symbol=Bound type=Bound
/// @definition.newtype symbol=Bound template=(in out T#2) backing={ kind: "included"; value: T#2 } | { kind: "unbounded" } constructors=[<T#2>({ kind: "included"; value: T#2 }) => Bound<T#2>, <T#2>({ kind: "unbounded" }) => Bound<T#2>, <T#2>({ kind: "included"; value: T#2 } | { kind: "unbounded" }) => Bound<T#2>]
/// @type.symbol symbol=Bound.T source=T type=T#2

    | { kind: "included"; value: T }
    /// @type.symbol symbol=Bound.kind#1 source="kind: \"included\"" type="included"
    /// @type.symbol symbol=Bound.value source="value: T" type=T#2
    /// @resolution.name source=T target=Bound.T

    | { kind: "unbounded" };
    /// @type.symbol symbol=Bound.kind#2 source="kind: \"unbounded\"" type="unbounded"

extension<T> of Bound<T> {
/// @generic.template symbol=<module>#2 parameters=(T#3)
/// @generic.instance id=Bound<T#3> template=Bound arguments=(T#3)
/// @definition.extension symbol=<module>#2 form=local target=Bound<T#3>
/// @definition.method symbol=included slot=included static=true type=(T#3) => Bound<T#3>
/// @type.symbol symbol=T#1 source=T type=T#3
/// @resolution.name source=Bound target=Bound
/// @resolution.name source=T target=T#1

    static included(value: T): Bound<T> {
    /// @type.symbol symbol=included type=(T#3) => Bound<T#3>
    /// @type.symbol symbol=included.value source="value: T" type=T#3
    /// @resolution.name source=T target=T#1
    /// @resolution.name source=Bound target=Bound
    /// @resolution.name source=T target=T#1

        Bound({ kind: "included", value })
        /// @resolution.name source=Bound target=Bound
        /// @resolution.construct source="Bound({ kind: \"included\", value })" parameters=({ kind: "included"; value: T#3 }) arguments=(provided({ kind: "included", value }) as { kind: "included"; value: T#3 }) return=Bound<T#3> kind=newtype target=Bound backing={ kind: "included"; value: T#3 } instance=Bound<T#3>
        /// @generic.instantiation id=Bound<T#3> template=Bound arguments=(T#3) owner=included
        /// @resolution.name source=value target=included.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=included.value

    }
}

extension<T: Copy> of Pair<T> {
/// @generic.template symbol=<module>#3 parameters=(T#4: Copy)
/// @generic.instance id=Pair<T#4> template=Pair arguments=(T#4)
/// @definition.extension symbol=<module>#3 form=local target=Pair<T#4>
/// @definition.method symbol=startBound slot=startBound type=<startBound.'a>(this: &startBound.'a immutable Pair<T#4>) => Bound<&startBound.'a immutable T#4>
/// @type.symbol symbol=T#2 source="T: Copy" type=T#4
/// @resolution.name source=Copy target=Copy
/// @resolution.name source=Pair target=Pair
/// @resolution.name source=T target=T#2

    startBound(&immutable this): Bound<&immutable T> {
    /// @generic.template symbol=startBound parent=template#3 parameters=('a)
    /// @type.symbol symbol=startBound type=<startBound.'a>(this: &startBound.'a immutable Pair<T#4>) => Bound<&startBound.'a immutable T#4>
    /// @generic.instance id="Bound<&startBound.'a immutable T#4>" template=Bound arguments=(&startBound.'a immutable T#4)
    /// @type.symbol symbol=startBound.this source="&immutable this" type=&startBound.'a immutable Pair<T#4>
    /// @resolution.name source=Bound target=Bound
    /// @generic.instance id="Bound<&'frame immutable T#4>" template=Bound arguments=(&'frame immutable T#4)
    /// @resolution.name source=T target=T#2

        Bound.included(&immutable this.start)
        /// @resolution.name source=Bound target=Bound
        /// @resolution.member source=Bound.included receiver=Bound type=(T#3) => Bound<T#3> kind=symbol target_receiver=Bound target=included
        /// @resolution.call source="Bound.included(&immutable this.start)" parameters=(&startBound.'a immutable T#4) arguments=(provided(&immutable this.start) as &startBound.'a immutable T#4) return=Bound<&startBound.'a immutable T#4> kind=symbol target=included instance="Bound<&startBound.'a immutable T#4>.<extension#1>.included"
        /// @generic.instantiation id="included<&startBound.'a immutable T#4>" template=included arguments=(&startBound.'a immutable T#4) owner=startBound
        /// @generic.instance id="included<&startBound.'a immutable T#4>" template=included arguments=(&startBound.'a immutable T#4)
        /// @resolution.member source=this.start receiver=&startBound.'a immutable Pair<T#4> type=T#4 kind=field target_receiver=&startBound.'a immutable Pair<T#4> key=start target=Pair.start target_type=T#4
        /// @resolution.receiver source=this kind=this declaration=<module>#3 type=&startBound.'a immutable Pair<T#4>
        /// @resolution.place source=this placement=startBound.'a lifetime=startBound.'a access="immutable"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.start placement=startBound.'a lifetime=startBound.'a access="immutable"
        /// @resolution.access source=this.start root=this keys=[start]

    }
}
"#);
}
