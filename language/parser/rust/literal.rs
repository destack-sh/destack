use destack_language_lexer::TokenType;

use crate::{
    ArrayLiteral, Expression, ParseResult, Parser, ScalarLiteral, StructLiteral, TupleLiteral,
};

impl<'a> Parser<'a> {
    /// Eat a literal.
    pub fn eat_literal(&mut self) -> ParseResult<Expression> {
        // array
        if self.peek_next_token(TokenType::OpenBracket).is_ok() {
            let array_literal = self.eat_array_literal()?;
            Ok(Expression::ArrayLiteral(array_literal))
        // tuple
        } else if self.peek_next_token(TokenType::OpenParenthesis).is_ok() {
            let tuple_literal = self.eat_tuple_literal()?;
            Ok(Expression::TupleLiteral(tuple_literal))
        // struct
        } else if self.peek_next_token(TokenType::OpenBrace).is_ok() {
            let struct_literal = self.eat_struct_literal()?;
            Ok(Expression::StructLiteral(struct_literal))
        // scalar
        } else {
            let scalar_literal = self.eat_scalar_literal()?;
            Ok(Expression::ScalarLiteral(scalar_literal))
        }
    }

    /// Eat a scalar literal.
    ///
    /// Examples:
    /// ```
    /// 1
    /// 1.0f64
    /// 7f32
    /// "Hello, world!"
    /// 'a'
    /// b'a'
    /// b"abc"
    /// 0x1234
    /// true
    /// false
    /// ```
    pub fn eat_scalar_literal(&mut self) -> ParseResult<ScalarLiteral> {
        todo!()
    }

    /// Eat an array literal (fixed or repeated).
    /// The individual elements are full expressions, not just literals.
    ///
    /// Examples:
    /// ```
    /// [1, 2, 3]
    /// [1.0f64, 2.0f64, 3.0f64]
    /// [10, false, "Hi"] // hetereogenous array is invalid but okay in AST
    /// [0; 10] // repeated array
    /// [false; 40] // repeated array
    pub fn eat_array_literal(&mut self) -> ParseResult<ArrayLiteral> {
        self.eat_token(TokenType::OpenBracket)?;
        let mut elements: Vec<Expression> = vec![];
        while self.peek_next_token(TokenType::Comma).is_ok() {
            self.eat_token(TokenType::Comma)?;
            let element = self.eat_expression()?;
            elements.push(element);
        }
        self.eat_token(TokenType::CloseBracket)?;
        Ok(ArrayLiteral::Fixed { elements })
    }

    /// Eat a tuple literal.
    /// The individual elements are full expressions, not just literals.
    ///
    /// Examples:
    /// ```
    /// (1, 2, 3)
    /// (1.0f64, 2.0f64, 3.0f64)
    /// (10, false, "Hi") // okay because it's a tuple
    /// ```
    pub fn eat_tuple_literal(&mut self) -> ParseResult<TupleLiteral> {
        self.eat_token(TokenType::OpenParenthesis)?;
        let mut elements: Vec<Expression> = vec![];
        while self.peek_next_token(TokenType::Comma).is_ok() {
            self.eat_token(TokenType::Comma)?;
            let element = self.eat_expression()?;
            elements.push(element);
        }
        self.eat_token(TokenType::CloseParenthesis)?;
        Ok(TupleLiteral { elements })
    }

    /// Eat a struct literal.
    ///
    /// Examples:
    /// ```
    /// Vector2 { x: 1.0f64, y: 2.0f64 }
    ///
    /// destack.geometry.Mesh2 {
    ///     vertices: [Vector3 { x: 1.0f64, y: 2.0f64, z: 3.0f64 }],
    ///     indices: [0, 1, 2]
    /// }
    /// ```
    pub fn eat_struct_literal(&mut self) -> ParseResult<StructLiteral> {
        todo!()
    }
}

#[cfg(test)]
mod tests {}
