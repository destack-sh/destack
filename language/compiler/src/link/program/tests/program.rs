use destack_artifact::EmitFormat;
use destack_bytecode::Opcode;
use destack_source::{ModuleId, PackageId, TargetId};

use super::super::ProgramLinker;
use super::harness::TestModule;

/// Generate one Program from multiple module objects.
#[test]
fn test_link_objects_builds_program() {
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
    v0: ref<int32, raw, mutable, space(static)> = global.address answer
    v1: int32 = load v0
    return v1
}
"#,
        [provider_module],
    );
    let strings = TestModule::merge_strings([&provider, &consumer]);
    let unused = strings.intern("unused repository string");
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
    .expect("objects should link");

    // retain call signatures for linking without assigning runtime identity
    let consumer = linker.object(consumer_module);
    let signature = consumer
        .calls()
        .first()
        .expect("caller site should exist")
        .signature;
    assert!(consumer.layouts().layout_id(signature).is_none());
    assert!(!linker.has_type(consumer_module, signature));

    let program = linker.link().expect("program should build");

    // retain every defined body under its final Program function id
    let caller = program
        .function_id_by_name("caller")
        .expect("caller export should exist");
    let callee = program
        .function_id_by_name("callee")
        .expect("callee export should exist");
    let read_answer = program
        .function_id_by_name("readAnswer")
        .expect("global reader export should exist");
    let code = program.bytecode();
    let sections = program.sections();
    assert!(code.function(sections, caller.index()).is_some());
    assert!(code.function(sections, callee.index()).is_some());
    assert!(code.function(sections, read_answer.index()).is_some());

    // preserve the logical call point through object emission and program linking
    let call_site = program
        .sites()
        .calls(sections)
        .iter()
        .find(|site| site.point.function == caller)
        .expect("caller site should exist");
    let instruction = code
        .operation(sections, caller.index(), call_site.point.operation)
        .expect("linked instruction should decode")
        .expect("call point should name an instruction");
    assert_eq!(instruction.opcode(), Opcode::CALL);
    assert_eq!(call_site.target.get(), Some(callee));

    // omit repository strings that no linked table references
    assert_eq!(program.string(unused), None);
}
