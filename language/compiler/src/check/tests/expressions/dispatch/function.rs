use crate::tests::{DirRows, TestSession};

#[test]
fn test_free_function_call_selects_function_symbol() {
    let session = TestSession::single(
        r#"
function add(left: int32, right: int32): int32 {
    return left + right;
}

const value = add(1, 2);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked()
            .with_node_types()
            .without_reference_types()
            .with_check_stats(),
        r#"
=== annotated ===
function add(left: int32, right: int32): int32 {
    return left + right;
}

const value: int32 = add(1, 2);

=== checked ===
function add(left: int32, right: int32): int32 {
/// @type.symbol symbol=add type=(int32, int32) => int32
/// @type.symbol symbol=left source="left: int32" type=int32
/// @type.symbol symbol=right source="right: int32" type=int32

    return left + right;
    /// @type.node source="left + right" type=int32
    /// @resolution.name source=left target=left
    /// @resolution.call source="left + right" parameters=(int32, int32) return=int32 kind=builtin builtin=binary.add
    /// @resolution.name source=right target=right

}

const value = add(1, 2);
/// @type.symbol symbol=value source=value type=int32
/// @type.node source="add(1, 2)" type=int32
/// @resolution.name source=add target=add
/// @resolution.call source="add(1, 2)" parameters=(int32, int32) return=int32 kind=symbol target=add
/// @type.node source=1 type=int32
/// @type.node source=2 type=int32

/// @check.stats.solve variables=3 types=13 constraints=4 obligations=0 solutions=3 bounds=4 decisions=5
"#);
}

#[test]
fn test_imported_function_call_selects_exported_symbol() {
    let compiler = TestSession::builder()
        .module(
            "math.ds",
            r#"
export function add(left: int32, right: int32): int32 {
    return left + right;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { add } from "./math.ds";

const value = add(1, 2);
"#,
        )
        .build();

    compiler.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_node_types().without_reference_types(),
        r#"
=== annotated ===
import { add } from "./math.ds";

const value: int32 = add(1, 2);

=== checked ===
import { add } from "./math.ds";

const value = add(1, 2);
/// @type.symbol symbol=value source=value type=int32
/// @type.node source="add(1, 2)" type=int32
/// @resolution.name source=add target=math.add
/// @resolution.call source="add(1, 2)" parameters=(int32, int32) return=int32 kind=symbol target=math.add
/// @type.node source=1 type=int32
/// @type.node source=2 type=int32
"#);
}

#[test]
fn test_optional_parameter_function_satisfies_required_parameter_target() {
    let session = TestSession::single(
        r#"
function source(value?: unknown): void {}
declare function use(callback: (value: unknown) => void): void;

use(source);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function source(value?: unknown): void {}
declare function use(callback: (value: unknown) => void): void;

use(source as (unknown) => void);

=== checked ===
function source(value?: unknown): void {}
/// @type.symbol symbol=source source="function source(value?: unknown): void {}" type=(unknown | undefined) => void
/// @type.symbol symbol=value source="value?: unknown" type=unknown | undefined

declare function use(callback: (value: unknown) => void): void;
/// @type.symbol symbol=use source="declare function use(callback: (value: unknown) => void): void" type=(Function<(unknown,), void>) => void
/// @type.symbol symbol=callback source="callback: (value: unknown) => void" type=Function<(unknown,), void>

use(source);
/// @type.node source=use type=(Function<(unknown,), void>) => void
/// @type.node source=use(source) type=void
/// @resolution.name source=use target=use
/// @resolution.call source=use(source) parameters=(Function<(unknown,), void>) return=void kind=symbol target=use
/// @type.node source=source type=(unknown | undefined) => void
/// @resolution.name source=source target=source
"#,
    );
}

#[test]
fn test_callback_can_ignore_contextual_parameter() {
    let session = TestSession::single(
        r#"
declare function map<T>(callback: (value: unknown) => T): T;

const value = map(() => 1);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare function map<T>(callback: (value: unknown) => T): T;

const value: float64 = map<float64>(() => 1);

=== checked ===
declare function map<T>(callback: (value: unknown) => T): T;
/// @generic.template source=declaration parameters=(T)
/// @type.symbol symbol=map source="declare function map<T>(callback: (value: unknown) => T): T" type=<T>(Function<(unknown,), T>) => T
/// @type.symbol symbol=map.T source=T type=T
/// @type.symbol symbol=callback source="callback: (value: unknown) => T" type=Function<(unknown,), T>
/// @type.symbol symbol=value#1 source="value: unknown" type=unknown
/// @resolution.name source=T target=map.T
/// @resolution.name source=T target=map.T

const value = map(() => 1);
/// @type.symbol symbol=value#2 source=value type=float64
/// @generic.instance source="map(() => 1)" id=map<float64>
/// @type.node source="map(() => 1)" type=float64
/// @type.node source=map type=(Function<(unknown,), float64>) => float64
/// @resolution.name source=map target=map
/// @resolution.call source="map(() => 1)" parameters=(Function<(unknown,), float64>) return=float64 kind=symbol target=map instance=map<float64>
/// @type.symbol symbol=symbol5 source="() => 1" type=Function<(), float64>
/// @type.node source="() => 1" type=Function<(), float64>
/// @type.node source=1 type=float64

/// @generic.instance id=map<float64> template=map arguments=(float64)
"#,
    );
}

#[test]
fn test_defaulted_parameter_may_be_omitted_at_the_call() {
    // a parameter with a default value is optional at the call site
    // while keeping its exact type inside the body
    let session = TestSession::single(
        r#"
function greet(name: string = "world"): string {
    name
}

let short = greet();
let long = greet("compiler");
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function greet(name: string = "world"): string {
    name
}

let short: string = greet();
let long: string = greet("compiler");

=== checked ===
function greet(name: string = "world"): string {
/// @type.symbol symbol=greet type=(string) => string
/// @type.symbol symbol=greet.name source="name: string = \"world\"" type=string
/// @type.node source="\"world\"" type="world"

    name
    /// @type.node source=name type=string
    /// @resolution.name source=name target=greet.name

}

let short = greet();
/// @type.symbol symbol=short source=short type=string
/// @type.node source=greet type=(string) => string
/// @type.node source=greet() type=string
/// @resolution.name source=greet target=greet
/// @resolution.call source=greet() parameters=(string) arguments=(omitted as string) return=string kind=symbol target=greet

let long = greet("compiler");
/// @type.symbol symbol=long source=long type=string
/// @type.node source="greet(\"compiler\")" type=string
/// @type.node source=greet type=(string) => string
/// @resolution.name source=greet target=greet
/// @resolution.call source="greet(\"compiler\")" parameters=(string) arguments=(provided("compiler") as string) return=string kind=symbol target=greet
/// @type.node source="\"compiler\"" type="compiler"

"#,
        r#""#,
    );
}
