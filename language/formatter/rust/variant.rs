use dyst_ast::BindingModifiers;
use dyst_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode, NodeId, VariantField};
use dyst_fir::prelude::*;
use dyst_fir::write;

#[inline]
pub(crate) fn format_binding_modifiers_prefix<'ast>(
    f: &mut DystFormatter<'ast, '_>,
    modifiers: BindingModifiers,
) -> FormatResult<()> {
    if let Some(visibility) = modifiers.visibility {
        write!(f, [visibility, space()])?;
    }
    if let Some(mutability) = modifiers.mutability {
        write!(f, [mutability, space()])?;
    }
    Ok(())
}

#[inline]
pub(crate) fn format_binding_modifiers_prefix_maybe<'ast>(
    f: &mut DystFormatter<'ast, '_>,
    modifiers: Option<BindingModifiers>,
) -> FormatResult<()> {
    if let Some(modifiers) = modifiers {
        format_binding_modifiers_prefix(f, modifiers)?;
    }
    Ok(())
}

#[inline]
pub(crate) fn format_binding_modifiers_postfix<'ast>(
    f: &mut DystFormatter<'ast, '_>,
    modifiers: BindingModifiers,
) -> FormatResult<()> {
    if let Some(visibility) = modifiers.visibility {
        write!(f, [visibility, space()])?;
    }
    Ok(())
}

#[inline]
pub(crate) fn format_binding_modifiers_postfix_maybe<'ast>(
    f: &mut DystFormatter<'ast, '_>,
    modifiers: Option<BindingModifiers>,
) -> FormatResult<()> {
    if let Some(modifiers) = modifiers {
        format_binding_modifiers_postfix(f, modifiers)?;
    }
    Ok(())
}

impl<'ast> FormatNode<'ast, VariantField> for VariantField {
    fn format_node(
        &self,
        node_id: NodeId<VariantField>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        match self {
            VariantField::Named {
                modifiers,
                name,
                ty,
                default,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // name
                write!(f, [name, token(":"), space()])?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                // type
                write!(f, [ty])?;
                // default
                if let Some(default) = default {
                    write!(f, [space(), token("="), space(), default])?;
                }
            }
            VariantField::Positional {
                modifiers,
                ty,
                default,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // type
                write!(f, [ty])?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                // default
                if let Some(default) = default {
                    write!(f, [space(), token("="), space(), default])?;
                }
            }
            VariantField::Dynamic {
                modifiers,
                name,
                ty,
                key,
                default,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // key
                write!(f, [token("[")])?;
                if let Some(name) = name {
                    write!(f, [name, token(":"), space()])?;
                }
                write!(f, [key, token("]"), token(":"), space(), ty])?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                // default
                if let Some(default) = default {
                    write!(f, [space(), token("="), space(), default])?;
                }
            }
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
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
    fn test_format_struct_with_representation_type() {
        assert_format!(
            "struct(uint64) Foo { a: int32 }",
            "struct(uint64) Foo {\n\ta: int32\n}",
            |p| p.eat_struct(DefinitionMeta::default()),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_tuple_fields() {
        assert_format!(
            "struct Foo(int32, boolean) { }",
            "struct Foo(int32, boolean) { }",
            |p| p.eat_struct(DefinitionMeta::default()),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_struct_with_tuple_body_statements() {
        assert_format!(
            "struct Foo(int32) { const X = 2 }",
            "struct Foo(int32) {\n\tconst X = 2\n}",
            |p| p.eat_struct(DefinitionMeta::default()),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_fields_and_defaults() {
        assert_format!(
            "struct { a: int32 = 42, b: boolean }",
            "struct {\n\ta: int32 = 42\n\tb: boolean\n}",
            |p| p.eat_struct(DefinitionMeta::default()),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_expressions() {
        assert_format!(
            r"struct { const X = 1 }",
            r"struct {
	const X = 1
}",
            |p| p.eat_struct(DefinitionMeta::default()),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_fields_and_statements() {
        assert_format!(
            "struct { a: int32, const X = 1 }",
            "struct {\n\ta: int32\n\n\tconst X = 1\n}",
            |p| p.eat_struct(DefinitionMeta::default()),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_static_parameters_and_super_types() {
        assert_format!(
            "struct Foo<T: Numeric>: Bar, Baz { }",
            "struct Foo<T: Numeric>: Bar, Baz { }",
            |p| p.eat_struct(DefinitionMeta::default()),
            DystFormatOptions::default()
        );
    }
}
