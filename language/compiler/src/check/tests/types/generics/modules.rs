use crate::tests::{DirRows, TestSession};

#[test]
fn test_imported_generic_type_reference_classifies_type_arguments() {
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
export interface Box<T> {
/// @generic.template source=declaration parameters=[T]
/// @type.symbol symbol=Box type=Box<T>
/// @definition.field symbol=Box.value source="value: T" key=value type=T
/// @definition.interface symbol=Box template=LocalGenericTemplateId(0)
/// @type.symbol symbol=Box.T source=T type=T

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T
    /// @resolution.name source=T target=Box.T

}

=== main.ds ===
import { Box } from "./lib.ds";

type Wrapped<T> = Box<T>;
/// @generic.template source=declaration parameters=[T]
/// @type.symbol symbol=Wrapped source="type Wrapped<T> = Box<T>" type=lib.Box<T>
/// @definition.type symbol=Wrapped source="type Wrapped<T> = Box<T>" template=LocalGenericTemplateId(0) value=lib.Box<T>
/// @type.symbol symbol=Wrapped.T source=T type=T
/// @generic.instance source=Box<T> id=lib.Box<T>
/// @resolution.name source=Box target=lib.Box
/// @resolution.name source=T target=Wrapped.T

/// @generic.instance id=lib.Box<T> template=lib.Box arguments=[T]
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
export newtype interface PartialEqual<T = this> {
/// @generic.template source=declaration parameters=[T#1 = PartialEqual<T#1>]
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
/// @generic.template source=declaration parameters=[T#2 = Equal<T#2>]
/// @type.symbol symbol=Equal source="export newtype interface Equal<T = this> extends PartialEqual<T> {}" type=Equal<T#2>
/// @definition.interface symbol=Equal source="export newtype interface Equal<T = this> extends PartialEqual<T> {}" template=LocalGenericTemplateId(1) nominal=true
/// @definition.extends symbol=Equal source=PartialEqual<T> target=PartialEqual instance=PartialEqual<T#2>
/// @type.symbol symbol=Equal.T source="T = this" type=T#2
/// @generic.instance source=PartialEqual<T> id=PartialEqual<T#2>
/// @resolution.name source=PartialEqual target=PartialEqual
/// @resolution.name source=T target=Equal.T

/// @generic.instance id=PartialEqual<T#2> template=PartialEqual arguments=[T#2]

=== main.ds ===
import { Equal } from "./ops.ds";

type Used = Equal<string>;
/// @type.symbol symbol=Used source="type Used = Equal<string>" type=ops.Equal<string>
/// @definition.type symbol=Used source="type Used = Equal<string>" value=ops.Equal<string>
/// @generic.instance source=Equal<string> id=ops.Equal<string>
/// @resolution.name source=Equal target=ops.Equal

/// @generic.instance id=ops.Equal<string> template=ops.Equal arguments=[string]
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
export function identity<T>(value: T): T {
/// @generic.template source=declaration parameters=[T]
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
import { identity } from "./lib.ds";

const number = identity(1);
/// @type.symbol symbol=number source=number type=1
/// @generic.instance source=identity(1) id=lib.identity<1>
/// @type.node source=identity type=<lib.identity.T>(lib.identity.T) => lib.identity.T
/// @type.node source=identity(1) type=1
/// @resolution.name source=identity target=lib.identity
/// @resolution.call source=identity(1) parameters=(1) return=1 kind=symbol target=lib.identity instance=lib.identity<1>
/// @type.node source=1 type=1

const text = identity("x");
/// @type.symbol symbol=text source=text type="x"
/// @generic.instance source="identity(\"x\")" id="lib.identity<\"x\">"
/// @type.node source="identity(\"x\")" type="x"
/// @type.node source=identity type=<lib.identity.T>(lib.identity.T) => lib.identity.T
/// @resolution.name source=identity target=lib.identity
/// @resolution.call source="identity(\"x\")" parameters=("x") return="x" kind=symbol target=lib.identity instance="lib.identity<\"x\">"
/// @type.node source="\"x\"" type="x"

/// @generic.instance id="lib.identity<\"x\">" template=lib.identity arguments=["x"]
/// @generic.instance id=lib.identity<1> template=lib.identity arguments=[1]
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
export function identity<T>(value: T) {
/// @generic.template source=declaration parameters=[T]
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=identity.T source=T type=T
/// @type.symbol symbol=value source="value: T" type=T
/// @resolution.name source=T target=identity.T

    return value;
    /// @type.node source=value type=T
    /// @resolution.name source=value target=value

}

=== main.ds ===
import { identity } from "./lib.ds";

const text = identity("x");
/// @type.symbol symbol=text source=text type="x"
/// @generic.instance source="identity(\"x\")" id="lib.identity<\"x\">"
/// @type.node source="identity(\"x\")" type="x"
/// @type.node source=identity type=<lib.identity.T>(lib.identity.T) => lib.identity.T
/// @resolution.name source=identity target=lib.identity
/// @resolution.call source="identity(\"x\")" parameters=("x") return="x" kind=symbol target=lib.identity instance="lib.identity<\"x\">"
/// @type.node source="\"x\"" type="x"

/// @generic.instance id="lib.identity<\"x\">" template=lib.identity arguments=["x"]
"#);
}
