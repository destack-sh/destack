use crate::parse::flags::ParserFlags;
use crate::parse::scope::TypeScope;
use crate::{Parser, ParserResult};
use destack_dir::{LocalNodeId, NodeType, TokenType, TypeExpression};
use smallvec::SmallVec;

impl Parser {
    /// Eat one type expression with the given parser flags.
    ///
    /// Examples:
    /// ```ds
    /// string
    /// readonly User[]
    /// keyof T extends K ? A : B
    /// ```
    pub(crate) fn eat_type_expression_in_flags(
        &mut self,
        flags: ParserFlags,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let scope = TypeScope::from_flags(flags);

        self.with_recursive_descent(NodeType::TypeExpression, |parser| {
            parser.eat_type_expression_scope(scope)
        })
    }

    /// Eat one complete type expression.
    ///
    /// Examples:
    /// ```ds
    /// User
    /// Array<string>
    /// { id: string; name?: string }
    /// ```
    pub(in crate::parse) fn eat_type_expression_body(
        &mut self,
        scope: TypeScope,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let start = self.span_start();
        let mut decorators = if !self.flags.is_in_decorator() && self.peek_is(TokenType::At) {
            self.eat_decorators_maybe()?
        } else {
            SmallVec::new()
        };

        let type_expression_id = self.eat_type_infix(&start, scope)?;
        self.attach_pending_decorators_to_type_expression(&mut decorators, type_expression_id);

        Ok(type_expression_id)
    }

    /// Eat one complete type expression under an explicit parser scope.
    ///
    /// Examples:
    /// ```ds
    /// string | number
    /// readonly User[]
    /// T extends U ? A : B
    /// ```
    pub(super) fn eat_type_expression_scope(
        &mut self,
        scope: TypeScope,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        self.with_flags(scope.flags, |parser| parser.eat_type_expression_body(scope))
    }

    /// Eat one type operator operand under an explicit type scope.
    ///
    /// Examples:
    /// ```ds
    /// number
    /// keyof T
    /// { id: string }
    /// ```
    pub(super) fn eat_type_operand(
        &mut self,
        scope: TypeScope,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        self.with_flags(scope.flags, |parser| {
            let start = parser.span_start();

            parser.eat_type_infix(&start, scope)
        })
    }

    /// Return parser flags for one nested type operand.
    pub(super) fn type_nested_flags(&self) -> ParserFlags {
        let mut flags = self.flags.not_in_position().in_type();

        if self.flags.is_disallow_type_conditional() {
            flags = flags.disallow_type_conditional();
        }

        if self.flags.is_in_type_conditional_right() {
            flags = flags.in_type_conditional_right();
        }

        flags
    }
}
