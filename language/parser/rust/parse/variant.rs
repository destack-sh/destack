#![allow(clippy::type_complexity)]

use dyst_ast::{
    BindingKind, BindingModifier, BindingScope, DeclarationKind, DefinitionMeta, Expression,
    Keyword,
};

use crate::TokenType;
use crate::parse::prelude::*;

use crate::{Field, NodeId, NodeType, Parser, ParserError, ParserResult};

pub(crate) static BINDING_MODIFIERS: [Keyword; 6] = [
    Keyword::Static,
    Keyword::Override,
    Keyword::Readonly,
    Keyword::Public,
    Keyword::Protected,
    Keyword::Private,
];

impl<'a> Parser<'a> {
    /// Peek a variant field: `name: Type` with optional default `= <expr>`.
    /// May be preceded by any number of field modifiers.
    pub(crate) fn peek_field(&self) -> ParserResult<()> {
        let pos = self.pos() as usize;
        if pos + 3 < self.tokens.len() {
            let token_ty = self.tokens[pos].token.ty;
            let next_token_ty = self.tokens[pos + 1].token.ty;
            let next_next_token_ty = self.tokens[pos + 2].token.ty;
            match (token_ty, next_token_ty, next_next_token_ty) {
                // identifier:
                (TokenType::Identifier, TokenType::Colon, _)
                // identifier?:
                | (TokenType::Identifier, TokenType::Maybe, TokenType::Colon)
                // identifier = 
                | (TokenType::Identifier, TokenType::Assign, _)
                // string:
                | (TokenType::Literal, TokenType::Colon, _)
                // string?:
                | (TokenType::Literal, TokenType::Maybe, TokenType::Colon)
                => {
                    return Ok(());
                }
                // [ (dynamic field, NOT function)
                | (TokenType::OpenBracket, _, _) => {
                    // only if the closing bracket is followed by a colon or opening parenthesis
                    if let Ok(closing_pos) = self.find_open_and_matching_close(TokenType::OpenBracket, TokenType::CloseBracket)
                        && let Some(token_after) = self.tokens.get(closing_pos as usize + 1)
                         && token_after.token.ty == TokenType::Colon {
                            return Ok(());
                        }
                }
                _ => {}
            }
        }
        Err(ParserError::expected(
            self.tokens[pos].span,
            TokenType::Identifier,
        ))
    }

    /// Eat a single variant field: `T`, `name: T`, or `name: T = <expr>` (including modifiers).
    ///
    /// Examples:
    /// ```
    /// bar: int32
    /// T
    /// public T
    /// readonly name: T
    /// baz: T
    /// public T
    /// readonly bar: int32
    /// baz?: T // shorthand for baz: T?
    /// "Content-Type": string
    /// [x: string]: any
    /// [string]: woof
    /// [T] = "hello"
    /// ```
    fn eat_field(&mut self) -> ParserResult<NodeId<Field>> {
        let start = self.mark();

        // modifiers
        let mut modifiers = self.eat_binding_modifiers_prefix_maybe()?;

        // dynamic field
        if self.peek_token(TokenType::OpenBracket).is_ok() {
            self.bump(); // eat open bracket

            // name
            let name = if self.peek_identifier().is_ok()
                && self.peek_next_token(TokenType::Colon).is_ok()
            {
                let name = self.eat_identifier()?;
                self.bump(); // eat colon
                Some(name)
            } else {
                None
            };

            // key
            let key = self.eat_expression().for_node_type(NodeType::Field)?;

            // close bracket
            self.eat_token(TokenType::CloseBracket)?;

            // type
            self.eat_token(TokenType::Colon)?;
            let ty = self
                .with_options(self.options.in_type(), |parser| parser.eat_expression())
                .for_node_type(NodeType::Definition)?;

            // default
            let default = if self.peek_token(TokenType::Assign).is_ok() {
                self.bump(); // eat assign
                Some(self.eat_expression().for_node_type(NodeType::Definition)?)
            } else {
                None
            };

            // field
            let field_id = self.tree.insert(
                Field::Dynamic {
                    modifiers,
                    name,
                    ty,
                    key,
                    default,
                },
                self.get_span_from(start),
            );
            Ok(field_id)
        }
        // static field
        else {
            // name
            let (name, is_maybe) = {
                // name:
                if self.peek_name().is_ok() && self.peek_next_token(TokenType::Colon).is_ok() {
                    let name = self.eat_name()?;
                    self.bump(); // eat colon
                    self.eat_newlines_maybe()?;
                    (Some(name), false)
                }
                // name?:
                else if self.peek_name().is_ok()
                    && self.peek_next_token(TokenType::Maybe).is_ok()
                    && self.peek_next_next_token(TokenType::Colon).is_ok()
                {
                    let name = self.eat_name()?;
                    self.bump(); // eat maybe
                    self.bump(); // eat colon
                    self.eat_newlines_maybe()?;
                    (Some(name), true)
                } else {
                    (None, false)
                }
            };

            // postfix modifiers
            if is_maybe {
                modifiers = match modifiers {
                    Some(modifiers) => Some(modifiers.with_kind(BindingKind::Maybe)),
                    None => Some(BindingModifier::default().with_kind(BindingKind::Maybe)),
                };
            }

            // type
            let ty = self
                .with_options(self.options.in_type(), |parser| parser.eat_expression())
                .for_node_type(NodeType::Definition)?;

            // default
            let default = if self.peek_token(TokenType::Assign).is_ok() {
                self.bump(); // eat assign
                self.eat_newlines_maybe()?;
                let default = self.eat_expression().for_node_type(NodeType::Definition)?;
                Some(default)
            } else {
                None
            };

            // field
            let field = if let Some(name) = name {
                Field::Named {
                    modifiers,
                    name,
                    ty,
                    default,
                }
            } else {
                Field::Positional {
                    modifiers,
                    ty,
                    default,
                }
            };
            let field_id = self.tree.insert(field, self.get_span_from(start));
            Ok(field_id)
        }
    }

    /// Eat a variant body (without the parenthesis, without any expressions).
    pub fn eat_variant_body_fields(&mut self) -> ParserResult<Vec<NodeId<Field>>> {
        let mut tuple_fields: Vec<NodeId<Field>> = Vec::new();
        loop {
            // stop at closing parenthesis
            if self.peek_token(TokenType::CloseParenthesis).is_ok()
                || self.peek_token(TokenType::CloseBrace).is_ok()
            {
                break;
            }
            // consume any stop
            else if self.peek_any_stop().is_ok() {
                self.eat_any_stop_with_newlines()?;
            }
            // keep eating tuple fields
            else {
                let field = self.eat_field()?;
                tuple_fields.push(field);
            }
        }
        Ok(tuple_fields)
    }

    /// Eat a variant body (without the header or `{` and `}`).
    pub fn eat_variant_body_mixed(
        &mut self,
        allow_fields: bool,
    ) -> ParserResult<(Vec<NodeId<Field>>, Vec<NodeId<Expression>>)> {
        // eat everything
        let mut fields: Vec<NodeId<Field>> = Vec::new();
        let mut expressions: Vec<NodeId<Expression>> = Vec::new();
        loop {
            // stop on closing brace
            if self.peek_token(TokenType::CloseBrace).is_ok() {
                break;
            }
            // consume any stop
            else if self.peek_any_stop().is_ok() {
                self.eat_any_stop_with_newlines()?;
                continue;
            }

            // modifiers (semi-speculative lookahead)
            let modifier_start = self.mark();
            let kind = if self.peek_keyword(Keyword::Declare).is_ok() {
                self.bump(); // eat declare
                DeclarationKind::Declaration
            } else {
                DeclarationKind::Definition
            };
            let visibility = if let Ok(Some(visibility)) = self.peek_visibility() {
                self.bump(); // eat visibility
                Some(visibility)
            } else {
                None
            };
            let scope = if self.peek_keyword(Keyword::Static).is_ok() {
                self.bump(); // eat static
                BindingScope::Static
            } else {
                BindingScope::Container
            };
            if self.peek_keyword(Keyword::Readonly).is_ok() {
                self.bump(); // eat readonly
            }

            // variant field
            if allow_fields && self.peek_field().is_ok() {
                self.rewind(modifier_start);
                let field = self.eat_field().for_node_type(NodeType::Field)?;
                fields.push(field);
            }
            // function shorthand
            else if self
                .peek_token_in(&[
                    TokenType::LessThan,
                    TokenType::OpenParenthesis,
                    TokenType::OpenBracket,
                ])
                .is_ok()
                || (self.peek_name().is_ok()
                    && self
                        .peek_next_token_in(&[
                            TokenType::LessThan,
                            TokenType::OpenParenthesis,
                            TokenType::OpenBracket,
                        ])
                        .is_ok())
            {
                let meta: DefinitionMeta = DefinitionMeta {
                    kind,
                    scope,
                    name: None,
                    key: None,
                    export: None,
                    visibility,
                };
                let function_id = self.eat_function(meta, false, false)?;
                let function_id = self.tree.insert(
                    Expression::Definition(function_id),
                    self.tree.spans.get(function_id),
                );

                expressions.push(function_id);
            }
            // function maybe shorthand
            else if self.peek_name().is_ok()
                && self.peek_next_token(TokenType::Maybe).is_ok()
                && self
                    .peek_next_next_token_in(&[
                        TokenType::LessThan,
                        TokenType::OpenParenthesis,
                        TokenType::OpenBracket,
                    ])
                    .is_ok()
            {
                let meta: DefinitionMeta = DefinitionMeta {
                    kind,
                    scope,
                    name: None,
                    key: None,
                    export: None,
                    visibility,
                };
                let function_id = self.eat_function(meta, true, false)?;
                let function_id = self.tree.insert(
                    Expression::Definition(function_id),
                    self.tree.spans.get(function_id),
                );
                // nocheckin: maybe binding / shorthands are just member functions!, ..
                expressions.push(function_id);
            }
            // eat any other expressions
            else {
                self.rewind(modifier_start);
                let expression_id = self
                    .with_options(self.options.nested_in_variant(), |parser| {
                        parser.try_eat_expression(TokenType::Newline)
                    })
                    .for_node_type(NodeType::Expression)?;
                expressions.push(expression_id);
            }
        }

        Ok((fields, expressions))
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::{Definition, Name, TypeLiteral, Visibility};
    use dyst_source::{LanguageCompatibility, LanguageOptions};

    use crate::tests::TestParser;
    use crate::{assert_expr_path, assert_node, assert_path, assert_string};

    use super::*;

    #[test]
    fn test_parse_field_with_implicit_self_function() {
        let mut test = TestParser::new(r#"onconnect: (this: Client) => void"#);
        let mut parser = test.prepare();

        let field = parser.eat_field().unwrap();
        assert_node!(parser.tree, field, Field::Named { modifiers: None, name: Name::Identifier(name), ty, default: None, .. } => {
            assert_string!(parser, *name, "onconnect");
            // (this: Client) => void;
            assert_node!(parser.tree, *ty, Expression::Definition(definition_id) => {
                assert_node!(parser.tree, *definition_id, Definition::Function { self_parameter: Some(self_parameter), return_type, .. } => {
                    assert_expr_path!(parser, parser.tree.get(self_parameter.ty.unwrap()), "Client");
                    // void
                    assert_node!(parser.tree, return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Void));
                });
            });
        });
    }

    #[test]
    fn test_parse_field_with_es_visibility_modifier() {
        let mut test = TestParser::new_with_options(
            r#"#name: string"#,
            LanguageOptions::default().with_compatibility(LanguageCompatibility::TypeScript),
        );
        let mut parser = test.prepare();

        let field = parser.eat_field().unwrap();
        assert_node!(parser.tree, field, Field::Named { modifiers: Some(modifiers), name: Name::Identifier(name), ty, default: None, .. } => {
            // #name means private for #Compatibility
            assert_eq!(modifiers.visibility.unwrap(), Visibility::Private);
            // name
            assert_string!(parser, *name, "name");
            // string
            assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::String));
        });
    }
}
