use crate::{AssignPattern, AssignPatternField, LocalNodeId, Pattern, PatternField};
use tspp_fir::format::FormatResult;
use tspp_fir::prelude::*;
use tspp_fir::write;

use crate::format::argument::list_like;
use crate::{FormatNode, Formatter};

impl<'ast> FormatNode<'ast, Pattern> for Pattern {
    fn format_node(
        &self,
        _node_id: LocalNodeId<Pattern>,
        f: &mut Formatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Pattern::Binding { name } => {
                write!(f, [name])?;
            }
            Pattern::Assign { pattern, value } => {
                write!(f, [pattern, space(), token("="), space(), value])?;
            }
            Pattern::Array { fields } => {
                write!(f, [list_like("[", "]", ",", fields)])?;
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
        f: &mut Formatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            PatternField::Named { name, pattern } => {
                write!(f, [name, token(":"), space(), pattern])?;
            }
            PatternField::Shorthand { name, value } => {
                write!(f, [name])?;
                if let Some(value) = value {
                    write!(f, [space(), token("="), space(), value])?;
                }
            }
            PatternField::Computed { key, pattern } => {
                write!(f, [token("["), key, token("]")])?;
                write!(f, [token(":"), space(), pattern])?;
            }
            PatternField::Positional { pattern } => {
                write!(f, [pattern])?;
            }
            PatternField::Spread { pattern } => {
                write!(f, [token("..."), pattern])?;
            }
            PatternField::Elision => {
                // elision is an empty slot; comma handled at list level
            }
        }

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, AssignPattern> for AssignPattern {
    fn format_node(
        &self,
        _node_id: LocalNodeId<AssignPattern>,
        f: &mut Formatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            AssignPattern::Expression { value } => {
                write!(f, [value])?;
            }
            AssignPattern::Assign { pattern, value } => {
                write!(f, [pattern, space(), token("="), space(), value])?;
            }
            AssignPattern::Array { fields } => {
                write!(f, [list_like("[", "]", ",", fields)])?;
            }
            AssignPattern::Object { fields } => {
                write!(f, [list_like("{", "}", ",", fields)])?;
            }
        }

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, AssignPatternField> for AssignPatternField {
    fn format_node(
        &self,
        _node_id: LocalNodeId<AssignPatternField>,
        f: &mut Formatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            AssignPatternField::Named { name, pattern } => {
                write!(f, [name, token(":"), space(), pattern])?;
            }
            AssignPatternField::Shorthand { name, value } => {
                write!(f, [name])?;
                if let Some(value) = value {
                    write!(f, [space(), token("="), space(), value])?;
                }
            }
            AssignPatternField::Computed { key, pattern } => {
                write!(f, [token("["), key, token("]")])?;
                write!(f, [token(":"), space(), pattern])?;
            }
            AssignPatternField::Positional { pattern } => {
                write!(f, [pattern])?;
            }
            AssignPatternField::Spread { pattern } => {
                write!(f, [token("..."), pattern])?;
            }
            AssignPatternField::Elision => {
                // elision is an empty slot, comma handled at list level
            }
        }

        Ok(())
    }
}
