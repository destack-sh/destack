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
function test.main.hasPrevious<T, P0>(v0: variant<uint1> { 0uint1 = P0; 1uint1 = void; }): boolean {
    local l0: variant<uint1> { 0uint1 = P0; 1uint1 = void; }

entry(v0: variant<uint1> { 0uint1 = P0; 1uint1 = void; }):
    store l0, v0
    v1: uint1 = variant.tag.load l0
    v2: uint1 = 1
    v3: boolean = eq v1, v2
    v4: boolean = not v3
    return v4
}
"#,
    );
    session.assert_mir_function(
        "main.ds",
        "test.main.hasPrevious",
        r#"
function test.main.hasPrevious<T, P0>(v0: variant<uint1> { 0uint1 = P0; 1uint1 = void; }): boolean {
    local l0: variant<uint1> { 0uint1 = P0; 1uint1 = void; }

entry(v0: variant<uint1> { 0uint1 = P0; 1uint1 = void; }):
    store l0, v0
    v1: uint1 = variant.tag.load l0
    v2: uint1 = 1
    v3: boolean = eq v1, v2
    v4: boolean = not v3
    return v4
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
    store l0, v0
    v1: P0 = load l0
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
    store l0, v0
    v1: P0 = load l0
    return v1
}
"#,
    );
}

/// Lower a qualified projection of a memory-kind associated const as an access dependent.
#[test]
fn test_lower_a_memory_kind_const_projection_as_an_access_dependent() {
    let session = TestSession::single(
        r#"
import { Access, Borrowed, Region } from "destack:memory";

newtype interface Dereference<const A: Access = "readonly"> {
    type Target;
    const OutputAccess: Access = A;
    dereference<const R: Region>(
        this: Borrowed<this, R, A>,
    ): Borrowed<this.Target, R, this.OutputAccess>;
}

function read<P: Dereference<"readonly", type Target = int32, const OutputAccess = "readonly">>(
    pointer: &readonly P,
): int32 {
    return *pointer.dereference();
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.read",
        r#"
@nocopy
type test.main.Dereference<A: Access> { }

function test.main.read<P, 'a>(v0: ref<?P, borrowed, 'a, readonly>): int32 {
    local l0: ref<?P, borrowed, 'a, readonly>

entry(v0: ref<?P, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<?P, borrowed, 'a, readonly> = load l0
    v2: ref<witness<P, test.main.Dereference<readonly>, Target>, borrowed, 'a, readonly> = call.witness P, test.main.Dereference<readonly>, test.main.Dereference.dereference<readonly>(v1): (ref<?P, borrowed, 'a, readonly>) => ref<witness<P, test.main.Dereference<readonly>, Target>, borrowed, 'a, readonly>
    v3: witness<P, test.main.Dereference<readonly>, Target> = load (*v2)
    return v3
}
"#,
    );
}
