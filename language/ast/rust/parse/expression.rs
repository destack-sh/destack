//! Parse expressions. Mostly defers to other parsers.

use dyst_language_token::TokenType;

use crate::{
    AssignOperator, BinaryOperator, Expression, InfixOperator, Keyword, NodeId, OperatorPrecedence,
    ParseError, ParseResult, Parser, Runtime, UnaryOperator,
};

impl BinaryOperator {
    /// Get the precedence of the binary operator.
    #[inline]
    pub fn precedence_group(&self) -> OperatorPrecedence {
        match self {
            // multiplication
            BinaryOperator::Multiply => OperatorPrecedence::Multiplication,
            BinaryOperator::WrappingMultiply => OperatorPrecedence::Multiplication,
            BinaryOperator::SaturatingMultiply => OperatorPrecedence::Multiplication,
            BinaryOperator::Divide => OperatorPrecedence::Multiplication,
            BinaryOperator::Remainder => OperatorPrecedence::Multiplication,

            // addition
            BinaryOperator::Add => OperatorPrecedence::Addition,
            BinaryOperator::WrappingAdd => OperatorPrecedence::Addition,
            BinaryOperator::SaturatingAdd => OperatorPrecedence::Addition,
            BinaryOperator::Subtract => OperatorPrecedence::Addition,
            BinaryOperator::WrappingSubtract => OperatorPrecedence::Addition,
            BinaryOperator::SaturatingSubtract => OperatorPrecedence::Addition,

            // shift
            BinaryOperator::ShiftLeft => OperatorPrecedence::Shift,
            BinaryOperator::SaturatingShiftLeft => OperatorPrecedence::Shift,
            BinaryOperator::ShiftRight => OperatorPrecedence::Shift,

            // bitwise
            BinaryOperator::BitwiseAnd => OperatorPrecedence::Bitwise,
            BinaryOperator::BitwiseXor => OperatorPrecedence::Bitwise,
            BinaryOperator::BitwiseOr => OperatorPrecedence::Bitwise,

            // comparison
            BinaryOperator::Equal => OperatorPrecedence::Comparison,
            BinaryOperator::NotEqual => OperatorPrecedence::Comparison,
            BinaryOperator::LessThan => OperatorPrecedence::Comparison,
            BinaryOperator::LessThanOrEqual => OperatorPrecedence::Comparison,
            BinaryOperator::GreaterThan => OperatorPrecedence::Comparison,
            BinaryOperator::GreaterThanOrEqual => OperatorPrecedence::Comparison,

            // logical
            BinaryOperator::LogicalAnd => OperatorPrecedence::Logical,
            BinaryOperator::LogicalOr => OperatorPrecedence::Logical,
        }
    }

    /// Get the precedence of the binary operator.
    pub fn precedence(self) -> u8 {
        // just transmute the enum value to an u8
        self as u8
    }

    /// Convert a TokenType to a BinaryOperator (if a direct mapping exists).
    #[inline]
    pub fn from_token_type(token_type: TokenType) -> Option<BinaryOperator> {
        match token_type {
            // multiplication
            TokenType::Multiply => Some(BinaryOperator::Multiply),
            TokenType::WrappingMultiply => Some(BinaryOperator::WrappingMultiply),
            TokenType::SaturatingMultiply => Some(BinaryOperator::SaturatingMultiply),
            TokenType::Divide => Some(BinaryOperator::Divide),
            TokenType::Remainder => Some(BinaryOperator::Remainder),

            // addition
            TokenType::Add => Some(BinaryOperator::Add),
            TokenType::WrappingAdd => Some(BinaryOperator::WrappingAdd),
            TokenType::SaturatingAdd => Some(BinaryOperator::SaturatingAdd),
            TokenType::Subtract => Some(BinaryOperator::Subtract),
            TokenType::WrappingSubtract => Some(BinaryOperator::WrappingSubtract),
            TokenType::SaturatingSubtract => Some(BinaryOperator::SaturatingSubtract),

            // shift
            TokenType::ShiftLeft => Some(BinaryOperator::ShiftLeft),
            TokenType::SaturatingShiftLeft => Some(BinaryOperator::SaturatingShiftLeft),
            TokenType::ShiftRight => Some(BinaryOperator::ShiftRight),

            // bitwise
            TokenType::BitwiseAnd => Some(BinaryOperator::BitwiseAnd),
            TokenType::BitwiseXor => Some(BinaryOperator::BitwiseXor),
            TokenType::BitwiseOr => Some(BinaryOperator::BitwiseOr),

            // comparison
            TokenType::Equal => Some(BinaryOperator::Equal),
            TokenType::NotEqual => Some(BinaryOperator::NotEqual),
            TokenType::LessThan => Some(BinaryOperator::LessThan),
            TokenType::LessThanOrEqual => Some(BinaryOperator::LessThanOrEqual),
            TokenType::GreaterThan => Some(BinaryOperator::GreaterThan),
            TokenType::GreaterThanOrEqual => Some(BinaryOperator::GreaterThanOrEqual),

            // logical
            TokenType::LogicalAnd => Some(BinaryOperator::LogicalAnd),
            TokenType::LogicalOr => Some(BinaryOperator::LogicalOr),

            _ => None,
        }
    }

    /// Convert a BinaryOperator to a TokenType (if a direct mapping exists).
    #[inline]
    pub fn as_token_type(&self) -> TokenType {
        match self {
            // multiplication
            BinaryOperator::Multiply => TokenType::Multiply,
            BinaryOperator::WrappingMultiply => TokenType::WrappingMultiply,
            BinaryOperator::SaturatingMultiply => TokenType::SaturatingMultiply,
            BinaryOperator::Divide => TokenType::Divide,
            BinaryOperator::Remainder => TokenType::Remainder,

            // addition
            BinaryOperator::Add => TokenType::Add,
            BinaryOperator::WrappingAdd => TokenType::WrappingAdd,
            BinaryOperator::SaturatingAdd => TokenType::SaturatingAdd,
            BinaryOperator::Subtract => TokenType::Subtract,
            BinaryOperator::WrappingSubtract => TokenType::WrappingSubtract,
            BinaryOperator::SaturatingSubtract => TokenType::SaturatingSubtract,

            // shift
            BinaryOperator::ShiftLeft => TokenType::ShiftLeft,
            BinaryOperator::SaturatingShiftLeft => TokenType::SaturatingShiftLeft,
            BinaryOperator::ShiftRight => TokenType::ShiftRight,

            // bitwise
            BinaryOperator::BitwiseAnd => TokenType::BitwiseAnd,
            BinaryOperator::BitwiseXor => TokenType::BitwiseXor,
            BinaryOperator::BitwiseOr => TokenType::BitwiseOr,

            // comparison
            BinaryOperator::Equal => TokenType::Equal,
            BinaryOperator::NotEqual => TokenType::NotEqual,
            BinaryOperator::LessThan => TokenType::LessThan,
            BinaryOperator::LessThanOrEqual => TokenType::LessThanOrEqual,
            BinaryOperator::GreaterThan => TokenType::GreaterThan,
            BinaryOperator::GreaterThanOrEqual => TokenType::GreaterThanOrEqual,

            // logical
            BinaryOperator::LogicalAnd => TokenType::LogicalAnd,
            BinaryOperator::LogicalOr => TokenType::LogicalOr,
        }
    }
}

impl UnaryOperator {
    /// Get the precedence of the unary operator.
    #[inline]
    pub fn precedence_group(&self) -> OperatorPrecedence {
        OperatorPrecedence::Prefix
    }

    /// Get the precedence of the unary operator.
    #[inline]
    pub fn precedence(self) -> u8 {
        // just transmute the enum value to an u8
        self as u8
    }

    /// Convert a TokenType to a UnaryOperator (if a direct mapping exists).
    #[inline]
    pub fn from_token_type(token_type: TokenType) -> Option<UnaryOperator> {
        match token_type {
            TokenType::Bang => Some(UnaryOperator::LogicalNot),
            TokenType::Subtract => Some(UnaryOperator::Negate),
            TokenType::WrappingSubtract => Some(UnaryOperator::WrappingNegate),
            TokenType::Multiply => Some(UnaryOperator::Dereference),
            TokenType::BitwiseAnd => Some(UnaryOperator::Reference),
            TokenType::BitwiseNot => Some(UnaryOperator::BitwiseNot),
            _ => None,
        }
    }

    /// Convert a UnaryOperator to a TokenType (if a direct mapping exists).
    #[inline]
    pub fn as_token_type(&self) -> TokenType {
        match self {
            UnaryOperator::LogicalNot => TokenType::Bang,
            UnaryOperator::Negate => TokenType::Subtract,
            UnaryOperator::WrappingNegate => TokenType::WrappingSubtract,
            UnaryOperator::Dereference => TokenType::Multiply,
            UnaryOperator::Reference => TokenType::BitwiseAnd,
            UnaryOperator::BitwiseNot => TokenType::BitwiseNot,
        }
    }
}

impl AssignOperator {
    /// Get the precedence of the assignment type.
    #[inline]
    pub fn precedence_group(&self) -> OperatorPrecedence {
        match self {
            // assignment
            AssignOperator::Assign => OperatorPrecedence::Assignment,

            // assignment multiplication
            AssignOperator::MultiplyAssign
            | AssignOperator::WrappingMultiplyAssign
            | AssignOperator::SaturatingMultiplyAssign
            | AssignOperator::DivideAssign
            | AssignOperator::RemainderAssign => OperatorPrecedence::AssignmentMultiplication,

            // assignment addition
            AssignOperator::AddAssign
            | AssignOperator::WrappingAddAssign
            | AssignOperator::SaturatingAddAssign
            | AssignOperator::SubtractAssign
            | AssignOperator::WrappingSubtractAssign
            | AssignOperator::SaturatingSubtractAssign => OperatorPrecedence::AssignmentAddition,

            // assignment shift
            AssignOperator::ShiftLeftAssign
            | AssignOperator::SaturatingShiftLeftAssign
            | AssignOperator::ShiftRightAssign => OperatorPrecedence::AssignmentShift,

            // assignment bitwise
            AssignOperator::BitwiseAndAssign
            | AssignOperator::BitwiseXorAssign
            | AssignOperator::BitwiseOrAssign => OperatorPrecedence::AssignmentBitwise,

            // assignment logical
            AssignOperator::LogicalAndAssign | AssignOperator::LogicalOrAssign => {
                OperatorPrecedence::AssignmentLogical
            }
        }
    }

    /// Get the precedence of the assignment type.
    #[inline]
    pub fn precedence(self) -> u8 {
        // just transmute the enum value to an u8
        self as u8
    }

    /// Convert a TokenType to an AssignOperator (if a direct mapping exists).
    #[inline]
    pub fn from_token_type(token_type: TokenType) -> Option<AssignOperator> {
        match token_type {
            TokenType::Assign => Some(AssignOperator::Assign),

            // addition
            TokenType::AddAssign => Some(AssignOperator::AddAssign),
            TokenType::WrappingAddAssign => Some(AssignOperator::WrappingAddAssign),
            TokenType::SaturatingAddAssign => Some(AssignOperator::SaturatingAddAssign),
            TokenType::SubtractAssign => Some(AssignOperator::SubtractAssign),
            TokenType::WrappingSubtractAssign => Some(AssignOperator::WrappingSubtractAssign),
            TokenType::SaturatingSubtractAssign => Some(AssignOperator::SaturatingSubtractAssign),

            // multiplication
            TokenType::MultiplyAssign => Some(AssignOperator::MultiplyAssign),
            TokenType::WrappingMultiplyAssign => Some(AssignOperator::WrappingMultiplyAssign),
            TokenType::SaturatingMultiplyAssign => Some(AssignOperator::SaturatingMultiplyAssign),
            TokenType::DivideAssign => Some(AssignOperator::DivideAssign),
            TokenType::RemainderAssign => Some(AssignOperator::RemainderAssign),

            // shift
            TokenType::ShiftLeftAssign => Some(AssignOperator::ShiftLeftAssign),
            TokenType::SaturatingShiftLeftAssign => Some(AssignOperator::SaturatingShiftLeftAssign),
            TokenType::ShiftRightAssign => Some(AssignOperator::ShiftRightAssign),

            // bitwise
            TokenType::BitwiseAndAssign => Some(AssignOperator::BitwiseAndAssign),
            TokenType::BitwiseOrAssign => Some(AssignOperator::BitwiseOrAssign),
            TokenType::BitwiseXorAssign => Some(AssignOperator::BitwiseXorAssign),

            // logical
            TokenType::LogicalAndAssign => Some(AssignOperator::LogicalAndAssign),
            TokenType::LogicalOrAssign => Some(AssignOperator::LogicalOrAssign),

            _ => None,
        }
    }

    /// Convert an AssignOperator to a TokenType (if a direct mapping exists).
    #[inline]
    pub fn as_token_type(&self) -> TokenType {
        match self {
            AssignOperator::Assign => TokenType::Assign,

            // addition
            AssignOperator::AddAssign => TokenType::AddAssign,
            AssignOperator::WrappingAddAssign => TokenType::WrappingAddAssign,
            AssignOperator::SaturatingAddAssign => TokenType::SaturatingAddAssign,
            AssignOperator::SubtractAssign => TokenType::SubtractAssign,
            AssignOperator::WrappingSubtractAssign => TokenType::WrappingSubtractAssign,
            AssignOperator::SaturatingSubtractAssign => TokenType::SaturatingSubtractAssign,

            // multiplication
            AssignOperator::MultiplyAssign => TokenType::MultiplyAssign,
            AssignOperator::WrappingMultiplyAssign => TokenType::WrappingMultiplyAssign,
            AssignOperator::SaturatingMultiplyAssign => TokenType::SaturatingMultiplyAssign,
            AssignOperator::DivideAssign => TokenType::DivideAssign,
            AssignOperator::RemainderAssign => TokenType::RemainderAssign,

            // shift
            AssignOperator::ShiftLeftAssign => TokenType::ShiftLeftAssign,
            AssignOperator::SaturatingShiftLeftAssign => TokenType::SaturatingShiftLeftAssign,
            AssignOperator::ShiftRightAssign => TokenType::ShiftRightAssign,

            // bitwise
            AssignOperator::BitwiseAndAssign => TokenType::BitwiseAndAssign,
            AssignOperator::BitwiseOrAssign => TokenType::BitwiseOrAssign,
            AssignOperator::BitwiseXorAssign => TokenType::BitwiseXorAssign,

            // logical
            AssignOperator::LogicalAndAssign => TokenType::LogicalAndAssign,
            AssignOperator::LogicalOrAssign => TokenType::LogicalOrAssign,
        }
    }
}

impl InfixOperator {
    /// Get the precedence of the infix operator.
    #[inline]
    pub fn precedence_group(&self) -> OperatorPrecedence {
        match self {
            InfixOperator::Binary(binary_operator) => binary_operator.precedence_group(),
            InfixOperator::Assign(assign_operator) => assign_operator.precedence_group(),
        }
    }

    /// Get the precedence of the infix operator.
    #[inline]
    pub fn precedence(self) -> u8 {
        match self {
            InfixOperator::Binary(binary_operator) => binary_operator.precedence(),
            InfixOperator::Assign(assign_operator) => assign_operator.precedence(),
        }
    }
}

impl<'a> Parser<'a> {
    /// Peek a unary operator.
    #[inline]
    pub fn peek_unary_operator(&self) -> ParseResult<UnaryOperator> {
        let token = self.peek()?;
        UnaryOperator::from_token_type(token.token.r#type)
            .ok_or(ParseError::UnexpectedToken(token.span))
    }

    /// Peek a binary operator.
    #[inline]
    pub fn peek_binary_operator(&self) -> ParseResult<BinaryOperator> {
        let token = self.peek()?;
        BinaryOperator::from_token_type(token.token.r#type)
            .ok_or(ParseError::UnexpectedToken(token.span))
    }

    /// Peek an assign operator.
    #[inline]
    pub fn peek_assign_operator(&self) -> ParseResult<AssignOperator> {
        let token = self.peek()?;
        AssignOperator::from_token_type(token.token.r#type)
            .ok_or(ParseError::UnexpectedToken(token.span))
    }

    /// Peek an infix operator.
    #[inline]
    pub fn peek_infix_operator(&self) -> ParseResult<InfixOperator> {
        let token = self.peek()?;
        if let Some(binary_operator) = BinaryOperator::from_token_type(token.token.r#type) {
            Ok(InfixOperator::Binary(binary_operator))
        } else if let Some(assign_operator) = AssignOperator::from_token_type(token.token.r#type) {
            Ok(InfixOperator::Assign(assign_operator))
        } else {
            Err(ParseError::UnexpectedToken(token.span))
        }
    }

    /// Make an expression from an infix operator.
    #[inline]
    fn make_infix_expression(
        &self,
        left: NodeId<Expression>,
        operator: InfixOperator,
        right: NodeId<Expression>,
    ) -> Expression {
        match operator {
            InfixOperator::Binary(binary_operator) => Expression::Binary {
                left,
                operator: binary_operator,
                right,
            },
            InfixOperator::Assign(assign_operator) => Expression::Assign {
                left,
                operator: assign_operator,
                right,
            },
        }
    }

    /// Eat an expression.
    pub fn eat_expression(
        &mut self,
        left_precedence: Option<u8>,
    ) -> ParseResult<NodeId<Expression>> {
        let start = self.mark();

        // runtime
        // (not a unary prefix, just gets merged into relevant expression)
        let runtime: Runtime = if self.peek_token(TokenType::At).is_ok() {
            self.bump(); // eat @
            Runtime::Static
        } else {
            Runtime::Dynamic
        };

        let mut left_expression_id: NodeId<Expression> = {
            //
            // ------------------------------------------------------------
            // Grouping
            // ------------------------------------------------------------
            //

            // parenthesis
            if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                self.bump(); // eat open paranthesis
                // parse inner expressions with reset precedence (new precedence "scope")
                let expression_id = self.eat_expression(None)?;
                self.eat_token(TokenType::CloseParenthesis)?;
                self.tree.set_span(expression_id, self.get_span_from(start));
                expression_id
            }
            //
            // ------------------------------------------------------------
            // Unary operations (prefix, right associative)
            // ------------------------------------------------------------
            //

            // unary operations
            else if let Ok(unary_operator) = self.peek_unary_operator() {
                let right_precedence = unary_operator.precedence();
                self.bump(); // eat unary operator (always because right associative)
                let right = self.eat_expression(Some(right_precedence))?;
                let expression = Expression::Unary {
                    operator: unary_operator,
                    right,
                };
                self.tree.allocate(expression, self.get_span_from(start))
            }
            //
            // ------------------------------------------------------------
            // Declarations
            // ------------------------------------------------------------
            //

            // module
            else if self.peek_keyword(Keyword::Module).is_ok() {
                let module_id = self.eat_module()?;
                let expression = Expression::Module(module_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // struct
            else if self.peek_keyword(Keyword::Struct).is_ok() {
                let struct_id = self.eat_struct()?;
                let expression = Expression::Struct(struct_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // enum
            else if self.peek_keyword(Keyword::Enum).is_ok() {
                let enum_id = self.eat_enum()?;
                let expression = Expression::Enum(enum_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // union
            else if self.peek_keyword(Keyword::Union).is_ok() {
                let union_id = self.eat_union()?;
                let expression = Expression::Union(union_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // trait
            else if self.peek_keyword(Keyword::Trait).is_ok() {
                let trait_id = self.eat_trait()?;
                let expression = Expression::Trait(trait_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // implement
            else if self.peek_keyword(Keyword::Implement).is_ok() {
                let implement_id = self.eat_implement()?;
                let expression = Expression::Implement(implement_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // function
            else if self.peek_keyword(Keyword::Function).is_ok() {
                let function_id = self.eat_function()?;
                let expression = Expression::Function(function_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            //
            // ------------------------------------------------------------
            // Control flow
            // ------------------------------------------------------------
            //
            // if
            else if self.peek_keyword(Keyword::If).is_ok() {
                let if_id = self.eat_if()?;
                let expression = Expression::If(if_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // while
            else if self.peek_keyword(Keyword::While).is_ok() {
                let while_id = self.eat_while()?;
                let expression = Expression::While(while_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // for
            else if self.peek_keyword(Keyword::For).is_ok() {
                let for_id = self.eat_for()?;
                let expression = Expression::For(for_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // loop
            else if self.peek_keyword(Keyword::Loop).is_ok() {
                let loop_id = self.eat_loop()?;
                let expression = Expression::Loop(loop_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // break
            else if self.peek_keyword(Keyword::Break).is_ok() {
                let break_id = self.eat_break()?;
                let expression = Expression::Break(break_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // continue
            else if self.peek_keyword(Keyword::Continue).is_ok() {
                let continue_id = self.eat_continue()?;
                let expression = Expression::Continue(continue_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // defer
            else if self.peek_keyword(Keyword::Defer).is_ok() {
                let defer_id = self.eat_defer()?;
                let expression = Expression::Defer(defer_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // return
            else if self.peek_keyword(Keyword::Return).is_ok() {
                let return_id = self.eat_return()?;
                let expression = Expression::Return(return_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // try
            else if self.peek_keyword(Keyword::Try).is_ok() {
                let try_id = self.eat_try_catch()?;
                let expression = Expression::Try(try_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // match
            else if self.peek_keyword(Keyword::Match).is_ok() {
                let match_id = self.eat_match()?;
                let expression = Expression::Match(match_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            //
            // ------------------------------------------------------------
            // Bindings / Literals / Aliases
            // ------------------------------------------------------------
            //
            // let
            else if self.peek_keyword(Keyword::Let).is_ok()
                || self.peek_keyword(Keyword::Var).is_ok()
            {
                let let_id = self.eat_let_or_var()?;
                let expression = Expression::Let(let_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // tuple
            else if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                let tuple_literal = self.eat_tuple_literal()?;
                let expression = Expression::TupleLiteral(tuple_literal);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // array
            else if self.peek_token(TokenType::OpenBracket).is_ok() {
                let array_literal = self.eat_array_literal()?;
                let expression = Expression::ArrayLiteral(array_literal);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // struct
            else if self.peek_struct_literal().is_ok() {
                let struct_literal = self.eat_struct_literal()?;
                let expression = Expression::StructLiteral(struct_literal);
                self.tree.allocate(expression, self.get_span_from(start))
            // scalar
            } else if self.peek_scalar_literal().is_ok() {
                let scalar_literal = self.eat_scalar_literal()?;
                let expression = Expression::ScalarLiteral(scalar_literal);
                self.tree.allocate(expression, self.get_span_from(start))
            // alias / path
            } else if self.peek_identifier().is_ok() {
                let path_id = self.eat_path()?;
                let expression = Expression::Path { path: path_id };
                self.tree.allocate(expression, self.get_span_from(start))
            // _
            } else {
                let expression = Expression::Error;
                self.tree.allocate(expression, self.get_span_from(start))
            }
        };

        //
        // ------------------------------------------------------------
        // Postfix operations
        // ------------------------------------------------------------
        //

        // eat all postfix operations
        loop {
            // index
            if self.peek_token(TokenType::OpenBracket).is_ok() {
                let index_id = self.eat_index_postfix(left_expression_id)?;
                let expression = Expression::Index(index_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // call
            else if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                let call_id = self.eat_call_postfix(left_expression_id, runtime)?;
                let expression = Expression::Call(call_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // cast
            else if self.peek_keyword(Keyword::As).is_ok() {
                let cast_id = self.eat_as_postfix(left_expression_id)?;
                let expression = Expression::Cast(cast_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // unwrap
            else if self.peek_token(TokenType::Question).is_ok() {
                self.bump(); // eat question
                let expression = Expression::Unwrap(left_expression_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // done
            else {
                break;
            }
        }

        //
        // ------------------------------------------------------------
        // Infix operations (binary and assign, left associative)
        // ------------------------------------------------------------
        //

        // eating infix while left precedence is weaker than right precedence
        while let Ok(right_operator) = self.peek_infix_operator()
            && (left_precedence.is_none() || left_precedence.unwrap() < right_operator.precedence())
        {
            self.bump(); // eat infix operator
            let right_expression_id = self.eat_expression(Some(right_operator.precedence()))?;
            let left_expression =
                self.make_infix_expression(left_expression_id, right_operator, right_expression_id);
            left_expression_id = self
                .tree
                .allocate(left_expression, self.get_span_from(start))
        }

        Ok(left_expression_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{
        BinaryOperator, Call, Expression, FieldLiteral, Runtime, ScalarLiteral, StructLiteral,
        Type, assert_node, assert_path,
    };

    /// Struct literals are disambiguated.
    /// geom.Vector2 { x: 1, y }
    #[test]
    #[ignore]
    fn test_parse_struct_literal_path() {
        let test = TestParser::new("geom.Vector2 { x: 1, y }");
        let mut parser = test.parser();

        let expr_id = parser.eat_expression(None).unwrap();
        let geom = parser.strings.intern("geom");
        let vector2 = parser.strings.intern("Vector2");
        let x = parser.strings.intern("x");
        let y = parser.strings.intern("y");

        // geom.Vector2 { x: 1, y }
        assert_node!(
            parser.tree,
            expr_id,
            Expression::StructLiteral(struct_literal_id) => {
                assert_node!(
                    parser.tree,
                    *struct_literal_id,
                    StructLiteral { r#type, fields } => {
                        // geom.Vector2
                        assert_node!(
                            parser.tree,
                            *r#type,
                            Type::Path { path, static_arguments: _ } => {
                                let p = parser.paths.get(*path);
                                assert_eq!(p.segments.len(), 2);
                                assert_eq!(p.segments[0], geom);
                                assert_eq!(p.segments[1], vector2);
                            }
                        );
                        // fields
                        assert_eq!(fields.len(), 2);
                        // x: 1
                        assert_node!(
                            parser.tree,
                            fields[0],
                            FieldLiteral::Named { name, value } => {
                                // x
                                assert_eq!(*name, x);
                                // 1
                                assert_node!(
                                    parser.tree,
                                    *value,
                                    Expression::ScalarLiteral(scalar_id) => {
                                        assert_node!(
                                            parser.tree,
                                            *scalar_id,
                                            ScalarLiteral::Integer(1, _)
                                        );
                                    }
                                );
                            }
                        );
                        // y
                        assert_node!(
                            parser.tree,
                            fields[1],
                            FieldLiteral::Named { name, value: _ } => {
                                // y
                                assert_eq!(*name, y);
                            }
                        );
                    }
                );
            }
        );
    }

    /// Struct literals with static parameters are disambiguated.
    /// geom.Mesh<Dims: 2, DType: float32> {
    ///
    ///     vertices: [1,]
    ///     y  
    /// }
    #[test]
    #[ignore]
    fn test_parse_struct_literal_path_with_static_parameters() {
        let test = TestParser::new(
            r##"
geom.Mesh<Dims: 2, DType: float32> { 
    vertices: [1,]
    y
}"##,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let _expr_id = parser.eat_expression(None).unwrap();
        // NOTE :Incomplete: struct literals with generics
    }

    /// Addition is left associative.
    /// a + b + c
    /// => ((a + b) + c)
    #[test]
    fn test_parse_precedence_addition_left_associative() {
        let test = TestParser::new("a + b + c");
        let mut parser = test.parser();
        let expr_id = parser.eat_expression(None).unwrap();

        let a = parser.strings.intern("a");
        let b = parser.strings.intern("b");
        let c = parser.strings.intern("c");

        assert_node!(
            parser.tree,
            expr_id,
            // ((a + b) + c)
            Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::Add);
                assert_node!(
                    parser.tree,
                    *left,
                    // (a + b)
                    Expression::Binary { left, operator, right } => {
                        // a
                        assert_path!(parser.tree, *left, a, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                        // +
                        assert_eq!(*operator, BinaryOperator::Add);
                        // b
                        assert_path!(parser.tree, *right, b, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                    }
                );
                // c
                assert_path!(parser.tree, *right, c, using |path_id| {
                    let p = parser.paths.get(path_id);
                    assert_eq!(p.segments.len(), 1);
                    p.segments[0]
                });
            }
        );
    }
    /// Multiplication has higher precedence than addition.
    /// a + b * c
    /// => (a + (b * c))
    #[test]
    fn test_parse_precedence_multiply_before_addition() {
        let test = TestParser::new("a + b * c");
        let mut parser = test.parser();
        let expr_id = parser.eat_expression(None).unwrap();

        let a = parser.strings.intern("a");
        let b = parser.strings.intern("b");
        let c = parser.strings.intern("c");

        assert_node!(
            parser.tree,
            expr_id,
            // (a + (b * c))
            Expression::Binary { left, operator, right } => {
                // +
                assert_eq!(*operator, BinaryOperator::Add);
                // a
                assert_path!(parser.tree, *left, a, using |path_id| {
                    let p = parser.paths.get(path_id);
                    assert_eq!(p.segments.len(), 1);
                    p.segments[0]
                });
                assert_node!(
                    parser.tree,
                    *right,
                    // (b * c)
                    Expression::Binary { left, operator, right } => {
                        // *
                        assert_eq!(*operator, BinaryOperator::Multiply);
                        // b
                        assert_path!(parser.tree, *left, b, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                        // c
                        assert_path!(parser.tree, *right, c, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                    }
                );
            }
        );
    }

    /// Parentheses override operator precedence.
    /// (a + b) * c
    /// => ((a + b) * c)
    #[test]
    fn test_parse_precedence_parentheses_override() {
        let test = TestParser::new("(a + b) * c");
        let mut parser = test.parser();
        let expr_id = parser.eat_expression(None).unwrap();

        let a = parser.strings.intern("a");
        let b = parser.strings.intern("b");
        let c = parser.strings.intern("c");

        assert_node!(
            parser.tree,
            expr_id,
            // ((a + b) * c)
            Expression::Binary { left, operator, right } => {
                // *
                assert_eq!(*operator, BinaryOperator::Multiply);
                assert_node!(
                    parser.tree,
                    *left,
                    // (a + b)
                    Expression::Binary { left, operator, right } => {
                        // +
                        assert_eq!(*operator, BinaryOperator::Add);
                        // a
                        assert_path!(parser.tree, *left, a, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                        // b
                        assert_path!(parser.tree, *right, b, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                    }
                );
                // c
                assert_path!(parser.tree, *right, c, using |path_id| {
                    let p = parser.paths.get(path_id);
                    assert_eq!(p.segments.len(), 1);
                    p.segments[0]
                });
            }
        );
    }

    /// Mixed precedence chain with addition and multiplication.
    /// a + b * c + d
    /// => ((a + (b * c)) + d)
    #[test]
    fn test_parse_precedence_chain_mixed() {
        let test = TestParser::new("a + b * c + d");
        let mut parser = test.parser();
        let expr_id = parser.eat_expression(None).unwrap();

        let a = parser.strings.intern("a");
        let b = parser.strings.intern("b");
        let c = parser.strings.intern("c");
        let d = parser.strings.intern("d");

        assert_node!(
            parser.tree,
            expr_id,
            // ((a + (b * c)) + d)
            Expression::Binary { left, operator, right } => {
                // +
                assert_eq!(*operator, BinaryOperator::Add);
                assert_node!(
                    parser.tree,
                    *left,
                    // (a + (b * c))
                    Expression::Binary { left, operator, right } => {
                        // +
                        assert_eq!(*operator, BinaryOperator::Add);
                        // a
                        assert_path!(parser.tree, *left, a, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                        assert_node!(
                            parser.tree,
                            *right,
                            // (b * c)
                            Expression::Binary { left, operator, right } => {
                                // *
                                assert_eq!(*operator, BinaryOperator::Multiply);
                                // b
                                assert_path!(parser.tree, *left, b, using |path_id| {
                                    let p = parser.paths.get(path_id);
                                    assert_eq!(p.segments.len(), 1);
                                    p.segments[0]
                                });
                                // c
                                assert_path!(parser.tree, *right, c, using |path_id| {
                                    let p = parser.paths.get(path_id);
                                    assert_eq!(p.segments.len(), 1);
                                    p.segments[0]
                                });
                            }
                        );
                    }
                );
                // d
                assert_path!(parser.tree, *right, d, using |path_id| {
                    let p = parser.paths.get(path_id);
                    assert_eq!(p.segments.len(), 1);
                    p.segments[0]
                });
            }
        );
    }

    /// Addition has higher precedence than bitwise or.
    /// a + b | c + d
    /// => ((a + b) | (c + d))
    #[test]
    fn test_parse_precedence_bitwise_vs_addition() {
        let test = TestParser::new("a + b | c + d");
        let mut parser = test.parser();
        let expr_id = parser.eat_expression(None).unwrap();

        let a = parser.strings.intern("a");
        let b = parser.strings.intern("b");
        let c = parser.strings.intern("c");
        let d = parser.strings.intern("d");

        assert_node!(
            parser.tree,
            expr_id,
            // ((a + b) | (c + d))
            Expression::Binary { left, operator, right } => {
                // |
                assert_eq!(*operator, BinaryOperator::BitwiseOr);
                assert_node!(
                    parser.tree,
                    *left,
                    // (a + b)
                    Expression::Binary { left, operator, right } => {
                        // +
                        assert_eq!(*operator, BinaryOperator::Add);
                        // a
                        assert_path!(parser.tree, *left, a, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                        // b
                        assert_path!(parser.tree, *right, b, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                    }
                );
                assert_node!(
                    parser.tree,
                    *right,
                    // (c + d)
                    Expression::Binary { left, operator, right } => {
                        // +
                        assert_eq!(*operator, BinaryOperator::Add);
                        // c
                        assert_path!(parser.tree, *left, c, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                        // d
                        assert_path!(parser.tree, *right, d, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                    }
                );
            }
        );
    }

    /// Comparison has higher precedence than logical and.
    /// a == b && c == d
    /// => ((a == b) && (c == d))
    #[test]
    fn test_parse_precedence_comparison_vs_logical() {
        let test = TestParser::new("a == b && c == d");
        let mut parser = test.parser();
        let expr_id = parser.eat_expression(None).unwrap();

        let a = parser.strings.intern("a");
        let b = parser.strings.intern("b");
        let c = parser.strings.intern("c");
        let d = parser.strings.intern("d");

        assert_node!(
            parser.tree,
            expr_id,
            // ((a == b) && (c == d))
            Expression::Binary { left, operator, right } => {
                // &&
                assert_eq!(*operator, BinaryOperator::LogicalAnd);
                assert_node!(
                    parser.tree,
                    *left,
                    // (a == b)
                    Expression::Binary { left, operator, right } => {
                        // ==
                        assert_eq!(*operator, BinaryOperator::Equal);
                        // a
                        assert_path!(parser.tree, *left, a, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                        // b
                        assert_path!(parser.tree, *right, b, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                    }
                );
                assert_node!(
                    parser.tree,
                    *right,
                    // (c == d)
                    Expression::Binary { left, operator, right } => {
                        // ==
                        assert_eq!(*operator, BinaryOperator::Equal);
                        // c
                        assert_path!(parser.tree, *left, c, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                        // d
                        assert_path!(parser.tree, *right, d, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                    }
                );
            }
        );
    }

    /// Unary prefix has higher precedence than multiplication.
    /// -a * b
    /// => ((-a) * b)
    #[test]
    fn test_parse_precedence_unary_before_multiply() {
        let test = TestParser::new("-a * b");
        let mut parser = test.parser();
        let expr_id = parser.eat_expression(None).unwrap();

        let a = parser.strings.intern("a");
        let b = parser.strings.intern("b");

        assert_node!(
            parser.tree,
            expr_id,
            // ((-a) * b)
            Expression::Binary { left, operator, right } => {
                // *
                assert_eq!(*operator, BinaryOperator::Multiply);
                assert_node!(
                    parser.tree,
                    *left,
                    // (-a)
                    Expression::Unary { operator: _, right } => {
                        // a
                        assert_path!(parser.tree, *right, a, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                    }
                );
                // b
                assert_path!(parser.tree, *right, b, using |path_id| {
                    let p = parser.paths.get(path_id);
                    assert_eq!(p.segments.len(), 1);
                    p.segments[0]
                });
            }
        );
    }

    /// Postfix call has higher precedence than addition.
    /// Static calls are right associative.
    /// a() + @b() / c
    /// => ((a()) + ((@b()) / c))
    #[test]
    fn test_parse_precedence_postfix_call_before_add() {
        let test = TestParser::new("a() + @b() / c");
        let mut parser = test.parser();
        let expr_id = parser.eat_expression(None).unwrap();

        let a = parser.strings.intern("a");
        let b = parser.strings.intern("b");
        let c = parser.strings.intern("c");

        assert_node!(
            parser.tree,
            expr_id,
            // ((a()) + ((@b()) / b))
            Expression::Binary { left, operator, right } => {
                // +
                assert_eq!(*operator, BinaryOperator::Add);
                // (a())
                assert_node!(
                    parser.tree,
                    *left,
                    // a()
                    Expression::Call(call_id) => {
                        assert_node!(
                            parser.tree,
                            *call_id,
                            Call { runtime, receiver, static_arguments: _, dynamic_arguments: _ } => {
                                assert_eq!(*runtime, Runtime::Dynamic);
                                // a
                                assert_path!(parser.tree, *receiver, a, using |path_id| {
                                    let path = parser.paths.get(path_id);
                                    assert_eq!(path.segments.len(), 1);
                                    path.segments[0]
                                });
                            }
                        );
                    }
                );
                // ((@b()) / c)
                assert_node!(
                    parser.tree,
                    *right,
                    Expression::Binary { left, operator, right } => {
                        // /
                        assert_eq!(*operator, BinaryOperator::Divide);
                        // (@b())
                        assert_node!(
                            parser.tree,
                            *left,
                            Expression::Call(call_id) => {
                                assert_node!(
                                    parser.tree,
                                    *call_id,
                                    Call { runtime, receiver, static_arguments: _, dynamic_arguments: _ } => {
                                        assert_eq!(*runtime, Runtime::Static);
                                        // b
                                        assert_path!(parser.tree, *receiver, b, using |path_id| {
                                            let path = parser.paths.get(path_id);
                                            assert_eq!(path.segments.len(), 1);
                                            path.segments[0]
                                        });
                                    }
                                );
                            }
                        );
                        // c
                        assert_path!(parser.tree, *right, c, using |path_id| {
                            let path = parser.paths.get(path_id);
                            assert_eq!(path.segments.len(), 1);
                            path.segments[0]
                        });
                    }
                );
            }
        );
    }
}
