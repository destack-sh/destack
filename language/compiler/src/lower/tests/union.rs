use destack_mir as mir;

use crate::TestProgram;

/// Lower union layouts into tagged union structs.
#[test]
fn test_lower_union_layout() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude();
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

    // assert the lowered mir
    test.assert_mir(
        module_id,
        "native",
        r#"
type takeShape#parameter:value#union { tag: uint8, payload: usize[1] }

function takeShape(v0: takeShape#parameter:value#union): int32 {
b0(v0: takeShape#parameter:value#union):
    v1: int32 = 0int32
    return v1
}
        "#,
    );

    test.with_mir_tree(module_id, "native", |tree, strings| {
        // build the expected union metadata name
        let union_metadata_name = "test/test:takeShape#parameter:value#union";

        // find the union struct type
        let union_type = test.type_by_metadata_name(tree, strings, union_metadata_name);

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

        let union_layout = test.union_layout(tree, union_type);
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
    let test = TestProgram::memory_sequential_with_prelude();
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

    // assert the lowered mir
    test.assert_mir(
        module_id,
        "native",
        r#"
type takeFrame#parameter:value#union { tag: uint8, payload: ref<void, managed, readonly> }

function takeFrame(v0: takeFrame#parameter:value#union): int32 {
b0(v0: takeFrame#parameter:value#union):
    v1: int32 = 0int32
    return v1
}
        "#,
    );

    test.with_mir_tree(module_id, "native", |tree, strings| {
        // build the expected union metadata name
        let union_metadata_name = "test/test:takeFrame#parameter:value#union";

        // find the union struct type
        let union_type = test.type_by_metadata_name(tree, strings, union_metadata_name);

        // assert the payload field is a managed reference
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

        let union_layout = test.union_layout(tree, union_type);
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
    let test = TestProgram::memory_sequential_with_prelude();
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
type makeShape#return#union { tag: uint8, payload: usize[1] }
type Circle { value: int32 }

function makeShape(v0: Circle): makeShape#return#union {
b0(v0: Circle):
    v1: uint8 = 0uint8
    v2: ref<usize1], raw, addressSpace(stack)> = stack.alloc usize[1]
    v3: uint64 = 0uint64
    v4: usize = cast.bit v3 -> usize
    v5: usize[1] = [v4]
    store v2, v5
    v6: ref<Circle, raw, addressSpace(stack)> = cast.bit v2 -> ref<Circle, raw, addressSpace(stack)>
    store v6, v0
    v7: usize[1] = load v2
    v8: makeShape#return#union = struct makeShape#return#union (v1, v7)
    return v8
}
        "#,
    );
}

/// Lower unions of a single reference type and null into nullable references.
#[test]
fn test_lower_union_nullable_reference() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Circle {
    value: int32 = 0;
}

function acceptNullable(value: Circle | null): Circle | null {
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
type Circle {
    value: int32;
}
function acceptNullable(v0: ref?<Circle, managed, readonly>): ref?<Circle, managed, readonly> {
b0(v0: ref?<Circle, managed, readonly>):
    return v0
}"#,
    );

    test.with_mir_tree(module_id, "native", |tree, strings| {
        // resolve the function signature types
        let function = test.function_by_name(tree, strings, "acceptNullable");
        let parameter_type = function.parameters.first().expect("missing parameter").ty;
        let return_type = function.return_type;

        // assert the parameter is a nullable managed reference
        let mir::Type::Reference {
            kind: parameter_kind,
            is_nullable: parameter_nullable,
            ..
        } = tree.get(parameter_type)
        else {
            panic!("expected nullable reference parameter type");
        };
        assert!(matches!(parameter_kind, mir::ReferenceKind::Managed));
        assert!(*parameter_nullable);

        // assert the return is a nullable managed reference
        let mir::Type::Reference {
            kind: return_kind,
            is_nullable: return_nullable,
            ..
        } = tree.get(return_type)
        else {
            panic!("expected nullable reference return type");
        };
        assert!(matches!(return_kind, mir::ReferenceKind::Managed));
        assert!(*return_nullable);

        // assert the nullable union type has no tagged union metadata
        let parameter_union = "test/test:acceptNullable#parameter:value#union";
        let return_union = "test/test:acceptNullable#return#union";
        let parameter_union_type = test.type_by_metadata_name(tree, strings, parameter_union);
        let return_union_type = test.type_by_metadata_name(tree, strings, return_union);
        assert!(tree.type_table.union_layout(parameter_union_type).is_none());
        assert!(tree.type_table.union_layout(return_union_type).is_none());
    });
}

/// Lower unions with null and undefined into tagged union layouts.
#[test]
fn test_lower_union_null_undefined_tagged() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Circle {
    value: int32 = 0;
}

function acceptUnion(value: Circle | null | undefined): Circle | null | undefined {
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
type Struct0 {
    tag: uint8;
    payload: usize[1];
}
function acceptUnion(v0: Struct0): Struct0 {
b0(v0: Struct0):
    return v0
}"#,
    );

    test.with_mir_tree(module_id, "native", |tree, strings| {
        // resolve the union metadata name for the return type
        let union_metadata_name = "test/test:acceptUnion#return#union";
        let union_type = test.type_by_metadata_name(tree, strings, union_metadata_name);

        // assert union metadata exists for the tagged union
        assert!(tree.type_table.union_layout(union_type).is_some());
    });
}

/// Lower union upcasts for null literals.
#[test]
fn test_lower_union_upcast_null_literal() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Circle {
    value: int32;
}

function makeNull(): Circle | null | undefined {
    return null;
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
type makeNull#return#union {
    tag: uint8;
    payload: usize[1];
}
function makeNull(): makeNull#return#union {
b0:
    v0: uint8 = 1uint8
    v1: uint64 = 0uint64
    v2: usize = cast.bit v1 -> usize
    v3: usize[1] = [v2]
    v4: makeNull#return#union = struct makeNull#return#union (v0, v3)
    return v4
}"#,
    );
}

/// Lower union upcasts for undefined literals.
#[test]
fn test_lower_union_upcast_undefined_literal() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Circle {
    value: int32;
}

function makeUndefined(): Circle | null | undefined {
    return undefined;
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
type makeUndefined#return#union {
    tag: uint8;
    payload: usize[1];
}
function makeUndefined(): makeUndefined#return#union {
b0:
    v0: uint8 = 2uint8
    v1: uint64 = 0uint64
    v2: usize = cast.bit v1 -> usize
    v3: usize[1] = [v2]
    v4: makeUndefined#return#union = struct makeUndefined#return#union (v0, v3)
    return v4
}"#,
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
type makeFrame#return#union {
    tag: uint8;
    payload: ref<void, managed, readonly>;
}
type Frame {
    first: int64;
    second: int64;
    third: int64;
}
function makeFrame(v0: Frame): makeFrame#return#union {
b0(v0: Frame):
    v1: uint8 = 0uint8
    v2: ref<Frame, managed, readonly> = managed.alloc Frame
    store v2, v0
    v3: ref<void, managed, readonly> = cast.bit v2 -> ref<void, managed, readonly>
    v4: makeFrame#return#union = struct makeFrame#return#union (v1, v3)
    return v4
}"#,
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
type takeCircle#parameter:value#union { tag: uint8, payload: usize[1] }
type Circle { value: int32 }

function takeCircle(v0: takeCircle#parameter:value#union): Circle {
b0(v0: takeCircle#parameter:value#union):
    v1: usize[1] = field.get v0, 1
    v2: ref<usize1], raw, addressSpace(stack)> = stack.alloc usize[1]
    store v2, v1
    v3: ref<Circle, raw, addressSpace(stack)> = cast.bit v2 -> ref<Circle, raw, addressSpace(stack)>
    v4: Circle = load v3
    return v4
}
        "#,
    );
}

/// Lower union tag comparisons into check terminators.
#[test]
fn test_lower_union_tag_check() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude();
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
type select#parameter:value#union { tag: uint8, payload: usize[1] }

function select(v0: select#parameter:value#union): int32 {
b0(v0: select#parameter:value#union):
    v1: uint8 = field.get v0, 0
    v2: uint8 = 0uint8
    v3: boolean = int.eq v1, v2
    check unionTag v1, 0 -> b1, b2
b1:
    v4: int32 = 1int32
    jump b3(v4)
b2:
    v5: int32 = 2int32
    jump b3(v5)
b3(v6: int32):
    return v6
}
        "#,
    );
}

/// Lower null literal comparisons to union tag checks.
#[test]
fn test_lower_union_null_literal_comparison() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Circle {
    value: int32;
}

function isNull(value: Circle | null | undefined): boolean {
    return value == null;
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
type isNull#parameter:value#union { tag: uint8, payload: usize[1] }

function isNull(v0: isNull#parameter:value#union): boolean {
b0(v0: isNull#parameter:value#union):
    v1: uint8 = field.get v0, 0
    v2: uint8 = 1uint8
    v3: boolean = int.eq v1, v2
    return v3
}
        "#,
    );
}

/// Lower undefined literal comparisons to union tag checks.
#[test]
fn test_lower_union_undefined_literal_comparison() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Circle {
    value: int32;
}

function isUndefined(value: Circle | null | undefined): boolean {
    return value == undefined;
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
type isUndefined#parameter:value#union { tag: uint8, payload: usize[1] }

function isUndefined(v0: isUndefined#parameter:value#union): boolean {
b0(v0: isUndefined#parameter:value#union):
    v1: uint8 = field.get v0, 0
    v2: uint8 = 2uint8
    v3: boolean = int.eq v1, v2
    return v3
}
        "#,
    );
}

/// Lower integer literal comparisons to union tag checks.
#[test]
fn test_lower_union_integer_literal_comparison() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function isOne(value: 1 | 2): boolean {
    return value == 1;
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
type isOne#parameter:value#union { tag: uint8, payload: usize[1] }

function isOne(v0: isOne#parameter:value#union): boolean {
b0(v0: isOne#parameter:value#union):
    v1: uint8 = field.get v0, 0
    v2: uint8 = 0uint8
    v3: boolean = int.eq v1, v2
    return v3
}
        "#,
    );
}

/// Lower literal comparisons when unions include non-literal elements.
#[test]
fn test_lower_union_literal_comparison_mixed() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function isReady(value: true | { value: int32 }): boolean {
    return value == true;
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
type isReady#parameter:value#union { tag: uint8, payload: usize[1] }

function isReady(v0: isReady#parameter:value#union): boolean {
b0(v0: isReady#parameter:value#union):
    v1: uint8 = field.get v0, 0
    v2: uint8 = 0uint8
    v3: boolean = int.eq v1, v2
    return v3
}
        "#,
    );
}

/// Lower discriminant comparisons to tag checks.
#[test]
fn test_lower_union_integer_discriminant() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude();
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
type isA#parameter:value#union { tag: uint8, payload: usize[1] }

function isA(v0: isA#parameter:value#union): boolean {
b0(v0: isA#parameter:value#union):
    v1: uint8 = field.get v0, 0
    v2: uint8 = 0uint8
    v3: boolean = int.eq v1, v2
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

    let string_alias = test.string_type_alias_definition();
    let string_a_name = test.string_literal_global_name("a");
    let string_b_name = test.string_literal_global_name("b");

    // assert the lowered mir
    let expected = r#"
type isA#parameter:value#union { tag: uint8, payload: usize[2] }
${string_alias}
global ${string_a}: ref<String, managed, readonly>, readonly = "a"
global ${string_b}: ref<String, managed, readonly>, readonly = "b"
function isA(v0: isA#parameter:value#union): boolean {
b0(v0: isA#parameter:value#union):
    v1: uint8 = field.get v0, 0
    v2: uint8 = 0uint8
    v3: boolean = int.eq v1, v2
    return v3
}
        "#;
    let expected = expected.replace("${string_alias}", string_alias);
    let expected = expected.replace("${string_a}", &string_a_name);
    let expected = expected.replace("${string_b}", &string_b_name);
    test.assert_mir(module_id, "native", &expected);
}

/// Lower boolean discriminant comparisons to tag checks.
#[test]
fn test_lower_union_boolean_discriminant() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude();
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
type isReady#parameter:value#union { tag: uint8, payload: usize[1] }

function isReady(v0: isReady#parameter:value#union): boolean {
b0(v0: isReady#parameter:value#union):
    v1: uint8 = field.get v0, 0
    v2: uint8 = 1uint8
    v3: boolean = int.eq v1, v2
    return v3
}
        "#,
    );
}

/// Lower float discriminant comparisons to tag checks.
#[test]
fn test_lower_union_float_discriminant() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude();
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
type isLarge#parameter:value#union { tag: uint8, payload: usize[2] }

function isLarge(v0: isLarge#parameter:value#union): boolean {
b0(v0: isLarge#parameter:value#union):
    v1: uint8 = field.get v0, 0
    v2: uint8 = 1uint8
    v3: boolean = int.eq v1, v2
    return v3
}
        "#,
    );
}
