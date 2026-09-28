use crate::tests::TestSession;

#[test]
fn test_lower_dynamic_newtype_to_its_erased_type() {
    let session = TestSession::single(
        r#"
import { Dynamic } from "tspp:memory";

interface Meter {
    read(&readonly this): int32;
}

newtype Reading = Dynamic<Meter>;
"#,
    );

    session.assert_mir_lowered(
        "main.tspp",
        r#"
type test.main.Reading = newtype<dynamic<test.main.Meter, managed, mutable, local>>;

@nocopy
type test.main.Meter { }

/// @layout.struct name=test.main.Meter size=0 align=1
/// @layout.struct name=type@5 size=0 align=1

/// @dispatch.shape constraint=type@1 function=read
"#,
    );
}

/// Lower atomic storage transparently over its value.
#[test]
fn test_lower_atomic_storage_transparently_over_its_value() {
    let session = TestSession::single(
        r#"
import { Atomic } from "tspp:sync";

newtype Counter = Atomic<int32>;
"#,
    );

    session.assert_mir_lowered(
        "main.tspp",
        r#"
type test.main.Counter = newtype<Atomic<int32>>;

type Atomic<T: AtomicSafe> {
    storage: T;
}

@nocopy
@languageItem("sync.AtomicSafe")
type AtomicSafe extends Concrete, Copy, SharedSafe { }

@nocopy
@languageItem("memory.Concrete")
type Concrete { }

@nocopy
@languageItem("memory.Copy")
type Copy extends Clone { }

@nocopy
@languageItem("memory.Clone")
type Clone { }

@nocopy
@languageItem("memory.SharedSafe")
type SharedSafe { }

/// @layout.struct name=Concrete size=0 align=1
/// @layout.struct name=Copy size=0 align=1
/// @layout.struct name=Clone size=0 align=1
/// @layout.struct name=SharedSafe size=0 align=1
/// @layout.struct name=type@6 size=0 align=1
/// @layout.struct name=Atomic<int32> size=4 align=4
/// @layout.field owner=Atomic<int32> index=0 name=storage offset=0 size=4 align=4
/// @layout.struct name=type@19 size=4 align=4
/// @layout.field owner=type@19 index=0 name=storage offset=0 size=4 align=4

/// @dispatch.shape constraint=type@9 function=clone function=cloneFrom
"#,
    );
}

#[test]
fn test_lower_unsafe_cell_transparently_over_its_value() {
    let session = TestSession::single(
        r#"
import { UnsafeCell } from "tspp:memory";

newtype Slot = UnsafeCell<int32>;
"#,
    );

    session.assert_mir_lowered(
        "main.tspp",
        r#"
type test.main.Slot = newtype<int32>;
"#,
    );
}

#[test]
fn test_lower_reflected_type_to_a_type_id() {
    let session = TestSession::single(
        r#"
import { Type } from "tspp:reflect";

export function accept(descriptor: Type<int32>): Type<int32> {
    return descriptor;
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.accept",
        r#"
export function test.main.accept(v0: typeId): typeId {
    local l0: typeId

entry(v0: typeId):
    store l0, v0
    v1: typeId = load l0
    return v1
}
"#,
    );
}
