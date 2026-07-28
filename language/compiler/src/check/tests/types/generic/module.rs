use crate::tests::{DirRows, TestSession};

#[test]
fn test_induce_lifetimes_through_a_forward_generic_bound() {
    let session = TestSession::single(
        r#"
interface Holder<T: View> {
    value: T;
}

struct View {
    user: &readonly User;
}

struct User {}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
interface Holder<in out T: View> {
    value: T;
}

struct View<'a> {
    user: &'a readonly User;
}

struct User {}

=== checked ===
interface Holder<T: View> {
/// @generic.template symbol=Holder parameters=(in out T: View<?1>)
/// @type.symbol symbol=Holder type=Holder
/// @definition.interface symbol=Holder template=(in out T: View<?1>)
/// @definition.where symbol=Holder relation=satisfies left=this right=Holder<T>
/// @definition.field symbol=Holder.value source="value: T" key=value type=T
/// @type.symbol symbol=Holder.T source="T: View" type=T
/// @resolution.name source=View target=View

    value: T;
    /// @type.symbol symbol=Holder.value source="value: T" type=T
    /// @resolution.name source=T target=Holder.T

}

struct View {
/// @generic.template symbol=View parameters=('a)
/// @type.symbol symbol=View type=View
/// @definition.struct symbol=View template=('a)
/// @definition.field symbol=View.user source="user: &readonly User" key=user type=&View.'a readonly User

    user: &readonly User;
    /// @type.symbol symbol=View.user source="user: &readonly User" type=&View.'a readonly User
    /// @resolution.name source=User target=User

}

struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"
"#,
    );
}

#[test]
fn test_induce_lifetimes_through_a_cross_module_declaration_cycle() {
    let compiler = TestSession::builder()
        .module(
            "b.ds",
            r#"
import { Foo } from "./a.ds";

export struct Bar {
    foo: Foo;
}

export struct Baz {
    user: &readonly User;
}

export struct User {}
"#,
        )
        .module(
            "a.ds",
            r#"
import { Baz } from "./b.ds";

export struct Foo {
    baz: Baz;
}
"#,
        )
        .build();

    compiler.assert_dir_checked_many(
        &["a.ds", "b.ds"],
        DirRows::checked().with_reference_types(),
        r#"
=== a.ds ===

=== annotated ===
import { Baz } from "./b.ds";

export struct Foo<'a> {
    baz: Baz<'a>;
}

=== checked ===
import { Baz } from "./b.ds";

export struct Foo {
/// @generic.template symbol=Foo parameters=('a)
/// @type.symbol symbol=Foo type=Foo
/// @definition.struct symbol=Foo template=('a)
/// @definition.field symbol=Foo.baz source="baz: Baz" key=baz type=b.Baz<Foo.'a>

    baz: Baz;
    /// @type.symbol symbol=Foo.baz source="baz: Baz" type=b.Baz<Foo.'a>
    /// @resolution.name source=Baz target=b.Baz

}

/// @generic.instance id=b.Baz<Foo.'a> template=b.Baz arguments=(Foo.'a)

=== b.ds ===

=== annotated ===
import { Foo } from "./a.ds";

export struct Bar<'a> {
    foo: Foo<'a>;
}

export struct Baz<'a> {
    user: &'a readonly User;
}

export struct User {}

=== checked ===
import { Foo } from "./a.ds";

export struct Bar {
/// @generic.template symbol=Bar parameters=('a)
/// @type.symbol symbol=Bar type=Bar
/// @definition.struct symbol=Bar template=('a)
/// @definition.field symbol=Bar.foo source="foo: Foo" key=foo type=a.Foo<Bar.'a>

    foo: Foo;
    /// @type.symbol symbol=Bar.foo source="foo: Foo" type=a.Foo<Bar.'a>
    /// @resolution.name source=Foo target=a.Foo

}

export struct Baz {
/// @generic.template symbol=Baz parameters=('a)
/// @type.symbol symbol=Baz type=Baz
/// @definition.struct symbol=Baz template=('a)
/// @definition.field symbol=Baz.user source="user: &readonly User" key=user type=&Baz.'a readonly User

    user: &readonly User;
    /// @type.symbol symbol=Baz.user source="user: &readonly User" type=&Baz.'a readonly User
    /// @resolution.name source=User target=User

}

export struct User {}
/// @type.symbol symbol=User source="export struct User {}" type=User
/// @definition.struct symbol=User source="export struct User {}"

/// @generic.instance id=a.Foo<Bar.'a> template=a.Foo arguments=(Bar.'a)
"#,
    );
}

#[test]
fn test_imported_generic_type_accepts_local_type_argument() {
    let compiler = TestSession::builder()
        .module(
            "lib.ds",
            r#"
export interface Box<T> {
    value: T;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Box } from "./lib.ds";

type Wrapped<T> = Box<T>;
"#,
        )
        .build();

    compiler.assert_dir_checked_many(
        &["lib.ds", "main.ds"],
        DirRows::checked().with_reference_types(),
        r#"
=== lib.ds ===

=== annotated ===
export interface Box<in out T> {
    value: T;
}

=== checked ===
export interface Box<T> {
/// @generic.template symbol=Box parameters=(in out T)
/// @type.symbol symbol=Box type=Box
/// @definition.interface symbol=Box template=(in out T)
/// @definition.where symbol=Box relation=satisfies left=this right=Box<T>
/// @definition.field symbol=Box.value source="value: T" key=value type=T
/// @type.symbol symbol=Box.T source=T type=T

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T
    /// @resolution.name source=T target=Box.T

}

=== main.ds ===

=== annotated ===
import { Box } from "./lib.ds";

type Wrapped<T> = Box<T>;

=== checked ===
import { Box } from "./lib.ds";

type Wrapped<T> = Box<T>;
/// @generic.template symbol=Wrapped parameters=(T)
/// @type.symbol symbol=Wrapped source="type Wrapped<T> = Box<T>" type=lib.Box<T>
/// @definition.type symbol=Wrapped source="type Wrapped<T> = Box<T>" template=(T) value=lib.Box<T>
/// @type.symbol symbol=Wrapped.T source=T type=T
/// @resolution.name source=Box target=lib.Box
/// @resolution.name source=T target=Wrapped.T

/// @generic.instance id=lib.Box<T> template=lib.Box arguments=(T)
"#,
    );
}

#[test]
fn test_generic_newtype_interface_extends_generic_interface() {
    let compiler = TestSession::builder()
        .module(
            "ops.ds",
            r#"
export newtype interface PartialEqual<T = this> {
    equal(other: T): boolean;
}

export newtype interface Equal<T = this> extends PartialEqual<T> {}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Equal } from "./ops.ds";

type Used = Equal<string>;
"#,
        )
        .build();

    compiler.assert_dir_checked_many(
        &["ops.ds", "main.ds"],
        DirRows::checked().with_reference_types(),
        r#"
=== ops.ds ===

=== annotated ===
export newtype interface PartialEqual<in T = this> {
    equal(other: T): boolean;
}

export newtype interface Equal<in T = this> extends PartialEqual<T> {}

=== checked ===
export newtype interface PartialEqual<T = this> {
/// @generic.template symbol=PartialEqual parameters=(in T#1 = this)
/// @type.symbol symbol=PartialEqual type=PartialEqual
/// @definition.interface symbol=PartialEqual template=(in T#1 = this) nominal=true
/// @definition.where symbol=PartialEqual relation=satisfies left=this right=PartialEqual<T#1>
/// @definition.method symbol=PartialEqual.equal source="equal(other: T): boolean" slot=equal type=(this: this, T#1) => boolean
/// @type.symbol symbol=PartialEqual.T source="T = this" type=T#1

    equal(other: T): boolean;
    /// @type.symbol symbol=PartialEqual.equal source="equal(other: T): boolean" type=(this: this, T#1) => boolean
    /// @type.symbol symbol=PartialEqual.equal.other source="other: T" type=T#1
    /// @resolution.name source=T target=PartialEqual.T

}

export newtype interface Equal<T = this> extends PartialEqual<T> {}
/// @generic.template symbol=Equal parameters=(in T#2 = this)
/// @type.symbol symbol=Equal source="export newtype interface Equal<T = this> extends PartialEqual<T> {}" type=Equal
/// @definition.interface symbol=Equal source="export newtype interface Equal<T = this> extends PartialEqual<T> {}" template=(in T#2 = this) nominal=true
/// @definition.where symbol=Equal source="export newtype interface Equal<T = this> extends PartialEqual<T> {}" relation=satisfies left=this right=Equal<T#2>
/// @definition.extends symbol=Equal source=PartialEqual<T> target=PartialEqual<T#2>
/// @type.symbol symbol=Equal.T source="T = this" type=T#2
/// @resolution.name source=PartialEqual target=PartialEqual
/// @resolution.name source=T target=Equal.T

=== main.ds ===

=== annotated ===
import { Equal } from "./ops.ds";

type Used = Equal<string>;

=== checked ===
import { Equal } from "./ops.ds";

type Used = Equal<string>;
/// @type.symbol symbol=Used source="type Used = Equal<string>" type=ops.Equal<string>
/// @definition.type symbol=Used source="type Used = Equal<string>" value=ops.Equal<string>
/// @resolution.name source=Equal target=ops.Equal

/// @generic.instance id=ops.Equal<string> template=ops.Equal arguments=(string)
"#,
    );
}

#[test]
fn test_imported_generic_function_instantiates_in_calling_module() {
    let compiler = TestSession::builder()
        .module(
            "lib.ds",
            r#"
export function identity<T>(value: T): T {
    return value;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { identity } from "./lib.ds";

const number = identity(1);
const text = identity("x");
"#,
        )
        .build();

    compiler.assert_dir_checked_many(
        &["lib.ds", "main.ds"],
        DirRows::checked().with_reference_types(),
        r#"
=== lib.ds ===

=== annotated ===
export function identity<T>(value: T): T {
    return value;
}

=== checked ===
export function identity<T>(value: T): T {
/// @generic.template symbol=identity parameters=(T)
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=identity.T source=T type=T
/// @type.symbol symbol=identity.value source="value: T" type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

    return value;
    /// @type.node source=value type=T
    /// @resolution.name source=value target=identity.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=identity.value

}

=== main.ds ===

=== annotated ===
import { identity } from "./lib.ds";

const number: 1 = identity<1>(1);
const text: "x" = identity<"x">("x");

=== checked ===
import { identity } from "./lib.ds";

const number = identity(1);
/// @type.symbol symbol=number source=number type=1
/// @resolution.pattern source=number kind=binding target=number
/// @type.node source=identity type=(1) => 1
/// @type.node source=identity(1) type=1
/// @resolution.name source=identity target=lib.identity
/// @resolution.call source=identity(1) parameters=(1) arguments=(provided(1) as 1) return=1 kind=symbol target=lib.identity instance=lib.identity<1>
/// @generic.instance source=identity(1) id=lib.identity<1>
/// @type.node source=1 type=1

const text = identity("x");
/// @type.symbol symbol=text source=text type="x"
/// @resolution.pattern source=text kind=binding target=text
/// @type.node source="identity(\"x\")" type="x"
/// @type.node source=identity type=("x") => "x"
/// @resolution.name source=identity target=lib.identity
/// @resolution.call source="identity(\"x\")" parameters=("x") arguments=(provided("x") as "x") return="x" kind=symbol target=lib.identity instance="lib.identity<\"x\">"
/// @generic.instance source="identity(\"x\")" id="lib.identity<\"x\">"
/// @type.node source="\"x\"" type="x"

/// @generic.instance id="lib.identity<\"x\">" template=lib.identity arguments=("x")
/// @generic.instance id=lib.identity<1> template=lib.identity arguments=(1)
"#,
    );
}

#[test]
fn test_imported_generic_function_uses_exported_body_inference() {
    let compiler = TestSession::builder()
        .module(
            "lib.ds",
            r#"
export function identity<T>(value: T) {
    return value;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { identity } from "./lib.ds";

const text = identity("x");
"#,
        )
        .build();

    compiler.assert_dir_checked_many(
        &["lib.ds", "main.ds"],
        DirRows::checked().with_reference_types(),
        r#"
=== lib.ds ===

=== annotated ===
export function identity<T>(value: T): T {
    return value;
}

=== checked ===
export function identity<T>(value: T) {
/// @generic.template symbol=identity parameters=(T)
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=identity.T source=T type=T
/// @type.symbol symbol=identity.value source="value: T" type=T
/// @resolution.name source=T target=identity.T

    return value;
    /// @type.node source=value type=T
    /// @resolution.name source=value target=identity.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=identity.value

}

=== main.ds ===

=== annotated ===
import { identity } from "./lib.ds";

const text: "x" = identity<"x">("x");

=== checked ===
import { identity } from "./lib.ds";

const text = identity("x");
/// @type.symbol symbol=text source=text type="x"
/// @resolution.pattern source=text kind=binding target=text
/// @type.node source="identity(\"x\")" type="x"
/// @type.node source=identity type=("x") => "x"
/// @resolution.name source=identity target=lib.identity
/// @resolution.call source="identity(\"x\")" parameters=("x") arguments=(provided("x") as "x") return="x" kind=symbol target=lib.identity instance="lib.identity<\"x\">"
/// @generic.instance source="identity(\"x\")" id="lib.identity<\"x\">"
/// @type.node source="\"x\"" type="x"

/// @generic.instance id="lib.identity<\"x\">" template=lib.identity arguments=("x")
"#,
    );
}

#[test]
fn test_defaulted_parameter_fills_omitted_annotation_argument() {
    let session = TestSession::single(
        r#"
interface Iter<T, in out R = unknown> {
    next(): T;
}

declare function probe(values: Iter<int32>): boolean;
const value = probe(todo("iter"));
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Iter<out T, in out R = unknown> {
    next(): T;
}

declare function probe(values: Dynamic<Iter<int32, unknown>>): boolean;
const value: boolean = probe(todo("iter" as string | undefined));

=== checked ===
interface Iter<T, in out R = unknown> {
/// @generic.template symbol=Iter parameters=(out T, in out R = unknown)
/// @type.symbol symbol=Iter type=Iter
/// @definition.interface symbol=Iter template=(out T, in out R = unknown)
/// @definition.where symbol=Iter relation=satisfies left=this right=Iter<T, R>
/// @definition.method symbol=Iter.next source="next(): T" slot=next type=(this: this) => T
/// @type.symbol symbol=Iter.T source=T type=T
/// @type.symbol symbol=Iter.R source="in out R = unknown" type=R

    next(): T;
    /// @type.symbol symbol=Iter.next source="next(): T" type=(this: this) => T
    /// @resolution.name source=T target=Iter.T

}

declare function probe(values: Iter<int32>): boolean;
/// @type.symbol symbol=probe source="declare function probe(values: Iter<int32>): boolean" type=(Dynamic<Iter<int32, unknown>>) => boolean
/// @type.symbol symbol=probe.values source="values: Iter<int32>" type=Dynamic<Iter<int32, unknown>>
/// @resolution.name source=Iter target=Iter

const value = probe(todo("iter"));
/// @type.symbol symbol=value source=value type=boolean
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=probe target=probe
/// @resolution.call source="probe(todo(\"iter\"))" parameters=(Dynamic<Iter<int32, unknown>>) arguments=(provided(todo("iter")) as Dynamic<Iter<int32, unknown>>) return=boolean kind=symbol target=probe
/// @resolution.name source=todo target=error.panic.todo
/// @resolution.call source="todo(\"iter\")" parameters=(string | undefined) arguments=(provided("iter") as string | undefined) return=never kind=symbol target=error.panic.todo

/// @generic.instance id="Iter<int32, unknown>" template=Iter arguments=(int32, unknown)
"#,
        "",
    );
}

#[test]
fn test_defaulted_parameter_fills_through_reexport_chain() {
    let session = TestSession::builder()
        .module(
            "inner.ds",
            r#"
export newtype interface Iter<T, in out R = unknown> {
    next(): T;
}
"#,
        )
        .module(
            "lib.ds",
            r#"
export { Iter } from "./inner.ds";
"#,
        )
        .module(
            "main.ds",
            r#"
import { Iter } from "./lib.ds";

declare function probe(values: Iter<int32>): boolean;
const value = probe(todo("iter"));
"#,
        )
        .build();

    session.assert_dir_checked_many(
        &["inner.ds", "lib.ds", "main.ds"],
        DirRows::checked(),
        r#"
=== inner.ds ===

=== annotated ===
export newtype interface Iter<out T, in out R = unknown> {
    next(): T;
}

=== checked ===
export newtype interface Iter<T, in out R = unknown> {
/// @generic.template symbol=Iter parameters=(out T, in out R = unknown)
/// @type.symbol symbol=Iter type=Iter
/// @definition.interface symbol=Iter template=(out T, in out R = unknown) nominal=true
/// @definition.where symbol=Iter relation=satisfies left=this right=Iter<T, R>
/// @definition.method symbol=Iter.next source="next(): T" slot=next type=(this: this) => T
/// @type.symbol symbol=Iter.T source=T type=T
/// @type.symbol symbol=Iter.R source="in out R = unknown" type=R

    next(): T;
    /// @type.symbol symbol=Iter.next source="next(): T" type=(this: this) => T
    /// @resolution.name source=T target=Iter.T

}

=== lib.ds ===

=== annotated ===
export { Iter } from "./inner.ds";

=== checked ===
export { Iter } from "./inner.ds";

=== main.ds ===

=== annotated ===
import { Iter } from "./lib.ds";

declare function probe(values: Dynamic<Iter<int32, unknown>>): boolean;
const value: boolean = probe(todo("iter" as string | undefined));

=== checked ===
import { Iter } from "./lib.ds";

declare function probe(values: Iter<int32>): boolean;
/// @type.symbol symbol=probe source="declare function probe(values: Iter<int32>): boolean" type=(Dynamic<inner.Iter<int32, unknown>>) => boolean
/// @type.symbol symbol=probe.values source="values: Iter<int32>" type=Dynamic<inner.Iter<int32, unknown>>
/// @resolution.name source=Iter target=inner.Iter

const value = probe(todo("iter"));
/// @type.symbol symbol=value source=value type=boolean
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=probe target=probe
/// @resolution.call source="probe(todo(\"iter\"))" parameters=(Dynamic<inner.Iter<int32, unknown>>) arguments=(provided(todo("iter")) as Dynamic<inner.Iter<int32, unknown>>) return=boolean kind=symbol target=probe
/// @resolution.name source=todo target=error.panic.todo
/// @resolution.call source="todo(\"iter\")" parameters=(string | undefined) arguments=(provided("iter") as string | undefined) return=never kind=symbol target=error.panic.todo

/// @generic.instance id="inner.Iter<int32, unknown>" template=inner.Iter arguments=(int32, unknown)
"#,
    );
}

#[test]
fn test_defaulted_parameter_fills_inside_import_cycle() {
    let session = TestSession::builder()
        .module(
            "a.ds",
            r#"
import { Iter } from "./b.ds";

export interface Marker {
    marked: boolean;
}

declare function probe(values: Iter<int32>): boolean;
const value = probe(todo("iter"));
"#,
        )
        .module(
            "b.ds",
            r#"
import { Marker } from "./a.ds";

export interface Iter<T, in out R = unknown> {
    next(): T;
    mark(): Marker;
}
"#,
        )
        .build();

    session.assert_dir_checked_many(
        &["a.ds", "b.ds"],
        DirRows::checked(),
        r#"
=== a.ds ===

=== annotated ===
import { Iter } from "./b.ds";

export interface Marker {
    marked: boolean;
}

declare function probe(values: Dynamic<Iter<int32, unknown>>): boolean;
const value: boolean = probe(todo("iter" as string | undefined));

=== checked ===
import { Iter } from "./b.ds";

export interface Marker {
/// @type.symbol symbol=Marker type=Marker
/// @definition.interface symbol=Marker
/// @definition.field symbol=Marker.marked source="marked: boolean" key=marked type=boolean

    marked: boolean;
    /// @type.symbol symbol=Marker.marked source="marked: boolean" type=boolean

}

declare function probe(values: Iter<int32>): boolean;
/// @type.symbol symbol=probe source="declare function probe(values: Iter<int32>): boolean" type=(Dynamic<b.Iter<int32, b.ds.type3>>) => boolean reduced=(Dynamic<b.Iter<int32, unknown>>) => boolean
/// @type.symbol symbol=probe.values source="values: Iter<int32>" type=Dynamic<b.Iter<int32, b.ds.type3>> reduced=Dynamic<b.Iter<int32, unknown>>
/// @resolution.name source=Iter target=b.Iter

const value = probe(todo("iter"));
/// @type.symbol symbol=value source=value type=boolean
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=probe target=probe
/// @resolution.call source="probe(todo(\"iter\"))" parameters=(Dynamic<b.Iter<int32, b.ds.type3>>) arguments=(provided(todo("iter")) as Dynamic<b.Iter<int32, b.ds.type3>>) return=boolean kind=symbol target=probe
/// @resolution.name source=todo target=error.panic.todo
/// @resolution.call source="todo(\"iter\")" parameters=(string | undefined) arguments=(provided("iter") as string | undefined) return=never kind=symbol target=error.panic.todo

/// @generic.instance id="b.Iter<int32, b.ds.type3>" template=b.Iter arguments=(int32, b.ds.type3)

=== b.ds ===

=== annotated ===
import { Marker } from "./a.ds";

export interface Iter<out T, in out R = unknown> {
    next(): T;
    mark(): Marker;
}

=== checked ===
import { Marker } from "./a.ds";

export interface Iter<T, in out R = unknown> {
/// @generic.template symbol=Iter parameters=(out T, in out R = unknown)
/// @type.symbol symbol=Iter type=Iter
/// @definition.interface symbol=Iter template=(out T, in out R = unknown)
/// @definition.where symbol=Iter relation=satisfies left=this right=Iter<T, R>
/// @definition.method symbol=Iter.mark source="mark(): Marker" slot=mark type=(this: this) => a.Marker
/// @definition.method symbol=Iter.next source="next(): T" slot=next type=(this: this) => T
/// @type.symbol symbol=Iter.T source=T type=T
/// @type.symbol symbol=Iter.R source="in out R = unknown" type=R

    next(): T;
    /// @type.symbol symbol=Iter.next source="next(): T" type=(this: this) => T
    /// @resolution.name source=T target=Iter.T

    mark(): Marker;
    /// @type.symbol symbol=Iter.mark source="mark(): Marker" type=(this: this) => a.Marker
    /// @resolution.name source=Marker target=a.Marker

}
"#,
    );
}

#[test]
fn test_generic_struct_pattern_infers_omitted_arguments() {
    let session = TestSession::single(
        r#"
struct Wrap<T> {
    value: T;
}

function unwrap(wrapped: Wrap<float64>): float64 {
    match (wrapped) {
        Wrap { value } => value
    }
}

const built = Wrap { value: 1 };
const out = unwrap(built);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Wrap<out T> {
    value: T;
}

function unwrap(wrapped: Wrap<float64>): float64 {
    match (wrapped) {
        Wrap { value } => value
    }
}

const built: Wrap<float64> = Wrap<float64> { value: 1 };
const out: float64 = unwrap(built);

=== checked ===
struct Wrap<T> {
/// @generic.template symbol=Wrap parameters=(out T)
/// @type.symbol symbol=Wrap type=Wrap
/// @definition.struct symbol=Wrap template=(out T)
/// @definition.field symbol=Wrap.value source="value: T" key=value type=T
/// @type.symbol symbol=Wrap.T source=T type=T

    value: T;
    /// @type.symbol symbol=Wrap.value source="value: T" type=T
    /// @resolution.name source=T target=Wrap.T

}

function unwrap(wrapped: Wrap<float64>): float64 {
/// @type.symbol symbol=unwrap type=(Wrap<float64>) => float64
/// @type.symbol symbol=unwrap.wrapped source="wrapped: Wrap<float64>" type=Wrap<float64>
/// @resolution.name source=Wrap target=Wrap

    match (wrapped) {
    /// @resolution.name source=wrapped target=unwrap.wrapped
    /// @resolution.place source=wrapped placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=wrapped root=unwrap.wrapped

        Wrap { value } => value
        /// @resolution.name source=Wrap target=Wrap
        /// @resolution.pattern source="Wrap { value }" kind=nominal_object target=Wrap instance=Wrap<float64> fields={ Wrap.value }
        /// @generic.instance source="Wrap { value }" id=Wrap<float64>
        /// @type.symbol symbol=unwrap.value source=value type=float64
        /// @resolution.name source=value target=unwrap.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=unwrap.value

    }
}

const built = Wrap { value: 1 };
/// @type.symbol symbol=built source=built type=Wrap<float64>
/// @resolution.pattern source=built kind=binding target=built
/// @resolution.name source=Wrap target=Wrap

const out = unwrap(built);
/// @type.symbol symbol=out source=out type=float64
/// @resolution.pattern source=out kind=binding target=out
/// @resolution.name source=unwrap target=unwrap
/// @resolution.call source=unwrap(built) parameters=(Wrap<float64>) arguments=(provided(built) as Wrap<float64>) return=float64 kind=symbol target=unwrap
/// @resolution.name source=built target=built
/// @resolution.place source=built placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=built root=built

/// @generic.instance id=Wrap<float64> template=Wrap arguments=(float64)
"#,
        "",
    );
}

#[test]
fn test_imported_struct_carries_an_alias_with_an_induced_lifetime() {
    let compiler = TestSession::builder()
        .module(
            "trigger.ds",
            r#"
export struct Attempt {
    module: string;
}

export type Predicate = (attempt: &readonly Attempt) => boolean;

export struct Trigger {
    predicate?: Predicate;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Trigger } from "./trigger.ds";

export struct Scenario {
    trigger: Trigger;
}

declare let scenario: Scenario;

scenario.trigger satisfies Trigger;
"#,
        )
        .build();

    compiler.assert_dir_checked_many(
        &["trigger.ds", "main.ds"],
        DirRows::checked(),
        r#"
=== trigger.ds ===

=== annotated ===
export struct Attempt {
    module: string;
}

export type Predicate<'a> = (attempt: &readonly Attempt) => boolean;

export struct Trigger<'a> {
    predicate?: Predicate<'a>;
}

=== checked ===
export struct Attempt {
/// @type.symbol symbol=Attempt type=Attempt
/// @definition.struct symbol=Attempt
/// @definition.field symbol=Attempt.module source="module: string" key=module type=string

    module: string;
    /// @type.symbol symbol=Attempt.module source="module: string" type=string

}

export type Predicate = (attempt: &readonly Attempt) => boolean;
/// @generic.template symbol=Predicate parameters=('a)
/// @type.symbol symbol=Predicate source="export type Predicate = (attempt: &readonly Attempt) => boolean" type=Function<(&Predicate.'a readonly Attempt,), boolean>
/// @definition.type symbol=Predicate source="export type Predicate = (attempt: &readonly Attempt) => boolean" template=('a) value=Function<(&Predicate.'a readonly Attempt,), boolean>
/// @resolution.name source=Attempt target=Attempt

export struct Trigger {
/// @generic.template symbol=Trigger parameters=('a)
/// @type.symbol symbol=Trigger type=Trigger
/// @definition.struct symbol=Trigger template=('a)
/// @definition.field symbol=Trigger.predicate source="predicate?: Predicate" key=predicate type=Predicate<Trigger.'a>

    predicate?: Predicate;
    /// @type.symbol symbol=Trigger.predicate source="predicate?: Predicate" type=Predicate<Trigger.'a> reduced=Function<(&Trigger.'a readonly Attempt,), boolean>
    /// @resolution.name source=Predicate target=Predicate

}

/// @generic.instance id=Predicate<Trigger.'a> template=Predicate arguments=(Trigger.'a)

=== main.ds ===

=== annotated ===
import { Trigger } from "./trigger.ds";

export struct Scenario<'a> {
    trigger: Trigger<'a>;
}

declare let scenario: Scenario<"static">;

scenario.trigger satisfies Trigger;

=== checked ===
import { Trigger } from "./trigger.ds";

export struct Scenario {
/// @generic.template symbol=Scenario parameters=('a)
/// @type.symbol symbol=Scenario type=Scenario
/// @definition.struct symbol=Scenario template=('a)
/// @definition.field symbol=Scenario.trigger source="trigger: Trigger" key=trigger type=trigger.Trigger<Scenario.'a>

    trigger: Trigger;
    /// @type.symbol symbol=Scenario.trigger source="trigger: Trigger" type=trigger.Trigger<Scenario.'a>
    /// @resolution.name source=Trigger target=trigger.Trigger

}

declare let scenario: Scenario;
/// @type.symbol symbol=scenario source=scenario type=Scenario<"static">
/// @resolution.pattern source=scenario kind=binding target=scenario
/// @resolution.name source=Scenario target=Scenario

scenario.trigger satisfies Trigger;
/// @resolution.name source=scenario target=scenario
/// @resolution.member source=scenario.trigger receiver=Scenario<"static"> type=trigger.Trigger<"static"> kind=field target_receiver=Scenario<"static"> key=trigger target=Scenario.trigger target_type=trigger.Trigger<"static">
/// @resolution.place source=scenario placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=scenario root=scenario
/// @resolution.place source=scenario.trigger placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=scenario.trigger root=scenario keys=[trigger]
/// @resolution.name source=Trigger target=trigger.Trigger

/// @generic.instance id="Scenario<\"static\">" template=Scenario arguments=("static")
/// @generic.instance id=trigger.Trigger<Scenario.'a> template=trigger.Trigger arguments=(Scenario.'a)
"#,
    );
}
