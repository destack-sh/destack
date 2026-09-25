use tspp_program::BindingId;
use tspp_source::{ModuleId, PackageId};

use crate::link::tests::TestModule;

/// Route one imported binding through linked native code and Program metadata.
#[test]
fn test_link_native_binding() {
    let package = PackageId::new(0);
    let module = ModuleId::new(package, 0);
    let object = TestModule::emit_native(
        module,
        r#"
@binding("runtime.touch", { provider: "runtime", effect: "deterministic", replay: "forbidden", affinity: "worker" })
external function touch(): int32

export function caller(): int32 {
entry:
    v0: int32 = call touch(): () => int32
    return v0
}
"#,
        [],
    );
    let program = TestModule::link(package, &[&object]);
    let caller = program
        .function_id_by_name("caller")
        .expect("native caller should retain its Program function");
    let binding_id = BindingId::from_name("runtime.touch");
    let binding = program
        .binding(binding_id)
        .expect("native binding should retain its declaration");
    let code = program
        .native()
        .expect("native linking should produce native code");

    // retain the imported runtime identity without requiring a native definition
    assert_eq!(binding.id, binding_id);
    assert!(binding.is_imported());
    assert!(
        code.function(program.sections(), caller.index()).is_some(),
        "native caller should retain linked code",
    );
}
