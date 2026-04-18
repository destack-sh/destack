use crate::{
    AccessorKind, Asynchrony, BindingAnchor, BindingKind, BindingModifier, BindingOperator,
    FunctionCardinality, Keyword, LocalNodeId, Member, Mutability, Property, VarianceModifier,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::format::argument::list_like;
use crate::format::function::format_function_signature_parameters;
use crate::{FormatNode, JsFormatter};

#[inline]
pub(crate) fn format_binding_modifiers_prefix<'ast>(
    f: &mut JsFormatter<'ast, '_>,
    modifiers: BindingModifier,
) -> FormatResult<()> {
    // variance
    if let Some(variance) = modifiers.variance {
        match variance {
            VarianceModifier::In => write!(f, [token("in"), space()])?,
            VarianceModifier::Out => write!(f, [token("out"), space()])?,
            VarianceModifier::InOut => {
                write!(f, [token("in"), space(), token("out"), space()])?;
            }
        }
    }
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
    // accessor
    if modifiers.accessor == Some(AccessorKind::Accessor) {
        write!(f, [Keyword::Accessor, space()])?;
    }
    Ok(())
}

#[inline]
pub(crate) fn format_binding_modifiers_prefix_maybe<'ast>(
    f: &mut JsFormatter<'ast, '_>,
    modifiers: Option<BindingModifier>,
) -> FormatResult<()> {
    if let Some(modifiers) = modifiers {
        format_binding_modifiers_prefix(f, modifiers)?;
    }
    Ok(())
}

#[inline]
pub(crate) fn format_binding_modifiers_postfix<'ast>(
    f: &mut JsFormatter<'ast, '_>,
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
    f: &mut JsFormatter<'ast, '_>,
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
        f: &mut JsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Property::Field {
                modifiers,
                key,
                value,
                is_shorthand,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // key
                write!(f, [key])?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;

                // value
                if !is_shorthand {
                    write!(f, [token(":"), space(), value])?;
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
                if signature.is_abstract {
                    write!(f, [Keyword::Abstract, space()])?;
                }

                if signature.is_override {
                    write!(f, [Keyword::Override, space()])?;
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
                // generic parameters
                if !signature.generic_parameters.is_empty() {
                    write!(f, [list_like("<", ">", ",", &signature.generic_parameters)])?;
                }
                // parameters
                format_function_signature_parameters(signature, f)?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                // return type
                if f.context().include_types()
                    && let Some(return_type) = signature.return_type
                {
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

impl<'ast> FormatNode<'ast, Member> for Member {
    fn format_node(
        &self,
        _node_id: LocalNodeId<Member>,
        f: &mut JsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Member::Field {
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
            Member::Method {
                modifiers,
                key,
                signature,
                body,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // abstraction
                if signature.is_abstract {
                    write!(f, [Keyword::Abstract, space()])?;
                }

                if signature.is_override {
                    write!(f, [Keyword::Override, space()])?;
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
                // generic parameters
                if !signature.generic_parameters.is_empty() {
                    write!(f, [list_like("<", ">", ",", &signature.generic_parameters)])?;
                }
                // parameters
                format_function_signature_parameters(signature, f)?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                // return type
                if f.context().include_types()
                    && let Some(return_type) = signature.return_type
                {
                    write!(f, [token(":"), space(), return_type])?;
                }
                // body
                if let Some(body) = body {
                    write!(f, [space(), body])?;
                }
            }
            Member::StaticBlock { body } => {
                // keyword
                write!(f, [Keyword::Static, space()])?;
                // body
                write!(f, [body])?;
            }
        }

        Ok(())
    }
}
