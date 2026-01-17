use destack_mir::{self as mir, CallDispatchKind};

use crate::TestProgram;

/// Lower interface itab metadata for structs.
#[test]
fn test_struct_itab_metadata() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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

/// Lower interface reference layouts into fat pointer structs.
#[test]
fn test_lower_interface_reference_layout() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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

    // lower the module
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.with_mir_tree(module_id, "native", |tree, strings| {
        // find the interface reference struct type
        let interface_type = test
            .find_struct_type_by_field_names(tree, strings, &["@object", "@itab"])
            .expect("missing interface reference type");

        // resolve the object and itab field types
        let object_type = test
            .struct_field_type_by_name(tree, strings, interface_type, "@object")
            .expect("missing interface object field");
        let itab_type = test
            .struct_field_type_by_name(tree, strings, interface_type, "@itab")
            .expect("missing interface itab field");

        // assert the object pointer field type
        let object_type = tree.get(object_type);
        let mir::Type::Reference { kind, pointee, .. } = object_type else {
            panic!("expected managed reference for interface object field");
        };
        assert!(matches!(kind, mir::ReferenceKind::Managed));
        assert!(matches!(tree.get(*pointee), mir::Type::Void));

        // assert the itab field type
        let itab_type = tree.get(itab_type);
        assert!(matches!(itab_type, mir::Type::Usize));
    });
}

/// Lower interface call metadata for interface dispatch.
#[test]
fn test_interface_call_metadata() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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
