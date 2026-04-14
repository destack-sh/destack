use crate::{ParseError, ParseResult, Parser};

use destack_ast::{
    Argument, Expression, Keyword, LocalNodeId, NodeType, TokenType, TypeExpression,
};

impl Parser {
    /// Eat one `import(...)` type argument list.
    fn eat_type_import_arguments(&mut self) -> ParseResult<Vec<LocalNodeId<Argument>>> {
        // argument list: `(`
        self.eat_token(TokenType::OpenParenthesis)?;
        self.eat_newlines_maybe()?;

        // empty argument list: `()`
        if self.peek_is(TokenType::CloseParenthesis) {
            self.bump(); // eat close parenthesis
            return Ok(vec![]);
        }

        // positional arguments
        let argument_options = self.options.nested().not_in_position();
        let arguments = self.with_options(argument_options, |parser| {
            parser.eat_positional_arguments_body(TokenType::CloseParenthesis)
        })?;

        // argument list: `)`
        self.eat_newlines_maybe()?;
        self.eat_type_token_or_recover_missing(
            TokenType::CloseParenthesis,
            NodeType::TypeExpression,
        )?;

        Ok(arguments)
    }

    /// Eat one `import(...)` type expression.
    pub fn eat_type_import_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark_span();

        // `import`
        self.eat_keyword(Keyword::Import)?;

        // `import(...)`
        let argument_options = self.options.nested().not_in_position();
        let arguments = self.with_options(argument_options, |parser| {
            parser.eat_type_import_arguments()
        })?;

        // target: first positional argument or one recovered placeholder
        let mut arguments = arguments;
        let (target, target_span) = if arguments.is_empty() {
            let target = self.recover_missing_expression_here(NodeType::Expression);
            let target_span = self.tree.get_span(target);

            arguments.push(self.insert_positional_argument(target));
            (target, target_span)
        } else {
            match self.tree.get(arguments[0]) {
                Argument::Positional { value, .. } => (*value, self.tree.get_span(*value)),
                Argument::Error => {
                    let target = self.recover_missing_expression_here(NodeType::Expression);
                    let target_span = self.tree.get_span(target);
                    (target, target_span)
                }
                _ => {
                    return Err(ParseError::expected(
                        self.tree.get_span(arguments[0]),
                        TokenType::Literal,
                    ));
                }
            }
        };

        // qualifier: `import("mod").Type<T>`
        let (qualifier, generic_arguments) = if self.peek_is(TokenType::Dot) {
            self.bump(); // eat dot
            let qualifier = self.eat_path()?;

            let generic_arguments = if self.peek_is(TokenType::Newline)
                && (self.is_token_after_newlines(self.pos(), TokenType::LessThan)
                    || self.is_token_after_newlines(self.pos(), TokenType::ShiftLeft))
            {
                self.eat_newlines_maybe()?;
                self.eat_generic_arguments_maybe()?.unwrap_or_default()
            } else {
                self.eat_generic_arguments_maybe()?.unwrap_or_default()
            };

            (Some(qualifier), generic_arguments)
        } else {
            (None, vec![])
        };

        // import type node
        let type_expression_id = self.insert_node(
            TypeExpression::Import {
                target,
                arguments,
                qualifier,
                generic_arguments,
            },
            self.get_span_from(&start),
        );
        self.tree.set_main_span(type_expression_id, target_span);

        Ok(self.insert_type_expression_value(type_expression_id))
    }
}
