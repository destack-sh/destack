use crate::{AssignPattern, AssignPatternField, Pattern, PatternField};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::format::argument::delimited;
use crate::format::identifier::format_shorthand;
use crate::{FormatNode, Formatter};

impl<'ast> FormatNode<'ast> for Pattern {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Pattern::Binding { identifier } => {
                write!(f, [identifier])?;
            }
            Pattern::Assign { pattern, value } => {
                write!(f, [pattern, space(), token("="), space(), value])?;
            }
            Pattern::Array { fields } => {
                write!(f, [delimited("[", "]", ",", fields)])?;
            }
            Pattern::Object { fields } => {
                write!(f, [delimited("{", "}", ",", fields)])?;
            }
            Pattern::Hole => {
                write!(f, [token(",")])?;
            }
        }

        Ok(())
    }
}

impl<'ast> FormatNode<'ast> for PatternField {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            PatternField::Named { name, pattern } => {
                write!(f, [name, token(":"), space(), pattern])?;
            }
            PatternField::Shorthand { identifier, value } => {
                format_shorthand(*identifier, f)?;
                if let Some(value) = value {
                    write!(f, [space(), token("="), space(), value])?;
                }
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

impl<'ast> FormatNode<'ast> for AssignPattern {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            AssignPattern::Expression { value } => {
                write!(f, [value])?;
            }
            AssignPattern::Assign { pattern, value } => {
                write!(f, [pattern, space(), token("="), space(), value])?;
            }
            AssignPattern::Array { fields } => {
                write!(f, [delimited("[", "]", ",", fields)])?;
            }
            AssignPattern::Object { fields } => {
                write!(f, [delimited("{", "}", ",", fields)])?;
            }
        }

        Ok(())
    }
}

impl<'ast> FormatNode<'ast> for AssignPatternField {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            AssignPatternField::Named { name, pattern } => {
                write!(f, [name, token(":"), space(), pattern])?;
            }
            AssignPatternField::Shorthand { identifier, value } => {
                format_shorthand(*identifier, f)?;
                if let Some(value) = value {
                    write!(f, [space(), token("="), space(), value])?;
                }
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
