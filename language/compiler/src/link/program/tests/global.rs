use std::sync::Arc;

use destack_memory::MemoryMap;
use destack_program::GlobalAddress;
use destack_source::{ModuleId, PackageId};

use crate::ProgramLinker;
use crate::link::tests::TestModule;

/// Link and materialize one cross-space global address initializer.
#[test]
fn test_link_static_global_address() {
    let package = PackageId::new(0);
    let module = ModuleId::new(package, 0);
    let emitted = TestModule::emit(
        module,
        r#"
constant answer: int32 = 42
shared global answerReference: ref<int32, borrowed, readonly, constant> = globalAddress answer
"#,
        [],
    );
    let strings = TestModule::merge_strings([&emitted]);
    let linker = ProgramLinker::new(
        package,
        vec![(emitted.module, emitted.object.clone())],
        &strings,
    )
    .expect("program linker should initialize");
    let answer = linker.global_id(module, linker.global(module, "answer"));
    let answer_reference = linker.global_id(module, linker.global(module, "answerReference"));
    let program = linker.link().expect("program should link");
    let memory =
        Arc::new(MemoryMap::reserve(64 * 1024, 8 * 1024).expect("world memory should reserve"));

    // materialize all runtime storage and read the linked address word
    let (constants, _immortals, shared) = program
        .materialize_runtime_statics(memory.clone())
        .expect("runtime statics should materialize");
    let answer = program.global(answer).expect("answer global should exist");
    let answer_reference = program
        .global(answer_reference)
        .expect("answer reference global should exist");
    let address = shared.reference(answer_reference);
    let address = address.offset().expect("shared address should be concrete");
    let bytes = memory
        .read_bytes(address, GlobalAddress::BYTE_LEN)
        .expect("linked address should read");
    let reference = u64::from_le_bytes(bytes.try_into().expect("address word should be complete"));

    let answer = constants
        .reference(answer)
        .offset()
        .expect("constant address should be concrete");

    assert_eq!(reference as usize, answer);
}
