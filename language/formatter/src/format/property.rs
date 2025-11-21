use crate::argument::list_like;
use crate::r#where::format_where_clause;
use crate::with::format_with_clause;
use crate::{DystFormatter, FormatNode};
use dyst_ast::{
    Asynchrony, BindingKind, BindingModifier, BindingOperator, BindingScope, FunctionAbstraction,
    FunctionCardinality, Keyword, LocalNodeId, Mutability, Property,
};
use dyst_fir::format::FormatResult;
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

/// Format a block of properties (with appropriate empty annotations)
pub(crate) fn format_block_of_properties<'ast>(
    f: &mut DystFormatter<'ast, '_>,
    properties: &[LocalNodeId<Property>],
) -> FormatResult<()> {
    for (i, &property_id) in properties.iter().enumerate() {
        // blank line between properties
        if i > 0 {
            write!(f, [hard_line_break()])?;
        }
        property_id.format(f)?;
    }
    Ok(())
}

impl<'ast> FormatNode<'ast, Property> for Property {
    fn format_node(
        &self,
        node_id: LocalNodeId<Property>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        match self {
            Property::Field {
                modifiers,
                key,
                value,
                default,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // key
                write!(f, [key])?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                // value
                if let Some(value) = value {
                    write!(f, [token(":"), space(), value])?;
                }
                // default
                if let Some(default) = default {
                    write!(f, [space(), token("="), space(), default])?;
                }
            }
            Property::Method {
                modifiers,
                key,
                signature,
                body,
            } => {
                let generics = signature.generics.as_ref();

                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;

                // abstraction
                match signature.abstraction {
                    FunctionAbstraction::Abstract => {
                        write!(f, [Keyword::Abstract, space()])?;
                    }
                    FunctionAbstraction::AbstractOverride => {
                        write!(f, [Keyword::Abstract, space()])?;
                        write!(f, [Keyword::Override, space()])?;
                    }
                    FunctionAbstraction::ConcreteOverride => {
                        write!(f, [Keyword::Override, space()])?;
                    }
                    FunctionAbstraction::Concrete => {}
                }

                // asynchrony
                if signature.asynchrony == Asynchrony::Async {
                    write!(f, [Keyword::Async, space()])?;
                }

                // mode
                if let Some(mode) = signature.mode {
                    if let Some(keyword) = mode.to_keyword() {
                        write!(f, [keyword])?;
                    }
                    if key.is_some() {
                        write!(f, [space()])?;
                    }
                }

                // cardinality
                if signature.cardinality == FunctionCardinality::Generator {
                    write!(f, [token("*")])?;
                }

                // key
                write!(f, [key])?;

                // static parameters
                if let Some(static_parameters) =
                    generics.and_then(|generics| generics.static_parameters.as_ref())
                    && !static_parameters.is_empty()
                {
                    write!(f, [list_like("<", ">", ",", static_parameters)])?;
                }

                // dynamic parameters
                write!(f, [list_like("(", ")", ",", &signature.dynamic_parameters)])?;

                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;

                // return type
                if let Some(return_type) = signature.return_type {
                    write!(f, [token(":"), space(), return_type])?;
                }

                // with clauses
                if let Some(with_clauses) =
                    generics.and_then(|generics| generics.with_clauses.as_ref())
                    && !with_clauses.is_empty()
                {
                    write!(f, [space()])?;
                    format_with_clause(f, with_clauses)?;
                }

                // where clauses
                if let Some(where_clauses) =
                    generics.and_then(|generics| generics.where_clauses.as_ref())
                    && !where_clauses.is_empty()
                {
                    write!(f, [space()])?;
                    format_where_clause(f, where_clauses)?;
                }

                // body
                if let Some(body) = body {
                    write!(f, [space(), body])?;
                }
            }
            Property::Spread { modifiers, value } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // keyword
                write!(f, [token("...")])?;
                // value
                write!(f, [value])?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
            }
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{DystFormatOptions, TestFormatter, assert_format};
    use dyst_ast::DeclarationDescriptor;

    #[test]
    fn test_format_struct_empty() {
        assert_format!(
            "struct { }",
            "struct { }",
            |p| p.eat_struct(DeclarationDescriptor::default()),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_struct_with_fields() {
        assert_format!(
            "struct { a: int32, b: boolean }",
            "struct {\n\ta: int32\n\tb: boolean\n}",
            |p| p.eat_struct(DeclarationDescriptor::default()),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_modified_fields() {
        assert_format!(
            "struct { readonly a: int32, private b: boolean }",
            "struct {\n\treadonly a: int32\n\tprivate b: boolean\n}",
            |p| p.eat_struct(DeclarationDescriptor::default()),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_name() {
        assert_format!(
            "struct Foo { a: int32 }",
            "struct Foo {\n\ta: int32\n}",
            |p| p.eat_struct(DeclarationDescriptor::default()),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_fields_and_defaults() {
        assert_format!(
            "struct { a?: int32 = 42, b: boolean? }",
            "struct {\n\ta?: int32 = 42\n\tb: boolean?\n}",
            |p| p.eat_struct(DeclarationDescriptor::default()),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_static_parameters_and_inheritance() {
        assert_format!(
            "struct Foo<T: Numeric> extends Bar implements Baz { }",
            "struct Foo<T: Numeric> extends Bar implements Baz { }",
            |p| p.eat_struct(DeclarationDescriptor::default()),
            DystFormatOptions::default()
        );
    }
}
