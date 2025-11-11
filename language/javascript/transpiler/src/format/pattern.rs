use dyst_fir::format::FormatResult;
use dyst_fir::prelude::*;
use dyst_fir::write;
use dyst_javascript_ast::{Mutability, NodeId, Pattern, PatternField};

use crate::format::argument::list_like;
use crate::{FormatNode, JavaScriptFormatter};

impl<'ast> FormatNode<'ast, Pattern> for Pattern {
    fn format_node(
        &self,
        _node_id: NodeId<Pattern>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Pattern::Binding { mutability, name } => {
                if let Some(mutability) = mutability
                    && *mutability == Mutability::Immutable
                {
                    write!(f, [token("const"), space()])?;
                }
                write!(f, [name])?;
            }
            Pattern::Array { elements } => {
                write!(f, [list_like("[", "]", ",", elements)])?;
            }
            Pattern::Object { fields } => {
                write!(f, [list_like("{", "}", ",", fields)])?;
            }
            Pattern::Rest { name } => {
                write!(f, [token("...")])?;
                if let Some(name) = name {
                    write!(f, [name])?;
                }
            }
            Pattern::Hole => {
                write!(f, [token(",")])?;
            }
        }
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, PatternField> for PatternField {
    fn format_node(
        &self,
        _node_id: NodeId<PatternField>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            PatternField::Named {
                mutability,
                name,
                alias,
                default,
            } => {
                if let Some(mutability) = mutability
                    && *mutability == Mutability::Immutable
                {
                    write!(f, [token("const"), space()])?;
                }
                write!(f, [name])?;
                if let Some(alias) = alias {
                    write!(f, [token(":"), space(), alias])?;
                }
                if let Some(default) = default {
                    write!(f, [space(), token("="), space(), default])?;
                }
            }
            PatternField::Pattern {
                mutability,
                pattern,
                default,
            } => {
                if let Some(mutability) = mutability
                    && *mutability == Mutability::Immutable
                {
                    write!(f, [token("const"), space()])?;
                }
                write!(f, [pattern])?;
                if let Some(default) = default {
                    write!(f, [space(), token("="), space(), default])?;
                }
            }
            PatternField::Positional { pattern } => {
                write!(f, [pattern])?;
            }
        }
        Ok(())
    }
}
