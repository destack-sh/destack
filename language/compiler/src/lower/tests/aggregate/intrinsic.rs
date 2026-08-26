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
type Meter { }

@copy
type Reading = newtype<dynamic<Meter, managed, mutable, local>>;

/// @layout.struct name=Meter size=0 align=1

/// @dispatch.shape constraint=type@1 function=read
"#,
    );
}

#[test]
fn test_lower_atomic_storage_to_an_atomic_cell() {
    let session = TestSession::single(
        r#"
@languageItem("sync.Atomic")
newtype Atomic<T> = intrinsic;

newtype Counter = Atomic<int32>;
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Atomic<int32> = atomic<int32>;

type Counter = newtype<Atomic<int32>>;
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
type UnsafeCell<int32> = int32;

@copy
type Slot = newtype<UnsafeCell<int32>>;
"#,
    );
}

#[test]
fn test_lower_the_lifetime_marker_without_a_representation() {
    let session = TestSession::single(
        r#"
@languageItem("memory.Lifetime")
newtype Lifetime = intrinsic;

newtype Meters = int32;
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Meters = newtype<int32>;
"#,
    );
}

#[test]
fn test_lower_reflected_type_to_a_type_descriptor() {
    let session = TestSession::single(
        r#"
import { Type } from "destack:reflect";

export function accept(descriptor: Type<int32>): Type<int32> {
    return descriptor;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Type<int32> = typeDescriptor;

function test.main.accept(v0: Type<int32>): Type<int32> {
entry(v0: Type<int32>):
    return v0
}
"#,
    );
}
