use crate::tests::{DirRows, TestSession};

#[test]
fn test_imported_generic_function_instantiates_in_calling_module() {
    let compiler = TestSession::new()
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
/// @generic.slot symbol=identity.T index=0 kind=type
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=value type=T

    return value;
    /// @resolution.name source=value target=value
    /// @type.node source=value type=T

}

=== main.ds ===
import { identity } from "./lib.ds";

const number = identity(1);
/// @type.symbol symbol=number type=1
/// @resolution.name source=identity target=lib.identity
/// @resolution.call source=identity(1) parameters=(1) return=1 kind=symbol target=lib.identity instance=lib.identity<1>
/// @generic.application source=identity(1) id=lib.identity<1>
/// @type.node source=identity type=<lib.identity.T>(lib.identity.T) => lib.identity.T
/// @type.node source=identity(1) type=1
/// @type.node source=1 type=1

const text = identity("x");
/// @type.symbol symbol=text type="x"
/// @resolution.name source=identity target=lib.identity
/// @resolution.call source="identity(\"x\")" parameters=("x") return="x" kind=symbol target=lib.identity instance="lib.identity<\"x\">"
/// @generic.application source="identity(\"x\")" id="lib.identity<\"x\">"
/// @type.node source="identity(\"x\")" type="x"
/// @type.node source=identity type=<lib.identity.T>(lib.identity.T) => lib.identity.T
/// @type.node source="\"x\"" type="x"
/// @generic.instance id="lib.identity<\"x\">" symbol=lib.identity arguments=["x"]
/// @generic.instance id=lib.identity<1> symbol=lib.identity arguments=[1]
"#);
}

#[test]
fn test_imported_generic_function_uses_exported_body_inference() {
    let compiler = TestSession::new()
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
/// @generic.slot symbol=identity.T index=0 kind=type
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=value type=T

    return value;
    /// @resolution.name source=value target=value
    /// @type.node source=value type=T

}

=== main.ds ===
import { identity } from "./lib.ds";

const text = identity("x");
/// @type.symbol symbol=text type="x"
/// @resolution.name source=identity target=lib.identity
/// @resolution.call source="identity(\"x\")" parameters=("x") return="x" kind=symbol target=lib.identity instance="lib.identity<\"x\">"
/// @generic.application source="identity(\"x\")" id="lib.identity<\"x\">"
/// @type.node source="identity(\"x\")" type="x"
/// @type.node source=identity type=<lib.identity.T>(lib.identity.T) => lib.identity.T
/// @type.node source="\"x\"" type="x"
/// @generic.instance id="lib.identity<\"x\">" symbol=lib.identity arguments=["x"]
"#);
}
