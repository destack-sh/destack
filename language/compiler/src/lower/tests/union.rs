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
type takeShape.payload#union {
    tag: uint8;
    storage: [usize; 1];
}

function takeShape(v0: takeShape.payload#union): int32 {
entry(v0: takeShape.payload#union):
    v1: int32 = 0
    return v1
}
"#,
    );

    test.with_mir_tree(module_id, "native", |tree, strings| {
        // build the expected union metadata name
        let union_metadata_name = "test/test:takeShape.payload#union";

        // find the union struct type
        let union_type = test.type_by_metadata_name(tree, strings, union_metadata_name);

        // resolve the tag and storage field types
        let tag_type = test.expect_struct_field_type_by_name(tree, strings, union_type, "tag");
        let storage_type =
            test.expect_struct_field_type_by_name(tree, strings, union_type, "storage");

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

        // assert the storage field type
        let storage_type = tree.get(storage_type);
        let mir::Type::Array {
            element, length, ..
        } = storage_type
        else {
            panic!("expected inline storage array for union");
        };
        assert_eq!(*length, 1);
        assert!(matches!(
            tree.get(element.ty().expect("array element should be concrete")),
            mir::Type::Usize
        ));

    });
}

/// Lower large unions into boxed storage.
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
type takeFrame.payload#union {
    tag: uint8;
    storage: ref<void, managed, readonly>;
}

function takeFrame(v0: takeFrame.payload#union): int32 {
entry(v0: takeFrame.payload#union):
    v1: int32 = 0
    return v1
}
"#,
    );

    test.with_mir_tree(module_id, "native", |tree, strings| {
        // build the expected union metadata name
        let union_metadata_name = "test/test:takeFrame.payload#union";

        // find the union struct type
        let union_type = test.type_by_metadata_name(tree, strings, union_metadata_name);

        // assert the storage field is a managed reference
        let storage_type =
            test.expect_struct_field_type_by_name(tree, strings, union_type, "storage");
        let storage_type = tree.get(storage_type);
        let mir::Type::Reference {
            kind: mir::ReferenceKind::Managed,
            pointee,
            ..
        } = storage_type
        else {
            panic!("expected managed reference storage for boxed union");
        };
        assert!(matches!(
            tree.get(pointee.ty().expect("storage pointee should be concrete")),
            mir::Type::Void
        ));
    });
}

/// Lower union upcasts into tagged union values.
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
    storage: [usize; 1];
}

type Circle {
    value: int32;
}

function makeShape(v0: Circle): makeShape.return#union {
entry(v0: Circle):
    v1: uint8 = 0
    v2: ref<[usize; 1], raw, space(frame)> = frame.alloc.zeroed [usize; 1]
    v3: uint64 = 0
    v4: usize = cast.bit v3 -> usize
    v5: [usize; 1] = array [usize; 1] (v4)
    store v2, v5
    v6: ref<Circle, raw, space(frame)> = cast.bit v2 -> ref<Circle, raw, space(frame)>
    store v6, v0
    v7: [usize; 1] = load v2
    v8: makeShape.return#union = struct makeShape.return#union (v1, v7)
    return v8
}
"#,
    );
}

/// Lower unions of a single reference type and null into references that allow null.
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

function acceptNullable(v0: ref<Circle, managed, readonly, nullable>): ref<Circle, managed, readonly, nullable> {
entry(v0: ref<Circle, managed, readonly, nullable>):
    return v0
}
"#,
    );

    test.with_mir_tree(module_id, "native", |tree, _strings| {
        // resolve the function signature types
        let function = test.function_by_name(tree, strings, "acceptNullable");
        let parameter_type = function.parameters.first().expect("missing parameter").ty;
        let return_type = function.return_type;

        // parameter reference
        let mir::Type::Reference {
            kind: parameter_kind,
            nullability: parameter_nullability,
            ..
        } = tree.get(
            parameter_type
                .ty()
                .expect("parameter type should be concrete"),
        )
        else {
            panic!("expected null reference parameter type");
        };
        assert!(matches!(parameter_kind, mir::ReferenceKind::Managed));
        assert_eq!(*parameter_nullability, mir::Nullability::Null);

        // return reference
        let mir::Type::Reference {
            kind: return_kind,
            nullability: return_nullability,
            ..
        } = tree.get(return_type.ty().expect("return type should be concrete"))
        else {
            panic!("expected null reference return type");
        };
        assert!(matches!(return_kind, mir::ReferenceKind::Managed));
        assert_eq!(*return_nullability, mir::Nullability::Null);

    });
}

/// Lower unions with null and undefined into tagged union layouts.
#[test]
fn test_lower_union_null_undefined_tagged() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude();
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
type acceptUnion.payload#union {
    tag: uint8;
    storage: [usize; 1];
}

function acceptUnion(v0: acceptUnion.payload#union): acceptUnion.payload#union {
entry(v0: acceptUnion.payload#union):
    return v0
}
"#,
    );

    test.with_mir_tree(module_id, "native", |tree, strings| {
        // resolve the union metadata names
        let parameter_union_name = "test/test:acceptUnion.payload#union";
        let return_union_name = "test/test:acceptUnion.return#union";
        let parameter_union_type = test.type_by_metadata_name(tree, strings, parameter_union_name);
        let return_union_type = test.type_by_metadata_name(tree, strings, return_union_name);

        // assert the parameter and return layouts use inline storage
        let parameter_storage =
            test.expect_struct_field_type_by_name(tree, strings, parameter_union_type, "storage");
        let return_storage =
            test.expect_struct_field_type_by_name(tree, strings, return_union_type, "storage");
        assert!(matches!(tree.get(parameter_storage), mir::Type::Array { .. }));
        assert!(matches!(tree.get(return_storage), mir::Type::Array { .. }));
    });
}

/// Lower union upcasts for null literals.
#[test]
fn test_lower_union_upcast_null_literal() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude();
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
    storage: [usize; 1];
}

function makeNull(): makeNull.return#union {
entry:
    v0: uint8 = 1
    v1: uint64 = 0
    v2: usize = cast.bit v1 -> usize
    v3: [usize; 1] = array [usize; 1] (v2)
    v4: makeNull.return#union = struct makeNull.return#union (v0, v3)
    return v4
}
"#,
    );
}

/// Lower union upcasts for undefined literals.
#[test]
fn test_lower_union_upcast_undefined_literal() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude();
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
    storage: [usize; 1];
}

function makeUndefined(): makeUndefined.return#union {
entry:
    v0: uint8 = 2
    v1: uint64 = 0
    v2: usize = cast.bit v1 -> usize
    v3: [usize; 1] = array [usize; 1] (v2)
    v4: makeUndefined.return#union = struct makeUndefined.return#union (v0, v3)
    return v4
}
"#,
    );
}

/// Lower boxed union upcasts into managed storage pointers.
#[test]
fn test_lower_union_upcast_boxed() {
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
    storage: ref<void, managed, readonly>;
}

function makeFrame(v0: Frame): makeFrame.return#union {
entry(v0: Frame):
    v1: uint8 = 0
    v2: ref<Frame, managed, readonly> = new.zeroed Frame
    store v2, v0
    v3: ref<void, managed, readonly> = cast.bit v2 -> ref<void, managed, readonly>
    v4: makeFrame.return#union = struct makeFrame.return#union (v1, v3)
    return v4
}
"#,
    );
}

/// Lower union downcasts into storage loads.
#[test]
fn test_lower_union_downcast() {
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
type takeCircle.payload#union {
    tag: uint8;
    storage: [usize; 1];
}

type Circle {
    value: int32;
}

function takeCircle(v0: takeCircle.payload#union): Circle {
entry(v0: takeCircle.payload#union):
    v1: [usize; 1] = field.get v0, 1
    v2: ref<[usize; 1], raw, space(frame)> = frame.alloc.zeroed [usize; 1]
    store v2, v1
    v3: ref<Circle, raw, space(frame)> = cast.bit v2 -> ref<Circle, raw, space(frame)>
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
type select.payload#union {
    tag: uint8;
    storage: [usize; 1];
}

function select(v0: select.payload#union): int32 {
entry(v0: select.payload#union):
    v1: uint8 = field.get v0, 0
    v2: uint8 = 0
    check variant.tag v1, 0uint8 => b1, b2

b1:
    v3: int32 = 1
    jump b3(v3)

b2:
    v4: int32 = 2
    jump b3(v4)

b3(v5: int32):
    return v5
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
type isNull.payload#union {
    tag: uint8;
    storage: [usize; 1];
}

function isNull(v0: isNull.payload#union): boolean {
entry(v0: isNull.payload#union):
    v1: uint8 = field.get v0, 0
    v2: uint8 = 1
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
type isUndefined.payload#union {
    tag: uint8;
    storage: [usize; 1];
}

function isUndefined(v0: isUndefined.payload#union): boolean {
entry(v0: isUndefined.payload#union):
    v1: uint8 = field.get v0, 0
    v2: uint8 = 2
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
type isOne.payload#union {
    tag: uint8;
    storage: [usize; 1];
}

function isOne(v0: isOne.payload#union): boolean {
entry(v0: isOne.payload#union):
    v1: uint8 = field.get v0, 0
    v2: uint8 = 0
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
type isReady.payload#union {
    tag: uint8;
    storage: [usize; 1];
}

function isReady(v0: isReady.payload#union): boolean {
entry(v0: isReady.payload#union):
    v1: uint8 = field.get v0, 0
    v2: uint8 = 0
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
type isA.payload#union {
    tag: uint8;
    storage: [usize; 1];
}

function isA(v0: isA.payload#union): boolean {
entry(v0: isA.payload#union):
    v1: uint8 = field.get v0, 0
    v2: uint8 = 0
    v3: boolean = int.eq v1, v2
    return v3
}
"#,
    );
}

/// Lower string discriminant comparisons to tag checks.
#[test]
fn test_lower_union_string_discriminant() {
    let test = TestProgram::memory_sequential_with_prelude();
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
type isA.payload#union { tag: uint8, storage: [usize; 2] }
readonly global ${string_a}: ref<String, managed, readonly> = "a"
function isA(v0: isA.payload#union): boolean {
entry(v0: isA.payload#union):
    v1: uint8 = field.get v0, 0
    v2: uint8 = 0uint8
    v3: boolean = int.eq v1, v2
    return v3
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
type isReady.payload#union {
    tag: uint8;
    storage: [usize; 1];
}

function isReady(v0: isReady.payload#union): boolean {
entry(v0: isReady.payload#union):
    v1: uint8 = field.get v0, 0
    v2: uint8 = 1
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
type isLarge.payload#union {
    tag: uint8;
    storage: [usize; 2];
}

function isLarge(v0: isLarge.payload#union): boolean {
entry(v0: isLarge.payload#union):
    v1: uint8 = field.get v0, 0
    v2: uint8 = 1
    v3: boolean = int.eq v1, v2
    return v3
}
"#,
    );
}
