use crate::format::function::{format_function_parameters, format_function_signature_parameters};
use crate::{
    Asynchrony, FormatNode, Formatter, FunctionRole, FunctionSignature, Key, Keyword, LocalNodeId,
    Member, MemberModifier, Property,
};
use tspp_fir::format::FormatResult;
use tspp_fir::prelude::*;
use tspp_fir::write;

impl<'ast> FormatNode<'ast, Property> for Property {
    fn format_node(
        &self,
        _node_id: LocalNodeId<Property>,
        f: &mut Formatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Self::Field {
                key,
                value,
                is_shorthand,
            } => {
                write!(f, [key])?;
                if !is_shorthand {
                    write!(f, [token(":"), space(), value])?;
                }
            }
            Self::Method {
                key,
                role,
                signature,
                body,
            } => {
                format_method(None, *key, *role, signature, f)?;
                write!(f, [space(), body])?;
            }
            Self::Spread { value } => {
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
        f: &mut Formatter<'ast, '_>,
    ) -> FormatResult<()> {
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
                format_method(Some(*modifiers), *key, *role, signature, f)?;
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
fn format_method<'ast>(
    modifiers: Option<MemberModifier>,
    key: Key,
    role: Option<FunctionRole>,
    signature: &FunctionSignature,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
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
    write!(f, [key])?;

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
