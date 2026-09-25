use crate::parse::{ExpressionPosition, ExpressionStop, TypePosition, TypeStop};
use crate::{ParseStart, Parser, ParserError, ParserResult};
use tspp_dir::{
    Expression, InferForm, Keyword, LocalNodeId, NodeType, TokenType, TupleElement, TupleForm,
    TypeExpression,
};

impl Parser {
    /// Parse one parenthesized type, tuple, or function head.
    pub(super) fn parse_parenthesized_type(
        &mut self,
        start: &ParseStart,
        stop: TypeStop,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        self.eat_token(TokenType::OpenParenthesis)?;

        // empty tuple
        if self.peek_is(TokenType::CloseParenthesis) {
            return self.finish_tuple_type(start, Vec::new(), TupleForm::Tuple);
        }

        // tuple head
        if self.peek_type_tuple() {
            let elements = self.parse_type_tuple_elements(stop, TokenType::CloseParenthesis)?;

            return self.finish_tuple_type(start, elements, TupleForm::Tuple);
        }

        // first type
        let ty = self.parse_type_or_recover_missing(
            TypePosition::Type,
            stop.nest(),
            NodeType::TypeExpression,
        )?;

        // tuple tail
        if self.peek_is(TokenType::Comma)
            || self.peek_optional_tuple_element(TokenType::CloseParenthesis)
        {
            let elements =
                self.parse_type_tuple_tail(start, ty, stop, TokenType::CloseParenthesis)?;

            return self.finish_tuple_type(start, elements, TupleForm::Tuple);
        }

        // grouped type
        self.eat_close_token_or_recover_missing(
            TokenType::CloseParenthesis,
            NodeType::TypeExpression,
        )?;

        self.record_parentheses(start, ty);

        Ok(ty)
    }

    /// Close and insert one parsed tuple type.
    fn finish_tuple_type(
        &mut self,
        start: &ParseStart,
        elements: Vec<LocalNodeId<TupleElement>>,
        form: TupleForm,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let close = match form {
            TupleForm::Tuple => TokenType::CloseParenthesis,
            TupleForm::Array => TokenType::CloseBracket,
        };

        // consume the matching delimiter
        self.eat_close_token_or_recover_missing(close, NodeType::TypeExpression)?;

        Ok(self.insert_node(
            TypeExpression::Tuple { form, elements },
            self.range_since(start),
        ))
    }

    /// Return whether the current token starts a constructor type expression.
    pub(super) fn peek_construct_type(&self) -> bool {
        if self.peek_keyword() == Some(Keyword::New) {
            return matches!(
                self.peek_next_token_type(),
                TokenType::LessThan | TokenType::OpenParenthesis
            );
        }

        if self.peek_keyword_at(0) == Some(Keyword::Abstract)
            && self.peek_keyword_at(1) == Some(Keyword::New)
        {
            return matches!(
                self.peek_token_type_at(2),
                TokenType::LessThan | TokenType::OpenParenthesis
            );
        }

        false
    }

    /// Return whether the current parenthesis group is a function type head.
    pub(super) fn peek_parenthesized_function_type(
        &self,
        position: TypePosition,
        stop: TypeStop,
    ) -> bool {
        if self.peek_token_type() != TokenType::OpenParenthesis {
            return false;
        }

        let Some(follow) =
            self.peek_token_after_group(0, TokenType::OpenParenthesis, TokenType::CloseParenthesis)
        else {
            return false;
        };

        if follow.is(TokenType::Colon) {
            return !stop.has(TypeStop::CONDITIONAL_COLON);
        }
        if !follow.is(TokenType::ArrowWide) {
            return false;
        }

        position != TypePosition::ArrowReturn || self.peek_parenthesized_parameter_list()
    }

    /// Parse one slice, fixed-array, or repeated Pattern placeholder type.
    pub(super) fn parse_bracket_type(
        &mut self,
        start: &ParseStart,
        stop: TypeStop,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        self.eat_token(TokenType::OpenBracket)?;

        // repeated Pattern placeholder
        if self.peek_repeated_pattern_marker() {
            let elements = self.parse_type_tuple_elements(stop, TokenType::CloseBracket)?;

            return self.finish_tuple_type(start, elements, TupleForm::Array);
        }

        // empty, labeled, or spread bracket tuple
        if self.peek_is(TokenType::CloseBracket) || self.peek_type_tuple() {
            let elements = self.parse_type_tuple_elements(stop, TokenType::CloseBracket)?;

            return self.reject_bracket_tuple_type(start, elements);
        }

        // element type
        let element = self.parse_type_or_recover_missing(
            TypePosition::Type,
            stop.nest(),
            NodeType::TypeExpression,
        )?;

        // fixed array
        if self.peek_is(TokenType::Semicolon) {
            return self.parse_fixed_array_type(start, element);
        }

        // comma or optional marker opens a bracket tuple
        if self.peek_is(TokenType::Comma)
            || self.peek_optional_tuple_element(TokenType::CloseBracket)
        {
            let elements =
                self.parse_type_tuple_tail(start, element, stop, TokenType::CloseBracket)?;

            return self.reject_bracket_tuple_type(start, elements);
        }

        // slice close
        self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::TypeExpression)?;

        Ok(self.insert_node(TypeExpression::Slice { element }, self.range_since(start)))
    }

    /// Close and reject one tuple type written with brackets.
    fn reject_bracket_tuple_type(
        &mut self,
        start: &ParseStart,
        elements: Vec<LocalNodeId<TupleElement>>,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::TypeExpression)?;

        // report the tuple over its whole bracket group
        let range = self.range_since(start);
        self.report_error(ParserError::bracket_tuple_type(range));

        Ok(self.insert_node(
            TypeExpression::Tuple {
                form: TupleForm::Array,
                elements,
            },
            range,
        ))
    }

    /// Parse one fixed-array type after its element.
    fn parse_fixed_array_type(
        &mut self,
        start: &ParseStart,
        element: LocalNodeId<TypeExpression>,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        self.eat_token(TokenType::Semicolon)?;

        // parse the fixed length or inference hole
        let length = if self.peek_identifier_is("_") {
            let length_start = self.mark_parse_start();
            self.bump();

            self.insert_node(
                Expression::Infer {
                    form: InferForm::Hole,
                    name: None,
                },
                self.range_since(&length_start),
            )
        } else {
            self.parse_expression_or_recover_missing(
                ExpressionPosition::Value,
                ExpressionStop::default(),
                NodeType::Expression,
            )?
        };

        // close the fixed array
        self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::TypeExpression)?;

        Ok(self.insert_node(
            TypeExpression::FixedArray { element, length },
            self.range_since(start),
        ))
    }
}
