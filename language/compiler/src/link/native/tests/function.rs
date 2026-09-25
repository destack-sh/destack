use tspp_native as native;
use tspp_program::FrameStateId;
use tspp_source::{ModuleId, PackageId};

use crate::link::tests::TestModule;

/// Link relocatable native blocks into one position-independent Program image.
#[test]
fn test_link_native_object() {
    let package = PackageId::new(0);
    let module = ModuleId::new(package, 0);
    let object = TestModule::emit_native(
        module,
        r#"
export function advance(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = add v0, v0
    poll
    return v1
}
"#,
        [],
    );
    let source = object
        .object
        .native()
        .expect("native emission should produce an object");
    let source_definition = source.definitions()[0]
        .get()
        .expect("native object should define advance");
    let source_body = source.blocks()[source_definition.body.index()];
    let source_entry = source.blocks()[source_definition.entry.index()];
    let program = TestModule::link(package, &[&object]);
    let code = program
        .native()
        .expect("native linking should produce native code");
    let sections = program.sections();
    let image = code.bytes(sections);
    let function = program
        .function_id_by_name("advance")
        .expect("native entry should retain its Program function");
    let function = code
        .function(sections, function.index())
        .expect("native function should retain linked entries");

    // place the typed body and runtime entry directly in the linked load image
    assert_eq!(function.body.bytes.byte_len, source_body.byte_len());
    assert_eq!(function.entry.bytes.byte_len, source_entry.byte_len());
    assert!(!function.body.bytes.bytes(image).is_empty());
    assert!(!function.entry.bytes.bytes(image).is_empty());

    // project the object-local poll map into one canonical Program frame state
    let [frame] = code.map().frames(sections) else {
        panic!("runtime poll should retain one native frame map");
    };
    assert!(program.frame_state(FrameStateId(frame.state)).is_some());
    assert!(frame.return_offset >= function.body.bytes.offset);
    assert!(frame.return_offset <= function.body.bytes.end());
}

/// Link object-local native traps to exact positions in the Program code image.
#[test]
fn test_link_native_traps() {
    let package = PackageId::new(0);
    let module = ModuleId::new(package, 0);
    let object = TestModule::emit_native(
        module,
        r#"
export function divide(v0: int64, v1: int64): int64 {
entry(v0: int64, v1: int64):
    v2: int64 = div v0, v1
    return v2
}
"#,
        [],
    );
    let source = object
        .object
        .native()
        .expect("native emission should produce an object");
    let source_definition = source.definitions()[0]
        .get()
        .expect("native object should define divide");
    let source_traps = source.map().traps(source.sections());
    let program = TestModule::link(package, &[&object]);
    let code = program
        .native()
        .expect("native linking should produce native code");
    let sections = program.sections();
    let function = program
        .function_id_by_name("divide")
        .expect("native entry should retain its Program function");
    let function = code
        .function(sections, function.index())
        .expect("native function should retain linked entries");
    let traps = code.map().traps(sections);

    // preserve each classification while projecting its block-relative offset
    let expected = source_traps
        .iter()
        .map(|trap| {
            assert_eq!(trap.block, source_definition.body);

            native::CodeTrap::new(function.body.bytes.offset + trap.offset, trap.trap)
        })
        .collect::<Vec<_>>();
    assert_eq!(traps, expected);
}
