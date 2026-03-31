use crate::{DestackFormatOptions, assert_format};
use destack_ast::{DeclarationDescriptor, EnumKind};

#[test]
fn test_format_enum_empty() {
    assert_format!(
        "enum { }",
        "enum {}",
        |p| p.eat_enum(&p.mark(), EnumKind::Enum, DeclarationDescriptor::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_enum_with_simple_fields() {
    assert_format!(
        "enum { A, B }",
        "enum {\n\tA,\n\tB,\n}",
        |p| p.eat_enum(&p.mark(), EnumKind::Enum, DeclarationDescriptor::default()),
        DestackFormatOptions::default_tab()
    );
}

#[test]
fn test_format_enum_with_annotations() {
    assert_format!(
        "enum { A }",
        "enum {\n\tA,\n}",
        |p| p.eat_enum(&p.mark(), EnumKind::Enum, DeclarationDescriptor::default()),
        DestackFormatOptions::default_tab()
    );
}

#[test]
fn test_format_enum_with_static_parameters() {
    let source = r"enum Machine<T: int32 = 3, IsSomething: boolean = true> {
    A = 1,
    B = T,
    @if(IsSomething)
    C = 3,
}";
    assert_format!(
        source,
        source,
        |p| p.eat_enum(&p.mark(), EnumKind::Enum, DeclarationDescriptor::default()),
        DestackFormatOptions::default()
    );
}
