use crate::tests::TestSession;

#[test]
fn test_lower_inferred_generic_calls_to_concrete_instances() {
    let session = TestSession::single(
        r#"
function pick<T>(chosen: T, other: T, flag: boolean): T {
    if (flag) {
        return chosen;
    }
    return other;
}

function choose(low: int32, high: int32, flag: boolean): float64 {
    let first = pick(low, high, flag);
    return pick(1.5, 2.5, flag);
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.choose(v0: int32, v1: int32, v2: boolean): float64 {
    local l0: int32

entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = call test.main.pick(v0, v1, v2): (int32, int32, boolean) => int32
    local.set l0, v3
    v4: float64 = 1.5
    v5: float64 = 2.5
    v6: float64 = call test.main.pick_1(v4, v5, v2): (float64, float64, boolean) => float64
    return v6
}

function test.main.pick(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    branch v2, b1, b2

b1:
    return v0

b2:
    return v1
}

function test.main.pick_1(v0: float64, v1: float64, v2: boolean): float64 {
entry(v0: float64, v1: float64, v2: boolean):
    branch v2, b1, b2

b1:
    return v0

b2:
    return v1
}
"#,
    );

    // assert distinct runtime representations survive display naming
    let lowered = session.mir_lowered("main.ds");
    let strings = session.repository().string_pool();
    let symbols: Vec<_> = lowered
        .tree
        .iter_nodes::<destack_mir::Function>()
        .filter_map(|(_, function)| {
            (strings.get(function.name) == "test.main.pick").then_some(function.symbol)
        })
        .collect();
    assert_eq!(symbols.len(), 2);
    assert_ne!(symbols[0], symbols[1]);
}

#[test]
fn test_lower_generic_function_over_structural_representation() {
    let session = TestSession::single(
        r#"
function identity<T>(value: T): T {
    return value;
}

function keep(value: (int32, boolean)): (int32, boolean) {
    return identity(value);
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.keep(v0: (int32, boolean)): (int32, boolean) {
entry(v0: (int32, boolean)):
    v1: (int32, boolean) = call test.main.identity(v0): ((int32, boolean)) => (int32, boolean)
    return v1
}

function test.main.identity(v0: (int32, boolean)): (int32, boolean) {
entry(v0: (int32, boolean)):
    return v0
}
/// @layout.tuple name=type@2 size=8 align=4
/// @layout.element owner=type@2 index=0 offset=0 size=4 align=4
/// @layout.element owner=type@2 index=1 offset=4 size=1 align=1
"#,
    );
}

#[test]
fn test_lower_generic_struct_arguments_to_distinct_instances() {
    let session = TestSession::single(
        r#"
struct Box<T> {
    value: ^T;
}

function readInt(value: Box<int32>): int32 {
    return value.value;
}

function readFloat(value: Box<float64>): float64 {
    return value.value;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Box {
    value: int32;
}

@copy
type Box_1 {
    value: float64;
}

function test.main.readInt(v0: Box): int32 {
entry(v0: Box):
    v1: int32 = field.get v0, 0
    return v1
}

function test.main.readFloat(v0: Box_1): float64 {
entry(v0: Box_1):
    v1: float64 = field.get v0, 0
    return v1
}
/// @layout.struct name=Box size=4 align=4
/// @layout.field owner=Box index=0 name=value offset=0 size=4 align=4
/// @layout.struct name=Box_1 size=8 align=8
/// @layout.field owner=Box_1 index=0 name=value offset=0 size=8 align=8
"#,
    );

    // assert nominal specializations keep independent identities
    let lowered = session.mir_lowered("main.ds");
    let strings = session.repository().string_pool();
    let symbols: Vec<_> = lowered
        .tree
        .iter_nodes::<destack_mir::TypeDeclaration>()
        .filter_map(|(_, declaration)| {
            (strings.get(declaration.name) == "Box")
                .then(|| lowered.tree.type_symbol(declaration.ty))
                .flatten()
        })
        .collect();
    assert_eq!(symbols.len(), 2);
    assert_ne!(symbols[0], symbols[1]);
}

#[test]
fn test_lower_repeated_instantiations_to_one_shared_instance() {
    let session = TestSession::single(
        r#"
function pick<T>(chosen: T, other: T, flag: boolean): T {
    if (flag) {
        return chosen;
    }
    return other;
}

function narrow(flag: boolean): float64 {
    return pick(1, 2, flag);
}

function wide(flag: boolean): float64 {
    return pick(30.5, 40.5, flag);
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.narrow(v0: boolean): float64 {
entry(v0: boolean):
    v1: float64 = 1
    v2: float64 = 2
    v3: float64 = call test.main.pick(v1, v2, v0): (float64, float64, boolean) => float64
    return v3
}

function test.main.wide(v0: boolean): float64 {
entry(v0: boolean):
    v1: float64 = 30.5
    v2: float64 = 40.5
    v3: float64 = call test.main.pick(v1, v2, v0): (float64, float64, boolean) => float64
    return v3
}

function test.main.pick(v0: float64, v1: float64, v2: boolean): float64 {
entry(v0: float64, v1: float64, v2: boolean):
    branch v2, b1, b2

b1:
    return v0

b2:
    return v1
}
"#,
    );
}

#[test]
fn test_lower_transitive_generic_calls_through_instance_substitution() {
    let session = TestSession::single(
        r#"
function pick<T>(chosen: T, other: T, flag: boolean): T {
    if (flag) {
        return chosen;
    }
    return other;
}

function retry<T>(value: T, fallback: T, flag: boolean): T {
    return pick(value, fallback, flag);
}

function settle(count: int32, limit: int32, flag: boolean): int32 {
    return retry(count, limit, flag);
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.settle(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = call test.main.retry(v0, v1, v2): (int32, int32, boolean) => int32
    return v3
}

function test.main.retry(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = call test.main.pick(v0, v1, v2): (int32, int32, boolean) => int32
    return v3
}

function test.main.pick(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    branch v2, b1, b2

b1:
    return v0

b2:
    return v1
}
"#,
    );
}

#[test]
fn test_lower_imported_generic_calls_as_local_instance_copies() {
    let session = TestSession::builder()
        .module(
            "lib.ds",
            r#"
export function pick<T>(chosen: T, other: T, flag: boolean): T {
    if (flag) {
        return chosen;
    }
    return other;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { pick } from "./lib";

function choose(low: int32, high: int32, flag: boolean): int32 {
    return pick(low, high, flag);
}
"#,
        )
        .build();

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.choose(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = call test.lib.pick(v0, v1, v2): (int32, int32, boolean) => int32
    return v3
}

function test.lib.pick(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    branch v2, b1, b2

b1:
    return v0

b2:
    return v1
}
"#,
    );

    session.assert_mir_lowered(
        "lib.ds", r#"
"#,
    );
}

#[test]
fn test_lower_transitive_imported_instances_through_foreign_bodies() {
    let session = TestSession::builder()
        .module(
            "lib.ds",
            r#"
export function pick<T>(chosen: T, other: T, flag: boolean): T {
    if (flag) {
        return chosen;
    }
    return other;
}

export function retry<T>(value: T, fallback: T, flag: boolean): T {
    return pick(value, fallback, flag);
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { retry } from "./lib";

function settle(count: int32, limit: int32, flag: boolean): int32 {
    return retry(count, limit, flag);
}
"#,
        )
        .build();

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.settle(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = call test.lib.retry(v0, v1, v2): (int32, int32, boolean) => int32
    return v3
}

function test.lib.retry(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = call test.lib.pick(v0, v1, v2): (int32, int32, boolean) => int32
    return v3
}

function test.lib.pick(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    branch v2, b1, b2

b1:
    return v0

b2:
    return v1
}
"#,
    );
}

#[test]
fn test_lower_instance_names_under_a_named_package() {
    let session = TestSession::builder()
        .module(
            "destack.json",
            r#"{
  "name": "app",
  "compiler": {
    "emitStats": true,
    "emitEvents": true,
    "emitCheckedTypes": true
  }
}"#,
        )
        .module(
            "main.ds",
            r#"
function pick<T>(chosen: T, other: T, flag: boolean): T {
    if (flag) {
        return chosen;
    }
    return other;
}

function choose(low: int32, high: int32, flag: boolean): int32 {
    return pick(low, high, flag);
}
"#,
        )
        .build();

    session.assert_mir_lowered(
        "main.ds",
        r#"
function app.main.choose(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = call app.main.pick(v0, v1, v2): (int32, int32, boolean) => int32
    return v3
}

function app.main.pick(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    branch v2, b1, b2

b1:
    return v0

b2:
    return v1
}
"#,
    );
}
