use dyst_ast::{BindingKind, BindingModifier, BindingOperator, BindingScope, Keyword, Mutability};
use dyst_fir::format::FormatResult;

use crate::DystFormatter;
use dyst_fir::prelude::*;
use dyst_fir::write;

#[inline]
pub(crate) fn format_binding_modifiers_prefix<'ast>(
    f: &mut DystFormatter<'ast, '_>,
    modifiers: BindingModifier,
) -> FormatResult<()> {
    // visibility
    if let Some(visibility) = modifiers.visibility {
        write!(f, [visibility, space()])?;
    }
    // scope
    if modifiers.scope == Some(BindingScope::Static) {
        write!(f, [Keyword::Static, space()])?;
    }
    // mutability
    if modifiers.mutability == Some(Mutability::Immutable) {
        write!(f, [Keyword::Readonly, space()])?;
    }
    // operator
    if modifiers.operator == Some(BindingOperator::AsConst) {
        write!(f, [Keyword::Const, space()])?;
    }
    Ok(())
}

#[inline]
pub(crate) fn format_binding_modifiers_prefix_maybe<'ast>(
    f: &mut DystFormatter<'ast, '_>,
    modifiers: Option<BindingModifier>,
) -> FormatResult<()> {
    if let Some(modifiers) = modifiers {
        format_binding_modifiers_prefix(f, modifiers)?;
    }
    Ok(())
}

#[inline]
pub(crate) fn format_binding_modifiers_postfix<'ast>(
    f: &mut DystFormatter<'ast, '_>,
    modifiers: BindingModifier,
) -> FormatResult<()> {
    // kind
    if modifiers.kind == Some(BindingKind::Maybe) {
        write!(f, [token("?")])?;
    }
    Ok(())
}

#[inline]
pub(crate) fn format_binding_modifiers_postfix_maybe<'ast>(
    f: &mut DystFormatter<'ast, '_>,
    modifiers: Option<BindingModifier>,
) -> FormatResult<()> {
    if let Some(modifiers) = modifiers {
        format_binding_modifiers_postfix(f, modifiers)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};
    use dyst_ast::DefinitionMeta;

    #[test]
    fn test_format_struct_empty() {
        assert_format!(
            "struct { }",
            "struct { }",
            |p| p.eat_struct(DefinitionMeta::default()),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_struct_with_fields() {
        assert_format!(
            "struct { a: int32, b: boolean }",
            "struct {\n\ta: int32\n\tb: boolean\n}",
            |p| p.eat_struct(DefinitionMeta::default()),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_modified_fields() {
        assert_format!(
            "struct { readonly a: int32, private b: boolean }",
            "struct {\n\treadonly a: int32\n\tprivate b: boolean\n}",
            |p| p.eat_struct(DefinitionMeta::default()),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_name() {
        assert_format!(
            "struct Foo { a: int32 }",
            "struct Foo {\n\ta: int32\n}",
            |p| p.eat_struct(DefinitionMeta::default()),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_fields_and_defaults() {
        assert_format!(
            "struct { a?: int32 = 42, b: boolean? }",
            "struct {\n\ta?: int32 = 42\n\tb: boolean?\n}",
            |p| p.eat_struct(DefinitionMeta::default()),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_static_parameters_and_inheritance() {
        assert_format!(
            "struct Foo<T: Numeric> extends Bar implements Baz { }",
            "struct Foo<T: Numeric> extends Bar implements Baz { }",
            |p| p.eat_struct(DefinitionMeta::default()),
            DystFormatOptions::default()
        );
    }
}
