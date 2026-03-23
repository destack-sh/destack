use destack_ast::{
    AbstractionModifier, AccessorKind, Argument, BindingAnchor, BindingKind, BindingModifier,
    BindingOperator, DeclarationKind, Expression, Keyword, LiteralType, LocalNodeId, Mutability,
    Name, NodeType, Parameter, Pattern, PostfixPosition, ScalarLiteral, StringId, Timing,
    TokenType, VarianceModifier,
};
use destack_source::{NodeSpanType, Span};

use crate::parse::parser::ParserOptions;
use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser};

/// Parsed parameter head as either one pattern or one named binding.
type ParsedParameterPatternOrName = (
    Option<LocalNodeId<Pattern>>,
    Option<StringId>,
    Option<destack_source::Span>,
);

impl Parser {
    /// Return parser contexts for pattern-shaped parameters.
    #[inline]
    fn parameter_pattern_contexts(&self) -> (ParserOptions, ParserOptions) {
        let ambient_context = self.options.with_before_type(true);
        let expression_context = self.options;
        (ambient_context, expression_context)
    }

    /// Return parser contexts for static parameter lists.
    #[inline]
    fn static_parameter_contexts(&self) -> (ParserOptions, ParserOptions) {
        let mut ambient_context = self.options.nested().with_static(true);
        if self.options.is_in_type()
            || self.options.is_in_decorator()
            || self.language.is_typescript()
        {
            ambient_context = ambient_context.with_type(true);
        }
        let expression_context = self.options.nested();
        (ambient_context, expression_context)
    }

    /// Return parser contexts for dynamic parameter lists.
    #[inline]
    fn dynamic_parameter_contexts(&self) -> (ParserOptions, ParserOptions) {
        let mut ambient_context = self.options.nested();
        if self.options.is_in_type() {
            ambient_context = ambient_context.with_type(true);
        }
        if self.options.is_in_variant() {
            ambient_context = ambient_context.with_variant(true);
        }
        let expression_context = self.options.nested();
        (ambient_context, expression_context)
    }

    /// Return parser contexts for static type arguments.
    #[inline]
    fn static_argument_contexts(&self) -> (ParserOptions, ParserOptions) {
        let mut ambient_context = self.options.nested().with_static(true);
        if self.options.is_in_type()
            || self.options.is_in_decorator()
            || self.language.is_destack()
            || self.language.is_typescript()
        {
            ambient_context = ambient_context.with_type(true);
        }
        let expression_context = self.options.nested();
        (ambient_context, expression_context)
    }

    /// Return parser contexts for dynamic arguments.
    #[inline]
    fn dynamic_argument_contexts(&self) -> (ParserOptions, ParserOptions) {
        let ambient_context = self.options.nested();
        let expression_context = self.options.nested();
        (ambient_context, expression_context)
    }

    /// Return the common context for non-sequence argument values.
    #[inline]
    fn current_non_sequence_argument_context(&self) -> ParserOptions {
        self.options
            .not_in_sequence_expression()
            .not_in_arrow_return_type()
    }

    /// Return the common context for positional argument values.
    #[inline]
    fn current_positional_argument_context(&self) -> ParserOptions {
        self.current_non_sequence_argument_context()
            .not_in_position()
    }

    /// Parse a parameter type annotation expression.
    #[inline]
    fn eat_parameter_type_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let ambient_context = self.options.with_type(true);
        let mut expression_context = self.options.not_in_position().not_in_left_precedence();
        if self.options.is_in_type_conditional_right() {
            expression_context = expression_context.in_type_conditional_right();
        }
        self.eat_expression(
            self.options
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
        )
    }

    /// Parse a parameter default value expression.
    #[inline]
    fn eat_parameter_default_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let ambient_context = self
            .options
            .with_type(self.options.is_in_static())
            .with_forbid_await(true)
            .with_variant(false);
        let expression_context = self.current_positional_argument_context();
        self.eat_expression(
            self.options
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
        )
    }

    /// Return true when the next token can start a member name.
    pub(crate) fn next_token_starts_member_name(&mut self) -> bool {
        // skip newlines after the modifier keyword
        let mut pos = self.pos() as usize;
        loop {
            self.ensure_token(pos + 1);
            let Some(token) = self.tokens().get(pos + 1) else {
                break;
            };
            if token.token.ty != TokenType::Newline {
                break;
            }
            pos += 1;
        }
        let Some(next_token) = self.tokens().get(pos + 1) else {
            return false;
        };

        // check for common member name starters
        if matches!(
            next_token.token.ty,
            TokenType::Identifier
                | TokenType::Hash
                | TokenType::OpenBracket
                | TokenType::OpenBrace
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
            Some(LiteralType::Boolean { .. }) => true,
            Some(LiteralType::Int { .. }) | Some(LiteralType::Float { .. }) => true,
            _ => false,
        }
    }

    /// Return true when `abstract` and `override` can be parsed as modifiers.
    fn can_parse_abstraction_modifier(&mut self) -> bool {
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
        let mut seen_variance_in = false;
        let mut seen_variance_out = false;
        let mut seen_accessor = false;

        // eat modifiers in any order
        loop {
            // fast path: modifiers only start on identifiers and known modifier keywords
            if !self.peek_is(TokenType::Identifier) {
                break;
            }
            let position_index = self.pos_index();
            let has_active_split = self.has_active_split();
            let current_keyword = if has_active_split {
                self.peek_any_keyword().ok()
            } else {
                self.keyword_for_index(position_index)
            };
            let is_out_variance_modifier = allow_variance_modifier
                && !has_active_split
                && self.identifier_equals_at(position_index, "out")
                && (self.peek_next_is(TokenType::Identifier) || self.is_next_keyword(Keyword::In));
            let can_start_modifier = current_keyword.is_some_and(|keyword| {
                matches!(
                    keyword,
                    Keyword::In
                        | Keyword::Public
                        | Keyword::Protected
                        | Keyword::Private
                        | Keyword::Declare
                        | Keyword::Static
                        | Keyword::Abstract
                        | Keyword::Override
                        | Keyword::Readonly
                        | Keyword::Const
                        | Keyword::Accessor
                        | Keyword::Comptime
                )
            }) || is_out_variance_modifier;
            if !can_start_modifier {
                break;
            }

            let mut progress = false;

            // modifier disambiguation for abstraction keywords
            let abstraction_is_modifier = self.can_parse_abstraction_modifier();

            // variance for static parameters
            if allow_variance_modifier {
                // handle 'in' variance modifier
                if current_keyword == Some(Keyword::In) {
                    let span = self.peek()?.span;
                    self.bump(); // eat in
                    if validate_modifier_order && seen_variance_in {
                        self.error(&ParseError::unexpected(span));
                    }
                    if validate_modifier_order && seen_variance_out {
                        self.error(&ParseError::unexpected(span));
                    }
                    modifiers.variance = Some(match modifiers.variance {
                        Some(VarianceModifier::Out) => VarianceModifier::InOut,
                        Some(VarianceModifier::InOut) => VarianceModifier::InOut,
                        _ => VarianceModifier::In,
                    });
                    seen_variance_in = true;
                    has_modifiers = true;
                    progress = true;
                }
                // handle 'out' variance modifier
                else if is_out_variance_modifier {
                    let span = self.peek()?.span;
                    self.bump(); // eat out
                    if validate_modifier_order && seen_variance_out {
                        self.error(&ParseError::unexpected(span));
                    }
                    modifiers.variance = Some(match modifiers.variance {
                        Some(VarianceModifier::In) => VarianceModifier::InOut,
                        Some(VarianceModifier::InOut) => VarianceModifier::InOut,
                        _ => VarianceModifier::Out,
                    });
                    seen_variance_out = true;
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
            if allow_declare_modifier && self.is_keyword(Keyword::Declare) {
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
            if modifiers.anchor.is_none() && self.is_keyword(Keyword::Static) {
                if !self.next_token_starts_member_name() {
                    break;
                }
                let span = self.peek()?.span;
                self.bump(); // eat static
                modifiers.anchor = Some(BindingAnchor::Static);
                if validate_modifier_order && seen_override {
                    self.error(&ParseError::unexpected(span));
                }
                if validate_modifier_order && seen_accessor {
                    self.error(&ParseError::unexpected(span));
                }
                seen_static = true;
                has_modifiers = true;
                progress = true;
            }

            // abstraction modifiers (abstract)
            if self.is_keyword(Keyword::Abstract) && abstraction_is_modifier {
                if !self.next_token_starts_member_name() {
                    break;
                }
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
            if self.is_keyword(Keyword::Override) && abstraction_is_modifier {
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
                self.next_token_starts_member_name()
            } else {
                true
            };
            if modifiers.mutability.is_none()
                && self.is_keyword(Keyword::Readonly)
                && readonly_is_modifier
            {
                self.bump(); // eat readonly
                modifiers.mutability = Some(Mutability::Immutable);
                seen_readonly = true;
                has_modifiers = true;
                progress = true;
            }

            // operator modifiers (const)
            if modifiers.operator.is_none() && self.is_keyword(Keyword::Const) {
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
                && self.is_keyword(Keyword::Accessor)
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
                seen_accessor = true;
                has_modifiers = true;
                progress = true;
            }

            // timing modifiers (comptime)
            if modifiers.timing.is_none() && self.is_keyword(Keyword::Comptime) {
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
        // collect runtime decorators so validation can check placement
        let decorators = if !self.options.is_in_type() {
            self.eat_decorators_maybe()?
        } else {
            smallvec::SmallVec::new()
        };

        let start = self.mark_span();

        let mut modifiers = self.eat_binding_modifiers_prefix_maybe(
            true,
            false,
            self.options.is_in_static(),
            false,
            true,
        )?;

        // variadic
        let is_variadic = if self.peek_is(TokenType::Spread) {
            self.bump(); // eat range or range wide
            true
        } else {
            false
        };

        // pattern/name
        let (pattern, name, name_span) =
            if is_variadic && (self.options.is_in_type() || self.options.is_in_variant()) {
                // variadic tuple labels in type positions: ...[name]: T
                if let Some((name, span)) = self.try_eat_variadic_tuple_label_name_with_span()? {
                    (None, Some(name), Some(span))
                } else {
                    self.eat_parameter_pattern_or_name()?
                }
            } else {
                self.eat_parameter_pattern_or_name()?
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
            // type markers can start on the next line
            let annotation_index = self.next_non_newline_index_from(self.pos_index());
            let annotation_token_type = self.token_type_at(annotation_index);
            let annotation_keyword = if annotation_token_type == TokenType::Identifier {
                self.keyword_for_index(annotation_index)
            } else {
                None
            };
            let has_type_annotation_marker = annotation_token_type == TokenType::Colon
                || self.options.is_in_static()
                    && matches!(
                        annotation_keyword,
                        Some(Keyword::Extends | Keyword::Implements)
                    );

            if has_type_annotation_marker {
                let type_start = self.mark_span();
                self.eat_newlines_maybe()?;
                self.bump(); // eat colon or keyword
                self.eat_newlines_maybe()?;
                let ty = self
                    .eat_parameter_type_expression()
                    .for_node_type(NodeType::Parameter)?;
                (Some(ty), Some(self.get_span_from(&type_start)))
            } else {
                (None, None)
            }
        };

        // = value
        let parameter = {
            let has_default_assign = self.peek_is(TokenType::Assign)
                || self.peek_is(TokenType::Newline)
                    && self.is_token_after_newlines(self.pos(), TokenType::Assign);
            if !is_variadic && has_default_assign {
                self.eat_newlines_maybe()?;
                self.bump(); // eat assign
                self.eat_newlines_maybe()?;

                // value
                let value = self
                    .eat_parameter_default_expression()
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
                if let Some(name) = name {
                    Parameter::VariadicNamed {
                        modifiers,
                        name,
                        ty,
                    }
                } else {
                    Parameter::VariadicPattern {
                        modifiers,
                        pattern: pattern.expect("peeked"),
                        ty,
                    }
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
        let parameter_id = self.insert_node(parameter, self.get_span_from(&start));

        // set main span to the name identifier
        if let Some(span) = name_span {
            self.tree.set_main_span(parameter_id, span);
        }

        // set type span for the type annotation
        if let Some(span) = ty_span {
            self.tree
                .set_side_span(parameter_id, NodeSpanType::Type, span);
        }

        // attach parsed decorators to the parameter node
        self.attach_decorators(parameter_id.id, decorators);

        // attach own-line parameter prefixes with block semantics first

        Ok(parameter_id)
    }

    /// Eat either a parameter pattern or a parameter name.
    fn eat_parameter_pattern_or_name(&mut self) -> ParseResult<ParsedParameterPatternOrName> {
        // pattern
        if matches!(
            self.peek_token_type(),
            TokenType::OpenParenthesis | TokenType::OpenBracket | TokenType::OpenBrace
        ) || self.language.is_destack() && self.peek_identifier_str_is("_")
        {
            let (ambient_context, expression_context) = self.parameter_pattern_contexts();
            let pattern = self.with_options(
                self.options
                    .with_ambient_context(ambient_context)
                    .with_expression_context(expression_context),
                |parser| parser.eat_pattern(),
            )?;
            return Ok((Some(pattern), None, None));
        }

        // name
        let (name, span) = self.eat_binding_identifier_with_span()?;
        Ok((None, Some(name), Some(span)))
    }

    /// Try to eat a variadic tuple label name in the shape `...[name]: T`.
    fn try_eat_variadic_tuple_label_name_with_span(
        &mut self,
    ) -> ParseResult<Option<(StringId, destack_source::Span)>> {
        if !self.peek_is(TokenType::OpenBracket) {
            return Ok(None);
        }

        // reject quickly when the tuple label head shape does not match `[name]:`
        let identifier_index = self.next_non_newline_index_from(self.pos_index().saturating_add(1));
        if self.token_type_at(identifier_index) != TokenType::Identifier {
            return Ok(None);
        }
        let close_bracket_index =
            self.next_non_newline_index_from(identifier_index.saturating_add(1));
        if self.token_type_at(close_bracket_index) != TokenType::CloseBracket {
            return Ok(None);
        }
        let colon_index = self.next_non_newline_index_from(close_bracket_index.saturating_add(1));
        if self.token_type_at(colon_index) != TokenType::Colon {
            return Ok(None);
        }

        self.bump(); // eat [
        self.eat_newlines_maybe()?;
        let (name, span) = self.eat_binding_identifier_with_span()?;
        self.eat_newlines_maybe()?;
        self.bump(); // eat ]
        self.eat_newlines_maybe()?;

        Ok(Some((name, span)))
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
        let in_js = self.language.is_javascript();
        let mut has_variadic_parameter = false;
        self.eat_newlines_maybe()?;
        while self.peek_is(TokenType::Identifier)
            || (self.options.is_in_static() && self.is_keyword(Keyword::In))
            // spread
            || self.peek_is(TokenType::Spread)
            // pattern
            || self.peek_is(TokenType::OpenParenthesis)
            || self.peek_is(TokenType::OpenBracket)
            || self.peek_is(TokenType::OpenBrace)
            // decorator
            || (!self.options.is_in_type() && self.peek_is(TokenType::At))
        {
            let parameter = self.eat_parameter().for_node_type(NodeType::Parameter)?;

            // rest parameters must be terminal in js, ts compatibility fixtures may allow more
            if has_variadic_parameter && in_js {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            if matches!(
                self.tree.get(parameter),
                Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. }
            ) {
                has_variadic_parameter = true;
            }

            parameters.push(parameter);
            self.eat_newlines_maybe()?;

            // in js: trailing separators after rest parameters are invalid
            if in_js && has_variadic_parameter && self.is_item_stop() {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            if self.is_item_stop() {
                self.eat_item_stop_with_newlines()?;
            } else {
                break;
            }
        }
        Ok(parameters)
    }

    /// Eat static parameters (including the `<` and `>` tokens) if they exist.
    ///
    /// `allow_empty_parameters` accepts `<>` as an empty parameter list.
    pub fn eat_static_parameters_maybe(
        &mut self,
        allow_empty_parameters: bool,
    ) -> ParseResult<Option<Vec<LocalNodeId<Parameter>>>> {
        if self.has_active_split() {
            let mark = self.mark_rewind();
            self.eat_newlines_maybe()?;
            if self.peek_is(TokenType::LessThan) {
                return Ok(Some(self.eat_static_parameters(allow_empty_parameters)?));
            }
            self.rewind(mark);
            return Ok(None);
        }

        let start_index = self.pos_index();
        let static_index = self.next_non_newline_index_from(start_index);
        if self.token_type_at(static_index) != TokenType::LessThan {
            return Ok(None);
        }

        if static_index != start_index {
            self.eat_newlines_maybe()?;
        }

        Ok(Some(self.eat_static_parameters(allow_empty_parameters)?))
    }

    /// Eat static parameters (including the `<` and `>` tokens).
    ///
    /// `allow_empty_parameters` accepts `<>` as an empty parameter list.
    pub fn eat_static_parameters(
        &mut self,
        allow_empty_parameters: bool,
    ) -> ParseResult<Vec<LocalNodeId<Parameter>>> {
        let start = self.mark_span();
        self.eat_token(TokenType::LessThan)?;
        self.eat_newlines_maybe()?;

        // optionally accept empty static parameters
        if self.peek_is(TokenType::GreaterThan) {
            self.bump();
            if allow_empty_parameters {
                return Ok(vec![]);
            }
            return Err(ParseError::expected(
                self.get_span_from(&start),
                TokenType::Identifier,
            ));
        }

        // regular static parameters
        let (ambient_context, expression_context) = self.static_parameter_contexts();
        let parameters = self.with_options(
            self.options
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
            |parser| parser.eat_parameters_body(),
        )?;
        self.eat_type_angle_close()?;
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
        let (ambient_context, expression_context) = self.dynamic_parameter_contexts();
        let parameters = self.with_options(
            self.options
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
            |parser| parser.eat_parameters_body(),
        )?;
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
        // hot path: plain positional value arguments in value contexts
        if !self.options.is_in_type()
            && !self.peek_is(TokenType::At)
            && !self.peek_is(TokenType::Spread)
        {
            let value = self.eat_expression_with_context_unchecked(
                self.current_positional_argument_context(),
            )?;
            let value_span = self.tree.get_span(value);
            let argument_id = self.insert_node(
                Argument::Positional {
                    modifiers: None,
                    value,
                },
                value_span,
            );
            return Ok(argument_id);
        }

        let start = self.mark_span();
        let mut modifiers = None;
        let decorators = if !self.options.is_in_type() {
            self.eat_decorators_maybe()?
        } else {
            smallvec::SmallVec::new()
        };

        // readonly tuple element modifiers in type context
        if self.options.is_in_type() && self.is_keyword(Keyword::Readonly) {
            self.bump(); // eat readonly
            modifiers = Some(BindingModifier {
                mutability: Some(Mutability::Immutable),
                ..BindingModifier::default()
            });
        }

        // detect labeled tuple element heads like `label: Type` and `label?: Type`
        let has_labeled_tuple_head = self.options.is_in_type()
            && self.peek_is(TokenType::Identifier)
            && (self.peek_next_is(TokenType::Colon)
                || self.peek_next_is(TokenType::Maybe) && self.peek_next_next_is(TokenType::Colon));

        // spread argument
        if self.peek_is(TokenType::Spread) {
            self.bump(); // eat spread

            // detect spread labels like `...label: Type` and `...label?: Type`
            let has_spread_labeled_tuple_head = self.options.is_in_type()
                && self.peek_is(TokenType::Identifier)
                && (self.peek_next_is(TokenType::Colon)
                    || self.peek_next_is(TokenType::Maybe)
                        && self.peek_next_next_is(TokenType::Colon));

            // spread label and value
            let (label, label_span, value) = if has_spread_labeled_tuple_head {
                let (label, label_span) = self.eat_identifier_with_span()?;

                // spread optional label marker
                if self.peek_is(TokenType::Maybe) {
                    self.bump(); // eat ?
                    if modifiers.is_none() {
                        modifiers = Some(BindingModifier::default());
                    }
                    modifiers.as_mut().unwrap().kind = Some(BindingKind::Maybe);
                }

                // spread labeled value
                self.bump(); // eat colon
                self.eat_newlines_maybe()?;
                let value = self.eat_expression_with_context_unchecked(
                    self.current_positional_argument_context(),
                )?;
                (Some(label), Some(label_span), value)
            } else {
                // spread positional value
                self.eat_newlines_maybe()?;
                let value = self.eat_expression_with_context_unchecked(
                    self.current_positional_argument_context(),
                )?;
                (None, None, value)
            };

            // build spread argument
            let argument_id = self.insert_node(
                Argument::Spread {
                    modifiers,
                    label,
                    value,
                },
                self.get_span_from(&start),
            );
            if let Some(label_span) = label_span {
                self.tree.set_main_span(argument_id, label_span);
            }
            self.attach_decorators(argument_id.id, decorators);
            return Ok(argument_id);
        }

        // labeled tuple element (only in type context)
        if has_labeled_tuple_head {
            let (label, label_span) = self.eat_identifier_with_span()?;

            // optional tuple label marker
            if self.peek_is(TokenType::Maybe) {
                self.bump(); // eat ?
                if modifiers.is_none() {
                    modifiers = Some(BindingModifier::default());
                }
                modifiers.as_mut().unwrap().kind = Some(BindingKind::Maybe);
            }

            // parse labeled tuple value
            self.bump(); // eat colon
            self.eat_newlines_maybe()?;
            let value = self.eat_expression_with_context_unchecked(
                self.current_non_sequence_argument_context(),
            )?;

            // build labeled argument
            let argument_id = self.insert_node(
                Argument::Labeled {
                    modifiers,
                    label,
                    value,
                },
                self.get_span_from(&start),
            );
            self.tree.set_main_span(argument_id, label_span);
            self.attach_decorators(argument_id.id, decorators);
            return Ok(argument_id);
        }

        // positional value expression
        let mut value =
            self.eat_expression_with_context_unchecked(self.current_positional_argument_context())?;

        // tuple optional marker after positional element
        if self.options.is_in_type() && self.peek_is(TokenType::Maybe) {
            let is_tuple_optional = self.is_token_after_newlines(self.pos(), TokenType::Comma)
                || self.is_token_after_newlines(self.pos(), TokenType::CloseBracket);
            if is_tuple_optional {
                self.bump(); // eat ?
                if modifiers.is_none() {
                    modifiers = Some(BindingModifier::default());
                }
                modifiers.as_mut().unwrap().kind = Some(BindingKind::Maybe);
            }
        }

        // normalize direct postfix maybe expressions into tuple modifiers
        if self.options.is_in_type()
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

        // build positional argument
        let argument_id = self.insert_node(
            Argument::Positional { modifiers, value },
            self.get_span_from(&start),
        );
        self.attach_decorators(argument_id.id, decorators);
        Ok(argument_id)
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
        let start = self.mark_span();
        // named argument (name: value)
        if self.peek_name_is() && self.peek_next_is(TokenType::Colon) {
            let (name, name_span) = self
                .eat_name_with_span()
                .for_node_type(NodeType::Argument)?;
            self.bump(); // eat colon
            self.eat_newlines_maybe()?;
            // value
            let value = self.eat_expression_with_context_unchecked(
                self.current_positional_argument_context(),
            )?;
            let argument_id = self.insert_node(
                Argument::Named {
                    modifiers: None,
                    name,
                    value,
                },
                self.get_span_from(&start),
            );
            self.tree.set_main_span(argument_id, name_span);
            Ok(argument_id)
        }
        // spread argument (...expr)
        else if self.peek_is(TokenType::Spread) {
            self.bump(); // eat spread
            let value = self.eat_expression_with_context_unchecked(
                self.current_positional_argument_context(),
            )?;
            let argument_id = self.insert_node(
                Argument::Spread {
                    modifiers: None,
                    label: None,
                    value,
                },
                self.get_span_from(&start),
            );
            Ok(argument_id)
        }
        // expression container ({expr}) - TSX syntax where {} are delimiters, not part of expr
        else if self.peek_is(TokenType::OpenBrace) {
            self.bump(); // eat {
            self.eat_newlines_maybe()?;

            // empty container (including comment-only containers)
            if self.peek_is(TokenType::CloseBrace) {
                let value = self
                    .tree
                    .insert(Expression::Stub, self.get_span_from(&start));

                self.bump(); // eat }
                let argument_id = self.insert_node(
                    Argument::Positional {
                        modifiers: None,
                        value,
                    },
                    self.get_span_from(&start),
                );
                return Ok(argument_id);
            }

            // spread child: {...expr}
            if self.peek_is(TokenType::Spread) {
                self.bump(); // eat spread
                let value_ambient_context = self.options.with_tree_literal(false);
                let value_expression_context = self
                    .options
                    .not_in_position()
                    .not_in_ternary_condition()
                    .not_in_left_precedence();
                let value = self.eat_expression(
                    self.options
                        .with_ambient_context(value_ambient_context)
                        .with_expression_context(value_expression_context),
                )?;

                self.eat_newlines_maybe()?;
                self.eat_token(TokenType::CloseBrace)?;
                let argument_id = self.insert_node(
                    Argument::Spread {
                        modifiers: None,
                        label: None,
                        value,
                    },
                    self.get_span_from(&start),
                );
                return Ok(argument_id);
            }

            let value_ambient_context = self.options.with_tree_literal(false);
            let value_expression_context = self
                .options
                .not_in_position()
                .not_in_ternary_condition()
                .not_in_left_precedence();
            let value = self.eat_expression(
                self.options
                    .with_ambient_context(value_ambient_context)
                    .with_expression_context(value_expression_context),
            )?;

            self.eat_newlines_maybe()?;
            self.eat_token(TokenType::CloseBrace)?;
            let argument_id = self.insert_node(
                Argument::Positional {
                    modifiers: None,
                    value,
                },
                self.get_span_from(&start),
            );
            Ok(argument_id)
        }
        // positional argument (bare expression like nested <Element />)
        else {
            // jsx content without braces must be text or nested tags
            if self.language.supports_jsx() && self.options.is_in_tree_literal() {
                let token = *self.peek()?;
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
            let value_expression_context =
                self.options.not_in_position().not_in_sequence_expression();
            let value = self.eat_expression_with_context_unchecked(value_expression_context)?;
            let argument_id = self.insert_node(
                Argument::Positional {
                    modifiers: None,
                    value,
                },
                self.get_span_from(&start),
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
        let start = self.mark_span();
        // spread argument
        if self.peek_is(TokenType::Spread) {
            self.bump(); // eat spread
            let value_expression_context = self.options.with_statement_position(true);
            let value = self.eat_expression_with_context_unchecked(value_expression_context)?;
            let argument_id = self.insert_node(
                Argument::Spread {
                    modifiers: None,
                    label: None,
                    value,
                },
                self.get_span_from(&start),
            );
            Ok(argument_id)
        }
        // spread expression container (like {...expr} in tree literals for #Compatibility)
        else if self.peek_tree_literal_spread_expression_container() {
            self.bump(); // eat open brace
            self.eat_newlines_maybe()?;
            self.bump(); // eat spread
            self.eat_newlines_maybe()?;
            let value_ambient_context = self.options.with_tree_literal(false);
            let value_expression_context = self
                .options
                .not_in_position()
                .not_in_ternary_condition()
                .not_in_left_precedence()
                .not_in_sequence_expression();
            let value = self.eat_expression(
                self.options
                    .with_ambient_context(value_ambient_context)
                    .with_expression_context(value_expression_context),
            )?;
            self.eat_newlines_maybe()?;
            self.eat_token(TokenType::CloseBrace)?;
            let argument_id = self.insert_node(
                Argument::Spread {
                    modifiers: None,
                    label: None,
                    value,
                },
                self.get_span_from(&start),
            );
            Ok(argument_id)
        }
        // named argument
        else {
            let name = self.eat_tree_literal_identifier()?;

            // explicit value separators, including newline wrapped forms
            let has_value_separator = self.peek_is(TokenType::Colon)
                || self.peek_is(TokenType::Assign)
                || self.peek_is(TokenType::Newline)
                    && (self.is_token_after_newlines(self.pos(), TokenType::Colon)
                        || self.is_token_after_newlines(self.pos(), TokenType::Assign));

            // named argument with value
            let value = if has_value_separator {
                self.eat_newlines_maybe()?;
                self.bump(); // eat colon or assign
                self.eat_newlines_maybe()?;

                // tsx expression container: attr={expr}
                if self.peek_is(TokenType::OpenBrace) {
                    self.bump(); // eat {
                    self.eat_newlines_maybe()?;
                    let value_ambient_context = self.options.with_tree_literal(false);
                    let value_expression_context = self
                        .options
                        .not_in_position()
                        .not_in_ternary_condition()
                        .not_in_left_precedence();
                    let value = self.eat_expression(
                        self.options
                            .with_ambient_context(value_ambient_context)
                            .with_expression_context(value_expression_context),
                    )?;
                    self.eat_newlines_maybe()?;
                    self.eat_token(TokenType::CloseBrace)?;
                    value
                }
                // string literal attribute
                else if self.peek_string_literal_is() {
                    let (string, span) = self.eat_string_literal_with_span()?;
                    self.insert_node(
                        Expression::ScalarLiteral(ScalarLiteral::String(string)),
                        span,
                    )
                }
                // shorthand array attribute
                else if self.peek_is(TokenType::OpenBracket) {
                    let value_start = self.mark_span();
                    let value_ambient_context = self.options.with_tree_literal(false);
                    let value_expression_context = self.options.not_in_position();
                    let elements = self.with_options(
                        self.options
                            .with_ambient_context(value_ambient_context)
                            .with_expression_context(value_expression_context),
                        |parser| parser.eat_array_literal(),
                    )?;
                    self.insert_node(
                        Expression::ArrayExpression { elements },
                        self.get_span_from(&value_start),
                    )
                }
                // tree literal attribute value
                else if self.language.supports_jsx() && self.peek_is(TokenType::LessThan) {
                    let value_ambient_context = self.options.with_tree_literal(true);
                    let value_expression_context = self.options.not_in_position();
                    self.with_options(
                        self.options
                            .with_ambient_context(value_ambient_context)
                            .with_expression_context(value_expression_context),
                        |parser| parser.eat_tree_literal(),
                    )?
                }
                // unexpected attribute value
                else {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }
            }
            // implicit boolean true
            else {
                self.insert_node(
                    Expression::ScalarLiteral(ScalarLiteral::Boolean(true)),
                    self.get_span_from(&start),
                )
            };

            let argument_id = self.insert_node(
                Argument::Named {
                    modifiers: None,
                    name: Name::Identifier(name),
                    value,
                },
                self.get_span_from(&start),
            );
            Ok(argument_id)
        }
    }

    /// Return true when the current tree literal argument starts with `{ ...`.
    fn peek_tree_literal_spread_expression_container(&mut self) -> bool {
        // must begin at an expression container
        if !self.peek_is(TokenType::OpenBrace) {
            return false;
        }

        // skip newline trivia before checking for spread
        let spread_index = self.first_non_newline_index_from(self.pos_index() + 1);
        self.token_type_at(spread_index) == TokenType::Spread
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

    /// Return true when the current token starts a labeled tuple head.
    #[inline]
    fn starts_labeled_tuple_head(&mut self) -> bool {
        self.peek_is(TokenType::Identifier)
            && (self.peek_next_is(TokenType::Colon)
                || self.peek_next_is(TokenType::Maybe) && self.peek_next_next_is(TokenType::Colon))
    }

    /// Eat top-level static type arguments.
    ///
    /// Type argument lists accept types only, so tuple labels and spread
    /// are only valid in nested tuple literals, not at the top level.
    #[inline]
    fn eat_static_type_arguments_body(&mut self) -> ParseResult<Vec<LocalNodeId<Argument>>> {
        let mut arguments = smallvec::SmallVec::<[LocalNodeId<Argument>; 4]>::new();
        self.eat_newlines_maybe()?;

        while self.has_more_tokens() {
            // stop on closing `>`
            if self.peek_is(TokenType::GreaterThan) {
                break;
            }

            // spread is not valid in top-level type argument lists
            if self.peek_is(TokenType::Spread) {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            // labeled tuple heads are only valid inside tuple literals
            if self.starts_labeled_tuple_head() {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            // parse one type argument
            let argument_id = self.eat_positional_argument()?;

            arguments.push(argument_id);
            self.eat_newlines_maybe()?;

            // continue through separators
            if Self::is_item_stop_token(self.peek_token_type()) {
                self.eat_item_stop_with_newlines()?;
            } else {
                break;
            }
        }

        Ok(arguments.into_vec())
    }

    /// Eat static arguments (including the `<` and `>` tokens).
    /// Only positional and spread arguments are allowed (no named arguments).
    /// Also handles `<<` (ShiftLeft) for patterns like `Extends<<T>() => ...>`.
    pub fn eat_static_arguments(&mut self) -> ParseResult<Vec<LocalNodeId<Argument>>> {
        let _timing = self.timing_scope(tags::PARSE_ARGUMENT);
        let start = self.mark_span();

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
                self.get_span_from(&start),
                TokenType::Identifier,
            ));
        }

        // regular static arguments (positional/spread only)
        let (ambient_context, expression_context) = self.static_argument_contexts();
        let static_arguments = self.with_options(
            self.options
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
            |parser| parser.eat_static_type_arguments_body(),
        )?;

        // ts expression contexts only close static args on a concrete `>` token
        // (this matches ts disambiguation for cases like `f<T>=x` and `x < y, x >>= y`.. sigh)
        let allow_glued_type_close = self.options.is_in_type()
            || self.options.is_in_decorator()
            || self.language.is_destack();

        self.eat_newlines_maybe()?;
        if allow_glued_type_close {
            self.eat_type_angle_close()?;
        } else {
            self.eat_token(TokenType::GreaterThan)?;
        }
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
        let (ambient_context, expression_context) = self.dynamic_argument_contexts();
        let dynamic_arguments = self.with_options(
            self.options
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
            |parser| parser.eat_positional_arguments_body(TokenType::CloseParenthesis),
        )?;

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
        let mut arguments = smallvec::SmallVec::<[LocalNodeId<Argument>; 4]>::new();
        self.eat_newlines_maybe()?;
        while self.has_more_tokens() {
            if self.peek_is(terminator) {
                break;
            }

            let argument_id = self.eat_positional_argument()?;

            self.eat_newlines_maybe()?;
            self.trim_argument_span_before_separator(argument_id, terminator);
            arguments.push(argument_id);
            if Self::is_item_stop_token(self.peek_token_type()) {
                self.eat_item_stop_with_newlines()?;
            } else {
                break;
            }
        }
        Ok(arguments.into_vec())
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
        let mut arguments = smallvec::SmallVec::<[LocalNodeId<Argument>; 4]>::new();
        self.eat_newlines_maybe()?;
        while self.has_more_tokens() {
            if self.peek_is(terminator) {
                break;
            }

            let argument_id = self.eat_tree_argument()?;

            self.eat_newlines_maybe()?;
            self.trim_argument_span_before_separator(argument_id, terminator);
            arguments.push(argument_id);
            if Self::is_item_stop_token(self.peek_token_type()) {
                self.eat_item_stop_with_newlines()?;
            } else {
                break;
            }
        }
        Ok(arguments.into_vec())
    }

    /// Clamp an argument span so it never crosses one immediate separator token.
    fn trim_argument_span_before_separator(
        &mut self,
        argument_id: LocalNodeId<Argument>,
        terminator: TokenType,
    ) {
        // locate the immediate separator token after trivia
        let mut token_index = self.pos() as usize;
        let separator_token = loop {
            let Some(token) = self.tokens().get(token_index).copied() else {
                return;
            };
            match token.token.ty {
                TokenType::Whitespace
                | TokenType::Newline
                | TokenType::LineComment
                | TokenType::BlockComment
                | TokenType::DocLineComment
                | TokenType::DocBlockComment => {
                    token_index += 1;
                }
                TokenType::Comma => break token,
                token_type if token_type == terminator => break token,
                _ => return,
            }
        };

        // clamp the argument span to the separator start
        let trim_end = separator_token.span.start;
        let argument_span = self.tree.get_span(argument_id);
        if argument_span.file != separator_token.span.file || argument_span.end <= trim_end {
            return;
        }

        let trimmed_argument_span = Span::new(argument_span.file, argument_span.start, trim_end);
        self.tree.set_span(argument_id, trimmed_argument_span);

        let value_id = match self.tree.get(argument_id) {
            Argument::Named { value, .. }
            | Argument::Labeled { value, .. }
            | Argument::Positional { value, .. }
            | Argument::Spread { value, .. } => *value,

            // error slots have no owned value span to trim
            Argument::Error => return,
        };
        let value_span = self.tree.get_span(value_id);
        if value_span.file != separator_token.span.file || value_span.end <= trim_end {
            return;
        }

        // keep value spans aligned with their owning argument spans
        let trimmed_value_span = Span::new(value_span.file, value_span.start, trim_end);
        self.tree.set_span(value_id, trimmed_value_span);
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        Annotation, AnnotationPosition, Argument, Asynchrony, BinaryOperator, BindingKind,
        BindingOperator, Declaration, Decorator, Expression, FunctionMode, IfKind, IntType, Member,
        Mutability, Name, Parameter, Pattern, PatternField, ScalarLiteral, Timing,
        TypeBinaryOperator, TypeLiteral, TypeUnaryOperator, Visibility,
    };
    use destack_source::LanguageType;

    use crate::{
        TestParser, assert_expression_path, assert_name, assert_node, assert_path, assert_string,
    };

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
    fn test_parse_parameter_default_async_lambda_with_await_body_typescript() {
        let mut test = TestParser::new_with_options(
            "loadFonts: () => Promise<void> = async () => { await Fonts.loadElementsFonts(elements); }",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();

        // parameter default should parse as an async lambda value
        assert_node!(parser.tree, parameter_id, Parameter::Named { name, ty: Some(_), default: Some(default), .. } => {
            assert_string!(parser, *name, "loadFonts");
            assert_node!(parser.tree, *default, Expression::Declaration(default_declaration_id) => {
                assert_node!(parser.tree, *default_declaration_id, Declaration::Function { signature, body: Some(body), .. } => {
                    assert_eq!(signature.asynchrony, Asynchrony::Async);
                    assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                        let block = parser.tree.get(*block_id);
                        assert_eq!(block.expressions.len(), 1);
                        assert_node!(parser.tree, block.expressions[0], Expression::Statement(statement_id) => {
                            assert_node!(parser.tree, *statement_id, Expression::Await { expression } => {
                                assert_node!(parser.tree, *expression, Expression::Call { .. });
                            });
                        });
                    });
                });
            });
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
    fn test_parse_parameter_optional_pattern() {
        // []? optional pattern parameter
        let mut test = TestParser::new_with_options("[]?", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Pattern { modifiers: Some(modifiers), pattern, .. } => {
            assert_eq!(modifiers.kind, Some(BindingKind::Maybe));
            assert_node!(parser.tree, *pattern, Pattern::Array { .. } => {});
        });
    }

    #[test]
    fn test_parse_parameter_underscore_name_typescript() {
        // _ in TypeScript parameters is a normal name
        let mut test = TestParser::new_with_options("_", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Named { modifiers: _, name, ty, default } => {
            assert_string!(parser, *name, "_");
            assert!(ty.is_none());
            assert!(default.is_none());
        });
    }

    #[test]
    fn test_parse_parameter_variadic() {
        // ...args
        let mut test = TestParser::new("...args");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::VariadicNamed { modifiers: _, name, ty } => {
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
        assert_node!(parser.tree, parameter_id, Parameter::VariadicNamed { modifiers: _, name, ty } => {
            assert_string!(parser, *name, "args");
            assert!(ty.is_some());
        });
    }

    #[test]
    fn test_parse_parameter_optional_variadic() {
        // ...args? optional rest parameter
        let mut test = TestParser::new_with_options("...args?", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::VariadicNamed { modifiers: Some(modifiers), name, .. } => {
            assert_eq!(modifiers.kind, Some(BindingKind::Maybe));
            assert_string!(parser, *name, "args");
        });
    }

    /// Parse variadic tuple parameter names.
    #[test]
    fn test_parse_parameter_variadic_tuple_name() {
        let mut test = TestParser::new("...[value]: [] | [TNext]");
        let mut parser = test.prepare();
        parser.options.set_in_type(true);
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::VariadicNamed { modifiers: _, name, ty } => {
            assert_string!(parser, *name, "value");
            assert!(ty.is_some());
        });
    }

    #[test]
    fn test_parse_parameter_variadic_array_pattern() {
        // ...[first, second]
        let mut test = TestParser::new_with_options("...[first, second]", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::VariadicPattern { modifiers: _, pattern, ty } => {
            assert!(ty.is_none());
            assert_node!(parser.tree, *pattern, Pattern::Array { fields, .. } => {
                assert_eq!(fields.len(), 2);
            });
        });
    }

    #[test]
    fn test_parse_parameter_variadic_array_pattern_with_type() {
        // ...[body, init]: ConstructorParameters<typeof Response>
        let mut test = TestParser::new_with_options(
            "...[body, init]: ConstructorParameters<typeof Response>",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::VariadicPattern { modifiers: _, pattern, ty } => {
            // [body, init]
            assert_node!(parser.tree, *pattern, Pattern::Array { fields, .. } => {
                assert_eq!(fields.len(), 2);

                assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, .. } => {
                    assert_name!(parser, *name, "body");
                });

                assert_node!(parser.tree, fields[1], PatternField::Named { name, pattern: None, .. } => {
                    assert_name!(parser, *name, "init");
                });
            });

            // ConstructorParameters<typeof Response>
            let ty = ty.expect("expected variadic tuple type annotation");
            assert_node!(parser.tree, ty, Expression::Path { path, static_arguments: Some(static_arguments) } => {
                assert_path!(parser, *path, "ConstructorParameters");
                assert_eq!(static_arguments.len(), 1);

                assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                    assert_node!(parser.tree, *value, Expression::TypeUnary { operator, right } => {
                        assert_eq!(*operator, TypeUnaryOperator::Typeof);
                        assert_node!(parser.tree, *right, Expression::Path { path, static_arguments: None } => {
                            assert_path!(parser, *path, "Response");
                        });
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_parameter_variadic_array_pattern_with_nested_object_and_defaults() {
        // ...[src, { id, systemId, input, syncSnapshot = false } = {} as any]: SpawnArguments<...>
        let mut test = TestParser::new_with_options(
            r#"...[
    src,
    { id, systemId, input, syncSnapshot = false } = {} as any
]: SpawnArguments<TContext, TExpressionEvent, TEvent, TActor>"#,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::VariadicPattern { modifiers: _, pattern, ty: Some(ty) } => {
            // [src, { ... } = {} as any]
            assert_node!(parser.tree, *pattern, Pattern::Array { fields } => {
                assert_eq!(fields.len(), 2);

                // src
                assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, default: None, .. } => {
                    assert_name!(parser, *name, "src");
                });

                // { id, systemId, input, syncSnapshot = false } = {} as any
                assert_node!(parser.tree, fields[1], PatternField::Positional { pattern, default: Some(default) } => {
                    assert_node!(parser.tree, *pattern, Pattern::Object { fields } => {
                        assert_eq!(fields.len(), 4);

                        assert_node!(parser.tree, fields[3], PatternField::Named { name, pattern: None, default: Some(default), .. } => {
                            assert_name!(parser, *name, "syncSnapshot");
                            assert_node!(parser.tree, *default, Expression::ScalarLiteral(ScalarLiteral::Boolean(false)));
                        });
                    });

                    assert_node!(parser.tree, *default, Expression::TypeBinary { operator, left, right } => {
                        assert_eq!(*operator, TypeBinaryOperator::Cast);
                        assert_node!(parser.tree, *left, Expression::ObjectExpression { properties, .. } => {
                            assert!(properties.is_empty());
                        });
                        assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Any));
                    });
                });
            });

            // SpawnArguments<TContext, TExpressionEvent, TEvent, TActor>
            assert_node!(parser.tree, *ty, Expression::Path { path, static_arguments: Some(static_arguments) } => {
                assert_path!(parser, *path, "SpawnArguments");
                assert_eq!(static_arguments.len(), 4);
            });
        });
    }

    #[test]
    fn test_parse_parameter_variadic_object_pattern() {
        // ...{ value: alias }
        let mut test =
            TestParser::new_with_options("...{ value: alias }", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::VariadicPattern { modifiers: _, pattern, ty } => {
            assert!(ty.is_none());
            assert_node!(parser.tree, *pattern, Pattern::Object { fields } => {
                assert_eq!(fields.len(), 1);
            });
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
    fn test_parse_static_parameters_multiline_union_constraint_with_default_typescript() {
        let mut test = TestParser::new_with_options(
            r#"<
  Return extends ReturnType<onRequestHookHandler<RawServer>>
    | ReturnType<onRequestAsyncHookHandler<RawServer>>
    = ReturnType<onRequestHookHandler<RawServer>>
>"#,
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        let static_parameters = parser.eat_static_parameters(true).unwrap();

        // Return extends ReturnType<onRequestHookHandler<RawServer>> | ReturnType<onRequestAsyncHookHandler<RawServer>> = ReturnType<onRequestHookHandler<RawServer>>
        assert_eq!(static_parameters.len(), 1);
        assert_node!(parser.tree, static_parameters[0], Parameter::Named { modifiers, name, ty: Some(ty), default: Some(default) } => {
            assert!(modifiers.is_none());
            assert_string!(parser, *name, "Return");

            // ReturnType<onRequestHookHandler<RawServer>> | ReturnType<onRequestAsyncHookHandler<RawServer>>
            assert_node!(parser.tree, *ty, Expression::Binary { operator, left, right } => {
                assert_eq!(*operator, BinaryOperator::ElementwiseOr);

                // ReturnType<onRequestHookHandler<RawServer>>
                assert_node!(parser.tree, *left, Expression::Path { path, static_arguments: Some(static_arguments) } => {
                    assert_path!(parser, *path, "ReturnType");
                    assert_eq!(static_arguments.len(), 1);

                    assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                        assert_node!(parser.tree, *value, Expression::Path { path, static_arguments: Some(static_arguments) } => {
                            assert_path!(parser, *path, "onRequestHookHandler");
                            assert_eq!(static_arguments.len(), 1);
                            assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                                assert_node!(parser.tree, *value, Expression::Path { path, static_arguments: None } => {
                                    assert_path!(parser, *path, "RawServer");
                                });
                            });
                        });
                    });
                });

                // ReturnType<onRequestAsyncHookHandler<RawServer>>
                assert_node!(parser.tree, *right, Expression::Path { path, static_arguments: Some(static_arguments) } => {
                    assert_path!(parser, *path, "ReturnType");
                    assert_eq!(static_arguments.len(), 1);

                    assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                        assert_node!(parser.tree, *value, Expression::Path { path, static_arguments: Some(static_arguments) } => {
                            assert_path!(parser, *path, "onRequestAsyncHookHandler");
                            assert_eq!(static_arguments.len(), 1);
                            assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                                assert_node!(parser.tree, *value, Expression::Path { path, static_arguments: None } => {
                                    assert_path!(parser, *path, "RawServer");
                                });
                            });
                        });
                    });
                });
            });

            // ReturnType<onRequestHookHandler<RawServer>>
            assert_node!(parser.tree, *default, Expression::Path { path, static_arguments: Some(static_arguments) } => {
                assert_path!(parser, *path, "ReturnType");
                assert_eq!(static_arguments.len(), 1);

                assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                    assert_node!(parser.tree, *value, Expression::Path { path, static_arguments: Some(static_arguments) } => {
                        assert_path!(parser, *path, "onRequestHookHandler");
                        assert_eq!(static_arguments.len(), 1);
                        assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                            assert_node!(parser.tree, *value, Expression::Path { path, static_arguments: None } => {
                                assert_path!(parser, *path, "RawServer");
                            });
                        });
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_parameter_with_readonly_public_modifier_order_reports_error() {
        // readonly public x: number
        let mut test =
            TestParser::new_with_options("readonly public x: number", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();

        // parser reports one modifier ordering error
        assert_eq!(parser.errors.len(), 1);

        // parameter shape remains recoverable for downstream parsing
        assert_node!(parser.tree, parameter_id, Parameter::Named { modifiers: Some(modifiers), name, ty: Some(ty), default: None } => {
            assert_string!(parser, *name, "x");
            assert_eq!(modifiers.mutability, Some(Mutability::Immutable));
            assert_eq!(modifiers.visibility, Some(Visibility::Public));
            assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Number));
        });
    }

    #[test]
    fn test_parse_constructor_parameter_with_readonly_public_modifier_order_reports_error() {
        // class D { constructor(readonly public x: number) {} }
        let mut test = TestParser::new_with_options(
            "class D { constructor(readonly public x: number) {} }",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // parser reports one modifier ordering error
        assert_eq!(parser.errors.len(), 1);
        assert_eq!(expressions.len(), 1);

        // constructor parameter remains available in the recovered class ast
        let expression_id = parser.unwrap_statement_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Class { members, .. } => {
                assert_eq!(members.len(), 1);

                assert_node!(parser.tree, members[0], Member::Method { signature, .. } => {
                    assert_eq!(signature.mode, Some(FunctionMode::Constructor));
                    assert_eq!(signature.dynamic_parameters.len(), 1);

                    assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { modifiers: Some(modifiers), name, ty: Some(ty), default: None } => {
                        assert_string!(parser, *name, "x");
                        assert_eq!(modifiers.mutability, Some(Mutability::Immutable));
                        assert_eq!(modifiers.visibility, Some(Visibility::Public));
                        assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Number));
                    });
                });
            });
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

    /// Parse TypeScript parameter decorators in constructors and methods.
    #[test]
    fn test_parse_parameter_decorators_typescript() {
        let input = r#"
class Test {
    constructor(@p1 t1, @p2 private t2, @p3 ...t3) {}

    method(@p1 t1, @p1 @p2 ...t2) {}
}
"#;
        let mut test = TestParser::new_with_options(input, LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert_eq!(expressions.len(), 1);

        // class Test { ... }
        let expression_id = parser.unwrap_statement_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Class { members, .. } => {
                assert_eq!(members.len(), 2);

                // constructor(@p1 t1, @p2 t2, @p3 ...t3)
                assert_node!(parser.tree, members[0], Member::Method { signature, .. } => {
                    assert_eq!(signature.mode, Some(FunctionMode::Constructor));
                    assert_eq!(signature.dynamic_parameters.len(), 3);

                    // @p1 t1
                    assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { modifiers: None, name, ty: None, default: None } => {
                        assert_string!(parser, *name, "t1");
                    });
                    let t1_annotations = parser.tree.get_annotations(signature.dynamic_parameters[0].id);
                    assert_eq!(t1_annotations.len(), 1);
                    assert_node!(parser.tree, t1_annotations[0], Annotation::Decorator { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPrefix);
                        assert_node!(parser.tree, *node, Decorator { expression } => {
                            assert_expression_path!(parser, parser.tree.get(*expression), "p1");
                        });
                    });

                    // @p2 t2
                    assert_node!(parser.tree, signature.dynamic_parameters[1], Parameter::Named { modifiers: Some(modifiers), name, ty: None, default: None } => {
                        assert_string!(parser, *name, "t2");
                        assert_eq!(modifiers.visibility, Some(Visibility::Private));
                    });
                    let t2_annotations = parser.tree.get_annotations(signature.dynamic_parameters[1].id);
                    assert_eq!(t2_annotations.len(), 1);
                    assert_node!(parser.tree, t2_annotations[0], Annotation::Decorator { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPrefix);
                        assert_node!(parser.tree, *node, Decorator { expression } => {
                            assert_expression_path!(parser, parser.tree.get(*expression), "p2");
                        });
                    });

                    // @p3 ...t3
                    assert_node!(parser.tree, signature.dynamic_parameters[2], Parameter::VariadicNamed { modifiers: None, name, ty: None } => {
                        assert_string!(parser, *name, "t3");
                    });
                    let t3_annotations = parser.tree.get_annotations(signature.dynamic_parameters[2].id);
                    assert_eq!(t3_annotations.len(), 1);
                    assert_node!(parser.tree, t3_annotations[0], Annotation::Decorator { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPrefix);
                        assert_node!(parser.tree, *node, Decorator { expression } => {
                            assert_expression_path!(parser, parser.tree.get(*expression), "p3");
                        });
                    });
                });

                // method(@p1 t1, @p2 ...t2)
                assert_node!(parser.tree, members[1], Member::Method { signature, .. } => {
                    assert_eq!(signature.mode, None);
                    assert_eq!(signature.dynamic_parameters.len(), 2);

                    // @p1 t1
                    assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { modifiers: None, name, ty: None, default: None } => {
                        assert_string!(parser, *name, "t1");
                    });
                    let method_t1_annotations = parser.tree.get_annotations(signature.dynamic_parameters[0].id);
                    assert_eq!(method_t1_annotations.len(), 1);
                    assert_node!(parser.tree, method_t1_annotations[0], Annotation::Decorator { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPrefix);
                        assert_node!(parser.tree, *node, Decorator { expression } => {
                            assert_expression_path!(parser, parser.tree.get(*expression), "p1");
                        });
                    });

                    // @p2 ...t2
                    assert_node!(parser.tree, signature.dynamic_parameters[1], Parameter::VariadicNamed { modifiers: None, name, ty: None } => {
                        assert_string!(parser, *name, "t2");
                    });
                    let method_t2_annotations = parser.tree.get_annotations(signature.dynamic_parameters[1].id);
                    assert_eq!(method_t2_annotations.len(), 2);
                    assert_node!(parser.tree, method_t2_annotations[0], Annotation::Decorator { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPrefix);
                        assert_node!(parser.tree, *node, Decorator { expression } => {
                            assert_expression_path!(parser, parser.tree.get(*expression), "p1");
                        });
                    });
                    assert_node!(parser.tree, method_t2_annotations[1], Annotation::Decorator { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPrefix);
                        assert_node!(parser.tree, *node, Decorator { expression } => {
                            assert_expression_path!(parser, parser.tree.get(*expression), "p2");
                        });
                    });
                });
            });
        });
    }

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

    #[test]
    fn test_parse_named_argument_with_newline_before_assign_in_tsx() {
        let mut test = TestParser::new_with_options(
            "onBroadcastSelected\n    = { this._onYouTubeBroadcastIDSelected }",
            LanguageType::TypeScriptXml,
        );
        let mut parser = test.prepare();
        let argument_id = parser.eat_tree_literal_argument().unwrap();

        assert_node!(parser.tree, argument_id, Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "onBroadcastSelected");
            assert_node!(parser.tree, *value, Expression::Member { left, name, .. } => {
                assert_node!(parser.tree, *left, Expression::This);
                assert_string!(parser, *name, "_onYouTubeBroadcastIDSelected");
            });
        });
    }

    #[test]
    fn test_parse_named_argument_with_numeric_kebab_segment() {
        let mut test = TestParser::new_with_options("panose-1=\"test\"", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let argument_id = parser.eat_tree_literal_argument().unwrap();
        assert_node!(parser.tree, argument_id, Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "panose1");
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string)) => {
                assert_string!(parser, *string, "test");
            });
        });
    }

    #[test]
    fn test_parse_named_argument_with_double_hyphen_kebab_segment() {
        let mut test = TestParser::new_with_options(
            "data-nextjs-container-errors-pseudo-html--diff={sign === '+' ? 'add' : 'remove'}",
            LanguageType::TypeScriptXml,
        );
        let mut parser = test.prepare();
        let argument_id = parser.eat_tree_literal_argument().unwrap();
        assert_node!(parser.tree, argument_id, Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "dataNextjsContainerErrorsPseudoHtmlDiff");
            assert_node!(parser.tree, *value, Expression::If { kind, .. } => {
                assert_eq!(*kind, IfKind::Ternary);
            });
        });
    }

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
    fn test_parse_dynamic_argument_span_trims_before_delayed_comma() {
        let source = r#"(
  a

  ,
  b
)"#;
        let mut test = TestParser::new(source);
        let mut parser = test.prepare();
        let arguments = parser.eat_dynamic_arguments().unwrap();

        // first argument and value span should both end at the separator
        assert_eq!(arguments.len(), 2);
        let first_argument_id = arguments[0];
        let first_value_id = match parser.tree.get(first_argument_id) {
            Argument::Positional { value, .. } => *value,
            _ => panic!("expected first positional argument"),
        };
        let first_argument_span = parser.tree.get_span(first_argument_id);
        let first_value_span = parser.tree.get_span(first_value_id);
        assert_eq!(first_argument_span.end, first_value_span.end);

        // verify spans do not cross the separator token
        let separator_offset = source.find(',').expect("expected comma separator") as u32;
        assert!(first_argument_span.end <= separator_offset);
        assert!(first_value_span.end <= separator_offset);
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
    fn test_parse_spread_argument_with_comment_newline() {
        // .../** @type {number[]} */\nargs
        let mut test = TestParser::new_with_options(
            ".../** @type {number[]} */\nargs",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        let argument_id = parser.eat_positional_argument().unwrap();

        assert_node!(parser.tree, argument_id, Argument::Spread { modifiers: _, label, value } => {
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
        parser.options.set_in_type(true);
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
        parser.options.set_in_type(true);
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

    #[test]
    fn test_parse_tuple_label_argument_multiline_union_type() {
        let mut test = TestParser::new("options?:\n  | SkipToken\n  | OtherOption");
        let mut parser = test.prepare();
        parser.options.set_in_type(true);
        let argument_id = parser.eat_positional_argument().unwrap();
        assert_node!(parser.tree, argument_id, Argument::Labeled { modifiers: Some(modifiers), label, value } => {
            assert_eq!(modifiers.kind, Some(BindingKind::Maybe));
            assert_string!(parser, *label, "options");
            assert_node!(parser.tree, *value, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::ElementwiseOr);
            });
        });
    }

    #[test]
    fn test_parse_static_arguments_with_nested_generics_and_union() {
        let mut test = TestParser::new_with_options(
            "<keyof ServerReservedEventsMap<never, never, never, never> | keyof NamespaceReservedEventsMap<never, never, never, never>>",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let static_arguments = parser.eat_static_arguments().unwrap();

        assert_eq!(static_arguments.len(), 1);
        assert_node!(parser.tree, static_arguments[0], Argument::Positional { modifiers: None, value } => {
            assert_node!(parser.tree, *value, Expression::Binary { left, operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                assert_node!(parser.tree, *left, Expression::TypeUnary { .. });
                assert_node!(parser.tree, *right, Expression::TypeUnary { .. });
            });
        });
    }
}
