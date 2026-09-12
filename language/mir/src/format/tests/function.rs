use super::assert_format;

/// Formats a simple add function canonically.
#[test]
fn test_format_simple_add() {
    assert_format(
        r#"
function add(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = add v0, v1
    return v2
}
"#,
    );
}

/// Formats local declarations and local access operations canonically.
#[test]
fn test_format_with_locals() {
    assert_format(
        r#"
function withLocals(): int64 {
    local l0: int64

entry:
    v0: int64 = 42
    store l0, v0
    v1: int64 = load l0
    return v1
}
"#,
    );
}

/// Formats local address operations canonically.
#[test]
fn test_format_local_address() {
    assert_format(
        r#"
function localAddr(): void {
    local l0: int32

entry:
    v0: ref<int32, borrowed, 'frame & frame, mutable> = address l0
    return
}
"#,
    );
}

/// Formats block control flow canonically.
#[test]
fn test_format_branch() {
    assert_format(
        r#"
function choose(v0: boolean, v1: int32, v2: int32): int32 {
entry(v0: boolean, v1: int32, v2: int32):
    branch v0 => b1(v1) | b2(v2)

b1(v3: int32):
    return v3

b2(v4: int32):
    return v4
}
"#,
    );
}

/// Formats void returns canonically.
#[test]
fn test_format_void_return() {
    assert_format(
        r#"
function noop(): void {
entry:
    return
}
"#,
    );
}

/// Formats environment functions and function values canonically.
#[test]
fn test_format_function_environment() {
    assert_format(
        r#"
@environment(ref<void, managed, mutable, local>)
function callee(v0: int32): int32 {
entry(v0: int32):
    v1: ref<void, managed, mutable, local> = function.environment.current
    return v0
}

@environment(ref<void, managed, mutable, local>)
function caller(): int32 {
entry:
    v0: ref<void, managed, mutable, local> = function.environment.current
    v1: function<(int32) => int32, repeatable, managed, mutable, local> = function.bind callee, v0
    v2: ref<void, managed, mutable, local> = function.environment v1
    v3: int32 = 1
    v4: int32 = call.indirect v1(v3): (int32) => int32
    return v4
}
"#,
    );
}

/// Formats concrete function instances and lifetime binders canonically.
#[test]
fn test_format_function_instances() {
    assert_format(
        r#"
function identity<int32>(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function borrow<int32, 'a>(v0: ref<int32, borrowed, 'a & local, readonly>): ref<int32, borrowed, 'a & local, readonly> {
entry(v0: ref<int32, borrowed, 'a & local, readonly>):
    return v0
}

function use(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call identity<int32>(v0): (int32) => int32
    return v1
}
"#,
    );
}

/// Preserves sparse SSA value identities.
#[test]
fn test_format_value_identity() {
    assert_format(
        r#"
function choose(v3: int32): int32 {
entry(v3: int32):
    v7: int32 = 1
    v9: int32 = add v3, v7
    return v9
}
"#,
    );
}

/// Preserves reference access spelling in canonical output.
#[test]
fn test_format_reference_access_preserved() {
    assert_format(
        r#"
function refMutability(v0: ref<int32, managed, mutable, local>, v1: ref<int32, unique, readonly, local>): ref<int32, managed, mutable, local> {
entry(v0: ref<int32, managed, mutable, local>, v1: ref<int32, unique, readonly, local>):
    return v0
}
"#,
    );
}

/// Formats templates, their applications, and witness calls canonically.
#[test]
fn test_format_polymorphic_function() {
    assert_format(
        r#"
type Clone = void;

type Tagged = void;

type Box<T> {
    value: T;
}

type Pair<T, U, 'a> {
    left: ref<T, borrowed, 'a & local, readonly>;
    right: U;
}

external function Clone.clone<Self: Clone>(Self): Self

function duplicate<T: Clone>(v0: T): T {
entry(v0: T):
    v1: T = call.witness T, Clone, Clone.clone(v0): (T) => T
    v2: Box<T> = aggregate (v1)
    v3: T = field.get v2, 0
    return v3
}

function caller(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call duplicate<int32>(v0): (int32) => int32
    return v1
}

function place<T, space S, access A, const N: usize, 'a>(v0: ref<T, borrowed, 'a & heap(S), A>, v1: [T; N]): ref<T, borrowed, 'a & static(S), A> {
entry(v0: ref<T, borrowed, 'a & heap(S), A>, v1: [T; N]):
    v2: ref<T, borrowed, 'a & static(S), A> = cast.bit v0 -> ref<T, borrowed, 'a & static(S), A>
    return v2
}

function measure<T>(): usize {
entry:
    v0: usize = size.of T
    v1: usize = align.of T
    v2: usize = stride.of Box<T>
    return v2
}

function tag<T: Tagged>(): int32 {
entry:
    v0: int32 = witness T, Tagged, Tag
    return v0
}
"#,
    );
}

/// Formats a declared specialization and the calls resolving to it canonically.
#[test]
fn test_format_specialization() {
    assert_format(
        r#"
function duplicate<T>(v0: T): T {
entry(v0: T):
    return v0
}

shared function duplicate<int32>(v0: int32): int32;

function caller(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call duplicate<int32>(v0): (int32) => int32
    return v1
}
"#,
    );
}
