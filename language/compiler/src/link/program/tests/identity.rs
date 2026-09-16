use destack_source::{ModuleId, PackageId};

use crate::link::tests::TestModule;
use crate::{LinkError, ProgramLinker};

/// Resolve functions, globals, and structural types across independently emitted objects.
#[test]
fn test_link_objects_resolves_imports() {
    let package = PackageId::new(0);
    let provider_module = ModuleId::new(package, 0);
    let consumer_module = ModuleId::new(package, 1);
    let provider = TestModule::emit(
        provider_module,
        r#"
export global answer: int32 = 42

export function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#,
        [],
    );
    let consumer = TestModule::emit(
        consumer_module,
        r#"
external global answer: int32
external function callee(int32): int32

export function caller(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call callee(v0): (int32) => int32
    return v1
}

export function readAnswer(): int32 {
entry:
    v0: ref<int32, borrowed, 'static, mutable, static> = address @answer
    v1: int32 = load (*v0)
    return v1
}
"#,
        [provider_module],
    );
    let strings = TestModule::merge_strings([&provider, &consumer]);
    let linker = ProgramLinker::new(
        package,
        vec![
            (consumer.module, consumer.object.clone()),
            (provider.module, provider.object.clone()),
        ],
        &strings,
    )
    .expect("objects should link");

    // resolve the external declaration to the exported definition
    let imported = linker.function(consumer_module, "callee");
    let exported = linker.function(provider_module, "callee");
    assert_eq!(
        linker.function_id(consumer_module, imported),
        linker.function_id(provider_module, exported)
    );

    // canonicalize primitive types while preserving independent functions
    let consumer_type = linker.int32_type(consumer_module);
    let provider_type = linker.int32_type(provider_module);
    assert_eq!(
        linker.type_id(consumer_module, consumer_type),
        linker.type_id(provider_module, provider_type)
    );
    assert_ne!(
        linker.function_id(consumer_module, linker.function(consumer_module, "caller")),
        linker.function_id(provider_module, exported)
    );

    // resolve the external global to exported storage
    let imported = linker.global(consumer_module, "answer");
    let exported = linker.global(provider_module, "answer");
    assert_eq!(
        linker.global_id(consumer_module, imported),
        linker.global_id(provider_module, exported)
    );
}

/// Resolve structurally equal anonymous signature types across module objects.
#[test]
fn test_link_objects_resolves_structural_signatures() {
    let package = PackageId::new(0);
    let provider_module = ModuleId::new(package, 0);
    let consumer_module = ModuleId::new(package, 1);
    let provider = TestModule::emit(
        provider_module,
        r#"
export function transform(v0: (int32, boolean)): (int32, boolean) {
entry(v0: (int32, boolean)):
    return v0
}
"#,
        [],
    );
    let consumer = TestModule::emit(
        consumer_module,
        r#"
external function transform((int32, boolean)): (int32, boolean)
"#,
        [provider_module],
    );
    let strings = TestModule::merge_strings([&provider, &consumer]);
    let linker = ProgramLinker::new(
        package,
        vec![
            (consumer.module, consumer.object.clone()),
            (provider.module, provider.object.clone()),
        ],
        &strings,
    )
    .expect("structurally equal signatures should link");

    // read the anonymous parameter type each object declared
    let imported = linker.function(consumer_module, "transform");
    let exported = linker.function(provider_module, "transform");
    let imported_object = linker.object(consumer_module);
    let exported_object = linker.object(provider_module);
    let imported_type = imported_object
        .function(imported)
        .expect("imported function should exist")
        .parameters[0]
        .ty;
    let exported_type = exported_object
        .function(exported)
        .expect("exported function should exist")
        .parameters[0]
        .ty;

    // merge structural runtime types while preserving nominal identities
    assert_eq!(
        linker.function_id(consumer_module, imported),
        linker.function_id(provider_module, exported)
    );
    assert_eq!(
        linker.type_id(consumer_module, imported_type),
        linker.type_id(provider_module, exported_type)
    );
}

/// Reject structurally equal signatures with different nominal identities.
#[test]
fn test_link_objects_rejects_nominal_signature_mismatch() {
    let package = PackageId::new(0);
    let provider_module = ModuleId::new(package, 0);
    let consumer_module = ModuleId::new(package, 1);
    let provider = TestModule::emit(
        provider_module,
        r#"
type Left {
    value: int32;
}

export function transform(v0: Left): int32 {
entry(v0: Left):
    v1: int32 = 0
    return v1
}
"#,
        [],
    );
    let consumer = TestModule::emit(
        consumer_module,
        r#"
type Right {
    value: int32;
}

external function transform(Right): int32
"#,
        [provider_module],
    );
    let symbol = consumer
        .object
        .functions()
        .iter()
        .next()
        .expect("transform declaration should exist")
        .symbol;
    let strings = TestModule::merge_strings([&provider, &consumer]);
    let error = ProgramLinker::new(
        package,
        vec![
            (consumer.module, consumer.object.clone()),
            (provider.module, provider.object.clone()),
        ],
        &strings,
    )
    .expect_err("different nominal signatures should not link");

    let LinkError::InvalidInput { context, .. } = error else {
        panic!("unexpected link error: {error:?}");
    };

    assert_eq!(
        context,
        format!("function symbol {symbol:?} has conflicting signatures")
    );
}

/// Reject multiple exported definitions for one persistent symbol.
#[test]
fn test_link_objects_rejects_duplicate_definition() {
    let package = PackageId::new(0);
    let first_module = ModuleId::new(package, 0);
    let second_module = ModuleId::new(package, 1);
    let source = r#"
export global answer: int32 = 42

export function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#;
    let first = TestModule::emit(first_module, source, []);
    let second = TestModule::emit(second_module, source, []);
    let symbol = first
        .object
        .functions()
        .iter()
        .next()
        .expect("callee function should exist")
        .symbol;
    let strings = TestModule::merge_strings([&first, &second]);
    let error = ProgramLinker::new(
        package,
        vec![
            (first.module, first.object.clone()),
            (second.module, second.object.clone()),
        ],
        &strings,
    )
    .expect_err("duplicate definitions should fail");

    let LinkError::InvalidInput {
        package: error_package,
        context,
        ..
    } = error
    else {
        panic!("unexpected link error: {error:?}");
    };

    assert_eq!(error_package, package);
    assert_eq!(
        context,
        format!("function symbol {symbol:?} has multiple definitions")
    );
}
