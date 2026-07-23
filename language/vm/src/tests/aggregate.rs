use destack_core::Optional;
use destack_mir::{Discriminant, DiscriminantField, VariantEncoding};
use destack_program::{
    ElementLayout, LayoutField, LayoutShapeBuilder, ScalarFormat, TypeId, VariantCaseLayout,
    VariantLayoutBuilder, Word,
};

use super::{TestMachine, TestProgram};

/// Execute packed aggregate, element, and variant operations through Program layouts.
#[test]
fn test_execute_aggregate_operations() {
    let scalar = LayoutShapeBuilder::Scalar(ScalarFormat::int(32, true));
    let fields = vec![
        LayoutField {
            name: Optional::none(),
            ty: TypeId(3),
            offset: 0,
            size: 4,
            alignment: 4,
        },
        LayoutField {
            name: Optional::none(),
            ty: TypeId(3),
            offset: 8,
            size: 4,
            alignment: 4,
        },
    ];
    let array = ElementLayout {
        element: TypeId(3),
        stride: 8,
        count: 2,
    };
    let variant = VariantLayoutBuilder::new(
        TypeId(3),
        TypeId(2),
        VariantEncoding::Direct {
            field: DiscriminantField::scalar(0, 4),
        },
    )
    .cases([
        VariantCaseLayout {
            discriminant: Discriminant::from_bits(3),
            ty: TypeId(3),
            payload_offset: 8,
        },
        VariantCaseLayout {
            discriminant: Discriminant::from_bits(7),
            ty: TypeId(3),
            payload_offset: 8,
        },
    ]);
    let program = TestProgram::new()
        .layout(0, LayoutShapeBuilder::Struct(fields), 16, 8)
        .layout(1, LayoutShapeBuilder::Array(array), 16, 8)
        .layout(2, LayoutShapeBuilder::Variant(variant), 16, 8)
        .layout(3, scalar, 4, 4);
    let mut machine = TestMachine::parse(
        r#"
type Pair
type Values
type Choice
type Int32

export function transform(r0: int32, r1: int32): (int32, int32, uint32, int32) {
    r2: words<2> = aggregate Pair (r0, r1)
    r4: words<2> = field.set r2, Pair, 1, r0
    r6: int32 = field.get r4, Pair, 1
    r7: words<2> = element.set r2, Values, 1, r0
    r9: int32 = element.get r7, Values, 1
    r10: words<2> = variant.new Choice, 1, r1
    r12: uint32 = variant.tag r10, Choice
    r13: int32 = variant.payload r10, Choice, 1
    r14: int32 = move r6
    r15: int32 = move r9
    r16: uint32 = move r12
    r17: int32 = move r13
    return r14, r15, r16, r17
}
"#,
        program,
    );

    let value = machine.complete("transform", &[Word::int32(11), Word::int32(22)]);

    assert_eq!(
        value,
        vec![
            Word::int32(11),
            Word::int32(11),
            Word::uint32(7),
            Word::int32(22),
        ]
    );
}

/// Move one value through canonical frame storage.
#[test]
fn test_execute_frame_memory() {
    let mut machine = TestMachine::parse(
        r#"
type Value

export function retain(r0: int32): int32 {
    slot s0: Value

    frame.store s0, r0
    r1: int32 = frame.load s0
    return r1
}
"#,
        TestProgram::new(),
    );

    let value = machine.complete("retain", &[Word::int32(53)]);

    assert_eq!(value, vec![Word::int32(53)]);
}
