use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser};

use destack_ast::{
    Expression, LiteralType, LocalNodeId, Name, NodeType, Pattern, PatternField, ScalarLiteral,
    TokenType, TypeExpression, TypeLiteral,
};
use destack_source::Span;

impl Parser {
    /// Eat a pattern.
    ///
    /// Examples:
    /// ```
    /// _
    /// 1
    /// 2 | 3
    /// (x, 0, ...)
    /// { a: 2 }
    /// Success(_)
    /// Vector2 { x: 0, y, z: zed }
    /// geom.Mesh<2, float32> { vertices: [2, ...] }
    /// ```
    pub fn eat_pattern(&mut self) -> ParseResult<LocalNodeId<Pattern>> {
        let _timing = self.timing_scope(tags::PARSE_PATTERN);
        let start = self.mark_span();

        // mutability
        let mutability = self.eat_mutability_maybe()?;

        // ------------------------------------------------------------
        // Primary patterns
        // ------------------------------------------------------------
        let mut pattern_id = {
            // wildcard
            if self.language.is_destack() && self.peek_identifier_str_is("_") {
                self.bump(); // eat wildcard
                self.tree
                    .insert(Pattern::Wildcard, self.get_span_from(&start))
            }
            // reference of
            else if self.peek_is(TokenType::ElementwiseAnd) {
                self.bump(); // eat &
                let mutability = self.eat_reference_mutability_maybe()?;
                let right_id = self.eat_pattern().for_node_type(NodeType::Pattern)?;
                self.insert_node(
                    Pattern::ReferenceOf {
                        mutability,
                        right: right_id,
                    },
                    self.get_span_from(&start),
                )
            }
            // value of
            else if self.peek_is(TokenType::ElementwiseXor) {
                self.bump(); // eat ^
                let mutability = self.eat_reference_mutability_maybe()?;
                let right_id = self.eat_pattern().for_node_type(NodeType::Pattern)?;
                self.insert_node(
                    Pattern::ValueOf {
                        mutability,
                        right: right_id,
                    },
                    self.get_span_from(&start),
                )
            }
            // tuple (without type, no struct tuples)
            else if self.peek_is(TokenType::OpenParenthesis) {
                self.bump(); // eat open parenthesis
                self.eat_newlines_maybe()?;
                let field_options = self.options.nested();
                let fields_result = self.with_options(field_options, |parser| {
                    parser.eat_pattern_field_list(TokenType::Comma, TokenType::CloseParenthesis)
                })?;
                let fields = fields_result;
                let pattern = Pattern::Tuple { fields };
                self.eat_close_token_or_recover_missing(
                    TokenType::CloseParenthesis,
                    NodeType::Pattern,
                )?;
                self.insert_node(pattern, self.get_span_from(&start))
            }
            // struct (without type)
            else if self.peek_is(TokenType::OpenBrace) {
                self.bump(); // eat open brace
                self.eat_newlines_maybe()?;
                let field_options = self.options.nested();
                let fields_result = self.with_options(field_options, |parser| {
                    parser.eat_pattern_field_list(TokenType::Comma, TokenType::CloseBrace)
                })?;
                let fields = fields_result;
                let pattern = Pattern::Object { fields };
                self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Pattern)?;
                self.insert_node(pattern, self.get_span_from(&start))
            }
            // array or slice
            else if self.peek_is(TokenType::OpenBracket) {
                self.bump(); // eat open bracket
                self.eat_newlines_maybe()?;
                let field_options = self.options.nested();
                let fields_result = self.with_options(field_options, |parser| {
                    parser.eat_pattern_field_list(TokenType::Comma, TokenType::CloseBracket)
                })?;
                let fields = fields_result;
                self.eat_newlines_maybe()?;
                self.eat_close_token_or_recover_missing(
                    TokenType::CloseBracket,
                    NodeType::Pattern,
                )?;
                self.tree
                    .insert(Pattern::Array { fields }, self.get_span_from(&start))
            }
            // literal expression
            else if self.is_scalar_literal_start() {
                let scalar_literal_id =
                    self.eat_scalar_literal().for_node_type(NodeType::Pattern)?;
                let expression_id = self.insert_node(
                    Expression::ScalarLiteral(scalar_literal_id),
                    self.get_span_from(&start),
                );
                self.insert_node(
                    Pattern::Expression {
                        value: expression_id,
                    },
                    self.get_span_from(&start),
                )
            }
            // null and undefined literals
            else if self.peek_identifier_str_is("null")
                || self.peek_identifier_str_is("undefined")
            {
                let is_null = self.peek_identifier_str_is("null");
                self.bump(); // eat literal identifier

                let pattern = if is_null {
                    let expression_id = self.tree.insert(
                        Expression::ScalarLiteral(ScalarLiteral::Null),
                        self.get_span_from(&start),
                    );
                    Pattern::Expression {
                        value: expression_id,
                    }
                } else {
                    let expression_id = self.tree.insert(
                        TypeExpression::Literal {
                            value: TypeLiteral::Undefined,
                        },
                        self.get_span_from(&start),
                    );
                    Pattern::TypeExpression {
                        value: expression_id,
                    }
                };
                self.insert_node(pattern, self.get_span_from(&start))
            }
            // binding with expression or pattern
            else if !self.options.is_in_before_type()
                && self.peek_identifier_is()
                && self.peek_next_is(TokenType::Colon)
            {
                let (name, name_span) = self.eat_binding_identifier_with_span()?;
                self.bump(); // eat colon
                let inner_pattern_options = self.options.in_static().in_before_block();
                let inner_pattern_result =
                    self.with_options(inner_pattern_options, |parser| parser.eat_pattern())?;
                let inner_pattern_id = inner_pattern_result;
                let pattern_id = self.insert_node(
                    Pattern::Binding {
                        mutability,
                        name,
                        pattern: Some(inner_pattern_id),
                    },
                    self.get_span_from(&start),
                );
                self.tree.set_main_span(pattern_id, name_span);
                pattern_id
            }
            // path or identifier
            else {
                let (path, segment_spans, last_span) = self
                    .eat_path_with_endpoint_spans()
                    .for_node_type(NodeType::Pattern)?;
                // tuple with path
                if self.peek_is(TokenType::OpenParenthesis) {
                    self.bump(); // eat open parenthesis
                    self.eat_newlines_maybe()?;
                    let fields = self
                        .eat_pattern_field_list(TokenType::Comma, TokenType::CloseParenthesis)
                        .for_node_type(NodeType::Pattern)?;
                    let expression_id = self.insert_node(
                        TypeExpression::Reference {
                            path,
                            generic_arguments: vec![],
                        },
                        self.get_span_from(&start),
                    );
                    let pattern = Pattern::TaggedTuple {
                        ty: expression_id,
                        fields,
                    };
                    self.eat_close_token_or_recover_missing(
                        TokenType::CloseParenthesis,
                        NodeType::Pattern,
                    )?;
                    self.insert_node(pattern, self.get_span_from(&start))
                }
                // struct with path
                else if !self.options.is_in_before_block() && self.peek_is(TokenType::OpenBrace) {
                    self.bump(); // eat open brace
                    self.eat_newlines_maybe()?;
                    let fields = self
                        .eat_pattern_field_list(TokenType::Comma, TokenType::CloseBrace)
                        .for_node_type(NodeType::Pattern)?;
                    let ty_id = self.insert_node(
                        TypeExpression::Reference {
                            path,
                            generic_arguments: vec![],
                        },
                        self.get_span_from(&start),
                    );
                    let pattern = Pattern::TaggedObject { ty: ty_id, fields };
                    self.eat_close_token_or_recover_missing(
                        TokenType::CloseBrace,
                        NodeType::Pattern,
                    )?;
                    self.insert_node(pattern, self.get_span_from(&start))
                }
                // path
                else if path.segments.len() > 1 {
                    let expression_id = self.insert_node(
                        Expression::QualifiedReference {
                            path,
                            generic_arguments: vec![],
                        },
                        self.get_span_from(&start),
                    );
                    self.set_path_expression_spans(expression_id, &segment_spans);
                    self.insert_node(
                        Pattern::Expression {
                            value: expression_id,
                        },
                        self.get_span_from(&start),
                    )
                }
                // nullish literals in patterns
                else if path.segments[0] == self.state.type_literal_identifiers.null_
                    || path.segments[0] == self.state.type_literal_identifiers.undefined
                {
                    let is_null = path.segments[0] == self.state.type_literal_identifiers.null_;
                    let pattern = if is_null {
                        let expression_id = self.insert_node(
                            Expression::ScalarLiteral(ScalarLiteral::Null),
                            self.get_span_from(&start),
                        );
                        self.tree.set_main_span(expression_id, last_span);
                        Pattern::Expression {
                            value: expression_id,
                        }
                    } else {
                        let expression_id = self.insert_node(
                            TypeExpression::Literal {
                                value: TypeLiteral::Undefined,
                            },
                            self.get_span_from(&start),
                        );
                        self.tree.set_main_span(expression_id, last_span);
                        Pattern::TypeExpression {
                            value: expression_id,
                        }
                    };
                    self.insert_node(pattern, self.get_span_from(&start))
                }
                // identifier
                else {
                    let pattern_id = self.insert_node(
                        Pattern::Binding {
                            mutability,
                            name: path.segments[0],
                            pattern: None,
                        },
                        self.get_span_from(&start),
                    );
                    self.tree.set_main_span(pattern_id, last_span);
                    pattern_id
                }
            }
        };

        // ------------------------------------------------------------
        // Postfix / infix patterns
        // ------------------------------------------------------------

        // must
        if self.peek_is(TokenType::Not) {
            self.bump(); // eat !
            let pattern = Pattern::Must(pattern_id);
            pattern_id = self.insert_node(pattern, self.get_span_from(&start));
        }
        // union
        if self.peek_is(TokenType::ElementwiseOr) && !self.options.is_in_union_pattern() {
            // eat all union "fields" (just unnamed patterns)
            let mut patterns: Vec<LocalNodeId<Pattern>> = vec![pattern_id];
            while self.peek_is(TokenType::ElementwiseOr) {
                self.bump(); // eat '|'
                let union_options = self.options.in_union_pattern();
                let field_pattern_result =
                    self.with_options(union_options, |parser| parser.eat_pattern())?;
                let field_pattern_id = field_pattern_result;
                patterns.push(field_pattern_id);
            }
            let pattern = Pattern::Union { patterns };
            let pattern_id = self.insert_node(pattern, self.get_span_from(&start));
            Ok(pattern_id)
        }
        // no infix
        else {
            Ok(pattern_id)
        }
    }

    /// Eat a pattern field list (like `x, y, z` or `1 | 2`).
    fn eat_pattern_field_list(
        &mut self,
        seperator: TokenType,
        terminator: TokenType,
    ) -> ParseResult<Vec<LocalNodeId<PatternField>>> {
        let mut fields: Vec<LocalNodeId<PatternField>> = Vec::new();
        let is_object_pattern = terminator == TokenType::CloseBrace;
        let enforce_terminal_spread =
            self.language.is_javascript() || self.language.is_typescript();
        let mut has_spread_field = false;

        while self.has_more_tokens() {
            // stop at the pattern terminator
            if self.peek_token_type() == terminator {
                break;
            }

            // spread fields must be terminal in typed and untyped patterns
            if enforce_terminal_spread && has_spread_field {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            // parse one field
            let field_start = self.mark_span();
            let mut name_span = None;
            let has_object_literal_alias_head =
                is_object_pattern && self.peek_object_pattern_alias_head();
            let can_start_named_or_spread_field = self.peek_name_is()
                || has_object_literal_alias_head
                || self.peek_mutability_is()
                || self.peek_is(TokenType::Spread);
            let pattern_field = {
                // elision: empty slot before separator (like `[,a]` or `[,,b]`)
                if self.peek_token_type() == seperator {
                    PatternField::Elision
                }
                // positional wildcard for tuples/arrays
                else if !is_object_pattern
                    && self.peek_identifier_str_is("_")
                    && !self.peek_next_is(TokenType::Colon)
                {
                    self.eat_positional_pattern_field()?
                }
                // computed property (object patterns only)
                else if is_object_pattern
                    && (self.peek_is(TokenType::OpenBracket)
                        || (self.peek_mutability_is() && self.peek_next_is(TokenType::OpenBracket)))
                {
                    let mutability = self.eat_mutability_maybe()?;
                    self.eat_token(TokenType::OpenBracket)?;
                    let key = self.eat_expression(
                        self.options.not_in_position().not_in_sequence_expression(),
                    )?;
                    self.eat_close_token_or_recover_missing_with(
                        TokenType::CloseBracket,
                        NodeType::PatternField,
                        |_, token_type| {
                            Self::is_close_delimiter_boundary_token(token_type)
                                || token_type == TokenType::Colon
                        },
                    )?;
                    self.eat_newlines_maybe()?;
                    self.eat_token(TokenType::Colon)?;
                    self.eat_newlines_maybe()?;
                    let pattern = self.eat_pattern().for_node_type(NodeType::Pattern)?;
                    let default = self.eat_pattern_field_default_maybe()?;
                    PatternField::Computed {
                        mutability,
                        key,
                        pattern: Some(pattern),
                        default,
                    }
                }
                // named field variants and spread fields
                else if can_start_named_or_spread_field {
                    let mutability = if is_object_pattern && self.peek_mutability_is() {
                        // treat mutability keywords as field names when a separator follows
                        // NOTE #Cleanup: revisit mutability in pattern field list
                        let has_separator_or_assignment = self.peek_next_is(TokenType::Colon)
                            || self.peek_next_is(seperator)
                            || self.peek_next_is(terminator)
                            || self.peek_next_is(TokenType::Assign)
                            || self.peek_next_is(TokenType::Maybe);
                        let has_separator_or_assignment_after_newline = self
                            .is_token_after_newlines(self.pos(), TokenType::Colon)
                            || self.is_token_after_newlines(self.pos(), seperator)
                            || self.is_token_after_newlines(self.pos(), terminator)
                            || self.is_token_after_newlines(self.pos(), TokenType::Assign)
                            || self.is_token_after_newlines(self.pos(), TokenType::Maybe);

                        if has_separator_or_assignment || has_separator_or_assignment_after_newline
                        {
                            None
                        } else {
                            self.eat_mutability_maybe()?
                        }
                    } else {
                        self.eat_mutability_maybe()?
                    };

                    // spread fields
                    if self.peek_is(TokenType::Spread) {
                        self.bump(); // eat spread

                        // spread with omitted target is allowed before separators and terminators
                        let pattern = if self.peek_is(TokenType::Newline)
                            && (self.is_token_after_newlines(self.pos(), seperator)
                                || self.is_token_after_newlines(self.pos(), terminator))
                        {
                            self.eat_newlines_maybe()?;
                            None
                        } else if self.peek_token_type() == seperator || self.peek_is(terminator) {
                            None
                        } else {
                            let pattern = self.eat_pattern().for_node_type(NodeType::Pattern)?;
                            Some(pattern)
                        };
                        PatternField::Spread {
                            mutability,
                            pattern,
                        }
                    }
                    // named alias and named pattern fields
                    else {
                        let has_named_colon_field = self.peek_name_is()
                            && self.peek_next_is(TokenType::Colon)
                            || is_object_pattern && self.peek_object_pattern_alias_head();
                        if has_named_colon_field {
                            let (name, _name_span) =
                                self.eat_pattern_field_name_with_span(terminator)?;
                            self.eat_newlines_maybe()?;
                            self.bump(); // eat colon
                            self.eat_newlines_maybe()?;

                            // named alias field
                            if self.peek_identifier_is() {
                                let (alias, alias_span) = self
                                    .eat_binding_identifier_with_span()
                                    .for_node_type(NodeType::Pattern)?;
                                name_span = Some(alias_span);
                                let default = self.eat_pattern_field_default_maybe()?;
                                PatternField::Alias {
                                    mutability,
                                    name,
                                    alias,
                                    default,
                                }
                            }
                            // named field with nested pattern
                            else {
                                let pattern =
                                    self.eat_pattern().for_node_type(NodeType::Pattern)?;
                                let default = self.eat_pattern_field_default_maybe()?;
                                PatternField::Named {
                                    mutability,
                                    name,
                                    pattern: Some(pattern),
                                    default,
                                }
                            }
                        }
                        // named shorthand field
                        else {
                            let (name, span) = self.eat_pattern_field_name_with_span(terminator)?;
                            name_span = Some(span);
                            let default = self.eat_pattern_field_default_maybe()?;
                            PatternField::Named {
                                mutability,
                                name,
                                pattern: None,
                                default,
                            }
                        }
                    }
                }
                // positional field
                else {
                    self.eat_positional_pattern_field()?
                }
            };
            let pattern_field_id = self
                .tree
                .insert(pattern_field, self.get_span_from(&field_start));
            if let Some(name_span) = name_span {
                self.tree.set_main_span(pattern_field_id, name_span);
            }
            fields.push(pattern_field_id);

            // typed and untyped object patterns require spread fields to terminate the list
            if enforce_terminal_spread
                && matches!(self.tree.get(pattern_field_id), PatternField::Spread { .. })
            {
                has_spread_field = true;
                let has_separator_after_spread = self.peek_token_type() == seperator;
                let has_non_terminal_newline_after_spread = self.peek_is(TokenType::Newline)
                    && !self.is_token_after_newlines(self.pos(), terminator);
                if has_separator_after_spread || has_non_terminal_newline_after_spread {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }
            }

            // consume separators and newline separators
            if self.peek_token_type() == seperator || self.peek_is(TokenType::Newline) {
                self.bump(); // eat separator
                self.eat_newlines_maybe()?;
            } else {
                break;
            }
        }

        Ok(fields)
    }

    // eat a positional pattern field with an optional default
    fn eat_positional_pattern_field(&mut self) -> ParseResult<PatternField> {
        let pattern = self.eat_pattern().for_node_type(NodeType::Pattern)?;
        let default = self.eat_pattern_field_default_maybe()?;

        Ok(PatternField::Positional { pattern, default })
    }

    // eat a pattern field default assignment if present
    fn eat_pattern_field_default_maybe(&mut self) -> ParseResult<Option<LocalNodeId<Expression>>> {
        let has_immediate_default = self.peek_is(TokenType::Assign);
        let has_newline_default = self.peek_is(TokenType::Newline)
            && self.is_token_after_newlines(self.pos(), TokenType::Assign);
        if !has_immediate_default && !has_newline_default {
            return Ok(None);
        }

        if has_newline_default {
            self.eat_newlines_maybe()?;
        }

        self.bump(); // eat assign
        let default =
            self.eat_expression(self.options.not_in_position().not_in_sequence_expression())?;
        Ok(Some(default))
    }

    // check whether object pattern field head is a literal alias key before `:`
    fn peek_object_pattern_alias_head(&mut self) -> bool {
        let has_numeric_alias_head =
            self.peek_numeric_literal_is() && self.peek_next_is(TokenType::Colon);
        let has_boolean_alias_head =
            self.peek_boolean_pattern_name_head() && self.peek_next_is(TokenType::Colon);
        has_numeric_alias_head || has_boolean_alias_head
    }

    // check whether next token is a boolean literal key in object patterns
    fn peek_boolean_pattern_name_head(&mut self) -> bool {
        self.peek().is_ok_and(|token| {
            token.token.ty == TokenType::Literal
                && matches!(token.token.literal, Some(LiteralType::Boolean { .. }))
        })
    }

    // eat a name for object pattern fields
    fn eat_pattern_field_name_with_span(
        &mut self,
        terminator: TokenType,
    ) -> ParseResult<(Name, Span)> {
        let is_numeric_object_key =
            terminator == TokenType::CloseBrace && self.peek_numeric_literal_is();
        if is_numeric_object_key {
            return self.eat_numeric_pattern_name_with_span();
        }
        let is_boolean_object_key =
            terminator == TokenType::CloseBrace && self.peek_boolean_pattern_name_head();
        if is_boolean_object_key {
            return self.eat_boolean_pattern_name_with_span();
        }

        self.eat_name_with_span()
    }

    // eat a numeric pattern field name as Name::Number
    fn eat_numeric_pattern_name_with_span(&mut self) -> ParseResult<(Name, Span)> {
        let token = *self.peek_numeric_literal()?;
        let key_string = self.file.span_str(token.span).to_string();

        let numeric_literal = self.eat_scalar_literal()?;
        if !matches!(
            numeric_literal,
            ScalarLiteral::Integer(_) | ScalarLiteral::Float(_) | ScalarLiteral::Bigint(_)
        ) {
            return Err(ParseError::unexpected(token.span));
        }

        let key_name = self.strings.intern(key_string);
        Ok((Name::Number(key_name), token.span))
    }

    // eat a boolean pattern field name as Name::Identifier
    fn eat_boolean_pattern_name_with_span(&mut self) -> ParseResult<(Name, Span)> {
        let token = *self.peek()?;
        if token.token.ty != TokenType::Literal
            || !matches!(token.token.literal, Some(LiteralType::Boolean { .. }))
        {
            return Err(ParseError::unexpected(token.span));
        }

        self.bump();
        let key_name = self.get_span_str(token.span).to_owned();
        let key_name = self.strings.intern(&key_name);
        Ok((Name::Identifier(key_name), token.span))
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        Expression, Mutability, Name, Pattern, PatternField, ScalarLiteral, TypeExpression,
    };
    use destack_source::LanguageType;

    use crate::{
        TestParser, assert_expression_path, assert_name, assert_node, assert_path, assert_string,
    };

    #[test]
    fn test_parse_pattern_wildcard() {
        // _
        let mut test = TestParser::new("_");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();
        assert_node!(parser.tree, pattern_id, Pattern::Wildcard);
    }

    #[test]
    fn test_parse_pattern_underscore_identifier() {
        // _ in TypeScript patterns is a normal binding name
        let mut test = TestParser::new_with_options("_", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();
        assert_node!(parser.tree, pattern_id, Pattern::Binding { mutability: None, name, pattern: None } => {
            assert_string!(parser, *name, "_");
        });
    }

    #[test]
    fn test_parse_tuple_pattern_with_missing_close_parenthesis() {
        let mut test = TestParser::new("(first, second");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_eq!(parser.errors.len(), 1);

        assert_node!(parser.tree, pattern_id, Pattern::Tuple { fields } => {
            assert_eq!(fields.len(), 2);
        });
    }

    #[test]
    fn test_parse_computed_pattern_field_with_missing_close_bracket() {
        let mut test = TestParser::new("{ [key: value }");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_eq!(parser.errors.len(), 1);

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], PatternField::Computed { key, pattern: Some(pattern), .. } => {
                assert_expression_path!(parser, parser.tree.get(*key), "key");
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "value");
                });
            });
        });
    }

    #[test]
    fn test_parse_pattern_reference() {
        // &_
        let mut test = TestParser::new("&_");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();
        // &
        assert_node!(parser.tree, pattern_id,
            Pattern::ReferenceOf { mutability: Some(mutability), right } => {
                // var
                assert_eq!(*mutability, Mutability::Mutable);
                // _
                assert_node!(parser.tree, *right, Pattern::Wildcard)
            }
        );

        // &1
        let mut test = TestParser::new("&1");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();
        // &
        assert_node!(parser.tree, pattern_id, Pattern::ReferenceOf { mutability: Some(mutability), right } => {
            assert_eq!(*mutability, Mutability::Mutable);
            // 1
            assert_node!(parser.tree, *right, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
        })
    }

    #[test]
    fn test_parse_pattern_value() {
        // ^x
        let mut test = TestParser::new("^x");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();
        assert_node!(parser.tree, pattern_id, Pattern::ValueOf { mutability: Some(mutability), right } => {
            assert_eq!(*mutability, Mutability::Mutable);
            assert_node!(parser.tree, *right, Pattern::Binding { mutability: None, name, pattern: None } => {
                assert_string!(parser, *name, "x");
            });
        });
    }

    #[test]
    fn test_parse_pattern_identifier() {
        let mut test = TestParser::new("x");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();
        assert_node!(parser.tree, pattern_id, Pattern::Binding { mutability: None, name, pattern: None } => {
            assert_string!(parser, *name, "x");
        });
    }

    #[test]
    fn test_parse_pattern_path() {
        let mut test = TestParser::new("MyEnum.A");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();
        assert_node!(parser.tree, pattern_id, Pattern::Expression { value } => {
            assert_expression_path!(parser, parser.tree.get(*value), "MyEnum.A");
        });
    }

    #[test]
    fn test_parse_pattern_tuple() {
        let mut test = TestParser::new("(x: 1, 2, var y, const z, ...)");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        // (x: 1, 2, var y, const z, ...)
        assert_node!(parser.tree, pattern_id, Pattern::Tuple { fields, .. } => {
            assert_eq!(fields.len(), 5);

            // x: 1
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: Some(pattern), mutability: None, default: None } => {
                assert_name!(parser, *name, "x");
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                });
            });

            // 2
            assert_node!(parser.tree, fields[1], PatternField::Positional { pattern, default: None } => {
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                });
            });

            // var y
            assert_node!(parser.tree, fields[2], PatternField::Named { name, pattern: None, mutability: Some(mutability), default: None } => {
                assert_name!(parser, *name, "y");
                assert_eq!(*mutability, Mutability::Mutable);
            });

            // const z
            assert_node!(parser.tree, fields[3], PatternField::Named { name, pattern: None, mutability: Some(mutability), default: None } => {
                assert_name!(parser, *name, "z");
                assert_eq!(*mutability, Mutability::Immutable);
            });

            // ...
            assert_node!(parser.tree, fields[4], PatternField::Spread { mutability: None, pattern: None } => {
            });
        });
    }

    #[test]
    fn test_parse_pattern_tuple_with_path() {
        let mut test = TestParser::new("Result.Success(_, ...)");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        // Result.Success(_, ..)
        assert_node!(parser.tree, pattern_id, Pattern::TaggedTuple { ty, fields } => {
            // Result.Success
            let _ = ty; // ty is required for TaggedTuple
            assert_eq!(fields.len(), 2);

            // _
            assert_node!(parser.tree, fields[0], PatternField::Positional { pattern, default: None } => {
                assert_node!(parser.tree, *pattern, Pattern::Wildcard);
            });

            // ..
            assert_node!(parser.tree, fields[1], PatternField::Spread { mutability: None, pattern: None } => {
            });
        });
    }

    #[test]
    fn test_parse_pattern_object_field_const_alias() {
        let mut test =
            TestParser::new_with_options("{ const: value, title }", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        // { const: value, title }
        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 2);

            // const: value
            assert_node!(parser.tree, fields[0], PatternField::Alias { mutability: None, name, alias, default: None } => {
                assert_name!(parser, *name, "const");
                assert_string!(parser, *alias, "value");
            });

            // title
            assert_node!(parser.tree, fields[1], PatternField::Named { mutability: None, name, pattern: None, default: None } => {
                assert_name!(parser, *name, "title");
            });
        });
    }

    #[test]
    fn test_parse_pattern_tuple_spread_non_terminal() {
        let mut test = TestParser::new("(x, ...rest, z)");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Tuple { fields, .. } => {
            assert_eq!(fields.len(), 3);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, mutability: None, default: None } => {
                assert_name!(parser, *name, "x");
            });
            assert_node!(parser.tree, fields[1], PatternField::Spread { mutability: None, pattern: Some(pattern) } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                    assert_string!(parser, *name, "rest");
                });
            });
            assert_node!(parser.tree, fields[2], PatternField::Named { name, pattern: None, mutability: None, default: None } => {
                assert_name!(parser, *name, "z");
            });
        });
    }

    #[test]
    fn test_parse_pattern_tuple_newline_separated() {
        let mut test = TestParser::new(
            "
(
    x: 1
    2,
    ...
)",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let pattern_id = parser.eat_pattern().unwrap();

        // (x: 1, 2, ..)
        assert_node!(parser.tree, pattern_id, Pattern::Tuple { fields, .. } => {
            assert_eq!(fields.len(), 3);

            // x: 1
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: Some(pattern), mutability: None, default: None } => {
                assert_name!(parser, *name, "x");
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                });
            });

            // 2
            assert_node!(parser.tree, fields[1], PatternField::Positional { pattern, default: None } => {
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                });
            });

            // ..
            assert_node!(parser.tree, fields[2], PatternField::Spread { mutability: None, pattern: None } => {
            });
        });
    }

    #[test]
    fn test_parse_pattern_union() {
        // 1 | 2 | 3
        let mut test = TestParser::new("1 | 2 | 3");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Union { patterns } => {
            assert_eq!(patterns.len(), 3);

            // 1
            assert_node!(parser.tree, patterns[0], Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });

            // 2
            assert_node!(parser.tree, patterns[1], Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            });

            // 3
            assert_node!(parser.tree, patterns[2], Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
            });
        });
    }

    #[test]
    fn test_parse_pattern_struct_anonymous() {
        let mut test = TestParser::new("{ x: 1, y, var z, const w: 4, ... }");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 5);

            // x: 1
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: Some(pattern), mutability: None, default: None } => {
                assert_name!(parser, *name, "x");
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                });
            });

            // y
            assert_node!(parser.tree, fields[1], PatternField::Named { name, pattern: None, mutability: None, default: None } => {
                assert_name!(parser, *name, "y");
            });

            // var z
            assert_node!(parser.tree, fields[2], PatternField::Named { name, pattern: None, mutability: Some(mutability), default: None } => {
                assert_name!(parser, *name, "z");
                assert_eq!(*mutability, Mutability::Mutable);
            });

            // const w: 4
            assert_node!(parser.tree, fields[3], PatternField::Named { name, pattern: Some(pattern), mutability: Some(mutability), default: None } => {
                assert_name!(parser, *name, "w");
                assert_eq!(*mutability, Mutability::Immutable);
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(4)));
                });
            });

            // ..
            assert_node!(parser.tree, fields[4], PatternField::Spread { mutability: None, pattern: None } => {
            });
        });
    }

    #[test]
    fn test_parse_pattern_named_default_after_comment_newline() {
        let mut test = TestParser::new_with_options("{d //comment\n= b}", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, default: Some(default), .. } => {
                assert_name!(parser, *name, "d");
                assert_expression_path!(parser, parser.tree.get(*default), "b");
            });
        });
    }

    #[test]
    fn test_parse_pattern_struct_numeric_name_aliases() {
        let mut test = TestParser::new("{ 0: fieldNameOrOptions, 1: from, length: argc }");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 3);
            assert_node!(parser.tree, fields[0], PatternField::Alias { name, alias, .. } => {
                assert_node!(name, Name::Number(name) => {
                    assert_string!(parser, *name, "0");
                });
                assert_string!(parser, *alias, "fieldNameOrOptions");
            });
            assert_node!(parser.tree, fields[1], PatternField::Alias { name, alias, .. } => {
                assert_node!(name, Name::Number(name) => {
                    assert_string!(parser, *name, "1");
                });
                assert_string!(parser, *alias, "from");
            });
            assert_node!(parser.tree, fields[2], PatternField::Alias { name, alias, .. } => {
                assert_name!(parser, *name, "length");
                assert_string!(parser, *alias, "argc");
            });
        });
    }

    #[test]
    fn test_parse_pattern_struct_boolean_name_aliases() {
        let mut test = TestParser::new_with_options(
            "{ false: decorators, true: metadata }",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 2);
            assert_node!(parser.tree, fields[0], PatternField::Alias { name, alias, .. } => {
                assert_name!(parser, *name, "false");
                assert_string!(parser, *alias, "decorators");
            });
            assert_node!(parser.tree, fields[1], PatternField::Alias { name, alias, .. } => {
                assert_name!(parser, *name, "true");
                assert_string!(parser, *alias, "metadata");
            });
        });
    }

    #[test]
    fn test_parse_pattern_struct_numeric_literal_field() {
        let mut test = TestParser::new_with_options("{ 5 }", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], PatternField::Positional { pattern, default: None } => {
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(5)));
                });
            });
        });
    }

    #[test]
    fn test_parse_pattern_struct_computed_field() {
        let mut test = TestParser::new("{ [key]: value }");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], PatternField::Computed { mutability: None, key, pattern, default } => {
                assert!(default.is_none());
                assert_node!(parser.tree, *key, Expression::Identifier { name } => {
                    assert_string!(parser, *name, "key");
                });
                assert_node!(parser.tree, pattern.unwrap(), Pattern::Binding { name, pattern: None, .. } => {
                    assert_string!(parser, *name, "value");
                });
            });
        });
    }

    #[test]
    fn test_parse_pattern_struct_with_path() {
        let mut test = TestParser::new("Vector2 { x: 0, y }");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::TaggedObject { ty, fields } => {
            assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments: _ } => {
                assert_path!(parser, *path, "Vector2");
            });
            assert_eq!(fields.len(), 2);

            // x: 0
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: Some(pattern), mutability: None, default: None } => {
                assert_name!(parser, *name, "x");
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
                });
            });

            // y
            assert_node!(parser.tree, fields[1], PatternField::Named { name, pattern: None, mutability: None, default: None } => {
                assert_name!(parser, *name, "y");
            });
        });
    }

    #[test]
    fn test_parse_pattern_slice() {
        let mut test = TestParser::new("[1, ...]");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Array { fields } => {
            assert_eq!(fields.len(), 2);
            // 1
            assert_node!(parser.tree, fields[0], PatternField::Positional { pattern, default: None } => {
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                });
            });
            // ...
            assert_node!(parser.tree, fields[1], PatternField::Spread { mutability: None, pattern: None } => {
            });
        });
    }

    /// Parse a spread field with an array pattern.
    #[test]
    fn test_parse_pattern_spread_array_pattern() {
        let mut test = TestParser::new("[...[x, y]]");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Array { fields } => {
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], PatternField::Spread { mutability: None, pattern: Some(pattern) } => {
                assert_node!(parser.tree, *pattern, Pattern::Array { fields } => {
                    assert_eq!(fields.len(), 2);
                    assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, .. } => {
                        assert_name!(parser, *name, "x");
                    });
                    assert_node!(parser.tree, fields[1], PatternField::Named { name, pattern: None, .. } => {
                        assert_name!(parser, *name, "y");
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_pattern_object_readonly_shorthand() {
        let mut test = TestParser::new_with_options("{ readonly }", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, mutability: None, pattern: None, default: None } => {
                assert_name!(parser, *name, "readonly");
            });
        });
    }

    #[test]
    fn test_parse_pattern_object_readonly_shorthand_with_newline() {
        let mut test = TestParser::new_with_options("{ readonly\n}", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, mutability: None, pattern: None, default: None } => {
                assert_name!(parser, *name, "readonly");
            });
        });
    }
    #[test]
    fn test_parse_pattern_object_readonly_shorthand_in_value_block_mode() {
        let mut test = TestParser::new("{ readonly }");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, mutability: None, pattern: None, default: None } => {
                assert_name!(parser, *name, "readonly");
            });
        });
    }

    #[test]
    fn test_parse_pattern_array_readonly_identifier() {
        let mut test =
            TestParser::new_with_options("[readonly, setReadonly]", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Array { fields } => {
            assert_eq!(fields.len(), 2);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, mutability: None, pattern: None, default: None } => {
                assert_name!(parser, *name, "readonly");
            });
            assert_node!(parser.tree, fields[1], PatternField::Named { name, mutability: None, pattern: None, default: None } => {
                assert_name!(parser, *name, "setReadonly");
            });
        });
    }

    #[test]
    fn test_parse_pattern_array_readonly_identifier_in_value_block_mode() {
        let mut test = TestParser::new("[readonly, setReadonly]");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Array { fields } => {
            assert_eq!(fields.len(), 2);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, mutability: None, pattern: None, default: None } => {
                assert_name!(parser, *name, "readonly");
            });
            assert_node!(parser.tree, fields[1], PatternField::Named { name, mutability: None, pattern: None, default: None } => {
                assert_name!(parser, *name, "setReadonly");
            });
        });
    }

    #[test]
    fn test_reject_pattern_object_readonly_modifier_with_name_in_value_block_mode() {
        let mut test = TestParser::new("{ readonly value }");
        let mut parser = test.prepare();

        let result = parser.eat_pattern();

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_pattern_object_spread_newline_before_terminator() {
        let mut test =
            TestParser::new_with_options("{\n  onSuccess,\n  ...rest\n}", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 2);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, mutability: None, default: None } => {
                assert_name!(parser, *name, "onSuccess");
            });
            assert_node!(parser.tree, fields[1], PatternField::Spread { mutability: None, pattern: Some(pattern) } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                    assert_string!(parser, *name, "rest");
                });
            });
        });
    }

    #[test]
    fn test_parse_pattern_must() {
        let mut test = TestParser::new("1!");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Must(inner) => {
            assert_node!(parser.tree, *inner, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
        });
    }

    #[test]
    fn test_parse_pattern_union_with_must_arms() {
        let mut test = TestParser::new("1! | 2!");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Union { patterns } => {
            assert_eq!(patterns.len(), 2);
            // 1!
            assert_node!(parser.tree, patterns[0], Pattern::Must(inner) => {
                assert_node!(parser.tree, *inner, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                });
            });
            // 2!
            assert_node!(parser.tree, patterns[1], Pattern::Must(inner) => {
                assert_node!(parser.tree, *inner, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                });
            });
        });
    }

    #[test]
    fn test_parse_pattern_union_with_trailing_must_arm() {
        let mut test = TestParser::new("1 | 2!");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Union { patterns } => {
            assert_eq!(patterns.len(), 2);
            // 1
            assert_node!(parser.tree, patterns[0], Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
            // 2!
            assert_node!(parser.tree, patterns[1], Pattern::Must(inner) => {
                assert_node!(parser.tree, *inner, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                });
            });
        });
    }

    #[test]
    fn test_parse_pattern_binding_with_pattern() {
        let mut test = TestParser::new("const value: 5");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Binding { mutability: Some(mutability), name, pattern: Some(pattern) } => {
            assert_eq!(*mutability, Mutability::Immutable);
            assert_string!(parser, *name, "value");
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(5)));
            });
        });
    }

    #[test]
    fn test_parse_pattern_array_elision() {
        // [,a] - elision before 'a'
        let mut test = TestParser::new("[,a]");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Array { fields } => {
            assert_eq!(fields.len(), 2);

            // elision (empty slot)
            assert_node!(parser.tree, fields[0], PatternField::Elision);

            // a (identifiers are parsed as Named shorthand)
            assert_node!(parser.tree, fields[1], PatternField::Named { mutability: None, name, pattern: None, default: None } => {
                assert_name!(parser, *name, "a");
            });
        });
    }

    #[test]
    fn test_parse_pattern_array_multiple_elisions() {
        // [,,a] - two elisions before 'a'
        let mut test = TestParser::new("[,,a]");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Array { fields } => {
            assert_eq!(fields.len(), 3);

            // first elision
            assert_node!(parser.tree, fields[0], PatternField::Elision);

            // second elision
            assert_node!(parser.tree, fields[1], PatternField::Elision);

            // a (identifiers are parsed as Named shorthand)
            assert_node!(parser.tree, fields[2], PatternField::Named { mutability: None, name, pattern: None, default: None } => {
                assert_name!(parser, *name, "a");
            });
        });
    }

    #[test]
    fn test_parse_pattern_array_trailing_elision() {
        // [a,] - element followed by trailing comma (not elision)
        let mut test = TestParser::new("[a,]");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Array { fields } => {
            // trailing comma doesn't create elision, just 'a'
            assert_eq!(fields.len(), 1);

            // a (identifiers are parsed as Named shorthand)
            assert_node!(parser.tree, fields[0], PatternField::Named { mutability: None, name, pattern: None, default: None } => {
                assert_name!(parser, *name, "a");
            });
        });
    }

    #[test]
    fn test_parse_object_pattern_defaults_do_not_consume_following_fields() {
        // {a,b=1,c:d,e:f=2,[g]:[h]}
        let mut test =
            TestParser::new_with_options("{a,b=1,c:d,e:f=2,[g]:[h]}", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 5);

            // b=1
            assert_node!(parser.tree, fields[1], PatternField::Named { default, .. } => {
                assert!(default.is_some());
            });

            // c:d
            assert_node!(parser.tree, fields[2], PatternField::Alias { default, .. } => {
                assert!(default.is_none());
            });

            // e:f=2
            assert_node!(parser.tree, fields[3], PatternField::Alias { default, .. } => {
                assert!(default.is_some());
            });

            // [g]:[h]
            assert_node!(parser.tree, fields[4], PatternField::Computed { pattern, .. } => {
                assert!(pattern.is_some());
            });
        });
    }

    #[test]
    fn test_parse_object_pattern_alias_and_computed_defaults() {
        // {c, d:e=1, [f]:g=2, h=i}
        let mut test =
            TestParser::new_with_options("{c, d:e=1, [f]:g=2, h=i}", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 4);

            // d:e=1
            assert_node!(parser.tree, fields[1], PatternField::Alias { default, .. } => {
                assert!(default.is_some());
            });

            // [f]:g=2
            assert_node!(parser.tree, fields[2], PatternField::Computed { default, .. } => {
                assert!(default.is_some());
            });

            // h=i
            assert_node!(parser.tree, fields[3], PatternField::Named { default, .. } => {
                assert!(default.is_some());
            });
        });
    }

    #[test]
    fn test_parse_object_pattern_computed_field_with_newline_after_colon() {
        // { [key]:\nvalue }
        let mut test = TestParser::new_with_options("{ [key]:\nvalue }", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 1);

            assert_node!(parser.tree, fields[0], PatternField::Computed { mutability: None, key, pattern: Some(pattern), default: None } => {
                assert_node!(parser.tree, *key, Expression::Identifier { name } => {
                    assert_string!(parser, *name, "key");
                });

                assert_node!(parser.tree, *pattern, Pattern::Binding { mutability: None, name, pattern: None } => {
                    assert_string!(parser, *name, "value");
                });
            });
        });
    }

    #[test]
    fn test_parse_object_pattern_alias_with_newline_after_colon() {
        // { source:\ntarget }
        let mut test =
            TestParser::new_with_options("{ source:\ntarget }", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 1);

            assert_node!(parser.tree, fields[0], PatternField::Alias { mutability: None, name, alias, default: None } => {
                assert_name!(parser, *name, "source");
                assert_string!(parser, *alias, "target");
            });
        });
    }
}
