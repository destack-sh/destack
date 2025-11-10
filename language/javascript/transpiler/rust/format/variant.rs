use dyst_fir::format::FormatResult;
use dyst_fir::prelude::*;
use dyst_fir::write;
use dyst_javascript_ast::{
    BindingKind, BindingModifier, BindingOperator, BindingScope, Field, Keyword, Mutability, NodeId,
};

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

impl<'ast> FormatNode<'ast, Field> for Field {
    fn format_node(
        &self,
        _node_id: NodeId<Field>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Field::Named {
                modifiers,
                name,
                ty,
                default,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // name
                write!(f, [name])?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                write!(f, [token(":"), space()])?;
                // type
                write!(f, [ty])?;
                // default
                if let Some(default) = default {
                    write!(f, [space(), token("="), space(), default])?;
                }
            }
            Field::Dynamic {
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

        Ok(())
    }
}
