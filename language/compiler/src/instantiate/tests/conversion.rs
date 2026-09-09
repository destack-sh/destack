use crate::tests::TestSession;

/// Clone a bitwise-copy argument in a template by loading through the receiver.
#[test]
fn test_instantiate_clones_a_copy_argument_as_a_load() {
    let session = TestSession::single(
        r#"
function duplicate<T: Clone>(value: &readonly T): T {
    return value.clone();
}

function main(): int32 {
    return duplicate(1);
}
"#,
    );

    session.assert_mir_elaborated(
        "main.ds", r#"
@languageItem("memory.Clone")
type Clone { }

@languageItem("memory.Concrete")
type Concrete { }

@languageItem("memory.Copy")
type Copy extends Clone { }

@languageItem("math.IntegerDomain")
type IntegerDomain { }

@languageItem("math.Zero")
type Zero { }

@languageItem("math.One")
type One { }

@languageItem("math.Integer")
type Integer extends Concrete, Copy, IntegerDomain, Zero, One { }

function test.main.main(): int32 {
    local l0: int32

entry:
    v0: int32 = 1
    local.set l0, v0
    v1: ref<int32, borrowed, 'frame, readonly, local> = local.address l0
    v2: int32 = call test.main.duplicate<int32>(v1): <'a>(ref<int32, borrowed, 'a, readonly, local>) => int32
    return v2
}

function test.main.duplicate<T: Clone, 'a>(v0: ref<T, borrowed, 'a, readonly, local>): T;

external function Clone.clone<this: Clone, 'a>(ref<this, borrowed, 'a, readonly, local>): this

external function Integer.Clone.clone<T: Integer, 'a>(ref<T, borrowed, 'a, readonly, local>): T

shared function Integer.Clone.clone<int32, 'a>(v0: ref<int32, borrowed, 'a, readonly, local>): int32 {
    local l0: ref<int32, borrowed, 'a, readonly, local>

entry(v0: ref<int32, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<int32, borrowed, 'a, readonly, local> = local.get l0
    v2: int32 = load v1
    return v2
}

shared function test.main.duplicate<int32, 'a>(v0: ref<int32, borrowed, 'a, readonly, local>): int32 {
    local l0: ref<int32, borrowed, 'a, readonly, local>

entry(v0: ref<int32, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<int32, borrowed, 'a, readonly, local> = local.get l0
    v2: int32 = load v1
    return v2
}

/// @layout.struct name=Clone size=0 align=1
/// @layout.struct name=Concrete size=0 align=1
/// @layout.struct name=Copy size=0 align=1
/// @layout.struct name=IntegerDomain size=0 align=1
/// @layout.struct name=Zero size=0 align=1
/// @layout.struct name=One size=0 align=1
/// @layout.struct name=Integer size=0 align=1

/// @dispatch.shape constraint=type@4 function=clone function=cloneFrom
/// @dispatch.shape constraint=type@20 function=zero
/// @dispatch.shape constraint=type@22 function=one
"#,
    );
}
