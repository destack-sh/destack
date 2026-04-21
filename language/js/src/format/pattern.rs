use crate::{AssignPattern, AssignPatternField, LocalNodeId, Mutability, Pattern, PatternField};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::format::argument::list_like;
use crate::{FormatNode, JsFormatter};

impl<'ast> FormatNode<'ast, Pattern> for Pattern {
    fn format_node(
        &self,
        _node_id: LocalNodeId<Pattern>,
        f: &mut JsFormatter<'ast, '_>,
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
        f: &mut JsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            PatternField::Named {
                mutability,
                name,
                is_shorthand,
                pattern,
            } => {
                if let Some(mutability) = mutability
                    && *mutability == Mutability::Immutable
                {
                    write!(f, [token("const"), space()])?;
                }
                write!(f, [name])?;

                if !is_shorthand {
                    let pattern = pattern.expect("expanded named js pattern field");
                    write!(f, [token(":"), space(), pattern])?;
                } else if let Some(pattern) = pattern {
                    write_shorthand_assignment_value(f, *pattern)?;
                }
            }
            PatternField::Computed {
                mutability,
                key,
                pattern,
            } => {
                if let Some(mutability) = mutability
                    && *mutability == Mutability::Immutable
                {
                    write!(f, [token("const"), space()])?;
                }
                write!(f, [token("["), key, token("]")])?;
                write!(f, [token(":"), space(), pattern])?;
            }
            PatternField::Positional { pattern } => {
                write!(f, [pattern])?;
            }
            PatternField::Spread {
                mutability: _,
                pattern,
            } => {
                write!(f, [token("...")])?;
                if let Some(pattern) = pattern {
                    write!(f, [pattern])?;
                }
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
        f: &mut JsFormatter<'ast, '_>,
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
        f: &mut JsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            AssignPatternField::Named {
                name,
                is_shorthand,
                pattern,
            } => {
                write!(f, [name])?;

                if !is_shorthand {
                    let pattern = pattern.expect("expanded named js assign pattern field");
                    write!(f, [token(":"), space(), pattern])?;
                } else if let Some(pattern) = pattern {
                    write_shorthand_assign_pattern_value(f, *pattern)?;
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
                write!(f, [token("...")])?;

                if let Some(pattern) = pattern {
                    write!(f, [pattern])?;
                }
            }
            AssignPatternField::Elision => {
                // elision is an empty slot, comma handled at list level
            }
        }

        Ok(())
    }
}

/// Write the value side of one shorthand assignment pattern.
fn write_shorthand_assignment_value(
    f: &mut JsFormatter<'_, '_>,
    pattern_id: LocalNodeId<Pattern>,
) -> FormatResult<()> {
    let pattern = f.context().tree.get(pattern_id);
    let Pattern::Assign { value, .. } = pattern else {
        unreachable!("expected shorthand assignment pattern");
    };

    write!(f, [space(), token("="), space(), value])
}

/// Write the value side of one shorthand assignment target.
fn write_shorthand_assign_pattern_value(
    f: &mut JsFormatter<'_, '_>,
    assign_pattern_id: LocalNodeId<AssignPattern>,
) -> FormatResult<()> {
    let assign_pattern = f.context().tree.get(assign_pattern_id);
    let AssignPattern::Assign { value, .. } = assign_pattern else {
        unreachable!("expected shorthand assignment target");
    };

    write!(f, [space(), token("="), space(), value])
}
