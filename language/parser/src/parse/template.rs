use crate::parse::ExpressionStop;
use tspp_dir::{
    Argument, Expression, LocalNodeId, StringId, TemplateChunk, TemplateLiteral, Token, TokenSpan,
    TokenType, TypeExpression,
};
use tspp_source::ByteRange;

use crate::lex::{InvalidEscape, cook};
use crate::parse::{ExpressionPosition, TypePosition, TypeStop};
use crate::{Parser, ParserError, ParserResult};

impl Parser {
    /// Peek a template literal.
    #[inline]
    pub fn peek_template_literal(&self) -> ParserResult<TokenSpan> {
        if self.peek_is(TokenType::TemplateString) || self.peek_is(TokenType::TemplateStringStart) {
            Ok(self.peek_token_span())
        } else {
            Err(ParserError::unexpected(self.peek_token_span()))
        }
    }

    /// Parse a template literal.
    ///
    /// Examples:
    /// ```tspp
    /// `hello`
    /// `hello ${name}`
    /// `SELECT * FROM users`
    /// `${stmt}`
    /// `SELECT * FROM users WHERE name = ${name}` AND age > ${group.age()} LIMIT 10`
    /// ```
    pub(crate) fn parse_template_literal(&mut self) -> ParserResult<TemplateLiteral> {
        self.parse_expression_template(false)
    }

    /// Parse a tagged template literal.
    ///
    /// Examples:
    /// ```tspp
    /// sql`SELECT * FROM users WHERE id = ${id}`
    /// ```
    pub(crate) fn parse_tagged_template_literal(&mut self) -> ParserResult<TemplateLiteral> {
        self.parse_expression_template(true)
    }

    /// Parse a value-space template with the selected escape rules.
    fn parse_expression_template(&mut self, is_tagged: bool) -> ParserResult<TemplateLiteral> {
        let (chunks, arguments) =
            self.parse_template_chunks(is_tagged, |parser| parser.parse_template_argument())?;

        if arguments.is_empty() && chunks.len() == 1 {
            Ok(TemplateLiteral::String { chunk: chunks[0] })
        } else {
            Ok(TemplateLiteral::InterpolatedString { chunks, arguments })
        }
    }

    /// Parse a type template literal.
    ///
    /// Examples:
    /// ```tspp
    /// `${K}`
    /// `foo-${Bar}`
    /// ```
    pub(crate) fn parse_type_template_literal(
        &mut self,
        stop: TypeStop,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let start = self.mark_parse_start();
        let (chunks, spans) = self.parse_template_chunks(false, |parser| {
            parser.parse_type(TypePosition::Type, stop.nest())
        })?;

        // type templates are never tagged, so every chunk decoded
        let strings = chunks
            .into_iter()
            .map(|chunk| chunk.cooked)
            .collect::<Option<Vec<StringId>>>()
            .ok_or_else(|| ParserError::unexpected(self.range_since(&start)))?;
        let expression = TypeExpression::TemplateLiteral { strings, spans };
        let expression_id = self.insert_node(expression, self.range_since(&start));

        // the template head is the first interpolation head when present
        if let TypeExpression::TemplateLiteral { spans, .. } = self.tree.get(expression_id)
            && let Some(first_span_expression_id) = spans.first()
        {
            let head_span = self.type_expression_head_range(*first_span_expression_id);
            self.tree.set_head_range(expression_id, head_span);
        }

        Ok(expression_id)
    }

    /// Parse a template literal body.
    fn parse_template_chunks<T>(
        &mut self,
        is_tagged: bool,
        mut parse_span: impl FnMut(&mut Parser) -> ParserResult<T>,
    ) -> ParserResult<(Vec<TemplateChunk>, Vec<T>)> {
        let next = self.eat();
        let next_str = self.file.span_str(next.span);

        // template string without interpolation
        if next.token.ty() == TokenType::TemplateString {
            let string = Self::template_chunk_body(next.token, next_str, 1, 1)?.to_owned();
            let chunk = self.template_chunk(next.span.range(), &string, is_tagged)?;

            return Ok((vec![chunk], Vec::new()));
        }

        // template string with interpolation
        if next.token.ty() == TokenType::TemplateStringStart {
            let mut chunks: Vec<TemplateChunk> = Vec::new();
            let mut spans: Vec<T> = Vec::new();

            // start chunk: remove ` prefix and ${ suffix
            let string = Self::template_chunk_body(next.token, next_str, 1, 2)?.to_owned();
            chunks.push(self.template_chunk(next.span.range(), &string, is_tagged)?);

            // eat until the end
            while !self.peek_is(TokenType::TemplateStringEnd) {
                // middle chunk: remove } prefix and ${ suffix
                if self.peek_is(TokenType::TemplateStringMiddle) {
                    let token = self.eat();
                    let token_str = self.file.span_str(token.span);
                    let string =
                        Self::template_chunk_body(token.token, token_str, 1, 2)?.to_owned();
                    chunks.push(self.template_chunk(token.span.range(), &string, is_tagged)?);
                }
                // interpolation expression
                else {
                    let span = parse_span(self)?;
                    if !self.peek_is(TokenType::TemplateStringMiddle)
                        && !self.peek_is(TokenType::TemplateStringEnd)
                    {
                        return Err(ParserError::unexpected(self.peek_token_span()));
                    }
                    spans.push(span);
                }
            }

            // end chunk: remove } prefix and ` suffix
            let token = self.eat_token(TokenType::TemplateStringEnd)?;
            let range = token.range();
            let token_str = &self.file.text()[range.start as usize..range.end as usize];
            let string = Self::template_chunk_body(token, token_str, 1, 1)?.to_owned();
            chunks.push(self.template_chunk(range, &string, is_tagged)?);

            return Ok((chunks, spans));
        }

        Err(ParserError::unexpected(next))
    }

    /// Return the body of one lexer-shaped template chunk.
    fn template_chunk_body(
        token: Token,
        token_str: &str,
        prefix_len: usize,
        suffix_len: usize,
    ) -> ParserResult<&str> {
        let Some(end) = token_str.len().checked_sub(suffix_len) else {
            return Err(ParserError::unexpected(token));
        };
        let Some(body) = token_str.get(prefix_len..end) else {
            return Err(ParserError::unexpected(token));
        };

        Ok(body)
    }

    /// Intern one template chunk as written and decoded, tagged templates keep invalid escapes.
    fn template_chunk(
        &mut self,
        range: ByteRange,
        string: &str,
        is_tagged: bool,
    ) -> ParserResult<TemplateChunk> {
        let raw = self.strings.intern(string);
        let cooked = match cook(string) {
            Ok(cooked) => Some(self.strings.intern(&cooked)),
            Err(InvalidEscape) if is_tagged => None,
            Err(InvalidEscape) => return Err(ParserError::expected(range, TokenType::Literal)),
        };

        Ok(TemplateChunk { cooked, raw })
    }

    /// Parse a template literal interpolation argument.
    pub(crate) fn parse_template_argument(&mut self) -> ParserResult<LocalNodeId<Argument>> {
        let start = self.mark_parse_start();

        // interpolations own their decorators like call arguments do
        let decorators = self.parse_decorators();
        let value = self.parse_template_interpolation()?;

        let argument_id =
            self.insert_node(Argument::Positional { value }, self.range_since(&start));
        self.attach_decorators(argument_id.id, decorators);

        Ok(argument_id)
    }

    /// Parse one template interpolation expression.
    ///
    /// Examples:
    /// ```tspp
    /// value
    /// condition ? yes : no
    /// ```
    fn parse_template_interpolation(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        self.parse_expression(
            ExpressionPosition::NestedStatement,
            ExpressionStop::default(),
        )
    }
}
