use destack_mir as mir;

use crate::TestProgram;

/// Lower union layouts into tagged boxed structs.
#[test]
fn test_lower_union_layout() {
    // set up the test program
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct A {
    value: int32;
}

struct B {
    value: int32;
}

function takeUnion(value: A | B): int32 {
    return 0;
}
"#,
    );

    // lower the module
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.with_mir_tree(module_id, "native", |tree, strings| {
        // find the union struct type
        let union_type = test
            .find_struct_type_by_metadata_prefix(tree, strings, &format!("@union:{module_id}:"))
            .expect("missing union layout type");

        // resolve the tag and payload field types
        let tag_type = test
            .struct_field_type_by_name(tree, strings, union_type, "@tag")
            .expect("missing union tag field");
        let payload_type = test
            .struct_field_type_by_name(tree, strings, union_type, "@payload")
            .expect("missing union payload field");

        // assert the tag field type
        let tag_type = tree.get(tag_type);
        let mir::Type::Int { width, signed } = tag_type else {
            panic!("expected integer type for union tag field");
        };
        assert_eq!(*width, 8);
        assert!(!*signed);

        // assert the payload field type
        let payload_type = tree.get(payload_type);
        let mir::Type::Reference { kind, pointee, .. } = payload_type else {
            panic!("expected managed reference for union payload field");
        };
        assert!(matches!(kind, mir::ReferenceKind::Managed));
        assert!(matches!(tree.get(*pointee), mir::Type::Void));
    });
}

/// Lower union upcasts into tagged boxed payloads.
#[test]
fn test_lower_union_upcast_mir() {
    // set up the test program
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct A {
    value: int32;
}

struct B {
    value: int32;
}

function makeUnion(value: A): A | B {
    return value;
}
"#,
    );

    // lower the module
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // assert the lowered mir
    test.assert_mir(
        module_id,
        "native",
        r#"
function @makeUnion(v0: { value: i32 }) -> { @tag: u8, @payload: ref<managed void> } {
block0(v0: { value: i32 }):
    v1 = iconst 0u8
    v2 = managed.alloc { value: i32 } -> ref<managed { value: i32 }>
    store v2, v0
    v3 = bitcast v2 -> ref<managed void>
    v4 = struct { @tag: u8, @payload: ref<managed void> } (v1, v3)
    return v4
}
        "#,
    );
}

/// Lower union downcasts into payload loads.
#[test]
fn test_lower_union_downcast_mir() {
    // set up the test program
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct A {
    value: int32;
}

struct B {
    value: int32;
}

function takeA(value: A | B): A {
    return value as A;
}
"#,
    );

    // lower the module
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // assert the lowered mir
    test.assert_mir(
        module_id,
        "native",
        r#"
function @takeA(v0: { @tag: u8, @payload: ref<managed void> }) -> { value: i32 } {
block0(v0: { @tag: u8, @payload: ref<managed void> }):
    v1 = field.get v0, 1
    v2 = bitcast v1 -> ref<managed { value: i32 }>
    v3 = load v2 -> { value: i32 }
    return v3
}
        "#,
    );
}

/// Lower discriminant comparisons to tag checks.
#[test]
fn test_lower_union_discriminant_compare_mir() {
    // set up the test program
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function isA(value: { kind: 1, value: int32 } | { kind: 0, value: int32 }): boolean {
    return value.kind == 0;
}
"#,
    );

    // lower the module
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // assert the lowered mir
    test.assert_mir(
        module_id,
        "native",
        r#"
function @isA(v0: { @tag: u8, @payload: ref<managed void> }) -> bool {
block0(v0: { @tag: u8, @payload: ref<managed void> }):
    v1 = field.get v0, 0
    v2 = iconst 0u8
    v3 = icmp_eq v1, v2
    return v3
}
        "#,
    );
}

/// Lower string discriminant comparisons to tag checks.
#[test]
#[ignore] // FUGU #Incomplete: native string type
fn test_lower_union_discriminant_compare_string_mir() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function isA(value: { kind: "b", value: int32 } | { kind: "a", value: int32 }): boolean {
    return value.kind == "a";
}
"#,
    );

    // lower the module
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // assert the lowered mir
    test.assert_mir(
        module_id,
        "native",
        r#"
function @isA(v0: { @tag: u8, @payload: ref<managed void> }) -> bool {
block0(v0: { @tag: u8, @payload: ref<managed void> }):
    v1 = field.get v0, 0
    v2 = iconst 0u8
    v3 = icmp_eq v1, v2
    return v3
}
        "#,
    );
}
