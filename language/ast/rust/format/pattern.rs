use dyst_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode, Mutability, NodeId, Pattern, PatternField};
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

impl<'ast> FormatNode<'ast, Pattern> for Pattern {
    fn format_node(
        &self,
        _node_id: NodeId<Pattern>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Pattern::Wildcard => write!(f, [token("_")]),
            Pattern::Rest => write!(f, [token("..")]),
            Pattern::Reference { target, mutability } => {
                if *mutability == Mutability::Mutable {
                    write!(f, [token("&var "), target])
                } else {
                    write!(f, [token("&"), target])
                }
            }
            Pattern::Literal(literal) => write!(f, [literal]),
            Pattern::Binding { name } => write!(f, [name]),
            Pattern::Path(path) => write!(f, [path]),
            Pattern::Range { start, end, .. } => write!(f, [start, token(".."), end,]),
            Pattern::Tuple { path, fields } => {
                if let Some(path) = path {
                    write!(f, [path])?;
                }
                write!(
                    f,
                    [group(&format_args![
                        token("("),
                        soft_block_indent(&format_with(|f| f
                            .join_with(&format_args![
                                if_group_fits_on_line(&token(",")),
                                soft_line_break_or_space()
                            ])
                            .entries(fields)
                            .finish())),
                        token(")"),
                    ])]
                )
            }
            Pattern::Slice { fields } => write!(
                f,
                [group(&format_args![
                    token("["),
                    soft_block_indent(&format_with(|f| f
                        .join_with(&format_args![
                            if_group_fits_on_line(&token(",")),
                            soft_line_break_or_space()
                        ])
                        .entries(fields)
                        .finish())),
                    token("]"),
                ])]
            ),
            Pattern::Struct { r#type, fields } => write!(
                f,
                [group(&format_args![
                    r#type,
                    token("{"),
                    soft_block_indent(&format_with(|f| f
                        .join_with(&format_args![
                            if_group_fits_on_line(&token(",")),
                            soft_line_break_or_space()
                        ])
                        .entries(fields)
                        .finish())),
                    token("}"),
                ])]
            ),
            Pattern::Union { fields } => write!(
                f,
                [format_with(|f| f
                    .join_with(token(" | "))
                    .entries(fields)
                    .finish())]
            ),
        }
    }
}

impl<'ast> FormatNode<'ast, PatternField> for PatternField {
    fn format_node(
        &self,
        _node_id: NodeId<PatternField>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            PatternField::Named {
                name,
                pattern,
                mutability,
            } => {
                if let Some(mutability) = mutability {
                    let mutability_token = if *mutability == Mutability::Mutable {
                        token("var")
                    } else {
                        token("const")
                    };
                    write!(f, [mutability_token, space(), name])?;
                }
                if let Some(pattern) = pattern {
                    write!(f, [name, token(": "), pattern])
                } else {
                    write!(f, [name])
                }
            }
            PatternField::NamedAlias {
                name,
                alias,
                mutability,
            } => {
                if let Some(mutability) = mutability {
                    let mutability_token = if *mutability == Mutability::Mutable {
                        token("var")
                    } else {
                        token("const")
                    };
                    write!(f, [mutability_token, space(), name])?;
                }
                write!(f, [name, token(": "), alias])
            }
            PatternField::Positional { pattern } => write!(f, [pattern]),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::format::tests::TestFormatter;
    use crate::{DystFormatOptions, ExpressionParserOptions, assert_format};

    #[test]
    fn test_format_pattern_wildcard() {
        assert_format!("_", "_", |p| p
            .eat_pattern(ExpressionParserOptions::default()));
    }

    #[test]
    fn test_format_pattern_rest() {
        assert_format!("..", "..", |p| p
            .eat_pattern(ExpressionParserOptions::default()));
    }

    #[test]
    fn test_format_pattern_reference() {
        assert_format!("&var _", "&var _", |p| p
            .eat_pattern(ExpressionParserOptions::default()));

        assert_format!("&1", "&1", |p| p
            .eat_pattern(ExpressionParserOptions::default()));
    }

    #[test]
    fn test_format_pattern_identifier() {
        assert_format!("x", "x", |p| p
            .eat_pattern(ExpressionParserOptions::default()));
    }

    #[test]
    fn test_format_pattern_path() {
        assert_format!("MyEnum.A", "MyEnum.A", |p| p
            .eat_pattern(ExpressionParserOptions::default()));
    }

    #[test]
    fn test_format_pattern_tuple() {
        assert_format!("(x: 1, 2, ..)", "(x: 1, 2, ..)", |p| p
            .eat_pattern(ExpressionParserOptions::default()));
    }

    #[test]
    fn test_format_pattern_tuple_with_path() {
        assert_format!("Result.Success(_, ..)", "Result.Success(_, ..)", |p| p
            .eat_pattern(ExpressionParserOptions::default()));
    }

    #[test]
    fn test_format_pattern_slice() {
        assert_format!("[1, 2, ..]", "[1, 2, ..]", |p| p
            .eat_pattern(ExpressionParserOptions::default()));
    }

    #[test]
    fn test_format_pattern_union() {
        assert_format!("1 | 2 | 3 | 4 | 5", "1 | 2 | 3 | 4 | 5", |p| p
            .eat_pattern(ExpressionParserOptions::default()));
    }
}
