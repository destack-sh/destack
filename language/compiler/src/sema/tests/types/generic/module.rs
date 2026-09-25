use crate::tests::{DirRows, TestSession};

/// Lifetimes write through a generic bound that names a type declared later.
#[test]
fn test_write_lifetimes_through_a_forward_generic_bound() {
    let session = TestSession::single(
        r#"
interface Holder<'a, T: View<'a>> {
    value: T;
}

struct View<'a> {
    user: &'a readonly User;
}

struct User {}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
interface Holder<'a, in out T: View<'a>> {
    value: T;
}

struct View<'a> {
    user: &'a readonly User;
}

struct User {}

=== dir ===
interface Holder<'a, T: View<'a>> {
/// @generic.template symbol=Holder parameters=('a#1, in out T: View<'a#1>, this: Holder<'a#1, T>)
/// @type.symbol symbol=Holder type=Holder
/// @definition.interface symbol=Holder template=('a#1, in out T: View<'a#1>, this: Holder<'a#1, T>)
/// @definition.where symbol=Holder relation=satisfies left=this right=Holder<'a#1, T>
/// @definition.field symbol=Holder.value source="value: T" key=value type=T
/// @type.symbol symbol=Holder.'a source='a type='a#1
/// @type.symbol symbol=Holder.T source="T: View<'a>" type=T
/// @resolution.name source=View target=View
/// @generic.instance id=View<'a#1> template=View arguments=('a#1)
/// @resolution.name source='a target=Holder.'a

    value: T;
    /// @type.symbol symbol=Holder.value source="value: T" type=T
    /// @resolution.name source=T target=Holder.T

}

struct View<'a> {
/// @generic.template symbol=View parameters=('a#2)
/// @type.symbol symbol=View type=View
/// @definition.struct symbol=View template=('a#2)
/// @definition.field symbol=View.user source="user: &'a readonly User" key=user type=&'a#2 readonly User
/// @type.symbol symbol=View.'a source='a type='a#2

    user: &'a readonly User;
    /// @type.symbol symbol=View.user source="user: &'a readonly User" type=&'a#2 readonly User
    /// @resolution.name source='a target=View.'a
    /// @resolution.name source=User target=User

}

struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"
"#,
    );
}

/// Lifetimes write through a declaration cycle that spans modules.
#[test]
fn test_write_lifetimes_through_a_cross_module_declaration_cycle() {
    let compiler = TestSession::builder()
        .module(
            "b.tspp",
            r#"
import { Foo } from "./a.tspp";

export struct Bar<'a> {
    foo: Foo<'a>;
}

export struct Baz<'a> {
    user: &'a readonly User;
}

export struct User {}
"#,
        )
        .module(
            "a.tspp",
            r#"
import { Baz } from "./b.tspp";

export struct Foo<'a> {
    baz: Baz<'a>;
}
"#,
        )
        .build();

    compiler.assert_dir_many(
        &["a.tspp", "b.tspp"],
        DirRows::checked().with_reference_types(),
        r#"
=== a.tspp ===

=== annotated ===
import { Baz } from "./b.tspp";

export struct Foo<'a> {
    baz: Baz<'a>;
}

=== dir ===
import { Baz } from "./b.tspp";

export struct Foo<'a> {
/// @generic.template symbol=Foo parameters=('a)
/// @type.symbol symbol=Foo type=Foo
/// @definition.struct symbol=Foo template=('a)
/// @definition.field symbol=Foo.baz source="baz: Baz<'a>" key=baz type=b.Baz<'a>
/// @type.symbol symbol=Foo.'a source='a type='a

    baz: Baz<'a>;
    /// @type.symbol symbol=Foo.baz source="baz: Baz<'a>" type=b.Baz<'a>
    /// @generic.instance id=b.Baz<'a> template=b.Baz arguments=('a)
    /// @resolution.name source=Baz target=b.Baz
    /// @resolution.name source='a target=Foo.'a

}

=== b.tspp ===

=== annotated ===
import { Foo } from "./a.tspp";

export struct Bar<'a> {
    foo: Foo<'a>;
}

export struct Baz<'a> {
    user: &'a readonly User;
}

export struct User {}

=== dir ===
import { Foo } from "./a.tspp";

export struct Bar<'a> {
/// @generic.template symbol=Bar parameters=('a#1)
/// @type.symbol symbol=Bar type=Bar
/// @definition.struct symbol=Bar template=('a#1)
/// @definition.field symbol=Bar.foo source="foo: Foo<'a>" key=foo type=a.Foo<'a#1>
/// @type.symbol symbol=Bar.'a source='a type='a#1

    foo: Foo<'a>;
    /// @type.symbol symbol=Bar.foo source="foo: Foo<'a>" type=a.Foo<'a#1>
    /// @generic.instance id=a.Foo<'a#1> template=a.Foo arguments=('a#1)
    /// @resolution.name source=Foo target=a.Foo
    /// @resolution.name source='a target=Bar.'a

}

export struct Baz<'a> {
/// @generic.template symbol=Baz parameters=('a#2)
/// @type.symbol symbol=Baz type=Baz
/// @definition.struct symbol=Baz template=('a#2)
/// @definition.field symbol=Baz.user source="user: &'a readonly User" key=user type=&'a#2 readonly User
/// @type.symbol symbol=Baz.'a source='a type='a#2

    user: &'a readonly User;
    /// @type.symbol symbol=Baz.user source="user: &'a readonly User" type=&'a#2 readonly User
    /// @resolution.name source='a target=Baz.'a
    /// @resolution.name source=User target=User

}

export struct User {}
/// @type.symbol symbol=User source="export struct User {}" type=User
/// @definition.struct symbol=User source="export struct User {}"
"#,
    );
}

/// An imported generic type accepts a locally declared type argument.
#[test]
fn test_imported_generic_type_accepts_local_type_argument() {
    let compiler = TestSession::builder()
        .module(
            "lib.tspp",
            r#"
export interface Box<T> {
    value: T;
}
"#,
        )
        .module(
            "main.tspp",
            r#"
import { Box } from "./lib.tspp";

type Wrapped<T> = Box<T>;
"#,
        )
        .build();

    compiler.assert_dir_many(
        &["lib.tspp", "main.tspp"],
        DirRows::checked().with_reference_types(),
        r#"
=== lib.tspp ===

=== annotated ===
export interface Box<in out T> {
    value: T;
}

=== dir ===
export interface Box<T> {
/// @generic.template symbol=Box parameters=(in out T, this: Box<T>)
/// @type.symbol symbol=Box type=Box
/// @definition.interface symbol=Box template=(in out T, this: Box<T>)
/// @definition.where symbol=Box relation=satisfies left=this right=Box<T>
/// @definition.field symbol=Box.value source="value: T" key=value type=T
/// @type.symbol symbol=Box.T source=T type=T

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T
    /// @resolution.name source=T target=Box.T

}

=== main.tspp ===

=== annotated ===
import { Box } from "./lib.tspp";

type Wrapped<T> = Box<T>;

=== dir ===
import { Box } from "./lib.tspp";

type Wrapped<T> = Box<T>;
/// @generic.template symbol=Wrapped parameters=(T)
/// @type.symbol symbol=Wrapped source="type Wrapped<T> = Box<T>" type=lib.Box<T>
/// @generic.instance id=lib.Box<T> template=lib.Box arguments=(T)
/// @definition.type symbol=Wrapped source="type Wrapped<T> = Box<T>" template=(T) value=lib.Box<T>
/// @type.symbol symbol=Wrapped.T source=T type=T
/// @resolution.name source=Box target=lib.Box
/// @resolution.name source=T target=Wrapped.T
"#,
    );
}

/// A generic newtype interface extends another generic interface.
#[test]
fn test_generic_newtype_interface_extends_generic_interface() {
    let compiler = TestSession::builder()
        .module(
            "ops.tspp",
            r#"
export newtype interface PartialEqual<T = this> {
    equal(other: T): boolean;
}

export newtype interface Equal<T = this> extends PartialEqual<T> {}
"#,
        )
        .module(
            "main.tspp",
            r#"
import { Equal } from "./ops.tspp";

type Used = Equal<string>;
"#,
        )
        .build();

    compiler.assert_dir_many(
        &["ops.tspp", "main.tspp"],
        DirRows::checked().with_reference_types(),
        r#"
=== ops.tspp ===

=== annotated ===
export newtype interface PartialEqual<in T = this> {
    equal(other: T): boolean;
}

export newtype interface Equal<in T = this> extends PartialEqual<T> {}

=== dir ===
export newtype interface PartialEqual<T = this> {
/// @generic.template symbol=PartialEqual parameters=(in T#1 = this, this: PartialEqual<T#1>)
/// @type.symbol symbol=PartialEqual type=PartialEqual
/// @generic.instance id=PartialEqual<this> template=PartialEqual arguments=(this)
/// @definition.interface symbol=PartialEqual template=(in T#1 = this, this: PartialEqual<T#1>) nominal=true
/// @definition.where symbol=PartialEqual relation=satisfies left=this right=PartialEqual<T#1>
/// @definition.method symbol=PartialEqual.equal source="equal(other: T): boolean" slot=equal type=(T#1) => boolean
/// @type.symbol symbol=PartialEqual.T source="T = this" type=T#1

    equal(other: T): boolean;
    /// @type.symbol symbol=PartialEqual.equal source="equal(other: T): boolean" type=(T#1) => boolean
    /// @type.symbol symbol=PartialEqual.equal.other source="other: T" type=T#1
    /// @resolution.name source=T target=PartialEqual.T

}

export newtype interface Equal<T = this> extends PartialEqual<T> {}
/// @generic.template symbol=Equal parameters=(in T#2 = this, this: Equal<T#2>)
/// @type.symbol symbol=Equal source="export newtype interface Equal<T = this> extends PartialEqual<T> {}" type=Equal
/// @generic.instance id=Equal<this> template=Equal arguments=(this)
/// @generic.instance id=PartialEqual<T#2> template=PartialEqual arguments=(T#2)
/// @definition.interface symbol=Equal source="export newtype interface Equal<T = this> extends PartialEqual<T> {}" template=(in T#2 = this, this: Equal<T#2>) nominal=true
/// @definition.where symbol=Equal source="export newtype interface Equal<T = this> extends PartialEqual<T> {}" relation=satisfies left=this right=Equal<T#2>
/// @definition.extends symbol=Equal source=PartialEqual<T> target=PartialEqual<T#2>
/// @type.symbol symbol=Equal.T source="T = this" type=T#2
/// @resolution.name source=PartialEqual target=PartialEqual
/// @resolution.name source=T target=Equal.T

=== main.tspp ===

=== annotated ===
import { Equal } from "./ops.tspp";

type Used = Equal<string>;

=== dir ===
import { Equal } from "./ops.tspp";

type Used = Equal<string>;
/// @type.symbol symbol=Used source="type Used = Equal<string>" type=ops.Equal<string>
/// @generic.instance id=ops.Equal<string> template=ops.Equal arguments=(string)
/// @definition.type symbol=Used source="type Used = Equal<string>" value=ops.Equal<string>
/// @resolution.name source=Equal target=ops.Equal
"#,
    );
}

/// An imported generic function instantiates in the calling module.
#[test]
fn test_imported_generic_function_instantiates_in_calling_module() {
    let compiler = TestSession::builder()
        .module(
            "lib.tspp",
            r#"
export function identity<T>(value: T): T {
    return value;
}
"#,
        )
        .module(
            "main.tspp",
            r#"
import { identity } from "./lib.tspp";

const number = identity(1);
const text = identity("x");
"#,
        )
        .build();

    compiler.assert_dir_many(
        &["lib.tspp", "main.tspp"],
        DirRows::checked().with_reference_types(),
        r#"
=== lib.tspp ===

=== annotated ===
export function identity<T>(value: T): T {
    return value;
}

=== dir ===
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

=== main.tspp ===

=== annotated ===
import { identity } from "./lib.tspp";

const number: int64 = identity<int64>(1);
const text: "x" = identity<"x">("x");

=== dir ===
import { identity } from "./lib.tspp";

const number = identity(1);
/// @type.symbol symbol=number source=number type=int64
/// @resolution.pattern source=number kind=binding target=number
/// @type.node source=identity type=(int64) => int64
/// @type.node source=identity(1) type=int64
/// @resolution.name source=identity target=lib.identity
/// @resolution.call source=identity(1) parameters=(int64) arguments=(provided(1) as int64) return=int64 kind=symbol target=lib.identity instance=lib.identity<int64>
/// @generic.instantiation id=lib.identity<int64> template=lib.identity arguments=(int64)
/// @generic.instance id=lib.identity<int64> template=lib.identity arguments=(int64)
/// @type.node source=1 type=1

const text = identity("x");
/// @type.symbol symbol=text source=text type="x"
/// @resolution.pattern source=text kind=binding target=text
/// @type.node source="identity(\"x\")" type="x"
/// @type.node source=identity type=("x") => "x"
/// @resolution.name source=identity target=lib.identity
/// @resolution.call source="identity(\"x\")" parameters=("x") arguments=(provided("x") as "x") return="x" kind=symbol target=lib.identity instance="lib.identity<\"x\">"
/// @generic.instantiation id="lib.identity<\"x\">" template=lib.identity arguments=("x")
/// @generic.instance id="lib.identity<\"x\">" template=lib.identity arguments=("x")
/// @type.node source="\"x\"" type="x"
"#,
    );
}

/// An imported generic function without a result annotation reports a diagnostic.
#[test]
fn test_reject_imported_generic_function_without_result_type() {
    let compiler = TestSession::builder()
        .module(
            "lib.tspp",
            r#"
export function identity<T>(value: T) {
    return value;
}
"#,
        )
        .module(
            "main.tspp",
            r#"
import { identity } from "./lib.tspp";

const text = identity("x");
"#,
        )
        .build();

    compiler.assert_dir_and_diagnostics("lib.tspp", DirRows::checked(), r#"
=== annotated ===
export function identity<T>(value: T) {
    return value;
}

=== dir ===
export function identity<T>(value: T) {
/// @generic.template symbol=identity parameters=(T)
/// @type.symbol symbol=identity type=<T>(T) => <error>
/// @type.symbol symbol=identity.T source=T type=T
/// @type.symbol symbol=identity.value source="value: T" type=T
/// @resolution.name source=T target=identity.T

    return value;
    /// @resolution.name source=value target=identity.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=identity.value

}
"#,
        r#"
/// @diagnostic.error id=missing-result-type message="function declaration needs a written result type"
/// @diagnostic.label line=2 column=17 span="identity" line_source="export function identity<T>(value: T) {"
/// @diagnostic.help message="state the result type on the declaration"
"#,
    );

    compiler.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { identity } from "./lib.tspp";

const text = identity<string>("x");

=== dir ===
import { identity } from "./lib.tspp";

const text = identity("x");
/// @type.symbol symbol=text source=text type=<error>
/// @resolution.pattern source=text kind=binding target=text
/// @type.node source="identity(\"x\")" type=<error>
/// @type.node source=identity type=(string) => <error>
/// @resolution.name source=identity target=lib.identity
/// @resolution.call source="identity(\"x\")" parameters=(string) arguments=(provided("x") as string) return=<error> kind=symbol target=lib.identity instance=lib.identity<string>
/// @generic.instantiation id=lib.identity<string> template=lib.identity arguments=(string)
/// @type.node source="\"x\"" type="x"
"#,
        r#"
/// @diagnostic.error id=missing-result-type message="function declaration needs a written result type"
/// @diagnostic.label line=2 column=17 span="identity" line_source="export function identity<T>(value: T) {"
/// @diagnostic.help message="state the result type on the declaration"
"#,
    );
}

/// A defaulted parameter fills the argument an annotation omits.
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
interface Iter<out T, in out R = unknown> {
    next(): T;
}

declare function probe(values: Iter<int32>): boolean;
const value: boolean = probe(todo("iter" as string | undefined));

=== dir ===
interface Iter<T, in out R = unknown> {
/// @generic.template symbol=Iter parameters=(out T, in out R = unknown, this: Iter<T, R>)
/// @type.symbol symbol=Iter type=Iter
/// @definition.interface symbol=Iter template=(out T, in out R = unknown, this: Iter<T, R>)
/// @definition.where symbol=Iter relation=satisfies left=this right=Iter<T, R>
/// @definition.method symbol=Iter.next source="next(): T" slot=next type=() => T
/// @type.symbol symbol=Iter.T source=T type=T
/// @type.symbol symbol=Iter.R source="in out R = unknown" type=R

    next(): T;
    /// @type.symbol symbol=Iter.next source="next(): T" type=() => T
    /// @resolution.name source=T target=Iter.T

}

declare function probe(values: Iter<int32>): boolean;
/// @type.symbol symbol=probe source="declare function probe(values: Iter<int32>): boolean" type=(Iter<int32, unknown>) => boolean
/// @resolution.name source=Iter target=Iter

const value = probe(todo("iter"));
/// @type.symbol symbol=value source=value type=boolean
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=probe target=probe
/// @resolution.call source="probe(todo(\"iter\"))" parameters=(Iter<int32, unknown>) arguments=(provided(todo("iter")) as Iter<int32, unknown>) return=boolean kind=symbol target=probe
/// @resolution.name source=todo target=todo
/// @resolution.call source="todo(\"iter\")" parameters=(string | undefined) arguments=(provided("iter") as string | undefined) return=never kind=symbol target=todo
"#,
        r#"
"#,
    );
}

/// A defaulted parameter fills through a chain of reexports.
#[test]
fn test_defaulted_parameter_fills_through_reexport_chain() {
    let session = TestSession::builder()
        .module(
            "inner.tspp",
            r#"
export newtype interface Iter<T, in out R = unknown> {
    next(): T;
}
"#,
        )
        .module(
            "lib.tspp",
            r#"
export { Iter } from "./inner.tspp";
"#,
        )
        .module(
            "main.tspp",
            r#"
import { Iter } from "./lib.tspp";

declare function probe(values: Iter<int32>): boolean;
const value = probe(todo("iter"));
"#,
        )
        .build();

    session.assert_dir_many(
        &["inner.tspp", "lib.tspp", "main.tspp"],
        DirRows::checked(),
        r#"
=== inner.tspp ===

=== annotated ===
export newtype interface Iter<out T, in out R = unknown> {
    next(): T;
}

=== dir ===
export newtype interface Iter<T, in out R = unknown> {
/// @generic.template symbol=Iter parameters=(out T, in out R = unknown, this: Iter<T, R>)
/// @type.symbol symbol=Iter type=Iter
/// @definition.interface symbol=Iter template=(out T, in out R = unknown, this: Iter<T, R>) nominal=true
/// @definition.where symbol=Iter relation=satisfies left=this right=Iter<T, R>
/// @definition.method symbol=Iter.next source="next(): T" slot=next type=() => T
/// @type.symbol symbol=Iter.T source=T type=T
/// @type.symbol symbol=Iter.R source="in out R = unknown" type=R

    next(): T;
    /// @type.symbol symbol=Iter.next source="next(): T" type=() => T
    /// @resolution.name source=T target=Iter.T

}

=== lib.tspp ===

=== annotated ===
export { Iter } from "./inner.tspp";

=== dir ===
export { Iter } from "./inner.tspp";

=== main.tspp ===

=== annotated ===
import { Iter } from "./lib.tspp";

declare function probe(values: Iter<int32>): boolean;
const value: boolean = probe(todo("iter" as string | undefined));

=== dir ===
import { Iter } from "./lib.tspp";

declare function probe(values: Iter<int32>): boolean;
/// @type.symbol symbol=probe source="declare function probe(values: Iter<int32>): boolean" type=(inner.Iter<int32, unknown>) => boolean
/// @generic.instance id="inner.Iter<int32, unknown>" template=inner.Iter arguments=(int32, unknown)
/// @resolution.name source=Iter target=inner.Iter

const value = probe(todo("iter"));
/// @type.symbol symbol=value source=value type=boolean
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=probe target=probe
/// @resolution.call source="probe(todo(\"iter\"))" parameters=(inner.Iter<int32, unknown>) arguments=(provided(todo("iter")) as inner.Iter<int32, unknown>) return=boolean kind=symbol target=probe
/// @resolution.name source=todo target=todo
/// @resolution.call source="todo(\"iter\")" parameters=(string | undefined) arguments=(provided("iter") as string | undefined) return=never kind=symbol target=todo
"#,
    );
}

/// A defaulted parameter fills inside an import cycle.
#[test]
fn test_defaulted_parameter_fills_inside_import_cycle() {
    let session = TestSession::builder()
        .module(
            "a.tspp",
            r#"
import { Iter } from "./b.tspp";

export interface Marker {
    marked: boolean;
}

declare function probe(values: Iter<int32>): boolean;
const value = probe(todo("iter"));
"#,
        )
        .module(
            "b.tspp",
            r#"
import { Marker } from "./a.tspp";

export interface Iter<T, in out R = unknown> {
    next(): T;
    mark(): Marker;
}
"#,
        )
        .build();

    session.assert_dir_many(
        &["a.tspp", "b.tspp"],
        DirRows::checked(),
        r#"
=== a.tspp ===

=== annotated ===
import { Iter } from "./b.tspp";

export interface Marker {
    marked: boolean;
}

declare function probe(values: Iter<int32>): boolean;
const value: boolean = probe(todo("iter" as string | undefined));

=== dir ===
import { Iter } from "./b.tspp";

export interface Marker {
/// @generic.template symbol=Marker parameters=(this: Marker)
/// @type.symbol symbol=Marker type=Marker
/// @definition.interface symbol=Marker template=(this: Marker)
/// @definition.where symbol=Marker relation=satisfies left=this right=Marker
/// @definition.field symbol=Marker.marked source="marked: boolean" key=marked type=boolean

    marked: boolean;
    /// @type.symbol symbol=Marker.marked source="marked: boolean" type=boolean

}

declare function probe(values: Iter<int32>): boolean;
/// @type.symbol symbol=probe source="declare function probe(values: Iter<int32>): boolean" type=(b.Iter<int32, unknown>) => boolean
/// @generic.instance id="b.Iter<int32, unknown>" template=b.Iter arguments=(int32, unknown)
/// @resolution.name source=Iter target=b.Iter

const value = probe(todo("iter"));
/// @type.symbol symbol=value source=value type=boolean
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=probe target=probe
/// @resolution.call source="probe(todo(\"iter\"))" parameters=(b.Iter<int32, unknown>) arguments=(provided(todo("iter")) as b.Iter<int32, unknown>) return=boolean kind=symbol target=probe
/// @resolution.name source=todo target=todo
/// @resolution.call source="todo(\"iter\")" parameters=(string | undefined) arguments=(provided("iter") as string | undefined) return=never kind=symbol target=todo

=== b.tspp ===

=== annotated ===
import { Marker } from "./a.tspp";

export interface Iter<out T, in out R = unknown> {
    next(): T;
    mark(): Marker;
}

=== dir ===
import { Marker } from "./a.tspp";

export interface Iter<T, in out R = unknown> {
/// @generic.template symbol=Iter parameters=(out T, in out R = unknown, this: Iter<T, R>)
/// @type.symbol symbol=Iter type=Iter
/// @definition.interface symbol=Iter template=(out T, in out R = unknown, this: Iter<T, R>)
/// @definition.where symbol=Iter relation=satisfies left=this right=Iter<T, R>
/// @definition.method symbol=Iter.mark source="mark(): Marker" slot=mark type=() => a.Marker
/// @definition.method symbol=Iter.next source="next(): T" slot=next type=() => T
/// @type.symbol symbol=Iter.T source=T type=T
/// @type.symbol symbol=Iter.R source="in out R = unknown" type=R

    next(): T;
    /// @type.symbol symbol=Iter.next source="next(): T" type=() => T
    /// @resolution.name source=T target=Iter.T

    mark(): Marker;
    /// @type.symbol symbol=Iter.mark source="mark(): Marker" type=() => a.Marker
    /// @resolution.name source=Marker target=a.Marker

}
"#,
    );
}

/// A generic struct pattern infers the arguments it omits.
#[test]
fn test_generic_struct_pattern_infers_omitted_arguments() {
    let session = TestSession::single(
        r#"
struct Wrap<T> {
    value: T;
}

function unwrap(wrapped: Wrap<int64>): int64 {
    match (wrapped) {
        Wrap { value } => value
    }
}

const built = Wrap { value: 1 };
const out = unwrap(built);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct Wrap<out T> {
    value: T;
}

function unwrap(wrapped: Wrap<int64>): int64 {
    match (wrapped) {
        Wrap { value } => value
    }
}

const built: Wrap<int64> = Wrap<int64> { value: 1 };
const out: int64 = unwrap(built);

=== dir ===
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

function unwrap(wrapped: Wrap<int64>): int64 {
/// @type.symbol symbol=unwrap type=(Wrap<int64>) => int64
/// @type.symbol symbol=unwrap.wrapped source="wrapped: Wrap<int64>" type=Wrap<int64>
/// @resolution.name source=Wrap target=Wrap

    match (wrapped) {
    /// @resolution.coverage exhaustive=true disjoint=true
    /// @resolution.name source=wrapped target=unwrap.wrapped
    /// @resolution.place source=wrapped placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=wrapped root=unwrap.wrapped

        Wrap { value } => value
        /// @resolution.name source=Wrap target=Wrap
        /// @resolution.pattern source="Wrap { value }" kind=nominal_object target=Wrap instance=Wrap<int64> fields={ Wrap.value }
        /// @generic.instantiation id=Wrap<int64> template=Wrap arguments=(int64)
        /// @type.symbol symbol=unwrap.value source=value type=int64
        /// @resolution.name source=value target=unwrap.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=value root=unwrap.value

    }
}

const built = Wrap { value: 1 };
/// @type.symbol symbol=built source=built type=Wrap<int64>
/// @resolution.pattern source=built kind=binding target=built
/// @resolution.name source=Wrap target=Wrap

const out = unwrap(built);
/// @type.symbol symbol=out source=out type=int64
/// @resolution.pattern source=out kind=binding target=out
/// @resolution.name source=unwrap target=unwrap
/// @resolution.call source=unwrap(built) parameters=(Wrap<int64>) arguments=(provided(built) as Wrap<int64>) return=int64 kind=symbol target=unwrap
/// @resolution.name source=built target=built
/// @resolution.place source=built placement="local" lifetime="static" access="immutable"
/// @resolution.access source=built root=built
"#,
        r#"
"#,
    );
}

/// An imported struct holds a field typed by an imported function type alias.
#[test]
fn test_imported_struct_carries_a_function_type_alias() {
    let compiler = TestSession::builder()
        .module(
            "trigger.tspp",
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
            "main.tspp",
            r#"
import { Trigger } from "./trigger.tspp";

export struct Scenario {
    trigger: Trigger;
}

declare let scenario: Scenario;

scenario.trigger satisfies Trigger;
"#,
        )
        .build();

    compiler.assert_dir_many(
        &["trigger.tspp", "main.tspp"],
        DirRows::checked(),
        r#"
=== trigger.tspp ===

=== annotated ===
export struct Attempt {
    module: string;
}

export type Predicate = (attempt: &readonly Attempt) => boolean;

export struct Trigger {
    predicate?: Predicate;
}

=== dir ===
export struct Attempt {
/// @type.symbol symbol=Attempt type=Attempt
/// @definition.struct symbol=Attempt
/// @definition.field symbol=Attempt.module source="module: string" key=module type=string

    module: string;
    /// @type.symbol symbol=Attempt.module source="module: string" type=string

}

export type Predicate = (attempt: &readonly Attempt) => boolean;
/// @type.symbol symbol=Predicate source="export type Predicate = (attempt: &readonly Attempt) => boolean" type=<type_expression.'a>(&type_expression.'a readonly Attempt) => boolean
/// @definition.type symbol=Predicate source="export type Predicate = (attempt: &readonly Attempt) => boolean" value=<type_expression.'a>(&type_expression.'a readonly Attempt) => boolean
/// @generic.template source=type_expression parameters=('a)
/// @type.symbol symbol=Predicate.attempt source="attempt: &readonly Attempt" type=&type_expression.'a readonly Attempt
/// @resolution.name source=Attempt target=Attempt

export struct Trigger {
/// @type.symbol symbol=Trigger type=Trigger
/// @definition.struct symbol=Trigger
/// @definition.field symbol=Trigger.predicate source="predicate?: Predicate" key=predicate type=Predicate

    predicate?: Predicate;
    /// @type.symbol symbol=Trigger.predicate source="predicate?: Predicate" type=Predicate
    /// @resolution.name source=Predicate target=Predicate

}

=== main.tspp ===

=== annotated ===
import { Trigger } from "./trigger.tspp";

export struct Scenario {
    trigger: Trigger;
}

declare let scenario: Scenario;

scenario.trigger satisfies Trigger;

=== dir ===
import { Trigger } from "./trigger.tspp";

export struct Scenario {
/// @type.symbol symbol=Scenario type=Scenario
/// @definition.struct symbol=Scenario
/// @definition.field symbol=Scenario.trigger source="trigger: Trigger" key=trigger type=trigger.Trigger

    trigger: Trigger;
    /// @type.symbol symbol=Scenario.trigger source="trigger: Trigger" type=trigger.Trigger
    /// @resolution.name source=Trigger target=trigger.Trigger

}

declare let scenario: Scenario;
/// @type.symbol symbol=scenario source=scenario type=Scenario
/// @resolution.pattern source=scenario kind=binding target=scenario
/// @resolution.name source=Scenario target=Scenario

scenario.trigger satisfies Trigger;
/// @resolution.name source=scenario target=scenario
/// @resolution.member source=scenario.trigger receiver=Scenario type=trigger.Trigger kind=field target_receiver=Scenario key=trigger target=Scenario.trigger target_type=trigger.Trigger
/// @resolution.place source=scenario placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=scenario root=scenario
/// @resolution.place source=scenario.trigger placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=scenario.trigger root=scenario keys=[trigger]
/// @resolution.name source=Trigger target=trigger.Trigger
"#,
    );
}

/// Alias defaults fill through reexported imports.
#[test]
fn test_fill_alias_defaults_through_reexported_imports() {
    let compiler = TestSession::builder()
        .module(
            "a.tspp",
            r#"
struct Marker {
    id: int32;
}

export type Box<T = Marker> = { value: T };
"#,
        )
        .module(
            "b.tspp",
            r#"
export { Box } from "./a.tspp";
"#,
        )
        .module(
            "c.tspp",
            r#"
import { Box } from "./b.tspp";

declare const boxed: Box;
const value = boxed.value;
"#,
        )
        .build();

    compiler.assert_dir_many(
        &["a.tspp", "b.tspp", "c.tspp"],
        DirRows::checked(),
        r#"
=== a.tspp ===

=== annotated ===
struct Marker {
    id: int32;
}

export type Box<T = Marker> = { value: T };

=== dir ===
struct Marker {
/// @type.symbol symbol=Marker type=Marker
/// @definition.struct symbol=Marker
/// @definition.field symbol=Marker.id source="id: int32" key=id type=int32

    id: int32;
    /// @type.symbol symbol=Marker.id source="id: int32" type=int32

}

export type Box<T = Marker> = { value: T };
/// @generic.template symbol=Box parameters=(T = Marker)
/// @type.symbol symbol=Box source="export type Box<T = Marker> = { value: T }" type={ value: T }
/// @definition.type symbol=Box source="export type Box<T = Marker> = { value: T }" template=(T = Marker) value={ value: T }
/// @type.symbol symbol=Box.T source="T = Marker" type=T
/// @resolution.name source=Marker target=Marker
/// @type.symbol symbol=Box.value source="value: T" type=T
/// @resolution.name source=T target=Box.T

=== b.tspp ===

=== annotated ===
export { Box } from "./a.tspp";

=== dir ===
export { Box } from "./a.tspp";

=== c.tspp ===

=== annotated ===
import { Box } from "./b.tspp";

declare const boxed: Box;
const value: Marker = boxed.value;

=== dir ===
import { Box } from "./b.tspp";

declare const boxed: Box;
/// @type.symbol symbol=boxed source=boxed type=a.Box<a.Marker>
/// @resolution.pattern source=boxed kind=binding target=boxed
/// @generic.instance id=a.Box<a.Marker> template=a.Box arguments=(a.Marker)
/// @resolution.name source=Box target=a.Box

const value = boxed.value;
/// @type.symbol symbol=value source=value type=a.Marker
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=boxed target=boxed
/// @resolution.member source=boxed.value receiver=a.Box<a.Marker> type=a.Marker kind=field target_receiver=a.Box<a.Marker> key=value target_type=a.Marker
/// @resolution.place source=boxed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=boxed root=boxed
/// @resolution.access source=boxed.value root=boxed keys=[value]
"#,
    );
}

/// An unannotated exported function reports a diagnostic at its declaration.
#[test]
fn test_reject_unannotated_exported_function_at_its_declaration() {
    let session = TestSession::builder()
        .module(
            "lib.tspp",
            r#"
export function make() {
    return 1;
}
"#,
        )
        .module(
            "main.tspp",
            r#"
import { make } from "./lib.tspp";

const value = make();
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "lib.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
export function make() {
    return 1;
}

=== dir ===
export function make() {
/// @type.symbol symbol=make type=() => <error>

    return 1;
}
"#,
        r#"
/// @diagnostic.error id=missing-result-type message="function declaration needs a written result type"
/// @diagnostic.label line=2 column=17 span="make" line_source="export function make() {"
/// @diagnostic.help message="state the result type on the declaration"
"#,
    );
}
