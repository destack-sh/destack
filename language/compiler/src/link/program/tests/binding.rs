use tspp_program::{BindingAffinity, BindingEffect, BindingId, BindingProvider, BindingReplay};
use tspp_source::{ModuleId, PackageId};

use crate::ProgramLinker;
use crate::link::tests::TestModule;

/// Resolve one binding declaration to its program-defined implementation.
#[test]
fn test_link_objects_resolves_binding_definition() {
    let package = PackageId::new(0);
    let provider_module = ModuleId::new(package, 0);
    let consumer_module = ModuleId::new(package, 1);
    let provider = TestModule::emit(
        provider_module,
        r#"
@binding("runtime.touch", { provider: "runtime", effect: "deterministic", replay: "forbidden", affinity: "worker", requires: ["runtime.debug.read"], platforms: ["linux"], families: ["unix"], hosts: ["native"] })
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
@binding("runtime.touch", { provider: "runtime", effect: "deterministic", replay: "forbidden", affinity: "worker", requires: ["runtime.debug.read"], platforms: ["linux"], families: ["unix"], hosts: ["native"] })
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
    let binding = program
        .function_binding(function)
        .expect("linked function should retain its binding");
    assert_eq!(binding.id, BindingId::from_name("runtime.touch"));
    assert_eq!(binding.function, function);
    assert!(!binding.is_imported());
    assert_eq!(binding.provider, BindingProvider::Runtime);
    assert_eq!(binding.effect, BindingEffect::Deterministic);
    assert_eq!(binding.replay, BindingReplay::Forbidden);
    assert_eq!(binding.affinity, BindingAffinity::Worker);
    assert_eq!(
        program.binding_requires(binding),
        &[strings.intern("runtime.debug.read")]
    );
    assert_eq!(
        program.binding_platforms(binding),
        &[strings.intern("linux")]
    );
    assert_eq!(program.binding_families(binding), &[strings.intern("unix")]);
    assert_eq!(program.binding_hosts(binding), &[strings.intern("native")]);
    assert!(
        program
            .bytecode()
            .function(program.sections(), function.index())
            .and_then(tspp_bytecode::Function::code)
            .is_some()
    );
}
