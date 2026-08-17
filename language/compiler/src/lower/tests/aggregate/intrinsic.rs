use crate::tests::TestSession;

#[test]
fn test_lower_dynamic_newtype_to_its_erased_carrier() {
    let session = TestSession::single(
        r#"
@languageItem("memory.Dynamic")
newtype Dynamic<T> = intrinsic;

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

type Dynamic<dynamic<Meter, managed, mutable>> = dynamic<Meter, managed, mutable>;

@copy
type Reading = newtype<Dynamic<dynamic<Meter, managed, mutable>>>;

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
fn test_lower_type_identity_to_the_runtime_type_id() {
    let session = TestSession::single(
        r#"
@languageItem("reflect.TypeId")
newtype TypeId = intrinsic;

newtype Tag = TypeId;
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type TypeId = typeId;

@copy
type Tag = newtype<TypeId>;
"#,
    );
}

#[test]
fn test_lower_unsafe_cell_transparently_over_its_value() {
    let session = TestSession::single(
        r#"
@languageItem("memory.UnsafeCell")
newtype UnsafeCell<T> = intrinsic;

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
type destack.reflect.type.Type<int32> = typeDescriptor;

function test.main.accept(v0: destack.reflect.type.Type<int32>): destack.reflect.type.Type<int32> {
entry(v0: destack.reflect.type.Type<int32>):
    return v0
}
"#,
    );
}
