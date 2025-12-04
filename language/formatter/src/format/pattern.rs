use destack_fir::format::FormatResult;

use crate::argument::list_like;
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{LocalNodeId, Mutability, Pattern, PatternField};
use destack_fir::prelude::*;
use destack_fir::{format_args, write};

impl<'ast> Format<DestackFormatContext<'ast>> for Mutability {
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Mutability::Immutable => write!(f, [token("const")]),
            Mutability::Mutable => write!(f, [token("var")]),
        }
    }
}

impl<'ast> FormatNode<'ast, Pattern> for Pattern {
    fn format_node(
        &self,
        node_id: LocalNodeId<Pattern>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        match self {
            Pattern::Wildcard => write!(f, [token("_")])?,
            Pattern::Maybe(unwrap) => write!(f, [unwrap, token("?")])?,
            Pattern::ReferenceOf { right, mutability } => {
                write!(f, [token("&")])?;
                if let Some(mutability) = mutability {
                    write!(f, [mutability, space()])?;
                }
                write!(f, [right])?;
            }
            Pattern::ValueOf { right, mutability } => {
                write!(f, [token("^")])?;
                if let Some(mutability) = mutability {
                    write!(f, [mutability, space()])?;
                }
                write!(f, [right])?;
            }
            Pattern::Binding {
                mutability,
                name,
                pattern,
            } => {
                if let Some(mutability) = mutability {
                    write!(f, [mutability, space()])?;
                }
                write!(f, [name])?;
                if let Some(pattern) = pattern {
                    write!(f, [token(":"), space(), pattern])?;
                }
            }
            Pattern::Expression { value } => write!(f, [value])?,
            Pattern::Range { start, end, .. } => write!(f, [start, token(".."), end,])?,
            Pattern::Tuple { fields } => write!(f, [list_like("(", ")", ",", fields)])?,
            Pattern::TaggedTuple { ty, fields } => {
                write!(f, [ty])?;
                write!(f, [list_like("(", ")", ",", fields)])?
            }
            Pattern::Array { fields } => {
                write!(f, [list_like("[", "]", ",", fields)])?;
            }
            Pattern::Object { fields } => {
                write!(f, [list_like("{", "}", ",", fields).include_space()])?;
            }
            Pattern::TaggedObject { ty, fields } => {
                write!(f, [ty, space()])?;
                write!(f, [list_like("{", "}", ",", fields).include_space()])?;
            }
            Pattern::Union { patterns } => write!(
                f,
                [format_with(|f| f
                    .join_with(&format_args![space(), token("|"), space()])
                    .entries(patterns)
                    .finish())]
            )?,
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, PatternField> for PatternField {
    fn format_node(
        &self,
        node_id: LocalNodeId<PatternField>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        match self {
            PatternField::Named {
                mutability,
                name,
                pattern,
                default,
            } => {
                if let Some(mutability) = mutability {
                    write!(f, [mutability, space()])?;
                }
                if let Some(pattern) = pattern {
                    write!(f, [name, token(":"), space(), pattern])?;
                } else {
                    write!(f, [name])?;
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
                if let Some(mutability) = mutability {
                    write!(f, [mutability, space()])?;
                }
                write!(f, [name, token(":"), space(), alias])?;
                if let Some(default) = default {
                    write!(f, [space(), token("="), space(), default])?;
                }
            }
            PatternField::Positional { pattern } => write!(f, [pattern])?,
            PatternField::Spread { mutability, name } => {
                if let Some(mutability) = mutability {
                    write!(f, [mutability, space()])?;
                }
                write!(f, [token("...")])?;
                if let Some(name) = name {
                    write!(f, [name])?;
                }
            }
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{DestackFormatOptions, TestFormatter, assert_format};

    #[test]
    fn test_format_pattern_wildcard() {
        assert_format!("_", "_", |p| p.eat_pattern());
    }

    #[test]
    fn test_format_pattern_reference() {
        assert_format!("&var _", "&var _", |p| p.eat_pattern());

        assert_format!("&1", "&1", |p| p.eat_pattern());
    }

    #[test]
    fn test_format_pattern_unwrap() {
        assert_format!("T?", "T?", |p| p.eat_pattern());
    }

    #[test]
    fn test_format_pattern_identifier() {
        assert_format!("x", "x", |p| p.eat_pattern());
    }

    #[test]
    fn test_format_pattern_path() {
        assert_format!("MyEnum.A", "MyEnum.A", |p| p.eat_pattern());
    }

    #[test]
    fn test_format_pattern_tuple() {
        assert_format!("(x: 1, 2, ...)", "(x: 1, 2, ...)", |p| p.eat_pattern());
    }

    #[test]
    fn test_format_pattern_tuple_with_path() {
        assert_format!("Result.Success(_, ...)", "Result.Success(_, ...)", |p| p
            .eat_pattern());
    }

    #[test]
    fn test_format_pattern_slice() {
        assert_format!("[1, 2, ...]", "[1, 2, ...]", |p| p.eat_pattern());
    }

    #[test]
    fn test_format_pattern_union() {
        assert_format!("1 | 2 | 3 | 4 | 5", "1 | 2 | 3 | 4 | 5", |p| p
            .eat_pattern());
    }
}
