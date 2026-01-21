use destack_mir as mir;

use crate::TestProgram;

/// Lower union layouts into tagged union structs.
#[test]
fn test_lower_union_layout() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Circle {
    value: int32;
}

struct Square {
    value: int32;
}

function takeShape(value: Circle | Square): int32 {
    return 0;
}
"#,
    );

    // lower the module
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.with_mir_tree(module_id, "native", |tree, strings| {
        // build the expected union metadata name
        let union_metadata_name = "test/test:takeShape#parameter:value#union";

        // find the union struct type
        let union_type = test.type_by_metadata_name(tree, strings, &union_metadata_name);

        // resolve the tag and payload field types
        let tag_type = test.expect_struct_field_type_by_name(tree, strings, union_type, "@tag");
        let payload_type =
            test.expect_struct_field_type_by_name(tree, strings, union_type, "@payload");

        // assert the tag field type
        let tag_type = tree.get(tag_type);
        let mir::Type::Int {
            width,
            is_signed: signed,
        } = tag_type
        else {
            panic!("expected integer type for union tag field");
        };
        assert_eq!(*width, 8);
        assert!(!*signed);

        // assert the payload field type
        let payload_type = tree.get(payload_type);
        let mir::Type::Array {
            element, length, ..
        } = payload_type
        else {
            panic!("expected inline payload array for union");
        };
        assert_eq!(*length, 1);
        assert!(matches!(tree.get(*element), mir::Type::Usize));

        let metadata = test.type_metadata(tree, union_type);
        let union_layout = metadata
            .union_layout
            .as_ref()
            .expect("missing union layout metadata");
        assert!(matches!(
            union_layout.payload_kind,
            mir::UnionPayloadKind::Inline
        ));
    });
}

/// Lower large unions into boxed payloads.
#[test]
fn test_lower_union_layout_boxed() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Frame {
    first: int64;
    second: int64;
    third: int64;
}

struct MegaFrame {
    first: int64;
    second: int64;
    third: int64;
    fourth: int64;
}

function takeFrame(value: Frame | MegaFrame): int32 {
    return 0;
}
"#,
    );

    // lower the module
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.with_mir_tree(module_id, "native", |tree, strings| {
        // build the expected union metadata name
        let union_metadata_name = "test/test:takeFrame#parameter:value#union";

        // find the union struct type
        let union_type = test.type_by_metadata_name(tree, strings, &union_metadata_name);

        // assert the payload field is a managed pointer
        let payload_type =
            test.expect_struct_field_type_by_name(tree, strings, union_type, "@payload");
        let payload_type = tree.get(payload_type);
        let mir::Type::Reference {
            kind: mir::ReferenceKind::Managed,
            pointee,
            ..
        } = payload_type
        else {
            panic!("expected managed reference payload for boxed union");
        };
        assert!(matches!(tree.get(*pointee), mir::Type::Void));

        let metadata = test.type_metadata(tree, union_type);
        let union_layout = metadata
            .union_layout
            .as_ref()
            .expect("missing union layout metadata");
        assert!(matches!(
            union_layout.payload_kind,
            mir::UnionPayloadKind::Boxed
        ));
    });
}

/// Lower union upcasts into tagged union payloads.
#[test]
fn test_lower_union_upcast() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Circle {
    value: int32;
}

struct Square {
    value: int32;
}

function makeShape(value: Circle): Circle | Square {
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
function @makeShape(v0: { value: i32 }) -> { @tag: u8, @payload: [usize; 1] } {
block0(v0: { value: i32 }):
    v1 = iconst 0u8
    v2 = stack.alloc [usize; 1] -> ref<raw mut [usize; 1]>
    v3 = iconst 0u64
    v4 = bitcast v3 -> usize
    v5 = array [usize; 1] (v4)
    store v2, v5
    v6 = bitcast v2 -> ref<raw mut { value: i32 }>
    store v6, v0
    v7 = load v2 -> [usize; 1]
    v8 = struct { @tag: u8, @payload: [usize; 1] } (v1, v7)
    return v8
}
        "#,
    );
}

/// Lower boxed union upcasts into managed payload pointers.
#[test]
fn test_lower_union_upcast_boxed() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Frame {
    first: int64;
    second: int64;
    third: int64;
}

struct MegaFrame {
    first: int64;
    second: int64;
    third: int64;
    fourth: int64;
}

function makeFrame(value: Frame): Frame | MegaFrame {
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
function @makeFrame(v0: { first: i64, second: i64, third: i64 }) -> { @tag: u8, @payload: ref<managed void> } {
block0(v0: { first: i64, second: i64, third: i64 }):
    v1 = iconst 0u8
    v2 = managed.alloc { first: i64, second: i64, third: i64 } -> ref<managed { first: i64, second: i64, third: i64 }>
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
fn test_lower_union_downcast() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Circle {
    value: int32;
}

struct Square {
    value: int32;
}

function takeCircle(value: Circle | Square): Circle {
    return value as Circle;
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
function @takeCircle(v0: { @tag: u8, @payload: [usize; 1] }) -> { value: i32 } {
block0(v0: { @tag: u8, @payload: [usize; 1] }):
    v1 = field.get v0, 1
    v2 = stack.alloc [usize; 1] -> ref<raw mut [usize; 1]>
    store v2, v1
    v3 = bitcast v2 -> ref<raw mut { value: i32 }>
    v4 = load v3 -> { value: i32 }
    return v4
}
        "#,
    );
}

/// Lower union tag comparisons into check terminators.
#[test]
fn test_lower_union_tag_check() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function select(value: { kind: 0, value: int32 } | { kind: 1, value: int32 }): int32 {
    return value.kind == 0 ? 1 : 2;
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
function @select(v0: { @tag: u8, @payload: [usize; 1] }) -> i32 {
block0(v0: { @tag: u8, @payload: [usize; 1] }):
    v1 = field.get v0, 0
    v2 = iconst 0u8
    v3 = icmp_eq v1, v2
    check v3, union v1, 0, block1, block2
block1:
    v4 = iconst 1i32
    jump block3(v4)
block2:
    v5 = iconst 2i32
    jump block3(v5)
block3(v6: i32):
    return v6
}
        "#,
    );
}

/// Lower discriminant comparisons to tag checks.
#[test]
fn test_lower_union_integer_discriminant() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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
function @isA(v0: { @tag: u8, @payload: [usize; 1] }) -> bool {
block0(v0: { @tag: u8, @payload: [usize; 1] }):
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
fn test_lower_union_string_discriminant() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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

/// Lower boolean discriminant comparisons to tag checks.
#[test]
fn test_lower_union_boolean_discriminant() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function isReady(value: { kind: true, value: int32 } | { kind: false, value: int32 }): boolean {
    return value.kind == true;
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
function @isReady(v0: { @tag: u8, @payload: [usize; 1] }) -> bool {
block0(v0: { @tag: u8, @payload: [usize; 1] }):
    v1 = field.get v0, 0
    v2 = iconst 1u8
    v3 = icmp_eq v1, v2
    return v3
}
        "#,
    );
}

/// Lower float discriminant comparisons to tag checks.
#[test]
fn test_lower_union_float_discriminant() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function isLarge(value: { kind: 1.5, value: int32 } | { kind: 0.5, value: int32 }): boolean {
    return value.kind == 1.5;
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
function @isLarge(v0: { @tag: u8, @payload: [usize; 2] }) -> bool {
block0(v0: { @tag: u8, @payload: [usize; 2] }):
    v1 = field.get v0, 0
    v2 = iconst 1u8
    v3 = icmp_eq v1, v2
    return v3
}
        "#,
    );
}
