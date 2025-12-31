use destack_ast::{
    AccessorKind, Argument, BindingAnchor, BindingKind, BindingModifier, BindingOperator,
    Expression, Keyword, LocalNodeId, Mutability, Name, NodeType, Parameter, Pattern,
    PostfixPosition, ScalarLiteral, StringId, Timing, TokenType,
};
use destack_source::NodeSpanType;

use crate::parse::prelude::*;
use crate::{ParseResult, Parser};

impl Parser {
    /// Eat a binding modifiers prefix (visibility and mutability).
    pub fn eat_binding_modifiers_prefix_maybe(&mut self) -> ParseResult<Option<BindingModifier>> {
        let mut modifiers: Option<BindingModifier> = None;
        // visibility
        if let Ok(Some(visibility)) = self.peek_visibility() {
            self.bump(); // eat visibility
            if modifiers.is_none() {
                modifiers = Some(BindingModifier::default());
            }
            modifiers.as_mut().unwrap().visibility = Some(visibility);
        }
        // scope
        if self.peek_keyword(Keyword::Static).is_ok() {
            self.bump(); // eat static
            if modifiers.is_none() {
                modifiers = Some(BindingModifier::default());
            }
            modifiers.as_mut().unwrap().anchor = Some(BindingAnchor::Static);
        }
        // mutability
        if self.peek_keyword(Keyword::Readonly).is_ok() {
            self.bump(); // eat readonly
            if modifiers.is_none() {
                modifiers = Some(BindingModifier::default());
            }
            modifiers.as_mut().unwrap().mutability = Some(Mutability::Immutable);
        }
        // operator
        if self.peek_keyword(Keyword::Const).is_ok() {
            self.bump(); // eat const
            if modifiers.is_none() {
                modifiers = Some(BindingModifier::default());
            }
            modifiers.as_mut().unwrap().operator = Some(BindingOperator::AsConst);
        }
        // accessor
        if self.peek_keyword(Keyword::Accessor).is_ok() {
            self.bump(); // eat accessor
            if modifiers.is_none() {
                modifiers = Some(BindingModifier::default());
            }
            modifiers.as_mut().unwrap().accessor = Some(AccessorKind::Accessor);
        }
        // timing
        if self.peek_keyword(Keyword::Comptime).is_ok() {
            self.bump(); // eat comptime
            if modifiers.is_none() {
                modifiers = Some(BindingModifier::default());
            }
            modifiers.as_mut().unwrap().timing = Some(Timing::Comptime);
        }
        Ok(modifiers)
    }

    /// Eat a binding modifiers postfix (maybe).
    pub fn eat_binding_modifiers_postfix_maybe(
        &mut self,
        modifiers: Option<BindingModifier>,
    ) -> ParseResult<Option<BindingModifier>> {
        if self.peek_token(TokenType::Maybe).is_ok() {
            self.bump(); // eat maybe
            if let Some(modifiers) = modifiers {
                Ok(Some(BindingModifier {
                    kind: Some(BindingKind::Maybe),
                    ..modifiers
                }))
            } else {
                Ok(Some(BindingModifier {
                    kind: Some(BindingKind::Maybe),
                    ..BindingModifier::default()
                }))
            }
        } else {
            Ok(modifiers)
        }
    }

    /// Eat a binding modifiers postfix.
    pub fn eat_binding_modifiers_postfix(
        &mut self,
        modifiers: Option<BindingModifier>,
    ) -> ParseResult<Option<BindingModifier>> {
        self.eat_token(TokenType::Maybe)?;
        if let Some(modifiers) = modifiers {
            Ok(Some(BindingModifier {
                kind: Some(BindingKind::Maybe),
                ..modifiers
            }))
        } else {
            Ok(Some(BindingModifier {
                kind: Some(BindingKind::Maybe),
                ..BindingModifier::default()
            }))
        }
    }

    /// Eat a parameter
    ///
    /// Examples:
    /// ```
    /// x
    /// T
    /// x: int32
    /// Validate: bool = false
    /// baz: @someMacro(T)
    /// _
    /// { x }
    /// { x }: MyType = Foo
    /// ...T
    /// ...args: int32[]
    /// ```
    pub fn eat_parameter(&mut self) -> ParseResult<LocalNodeId<Parameter>> {
        let start = self.mark();

        let mut modifiers = self.eat_binding_modifiers_prefix_maybe()?;

        // variadic
        let is_variadic = if self.peek_token(TokenType::Spread).is_ok() {
            self.bump(); // eat range or range wide
            true
        } else {
            false
        };

        // pattern/name
        let (pattern, name, name_span): (
            Option<LocalNodeId<Pattern>>,
            Option<StringId>,
            Option<destack_source::Span>,
        ) = {
            // ...[name] is just a name, not a pattern
            if is_variadic
                && (self.options.in_type || self.options.in_variant)
                && self.peek_token(TokenType::OpenBracket).is_ok()
            {
                self.bump(); // eat [
                self.eat_newlines_maybe()?;
                let (name, span) = self.eat_identifier_with_span()?;
                self.eat_newlines_maybe()?;
                self.eat_token(TokenType::CloseBracket)?;
                (None, Some(name), Some(span))
            }
            // pattern
            else if !is_variadic
                && (self
                    .peek_token_in(&[
                        TokenType::OpenParenthesis,
                        TokenType::OpenBracket,
                        TokenType::OpenBrace,
                    ])
                    .is_ok()
                    || self.peek_identifier_str("_").is_ok())
            {
                let pattern = self
                    .with_options(self.options.in_before_type(), |parser| parser.eat_pattern())?;
                (Some(pattern), None, None)
            }
            // name
            else {
                let (name, span) = self.eat_identifier_with_span()?;
                (None, Some(name), Some(span))
            }
        };

        // ? maybe
        if self.peek_token(TokenType::Maybe).is_ok() {
            self.bump(); // eat maybe
            if modifiers.is_none() {
                modifiers = Some(BindingModifier::default());
            }
            modifiers.as_mut().unwrap().kind = Some(BindingKind::Maybe);
        }

        // : type (or keyword for #Compatibility)
        let (ty, ty_span) = {
            if self.peek_colon().is_ok()
                || (self.options.in_static
                    && (self.peek_keyword(Keyword::Extends).is_ok()
                        || self.peek_keyword(Keyword::Implements).is_ok()))
            {
                let type_start = self.mark();
                self.bump(); // eat colon or keyword
                self.eat_newlines_maybe()?;
                let ty = self
                    .with_options(self.options.not_in_position().in_type(), |parser| {
                        parser.eat_expression()
                    })
                    .for_node_type(NodeType::Parameter)?;
                (Some(ty), Some(self.get_span_from(type_start)))
            } else {
                (None, None)
            }
        };

        // = value
        let parameter = {
            // has default value
            if !is_variadic && self.peek_token(TokenType::Assign).is_ok() {
                self.bump(); // eat assign
                self.eat_newlines_maybe()?;
                let value = self
                    .with_options(self.options.not_in_position(), |parser| {
                        parser.eat_expression()
                    })
                    .for_node_type(NodeType::Parameter)?;
                // named with default
                if let Some(name) = name {
                    Parameter::Named {
                        modifiers,
                        name,
                        ty,
                        default: Some(value),
                    }
                }
                // pattern with default
                else {
                    Parameter::Pattern {
                        modifiers,
                        pattern: pattern.expect("peeked"),
                        ty,
                        default: Some(value),
                    }
                }
            }
            // variadic parameter (cannot have a default value)
            else if is_variadic {
                Parameter::Variadic {
                    modifiers,
                    name: name.expect("peeked"),
                    ty,
                }
            }
            // no default value
            else {
                // named without default
                if let Some(name) = name {
                    Parameter::Named {
                        modifiers,
                        name,
                        ty,
                        default: None,
                    }
                }
                // pattern without default
                else {
                    Parameter::Pattern {
                        modifiers,
                        pattern: pattern.expect("peeked"),
                        ty,
                        default: None,
                    }
                }
            }
        };

        // parameter
        let parameter_id = self.tree.insert(parameter, self.get_span_from(start));

        // set main span to the name identifier
        if let Some(span) = name_span {
            self.tree.set_main_span(parameter_id, span);
        }

        // set type span for the type annotation
        if let Some(span) = ty_span {
            self.tree
                .set_side_span(parameter_id, NodeSpanType::Type, span);
        }

        Ok(parameter_id)
    }

    /// Eat a parameter list. May be comma or newline separated.
    ///
    /// Examples:
    /// ```
    /// x: int32
    /// x: int32, y: int32
    /// ```
    pub fn eat_parameters_body(&mut self) -> ParseResult<Vec<LocalNodeId<Parameter>>> {
        let mut parameters: Vec<LocalNodeId<Parameter>> = Vec::new();
        self.eat_newlines_maybe()?;
        while self.peek_token(TokenType::Identifier).is_ok()
            // spread
            || self.peek_token(TokenType::Spread).is_ok()
            // pattern
            || self.peek_token(TokenType::OpenParenthesis).is_ok()
            || self.peek_token(TokenType::OpenBracket).is_ok()
            || self.peek_token(TokenType::OpenBrace).is_ok()
        {
            let parameter = self.eat_parameter().for_node_type(NodeType::Parameter)?;
            parameters.push(parameter);
            self.eat_newlines_maybe()?;
            if self.peek_item_stop().is_ok() {
                self.eat_item_stop_with_newlines()?;
            } else {
                break;
            }
        }
        Ok(parameters)
    }

    /// Eat static parameters (including the `<` and `>` tokens) if they exist.
    pub fn eat_static_parameters_maybe(
        &mut self,
    ) -> ParseResult<Option<Vec<LocalNodeId<Parameter>>>> {
        if self.peek_token(TokenType::LessThan).is_ok() {
            return Ok(Some(self.eat_static_parameters()?));
        }
        Ok(None)
    }

    /// Eat static parameters (including the `<` and `>` tokens).
    pub fn eat_static_parameters(&mut self) -> ParseResult<Vec<LocalNodeId<Parameter>>> {
        let start = self.mark();
        self.eat_token(TokenType::LessThan)?;
        self.eat_newlines_maybe()?;

        // empty static parameters are not allowed
        if self.peek_token(TokenType::GreaterThan).is_ok() {
            return Err(ParseError::expected(
                self.get_span_from(start),
                TokenType::Identifier,
            ));
        }

        // regular static parameters
        let parameters = self.with_options(self.options.nested().in_static(), |parser| {
            parser.eat_parameters_body()
        })?;
        self.eat_token(TokenType::GreaterThan)?;
        Ok(parameters)
    }

    /// Eat dynamic parameters (including the `(` and `)` tokens) if they exist.
    pub fn eat_dynamic_parameters_maybe(
        &mut self,
    ) -> ParseResult<Option<Vec<LocalNodeId<Parameter>>>> {
        if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            return Ok(Some(self.eat_dynamic_parameters()?));
        }
        Ok(None)
    }

    /// Eat dynamic parameters (including the `(` and `)` tokens).
    pub fn eat_dynamic_parameters(&mut self) -> ParseResult<Vec<LocalNodeId<Parameter>>> {
        self.eat_token(TokenType::OpenParenthesis)?;
        self.eat_newlines_maybe()?;

        // empty dynamic parameters
        if self.peek_token(TokenType::CloseParenthesis).is_ok() {
            self.bump(); // eat close parenthesis
            return Ok(vec![]);
        }

        // regular dynamic parameters
        let mut parameter_options = self.options.nested();
        if self.options.in_type {
            parameter_options = parameter_options.in_type();
        }
        if self.options.in_variant {
            parameter_options = parameter_options.in_variant();
        }
        let parameters =
            self.with_options(parameter_options, |parser| parser.eat_parameters_body())?;
        self.eat_token(TokenType::CloseParenthesis)?;
        Ok(parameters)
    }

    /// Split out an explicit `this` parameter (type-only).
    pub fn split_this_parameter_maybe(
        &mut self,
        mut parameters: Vec<LocalNodeId<Parameter>>,
    ) -> (Option<LocalNodeId<Parameter>>, Vec<LocalNodeId<Parameter>>) {
        // only the first parameter can be `this`
        let this_parameter = if let Some(first_id) = parameters.first().copied() {
            if let Parameter::Named { name, .. } = self.tree.get(first_id) {
                let this_id = self.strings.intern("this");
                if *name == this_id {
                    Some(first_id)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        if this_parameter.is_some() {
            parameters.remove(0);
        }

        (this_parameter, parameters)
    }

    /// Eat a positional argument (positional, spread, or labeled tuple element).
    /// Used for dynamic arguments, static arguments, and tuple literals.
    ///
    /// Examples:
    /// ```
    /// 2
    /// foo()
    /// ...args
    /// start: number    // labeled tuple element (type context only)
    /// ```
    #[inline]
    pub fn eat_positional_argument(&mut self) -> ParseResult<LocalNodeId<Argument>> {
        let start = self.mark();
        let mut modifiers = None;

        if self.options.in_type && self.peek_keyword(Keyword::Readonly).is_ok() {
            self.bump(); // eat readonly
            modifiers = Some(BindingModifier {
                mutability: Some(Mutability::Immutable),
                ..BindingModifier::default()
            });
        }

        // spread argument
        if self.peek_token(TokenType::Spread).is_ok() {
            self.bump(); // eat spread
            let value = self.with_options(self.options.not_in_position(), |parser| {
                parser.eat_expression()
            })?;
            let argument_id = self.tree.insert(
                Argument::Spread { modifiers, value },
                self.get_span_from(start),
            );
            Ok(argument_id)
        }
        // labeled tuple element (only in type context): label: type
        else if self.options.in_type
            && self.peek_token(TokenType::Identifier).is_ok()
            && (self.peek_next_token(TokenType::Colon).is_ok()
                || self.peek_next_token(TokenType::Maybe).is_ok()
                    && self.peek_next_next_token(TokenType::Colon).is_ok())
        {
            let label = self.eat_identifier()?;
            if self.peek_token(TokenType::Maybe).is_ok() {
                self.bump(); // eat ?
                if modifiers.is_none() {
                    modifiers = Some(BindingModifier::default());
                }
                modifiers.as_mut().unwrap().kind = Some(BindingKind::Maybe);
            }
            self.bump(); // eat colon
            let value = self.eat_expression()?;
            let argument_id = self.tree.insert(
                Argument::Labeled {
                    modifiers,
                    label,
                    value,
                },
                self.get_span_from(start),
            );
            Ok(argument_id)
        }
        // positional argument
        else {
            let mut value = self.eat_expression()?;
            if self.options.in_type
                && let Expression::Maybe {
                    position: PostfixPosition::Direct,
                    left,
                } = self.tree.get(value)
            {
                if modifiers.is_none() {
                    modifiers = Some(BindingModifier::default());
                }
                modifiers.as_mut().unwrap().kind = Some(BindingKind::Maybe);
                value = *left;
            }
            let argument_id = self.tree.insert(
                Argument::Positional { modifiers, value },
                self.get_span_from(start),
            );
            Ok(argument_id)
        }
    }

    /// Eat an argument (e.g., `x: 1` or `y`).
    /// Supports named arguments (for tree literal children and import with syntax).
    ///
    /// Examples:
    /// ```
    /// x: 1
    /// y
    /// 2
    /// ...args
    /// "Content-Type": "application/json"
    /// ```
    #[inline]
    pub fn eat_tree_argument(&mut self) -> ParseResult<LocalNodeId<Argument>> {
        let start = self.mark();
        // named argument (name: value)
        if self.peek_name().is_ok() && self.peek_next_token(TokenType::Colon).is_ok() {
            let name = self.eat_name().for_node_type(NodeType::Argument)?;
            self.bump(); // eat colon
            self.eat_newlines_maybe()?;
            // value
            let value = self.with_options(self.options.not_in_position(), |parser| {
                parser.eat_expression()
            })?;
            let argument_id = self.tree.insert(
                Argument::Named {
                    modifiers: None,
                    name,
                    value,
                },
                self.get_span_from(start),
            );
            Ok(argument_id)
        }
        // spread argument (...expr)
        else if self.peek_token(TokenType::Spread).is_ok() {
            self.bump(); // eat spread
            let value = self.with_options(self.options.not_in_position(), |parser| {
                parser.eat_expression()
            })?;
            let argument_id = self.tree.insert(
                Argument::Spread {
                    modifiers: None,
                    value,
                },
                self.get_span_from(start),
            );
            Ok(argument_id)
        }
        // expression container ({expr}) - TSX syntax where {} are delimiters, not part of expr
        else if self.peek_token(TokenType::OpenBrace).is_ok() {
            self.bump(); // eat {
            self.eat_newlines_maybe()?;
            // if empty container (e.g., {/* comment */} where comment is filtered out)
            if self.peek_token(TokenType::CloseBrace).is_ok() {
                self.bump(); // eat }
                // insert a stub expression for empty container
                let value = self
                    .tree
                    .insert(Expression::Stub, self.get_span_from(start));
                let argument_id = self.tree.insert(
                    Argument::Positional {
                        modifiers: None,
                        value,
                    },
                    self.get_span_from(start),
                );
                return Ok(argument_id);
            }
            let value = self.with_options(
                self.options
                    .not_in_position()
                    .not_in_tree_literal()
                    .not_in_left_precedence(),
                |parser| parser.eat_expression(),
            )?;
            self.eat_newlines_maybe()?;
            self.eat_token(TokenType::CloseBrace)?;
            let argument_id = self.tree.insert(
                Argument::Positional {
                    modifiers: None,
                    value,
                },
                self.get_span_from(start),
            );
            Ok(argument_id)
        }
        // positional argument (bare expression like nested <Element />)
        else {
            let value = self.with_options(self.options.not_in_position(), |parser| {
                parser.eat_expression()
            })?;
            let argument_id = self.tree.insert(
                Argument::Positional {
                    modifiers: None,
                    value,
                },
                self.get_span_from(start),
            );
            Ok(argument_id)
        }
    }

    /// Eat a tree literal argument (e.g., `x=1` or `long-name=2` or `flag-is-set`).
    ///
    /// Examples:
    /// ```
    /// x=1
    /// y
    /// 2
    /// ...args
    /// ```
    #[inline]
    pub fn eat_tree_literal_argument(&mut self) -> ParseResult<LocalNodeId<Argument>> {
        let start = self.mark();
        // spread argument
        if self.peek_token(TokenType::Spread).is_ok() {
            self.bump(); // eat spread
            let value = self.with_options(self.options.in_statement_position(), |parser| {
                parser.eat_expression()
            })?;
            let argument_id = self.tree.insert(
                Argument::Spread {
                    modifiers: None,
                    value,
                },
                self.get_span_from(start),
            );
            Ok(argument_id)
        }
        // nested spread argument (like {...b} in tree literals for #Compatibility)
        else if self.peek_token(TokenType::OpenBrace).is_ok()
            && self.peek_next_token(TokenType::Spread).is_ok()
            && self.peek_next_next_token(TokenType::Identifier).is_ok()
        {
            self.bump(); // eat open brace
            self.bump(); // eat spread
            let value = self.with_options(self.options.in_statement_position(), |parser| {
                parser.eat_expression()
            })?;
            self.eat_token(TokenType::CloseBrace)?;
            let argument_id = self.tree.insert(
                Argument::Spread {
                    modifiers: None,
                    value,
                },
                self.get_span_from(start),
            );
            Ok(argument_id)
        }
        // named argument
        else {
            let name = self.eat_tree_literal_identifier()?;

            // named argument with value
            let value = if self.peek_token(TokenType::Colon).is_ok()
                || self.peek_token(TokenType::Assign).is_ok()
            {
                self.bump(); // eat colon or assign
                self.eat_newlines_maybe()?;
                // TSX expression container: attr={expr} - braces are delimiters
                if self.peek_token(TokenType::OpenBrace).is_ok() {
                    self.bump(); // eat {
                    self.eat_newlines_maybe()?;
                    let value = self.with_options(
                        self.options
                            .not_in_position()
                            .not_in_tree_literal()
                            .not_in_left_precedence(),
                        |parser| parser.eat_expression(),
                    )?;
                    self.eat_newlines_maybe()?;
                    self.eat_token(TokenType::CloseBrace)?;
                    value
                } else {
                    // bare expression (like attr="string" or attr=true)
                    self.with_options(self.options.not_in_position(), |parser| {
                        parser.eat_expression()
                    })?
                }
            }
            // implicit boolean true
            else {
                self.tree.insert(
                    Expression::ScalarLiteral(ScalarLiteral::Boolean(true)),
                    self.get_span_from(start),
                )
            };

            let argument_id = self.tree.insert(
                Argument::Named {
                    modifiers: None,
                    name: Name::Identifier(name),
                    value,
                },
                self.get_span_from(start),
            );
            Ok(argument_id)
        }
    }

    /// Eat static arguments (including the `<` and `>` tokens) if they exist.
    pub fn eat_static_arguments_maybe(
        &mut self,
    ) -> ParseResult<Option<Vec<LocalNodeId<Argument>>>> {
        if self.peek_token(TokenType::LessThan).is_ok() {
            return Ok(Some(self.eat_static_arguments()?));
        }
        Ok(None)
    }

    /// Eat static arguments (including the `<` and `>` tokens).
    /// Only positional and spread arguments are allowed (no named arguments).
    pub fn eat_static_arguments(&mut self) -> ParseResult<Vec<LocalNodeId<Argument>>> {
        let start = self.mark();
        self.eat_token(TokenType::LessThan)?;
        self.eat_newlines_maybe()?;

        // empty static arguments are not allowed
        if self.peek_token(TokenType::GreaterThan).is_ok() {
            return Err(ParseError::expected(
                self.get_span_from(start),
                TokenType::Identifier,
            ));
        }

        // regular static arguments (positional/spread only)
        let mut options = self.options.nested().in_static();
        if self.options.in_type {
            options = options.in_type();
        }
        let static_arguments = self.with_options(options, |parser| {
            parser.eat_positional_arguments_body(TokenType::GreaterThan)
        })?;

        self.eat_newlines_maybe()?;
        self.eat_token(TokenType::GreaterThan)?;
        Ok(static_arguments)
    }

    /// Eat dynamic arguments (including the `(` and `)` tokens) if they exist.
    pub fn eat_dynamic_arguments_maybe(
        &mut self,
    ) -> ParseResult<Option<Vec<LocalNodeId<Argument>>>> {
        if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            return Ok(Some(self.eat_dynamic_arguments()?));
        }
        Ok(None)
    }

    /// Eat dynamic arguments (including the `(` and `)` tokens).
    /// Only positional and spread arguments are allowed (no named arguments).
    pub fn eat_dynamic_arguments(&mut self) -> ParseResult<Vec<LocalNodeId<Argument>>> {
        self.eat_token(TokenType::OpenParenthesis)?;
        self.eat_newlines_maybe()?;

        // empty dynamic arguments
        if self.peek_token(TokenType::CloseParenthesis).is_ok() {
            self.bump(); // eat close parenthesis
            return Ok(vec![]);
        }

        // regular dynamic arguments
        let dynamic_arguments = self.with_options(self.options.nested(), |parser| {
            parser.eat_positional_arguments_body(TokenType::CloseParenthesis)
        })?;

        self.eat_newlines_maybe()?;
        self.eat_token(TokenType::CloseParenthesis)?;

        Ok(dynamic_arguments)
    }

    /// Eat a call argument list (positional/spread only). May be comma or newline separated.
    ///
    /// Examples:
    /// ```
    /// 1, 2
    /// foo()
    /// ...args
    /// ```
    #[inline]
    pub fn eat_positional_arguments_body(
        &mut self,
        terminator: TokenType,
    ) -> ParseResult<Vec<LocalNodeId<Argument>>> {
        let mut arguments: Vec<LocalNodeId<Argument>> = Vec::new();
        self.eat_newlines_maybe()?;
        while self.peek().is_ok() {
            if self.peek_token(terminator).is_ok() {
                break;
            }
            let argument_id = self.eat_positional_argument()?;
            arguments.push(argument_id);
            self.eat_newlines_maybe()?;
            if self.peek_item_stop().is_ok() {
                self.eat_item_stop_with_newlines()?;
            } else {
                break;
            }
        }
        Ok(arguments)
    }

    /// Eat an argument list (including named). May be comma or newline separated.
    /// Used for tree literal children and import assertions where named arguments are allowed.
    ///
    /// Examples:
    /// ```
    /// T: SomeType
    /// 3
    /// T: SomeType, U: OtherType
    /// ```
    #[inline]
    pub fn eat_arguments_body(
        &mut self,
        terminator: TokenType,
    ) -> ParseResult<Vec<LocalNodeId<Argument>>> {
        let mut arguments: Vec<LocalNodeId<Argument>> = Vec::new();
        self.eat_newlines_maybe()?;
        while self.peek().is_ok() {
            if self.peek_token(terminator).is_ok() {
                break;
            }
            let argument_id = self.eat_tree_argument()?;
            arguments.push(argument_id);
            self.eat_newlines_maybe()?;
            if self.peek_item_stop().is_ok() {
                self.eat_item_stop_with_newlines()?;
            } else {
                break;
            }
        }
        Ok(arguments)
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        Argument, BindingKind, BindingOperator, Expression, IntType, Mutability, Name, Parameter,
        Pattern, PatternField, ScalarLiteral, Timing, TypeLiteral, Visibility,
    };

    use crate::{TestParser, assert_name, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_parameter_type_only() {
        // T
        let mut test = TestParser::new("T");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Named { modifiers: _, name, ty, default } => {
            assert_string!(parser, *name, "T");
            assert!(ty.is_none());
            assert!(default.is_none());
        });
    }

    #[test]
    fn test_parse_parameter_with_type() {
        // x: int32
        let mut test = TestParser::new("x: int32");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Named { modifiers: _, name, ty, default } => {
            assert_string!(parser, *name, "x");
            assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary {
                width: Some(32),
                is_signed: true
            })));
            assert!(default.is_none());
        });
    }

    #[test]
    fn test_parse_parameter_with_maybe_type() {
        // x?: int32
        let mut test = TestParser::new("x?: int32");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Named { modifiers: Some(modifiers), name, ty: Some(ty), default: None } => {
            assert_string!(parser, *name, "x");
            assert_eq!(modifiers.kind, Some(BindingKind::Maybe));
            assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary {
                width: Some(32),
                is_signed: true
            })));
        });
    }

    #[test]
    fn test_parse_parameter_with_default() {
        // validate: boolean = false
        let mut test = TestParser::new("validate: boolean = false");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Named { modifiers: _, name, ty, default } => {
            assert_string!(parser, *name, "validate");
            assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Boolean));
            assert!(default.is_some());
        });
    }

    #[test]
    fn test_parse_parameter_with_pattern_and_defaults() {
        // { x }: T = false
        let mut test = TestParser::new("{ x = 4 }: boolean = false");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Pattern { modifiers: _, pattern, ty: Some(ty), default: Some(default) } => {
            // { x = 4 }
            assert_node!(parser.tree, *pattern, Pattern::Object { fields } => {
                assert_node!(parser.tree, fields[0], PatternField::Named { mutability: None, name, pattern: None, default: Some(default) } => {
                    // x
                    assert_name!(parser, *name, "x");
                    // 4
                    assert_node!(parser.tree, *default, Expression::ScalarLiteral(ScalarLiteral::Integer(4)));
                });
            });
            // boolean
            assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Boolean));
            // = false
            assert_node!(parser.tree, *default, Expression::ScalarLiteral(ScalarLiteral::Boolean(false)));
        });
    }

    #[test]
    fn test_parse_parameter_variadic() {
        // ...args
        let mut test = TestParser::new("...args");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Variadic { modifiers: _, name, ty } => {
            assert_string!(parser, *name, "args");
            assert!(ty.is_none());
        });
    }

    #[test]
    fn test_parse_parameter_variadic_with_type() {
        // ...args: int32[]
        let mut test = TestParser::new("...args: int32[]");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Variadic { modifiers: _, name, ty } => {
            assert_string!(parser, *name, "args");
            assert!(ty.is_some());
        });
    }

    /// Parse variadic tuple parameter names.
    #[test]
    fn test_parse_parameter_variadic_tuple_name() {
        let mut test = TestParser::new("...[value]: [] | [TNext]");
        let mut parser = test.prepare();
        parser.options.in_variant = true;
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Variadic { modifiers: _, name, ty } => {
            assert_string!(parser, *name, "value");
            assert!(ty.is_some());
        });
    }

    #[test]
    fn test_parse_parameter_multiline() {
        // x: int32
        let mut test = TestParser::new("x:\n\tint32");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Named { modifiers: _, name, ty: Some(ty), default: None } => {
            assert_string!(parser, *name, "x");
            assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary {
                width: Some(32),
                is_signed: true
            })));
        });
    }

    #[test]
    fn test_parse_parameter_with_modifiers() {
        // private readonly const x: 1
        let mut test = TestParser::new("private readonly const x: 1");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Named { modifiers: Some(modifiers), .. } => {
            assert_eq!(modifiers.visibility, Some(Visibility::Private));
            assert_eq!(modifiers.mutability, Some(Mutability::Immutable));
            assert_eq!(modifiers.operator, Some(BindingOperator::AsConst));
        });
    }

    #[test]
    fn test_parse_parameter_comptime() {
        // comptime n: int32
        let mut test = TestParser::new("comptime n: int32");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Named { modifiers: Some(modifiers), name, .. } => {
            assert_string!(parser, *name, "n");
            assert_eq!(modifiers.timing, Some(Timing::Comptime));
        });
    }

    // named arguments (only valid for tree literals, not dynamic or static arguments)

    #[test]
    fn test_parse_named_argument() {
        // x: 1
        let mut test = TestParser::new("x: 1");
        let mut parser = test.prepare();
        let argument_id = parser.eat_tree_argument().unwrap();
        assert_node!(parser.tree, argument_id, Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
            // x
            assert_string!(parser, *name, "x");
            // 1
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
    }

    // positional arguments (for dynamic args, static args, tuples)

    #[test]
    fn test_parse_positional_argument() {
        // 3
        let mut test = TestParser::new("3");
        let mut parser = test.prepare();
        let argument_id = parser.eat_positional_argument().unwrap();

        assert_node!(parser.tree, argument_id, Argument::Positional { modifiers: _, value } => {
            // 3
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
        });
    }

    #[test]
    fn test_parse_spread_argument() {
        // ...args
        let mut test = TestParser::new("...args");
        let mut parser = test.prepare();
        let argument_id = parser.eat_positional_argument().unwrap();
        assert_node!(parser.tree, argument_id, Argument::Spread { modifiers: _, value } => {
            // ...args
            assert_node!(parser.tree, *value, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "args");
            });
        });
    }
}
