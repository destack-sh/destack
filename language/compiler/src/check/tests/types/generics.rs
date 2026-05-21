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
        DirRows::checked(),
        r#"
function identity<T>(value: T): T {
/// @generic.slot symbol=identity.T index=0 kind=type
/// @type.symbol symbol=identity type=<T>(T) => T

    return value;
}

const number = identity(1);
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity(1)" parameters=[int32] return=int32 kind=symbol target=identity instance=identity<int32>
/// @instance.application source="identity(1)" id=identity<int32>
/// @type.symbol symbol=number type=int32

const text = identity("x");
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity(\"x\")" parameters=[string] return=string kind=symbol target=identity instance=identity<string>
/// @instance.application source="identity(\"x\")" id=identity<string>
/// @type.symbol symbol=text type=string

/// @instance.entry id=identity<int32> symbol=identity arguments=[int32]
/// @instance.entry id=identity<string> symbol=identity arguments=[string]
"#);
}

#[test]
fn test_check_reuses_identical_generic_instances() {
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
        DirRows::checked(),
        r#"
function identity<T>(value: T): T {
/// @generic.slot symbol=identity.T index=0 kind=type
/// @type.symbol symbol=identity type=<T>(T) => T

    return value;
}

const first = identity(1);
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity(1)" parameters=[int32] return=int32 kind=symbol target=identity instance=identity<int32>
/// @instance.application source="identity(1)" id=identity<int32>
/// @type.symbol symbol=first type=int32

const second = identity(2);
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity(2)" parameters=[int32] return=int32 kind=symbol target=identity instance=identity<int32>
/// @instance.application source="identity(2)" id=identity<int32>
/// @type.symbol symbol=second type=int32

/// @instance.entry id=identity<int32> symbol=identity arguments=[int32]
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
        DirRows::checked(),
        r#"
function identity<T>(value: T): T {
/// @generic.slot symbol=identity.T index=0 kind=type
/// @type.symbol symbol=identity type=<T>(T) => T

    return value;
}

const text = identity<string>("x");
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity<string>(\"x\")" parameters=[string] return=string kind=symbol target=identity instance=identity<string>
/// @instance.application source="identity<string>(\"x\")" id=identity<string>
/// @type.symbol symbol=text type=string

/// @instance.entry id=identity<string> symbol=identity arguments=[string]
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
        DirRows::checked(),
        r#"
function identity<T>(value: T): T {
/// @generic.slot symbol=identity.T index=0 kind=type
/// @type.symbol symbol=identity type=<T>(T) => T

    return value;
}

const asInt = identity<int32>;
/// @resolution.name source=identity target=identity
/// @instance.application source="identity<int32>" id=identity<int32>
/// @type.symbol symbol=asInt type=(int32) => int32

/// @instance.entry id=identity<int32> symbol=identity arguments=[int32]
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
        DirRows::checked(),
        r#"
declare function pair<T, U = T>(left: T, right?: U): (T, U);
/// @generic.slot symbol=pair.T index=0 kind=type
/// @generic.slot symbol=pair.U index=1 kind=type default=pair.T
/// @type.symbol symbol=pair type=<T, U = T>(T, U?) => (T, U)

const defaulted = pair(1);
/// @resolution.name source=pair target=pair
/// @resolution.call source="pair(1)" parameters=[int32] return=(int32, int32) kind=symbol target=pair instance="pair<int32, int32>"
/// @instance.application source="pair(1)" id="pair<int32, int32>"
/// @type.symbol symbol=defaulted type=(int32, int32)

const overridden = pair(1, "x");
/// @resolution.name source=pair target=pair
/// @resolution.call source="pair(1, \"x\")" parameters=[int32, string] return=(int32, string) kind=symbol target=pair instance="pair<int32, string>"
/// @instance.application source="pair(1, \"x\")" id="pair<int32, string>"
/// @type.symbol symbol=overridden type=(int32, string)

/// @instance.entry id="pair<int32, int32>" symbol=pair arguments=[int32, int32]
/// @instance.entry id="pair<int32, string>" symbol=pair arguments=[int32, string]
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
        DirRows::checked(),
        r#"
function take<comptime N: uint>(value: [uint8; N]): [uint8; N] {
/// @generic.slot symbol=take.N index=0 kind=static constraint=uint
/// @type.symbol symbol=take type=<N: uint>([uint8; N]) => [uint8; N]

    return value;
}

const bytes = take<4>([1, 2, 3, 4]);
/// @resolution.name source=take target=take
/// @resolution.call source="take<4>([1, 2, 3, 4])" parameters=[[uint8; 4]] return=[uint8; 4] kind=symbol target=take instance=take<4>
/// @instance.application source="take<4>([1, 2, 3, 4])" id=take<4>
/// @type.symbol symbol=bytes type=[uint8; 4]

/// @instance.entry id=take<4> symbol=take arguments=[4]
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
        DirRows::checked(),
        r#"
function choose<comptime Flag: boolean = true>(value: int32): int32 {
/// @generic.slot symbol=choose.Flag index=0 kind=static constraint=boolean default=true
/// @type.symbol symbol=choose type=<Flag: boolean = true>(int32) => int32

    return value;
}

const value = choose(1);
/// @resolution.name source=choose target=choose
/// @resolution.call source="choose(1)" parameters=[int32] return=int32 kind=symbol target=choose instance=choose<true>
/// @instance.application source="choose(1)" id=choose<true>
/// @type.symbol symbol=value type=int32

/// @instance.entry id=choose<true> symbol=choose arguments=[true]
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
        DirRows::checked(),
        r#"
type Tagged<comptime Tag: string> = { tag: Tag };
/// @generic.slot symbol=Tagged.Tag index=0 kind=static constraint=string
/// @type.symbol symbol=Tagged type={ tag: Tag }

type Flagged<comptime Config: { name: string; enabled: boolean }> = Config;
/// @generic.slot symbol=Flagged.Config index=0 kind=static constraint={ name: string; enabled: boolean }
/// @type.symbol symbol=Flagged type=Config

declare const tagged: Tagged<"alpha">;
/// @resolution.name source=Tagged target=Tagged
/// @instance.application source="Tagged<\"alpha\">" id="Tagged<\"alpha\">"
/// @type.symbol symbol=tagged type={ tag: "alpha" }

declare const flagged: Flagged<{ name: "search"; enabled: true }>;
/// @resolution.name source=Flagged target=Flagged
/// @instance.application source="Flagged<{ name: \"search\"; enabled: true }>" id="Flagged<{ name: \"search\"; enabled: true }>"
/// @type.symbol symbol=flagged type={ name: "search"; enabled: true }

/// @instance.entry id="Tagged<\"alpha\">" symbol=Tagged arguments=["alpha"]
/// @instance.entry id="Flagged<{ name: \"search\"; enabled: true }>" symbol=Flagged arguments=[{ name: "search"; enabled: true }]
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
        DirRows::checked(),
        r#"
function identity<T>(value: T): T {
/// @generic.slot symbol=identity.T index=0 kind=type
/// @type.symbol symbol=identity type=<T>(T) => T

    return value;
}

identity<int32>("x");
/// @resolution.name source=identity target=identity
/// @type.node source="identity<int32>(\"x\")" type=int32

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
        DirRows::checked(),
        r#"
=== lib.ds ===
export function identity<T>(value: T): T {
/// @generic.slot symbol=identity.T index=0 kind=type
/// @type.symbol symbol=identity type=<T>(T) => T

    return value;
}


=== main.ds ===
import { identity } from "./lib.ds";
/// @resolution.name source=identity target=lib.identity

const number = identity(1);
/// @resolution.name source=identity target=lib.identity
/// @resolution.call source="identity(1)" parameters=[int32] return=int32 kind=symbol target=lib.identity instance=lib.identity<int32>
/// @instance.application source="identity(1)" id=lib.identity<int32>
/// @type.symbol symbol=number type=int32

const text = identity("x");
/// @resolution.name source=identity target=lib.identity
/// @resolution.call source="identity(\"x\")" parameters=[string] return=string kind=symbol target=lib.identity instance=lib.identity<string>
/// @instance.application source="identity(\"x\")" id=lib.identity<string>
/// @type.symbol symbol=text type=string

/// @instance.entry id=lib.identity<int32> symbol=lib.identity arguments=[int32]
/// @instance.entry id=lib.identity<string> symbol=lib.identity arguments=[string]
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
        DirRows::checked(),
        r#"
=== lib.ds ===
export function identity<T>(value: T) {
/// @generic.slot symbol=identity.T index=0 kind=type
/// @type.symbol symbol=identity type=<T>(T) => T

    return value;
    /// @resolution.name source=value target=value
    /// @type.node source=value type=T
}


=== main.ds ===
import { identity } from "./lib.ds";
/// @resolution.name source=identity target=lib.identity

const text = identity("x");
/// @resolution.name source=identity target=lib.identity
/// @resolution.call source="identity(\"x\")" parameters=[string] return=string kind=symbol target=lib.identity instance=lib.identity<string>
/// @instance.application source="identity(\"x\")" id=lib.identity<string>
/// @type.symbol symbol=text type=string

/// @instance.entry id=lib.identity<string> symbol=lib.identity arguments=[string]
"#);
}
