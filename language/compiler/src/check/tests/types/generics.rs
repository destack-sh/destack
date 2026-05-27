use crate::tests::{DirRows, TestSession};

#[test]
fn test_check_records_generic_call_instances() {
    let session = TestSession::single(
        r#"
function identity<T>(value: T): T {
    return value;
}

const number = identity(1);
const text = identity("x");
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
function identity<T>(value: T): T {
/// @generic.slot symbol=identity.T index=0 kind=type
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=value type=T

    return value;
    /// @resolution.name source=value target=value
    /// @type.node source=value type=T

}

const number = identity(1);
/// @type.symbol symbol=number type=1
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity(1) parameters=[1] return=1 kind=symbol target=identity instance=identity<1>
/// @generic.application source=identity(1) id=identity<1>
/// @type.node source=identity type=<T>(T) => T
/// @type.node source=identity(1) type=1
/// @type.node source=1 type=1

const text = identity("x");
/// @type.symbol symbol=text type="x"
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity(\"x\")" parameters=["x"] return="x" kind=symbol target=identity instance="identity<\"x\">"
/// @generic.application source="identity(\"x\")" id="identity<\"x\">"
/// @type.node source="identity(\"x\")" type="x"
/// @type.node source=identity type=<T>(T) => T
/// @type.node source="\"x\"" type="x"
/// @generic.instance id="identity<\"x\">" symbol=identity arguments=["x"]
/// @generic.instance id=identity<1> symbol=identity arguments=[1]
"#);
}

#[test]
fn test_check_records_distinct_literal_generic_instances() {
    let session = TestSession::single(
        r#"
function identity<T>(value: T): T {
    return value;
}

const first = identity(1);
const second = identity(2);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
function identity<T>(value: T): T {
/// @generic.slot symbol=identity.T index=0 kind=type
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=value type=T

    return value;
    /// @resolution.name source=value target=value
    /// @type.node source=value type=T

}

const first = identity(1);
/// @type.symbol symbol=first type=1
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity(1) parameters=[1] return=1 kind=symbol target=identity instance=identity<1>
/// @generic.application source=identity(1) id=identity<1>
/// @type.node source=identity type=<T>(T) => T
/// @type.node source=identity(1) type=1
/// @type.node source=1 type=1

const second = identity(2);
/// @type.symbol symbol=second type=2
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity(2) parameters=[2] return=2 kind=symbol target=identity instance=identity<2>
/// @generic.application source=identity(2) id=identity<2>
/// @type.node source=identity type=<T>(T) => T
/// @type.node source=identity(2) type=2
/// @type.node source=2 type=2
/// @generic.instance id=identity<1> symbol=identity arguments=[1]
/// @generic.instance id=identity<2> symbol=identity arguments=[2]
"#);
}

#[test]
fn test_check_records_explicit_generic_call_arguments() {
    let session = TestSession::single(
        r#"
function identity<T>(value: T): T {
    return value;
}

const text = identity<string>("x");
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
function identity<T>(value: T): T {
/// @generic.slot symbol=identity.T index=0 kind=type
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=value type=T

    return value;
    /// @resolution.name source=value target=value
    /// @type.node source=value type=T

}

const text = identity<string>("x");
/// @type.symbol symbol=text type=string
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity<string>(\"x\")" parameters=[string] return=string kind=symbol target=identity instance=identity<string>
/// @generic.application source="identity<string>(\"x\")" id=identity<string>
/// @type.node source="identity<string>(\"x\")" type=string
/// @type.node source=identity type=<T>(T) => T
/// @type.node source="\"x\"" type=string
/// @generic.instance id=identity<string> symbol=identity arguments=[string]
"#);
}

#[test]
fn test_check_records_explicit_generic_function_references() {
    let session = TestSession::single(
        r#"
function identity<T>(value: T): T {
    return value;
}

const asInt = identity<int32>;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
function identity<T>(value: T): T {
/// @generic.slot symbol=identity.T index=0 kind=type
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=value type=T

    return value;
    /// @resolution.name source=value target=value
    /// @type.node source=value type=T

}

const asInt = identity<int32>;
/// @type.symbol symbol=asInt type=(int32) => int32
/// @resolution.name source=identity target=identity
/// @generic.application source=identity<int32> id=identity<int32>
/// @type.node source=identity type=<T>(T) => T
/// @type.node source=identity<int32> type=(int32) => int32
/// @generic.instance id=identity<int32> symbol=identity arguments=[int32]
"#,
    );
}

#[test]
fn test_check_records_defaulted_generic_arguments() {
    let session = TestSession::single(
        r#"
declare function pair<T, U = T>(left: T, right?: U): (T, U);

const defaulted = pair(1);
const overridden = pair(1, "x");
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
declare function pair<T, U = T>(left: T, right?: U): (T, U);
/// @generic.slot symbol=pair.T index=0 kind=type
/// @generic.slot symbol=pair.U index=1 kind=type default=T
/// @type.symbol symbol=pair type=<T, U = T>(T, U?) => (T, U)
/// @type.symbol symbol=left type=T
/// @type.symbol symbol=right type=U

const defaulted = pair(1);
/// @type.symbol symbol=defaulted type=(1, 1)
/// @resolution.name source=pair target=pair
/// @resolution.call source=pair(1) parameters=[1, 1] return=(1, 1) kind=symbol target=pair instance="pair<1, 1>"
/// @generic.application source=pair(1) id="pair<1, 1>"
/// @type.node source=pair type=<T, U = T>(T, U?) => (T, U)
/// @type.node source=pair(1) type=(1, 1)
/// @type.node source=1 type=1

const overridden = pair(1, "x");
/// @type.symbol symbol=overridden type=(1, "x")
/// @resolution.name source=pair target=pair
/// @resolution.call source="pair(1, \"x\")" parameters=[1, "x"] return=(1, "x") kind=symbol target=pair instance="pair<1, \"x\">"
/// @generic.application source="pair(1, \"x\")" id="pair<1, \"x\">"
/// @type.node source="pair(1, \"x\")" type=(1, "x")
/// @type.node source=pair type=<T, U = T>(T, U?) => (T, U)
/// @type.node source=1 type=1
/// @type.node source="\"x\"" type="x"
/// @generic.instance id="pair<1, 1>" symbol=pair arguments=[1, 1]
/// @generic.instance id="pair<1, \"x\">" symbol=pair arguments=[1, "x"]
"#);
}

#[test]
fn test_check_records_static_value_generic_arguments() {
    let session = TestSession::single(
        r#"
function take<comptime N: uint>(value: [uint8; N]): [uint8; N] {
    return value;
}

const bytes = take<4>([1, 2, 3, 4]);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
function take<comptime N: uint>(value: [uint8; N]): [uint8; N] {
/// @generic.slot symbol=take.N index=0 kind=static constraint=uint
/// @type.symbol symbol=value type=[uint8; N]
/// @resolution.name source=N target=N
/// @resolution.name source=N target=N

    return value;
    /// @resolution.name source=value target=value
    /// @type.node source=value type=[uint8; N]

}

const bytes = take<4>([1, 2, 3, 4]);
/// @type.symbol symbol=bytes type=[uint8; 4]
/// @resolution.name source=take target=take
/// @resolution.call source="take<4>([1, 2, 3, 4])" parameters=[[uint8; 4]] return=[uint8; 4] kind=symbol target=take instance=take<4>
/// @generic.application source="take<4>([1, 2, 3, 4])" id=take<4>
/// @type.node source="take<4>([1, 2, 3, 4])" type=[uint8; 4]
/// @type.node source=[1, 2, 3, 4] type=[uint8; 4]
/// @type.node source=1 type=float64
/// @type.node source=2 type=float64
/// @type.node source=3 type=float64
/// @type.node source=4 type=float64
/// @generic.instance id=take<4> symbol=take arguments=[4]
"#);
}

#[test]
fn test_check_records_static_value_generic_defaults() {
    let session = TestSession::single(
        r#"
function choose<comptime Flag: boolean = true>(value: int32): int32 {
    return value;
}

const value = choose(1);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
function choose<comptime Flag: boolean = true>(value: int32): int32 {
/// @generic.slot symbol=choose.Flag index=0 kind=static constraint=boolean default=true
/// @type.node source=true type=true
/// @type.symbol symbol=value#1 type=int32

    return value;
    /// @resolution.name source=value target=value#1
    /// @type.node source=value type=int32

}

const value = choose(1);
/// @type.symbol symbol=value#2 type=int32
/// @resolution.name source=choose target=choose
/// @resolution.call source=choose(1) parameters=[int32] return=int32 kind=symbol target=choose instance=choose<true>
/// @generic.application source=choose(1) id=choose<true>
/// @type.node source=choose(1) type=int32
/// @type.node source=1 type=int32
/// @generic.instance id=choose<true> symbol=choose arguments=[true]
"#);
}

#[test]
fn test_check_records_literal_static_value_generic_arguments() {
    let session = TestSession::single(
        r#"
type Tagged<comptime Tag: string> = { tag: Tag };
type Flagged<comptime Config: { name: string; enabled: boolean }> = Config;

declare const tagged: Tagged<"alpha">;
declare const flagged: Flagged<{ name: "search"; enabled: true }>;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
type Tagged<comptime Tag: string> = { tag: Tag };
/// @generic.slot symbol=Tagged.Tag index=0 kind=static constraint=string
/// @type.symbol symbol=Tagged type={ tag: Tag }

type Flagged<comptime Config: { name: string; enabled: boolean }> = Config;
/// @generic.slot symbol=Flagged.Config index=0 kind=static constraint={ name: string; enabled: boolean }
/// @type.symbol symbol=Flagged type=Config

declare const tagged: Tagged<"alpha">;
/// @type.symbol symbol=tagged type={ tag: "alpha" }
/// @resolution.name source=Tagged target=Tagged
/// @generic.application source="Tagged<\"alpha\">" id="Tagged<\"alpha\">"

declare const flagged: Flagged<{ name: "search"; enabled: true }>;
/// @type.symbol symbol=flagged type={ name: "search"; enabled: true }
/// @resolution.name source=Flagged target=Flagged
/// @generic.application source="Flagged<{ name: \"search\"; enabled: true }>" id="Flagged<{ name: \"search\"; enabled: true }>"
/// @generic.instance id="Flagged<{ name: \"search\"; enabled: true }>" symbol=Flagged arguments=[{ name: "search"; enabled: true }]
/// @generic.instance id="Tagged<\"alpha\">" symbol=Tagged arguments=["alpha"]
"#,
    );
}

#[test]
fn test_check_reports_explicit_generic_argument_mismatches() {
    let session = TestSession::single(
        r#"
function identity<T>(value: T): T {
    return value;
}

identity<int32>("x");
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
function identity<T>(value: T): T {
/// @generic.slot symbol=identity.T index=0 kind=type
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=value type=T

    return value;
    /// @resolution.name source=value target=value
    /// @type.node source=value type=T

}

identity<int32>("x");
/// @resolution.name source=identity target=identity
/// @type.node source=identity type=<T>(T) => T
/// @type.node source="\"x\"" type="x"

"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=6 column=17 source="identity<int32>(\"x\");"
"#,
    );
}

#[test]
fn test_check_records_imported_generic_instances_in_calling_module() {
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
/// @resolution.call source=identity(1) parameters=[1] return=1 kind=symbol target=lib.identity instance=lib.identity<1>
/// @generic.application source=identity(1) id=lib.identity<1>
/// @type.node source=identity type=<lib.identity.T>(lib.identity.T) => lib.identity.T
/// @type.node source=identity(1) type=1
/// @type.node source=1 type=1

const text = identity("x");
/// @type.symbol symbol=text type="x"
/// @resolution.name source=identity target=lib.identity
/// @resolution.call source="identity(\"x\")" parameters=["x"] return="x" kind=symbol target=lib.identity instance="lib.identity<\"x\">"
/// @generic.application source="identity(\"x\")" id="lib.identity<\"x\">"
/// @type.node source="identity(\"x\")" type="x"
/// @type.node source=identity type=<lib.identity.T>(lib.identity.T) => lib.identity.T
/// @type.node source="\"x\"" type="x"
/// @generic.instance id="lib.identity<\"x\">" symbol=lib.identity arguments=["x"]
/// @generic.instance id=lib.identity<1> symbol=lib.identity arguments=[1]
"#);
}

#[test]
fn test_check_consumes_body_inferred_generic_exports() {
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
/// @resolution.call source="identity(\"x\")" parameters=["x"] return="x" kind=symbol target=lib.identity instance="lib.identity<\"x\">"
/// @generic.application source="identity(\"x\")" id="lib.identity<\"x\">"
/// @type.node source="identity(\"x\")" type="x"
/// @type.node source=identity type=<lib.identity.T>(lib.identity.T) => lib.identity.T
/// @type.node source="\"x\"" type="x"
/// @generic.instance id="lib.identity<\"x\">" symbol=lib.identity arguments=["x"]
"#);
}
