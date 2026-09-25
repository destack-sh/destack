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
shared global answerReference: ref<int32, borrowed, 'static, readonly> = globalAddress answer
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
    let (constants, shared) = program
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

    // resolve the constant address the linked reference should carry
    let answer = constants
        .reference(answer)
        .offset()
        .expect("constant address should be concrete");

    assert_eq!(reference as usize, answer);
}

/// Link and materialize one string value constant into its header and payload bytes.
#[test]
fn test_link_string_value_constant() {
    let package = PackageId::new(0);
    let module = ModuleId::new(package, 0);
    let emitted = TestModule::emit(
        module,
        r#"
@languageItem("string.String")
type String {
    codeUnits: slice<uint16, managed, mutable, local>;
}

constant string.0: String = "hi"
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
    let literal = linker.global_id(module, linker.global(module, "string.0"));
    let program = linker.link().expect("program should link");
    let memory =
        Arc::new(MemoryMap::reserve(64 * 1024, 8 * 1024).expect("world memory should reserve"));

    // materialize all runtime storage and read the string header words
    let (constants, _shared) = program
        .materialize_runtime_statics(memory.clone())
        .expect("runtime statics should materialize");
    let literal = program.global(literal).expect("string global should exist");
    let address = constants.reference(literal);
    let address = address.offset().expect("header address should be concrete");
    let bytes = memory
        .read_bytes(address, 16)
        .expect("header words should read");
    let units_address = u64::from_le_bytes(bytes[0..8].try_into().expect("address word"));
    let length = u64::from_le_bytes(bytes[8..16].try_into().expect("length word"));

    // read the UTF-16 code units at the relocated payload address
    assert_eq!(length, 2);
    let units = memory
        .read_bytes(units_address as usize, 4)
        .expect("payload units should read");
    assert_eq!(units, [b'h', 0, b'i', 0]);
}
