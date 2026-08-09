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
            ,
        r#"
=== annotated ===
function add(left: int32, right: int32): int32 {
    return left + right;
}

const value: int32 = add(1, 2);

=== checked ===
function add(left: int32, right: int32): int32 {
/// @type.symbol symbol=add type=(int32, int32) => int32
/// @type.symbol symbol=add.left source="left: int32" type=int32
/// @type.symbol symbol=add.right source="right: int32" type=int32

    return left + right;
    /// @type.node source="left + right" type=int32
    /// @resolution.name source=left target=add.left
    /// @resolution.operator source="left + right" type=int32 operator="+" kind=builtin operands=[left as int32 families=(integer), right as int32 families=(integer)]
    /// @resolution.place source=left placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=left root=add.left
    /// @resolution.name source=right target=add.right
    /// @resolution.place source=right placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=right root=add.right

}

const value = add(1, 2);
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="add(1, 2)" type=int32
/// @resolution.name source=add target=add
/// @resolution.call source="add(1, 2)" parameters=(int32, int32) arguments=(provided(1) as int32, provided(2) as int32) return=int32 kind=symbol target=add
/// @type.node source=1 type=1
/// @type.node source=2 type=2
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
        DirRows::checked()
            .with_node_types()
            .without_reference_types(),
        r#"
=== annotated ===
import { add } from "./math.ds";

const value: int32 = add(1, 2);

=== checked ===
import { add } from "./math.ds";

const value = add(1, 2);
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="add(1, 2)" type=int32
/// @resolution.name source=add target=math.add
/// @resolution.call source="add(1, 2)" parameters=(int32, int32) arguments=(provided(1) as int32, provided(2) as int32) return=int32 kind=symbol target=math.add
/// @type.node source=1 type=1
/// @type.node source=2 type=2
"#,
    );
}

#[test]
fn test_optional_parameter_function_needs_a_thunk_for_required_targets() {
    // an optional-parameter callback stores its parameter as a union
    //  carrier, so passing it where a required-parameter function type is
    //  expected changes the interior representation; a compiler-synthesized
    //  thunk coercion is the designed path to make this flow again
    let session = TestSession::single(
        r#"
function source(value?: unknown): void {}
declare function use(callback: (value: unknown) => void): void;

use(source);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function source(value?: Dynamic<unknown>): void {}
declare function use(callback: (arg0: Dynamic<unknown>) => void): void;

use(source);

=== checked ===
function source(value?: unknown): void {}
/// @type.symbol symbol=source source="function source(value?: unknown): void {}" type=(Dynamic<unknown> | undefined?) => void
/// @type.symbol symbol=source.value source="value?: unknown" type=Dynamic<unknown> | undefined

declare function use(callback: (value: unknown) => void): void;
/// @type.symbol symbol=use source="declare function use(callback: (value: unknown) => void): void" type=(Function<(unknown,), void>) => void
/// @type.symbol symbol=use.callback source="callback: (value: unknown) => void" type=Function<(Dynamic<unknown>,), void>

use(source);
/// @type.node source=use type=(Function<(unknown,), void>) => void
/// @type.node source=use(source) type=void
/// @resolution.name source=use target=use
/// @resolution.call source=use(source) parameters=(Function<(unknown,), void>) arguments=(provided(source) as Function<(unknown,), void>) return=void kind=symbol target=use
/// @type.node source=source type=(Dynamic<unknown> | undefined?) => void
/// @resolution.name source=source target=source
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type '(Dynamic<unknown> | undefined) => void' is not assignable to parameter of type '(unknown) => void'"
/// @diagnostic.label line=5 column=5 span="source" line_source="use(source);"
/// @diagnostic.related line=5 column=1 span="use(source)" line_source="use(source);" message="in this call"
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
declare function map<T>(callback: (arg0: Dynamic<unknown>) => T): T;

const value: float64 = map<float64>((): float64 => 1);

=== checked ===
declare function map<T>(callback: (value: unknown) => T): T;
/// @generic.template symbol=map parameters=(T)
/// @type.symbol symbol=map source="declare function map<T>(callback: (value: unknown) => T): T" type=<T>(Function<(unknown,), T>) => T
/// @type.symbol symbol=map.T source=T type=T
/// @type.symbol symbol=map.callback source="callback: (value: unknown) => T" type=Function<(Dynamic<unknown>,), T>
/// @resolution.name source=T target=map.T
/// @resolution.name source=T target=map.T

const value = map(() => 1);
/// @type.symbol symbol=value source=value type=float64
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="map(() => 1)" type=float64
/// @type.node source=map type=(Function<(unknown,), float64>) => float64
/// @resolution.name source=map target=map
/// @resolution.call source="map(() => 1)" parameters=(Function<(unknown,), float64>) arguments=(provided(() => 1) as Function<(unknown,), float64>) return=float64 kind=symbol target=map instance=map<float64>
/// @generic.instance source="map(() => 1)" id=map<float64>
/// @type.symbol symbol=symbol5 source="() => 1" type=Function<(), float64>
/// @type.node source="() => 1" type=Function<(), float64>
/// @type.node source=1 type=1

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

    session.assert_dir_checked(
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
/// @type.symbol symbol=greet type=(string?) => string
/// @type.symbol symbol=greet.name source="name: string = \"world\"" type=string
/// @type.node source="\"world\"" type="world"

    name
    /// @type.node source=name type=string
    /// @resolution.name source=name target=greet.name
    /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=name root=greet.name

}

let short = greet();
/// @type.symbol symbol=short source=short type=string
/// @resolution.pattern source=short kind=binding target=short
/// @type.node source=greet type=(string?) => string
/// @type.node source=greet() type=string
/// @resolution.name source=greet target=greet
/// @resolution.call source=greet() parameters=(string) arguments=(omitted as string) return=string kind=symbol target=greet

let long = greet("compiler");
/// @type.symbol symbol=long source=long type=string
/// @resolution.pattern source=long kind=binding target=long
/// @type.node source="greet(\"compiler\")" type=string
/// @type.node source=greet type=(string?) => string
/// @resolution.name source=greet target=greet
/// @resolution.call source="greet(\"compiler\")" parameters=(string) arguments=(provided("compiler") as string) return=string kind=symbol target=greet
/// @type.node source="\"compiler\"" type="compiler"
"#,
    );
}
