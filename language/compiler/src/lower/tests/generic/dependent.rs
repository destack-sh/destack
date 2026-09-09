use crate::tests::TestSession;

/// Lower a conditional alias with an infer placeholder inside a signature union as its dependent.
#[test]
fn test_lower_an_inferring_alias_union_as_a_dependent_parameter() {
    let session = TestSession::single(
        r#"
struct Task<T> {
    value: T;
}

type Unwrap<T> = T extends Task<infer U> ? U : T;

function hasPrevious<T>(previous: Unwrap<T> | undefined): boolean {
    return previous !== undefined;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.hasPrevious",
        r#"
function test.main.hasPrevious<T, P0>(v0: variant<uint1> { 0uint1 = void; 1uint1 = P0; }): boolean {
    local l0: variant<uint1> { 0uint1 = void; 1uint1 = P0; }

entry(v0: variant<uint1> { 0uint1 = void; 1uint1 = P0; }):
    local.set l0, v0
    v1: ref<variant<uint1> { 0uint1 = void; 1uint1 = P0; }, borrowed, 'frame, readonly, frame> = local.project l0
    v2: uint1 = variant.tag.load v1
    v3: uint1 = 0
    v4: boolean = eq v2, v3
    v5: boolean = not v4
    return v5
}
"#,
    );
    session.assert_mir_function(
        "main.ds",
        "test.main.hasPrevious",
        r#"
function test.main.hasPrevious<T, P0>(v0: variant<uint1> { 0uint1 = void; 1uint1 = P0; }): boolean {
    local l0: variant<uint1> { 0uint1 = void; 1uint1 = P0; }

entry(v0: variant<uint1> { 0uint1 = void; 1uint1 = P0; }):
    local.set l0, v0
    v1: ref<variant<uint1> { 0uint1 = void; 1uint1 = P0; }, borrowed, 'frame, readonly, frame> = local.project l0
    v2: uint1 = variant.tag.load v1
    v3: uint1 = 0
    v4: boolean = eq v2, v3
    v5: boolean = not v4
    return v5
}
"#,
    );
}

/// Lower an intersection of open value operands as the dependent parameter its template mints.
#[test]
fn test_lower_an_open_intersection_as_a_dependent_parameter() {
    let session = TestSession::single(
        r#"
struct Context {
    id: int32;
}

type Merged<A, B> = Context & A & B;

function keep<A, B>(value: Merged<A, B>): Merged<A, B> {
    return value;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.keep",
        r#"
function test.main.keep<A, B, P0>(v0: P0): P0 {
    local l0: P0

entry(v0: P0):
    local.set l0, v0
    v1: P0 = local.get l0
    return v1
}
"#,
    );
    session.assert_mir_function(
        "main.ds",
        "test.main.keep",
        r#"
function test.main.keep<A, B, P0>(v0: P0): P0 {
    local l0: P0

entry(v0: P0):
    local.set l0, v0
    v1: P0 = local.get l0
    return v1
}
"#,
    );
}
