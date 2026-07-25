use destack_artifact::EmitFormat;
use destack_program::BindingId;
use destack_source::{ModuleId, PackageId, TargetId};

use super::super::ProgramLinker;
use super::harness::TestModule;

/// Resolve one binding declaration to its program-defined implementation.
#[test]
fn test_link_objects_resolves_binding_definition() {
    let package = PackageId::new(0);
    let provider_module = ModuleId::new(package, 0);
    let consumer_module = ModuleId::new(package, 1);
    let provider = TestModule::emit(
        provider_module,
        r#"
@binding("runtime.touch")
export function touchImplementation(): int32 {
entry:
    v0: int32 = 42
    return v0
}
"#,
        [],
    );
    let consumer = TestModule::emit(
        consumer_module,
        r#"
@binding("runtime.touch")
external function touchAlias(): int32

export function caller(): int32 {
entry:
    v0: int32 = call touchAlias(): () => int32
    return v0
}
"#,
        [provider_module],
    );
    let strings = TestModule::merge_strings([&provider, &consumer]);
    let linker = ProgramLinker::new(
        package,
        TargetId::new(package, "test"),
        EmitFormat::Bytecode,
        vec![
            (consumer.module, consumer.object.clone()),
            (provider.module, provider.object.clone()),
        ],
        &strings,
    )
    .expect("binding implementation should resolve");
    let imported = linker.function(consumer_module, "touchAlias");
    let defined = linker.function(provider_module, "touchImplementation");
    let function = linker.function_id(provider_module, defined);

    // preserve one callable identity across the binding declaration and definition
    assert_eq!(linker.function_id(consumer_module, imported), function);
    let program = linker.link().expect("binding implementation should link");
    assert_eq!(
        program.function_binding(function),
        Some(BindingId::from_name("runtime.touch"))
    );
    assert!(
        program
            .bytecode()
            .function(program.sections(), function.index())
            .and_then(destack_bytecode::Function::code)
            .is_some()
    );
}
