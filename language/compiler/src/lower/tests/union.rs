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
type takeShape.value#union { tag: uint8, payload: usize[1] }

function takeShape(value0: takeShape.value#union): int32 {
entry0(value0: takeShape.value#union):
    value1: int32 = 0int32
    return value1
}
        "#,
    );

    test.with_mir_tree(module_id, "native", |tree, strings| {
        // build the expected union metadata name
        let union_metadata_name = "test/test:takeShape.value#union";

        // find the union struct type
        let union_type = test.type_by_metadata_name(tree, strings, union_metadata_name);

        // resolve the tag and payload field types
        let tag_type = test.expect_struct_field_type_by_name(tree, strings, union_type, "tag");
        let payload_type =
            test.expect_struct_field_type_by_name(tree, strings, union_type, "payload");

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
        assert!(matches!(
            tree.get(element.ty().expect("array element should be concrete")),
            mir::Type::Usize
        ));

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
type takeFrame.value#union { tag: uint8, payload: ref<void, managed, readonly> }

function takeFrame(value0: takeFrame.value#union): int32 {
entry0(value0: takeFrame.value#union):
    value1: int32 = 0int32
    return value1
}
        "#,
    );

    test.with_mir_tree(module_id, "native", |tree, strings| {
        // build the expected union metadata name
        let union_metadata_name = "test/test:takeFrame.value#union";

        // find the union struct type
        let union_type = test.type_by_metadata_name(tree, strings, union_metadata_name);

        // assert the payload field is a managed reference
        let payload_type =
            test.expect_struct_field_type_by_name(tree, strings, union_type, "payload");
        let payload_type = tree.get(payload_type);
        let mir::Type::Reference {
            kind: mir::ReferenceKind::Managed,
            pointee,
            ..
        } = payload_type
        else {
            panic!("expected managed reference payload for boxed union");
        };
        assert!(matches!(
            tree.get(pointee.ty().expect("payload pointee should be concrete")),
            mir::Type::Void
        ));

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
type makeShape.return#union {
    tag: uint8;
    payload: usize[1];
}
type Circle {
    value: int32;
}

function makeShape(value0: Circle): makeShape.return#union {
entry0(value0: Circle):
    value1: uint8 = 0uint8
    value2: ref<usize[1], raw, space(stack)> = stack.alloc usize[1]
    value3: uint64 = 0uint64
    value4: usize = cast.bit value3 -> usize
    value5: usize[1] = array usize[1] (value4)
    store value2, value5
    value6: ref<Circle, raw, space(stack)> = cast.bit value2 -> ref<Circle, raw, space(stack)>
    store value6, value0
    value7: usize[1] = load value2
    value8: makeShape.return#union = struct makeShape.return#union (value1, value7)
    return value8
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

function acceptNullable(value0: ref?<Circle, managed, readonly>): ref?<Circle, managed, readonly> {
entry0(value0: ref?<Circle, managed, readonly>):
    return value0
}
"#,
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
        } = tree.get(
            parameter_type
                .ty()
                .expect("parameter type should be concrete"),
        )
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
        } = tree.get(return_type.ty().expect("return type should be concrete"))
        else {
            panic!("expected nullable reference return type");
        };
        assert!(matches!(return_kind, mir::ReferenceKind::Managed));
        assert!(*return_nullable);

        // assert the nullable union type has no tagged union metadata
        let parameter_union = "test/test:acceptNullable.value#union";
        let return_union = "test/test:acceptNullable.return#union";
        let parameter_union_type = test.type_by_metadata_name(tree, strings, parameter_union);
        let return_union_type = test.type_by_metadata_name(tree, strings, return_union);
        assert!(
            tree.metadata
                .layout
                .union_layout(parameter_union_type)
                .is_none()
        );
        assert!(
            tree.metadata
                .layout
                .union_layout(return_union_type)
                .is_none()
        );
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
type acceptUnion.value#union {
    tag: uint8;
    payload: usize[1];
}

function acceptUnion(value0: acceptUnion.value#union): acceptUnion.value#union {
entry0(value0: acceptUnion.value#union):
    return value0
}"#,
    );

    test.with_mir_tree(module_id, "native", |tree, strings| {
        // resolve the union metadata names
        let parameter_union_name = "test/test:acceptUnion.value#union";
        let return_union_name = "test/test:acceptUnion.return#union";
        let parameter_union_type = test.type_by_metadata_name(tree, strings, parameter_union_name);
        let return_union_type = test.type_by_metadata_name(tree, strings, return_union_name);

        // assert the parameter and return layouts are tagged unions
        let parameter_layout = test.union_layout(tree, parameter_union_type);
        let return_layout = test.union_layout(tree, return_union_type);
        assert!(matches!(
            parameter_layout.payload_kind,
            mir::UnionPayloadKind::Inline
        ));
        assert!(matches!(
            return_layout.payload_kind,
            mir::UnionPayloadKind::Inline
        ));
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
type makeNull.return#union {
    tag: uint8;
    payload: usize[1];
}

function makeNull(): makeNull.return#union {
entry0:
    value0: uint8 = 1uint8
    value1: uint64 = 0uint64
    value2: usize = cast.bit value1 -> usize
    value3: usize[1] = array usize[1] (value2)
    value4: makeNull.return#union = struct makeNull.return#union (value0, value3)
    return value4
}
"#,
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
type makeUndefined.return#union {
    tag: uint8;
    payload: usize[1];
}

function makeUndefined(): makeUndefined.return#union {
entry0:
    value0: uint8 = 2uint8
    value1: uint64 = 0uint64
    value2: usize = cast.bit value1 -> usize
    value3: usize[1] = array usize[1] (value2)
    value4: makeUndefined.return#union = struct makeUndefined.return#union (value0, value3)
    return value4
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
type Frame {
    first: int64;
    second: int64;
    third: int64;
}
type makeFrame.return#union {
    tag: uint8;
    payload: ref<void, managed, readonly>;
}

function makeFrame(value0: Frame): makeFrame.return#union {
entry0(value0: Frame):
    value1: uint8 = 0uint8
    value2: ref<Frame, managed, readonly> = new Frame
    store value2, value0
    value3: ref<void, managed, readonly> = cast.bit value2 -> ref<void, managed, readonly>
    value4: makeFrame.return#union = struct makeFrame.return#union (value1, value3)
    return value4
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
type takeCircle.value#union { tag: uint8, payload: usize[1] }
type Circle { value: int32 }

function takeCircle(value0: takeCircle.value#union): Circle {
entry0(value0: takeCircle.value#union):
    value1: usize[1] = field.get value0, 1
    value2: ref<usize[1], raw, space(stack)> = stack.alloc usize[1]
    store value2, value1
    value3: ref<Circle, raw, space(stack)> = cast.bit value2 -> ref<Circle, raw, space(stack)>
    value4: Circle = load value3
    return value4
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
type select.value#union { tag: uint8, payload: usize[1] }

function select(value0: select.value#union): int32 {
entry0(value0: select.value#union):
    value1: uint8 = field.get value0, 0
    value2: uint8 = 0uint8
    check unionTag value1, 0 -> block1, block2
block1:
    value3: int32 = 1int32
    jump block3(value3)
block2:
    value4: int32 = 2int32
    jump block3(value4)
block3(value5: int32):
    return value5
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
type isNull.value#union { tag: uint8, payload: usize[1] }

function isNull(value0: isNull.value#union): boolean {
entry0(value0: isNull.value#union):
    value1: uint8 = field.get value0, 0
    value2: uint8 = 1uint8
    value3: boolean = int.eq value1, value2
    return value3
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
type isUndefined.value#union { tag: uint8, payload: usize[1] }

function isUndefined(value0: isUndefined.value#union): boolean {
entry0(value0: isUndefined.value#union):
    value1: uint8 = field.get value0, 0
    value2: uint8 = 2uint8
    value3: boolean = int.eq value1, value2
    return value3
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
type isOne.value#union { tag: uint8, payload: usize[1] }

function isOne(value0: isOne.value#union): boolean {
entry0(value0: isOne.value#union):
    value1: uint8 = field.get value0, 0
    value2: uint8 = 0uint8
    value3: boolean = int.eq value1, value2
    return value3
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
type isReady.value#union { tag: uint8, payload: usize[1] }

function isReady(value0: isReady.value#union): boolean {
entry0(value0: isReady.value#union):
    value1: uint8 = field.get value0, 0
    value2: uint8 = 0uint8
    value3: boolean = int.eq value1, value2
    return value3
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
type isA.value#union { tag: uint8, payload: usize[1] }

function isA(value0: isA.value#union): boolean {
entry0(value0: isA.value#union):
    value1: uint8 = field.get value0, 0
    value2: uint8 = 0uint8
    value3: boolean = int.eq value1, value2
    return value3
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
    // assert the lowered mir
    let expected = r#"
${string_alias}
type isA.value#union { tag: uint8, payload: usize[2] }
global ${string_a}: ref<String, managed, readonly>, readonly = "a"
function isA(value0: isA.value#union): boolean {
entry0(value0: isA.value#union):
    value1: uint8 = field.get value0, 0
    value2: uint8 = 0uint8
    value3: boolean = int.eq value1, value2
    return value3
}
        "#;
    let expected = expected.replace("${string_alias}", string_alias);
    let expected = expected.replace("${string_a}", &string_a_name);
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
type isReady.value#union { tag: uint8, payload: usize[1] }

function isReady(value0: isReady.value#union): boolean {
entry0(value0: isReady.value#union):
    value1: uint8 = field.get value0, 0
    value2: uint8 = 1uint8
    value3: boolean = int.eq value1, value2
    return value3
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
type isLarge.value#union { tag: uint8, payload: usize[2] }

function isLarge(value0: isLarge.value#union): boolean {
entry0(value0: isLarge.value#union):
    value1: uint8 = field.get value0, 0
    value2: uint8 = 1uint8
    value3: boolean = int.eq value1, value2
    return value3
}
        "#,
    );
}
