use destack_ast::{
    AbstractionModifier, AccessorKind, Argument, BindingAnchor, BindingKind, BindingModifier,
    BindingOperator, DeclarationKind, Expression, Keyword, LiteralType, LocalNodeId, Mutability,
    Name, NodeType, Parameter, Pattern, PostfixPosition, ScalarLiteral, StringId, Timing,
    TokenType, VarianceModifier,
};
use destack_source::NodeSpanType;

use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser};

impl Parser {
    /// Return true when the next token can start a member name.
    fn next_token_starts_member_name(&self) -> bool {
        // skip newlines after the modifier keyword
        let mut pos = self.pos() as usize;
        while let Some(token) = self.tokens.get(pos + 1)
            && token.token.ty == TokenType::Newline
        {
            pos += 1;
        }
        let Some(next_token) = self.tokens.get(pos + 1) else {
            return false;
        };

        // check for common member name starters
        if matches!(
            next_token.token.ty,
            TokenType::Identifier
                | TokenType::Hash
                | TokenType::OpenBracket
                | TokenType::OpenBrace
                | TokenType::OpenParenthesis
                | TokenType::Spread
                | TokenType::Multiply
        ) {
            return true;
        }
        if next_token.token.ty != TokenType::Literal {
            return false;
        }

        // check for valid literal member names
        match next_token.token.literal {
            Some(LiteralType::String {
                is_terminated: true,
                has_invalid_escape: false,
            }) => true,
            Some(LiteralType::Character { is_terminated, .. })
                if is_terminated
                    && (self.language.is_typescript() || self.language.is_javascript()) =>
            {
                true
            }
            Some(LiteralType::Int { .. }) | Some(LiteralType::Float { .. }) => true,
            _ => false,
        }
    }

    /// Return true when `abstract` and `override` can be parsed as modifiers.
    fn can_parse_abstraction_modifier(&self) -> bool {
        !self.peek_next_is(TokenType::Colon)
            && !self.peek_next_is(TokenType::Maybe)
            && !self.peek_next_is(TokenType::LessThan)
            && !self.peek_next_is(TokenType::OpenParenthesis)
    }

    /// Eat a binding modifiers prefix when present.
    pub fn eat_binding_modifiers_prefix_maybe(
        &mut self,
        allow_readonly_key: bool,
        allow_accessor_modifier: bool,
        allow_variance_modifier: bool,
        allow_declare_modifier: bool,
        validate_modifier_order: bool,
    ) -> ParseResult<Option<BindingModifier>> {
        // initialize modifier state
        let mut modifiers = BindingModifier::default();
        let mut has_modifiers = false;

        // modifier ordering for TS compatibility
        let mut seen_static = false;
        let mut seen_override = false;
        let mut seen_readonly = false;

        // eat modifiers in any order
        loop {
            let mut progress = false;

            // modifier disambiguation for abstraction keywords
            let abstraction_is_modifier = self.can_parse_abstraction_modifier();

            // variance for static parameters
            if self.options.in_static || allow_variance_modifier {
                // handle 'in' variance modifier
                if self.peek_keyword(Keyword::In).is_ok() {
                    self.bump(); // eat in
                    modifiers.variance = Some(match modifiers.variance {
                        Some(VarianceModifier::Out) => VarianceModifier::InOut,
                        Some(VarianceModifier::InOut) => VarianceModifier::InOut,
                        _ => VarianceModifier::In,
                    });
                    has_modifiers = true;
                    progress = true;
                }
                // handle 'out' variance modifier
                else if self.peek_identifier_str("out").is_ok()
                    && (self.peek_next_is(TokenType::Identifier)
                        || self.peek_next_keyword(Keyword::In).is_ok())
                {
                    self.bump(); // eat out
                    modifiers.variance = Some(match modifiers.variance {
                        Some(VarianceModifier::In) => VarianceModifier::InOut,
                        Some(VarianceModifier::InOut) => VarianceModifier::InOut,
                        _ => VarianceModifier::Out,
                    });
                    has_modifiers = true;
                    progress = true;
                }
            }

            // visibility modifiers
            if let Ok(Some(visibility)) = self.peek_visibility() {
                if !self.next_token_starts_member_name() {
                    break;
                }
                let span = self.peek()?.span;
                self.bump(); // eat visibility
                if modifiers.visibility.is_some() {
                    if validate_modifier_order {
                        self.error(&ParseError::unexpected(span));
                    }
                } else {
                    if validate_modifier_order && (seen_static || seen_override || seen_readonly) {
                        self.error(&ParseError::unexpected(span));
                    }
                    modifiers.visibility = Some(visibility);
                }
                has_modifiers = true;
                progress = true;
            }

            // declaration modifiers
            if allow_declare_modifier && self.peek_keyword(Keyword::Declare).is_ok() {
                if !self.next_token_starts_member_name() {
                    break;
                }
                let span = self.peek()?.span;
                self.bump(); // eat declare
                if modifiers.declaration.is_some() {
                    if validate_modifier_order {
                        self.error(&ParseError::unexpected(span));
                    }
                } else {
                    modifiers.declaration = Some(DeclarationKind::Declaration);
                }
                has_modifiers = true;
                progress = true;
            }

            // scope modifiers (static)
            if modifiers.anchor.is_none() && self.peek_keyword(Keyword::Static).is_ok() {
                if !self.next_token_starts_member_name() {
                    break;
                }
                let span = self.peek()?.span;
                self.bump(); // eat static
                modifiers.anchor = Some(BindingAnchor::Static);
                if validate_modifier_order && seen_override {
                    self.error(&ParseError::unexpected(span));
                }
                seen_static = true;
                has_modifiers = true;
                progress = true;
            }

            // abstraction modifiers (abstract)
            if self.peek_keyword(Keyword::Abstract).is_ok() && abstraction_is_modifier {
                if !self.next_token_starts_member_name() {
                    break;
                }
                let _span = self.peek()?.span;
                self.bump(); // eat abstract
                modifiers.abstraction = Some(match modifiers.abstraction {
                    None => AbstractionModifier::Abstract,
                    Some(AbstractionModifier::Override) => AbstractionModifier::AbstractOverride,
                    Some(AbstractionModifier::Abstract) => AbstractionModifier::Abstract,
                    Some(AbstractionModifier::AbstractOverride) => {
                        AbstractionModifier::AbstractOverride
                    }
                });
                has_modifiers = true;
                progress = true;
            }

            // abstraction modifiers (override)
            if self.peek_keyword(Keyword::Override).is_ok() && abstraction_is_modifier {
                if !self.next_token_starts_member_name() {
                    break;
                }
                let span = self.peek()?.span;
                self.bump(); // eat override
                modifiers.abstraction = Some(match modifiers.abstraction {
                    None => AbstractionModifier::Override,
                    Some(AbstractionModifier::Abstract) => AbstractionModifier::AbstractOverride,
                    Some(AbstractionModifier::Override) => AbstractionModifier::Override,
                    Some(AbstractionModifier::AbstractOverride) => {
                        AbstractionModifier::AbstractOverride
                    }
                });
                if validate_modifier_order && seen_readonly {
                    self.error(&ParseError::unexpected(span));
                }
                seen_override = true;
                has_modifiers = true;
                progress = true;
            }

            // mutability modifiers (readonly)
            // allow treating readonly as a key in property contexts
            let readonly_is_modifier = if allow_readonly_key {
                self.peek_next_is(TokenType::Identifier)
                    || self.peek_next_is(TokenType::OpenBracket)
            } else {
                true
            };
            if modifiers.mutability.is_none()
                && self.peek_keyword(Keyword::Readonly).is_ok()
                && readonly_is_modifier
            {
                self.bump(); // eat readonly
                modifiers.mutability = Some(Mutability::Immutable);
                seen_readonly = true;
                has_modifiers = true;
                progress = true;
            }

            // explicit mutability modifiers (mut)
            if modifiers.mutability.is_none() && self.peek_keyword(Keyword::Mut).is_ok() {
                self.bump(); // eat mut
                modifiers.mutability = Some(Mutability::Mutable);
                has_modifiers = true;
                progress = true;
            }

            // operator modifiers (const)
            if modifiers.operator.is_none() && self.peek_keyword(Keyword::Const).is_ok() {
                if !self.next_token_starts_member_name() {
                    break;
                }
                self.bump(); // eat const
                modifiers.operator = Some(BindingOperator::AsConst);
                has_modifiers = true;
                progress = true;
            }

            // accessor modifiers
            let accessor_is_modifier = allow_accessor_modifier
                && self.peek_keyword(Keyword::Accessor).is_ok()
                && !self.peek_next_is(TokenType::Colon)
                && !self.peek_next_is(TokenType::Maybe)
                && !self.peek_next_is(TokenType::LessThan)
                && !self.peek_next_is(TokenType::OpenParenthesis);
            if modifiers.accessor.is_none() && accessor_is_modifier {
                if !self.next_token_starts_member_name() {
                    break;
                }
                self.bump(); // eat accessor
                modifiers.accessor = Some(AccessorKind::Accessor);
                has_modifiers = true;
                progress = true;
            }

            // timing modifiers (comptime)
            if modifiers.timing.is_none() && self.peek_keyword(Keyword::Comptime).is_ok() {
                if !self.next_token_starts_member_name() {
                    break;
                }
                self.bump(); // eat comptime
                modifiers.timing = Some(Timing::Comptime);
                has_modifiers = true;
                progress = true;
            }

            // exit loop if no progress made
            if !progress {
                break;
            }
        }

        if has_modifiers {
            Ok(Some(modifiers))
        } else {
            Ok(None)
        }
    }

    /// Eat a binding modifiers postfix (maybe).
    pub fn eat_binding_modifiers_postfix_maybe(
        &mut self,
        modifiers: Option<BindingModifier>,
    ) -> ParseResult<Option<BindingModifier>> {
        if self.peek_is(TokenType::Maybe) {
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

        let mut modifiers =
            self.eat_binding_modifiers_prefix_maybe(true, false, false, false, true)?;

        // variadic
        let is_variadic = if self.peek_is(TokenType::Spread) {
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
                && self.peek_is(TokenType::OpenBracket)
            {
                self.bump(); // eat [
                self.eat_newlines_maybe()?;
                let (name, span) = self.eat_binding_identifier_with_span()?;
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
                let (name, span) = self.eat_binding_identifier_with_span()?;
                (None, Some(name), Some(span))
            }
        };

        // ? maybe
        if self.peek_is(TokenType::Maybe) {
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
                let mut type_options = self
                    .options
                    .not_in_position()
                    .not_in_left_precedence()
                    .in_type();
                if self.options.in_type_conditional_right {
                    type_options = type_options.in_type_conditional_right();
                }
                let ty = self
                    .with_options(type_options, |parser| parser.eat_expression())
                    .for_node_type(NodeType::Parameter)?;
                (Some(ty), Some(self.get_span_from(type_start)))
            } else {
                (None, None)
            }
        };

        // = value
        let parameter = {
            // has default value
            if !is_variadic && self.peek_is(TokenType::Assign) {
                self.bump(); // eat assign
                self.eat_newlines_maybe()?;

                // value
                let value_options = if self.options.in_static {
                    self.options
                        .not_in_position()
                        .not_in_sequence_expression()
                        .in_type()
                } else {
                    self.options.not_in_position().not_in_sequence_expression()
                };
                let value = self
                    .with_options(value_options, |parser| parser.eat_expression())
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
        while self.peek_is(TokenType::Identifier)
            || (self.options.in_static && self.peek_keyword(Keyword::In).is_ok())
            // spread
            || self.peek_is(TokenType::Spread)
            // pattern
            || self.peek_is(TokenType::OpenParenthesis)
            || self.peek_is(TokenType::OpenBracket)
            || self.peek_is(TokenType::OpenBrace)
        {
            let parameter = self.eat_parameter().for_node_type(NodeType::Parameter)?;
            parameters.push(parameter);
            self.eat_newlines_maybe()?;
            if self.is_item_stop() {
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
        let mark = self.mark();
        self.eat_newlines_maybe()?;
        if self.peek_is(TokenType::LessThan) {
            Ok(Some(self.eat_static_parameters()?))
        } else {
            self.rewind(mark);
            Ok(None)
        }
    }

    /// Eat static parameters (including the `<` and `>` tokens).
    pub fn eat_static_parameters(&mut self) -> ParseResult<Vec<LocalNodeId<Parameter>>> {
        let start = self.mark();
        self.eat_token(TokenType::LessThan)?;
        self.eat_newlines_maybe()?;

        // empty static parameters are not allowed
        if self.peek_is(TokenType::GreaterThan) {
            return Err(ParseError::expected(
                self.get_span_from(start),
                TokenType::Identifier,
            ));
        }

        // regular static parameters
        let mut options = self.options.nested().in_static();
        if self.options.in_type || self.language.is_typescript() {
            options = options.in_type();
        }
        let parameters = self.with_options(options, |parser| parser.eat_parameters_body())?;
        self.eat_token(TokenType::GreaterThan)?;
        Ok(parameters)
    }

    /// Eat dynamic parameters (including the `(` and `)` tokens) if they exist.
    pub fn eat_dynamic_parameters_maybe(
        &mut self,
    ) -> ParseResult<Option<Vec<LocalNodeId<Parameter>>>> {
        if self.peek_is(TokenType::OpenParenthesis) {
            return Ok(Some(self.eat_dynamic_parameters()?));
        }
        Ok(None)
    }

    /// Eat dynamic parameters (including the `(` and `)` tokens).
    pub fn eat_dynamic_parameters(&mut self) -> ParseResult<Vec<LocalNodeId<Parameter>>> {
        self.eat_token(TokenType::OpenParenthesis)?;
        self.eat_newlines_maybe()?;

        // empty dynamic parameters
        if self.peek_is(TokenType::CloseParenthesis) {
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
        if self.peek_is(TokenType::Spread) {
            self.bump(); // eat spread
            let (label, label_span, value) = if self.options.in_type
                && self.peek_is(TokenType::Identifier)
                && (self.peek_next_is(TokenType::Colon)
                    || self.peek_next_is(TokenType::Maybe)
                        && self.peek_next_next_token(TokenType::Colon).is_ok())
            {
                let (label, label_span) = self.eat_identifier_with_span()?;
                if self.peek_is(TokenType::Maybe) {
                    self.bump(); // eat ?
                    if modifiers.is_none() {
                        modifiers = Some(BindingModifier::default());
                    }
                    modifiers.as_mut().unwrap().kind = Some(BindingKind::Maybe);
                }
                self.bump(); // eat colon
                let value = self.eat_expression()?;
                (Some(label), Some(label_span), value)
            } else {
                let value = self.with_options(
                    self.options.not_in_position().not_in_sequence_expression(),
                    |parser| parser.eat_expression(),
                )?;
                (None, None, value)
            };
            let argument_id = self.tree.insert(
                Argument::Spread {
                    modifiers,
                    label,
                    value,
                },
                self.get_span_from(start),
            );
            if let Some(label_span) = label_span {
                self.tree.set_main_span(argument_id, label_span);
            }
            Ok(argument_id)
        }
        // labeled tuple element (only in type context): label: type
        else if self.options.in_type
            && self.peek_is(TokenType::Identifier)
            && (self.peek_next_is(TokenType::Colon)
                || self.peek_next_is(TokenType::Maybe)
                    && self.peek_next_next_token(TokenType::Colon).is_ok())
        {
            let (label, label_span) = self.eat_identifier_with_span()?;
            if self.peek_is(TokenType::Maybe) {
                self.bump(); // eat ?
                if modifiers.is_none() {
                    modifiers = Some(BindingModifier::default());
                }
                modifiers.as_mut().unwrap().kind = Some(BindingKind::Maybe);
            }
            self.bump(); // eat colon
            let value = self.with_options(self.options.not_in_sequence_expression(), |parser| {
                parser.eat_expression()
            })?;
            let argument_id = self.tree.insert(
                Argument::Labeled {
                    modifiers,
                    label,
                    value,
                },
                self.get_span_from(start),
            );
            self.tree.set_main_span(argument_id, label_span);
            Ok(argument_id)
        }
        // positional argument
        else {
            let mut value = self
                .with_options(self.options.not_in_sequence_expression(), |parser| {
                    parser.eat_expression()
                })?;
            if self.options.in_type && self.peek_is(TokenType::Maybe) {
                let is_tuple_optional = self
                    .peek_token_after_newlines(self.pos(), TokenType::Comma)
                    .is_ok()
                    || self
                        .peek_token_after_newlines(self.pos(), TokenType::CloseBracket)
                        .is_ok();
                if is_tuple_optional {
                    self.bump(); // eat ?
                    if modifiers.is_none() {
                        modifiers = Some(BindingModifier::default());
                    }
                    modifiers.as_mut().unwrap().kind = Some(BindingKind::Maybe);
                }
            }
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
        if self.peek_name().is_ok() && self.peek_next_is(TokenType::Colon) {
            let (name, name_span) = self
                .eat_name_with_span()
                .for_node_type(NodeType::Argument)?;
            self.bump(); // eat colon
            self.eat_newlines_maybe()?;
            // value
            let value = self.with_options(
                self.options.not_in_position().not_in_sequence_expression(),
                |parser| parser.eat_expression(),
            )?;
            let argument_id = self.tree.insert(
                Argument::Named {
                    modifiers: None,
                    name,
                    value,
                },
                self.get_span_from(start),
            );
            self.tree.set_main_span(argument_id, name_span);
            Ok(argument_id)
        }
        // spread argument (...expr)
        else if self.peek_is(TokenType::Spread) {
            self.bump(); // eat spread
            let value = self.with_options(
                self.options.not_in_position().not_in_sequence_expression(),
                |parser| parser.eat_expression(),
            )?;
            let argument_id = self.tree.insert(
                Argument::Spread {
                    modifiers: None,
                    label: None,
                    value,
                },
                self.get_span_from(start),
            );
            Ok(argument_id)
        }
        // expression container ({expr}) - TSX syntax where {} are delimiters, not part of expr
        else if self.peek_is(TokenType::OpenBrace) {
            self.bump(); // eat {
            self.eat_newlines_maybe()?;

            // empty container (including comment-only containers)
            if self.peek_is(TokenType::CloseBrace) {
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

            // spread child: {...expr}
            if self.peek_is(TokenType::Spread) {
                self.bump(); // eat spread
                let value = self.with_options(
                    self.options
                        .not_in_position()
                        .not_in_tree_literal()
                        .not_in_left_precedence()
                        .not_in_sequence_expression(),
                    |parser| parser.eat_expression(),
                )?;
                self.eat_newlines_maybe()?;
                self.eat_token(TokenType::CloseBrace)?;
                let argument_id = self.tree.insert(
                    Argument::Spread {
                        modifiers: None,
                        label: None,
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
                    .not_in_left_precedence()
                    .not_in_sequence_expression(),
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
            // jsx content without braces must be text or nested tags
            if self.language.supports_jsx() {
                let token = self.peek()?;
                let is_tree_text = token.token.ty == TokenType::Literal
                    && matches!(
                        token.token.literal,
                        Some(LiteralType::TreeString)
                            | Some(LiteralType::Character {
                                is_html_entity: true,
                                ..
                            })
                    );
                let is_tree_literal =
                    token.token.ty == TokenType::LessThan && self.peek_tree_literal().is_ok();
                if !is_tree_text && !is_tree_literal {
                    return Err(ParseError::unexpected(token.span));
                }
            }
            let value = self.with_options(
                self.options.not_in_position().not_in_sequence_expression(),
                |parser| parser.eat_expression(),
            )?;
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
        if self.peek_is(TokenType::Spread) {
            self.bump(); // eat spread
            let value = self.with_options(self.options.in_statement_position(), |parser| {
                parser.eat_expression()
            })?;
            let argument_id = self.tree.insert(
                Argument::Spread {
                    modifiers: None,
                    label: None,
                    value,
                },
                self.get_span_from(start),
            );
            Ok(argument_id)
        }
        // spread expression container (like {...expr} in tree literals for #Compatibility)
        else if self.peek_is(TokenType::OpenBrace) && self.peek_next_is(TokenType::Spread) {
            self.bump(); // eat open brace
            self.bump(); // eat spread
            self.eat_newlines_maybe()?;
            let value = self.with_options(
                self.options
                    .not_in_position()
                    .not_in_tree_literal()
                    .not_in_left_precedence()
                    .not_in_sequence_expression(),
                |parser| parser.eat_expression(),
            )?;
            self.eat_newlines_maybe()?;
            self.eat_token(TokenType::CloseBrace)?;
            let argument_id = self.tree.insert(
                Argument::Spread {
                    modifiers: None,
                    label: None,
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
            let value = if self.peek_is(TokenType::Colon) || self.peek_is(TokenType::Assign) {
                self.bump(); // eat colon or assign
                self.eat_newlines_maybe()?;
                // tsx expression container: attr={expr}
                if self.peek_is(TokenType::OpenBrace) {
                    self.bump(); // eat {
                    self.eat_newlines_maybe()?;
                    let value = self.with_options(
                        self.options
                            .not_in_position()
                            .not_in_tree_literal()
                            .not_in_left_precedence()
                            .not_in_sequence_expression(),
                        |parser| parser.eat_expression(),
                    )?;
                    self.eat_newlines_maybe()?;
                    self.eat_token(TokenType::CloseBrace)?;
                    value
                }
                // string literal attribute
                else if self.peek_string_literal().is_ok() {
                    let (string, span) = self.eat_string_literal_with_span()?;
                    self.tree.insert(
                        Expression::ScalarLiteral(ScalarLiteral::String(string)),
                        span,
                    )
                }
                // shorthand array attribute
                else if self.peek_is(TokenType::OpenBracket) {
                    let value_start = self.mark();
                    let elements = self.with_options(
                        self.options.not_in_position().not_in_tree_literal(),
                        |parser| parser.eat_array_literal(),
                    )?;
                    self.tree.insert(
                        Expression::ArrayExpression { elements },
                        self.get_span_from(value_start),
                    )
                }
                // unexpected attribute value
                else {
                    return Err(ParseError::unexpected(self.peek()?.span));
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
    /// Also handles `<<` (ShiftLeft) for patterns like `Extends<<T>() => ...>`.
    pub fn eat_static_arguments_maybe(
        &mut self,
    ) -> ParseResult<Option<Vec<LocalNodeId<Argument>>>> {
        if self.peek_is(TokenType::LessThan) || self.peek_is(TokenType::ShiftLeft) {
            return Ok(Some(self.eat_static_arguments()?));
        }
        Ok(None)
    }

    /// Eat static arguments (including the `<` and `>` tokens).
    /// Only positional and spread arguments are allowed (no named arguments).
    /// Also handles `<<` (ShiftLeft) for patterns like `Extends<<T>() => ...>`.
    pub fn eat_static_arguments(&mut self) -> ParseResult<Vec<LocalNodeId<Argument>>> {
        let _timing = self.timing_scope(tags::PARSE_ARGUMENT);
        let start = self.mark();

        // handle both `<` and `<<` (ShiftLeft) as opening token
        // `<<` occurs when the first argument is a generic arrow function like `<T>() => ...`
        if self.peek_is(TokenType::LessThan) {
            self.bump(); // eat `<`
        } else if self.peek_is(TokenType::ShiftLeft) {
            // split `<<` into `<` (consumed) + `<` (pending as split token)
            // the pending `<` will be seen by the first argument as its generic opening
            self.split_shift_left();
        } else {
            return Err(ParseError::expected(self.peek()?.span, TokenType::LessThan));
        }
        self.eat_newlines_maybe()?;

        // empty static arguments are not allowed
        if self.peek_is(TokenType::GreaterThan) {
            return Err(ParseError::expected(
                self.get_span_from(start),
                TokenType::Identifier,
            ));
        }

        // regular static arguments (positional/spread only)
        let mut options = self.options.nested().in_static();
        if self.options.in_type || self.language.is_typescript() {
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
        if self.peek_is(TokenType::OpenParenthesis) {
            return Ok(Some(self.eat_dynamic_arguments()?));
        }
        Ok(None)
    }

    /// Eat dynamic arguments (including the `(` and `)` tokens).
    /// Only positional and spread arguments are allowed (no named arguments).
    pub fn eat_dynamic_arguments(&mut self) -> ParseResult<Vec<LocalNodeId<Argument>>> {
        let _timing = self.timing_scope(tags::PARSE_ARGUMENT);
        self.eat_token(TokenType::OpenParenthesis)?;
        self.eat_newlines_maybe()?;

        // empty dynamic arguments
        if self.peek_is(TokenType::CloseParenthesis) {
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
        while self.has_more_tokens() {
            if self.peek_token_type() == terminator {
                break;
            }
            let argument_id = self.eat_positional_argument()?;
            arguments.push(argument_id);
            self.eat_newlines_maybe()?;
            if self.is_item_stop() {
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
        while self.has_more_tokens() {
            if self.peek_token_type() == terminator {
                break;
            }
            let argument_id = self.eat_tree_argument()?;
            arguments.push(argument_id);
            self.eat_newlines_maybe()?;
            if self.is_item_stop() {
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
    fn test_parse_parameter_readonly_name() {
        // readonly: int32
        let mut test = TestParser::new("readonly: int32");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Named { modifiers, name, ty, default: None } => {
            assert!(modifiers.is_none());
            assert_string!(parser, *name, "readonly");
            let ty = ty.expect("expected type annotation");
            assert_node!(parser.tree, ty, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary {
                width: Some(32),
                is_signed: true
            })));
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

        let main_span = parser
            .tree
            .get_main_span(argument_id)
            .expect("expected argument name span");
        assert_eq!(parser.get_span_str(main_span), "x");
    }

    #[test]
    fn test_parse_named_argument_string_span() {
        let mut test = TestParser::new("\"Content-Type\": 1");
        let mut parser = test.prepare();
        let argument_id = parser.eat_tree_argument().unwrap();
        assert_node!(parser.tree, argument_id, Argument::Named { modifiers: _, name: Name::String(name), value } => {
            assert_string!(parser, *name, "Content-Type");
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });

        let main_span = parser
            .tree
            .get_main_span(argument_id)
            .expect("expected argument name span");
        assert_eq!(parser.get_span_str(main_span), "\"Content-Type\"");
    }

    #[test]
    fn test_parse_named_argument_string_literal_value() {
        let mut test = TestParser::new("title=\"hello\"");
        let mut parser = test.prepare();
        let argument_id = parser.eat_tree_literal_argument().unwrap();
        assert_node!(parser.tree, argument_id, Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "title");
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string)) => {
                assert_string!(parser, *string, "hello");
            });
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
        assert_node!(parser.tree, argument_id, Argument::Spread { modifiers: _, label, value } => {
            // ...args
            assert!(label.is_none());
            assert_node!(parser.tree, *value, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "args");
            });
        });
    }

    #[test]
    fn test_parse_spread_tuple_label_argument() {
        // ...args: number
        let mut test = TestParser::new("...args: number");
        let mut parser = test.prepare();
        parser.options.in_type = true;
        let argument_id = parser.eat_positional_argument().unwrap();
        assert_node!(parser.tree, argument_id, Argument::Spread { modifiers: _, label, value } => {
            assert_string!(parser, label.unwrap(), "args");
            assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Number));
        });

        let main_span = parser
            .tree
            .get_main_span(argument_id)
            .expect("expected spread label span");
        assert_eq!(parser.get_span_str(main_span), "args");
    }

    #[test]
    fn test_parse_tuple_label_argument_span() {
        let mut test = TestParser::new("label: number");
        let mut parser = test.prepare();
        parser.options.in_type = true;
        let argument_id = parser.eat_positional_argument().unwrap();
        assert_node!(parser.tree, argument_id, Argument::Labeled { modifiers: _, label, value } => {
            assert_string!(parser, *label, "label");
            assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Number));
        });

        let main_span = parser
            .tree
            .get_main_span(argument_id)
            .expect("expected label span");
        assert_eq!(parser.get_span_str(main_span), "label");
    }
}
