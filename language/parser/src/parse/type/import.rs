use crate::{ParseError, ParseResult, Parser};

use destack_ast::{Argument, Keyword, LocalNodeId, NodeType, TokenType, TypeExpression};

impl Parser {
    /// Eat one `import(...)` type argument list.
    fn eat_type_import_arguments(&mut self) -> ParseResult<Vec<LocalNodeId<Argument>>> {
        // argument list: `(`
        self.eat_token(TokenType::OpenParenthesis)?;

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
        self.eat_type_token_or_recover_missing(
            TokenType::CloseParenthesis,
            NodeType::TypeExpression,
        )?;

        Ok(arguments)
    }

    /// Eat one `import(...)` type expression.
    ///
    /// Examples:
    /// ```
    /// import("pkg")
    /// import("pkg").Foo
    /// import("pkg").Foo<string>
    /// ```
    pub fn eat_type_import_expression(&mut self) -> ParseResult<LocalNodeId<TypeExpression>> {
        let start = self.span_start();

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
        if !arguments.is_empty() {
            arguments.remove(0);
        }

        // qualifier: `import("mod").Type<T>`
        let (qualifier, generic_arguments) = if self.peek_is(TokenType::Dot) {
            self.bump(); // eat dot
            let qualifier = self.eat_path()?;
            let generic_arguments = self.eat_generic_arguments_maybe()?.unwrap_or_default();

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

        Ok(type_expression_id)
    }
}
