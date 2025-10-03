use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

use crate::argument::list_like;
use crate::block::format_block;
use crate::r#let::FormatScopedMutability;
use crate::literal::format_scalar_literal;
use crate::{
    AssignOperator, BinaryOperator, DystFormatContext, DystFormatter, Expression, FormatNode,
    Keyword, Mutability, NodeId, Runtime, ScopedMutability, UnaryOperator, Visibility,
    empty_block_with_infix_annotations,
};

/// Walk a chain of if expressions and collect the if/else if/else nodes.
pub(crate) fn format_if_chain<'ast>(
    f: &mut DystFormatter<'ast, '_>,
    node_id: NodeId<Expression>,
) -> FormatResult<()> {
    // walk the chain
    let mut next_if_id = node_id;
    loop {
        let if_node = f.context().tree.get(next_if_id);
        match if_node {
            // if or else if
            Expression::If {
                runtime,
                condition,
                then_block,
                else_block,
            } => {
                // runtime
                if let Some(runtime) = runtime
                    && *runtime == Runtime::Static
                {
                    write!(f, [token("@")])?;
                }

                // if <condition>
                write!(f, [Keyword::If, space(), condition, space()])?;

                // then block
                format_block(f, *then_block)?;

                // next node
                if let Some(else_block) = else_block {
                    write!(f, [space(), Keyword::Else, space()])?;
                    match f.context().tree.get(*else_block) {
                        // else if
                        Expression::If { .. } => {
                            next_if_id = *else_block;
                        }
                        // else
                        Expression::Block(else_block_id) => {
                            format_block(f, *else_block_id)?;
                            break;
                        }
                        _ => panic!("invalid if chain: {else_block:?}"),
                    }
                } else {
                    // bare if
                    break;
                }
            }
            // shouldn't be anything else
            _ => panic!("invalid if chain: {if_node:?}"),
        }
    }
    Ok(())
}

/// Format a match expression.
#[inline]
pub(crate) fn format_match<'ast>(
    f: &mut DystFormatter<'ast, '_>,
    node_id: NodeId<Expression>,
    include_prefix: bool,
) -> FormatResult<()> {
    let match_node = f.context().tree.get(node_id);
    let Expression::Match {
        runtime,
        value,
        cases,
    } = &match_node
    else {
        panic!("invalid match expression: {match_node:?}");
    };

    if include_prefix {
        // runtime
        if let Some(runtime) = runtime
            && *runtime == Runtime::Static
        {
            write!(f, [token("@")])?;
        }

        // match <expression>
        write!(f, [Keyword::Match, space()])?;
    }

    write!(f, [value])?;

    // empty match body
    if cases.is_empty() {
        write!(f, [space(), empty_block_with_infix_annotations(node_id)])?;
        write!(f, [f.context().any_postfix_annotations(node_id)])?;
        return Ok(());
    }

    // match cases
    write!(f, [space(), token("{"), hard_line_break()])?;
    write!(
        f,
        [group(&format_args![block_indent(&format_with(|f| f
            .join_with(hard_line_break())
            .entries(cases)
            .finish())),])]
    )?;
    write!(f, [f.context().block_infix_annotations(node_id)])?;
    write!(f, [hard_line_break(), token("}")])?;

    Ok(())
}

impl<'ast> FormatNode<'ast, Expression> for Expression {
    fn format_node(
        &self,
        node_id: NodeId<Expression>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        match self {
            // definition
            Expression::Definition(node) => node.format(f)?,

            // block
            Expression::Block(node) => node.format(f)?,

            // with
            Expression::With { clauses } => {
                // keyword
                write!(f, [Keyword::With])?;
                if clauses.is_empty() {
                    return Ok(());
                }
                write!(f, [space()])?;

                // clauses
                write!(
                    f,
                    [best_fit_parenthesize(&format_with(|f| {
                        f.join_with(&format_args![
                            if_group_fits_on_line(&token(",")),
                            soft_line_break_or_space()
                        ])
                        .entries(clauses)
                        .finish()
                    }))]
                )?;
            }

            // use
            Expression::Use {
                visibility,
                clauses,
                body,
            } => {
                // visibility
                if let Some(visibility) = visibility {
                    let keyword = match visibility {
                        Visibility::Public => Keyword::Public,
                        Visibility::Private => Keyword::Private,
                    };
                    write!(f, [keyword, space()])?;
                }

                // keyword
                write!(f, [Keyword::Use, space()])?;
                {
                    let mut first = true;
                    for clause in clauses {
                        if !first {
                            write!(f, [token(", ")])?;
                        }
                        first = false;
                        write!(f, [*clause])?;
                    }
                }

                // scoped body
                if let Some(body) = body {
                    write!(f, [space(), body])?;
                }
            }

            // let
            Expression::Let {
                mutability,
                visibility: _,
                pattern,
                r#type,
                value,
            } => {
                write!(
                    f,
                    [group(&format_with(|f| {
                        // let
                        if mutability.is_immutable() {
                            write!(f, [Keyword::Let])?;
                        }
                        // mutability
                        write!(
                            f,
                            [FormatScopedMutability::implicit_const(mutability.clone())]
                        )?;
                        // emit pattern with optional type and value
                        write!(f, [space(), pattern])?;
                        if let Some(r#type) = r#type {
                            write!(f, [token(": "), r#type])?;
                        }
                        if let Some(value) = value {
                            write!(
                                f,
                                [
                                    space(),
                                    token("="),
                                    soft_block_indent(&format_args![
                                        soft_line_break_or_space(),
                                        value
                                    ])
                                ]
                            )
                        } else {
                            Ok(())
                        }
                    }))]
                )?;
            }

            // if
            Expression::If { .. } => {
                write!(f, [group(&format_with(|f| format_if_chain(f, node_id)))])?;
            }

            // while
            Expression::While {
                runtime,
                condition,
                body,
            } => {
                if let Some(runtime) = runtime
                    && *runtime == Runtime::Static
                {
                    write!(f, [token("@")])?;
                }
                write!(f, [Keyword::While, space(), condition, space(), body])?;
            }

            // for
            Expression::For {
                runtime,
                pattern,
                iterator,
                body,
            } => {
                if let Some(runtime) = runtime
                    && *runtime == Runtime::Static
                {
                    write!(f, [token("@")])?;
                }
                write!(
                    f,
                    [
                        Keyword::For,
                        space(),
                        pattern,
                        space(),
                        Keyword::In,
                        space(),
                        iterator,
                        space(),
                        body
                    ]
                )?;
            }

            // loop
            Expression::Loop { runtime, body } => {
                if let Some(runtime) = runtime
                    && *runtime == Runtime::Static
                {
                    write!(f, [token("@")])?;
                }
                write!(f, [Keyword::Loop, space(), body])?;
            }

            // try
            Expression::Try {
                runtime,
                r#try,
                catch,
            } => {
                // runtime
                if let Some(runtime) = runtime
                    && *runtime == Runtime::Static
                {
                    write!(f, [token("@")])?;
                }

                // try <expression>
                write!(f, [Keyword::Try, space(), r#try])?;

                // catch <expression>
                if let Some(catch) = catch {
                    write!(f, [space(), Keyword::Catch, space()])?;
                    format_match(f, *catch, false)?;
                }
            }

            // match
            Expression::Match { .. } => {
                format_match(f, node_id, true)?;
            }

            // break
            Expression::Break { label, value } => {
                write!(f, [Keyword::Break])?;
                if let Some(label) = label {
                    write!(f, [space(), token(":"), label])?;
                }
                if let Some(value) = value {
                    write!(f, [space(), value])?;
                }
            }

            // continue
            Expression::Continue { label } => {
                write!(f, [Keyword::Continue])?;
                if let Some(label) = label {
                    write!(f, [space(), token(":"), label])?;
                }
            }

            // defer
            Expression::Defer { expression, catch } => {
                write!(f, [Keyword::Defer])?;
                if let Some(expression) = expression {
                    write!(f, [space(), expression])?;
                }
                if let Some(catch) = catch {
                    write!(
                        f,
                        [
                            space(),
                            Keyword::Catch,
                            space(),
                            token("{"),
                            catch,
                            token("}")
                        ]
                    )?;
                }
            }

            // return
            Expression::Return { value } => {
                write!(f, [token("return")])?;
                if let Some(value) = value {
                    write!(f, [space(), value])?;
                }
            }

            // path
            Expression::Path {
                path,
                static_arguments,
            } => {
                write!(f, [path])?;

                // static arguments
                if let Some(static_arguments) = static_arguments
                    && !static_arguments.is_empty()
                {
                    write!(
                        f,
                        [group(&format_args![
                            token("<"),
                            soft_block_indent(&format_with(|f| {
                                f.join_with(&format_args![
                                    if_group_fits_on_line(&token(",")),
                                    soft_line_break_or_space()
                                ])
                                .entries(static_arguments)
                                .finish()
                            })),
                            token(">")
                        ])]
                    )?;
                }
            }

            // scalar literal
            Expression::ScalarLiteral(node) => {
                format_scalar_literal(node, f.context().tree.get_span(node_id), f)?;
            }

            // type literal
            Expression::TypeLiteral(node) => node.format(f)?,

            // range literal
            Expression::RangeLiteral {
                start,
                end,
                is_inclusive,
            } => {
                if *is_inclusive {
                    write!(f, [start, token("..="), end,])?;
                } else {
                    write!(f, [start, token(".."), end,])?;
                }
            }

            // array literal
            Expression::ArrayLiteral { elements } => {
                write!(f, [list_like("[", "]", ",", elements)])?;
            }

            // tuple literal
            Expression::TupleLiteral { elements } => {
                if elements.len() == 1 {
                    write!(f, [token("("), elements[0], token(","), token(")")])?;
                } else {
                    write!(f, [list_like("(", ")", ",", elements)])?;
                }
            }

            // struct literal
            Expression::StructLiteral { r#type, fields } => {
                write!(f, [r#type, space(), list_like("{", "}", ",", fields)])?;
            }

            // parenthesized
            Expression::Parenthesized { expression } => {
                write!(f, [token("("), expression, token(")")])?
            }

            // unary
            Expression::Unary { operator, right } => write!(f, [operator, right])?,

            // reference
            Expression::Reference { mutability, right } => {
                write!(f, [token("&")])?;
                write!(
                    f,
                    [FormatScopedMutability::implicit_const(mutability.clone()),]
                )?;
                match mutability {
                    ScopedMutability::Unscoped { mutability } => {
                        if *mutability != Mutability::Immutable {
                            write!(f, [space()])?;
                        }
                    }
                    ScopedMutability::Scoped { .. } => {
                        write!(f, [space()])?;
                    }
                }
                right.format(f)?;
            }

            // member
            Expression::Member { receiver, path } => write!(f, [receiver, token("."), path])?,

            // index
            Expression::Index { receiver, index } => {
                if let Some(index) = index {
                    write!(f, [receiver, token("["), index, token("]")])?;
                } else {
                    write!(f, [receiver, token("[]")])?;
                }
            }

            // call
            Expression::Call {
                runtime,
                receiver,
                dynamic_arguments,
            } => {
                let runtime = runtime.unwrap_or(Runtime::Dynamic);
                if runtime == Runtime::Static {
                    write!(f, [token("@")])?;
                }
                write!(f, [receiver])?;
                if runtime == Runtime::Dynamic || !dynamic_arguments.is_empty() {
                    write!(f, [list_like("(", ")", ",", dynamic_arguments)])?;
                }
            }

            // maybe
            Expression::Maybe(expr) => write!(f, [expr, token("?")])?,

            // must
            Expression::Must(expr) => write!(f, [expr, token("!")])?,

            // binary
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                write!(f, [left])?;
                if !f.context().has_postfix_annotation(*left) {
                    write!(f, [space()])?;
                }
                write!(f, [operator, space(), right])?;
            }

            // assign
            Expression::Assign {
                left,
                operator,
                right,
            } => {
                write!(f, [left])?;
                if !f.context().has_postfix_annotation(*left) {
                    write!(f, [space()])?;
                }
                write!(f, [operator, space(), right])?;
            }

            // error
            Expression::Error => panic!("invalid expression: {self:?}"),
        };

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for UnaryOperator {
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        let token = match self {
            UnaryOperator::Not => token("!"),
            UnaryOperator::Negate => token("-"),
            UnaryOperator::WrappingNegate => token("-%"),
            UnaryOperator::ElementwiseNot => token("~"),
            UnaryOperator::Dereference => token("*"),
            UnaryOperator::Virtual => token("$"),
            UnaryOperator::Spread => token(".."),
        };
        write!(f, [token])
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for BinaryOperator {
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        let token = match self {
            // multiplication
            BinaryOperator::Multiply => token("*"),
            BinaryOperator::WrappingMultiply => token("*%"),
            BinaryOperator::SaturatingMultiply => token("*|"),
            BinaryOperator::Divide => token("/"),
            BinaryOperator::Remainder => token("%"),

            // addition
            BinaryOperator::Add => token("+"),
            BinaryOperator::WrappingAdd => token("+%"),
            BinaryOperator::SaturatingAdd => token("+|"),
            BinaryOperator::Subtract => token("-"),
            BinaryOperator::WrappingSubtract => token("-%"),
            BinaryOperator::SaturatingSubtract => token("-|"),

            // shift
            BinaryOperator::ShiftLeft => token("<<"),
            BinaryOperator::SaturatingShiftLeft => token("<<|"),
            BinaryOperator::ShiftRight => token(">>"),

            // elementwise
            BinaryOperator::ElementwiseAnd => token("&"),
            BinaryOperator::ElementwiseXor => token("^"),
            BinaryOperator::ElementwiseOr => token("|"),

            // comparison
            BinaryOperator::Equal => token("=="),
            BinaryOperator::NotEqual => token("!="),
            BinaryOperator::LessThan => token("<"),
            BinaryOperator::LessThanOrEqual => token("<="),
            BinaryOperator::GreaterThan => token(">"),
            BinaryOperator::GreaterThanOrEqual => token(">="),

            // logical
            BinaryOperator::And => token("&&"),
            BinaryOperator::Or => token("||"),
            BinaryOperator::Coalesce => token("??"),
            BinaryOperator::Cast => token("as"),
        };
        write!(f, [token])
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for AssignOperator {
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        let token = token(match self {
            AssignOperator::Assign => "=",

            // addition
            AssignOperator::AddAssign => "+=",
            AssignOperator::WrappingAddAssign => "+%=",
            AssignOperator::SaturatingAddAssign => "+|=",
            AssignOperator::SubtractAssign => "-=",
            AssignOperator::WrappingSubtractAssign => "-%=",
            AssignOperator::SaturatingSubtractAssign => "-|=",

            // multiplication
            AssignOperator::MultiplyAssign => "*=",
            AssignOperator::WrappingMultiplyAssign => "*%=",
            AssignOperator::SaturatingMultiplyAssign => "*|=",
            AssignOperator::DivideAssign => "/=",
            AssignOperator::RemainderAssign => "%=",

            // shift
            AssignOperator::ShiftLeftAssign => "<<=",
            AssignOperator::SaturatingShiftLeftAssign => "<<|=",
            AssignOperator::ShiftRightAssign => ">>=",

            // elementwise
            AssignOperator::ElementwiseAndAssign => "&=",
            AssignOperator::ElementwiseOrAssign => "|=",
            AssignOperator::ElementwiseXorAssign => "^=",

            // logical
            AssignOperator::AndAssign => "&&=",
            AssignOperator::OrAssign => "||=",
        });
        write!(f, [token])
    }
}

#[cfg(test)]
mod tests {
    use crate::format::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    /// Simple expressions should stay on one line.
    #[test]
    fn test_format_expression_simple() {
        assert_format!(
            "1 + 2 * 3 - a / b % c",
            "1 + 2 * 3 - a / b % c",
            |p| p.eat_expression(),
            DystFormatOptions::default_tab()
        );
    }

    /// Parenthesized expressions should retain their parentheses.
    #[test]
    fn test_format_expression_parenthesized() {
        assert_format!(
            "(((1 + 2) * 3) - a / (b % c))",
            "(((1 + 2) * 3) - a / (b % c))",
            |p| p.eat_expression(),
            DystFormatOptions::default_tab()
        );
    }

    /// Empty parenthesis/arguments/tuplestuples should be respected.
    #[test]
    fn test_format_expression_nested_empty_parenthesis() {
        assert_format!(
            "(((())))",
            "(((())))",
            |p| p.eat_expression(),
            DystFormatOptions::default()
        );
    }

    /// Empty parenthesis/arguments/tuplestuples should be respected.
    #[test]
    fn test_format_expression_nested_empty_arguments() {
        assert_format!(
            "foo<()>(((())))",
            "foo<()>(((())))",
            |p| p.eat_expression(),
            DystFormatOptions::default()
        );
    }
}
