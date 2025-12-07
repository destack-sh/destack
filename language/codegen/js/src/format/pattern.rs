use crate::{LocalNodeId, Mutability, Pattern, PatternField};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::format::argument::list_like;
use crate::{FormatNode, CodegenJsFormatter};

impl<'ast> FormatNode<'ast, Pattern> for Pattern {
    fn format_node(
        &self,
        _node_id: LocalNodeId<Pattern>,
        f: &mut CodegenJsFormatter<'ast, '_>,
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
        _node_id: LocalNodeId<PatternField>,
        f: &mut CodegenJsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            PatternField::Named {
                mutability,
                name,
                pattern,
                default,
            } => {
                if let Some(mutability) = mutability
                    && *mutability == Mutability::Immutable
                {
                    write!(f, [token("const"), space()])?;
                }
                write!(f, [name])?;
                if let Some(pattern) = pattern {
                    write!(f, [token(":"), space(), pattern])?;
                }
                if let Some(default) = default {
                    write!(f, [space(), token("="), space(), default])?;
                }
            }
            PatternField::Alias {
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
                write!(f, [name, token(":"), space(), alias])?;
                if let Some(default) = default {
                    write!(f, [space(), token("="), space(), default])?;
                }
            }
            PatternField::Positional { pattern } => {
                write!(f, [pattern])?;
            }
            PatternField::Spread {
                mutability: _,
                name,
            } => {
                write!(f, [token("...")])?;
                if let Some(name) = name {
                    write!(f, [name])?;
                }
            }
        }
        Ok(())
    }
}
