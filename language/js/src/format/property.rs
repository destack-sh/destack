use crate::format::function::{format_function_parameters, format_function_signature_parameters};
use crate::format::identifier::format_shorthand;
use crate::{
    Asynchrony, Context, FormatNode, Formatter, FunctionRole, FunctionSignature, Keyword, Member,
    MemberModifier, Property,
};
use destack_fir::format::{Format, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

impl<'ast> FormatNode<'ast> for Property {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Self::Field { key, value } => {
                write!(f, [key, token(":"), space(), value])?;
            }
            Self::Shorthand { value } => {
                format_shorthand(*value, f)?;
            }
            Self::Method {
                key,
                role,
                signature,
                body,
            } => {
                format_method(None, key, *role, signature, f)?;
                write!(f, [space(), body])?;
            }
            Self::Spread { value } => {
                write!(f, [token("..."), value])?;
            }
        }

        Ok(())
    }
}

impl<'ast> FormatNode<'ast> for Member {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Self::Field {
                modifiers,
                key,
                default,
            } => {
                format_member_modifiers(*modifiers, f)?;
                write!(f, [key])?;
                if let Some(default) = default {
                    write!(f, [space(), token("="), space(), default])?;
                }
            }
            Self::Method {
                modifiers,
                key,
                role,
                signature,
                body,
            } => {
                format_method(Some(*modifiers), key, *role, signature, f)?;
                write!(f, [space(), body])?;
            }
            Self::Constructor { parameters, body } => {
                write!(f, [Keyword::Constructor])?;
                format_function_parameters(parameters, f)?;
                write!(f, [space(), body])?;
            }
            Self::StaticBlock { body } => {
                write!(f, [Keyword::Static, space(), body])?;
            }
        }

        Ok(())
    }
}

/// Format one method header.
fn format_method<'ast, T>(
    modifiers: Option<MemberModifier>,
    key: &T,
    role: Option<FunctionRole>,
    signature: &FunctionSignature,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()>
where
    T: Format<'ast, Context<'ast>>,
{
    if let Some(modifiers) = modifiers {
        format_member_modifiers(modifiers, f)?;
    }
    if signature.asynchrony == Asynchrony::Async {
        write!(f, [Keyword::Async, space()])?;
    }
    if let Some(role) = role {
        write!(f, [role.keyword(), space()])?;
    }
    if signature.is_generator {
        write!(f, [token("*")])?;
    }
    key.format(f)?;

    format_function_signature_parameters(signature, f)
}

/// Format the runtime modifiers of one class member.
fn format_member_modifiers<'ast>(
    modifiers: MemberModifier,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    if modifiers.is_static {
        write!(f, [Keyword::Static, space()])?;
    }
    if modifiers.is_accessor {
        write!(f, [Keyword::Accessor, space()])?;
    }

    Ok(())
}
