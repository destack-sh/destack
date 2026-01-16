use destack_mir::CallDispatchKind;

use crate::TestProgram;

/// Lower interface itab metadata for structs.
#[test]
fn test_struct_itab_metadata() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Drawable {
    color: int32;
    draw(): int32;
}

struct Circle implements Drawable {
    color: int32;
    radius: int32;

    draw(): int32 { return this.color; }
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.with_mir_tree(module_id, "native", |tree, strings| {
        // collect interface dispatch tables
        let itab = test.expect_single_interface_table(tree);

        // assert the itab slot layout
        assert!(matches!(itab.slots[0], destack_mir::DispatchSlot::TypeTag));

        // assert the field offset slot
        let offset = test
            .interface_field_offset(itab, strings, "color")
            .expect("missing color field offset");
        assert_eq!(offset, 0);

        // assert the interface method slot
        let target_name = test
            .interface_method_target_name(itab, tree, strings, "draw")
            .expect("missing draw interface method");
        assert_eq!(target_name, "draw");
    });
}

/// Lower interface call metadata for interface dispatch.
#[test]
fn test_interface_call_metadata() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Drawable {
    color: int32;
    draw(): int32;
}

struct Circle implements Drawable {
    color: int32;
    radius: int32;

    draw(): int32 { return this.color; }
}

function useDrawable(d: Drawable): int32 {
    return d.draw();
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.with_mir_tree(module_id, "native", |tree, strings| {
        // locate the interface call metadata
        let metadata = test
            .find_interface_call_metadata_by_name(tree, strings, "useDrawable")
            .expect("missing interface call metadata");

        // assert the dispatch payload
        let CallDispatchKind::Interface { slot_id } = metadata.dispatch else {
            panic!("expected interface dispatch metadata");
        };
        assert_eq!(slot_id, 2);
        assert!(metadata.receiver.is_some());
        assert!(metadata.declaring_type.is_some());
    });
}
