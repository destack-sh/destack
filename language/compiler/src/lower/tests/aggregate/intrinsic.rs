use crate::tests::TestSession;

#[test]
fn test_lower_dynamic_newtype_to_its_erased_type() {
    let session = TestSession::single(
        r#"
import { Dynamic } from "destack:memory";

interface Meter {
    read(&readonly this): int32;
}

newtype Reading = Dynamic<Meter>;
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type test.main.Meter { }

@copy
type test.main.Reading = newtype<dynamic<test.main.Meter, managed, mutable, local>>;

/// @layout.struct name=test.main.Meter size=0 align=1

/// @dispatch.shape constraint=type@1 function=read
"#,
    );
}

#[test]
fn test_lower_atomic_storage_to_an_atomic_cell() {
    let session = TestSession::single(
        r#"
import { Atomic } from "destack:sync";

newtype Counter = Atomic<int32>;
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type test.main.Counter = newtype<atomic<int32>>;

@languageItem("memory.Clone")
type Clone { }

/// @layout.struct name=Clone size=0 align=1

/// @dispatch.shape constraint=type@4 function=clone function=cloneFrom
"#,
    );
}

#[test]
fn test_lower_unsafe_cell_transparently_over_its_value() {
    let session = TestSession::single(
        r#"
import { UnsafeCell } from "destack:memory";

newtype Slot = UnsafeCell<int32>;
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type test.main.Slot = newtype<int32>;
"#,
    );
}

#[test]
fn test_lower_reflected_type_to_a_type_id() {
    let session = TestSession::single(
        r#"
import { Type } from "destack:reflect";

export function accept(descriptor: Type<int32>): Type<int32> {
    return descriptor;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.accept",
        r#"
function test.main.accept(v0: typeId): typeId {
    local l0: typeId

entry(v0: typeId):
    local.set l0, v0
    v1: typeId = local.get l0
    return v1
}
"#,
    );
}
