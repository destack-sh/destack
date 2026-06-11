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
/// @type.node source=1 type=1
/// @type.node source=2 type=2

/// @check.stats.solve variables=14 terms=13 constraints=37 obligations=0 solutions=14 bounds=33 decisions=5
"#);
}

#[test]
fn test_imported_function_call_selects_exported_symbol() {
    let compiler = TestSession::new()
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
import { add } from "./math.ds";

const value = add(1, 2);
/// @type.symbol symbol=value source=value type=int32
/// @type.node source="add(1, 2)" type=int32
/// @resolution.name source=add target=math.add
/// @resolution.call source="add(1, 2)" parameters=(int32, int32) return=int32 kind=symbol target=math.add
/// @type.node source=1 type=1
/// @type.node source=2 type=2
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
function source(value?: unknown): void {}
/// @type.symbol symbol=source source="function source(value?: unknown): void {}" type=(unknown | undefined?) => ()
/// @type.symbol symbol=value#1 source="value?: unknown" type=unknown | undefined

declare function use(callback: (value: unknown) => void): void;
/// @type.symbol symbol=use source="declare function use(callback: (value: unknown) => void): void" type=((unknown) => ()) => ()
/// @type.symbol symbol=callback source="callback: (value: unknown) => void" type=(unknown) => ()
/// @type.symbol symbol=value#2 source="value: unknown" type=unknown

use(source);
/// @type.node source=use type=((unknown) => ()) => ()
/// @type.node source=use(source) type=()
/// @resolution.name source=use target=use
/// @resolution.call source=use(source) parameters=((unknown) => ()) return=() kind=symbol target=use
/// @type.node source=source type=(unknown | undefined?) => ()
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
declare function map<T>(callback: (value: unknown) => T): T;
/// @generic.template source=declaration parameters=[T]
/// @type.symbol symbol=map source="declare function map<T>(callback: (value: unknown) => T): T" type=<T>((unknown) => T) => T
/// @type.symbol symbol=map.T source=T type=T
/// @type.symbol symbol=callback source="callback: (value: unknown) => T" type=(unknown) => T
/// @type.symbol symbol=value#1 source="value: unknown" type=unknown
/// @resolution.name source=T target=map.T
/// @resolution.name source=T target=map.T

const value = map(() => 1);
/// @type.symbol symbol=value#2 source=value type=1
/// @generic.instance source="map(() => 1)" id=map<1>
/// @type.node source="map(() => 1)" type=1
/// @type.node source=map type=<T>((unknown) => T) => T
/// @resolution.name source=map target=map
/// @resolution.call source="map(() => 1)" parameters=((unknown) => 1) return=1 kind=symbol target=map instance=map<1>
/// @type.symbol symbol=symbol5 source="() => 1" type=() => 1
/// @type.node source="() => 1" type=() => 1
/// @type.node source=1 type=1

/// @generic.instance id=map<1> template=map arguments=[1]
"#,
    );
}
