use destack_fir::format::{Format, FormatResult};
use destack_fir::prelude::*;
use destack_fir::{format_args, write};

use crate::format::expression::format_expression_id_with_precedence;
use crate::format::identifier::format_shorthand;
use crate::{
    ArrayAssignPatternField, ArrayPatternField, AssignPattern, Context, FormatNode, Formatter,
    LocalNodeId, ObjectAssignPatternField, ObjectPatternField, Pattern, Place, Precedence, Tree,
    TreeStore,
};

impl<'ast> FormatNode<'ast> for Place {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Self::Identifier { identifier } => write!(f, [identifier])?,
            Self::Member { object, property } => {
                let needs_extra_dot = format_place_object(*object, f)?;
                if needs_extra_dot {
                    write!(f, [token(".")])?;
                }
                write!(f, [token("."), property])?;
            }
            Self::PrivateMember { object, property } => {
                let needs_extra_dot = format_place_object(*object, f)?;
                if needs_extra_dot {
                    write!(f, [token(".")])?;
                }
                write!(f, [token("."), token("#"), property])?;
            }
            Self::Index { object, key } => {
                let needs_parentheses = f
                    .context()
                    .tree
                    .get(*object)
                    .is_optional_chain(f.context().tree);
                if needs_parentheses {
                    write!(f, [token("(")])?;
                }
                format_expression_id_with_precedence(*object, Precedence::Call, f)?;
                if needs_parentheses {
                    write!(f, [token(")")])?;
                }
                write!(f, [token("[")])?;
                format_expression_id_with_precedence(*key, Precedence::Lowest, f)?;
                write!(f, [token("]")])?;
            }
        }

        Ok(())
    }
}

impl<'ast> FormatNode<'ast> for AssignPattern {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Self::Place { place } => write!(f, [place])?,
            Self::Array { fields, rest } => {
                let ends_in_elision = rest.is_none()
                    && fields.last().is_some_and(|field| {
                        matches!(
                            f.context().tree.get(*field),
                            ArrayAssignPatternField::Elision
                        )
                    });
                format_pattern("[", "]", fields, *rest, ends_in_elision, f)?;
            }
            Self::Object { fields, rest } => {
                format_pattern("{", "}", fields, *rest, false, f)?;
            }
        }

        Ok(())
    }
}

impl<'ast> FormatNode<'ast> for ArrayAssignPatternField {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Self::Positional { pattern, default } => {
                write!(f, [pattern])?;
                format_default(*default, f)?;
            }
            Self::Elision => {}
        }

        Ok(())
    }
}

impl<'ast> FormatNode<'ast> for ObjectAssignPatternField {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Self::Named {
                name,
                pattern,
                default,
            } => {
                write!(f, [name, token(":"), space(), pattern])?;
                format_default(*default, f)?;
            }
            Self::Shorthand {
                identifier,
                default,
            } => {
                format_shorthand(*identifier, f)?;
                format_default(*default, f)?;
            }
        }

        Ok(())
    }
}

impl<'ast> FormatNode<'ast> for Pattern {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Self::Binding { identifier } => write!(f, [identifier])?,
            Self::Array { fields, rest } => {
                let ends_in_elision = rest.is_none()
                    && fields.last().is_some_and(|field| {
                        matches!(f.context().tree.get(*field), ArrayPatternField::Elision)
                    });
                format_pattern("[", "]", fields, *rest, ends_in_elision, f)?;
            }
            Self::Object { fields, rest } => {
                format_pattern("{", "}", fields, *rest, false, f)?;
            }
        }

        Ok(())
    }
}

impl<'ast> FormatNode<'ast> for ArrayPatternField {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Self::Positional { pattern, default } => {
                write!(f, [pattern])?;
                format_default(*default, f)?;
            }
            Self::Elision => {}
        }

        Ok(())
    }
}

impl<'ast> FormatNode<'ast> for ObjectPatternField {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Self::Named {
                name,
                pattern,
                default,
            } => {
                write!(f, [name, token(":"), space(), pattern])?;
                format_default(*default, f)?;
            }
            Self::Shorthand {
                identifier,
                default,
            } => {
                format_shorthand(*identifier, f)?;
                format_default(*default, f)?;
            }
        }

        Ok(())
    }
}

/// Format one optional pattern default.
pub(crate) fn format_default<'ast>(
    default: Option<LocalNodeId<crate::Expression>>,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    if let Some(default) = default {
        write!(f, [space(), token("="), space()])?;
        format_expression_id_with_precedence(default, Precedence::Assignment, f)?;
    }

    Ok(())
}

/// Format one binding or assignment pattern collection.
pub(crate) fn format_pattern<'ast, T, R>(
    open: &'static str,
    close: &'static str,
    fields: &[LocalNodeId<T>],
    rest: Option<R>,
    ends_in_elision: bool,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()>
where
    T: FormatNode<'ast>,
    R: Copy + Format<'ast, Context<'ast>>,
    Tree: TreeStore<T>,
{
    let body = format_with(|f| {
        for (index, field) in fields.iter().enumerate() {
            if index > 0 {
                write!(f, [token(","), soft_line_break_or_space()])?;
            }
            field.format(f)?;
        }

        if let Some(rest) = rest {
            if !fields.is_empty() {
                write!(f, [token(","), soft_line_break_or_space()])?;
            }
            write!(f, [token("..."), rest])?;
        } else if ends_in_elision {
            write!(f, [token(",")])?;
        } else if !fields.is_empty() {
            write!(f, [if_group_breaks(&token(","))])?;
        }

        Ok(())
    });

    write!(
        f,
        [group(&format_args![
            &token(open),
            block_indent(&body),
            &token(close)
        ])]
    )
}

/// Format one non-optional place object.
fn format_place_object<'ast>(
    id: LocalNodeId<crate::Expression>,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<bool> {
    let expression = f.context().tree.get(id);
    let needs_parentheses = expression.is_optional_chain(f.context().tree);
    if needs_parentheses {
        write!(f, [token("(")])?;
    }
    format_expression_id_with_precedence(id, Precedence::Call, f)?;
    if needs_parentheses {
        write!(f, [token(")")])?;
    }

    Ok(!needs_parentheses && expression.needs_decimal_member_dot(false))
}
