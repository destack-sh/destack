use tspp_mir::{Discriminant, DiscriminantField, VariantEncoding};
use tspp_program::{
    LayoutShapeBuilder, ScalarFormat, TypeId, VariantCaseLayout, VariantLayoutBuilder, Word,
};

use super::{TestMachine, TestProgram};

/// Construct, update, and extract one packed aggregate.
#[test]
fn test_execute_aggregate() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    aggregate r2:r3, [r0 @ 0:4, r1 @ 8:4]
    insert r4:r5, r2:r3, 8:4, r0
    extract r6, r4:r5, 8:4
    insert r7:r8, r2:r3, 8:4, r0
    extract r9, r7:r8, 8:4
    move r10, r6
    move r11, r9
    return r10:r11
}
"#,
        TestProgram::words(),
    );

    let value = machine.complete(0, &[Word::int32(11), Word::int32(22)]);

    assert_eq!(value, vec![Word::int32(11), Word::int32(11)]);
}

/// Construct and inspect one variant through its linked physical layout.
#[test]
fn test_execute_variant() {
    let variant = VariantLayoutBuilder::new(
        TypeId(1),
        VariantEncoding::Direct {
            field: DiscriminantField::scalar(0, 4),
        },
    )
    .cases([
        VariantCaseLayout {
            discriminant: Discriminant::from_bits(3),
            ty: TypeId(1),
            payload_offset: 8,
        },
        VariantCaseLayout {
            discriminant: Discriminant::from_bits(7),
            ty: TypeId(1),
            payload_offset: 8,
        },
    ]);
    let program = TestProgram::words()
        .layout(0, LayoutShapeBuilder::Variant(variant), 16, 8)
        .layout(
            1,
            LayoutShapeBuilder::Scalar(ScalarFormat::int(32, true)),
            4,
            4,
        );
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    variant.new r1:r2, l0, 1, r0
    variant.tag r3, r1:r2, l0
    extract r4, r1:r2, 8:4
    return r3:r4
}
"#,
        program,
    );

    let value = machine.complete(0, &[Word::int32(22)]);

    assert_eq!(value, vec![Word::uint32(7), Word::int32(22)]);
}
