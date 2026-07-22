use crate::tests::TestSession;

#[test]
fn test_lower_inferred_generic_calls_to_materialized_instances() {
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
function main.choose(v0: int32, v1: int32, v2: boolean): float64 {
    local l0: int32

entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = call main.pick#int32(v0, v1, v2)
    local.set l0, v3
    v4: float64 = 1.5
    v5: float64 = 2.5
    v6: float64 = call main.pick#float64(v4, v5, v2)
    return v6
}

function main.pick#int32(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    branch v2, b1, b2

b1:
    return v0

b2:
    return v1
}

function main.pick#float64(v0: float64, v1: float64, v2: boolean): float64 {
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
function main.narrow(v0: boolean): float64 {
entry(v0: boolean):
    v1: float64 = 1
    v2: float64 = 2
    v3: float64 = call main.pick#float64(v1, v2, v0)
    return v3
}

function main.wide(v0: boolean): float64 {
entry(v0: boolean):
    v1: float64 = 30.5
    v2: float64 = 40.5
    v3: float64 = call main.pick#float64(v1, v2, v0)
    return v3
}

function main.pick#float64(v0: float64, v1: float64, v2: boolean): float64 {
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
function main.settle(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = call main.retry#int32(v0, v1, v2)
    return v3
}

function main.retry#int32(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = call main.pick#int32(v0, v1, v2)
    return v3
}

function main.pick#int32(v0: int32, v1: int32, v2: boolean): int32 {
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
function main.choose(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = call lib.pick#int32(v0, v1, v2)
    return v3
}

function lib.pick#int32(v0: int32, v1: int32, v2: boolean): int32 {
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
function main.settle(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = call lib.retry#int32(v0, v1, v2)
    return v3
}

function lib.retry#int32(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = call lib.pick#int32(v0, v1, v2)
    return v3
}

function lib.pick#int32(v0: int32, v1: int32, v2: boolean): int32 {
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
    v3: int32 = call app.main.pick#int32(v0, v1, v2)
    return v3
}

function app.main.pick#int32(v0: int32, v1: int32, v2: boolean): int32 {
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
