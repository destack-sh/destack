use dyst_fir::format::FormatResult;
use dyst_fir::prelude::*;
use dyst_fir::write;
use dyst_javascript_ast::{
    Asynchrony, BindingAnchor, BindingKind, BindingModifier, BindingOperator, FunctionAbstraction,
    FunctionCardinality, Keyword, LocalNodeId, Mutability, Property,
};

use crate::format::argument::list_like;
use crate::{FormatNode, JavaScriptFormatter};

#[inline]
pub(crate) fn format_binding_modifiers_prefix<'ast>(
    f: &mut JavaScriptFormatter<'ast, '_>,
    modifiers: BindingModifier,
) -> FormatResult<()> {
    // visibility
    if let Some(visibility) = modifiers.visibility {
        write!(f, [visibility, space()])?;
    }
    // scope
    if modifiers.anchor == Some(BindingAnchor::Static) {
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
    f: &mut JavaScriptFormatter<'ast, '_>,
    modifiers: Option<BindingModifier>,
) -> FormatResult<()> {
    if let Some(modifiers) = modifiers {
        format_binding_modifiers_prefix(f, modifiers)?;
    }
    Ok(())
}

#[inline]
pub(crate) fn format_binding_modifiers_postfix<'ast>(
    f: &mut JavaScriptFormatter<'ast, '_>,
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
    f: &mut JavaScriptFormatter<'ast, '_>,
    modifiers: Option<BindingModifier>,
) -> FormatResult<()> {
    if let Some(modifiers) = modifiers {
        format_binding_modifiers_postfix(f, modifiers)?;
    }
    Ok(())
}

impl<'ast> FormatNode<'ast, Property> for Property {
    fn format_node(
        &self,
        _node_id: LocalNodeId<Property>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
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
                if let Some(static_parameters) = signature
                    .generics
                    .as_ref()
                    .and_then(|generics| generics.static_parameters.as_ref())
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
                // body
                if let Some(body) = body {
                    write!(f, [space(), body])?;
                }
            }
            Property::Spread { modifiers, value } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // value
                write!(f, [token("..."), value])?;
            }
        }

        Ok(())
    }
}
