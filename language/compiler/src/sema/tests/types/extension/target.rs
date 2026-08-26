use crate::tests::{DirRows, TestSession};

#[test]
fn test_extend_a_tuple_at_its_language_item() {
    let session = TestSession::single(
        r#"
extension of (int32, string) {
    count(): int32 {
        return this[0];
    }
}

declare const pair: (int32, string);

const count = pair.count();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
extension of (int32, string) {
    count(): int32 {
        return this[0];
    }
}

declare const pair: (int32, string);

const count: int32 = pair.count<"constant">();

=== dir ===
extension of (int32, string) {
/// @definition.extension symbol=<module>#2 form=local target=(int32, string)
/// @definition.method symbol=count#1 slot=count type=<count#1.'a, count#1.P1: Place>(this: &count#1.'a readonly this) => int32

    count(): int32 {
    /// @generic.template symbol=count#1 parameters=('a, P1: Place)
    /// @type.symbol symbol=count#1 type=<count#1.'a, count#1.P1: Place>(this: &count#1.'a readonly this) => int32
    /// @type.symbol symbol=count.this type=&count#1.'a readonly (int32, string)

        return this[0];
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&count#1.'a readonly (int32, string)
        /// @resolution.place source=this placement=count#1.P1 lifetime=count#1.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this[0] placement=count#1.P1 lifetime=count#1.'a access="readonly"
        /// @resolution.access source=this[0] root=this keys=[0]
        /// @resolution.subscript source=this[0] type=int32 kind=member target="receiver=&count#1.'a readonly (int32, string), target=field(receiver=(int32, string), target=0, type=int32), type=int32"

    }
}

declare const pair: (int32, string);
/// @type.symbol symbol=pair source=pair type=(int32, string)
/// @resolution.pattern source=pair kind=binding target=pair

const count = pair.count();
/// @type.symbol symbol=count source=count type=int32
/// @resolution.pattern source=count kind=binding target=count
/// @resolution.name source=pair target=pair
/// @resolution.member source=pair.count receiver=(int32, string) type=<count#1.'a, count#1.P1: Place>(this: &count#1.'a readonly (int32, string)) => int32 kind=symbol target_receiver=(int32, string) target=count#1
/// @resolution.call source=pair.count() parameters=() return=int32 kind=symbol target=count#1 receiver=(int32, string) adjustments=(borrow(&'static readonly constant (int32, string))) instance="(int32, string).<extension#1>.count#1<\"constant\">"
/// @resolution.place source=pair placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=pair root=pair
/// @generic.instantiation id="count#1<\"constant\">" template=count#1 arguments=("constant")
"#,
        r#"

"#,
    );
}

/// Extend a tuple whose elements are the extension's parameters, instantiated per receiver.
#[test]
fn test_extend_a_generic_tuple_over_its_element_parameters() {
    let session = TestSession::single(
        r#"
extension<First: Copy, Second: Copy> of (First, Second) {
    swap(): (Second, First) {
        return (this[1], this[0]);
    }
}

declare const pair: (int32, boolean);

const swapped = pair.swap();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
extension<First: Copy, Second: Copy> of (First, Second) {
    swap(): (Second, First) {
        return (this[1], this[0]);
    }
}

declare const pair: (int32, boolean);

const swapped: (boolean, int32) = pair.swap<int32, boolean, "constant">();

=== dir ===
extension<First: Copy, Second: Copy> of (First, Second) {
/// @generic.template symbol=<module>#2 parameters=(First: Copy, Second: Copy)
/// @definition.extension symbol=<module>#2 form=local target=(First, Second)
/// @definition.method symbol=swap slot=swap type=<swap.'a, swap.P1: Place>(this: &swap.'a readonly this) => (Second, First)
/// @type.symbol symbol=First source="First: Copy" type=First
/// @resolution.name source=Copy target=Copy
/// @type.symbol symbol=Second source="Second: Copy" type=Second
/// @resolution.name source=Copy target=Copy
/// @resolution.name source=First target=First
/// @resolution.name source=Second target=Second

    swap(): (Second, First) {
    /// @generic.template symbol=swap parent=template#0 parameters=('a, P1: Place)
    /// @type.symbol symbol=swap type=<swap.'a, swap.P1: Place>(this: &swap.'a readonly this) => (Second, First)
    /// @type.symbol symbol=swap.this type=&swap.'a readonly (First, Second)
    /// @resolution.name source=Second target=Second
    /// @resolution.name source=First target=First

        return (this[1], this[0]);
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&swap.'a readonly (First, Second)
        /// @resolution.place source=this placement=swap.P1 lifetime=swap.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this[1] placement=swap.P1 lifetime=swap.'a access="readonly"
        /// @resolution.access source=this[1] root=this keys=[1]
        /// @resolution.subscript source=this[1] type=Second kind=member target="receiver=&swap.'a readonly (First, Second), target=field(receiver=(First, Second), target=1, type=Second), type=Second"
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&swap.'a readonly (First, Second)
        /// @resolution.place source=this placement=swap.P1 lifetime=swap.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this[0] placement=swap.P1 lifetime=swap.'a access="readonly"
        /// @resolution.access source=this[0] root=this keys=[0]
        /// @resolution.subscript source=this[0] type=First kind=member target="receiver=&swap.'a readonly (First, Second), target=field(receiver=(First, Second), target=0, type=First), type=First"

    }
}

declare const pair: (int32, boolean);
/// @type.symbol symbol=pair source=pair type=(int32, boolean)
/// @resolution.pattern source=pair kind=binding target=pair

const swapped = pair.swap();
/// @type.symbol symbol=swapped source=swapped type=(boolean, int32)
/// @resolution.pattern source=swapped kind=binding target=swapped
/// @resolution.name source=pair target=pair
/// @resolution.member source=pair.swap receiver=(int32, boolean) type=<swap.'a, swap.P1: Place>(this: &swap.'a readonly (int32, boolean)) => (boolean, int32) kind=symbol target_receiver=(int32, boolean) target=swap
/// @resolution.call source=pair.swap() parameters=() return=(boolean, int32) kind=symbol target=swap receiver=(int32, boolean) adjustments=(borrow(&'static readonly constant (int32, boolean))) instance="(First, Second).<extension#1>.swap<\"constant\">"
/// @resolution.place source=pair placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=pair root=pair
/// @generic.instantiation id="swap<int32, boolean, \"constant\">" template=swap arguments=(int32, boolean, "constant")
/// @generic.instantiation id="swap<int32, boolean>" template=swap arguments=(int32, boolean)
"#,
        r#"

"#,
    );
}

/// Select a repeated-parameter tuple extension only where the elements unify.
#[test]
fn test_reject_a_generic_tuple_extension_on_unequal_elements() {
    let session = TestSession::single(
        r#"
extension<T: Copy> of (T, T) {
    head(): T {
        return this[0];
    }
}

declare const same: (int32, int32);
declare const mixed: (int32, string);

const first = same.head();
const missing = mixed.head();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
extension<T: Copy> of (T, T) {
    head(): T {
        return this[0];
    }
}

declare const same: (int32, int32);
declare const mixed: (int32, string);

const first: int32 = same.head<int32, "constant">();
const missing = mixed.head();

=== dir ===
extension<T: Copy> of (T, T) {
/// @generic.template symbol=<module>#2 parameters=(T: Copy)
/// @definition.extension symbol=<module>#2 form=local target=(T, T)
/// @definition.method symbol=head slot=head type=<head.'a, head.P1: Place>(this: &head.'a readonly this) => T
/// @type.symbol symbol=T source="T: Copy" type=T
/// @resolution.name source=Copy target=Copy
/// @resolution.name source=T target=T
/// @resolution.name source=T target=T

    head(): T {
    /// @generic.template symbol=head parent=template#0 parameters=('a, P1: Place)
    /// @type.symbol symbol=head type=<head.'a, head.P1: Place>(this: &head.'a readonly this) => T
    /// @type.symbol symbol=head.this type=&head.'a readonly (T, T)
    /// @resolution.name source=T target=T

        return this[0];
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&head.'a readonly (T, T)
        /// @resolution.place source=this placement=head.P1 lifetime=head.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this[0] placement=head.P1 lifetime=head.'a access="readonly"
        /// @resolution.access source=this[0] root=this keys=[0]
        /// @resolution.subscript source=this[0] type=T kind=member target="receiver=&head.'a readonly (T, T), target=field(receiver=(T, T), target=0, type=T), type=T"

    }
}

declare const same: (int32, int32);
/// @type.symbol symbol=same source=same type=(int32, int32)
/// @resolution.pattern source=same kind=binding target=same

declare const mixed: (int32, string);
/// @type.symbol symbol=mixed source=mixed type=(int32, string)
/// @resolution.pattern source=mixed kind=binding target=mixed

const first = same.head();
/// @type.symbol symbol=first source=first type=int32
/// @resolution.pattern source=first kind=binding target=first
/// @resolution.name source=same target=same
/// @resolution.member source=same.head receiver=(int32, int32) type=<head.'a, head.P1: Place>(this: &head.'a readonly (int32, int32)) => int32 kind=symbol target_receiver=(int32, int32) target=head
/// @resolution.call source=same.head() parameters=() return=int32 kind=symbol target=head receiver=(int32, int32) adjustments=(borrow(&'static readonly constant (int32, int32))) instance="(T, T).<extension#1>.head<\"constant\">"
/// @resolution.place source=same placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=same root=same
/// @generic.instantiation id="head<int32, \"constant\">" template=head arguments=(int32, "constant")
/// @generic.instantiation id=head<int32> template=head arguments=(int32)

const missing = mixed.head();
/// @type.symbol symbol=missing source=missing type=<error>
/// @resolution.pattern source=missing kind=binding target=missing
/// @resolution.name source=mixed target=mixed
/// @resolution.place source=mixed placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=mixed root=mixed
/// @resolution.rejected source=mixed.head
/// @resolution.rejected source=mixed.head()
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'head' does not exist on type '(int32, string)'"
/// @diagnostic.label line=12 column=23 span="head" line_source="const missing = mixed.head();"
"#,
    );
}

#[test]
fn test_extend_a_fixed_array_at_its_language_item() {
    let session = TestSession::single(
        r#"
extension of [int32; 3] {
    head(): int32 {
        return this[0];
    }
}

declare const triple: [int32; 3];

const head = triple.head();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
extension of [int32; 3] {
    head(): int32 {
        return this[0];
    }
}

declare const triple: [int32; 3];

const head: int32 = triple.head<"constant">();

=== dir ===
extension of [int32; 3] {
/// @definition.extension symbol=<module>#2 form=local target=FixedArray<int32, 3>
/// @definition.method symbol=head#1 slot=head type=<head#1.'a, head#1.P1: Place>(this: &head#1.'a readonly this) => int32

    head(): int32 {
    /// @generic.template symbol=head#1 parameters=('a, P1: Place)
    /// @type.symbol symbol=head#1 type=<head#1.'a, head#1.P1: Place>(this: &head#1.'a readonly this) => int32
    /// @type.symbol symbol=head.this type=&head#1.'a readonly FixedArray<int32, 3>

        return this[0];
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&head#1.'a readonly FixedArray<int32, 3>
        /// @resolution.place source=this placement=head#1.P1 lifetime=head#1.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this[0] placement=head#1.P1 lifetime=head#1.'a access="readonly"
        /// @resolution.access source=this[0] root=this keys=[0]
        /// @resolution.subscript source=this[0] type=int32 kind=call target="index#1(parameters=(isize), arguments=(provided(0) as isize), return=WithAccess<&head#1.'a int32, \"readonly\">)"
        /// @generic.instantiation id="index#1<int32, 3, \"readonly\", head#1.P1>" template=index#1 arguments=(int32, 3, "readonly", head#1.P1)

    }
}

declare const triple: [int32; 3];
/// @type.symbol symbol=triple source=triple type=FixedArray<int32, 3>
/// @resolution.pattern source=triple kind=binding target=triple

const head = triple.head();
/// @type.symbol symbol=head source=head type=int32
/// @resolution.pattern source=head kind=binding target=head
/// @resolution.name source=triple target=triple
/// @resolution.member source=triple.head receiver=FixedArray<int32, 3> type=<head#1.'a, head#1.P1: Place>(this: &head#1.'a readonly FixedArray<int32, 3>) => int32 kind=symbol target_receiver=FixedArray<int32, 3> target=head#1
/// @resolution.call source=triple.head() parameters=() return=int32 kind=symbol target=head#1 receiver=FixedArray<int32, 3> adjustments=(borrow(&'static readonly constant FixedArray<int32, 3>)) instance="FixedArray<int32, 3>.<extension#1>.head#1<\"constant\">"
/// @resolution.place source=triple placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=triple root=triple
/// @generic.instantiation id="head#1<\"constant\">" template=head#1 arguments=("constant")
"#,
        r#"
"#,
    );
}

#[test]
fn test_extend_a_slice_at_its_language_item() {
    let session = TestSession::single(
        r#"
extension of [int32] {
    hasNone(): boolean {
        return this.length == 0;
    }
}

declare const values: &readonly [int32];

const none = values.hasNone();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
extension of [int32] {
    hasNone(): boolean {
        return this.length == 0;
    }
}

declare const values: &'static readonly [int32];

const none: boolean = values.hasNone();

=== dir ===
extension of [int32] {
/// @definition.extension symbol=<module>#2 form=local target=Slice<int32>
/// @definition.method symbol=hasNone slot=hasNone type=(this: this) => boolean

    hasNone(): boolean {
    /// @type.symbol symbol=hasNone type=(this: this) => boolean
    /// @type.symbol symbol=hasNone.this type=Slice<int32>

        return this.length == 0;
        /// @resolution.member source=this.length receiver=Slice<int32> type=isize kind=call target="length(parameters=(), arguments=(), return=isize)"
        /// @resolution.operator source="this.length == 0" type=boolean operator="==" kind=builtin operands=[this.length as isize families=(integer), 0 as isize families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Slice<int32>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id="length<int32, \"local\">" template=length arguments=(int32, "local")

    }
}

declare const values: &readonly [int32];
/// @type.symbol symbol=values source=values type=&'static readonly constant Slice<int32>
/// @resolution.pattern source=values kind=binding target=values

const none = values.hasNone();
/// @type.symbol symbol=none source=none type=boolean
/// @resolution.pattern source=none kind=binding target=none
/// @resolution.name source=values target=values
/// @resolution.member source=values.hasNone receiver=&'static readonly constant Slice<int32> type=(this: Slice<int32>) => boolean kind=symbol target_receiver=&'static readonly constant Slice<int32> target=hasNone
/// @resolution.call source=values.hasNone() parameters=() return=boolean kind=symbol target=hasNone receiver=&'static readonly constant Slice<int32> adjustments=(&'static readonly constant Slice<int32> => direct -> Slice<int32>)
/// @resolution.place source=values placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=values root=values
"#,
        r#"
"#,
    );
}

#[test]
fn test_extend_a_function_type_at_its_language_item() {
    let session = TestSession::single(
        r#"
extension of (value: int32) => int32 {
    arity(): int32 {
        return 1;
    }
}

declare const increment: (value: int32) => int32;

const arity = increment.arity();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
extension of (value: int32) => int32 {
    arity(): int32 {
        return 1;
    }
}

declare const increment: (arg0: int32) => int32;

const arity: int32 = increment.arity();

=== dir ===
extension of (value: int32) => int32 {
/// @definition.extension symbol=<module>#2 form=local target=Function<(int32,), int32>
/// @definition.method symbol=arity#1 slot=arity type=(this: this) => int32
/// @type.symbol symbol=value#1 source="value: int32" type=int32

    arity(): int32 {
    /// @type.symbol symbol=arity#1 type=(this: this) => int32
    /// @type.symbol symbol=arity.this type=Function<(int32,), int32>

        return 1;
    }
}

declare const increment: (value: int32) => int32;
/// @type.symbol symbol=increment source=increment type=Function<(int32,), int32>
/// @resolution.pattern source=increment kind=binding target=increment
/// @type.symbol symbol=value source="value: int32" type=int32

const arity = increment.arity();
/// @type.symbol symbol=arity source=arity type=int32
/// @resolution.pattern source=arity kind=binding target=arity
/// @resolution.name source=increment target=increment
/// @resolution.member source=increment.arity receiver=Function<(int32,), int32> type=(this: Function<(int32,), int32>) => int32 kind=symbol target_receiver=Function<(int32,), int32> target=arity#1
/// @resolution.call source=increment.arity() parameters=() return=int32 kind=symbol target=arity#1 receiver=Function<(int32,), int32>
/// @resolution.place source=increment placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=increment root=increment
"#,
        r#"

"#,
    );
}

#[test]
fn test_reject_an_object_type_extension_target() {
    let session = TestSession::single(
        r#"
extension of { x: int32 } {
    double(): int32 {
        return this.x * 2;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
extension of { x: int32 } {
    double(): int32 {
        return this.x * 2;
    }
}

=== dir ===
extension of { x: int32 } {
/// @definition.extension symbol=<module>#2 form=local target={ x: int32 }
/// @definition.method symbol=double slot=double type=<double.P0: Place>(this: Managed<this, double.P0>) => int32
/// @type.symbol symbol=x source="x: int32" type=int32

    double(): int32 {
    /// @generic.template symbol=double parameters=(P0: Place)
    /// @type.symbol symbol=double type=<double.P0: Place>(this: Managed<this, double.P0>) => int32
    /// @type.symbol symbol=double.this type=Managed<{ x: int32 }, double.P0>

        return this.x * 2;
        /// @resolution.member source=this.x receiver=Managed<{ x: int32 }, double.P0> type=int32 kind=field target_receiver=Managed<{ x: int32 }, double.P0> key=x target_type=int32
        /// @resolution.operator source="this.x * 2" type=int32 operator="*" kind=builtin operands=[this.x as int32 families=(integer), 2 as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Managed<{ x: int32 }, double.P0>
        /// @resolution.place source=this placement=double.P0 lifetime="frame" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.x placement=double.P0 lifetime="frame" access="mutable"
        /// @resolution.access source=this.x root=this keys=[x]

    }
}
"#,
        r#"
/// @diagnostic.error id=invalid-extension-target message="extension target '{ x: int32 }' has no root declaration"
/// @diagnostic.label line=2 column=1 span="extension of { x: int32 } {\n    double(): int32 {\n        return this.x * 2;\n    }\n}" line_source="extension of { x: int32 } {"
/// @diagnostic.note message="an extension targets a declaration, a primitive, a tuple, array, slice, or function type, or a bounded type parameter"
"#,
    );
}

#[test]
fn test_reject_an_intersection_extension_target() {
    let session = TestSession::single(
        r#"
interface Named {
    name: string;
}
interface Aged {
    age: int32;
}

extension of Named & Aged {
    describe(): string {
        return this.name;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Named {
    name: string;
}
interface Aged {
    age: int32;
}

extension of Named & Aged {
    describe(): string {
        return this.name;
    }
}

=== dir ===
interface Named {
/// @type.symbol symbol=Named type=Named
/// @definition.interface symbol=Named
/// @definition.where symbol=Named relation=satisfies left=this right=Named
/// @definition.field symbol=Named.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=Named.name source="name: string" type=string

}
interface Aged {
/// @type.symbol symbol=Aged type=Aged
/// @definition.interface symbol=Aged
/// @definition.where symbol=Aged relation=satisfies left=this right=Aged
/// @definition.field symbol=Aged.age source="age: int32" key=age type=int32

    age: int32;
    /// @type.symbol symbol=Aged.age source="age: int32" type=int32

}

extension of Named & Aged {
/// @definition.extension symbol=<module>#2 form=local target=Named & Aged
/// @definition.method symbol=describe slot=describe type=<describe.P0: Place>(this: Managed<this, describe.P0>) => string
/// @resolution.name source=Named target=Named
/// @resolution.name source=Aged target=Aged

    describe(): string {
    /// @generic.template symbol=describe parameters=(P0: Place)
    /// @type.symbol symbol=describe type=<describe.P0: Place>(this: Managed<this, describe.P0>) => string
    /// @type.symbol symbol=describe.this type=Managed<Named & Aged, describe.P0>

        return this.name;
        /// @resolution.member source=this.name receiver=Managed<Named & Aged, describe.P0> type=Managed<string, describe.P0> kind=field target_receiver=Managed<Named & Aged, describe.P0> key=name target=Named.name target_type=Managed<string, describe.P0>
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Managed<Named & Aged, describe.P0>
        /// @resolution.place source=this placement=describe.P0 lifetime="frame" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.name placement=describe.P0 lifetime="frame" access="mutable"
        /// @resolution.access source=this.name root=this keys=[name]

    }
}
"#,
        r#"
/// @diagnostic.error id=invalid-extension-target message="extension target 'Named & Aged' has no root declaration"
/// @diagnostic.label line=9 column=20 span="&" line_source="extension of Named & Aged {"
/// @diagnostic.note message="an extension targets a declaration, a primitive, a tuple, array, slice, or function type, or a bounded type parameter"
"#,
    );
}

#[test]
fn test_reject_a_union_extension_target() {
    let session = TestSession::single(
        r#"
struct Circle {}
struct Square {}

extension of Circle | Square {
    area(): int32 {
        return 1;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Circle {}
struct Square {}

extension of Circle | Square {
    area(): int32 {
        return 1;
    }
}

=== dir ===
struct Circle {}
/// @type.symbol symbol=Circle source="struct Circle {}" type=Circle
/// @definition.struct symbol=Circle source="struct Circle {}"

struct Square {}
/// @type.symbol symbol=Square source="struct Square {}" type=Square
/// @definition.struct symbol=Square source="struct Square {}"

extension of Circle | Square {
/// @definition.extension symbol=<module>#2 form=local target=Circle | Square
/// @definition.method symbol=area slot=area type=(this: this) => int32
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=Square target=Square

    area(): int32 {
    /// @type.symbol symbol=area type=(this: this) => int32
    /// @type.symbol symbol=area.this type=Circle | Square

        return 1;
    }
}
"#,
        r#"
/// @diagnostic.error id=invalid-extension-target message="extension target 'Circle | Square' has no root declaration"
/// @diagnostic.label line=5 column=21 span="|" line_source="extension of Circle | Square {"
/// @diagnostic.note message="an extension targets a declaration, a primitive, a tuple, array, slice, or function type, or a bounded type parameter"
"#,
    );
}

#[test]
fn test_extend_a_nominal_through_its_alias() {
    let session = TestSession::single(
        r#"
struct User {}

type Account = User;

extension of Account {
    label(): string {
        return "account";
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct User {}

type Account = User;

extension of Account {
    label(): string {
        return "account";
    }
}

=== dir ===
struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"

type Account = User;
/// @type.symbol symbol=Account source="type Account = User" type=User
/// @definition.type symbol=Account source="type Account = User" value=User
/// @resolution.name source=User target=User

extension of Account {
/// @definition.extension symbol=<module>#2 form=local target=User
/// @definition.method symbol=label slot=label type=<label.'a, label.P1: Place>(this: &label.'a readonly this) => string
/// @resolution.name source=Account target=Account

    label(): string {
    /// @generic.template symbol=label parameters=('a, P1: Place)
    /// @type.symbol symbol=label type=<label.'a, label.P1: Place>(this: &label.'a readonly this) => string
    /// @type.symbol symbol=label.this type=&label.'a readonly User

        return "account";
    }
}
"#,
        r#"

"#,
    );
}

#[test]
fn test_reject_members_on_an_unbounded_blanket() {
    let session = TestSession::single(
        r#"
extension<T> of T {
    describe(): string {
        return "anything";
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
extension<T> of T {
    describe(): string {
        return "anything";
    }
}

=== dir ===
extension<T> of T {
/// @generic.template symbol=<module>#2 parameters=(T)
/// @definition.extension symbol=<module>#2 form=local target=T
/// @definition.method symbol=describe slot=describe type=(this: this) => string
/// @type.symbol symbol=T source=T type=T
/// @resolution.name source=T target=T

    describe(): string {
    /// @type.symbol symbol=describe type=(this: this) => string
    /// @type.symbol symbol=describe.this type=T

        return "anything";
    }
}
"#,
        r#"
/// @diagnostic.error id=unanchored-blanket-member message="member 'describe' on a blanket extension implements no declared interface member"
/// @diagnostic.label line=2 column=17 span="T" line_source="extension<T> of T {"
"#,
    );
}

#[test]
fn test_reject_a_where_clause_bounding_no_parameter() {
    let session = TestSession::single(
        r#"
newtype interface Show {
    show(this): string;
}

struct User {}

extension of User where int32: Show {
    label(): string {
        return "user";
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Show {
    show(this): string;
}

struct User {}

extension of User where int32: Show {
    label(): string {
        return "user";
    }
}

=== dir ===
newtype interface Show {
/// @type.symbol symbol=Show type=Show
/// @definition.interface symbol=Show nominal=true
/// @definition.where symbol=Show relation=satisfies left=this right=Show
/// @definition.method symbol=Show.show source="show(this): string" slot=show type=(this: this) => string

    show(this): string;
    /// @type.symbol symbol=Show.show source="show(this): string" type=(this: this) => string
    /// @type.symbol symbol=Show.show.this source=this type=this

}

struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"

extension of User where int32: Show {
/// @definition.extension symbol=<module>#2 form=local target=User
/// @definition.method symbol=label slot=label type=<label.'a, label.P1: Place>(this: &label.'a readonly this) => string
/// @resolution.name source=User target=User
/// @resolution.name source=Show target=Show

    label(): string {
    /// @generic.template symbol=label parent=template#1 parameters=('a, P1: Place)
    /// @type.symbol symbol=label type=<label.'a, label.P1: Place>(this: &label.'a readonly this) => string
    /// @type.symbol symbol=label.this type=&label.'a readonly User

        return "user";
    }
}
"#,
        r#"
/// @diagnostic.error id=where-clause-without-parameter message="where clause bounds no parameter of the declaration"
/// @diagnostic.label line=8 column=25 span="int32" line_source="extension of User where int32: Show {"
"#,
    );
}
