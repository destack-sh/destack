use crate::tests::{DirRows, TestSession};

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
export interface Box<T> {
    value: T;
}

=== checked ===
export interface Box<T> {
/// @generic.template source=declaration parameters=(T)
/// @type.symbol symbol=Box type=Box<T>
/// @definition.interface symbol=Box template=LocalGenericTemplateId(0)
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
/// @generic.template source=declaration parameters=(T)
/// @type.symbol symbol=Wrapped source="type Wrapped<T> = Box<T>" type=lib.Box<T>
/// @definition.type symbol=Wrapped source="type Wrapped<T> = Box<T>" template=(T) value=lib.Box<T>
/// @type.symbol symbol=Wrapped.T source=T type=T
/// @generic.instance source=Box<T> id=lib.Box<T>
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
export newtype interface PartialEqual<T = this> {
    equal(other: T): boolean;
}

export newtype interface Equal<T = this> extends PartialEqual<T> {}

=== checked ===
export newtype interface PartialEqual<T = this> {
/// @generic.template source=declaration parameters=(T#1 = PartialEqual<T#1>)
/// @type.symbol symbol=PartialEqual type=PartialEqual<T#1>
/// @definition.interface symbol=PartialEqual template=LocalGenericTemplateId(0) nominal=true
/// @definition.method symbol=PartialEqual.equal source="equal(other: T): boolean" slot=equal type=(this: PartialEqual<T#1>, T#1) => boolean
/// @type.symbol symbol=PartialEqual.T source="T = this" type=T#1

    equal(other: T): boolean;
    /// @type.symbol symbol=PartialEqual.equal source="equal(other: T): boolean" type=(this: PartialEqual<T#1>, T#1) => boolean
    /// @type.symbol symbol=other source="other: T" type=T#1
    /// @resolution.name source=T target=PartialEqual.T

}

export newtype interface Equal<T = this> extends PartialEqual<T> {}
/// @generic.template source=declaration parameters=(T#2 = Equal<T#2>)
/// @type.symbol symbol=Equal source="export newtype interface Equal<T = this> extends PartialEqual<T> {}" type=Equal<T#2>
/// @definition.interface symbol=Equal source="export newtype interface Equal<T = this> extends PartialEqual<T> {}" template=LocalGenericTemplateId(1) nominal=true
/// @definition.extends symbol=Equal source=PartialEqual<T> target=PartialEqual instance=PartialEqual<T#2>
/// @type.symbol symbol=Equal.T source="T = this" type=T#2
/// @generic.instance source=PartialEqual<T> id=PartialEqual<T#2>
/// @resolution.name source=PartialEqual target=PartialEqual
/// @resolution.name source=T target=Equal.T

/// @generic.instance id=PartialEqual<T#2> template=PartialEqual arguments=(T#2)

=== main.ds ===

=== annotated ===
import { Equal } from "./ops.ds";

type Used = Equal<string>;

=== checked ===
import { Equal } from "./ops.ds";

type Used = Equal<string>;
/// @type.symbol symbol=Used source="type Used = Equal<string>" type=ops.Equal<string>
/// @definition.type symbol=Used source="type Used = Equal<string>" value=ops.Equal<string>
/// @generic.instance source=Equal<string> id=ops.Equal<string>
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
/// @generic.template source=declaration parameters=(T)
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=identity.T source=T type=T
/// @type.symbol symbol=value source="value: T" type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

    return value;
    /// @type.node source=value type=T
    /// @resolution.name source=value target=value

}

=== main.ds ===

=== annotated ===
import { identity } from "./lib.ds";

const number: float64 = identity<float64>(1);
const text: string = identity<string>("x");

=== checked ===
import { identity } from "./lib.ds";

const number = identity(1);
/// @type.symbol symbol=number source=number type=float64
/// @generic.instance source=identity(1) id=lib.identity<float64>
/// @type.node source=identity type=(float64) => float64
/// @type.node source=identity(1) type=float64
/// @resolution.name source=identity target=lib.identity
/// @resolution.call source=identity(1) parameters=(float64) return=float64 kind=symbol target=lib.identity instance=lib.identity<float64>
/// @type.node source=1 type=float64

const text = identity("x");
/// @type.symbol symbol=text source=text type=string
/// @generic.instance source="identity(\"x\")" id=lib.identity<string>
/// @type.node source="identity(\"x\")" type=string
/// @type.node source=identity type=(string) => string
/// @resolution.name source=identity target=lib.identity
/// @resolution.call source="identity(\"x\")" parameters=(string) return=string kind=symbol target=lib.identity instance=lib.identity<string>
/// @type.node source="\"x\"" type=string

/// @generic.instance id=lib.identity<float64> template=lib.identity arguments=(float64)
/// @generic.instance id=lib.identity<string> template=lib.identity arguments=(string)
"#);
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
/// @generic.template source=declaration parameters=(T)
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=identity.T source=T type=T
/// @type.symbol symbol=value source="value: T" type=T
/// @resolution.name source=T target=identity.T

    return value;
    /// @type.node source=value type=T
    /// @resolution.name source=value target=value

}

=== main.ds ===

=== annotated ===
import { identity } from "./lib.ds";

const text: string = identity<string>("x");

=== checked ===
import { identity } from "./lib.ds";

const text = identity("x");
/// @type.symbol symbol=text source=text type=string
/// @generic.instance source="identity(\"x\")" id=lib.identity<string>
/// @type.node source="identity(\"x\")" type=string
/// @type.node source=identity type=(string) => string
/// @resolution.name source=identity target=lib.identity
/// @resolution.call source="identity(\"x\")" parameters=(string) return=string kind=symbol target=lib.identity instance=lib.identity<string>
/// @type.node source="\"x\"" type=string

/// @generic.instance id=lib.identity<string> template=lib.identity arguments=(string)
"#);
}

#[test]
fn test_defaulted_parameter_fills_omitted_annotation_argument() {
    let session = TestSession::single(
        r#"
interface Iter<T, R = unknown> {
    next(): T;
}

declare function probe(values: Iter<int32>): boolean;
const value = probe(todo("iter"));
"#,
    );

    session.assert_dir_checked_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
interface Iter<T, R = unknown> {
    next(): T;
}

declare function probe<T0: Iter<int32, unknown>>(values: T0): boolean;
const value: boolean = probe<never>(todo("iter" as string | undefined));

=== checked ===
interface Iter<T, R = unknown> {
/// @generic.template symbol=Iter parameters=(T, R = unknown)
/// @type.symbol symbol=Iter type=Iter
/// @definition.interface symbol=Iter template=(T, R = unknown)
/// @definition.where symbol=Iter relation=satisfies left=this right=Iter<T, R>
/// @definition.method symbol=Iter.next source="next(): T" slot=next type=(this: Iter<T, R>) => T
/// @type.symbol symbol=Iter.T source=T type=T
/// @type.symbol symbol=Iter.R source="R = unknown" type=R

    next(): T;
    /// @type.symbol symbol=Iter.next source="next(): T" type=(this: Iter<T, R>) => T
    /// @resolution.name source=T target=Iter.T

}

declare function probe(values: Iter<int32>): boolean;
/// @generic.template symbol=probe parameters=(T0: Iter<int32, unknown>)
/// @type.symbol symbol=probe source="declare function probe(values: Iter<int32>): boolean" type=<probe.T0: Iter<int32, unknown>>(probe.T0) => boolean
/// @type.symbol symbol=probe.values source="values: Iter<int32>" type=probe.T0
/// @resolution.name source=Iter target=Iter

const value = probe(todo("iter"));
/// @type.symbol symbol=value source=value type=boolean
/// @resolution.name source=probe target=probe
/// @resolution.call source="probe(todo(\"iter\"))" parameters=(never) arguments=(provided(todo("iter")) as never) return=boolean kind=symbol target=probe instance=probe<never>
/// @generic.instance source="probe(todo(\"iter\"))" id=probe<never>
/// @resolution.name source=todo target=error.panic.todo
/// @resolution.call source="todo(\"iter\")" parameters=(string | undefined) arguments=(provided("iter") as string | undefined) return=never kind=symbol target=error.panic.todo

/// @generic.instance id="Iter<T, R>" template=Iter arguments=(T, R)
/// @generic.instance id=probe<never> template=probe arguments=(never)
"#, "");
}

#[test]
fn test_defaulted_parameter_fills_through_reexport_chain() {
    let session = TestSession::builder()
        .module(
            "inner.ds",
            r#"
export newtype interface Iter<T, R = unknown> {
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

    session.assert_dir_checked_many(&["inner.ds", "lib.ds", "main.ds"], DirRows::checked(), r#"
=== inner.ds ===

=== annotated ===
export newtype interface Iter<T, R = unknown> {
    next(): T;
}

=== checked ===
export newtype interface Iter<T, R = unknown> {
/// @generic.template symbol=Iter parameters=(T, R = unknown)
/// @type.symbol symbol=Iter type=Iter
/// @definition.interface symbol=Iter template=(T, R = unknown) nominal=true
/// @definition.where symbol=Iter relation=satisfies left=this right=Iter<T, R>
/// @definition.method symbol=Iter.next source="next(): T" slot=next type=(this: Iter<T, R>) => T
/// @type.symbol symbol=Iter.T source=T type=T
/// @type.symbol symbol=Iter.R source="R = unknown" type=R

    next(): T;
    /// @type.symbol symbol=Iter.next source="next(): T" type=(this: Iter<T, R>) => T
    /// @resolution.name source=T target=Iter.T

}

/// @generic.instance id="Iter<T, R>" template=Iter arguments=(T, R)

=== lib.ds ===

=== annotated ===
export { Iter } from "./inner.ds";

=== checked ===
export { Iter } from "./inner.ds";

=== main.ds ===

=== annotated ===
import { Iter } from "./lib.ds";

declare function probe<T0: Iter<int32, unknown>>(values: T0): boolean;
const value: boolean = probe<never>(todo("iter" as string | undefined));

=== checked ===
import { Iter } from "./lib.ds";

declare function probe(values: Iter<int32>): boolean;
/// @generic.template symbol=probe parameters=(T0: inner.Iter<int32, unknown>)
/// @type.symbol symbol=probe source="declare function probe(values: Iter<int32>): boolean" type=<probe.T0: inner.Iter<int32, unknown>>(probe.T0) => boolean
/// @type.symbol symbol=probe.values source="values: Iter<int32>" type=probe.T0
/// @resolution.name source=Iter target=inner.Iter

const value = probe(todo("iter"));
/// @type.symbol symbol=value source=value type=boolean
/// @resolution.name source=probe target=probe
/// @resolution.call source="probe(todo(\"iter\"))" parameters=(never) arguments=(provided(todo("iter")) as never) return=boolean kind=symbol target=probe instance=probe<never>
/// @generic.instance source="probe(todo(\"iter\"))" id=probe<never>
/// @resolution.name source=todo target=error.panic.todo
/// @resolution.call source="todo(\"iter\")" parameters=(string | undefined) arguments=(provided("iter") as string | undefined) return=never kind=symbol target=error.panic.todo

/// @generic.instance id=probe<never> template=probe arguments=(never)
"#);
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

export interface Iter<T, R = unknown> {
    next(): T;
    mark(): Marker;
}
"#,
        )
        .build();

    session.assert_dir_checked_many(&["a.ds", "b.ds"], DirRows::checked(), r#"
=== a.ds ===

=== annotated ===
import { Iter } from "./b.ds";

export interface Marker {
    marked: boolean;
}

declare function probe<T0: Iter<int32, unknown>>(values: T0): boolean;
const value: boolean = probe<never>(todo("iter" as string | undefined));

=== checked ===
import { Iter } from "./b.ds";

export interface Marker {
/// @generic.template symbol=Marker parameters=()
/// @type.symbol symbol=Marker type=Marker
/// @definition.interface symbol=Marker template=()
/// @definition.field symbol=Marker.marked source="marked: boolean" key=marked type=boolean

    marked: boolean;
    /// @type.symbol symbol=Marker.marked source="marked: boolean" type=boolean

}

declare function probe(values: Iter<int32>): boolean;
/// @generic.template symbol=probe parameters=(T0: b.Iter<int32, b.ds.type3>)
/// @type.symbol symbol=probe source="declare function probe(values: Iter<int32>): boolean" type=<probe.T0: b.Iter<int32, b.ds.type3>>(probe.T0) => boolean
/// @type.symbol symbol=probe.values source="values: Iter<int32>" type=probe.T0
/// @resolution.name source=Iter target=b.Iter

const value = probe(todo("iter"));
/// @type.symbol symbol=value source=value type=boolean
/// @resolution.name source=probe target=probe
/// @resolution.call source="probe(todo(\"iter\"))" parameters=(never) arguments=(provided(todo("iter")) as never) return=boolean kind=symbol target=probe instance=probe<never>
/// @generic.instance source="probe(todo(\"iter\"))" id=probe<never>
/// @resolution.name source=todo target=error.panic.todo
/// @resolution.call source="todo(\"iter\")" parameters=(string | undefined) arguments=(provided("iter") as string | undefined) return=never kind=symbol target=error.panic.todo

/// @generic.instance id=probe<never> template=probe arguments=(never)

=== b.ds ===

=== annotated ===
import { Marker } from "./a.ds";

export interface Iter<T, R = unknown> {
    next(): T;
    mark(): Marker;
}

=== checked ===
import { Marker } from "./a.ds";

export interface Iter<T, R = unknown> {
/// @generic.template symbol=Iter parameters=(T, R = unknown)
/// @type.symbol symbol=Iter type=Iter
/// @definition.interface symbol=Iter template=(T, R = unknown)
/// @definition.where symbol=Iter relation=satisfies left=this right=Iter<T, R>
/// @definition.method symbol=Iter.mark source="mark(): Marker" slot=mark type=(this: Iter<T, R>) => a.Marker
/// @definition.method symbol=Iter.next source="next(): T" slot=next type=(this: Iter<T, R>) => T
/// @type.symbol symbol=Iter.T source=T type=T
/// @type.symbol symbol=Iter.R source="R = unknown" type=R

    next(): T;
    /// @type.symbol symbol=Iter.next source="next(): T" type=(this: Iter<T, R>) => T
    /// @resolution.name source=T target=Iter.T

    mark(): Marker;
    /// @type.symbol symbol=Iter.mark source="mark(): Marker" type=(this: Iter<T, R>) => a.Marker
    /// @resolution.name source=Marker target=a.Marker

}

/// @generic.instance id="Iter<T, R>" template=Iter arguments=(T, R)
"#);
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

    session.assert_dir_checked_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
struct Wrap<T> {
    value: T;
}

function unwrap(wrapped: Wrap<float64>): float64 {
    match (wrapped) {
        Wrap { value } => value
    }
}

const built: Wrap<float64> = Wrap { value: 1 };
const out: float64 = unwrap(built);

=== checked ===
struct Wrap<T> {
/// @generic.template symbol=Wrap parameters=(T)
/// @type.symbol symbol=Wrap type=Wrap
/// @definition.struct symbol=Wrap template=(T)
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

        Wrap { value } => value
        /// @resolution.name source=Wrap target=Wrap
        /// @resolution.pattern source="Wrap { value }" kind=nominal_object target=Wrap instance=Wrap<float64> fields={ Wrap.value }
        /// @generic.instance source="Wrap { value }" id=Wrap<float64>
        /// @type.symbol symbol=unwrap.value source=value type=float64
        /// @resolution.name source=value target=unwrap.value

    }
}

const built = Wrap { value: 1 };
/// @type.symbol symbol=built source=built type=Wrap<float64>
/// @resolution.name source=Wrap target=Wrap

const out = unwrap(built);
/// @type.symbol symbol=out source=out type=float64
/// @resolution.name source=unwrap target=unwrap
/// @resolution.call source=unwrap(built) parameters=(Wrap<float64>) arguments=(provided(built) as Wrap<float64>) return=float64 kind=symbol target=unwrap
/// @resolution.name source=built target=built

/// @generic.instance id=Wrap<float64> template=Wrap arguments=(float64)
"#, "");
}
