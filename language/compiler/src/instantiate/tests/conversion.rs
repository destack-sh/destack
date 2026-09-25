use crate::tests::TestSession;

/// Clone a bitwise-copy argument in a template by loading through the receiver.
#[test]
fn test_instantiate_clones_a_copy_argument_as_a_load() {
    let session = TestSession::single(
        r#"
function duplicate<T: Clone>(value: &immutable T): T {
    return value.clone();
}

function main(): int32 {
    return duplicate(1);
}
"#,
    );

    session.assert_mir_elaborated(
        "main.tspp", r#"
@nocopy
@languageItem("memory.Clone")
type Clone { }

@nocopy
@languageItem("math.Integer")
type Integer extends Concrete, Copy, IntegerDomain, Zero, One { }

@nocopy
@languageItem("memory.Concrete")
type Concrete { }

@nocopy
@languageItem("memory.Copy")
type Copy extends Clone { }

@nocopy
@languageItem("math.IntegerDomain")
type IntegerDomain { }

@nocopy
@languageItem("math.Zero")
type Zero { }

@nocopy
@languageItem("math.One")
type One { }

function test.main.main(): int32 {
    local l0: int32

entry:
    v0: int32 = 1
    store l0, v0
    v1: ref<int32, borrowed, 'frame, immutable> = address l0
    v2: int32 = call test.main.duplicate<int32>(v1): (ref<int32, borrowed, 'frame, immutable>) => int32
    return v2
}

function test.main.duplicate<T: Clone, 'a>(v0: ref<?T, borrowed, 'a, immutable>): T;

external function Clone.clone<this: Clone, 'a>(ref<?this, borrowed, 'a, immutable>): ?this

external function Integer.Clone.clone<T: Integer, 'a>(ref<?T, borrowed, 'a, immutable>): ?T

shared function Integer.Clone.clone<int32, 'a>(v0: ref<int32, borrowed, 'a, immutable>): int32 {
    local l0: ref<int32, borrowed, 'a, immutable>

entry(v0: ref<int32, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<int32, borrowed, 'a, immutable> = load l0
    v2: int32 = load (*v1)
    return v2
}

shared function test.main.duplicate<int32, 'a>(v0: ref<int32, borrowed, 'a, immutable>): int32 {
    local l0: ref<int32, borrowed, 'a, immutable>

entry(v0: ref<int32, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<int32, borrowed, 'a, immutable> = load l0
    v2: int32 = call Integer.Clone.clone<int32>(v1): (ref<int32, borrowed, 'a, immutable>) => int32
    v3: int32 = copy v2
    return v3
}

/// @dispatch.shape constraint=type@4 function=clone function=cloneFrom
"#,
    );
}
