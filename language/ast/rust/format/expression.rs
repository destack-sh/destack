use dyst_fir::format::FormatResult;
use dyst_fir::prelude::*;
use dyst_fir::write;

use crate::r#let::FormatScopedMutability;
use crate::{
    AssignOperator, BinaryOperator, DystFormatContext, DystFormatter, Expression, FormatNode,
    Mutability, NodeId, ScopedMutability, UnaryOperator,
};

impl<'ast> FormatNode<'ast, Expression> for Expression {
    fn format_node(
        &self,
        node_id: NodeId<Expression>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;
        match self {
            Expression::Module(node) => node.format(f)?,
            Expression::Struct(node) => node.format(f)?,
            Expression::Enum(node) => node.format(f)?,
            Expression::Union(node) => node.format(f)?,
            Expression::Trait(node) => node.format(f)?,
            Expression::Implement(node) => node.format(f)?,
            Expression::Function(node) => node.format(f)?,
            Expression::Block(node) => node.format(f)?,

            Expression::Let(node) => node.format(f)?,
            Expression::If(node) => node.format(f)?,
            Expression::While(node) => node.format(f)?,
            Expression::For(node) => node.format(f)?,
            Expression::Loop(node) => node.format(f)?,
            Expression::Break(node) => node.format(f)?,
            Expression::Continue(node) => node.format(f)?,
            Expression::Defer(node) => node.format(f)?,
            Expression::Return(node) => node.format(f)?,
            Expression::Try(node) => node.format(f)?,
            Expression::Match(node) => node.format(f)?,

            Expression::Path(p) => p.format(f)?,
            Expression::ScalarLiteral(node) => node.format(f)?,
            Expression::RangeLiteral(node) => node.format(f)?,
            Expression::ArrayLiteral(node) => node.format(f)?,
            Expression::TupleLiteral(node) => node.format(f)?,
            Expression::StructLiteral(node) => node.format(f)?,

            Expression::Unary { operator, right } => write!(f, [operator, right])?,
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
            Expression::Member { receiver, path } => write!(f, [receiver, token("."), path])?,
            Expression::Index(node) => node.format(f)?,
            Expression::Call(node) => node.format(f)?,
            Expression::Cast(node) => node.format(f)?,
            Expression::Unwrap(expr) => write!(f, [expr, token("?")])?,
            Expression::UnwrapOrPanic(expr) => write!(f, [expr, token("!")])?,
            Expression::Coalesce(node) => node.format(f)?,
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

            Expression::Error => panic!("invalid expression: {self:?}"),
        };
        write!(f, [f.context().any_postfix_annotations(node_id)])?;
        Ok(())
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for UnaryOperator {
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        let token = match self {
            UnaryOperator::Not => token("!"),
            UnaryOperator::Negate => token("-"),
            UnaryOperator::WrappingNegate => token("-%"),
            UnaryOperator::BitwiseNot => token("~"),
            UnaryOperator::Dereference => token("*"),
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

            // bitwise
            BinaryOperator::BitwiseAnd => token("&"),
            BinaryOperator::BitwiseXor => token("^"),
            BinaryOperator::BitwiseOr => token("|"),

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

            // bitwise
            AssignOperator::BitwiseAndAssign => "&=",
            AssignOperator::BitwiseOrAssign => "|=",
            AssignOperator::BitwiseXorAssign => "^=",

            // logical
            AssignOperator::AndAssign => "&&=",
            AssignOperator::OrAssign => "||=",
        });
        write!(f, [token])
    }
}

#[cfg(test)]
mod tests {}
