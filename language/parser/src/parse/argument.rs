use destack_ast::{
    Argument, Expression, GenericArgument, GenericParameter, Keyword, LiteralType, LocalNodeId,
    Name, NodeType, Parameter, Pattern, ScalarLiteral, StringId, TokenType, TypeExpression,
    VarianceModifier, Visibility,
};
use destack_source::{NodeSpanType, Span};

use crate::parse::parser::ParserOptions;
use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserMark};

/// Parsed parameter head as either one pattern or one named binding.
type ParsedParameterPatternOrName = (Option<LocalNodeId<Pattern>>, Option<StringId>, Option<Span>);

/// Parsed modifiers for one binding-like head.
#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub(crate) struct BindingModifiers {
    /// The visibility modifier.
    pub visibility: Option<Visibility>,
    /// The variance modifier.
    pub variance: Option<VarianceModifier>,
    /// Whether `declare` was present.
    pub is_ambient: bool,
    /// Whether `static` was present.
    pub is_static: bool,
    /// Whether `abstract` was present.
    pub is_abstract: bool,
    /// Whether `override` was present.
    pub is_override: bool,
    /// Whether `readonly` was present.
    pub is_readonly: bool,
    /// Whether `const` was present.
    pub is_const_asserted: bool,
    /// Whether `accessor` was present.
    pub is_accessor: bool,
    /// Whether `comptime` was present.
    pub is_comptime: bool,
    /// Whether `?` was present.
    pub is_optional: bool,
    /// Whether `!` was present.
    pub is_definite: bool,
}

impl BindingModifiers {
    /// Return whether the modifier set is empty.
    pub(crate) fn is_empty(self) -> bool {
        self == Self::default()
    }
}

impl Parser {
    /// Eat a committed generic argument close token or recover one missing `>`.
    fn eat_type_angle_close_or_recover_missing(&mut self, owner: NodeType) -> ParseResult<()> {
        if matches!(
            self.peek_token_type(),
            TokenType::GreaterThan
                | TokenType::ShiftRight
                | TokenType::UnsignedShiftRight
                | TokenType::GreaterThanOrEqual
                | TokenType::ShiftRightAssign
                | TokenType::UnsignedShiftRightAssign
        ) {
            self.eat_type_angle_close()?;
            return Ok(());
        }

        let token_type = self.peek_token_type();
        let index = self.pos_index();
        let is_recoverable_boundary =
            // allow end and generic closers
            token_type == TokenType::End
                || self.options.is_in_static() && Self::starts_type_angle_close(token_type)

                // allow ternary and arrow continuations
                || self.options.is_in_ternary_condition() && token_type == TokenType::Colon
                || self.options.is_in_type()
                    && matches!(token_type, TokenType::Arrow | TokenType::ArrowWide)

                // allow stops and delimiters
                || matches!(
                    token_type,
                    TokenType::Comma
                        | TokenType::Semicolon
                        | TokenType::Newline
                        | TokenType::Maybe
                        | TokenType::TemplateString
                        | TokenType::TemplateStringStart
                )
                || Self::is_close_delimiter_token(token_type)
                || matches!(
                    token_type,
                    TokenType::OpenParenthesis | TokenType::OpenBracket | TokenType::Dot
                )

                // allow heritage terminators after generic arguments
                || self.options.is_in_super_type()
                    && (token_type == TokenType::OpenBrace
                        || token_type == TokenType::Identifier
                            && matches!(
                                self.keyword_for_index(index),
                                Some(Keyword::Implements | Keyword::With | Keyword::Where)
                            ))

                // allow operator continuations
                || self.has_infix_or_assign_operator_at_index(index);

        self.recover_missing_token_here(TokenType::GreaterThan, owner, is_recoverable_boundary)
    }

    /// Return the common context for non-sequence argument values.
    #[inline]
    fn current_non_sequence_argument_context(&self) -> ParserOptions {
        self.options
            .not_in_sequence_expression()
            .not_in_arrow_return_type()
    }

    /// Return whether the current token ends one generic argument payload.
    #[inline]
    fn current_token_ends_generic_argument(&mut self) -> bool {
        if self.peek_is(TokenType::Comma) || self.peek_starts_type_angle_close() {
            return true;
        }

        // allow one final type argument to commit before one missing close angle at eof
        if self.peek_is(TokenType::End) {
            return true;
        }

        if !self.peek_is(TokenType::Newline) {
            return false;
        }

        if self.is_token_after_newlines(self.pos(), TokenType::Comma) {
            return true;
        }

        let next_index = self.first_non_newline_index_from(self.pos_index());
        Self::starts_type_angle_close(self.token_type_at(next_index))
    }

    /// Return whether one generic argument slot stays in type space.
    fn generic_argument_slot_stays_in_type_space(&mut self, context: ParserOptions) -> bool {
        // non type ambient sites parse generic arguments as plain expressions
        if !self.options.is_in_type() {
            return false;
        }

        // type ambient sites commit only when one full type expression consumes the slot
        let speculative_start = self.mark();
        let speculative_start_idx = self.tree.next_id();
        let parsed_type_expression =
            self.with_options(context, |parser| parser.eat_type_expression());
        let prefers_type_expression =
            parsed_type_expression.is_ok() && self.current_token_ends_generic_argument();

        self.restore(speculative_start, speculative_start_idx);

        prefers_type_expression
    }

    /// Eat one mixed generic argument node.
    fn eat_generic_argument(
        &mut self,
        context: ParserOptions,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<GenericArgument>> {
        // direct spread generic arguments are not supported
        if self.peek_is(TokenType::Spread) {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        // type ambient sites commit only when one full type expression owns the slot
        if self.generic_argument_slot_stays_in_type_space(context) {
            let value = self.with_options(context, |parser| parser.eat_type_expression())?;

            return Ok(self.insert_node(GenericArgument::Type { value }, self.get_span_from(start)));
        }

        // otherwise parse the slot in value space
        let value_ambient_context = self.options.with_type(false);
        let value = self.eat_expression(
            self.options
                .with_ambient_context(value_ambient_context)
                .with_expression_context(context),
        )?;

        Ok(self.insert_node(GenericArgument::Value { value }, self.get_span_from(start)))
    }

    /// Return the common context for positional argument values.
    #[inline]
    fn current_positional_argument_context(&self) -> ParserOptions {
        self.current_non_sequence_argument_context()
            .not_in_position()
    }

    /// Return true when the current keyword should end a malformed parameter list after a newline.
    fn current_keyword_starts_parameter_recovery_boundary(&mut self) -> bool {
        // only statement scoped dynamic parameters should stop at newline led keywords
        if self.options.is_in_static() || !self.options.is_in_statement_context() {
            return false;
        }

        // only newline led keywords can start a following statement
        if !self.input_has_line_terminator_before_current_token() {
            return false;
        }

        // parameter heads should win when the keyword is already followed by a
        // parameter continuation like `type:` or `namespace:`
        if self.current_keyword_continues_parameter_head() {
            return false;
        }

        // statement keywords
        let Some(keyword) = self.keyword_for_index(self.pos_index()) else {
            return false;
        };

        matches!(
            keyword,
            Keyword::Abstract
                | Keyword::Break
                | Keyword::Class
                | Keyword::Const
                | Keyword::Continue
                | Keyword::Do
                | Keyword::Enum
                | Keyword::Export
                | Keyword::Extension
                | Keyword::For
                | Keyword::Function
                | Keyword::If
                | Keyword::Import
                | Keyword::Interface
                | Keyword::Let
                | Keyword::Match
                | Keyword::Namespace
                | Keyword::Return
                | Keyword::Switch
                | Keyword::Throw
                | Keyword::Type
                | Keyword::Try
                | Keyword::Using
                | Keyword::Var
                | Keyword::While
        )
    }

    /// Return true when the current keyword token already looks like a parameter head.
    fn current_keyword_continues_parameter_head(&mut self) -> bool {
        let next_index = self.next_non_newline_index_from(self.pos_index().saturating_add(1));
        let next_token_type = self.token_type_at(next_index);

        matches!(
            next_token_type,
            TokenType::Assign
                | TokenType::CloseParenthesis
                | TokenType::Colon
                | TokenType::Comma
                | TokenType::Maybe
        )
    }

    /// Return true when one recovered argument list should stop at the current statement boundary.
    fn should_end_recovered_argument_list_at_statement_boundary(
        &mut self,
        is_recovered_argument: bool,
    ) -> bool {
        is_recovered_argument
            && self.options.is_in_statement_context()
            && self.input_has_line_terminator_before_current_token()
    }

    /// Return true when one parsed argument came from a recovered missing or error slot.
    fn argument_has_recovered_slot(&self, argument_id: LocalNodeId<Argument>) -> bool {
        match self.tree.get(argument_id) {
            Argument::Error => true,

            // missing and error values should stop newline led statement calls locally
            Argument::Named { value, .. }
            | Argument::Labeled { value, .. }
            | Argument::Positional { value, .. }
            | Argument::Spread { value, .. } => {
                matches!(
                    self.tree.get(*value),
                    Expression::Missing | Expression::Error
                )
            }
        }
    }

    /// Parse a parameter type annotation expression.
    #[inline]
    fn eat_parameter_type_expression(&mut self) -> ParseResult<LocalNodeId<TypeExpression>> {
        let ambient_context = self.options.with_type(true);
        let mut expression_context = self.options.not_in_position().not_in_left_precedence();
        if self.options.is_in_type_conditional_right() {
            expression_context = expression_context.in_type_conditional_right();
        }
        self.eat_type_expression_node_or_recover_missing(
            self.options
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
            NodeType::Parameter,
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
    pub(crate) fn eat_binding_modifiers_prefix_maybe(
        &mut self,
        allow_readonly_key: bool,
        allow_accessor_modifier: bool,
        allow_variance_modifier: bool,
        allow_declare_modifier: bool,
        validate_modifier_order: bool,
    ) -> ParseResult<Option<BindingModifiers>> {
        // initialize modifier state
        let mut modifiers = BindingModifiers::default();
        let mut has_modifiers = false;

        // modifier ordering for typed member forms
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
            let current_keyword = self.keyword_for_index(position_index);
            let is_out_variance_modifier = allow_variance_modifier
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

            // variance for generic parameters
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
                if modifiers.is_ambient {
                    if validate_modifier_order {
                        self.error(&ParseError::unexpected(span));
                    }
                } else {
                    modifiers.is_ambient = true;
                }
                has_modifiers = true;
                progress = true;
            }

            // scope modifiers (static)
            if !modifiers.is_static && self.is_keyword(Keyword::Static) {
                if !self.next_token_starts_member_name() {
                    break;
                }
                let span = self.peek()?.span;
                self.bump(); // eat static
                modifiers.is_static = true;
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
                modifiers.is_abstract = true;
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
                modifiers.is_override = true;
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
            if !modifiers.is_readonly && self.is_keyword(Keyword::Readonly) && readonly_is_modifier
            {
                self.bump(); // eat readonly
                modifiers.is_readonly = true;
                seen_readonly = true;
                has_modifiers = true;
                progress = true;
            }

            // operator modifiers (const)
            if !modifiers.is_const_asserted && self.is_keyword(Keyword::Const) {
                if !self.next_token_starts_member_name() {
                    break;
                }
                self.bump(); // eat const
                modifiers.is_const_asserted = true;
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
            if !modifiers.is_accessor && accessor_is_modifier {
                if !self.next_token_starts_member_name() {
                    break;
                }
                self.bump(); // eat accessor
                modifiers.is_accessor = true;
                seen_accessor = true;
                has_modifiers = true;
                progress = true;
            }

            // timing modifiers (comptime)
            if !modifiers.is_comptime && self.is_keyword(Keyword::Comptime) {
                if !self.next_token_starts_member_name() {
                    break;
                }
                self.bump(); // eat comptime
                modifiers.is_comptime = true;
                has_modifiers = true;
                progress = true;
            }

            // exit loop if no progress made
            if !progress {
                break;
            }
        }

        if has_modifiers && !modifiers.is_empty() {
            Ok(Some(modifiers))
        } else {
            Ok(None)
        }
    }

    /// Eat one optional postfix binding modifier.
    pub(crate) fn eat_binding_modifiers_postfix_maybe(
        &mut self,
        modifiers: Option<BindingModifiers>,
    ) -> ParseResult<Option<BindingModifiers>> {
        if self.peek_is(TokenType::Maybe) {
            self.bump(); // eat maybe
            let mut modifiers = modifiers.unwrap_or_default();
            modifiers.is_optional = true;
            Ok(Some(modifiers))
        } else {
            Ok(modifiers)
        }
    }

    /// Eat one parameter.
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
            let mut modifier_set = modifiers.unwrap_or_default();
            modifier_set.is_optional = true;
            modifiers = Some(modifier_set);
        }

        // : type (or keyword for #Compatibility)
        let (declared_type, ty_span) = {
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
                let declared_type = if self.peek_is(TokenType::Assign)
                    || self.peek_is(TokenType::Comma)
                    || self.peek_is(TokenType::CloseParenthesis)
                    || self.peek_starts_type_angle_close()
                    || self.peek_is(TokenType::End)
                {
                    self.recover_missing_type_expression_here(NodeType::Parameter)
                } else {
                    self.eat_parameter_type_expression()?
                };
                let type_span = self.get_span_from(&type_start);
                (Some(declared_type), Some(type_span))
            } else {
                (None, None)
            }
        };

        let visibility = modifiers.and_then(|modifier_set| modifier_set.visibility);
        let is_readonly = modifiers.is_some_and(|modifier_set| modifier_set.is_readonly);
        let is_optional = modifiers.is_some_and(|modifier_set| modifier_set.is_optional);

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
                let value = if self.peek_is(TokenType::Comma)
                    || self.peek_is(TokenType::CloseParenthesis)
                    || self.peek_starts_type_angle_close()
                    || self.peek_is(TokenType::End)
                {
                    self.recover_missing_expression_here(NodeType::Parameter)
                } else {
                    self.eat_parameter_default_expression()
                        .for_node_type(NodeType::Parameter)?
                };

                // named with default
                if let Some(name) = name {
                    Parameter::Named {
                        name,
                        visibility,
                        is_readonly,
                        is_optional,
                        declared_type,
                        default: Some(value),
                    }
                }
                // pattern with default
                else {
                    Parameter::Pattern {
                        pattern: pattern.expect("peeked"),
                        is_optional,
                        declared_type,
                        default: Some(value),
                    }
                }
            }
            // variadic parameter (cannot have a default value)
            else if is_variadic {
                if let Some(name) = name {
                    Parameter::VariadicNamed {
                        name,
                        visibility,
                        is_readonly,
                        declared_type,
                    }
                } else {
                    Parameter::VariadicPattern {
                        pattern: pattern.expect("peeked"),
                        declared_type,
                    }
                }
            }
            // no default value
            else {
                // named without default
                if let Some(name) = name {
                    Parameter::Named {
                        name,
                        visibility,
                        is_readonly,
                        is_optional,
                        declared_type,
                        default: None,
                    }
                }
                // pattern without default
                else {
                    Parameter::Pattern {
                        pattern: pattern.expect("peeked"),
                        is_optional,
                        declared_type,
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
            let ambient_context = self.options.with_before_type(true);
            let expression_context = self.options;
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
    ) -> ParseResult<Option<(StringId, Span)>> {
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

    /// Eat one parameter list.
    /// Parameters may be comma or newline separated.
    ///
    /// Examples:
    /// ```
    /// x: int32
    /// x: int32, y: int32
    /// x: int32
    /// y: int32
    /// ```
    pub fn eat_parameters_body(&mut self) -> ParseResult<Vec<LocalNodeId<Parameter>>> {
        let mut parameters: Vec<LocalNodeId<Parameter>> = Vec::new();
        let in_js = self.language.is_javascript();
        let mut has_variadic_parameter = false;
        self.eat_newlines_maybe()?;
        while self.has_more_tokens() {
            if self.peek_is(TokenType::CloseParenthesis) || self.peek_starts_type_angle_close() {
                break;
            }

            // newline led statement keywords should stay outside malformed parameter lists
            if self.current_keyword_starts_parameter_recovery_boundary() {
                break;
            }

            // one parameter slot
            let parameter_start = self.mark_span();
            let mut is_recovered_parameter = false;
            let parameter = match self.eat_parameter().for_node_type(NodeType::Parameter) {
                Ok(parameter) => parameter,
                Err(error) => {
                    is_recovered_parameter = true;
                    let recover_at_statement_keyword =
                        self.current_keyword_starts_parameter_recovery_boundary();

                    // newline led keyword statements should stay outside malformed parameter lists
                    if recover_at_statement_keyword {
                        let error = ParseError::from_source_maybe(
                            self.get_span_from(&parameter_start),
                            Some(error),
                        );
                        self.error(&error);
                    } else {
                        self.try_recover_in_item_list(
                            &parameter_start,
                            if self.options.is_in_static() {
                                TokenType::GreaterThan
                            } else {
                                TokenType::CloseParenthesis
                            },
                            Some(error),
                        )?;
                    }

                    self.insert_node(Parameter::Error, self.get_span_from(&parameter_start))
                }
            };

            // rest parameters must be terminal in untyped parameter lists
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

            // untyped parameter lists reject trailing separators after rest parameters
            if in_js && has_variadic_parameter && self.is_item_stop() {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            // continue regular parameter lists after a real separator
            if self.peek_is(TokenType::Comma) {
                self.eat_item_stop_with_newlines()?;

                if !is_recovered_parameter {
                    continue;
                }

                if self.current_keyword_starts_parameter_recovery_boundary() {
                    break;
                }

                if !self.can_continue_after_recovered_item(
                    if self.options.is_in_static() {
                        TokenType::GreaterThan
                    } else {
                        TokenType::CloseParenthesis
                    },
                    is_recovered_parameter,
                ) {
                    break;
                }

                continue;
            }
            // recovered parameter lists should stop before a newline led keyword statement
            // recovered slots may continue across newline separators only
            else if is_recovered_parameter
                && (self.current_keyword_starts_parameter_recovery_boundary()
                    || !self.can_continue_after_recovered_item(
                        if self.options.is_in_static() {
                            TokenType::GreaterThan
                        } else {
                            TokenType::CloseParenthesis
                        },
                        is_recovered_parameter,
                    ))
            {
                break;
            }
        }
        Ok(parameters)
    }

    /// Parse one generic parameter in a generic parameter list.
    fn eat_generic_parameter(&mut self) -> ParseResult<LocalNodeId<GenericParameter>> {
        let start = self.mark_span();

        // generic parameters accept only the dedicated generic modifiers
        let mut variance = None;
        let mut is_comptime = false;

        loop {
            if self.is_keyword(Keyword::In) {
                self.bump(); // eat in
                variance = Some(match variance {
                    Some(VarianceModifier::Out) => VarianceModifier::InOut,
                    Some(VarianceModifier::InOut) => VarianceModifier::InOut,
                    _ => VarianceModifier::In,
                });
                continue;
            }

            let position_index = self.pos_index();
            let is_out_modifier = self.peek_is(TokenType::Identifier)
                && self.identifier_equals_at(position_index, "out")
                && self.peek_next_is(TokenType::Identifier);
            if is_out_modifier {
                self.bump(); // eat out
                variance = Some(match variance {
                    Some(VarianceModifier::In) => VarianceModifier::InOut,
                    Some(VarianceModifier::InOut) => VarianceModifier::InOut,
                    _ => VarianceModifier::Out,
                });
                continue;
            }

            if self.is_keyword(Keyword::Comptime) || self.is_keyword(Keyword::Const) {
                self.bump(); // eat comptime or const
                is_comptime = true;
                continue;
            }

            break;
        }

        // generic parameters are always named
        let (name, name_span) = self.eat_binding_identifier_with_span()?;

        // generic parameters are type parameters by default
        // value parameters must opt in with `comptime` or `const`
        let annotation_index = self.next_non_newline_index_from(self.pos_index());
        let annotation_token_type = self.token_type_at(annotation_index);
        let annotation_keyword = if annotation_token_type == TokenType::Identifier {
            self.keyword_for_index(annotation_index)
        } else {
            None
        };
        let has_colon_annotation = annotation_token_type == TokenType::Colon;
        let has_type_constraint = matches!(
            annotation_keyword,
            Some(Keyword::Extends | Keyword::Implements)
        );
        let has_annotation = has_colon_annotation || has_type_constraint;
        let is_value_parameter = is_comptime;

        let (declared_type, declared_type_span) = if has_annotation {
            let type_start = self.mark_span();
            self.eat_newlines_maybe()?;
            self.bump(); // eat colon or relation keyword
            self.eat_newlines_maybe()?;
            let declared_type = if self.peek_is(TokenType::Assign)
                || self.peek_is(TokenType::Comma)
                || self.peek_starts_type_angle_close()
                || self.peek_is(TokenType::End)
            {
                self.recover_missing_type_expression_here(NodeType::GenericParameter)
            } else {
                self.eat_parameter_type_expression()?
            };
            (Some(declared_type), Some(self.get_span_from(&type_start)))
        } else {
            (None, None)
        };

        // type parameters default in type space, value parameters default in expression space
        let default = if self.peek_is(TokenType::Assign)
            || self.peek_is(TokenType::Newline)
                && self.is_token_after_newlines(self.pos(), TokenType::Assign)
        {
            self.eat_newlines_maybe()?;
            self.bump(); // eat =
            self.eat_newlines_maybe()?;

            if is_value_parameter {
                let value = if self.peek_is(TokenType::Comma)
                    || self.peek_starts_type_angle_close()
                    || self.peek_is(TokenType::End)
                {
                    self.recover_missing_expression_here(NodeType::GenericParameter)
                } else {
                    self.eat_parameter_default_expression()?
                };
                Ok::<_, ParseError>((Some(value), None))
            } else {
                let value = if self.peek_is(TokenType::Comma)
                    || self.peek_starts_type_angle_close()
                    || self.peek_is(TokenType::End)
                {
                    self.recover_missing_type_expression_here(NodeType::GenericParameter)
                } else {
                    self.eat_parameter_type_expression()?
                };
                Ok::<_, ParseError>((None, Some(value)))
            }?
        } else {
            (None, None)
        };

        // generic value parameters must be marked comptime
        let parameter = if is_value_parameter {
            GenericParameter::Value {
                name,
                declared_type,
                default: default.0,
                is_comptime,
            }
        } else {
            GenericParameter::Type {
                name,
                variance,
                constraint: declared_type,
                default: default.1,
            }
        };

        let parameter_id = self.insert_node(parameter, self.get_span_from(&start));
        self.tree.set_main_span(parameter_id, name_span);

        if let Some(span) = declared_type_span {
            self.tree
                .set_side_span(parameter_id, NodeSpanType::Type, span);
        }

        Ok(parameter_id)
    }

    /// Eat generic parameters, including the `<` and `>` tokens, if they exist.
    ///
    /// `allow_empty_parameters` accepts `<>` as an empty parameter list.
    pub fn eat_generic_parameters_maybe(
        &mut self,
        allow_empty_parameters: bool,
    ) -> ParseResult<Option<Vec<LocalNodeId<GenericParameter>>>> {
        let start_index = self.pos_index();
        let static_index = self.next_non_newline_index_from(start_index);
        if self.token_type_at(static_index) != TokenType::LessThan {
            return Ok(None);
        }

        if static_index != start_index {
            self.eat_newlines_maybe()?;
        }

        Ok(Some(self.eat_generic_parameters(allow_empty_parameters)?))
    }

    /// Eat generic parameters, including the `<` and `>` tokens.
    ///
    /// `allow_empty_parameters` accepts `<>` as an empty parameter list.
    pub fn eat_generic_parameters(
        &mut self,
        allow_empty_parameters: bool,
    ) -> ParseResult<Vec<LocalNodeId<GenericParameter>>> {
        let start = self.mark_span();
        self.eat_token(TokenType::LessThan)?;
        self.eat_newlines_maybe()?;

        // optionally accept empty generic parameters
        if self.peek_starts_type_angle_close() {
            self.eat_type_angle_close()?;
            if allow_empty_parameters {
                return Ok(vec![]);
            }
            return Err(ParseError::expected(
                self.get_span_from(&start),
                TokenType::Identifier,
            ));
        }

        // regular generic parameters
        let mut ambient_context = self.options.nested().with_static(true);
        if self.options.is_in_type()
            || self.options.is_in_decorator()
            || self.language.is_typescript()
        {
            ambient_context = ambient_context.with_type(true);
        }
        let expression_context = self.options.nested();
        let parameters = self.with_options(
            self.options
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
            |parser| {
                let mut parameters = Vec::new();
                parser.eat_newlines_maybe()?;
                while parser.has_more_tokens() {
                    if parser.peek_starts_type_angle_close() {
                        break;
                    }

                    let parameter = parser
                        .eat_generic_parameter()
                        .for_node_type(NodeType::GenericParameter)?;
                    parameters.push(parameter);

                    parser.eat_newlines_maybe()?;
                    if parser.peek_starts_type_angle_close() {
                        break;
                    }

                    if parser.peek_is(TokenType::Comma) {
                        parser.bump(); // eat comma
                        parser.eat_newlines_maybe()?;
                        continue;
                    }

                    break;
                }

                Ok(parameters)
            },
        )?;
        self.eat_type_angle_close_or_recover_missing(NodeType::Expression)?;
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
        let mut ambient_context = self.options.nested();
        if self.options.is_in_type() {
            ambient_context = ambient_context.with_type(true);
        }
        if self.options.is_in_variant() {
            ambient_context = ambient_context.with_variant(true);
        }
        let expression_context = self.options.nested();
        let parameters = self.with_options(
            self.options
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
            |parser| parser.eat_parameters_body(),
        )?;
        self.eat_list_close_token_or_recover_missing(
            TokenType::CloseParenthesis,
            NodeType::Parameter,
        )?;
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
    /// Used for dynamic arguments, generic arguments, and tuple literals.
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
        // hot path: plain positional value arguments
        if !self.peek_is(TokenType::At) && !self.peek_is(TokenType::Spread) {
            // empty slots should recover as argument list errors, not expression errors
            if Self::is_expression_slot_boundary_token(self.peek_token_type()) {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            let value = self.eat_expression_with_context_unchecked(
                self.current_positional_argument_context(),
            )?;
            let value_span = self.tree.get_span(value);
            let argument_id = self.insert_node(Argument::Positional { value }, value_span);
            return Ok(argument_id);
        }

        let start = self.mark_span();
        let decorators = if !self.options.is_in_type() {
            self.eat_decorators_maybe()?
        } else {
            smallvec::SmallVec::new()
        };

        // spread argument
        if self.peek_is(TokenType::Spread) {
            self.bump(); // eat spread
            self.eat_newlines_maybe()?;
            let value = self.eat_expression_with_context_unchecked(
                self.current_positional_argument_context(),
            )?;

            // build spread argument
            let argument_id = self.insert_node(
                Argument::Spread { label: None, value },
                self.get_span_from(&start),
            );
            self.attach_decorators(argument_id.id, decorators);
            return Ok(argument_id);
        }

        // positional value expression
        let value =
            self.eat_expression_with_context_unchecked(self.current_positional_argument_context())?;

        // build positional argument
        let argument_id =
            self.insert_node(Argument::Positional { value }, self.get_span_from(&start));
        self.attach_decorators(argument_id.id, decorators);
        Ok(argument_id)
    }

    /// Eat one argument.
    /// Supports named arguments for tree literal children and import with syntax.
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
        self.eat_tree_argument_with_child_context(false)
    }

    /// Eat one tree child argument and optionally re-enter child lexing after `}`.
    ///
    /// Examples:
    /// ```
    /// x: 1
    /// ...args
    /// {value}
    /// <Widget prop=value />
    /// ```
    pub(crate) fn eat_tree_argument_with_child_context(
        &mut self,
        in_tree_child: bool,
    ) -> ParseResult<LocalNodeId<Argument>> {
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
            let argument_id =
                self.insert_node(Argument::Named { name, value }, self.get_span_from(&start));
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
                Argument::Spread { label: None, value },
                self.get_span_from(&start),
            );
            Ok(argument_id)
        }
        // expression container ({expr}): braces are delimiters, not part of the expression
        else if self.peek_is(TokenType::OpenBrace) {
            self.bump(); // eat {
            self.eat_newlines_maybe()?;

            // empty container (including comment-only containers)
            if self.peek_is(TokenType::CloseBrace) {
                let value = self
                    .tree
                    .insert(Expression::Stub, self.get_span_from(&start));

                self.bump(); // eat }
                let argument_id =
                    self.insert_node(Argument::Positional { value }, self.get_span_from(&start));
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
                if in_tree_child {
                    self.expect_tree_child(TokenType::CloseBrace)?;
                } else {
                    self.eat_close_token_or_recover_missing(
                        TokenType::CloseBrace,
                        NodeType::Argument,
                    )?;
                }
                let argument_id = self.insert_node(
                    Argument::Spread { label: None, value },
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
            if in_tree_child {
                self.expect_tree_child(TokenType::CloseBrace)?;
            } else {
                self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Argument)?;
            }
            let argument_id =
                self.insert_node(Argument::Positional { value }, self.get_span_from(&start));
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

                if is_tree_text {
                    let value = self.eat_tree_child_scalar_expression(in_tree_child)?;
                    let argument_id = self
                        .insert_node(Argument::Positional { value }, self.get_span_from(&start));
                    return Ok(argument_id);
                }
            }

            let value = if self.peek_is(TokenType::LessThan) && self.peek_tree_literal().is_ok() {
                self.eat_tree_literal_with_child_context(in_tree_child)?
            } else {
                let value_expression_context =
                    self.options.not_in_position().not_in_sequence_expression();
                self.eat_expression_with_context_unchecked(value_expression_context)?
            };
            let argument_id =
                self.insert_node(Argument::Positional { value }, self.get_span_from(&start));
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
                Argument::Spread { label: None, value },
                self.get_span_from(&start),
            );
            Ok(argument_id)
        }
        // spread expression container (like {...expr} in tree literals for #Compatibility)
        else if self.starts_tree_spread_attribute() {
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
            self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Argument)?;
            let argument_id = self.insert_node(
                Argument::Spread { label: None, value },
                self.get_span_from(&start),
            );
            Ok(argument_id)
        }
        // named argument
        else {
            let name = self.eat_tree_literal_identifier()?;

            // explicit value separators, including newline wrapped forms
            let has_value_separator = self.tree_literal_argument_has_value_separator();

            // named argument with value
            let value = if has_value_separator {
                self.set_tree_attribute_value(true);
                self.eat_newlines_maybe()?;
                self.bump(); // eat colon or assign
                self.eat_newlines_maybe()?;

                // tree expression container: attr={expr}
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
                    self.eat_close_token_or_recover_missing(
                        TokenType::CloseBrace,
                        NodeType::Argument,
                    )?;
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
                    name: Name::Identifier(name),
                    value,
                },
                self.get_span_from(&start),
            );
            Ok(argument_id)
        }
    }

    /// Return true when the current tree literal argument starts with `{ ...`.
    pub(crate) fn starts_tree_spread_attribute(&mut self) -> bool {
        // must begin at an expression container
        if !self.peek_is(TokenType::OpenBrace) {
            return false;
        }

        // skip newline trivia before checking for spread
        let spread_index = self.first_non_newline_index_from(self.pos_index() + 1);
        self.token_type_at(spread_index) == TokenType::Spread
    }

    /// Return true when the current tree argument has an explicit value separator.
    pub(crate) fn tree_literal_argument_has_value_separator(&mut self) -> bool {
        self.peek_is(TokenType::Colon)
            || self.peek_is(TokenType::Assign)
            || self.peek_is(TokenType::Newline)
                && (self.is_token_after_newlines(self.pos(), TokenType::Colon)
                    || self.is_token_after_newlines(self.pos(), TokenType::Assign))
    }

    /// Eat generic arguments, including the `<` and `>` tokens, if they exist.
    /// Also handles `<<` (ShiftLeft) for patterns like `Extends<<T>() => ...>`.
    pub fn eat_generic_arguments_maybe(
        &mut self,
    ) -> ParseResult<Option<Vec<LocalNodeId<GenericArgument>>>> {
        if self.peek_is(TokenType::LessThan) || self.peek_is(TokenType::ShiftLeft) {
            return Ok(Some(self.eat_generic_arguments()?));
        }
        Ok(None)
    }

    /// Eat top-level generic arguments.
    #[inline]
    fn eat_generic_arguments_body(&mut self) -> ParseResult<Vec<LocalNodeId<GenericArgument>>> {
        let mut arguments = smallvec::SmallVec::<[LocalNodeId<GenericArgument>; 4]>::new();
        self.eat_newlines_maybe()?;

        while self.has_more_tokens() {
            // stop on closing `>`
            if self.peek_starts_type_angle_close() {
                break;
            }

            // one argument slot
            let argument_start = self.mark_span();
            let mut is_recovered_argument = false;
            let argument_id = match self.eat_generic_argument(
                self.current_non_sequence_argument_context(),
                &argument_start,
            ) {
                Ok(argument) => argument,
                Err(error) => {
                    is_recovered_argument = true;
                    self.try_recover_in_item_list(
                        &argument_start,
                        TokenType::GreaterThan,
                        Some(error),
                    )?;

                    self.insert_node(GenericArgument::Error, self.get_span_from(&argument_start))
                }
            };

            arguments.push(argument_id);
            self.eat_newlines_maybe()?;

            // continue through separators
            if self.peek_is(TokenType::Comma) {
                self.eat_item_stop_with_newlines()?;
            }
            // recovered slots may continue across newline separators only
            else if !self
                .can_continue_after_recovered_item(TokenType::GreaterThan, is_recovered_argument)
            {
                break;
            }
        }

        Ok(arguments.into_vec())
    }

    /// Eat generic arguments, including the `<` and `>` tokens.
    /// Only type and value arguments are allowed.
    /// Also handles `<<` (ShiftLeft) for patterns like `Extends<<T>() => ...>`.
    pub fn eat_generic_arguments(&mut self) -> ParseResult<Vec<LocalNodeId<GenericArgument>>> {
        let _timing = self.timing_scope(tags::PARSE_ARGUMENT);
        let start = self.mark_span();
        let used_shift_left_start = self.peek_is(TokenType::ShiftLeft);

        // handle both `<` and `<<` (ShiftLeft) as opening token
        // `<<` occurs when the first argument is a generic arrow function like `<T>() => ...`
        if self.peek_is(TokenType::LessThan) {
            self.bump(); // eat `<`
        } else if self.peek_is(TokenType::ShiftLeft) {
            if !self.re_lex_generic_l_angle() {
                return Err(ParseError::expected(self.peek()?.span, TokenType::LessThan));
            }

            self.bump();
        } else {
            return Err(ParseError::expected(self.peek()?.span, TokenType::LessThan));
        }
        self.eat_newlines_maybe()?;

        // empty generic arguments only recover in committed type-like contexts
        let allow_empty_generic_arguments =
            self.options.is_in_type() || self.options.is_in_decorator();
        let has_empty_generic_arguments = if allow_empty_generic_arguments {
            self.peek_starts_type_angle_close()
        } else {
            self.peek_starts_expression_type_angle_close()
        };

        let generic_arguments = if has_empty_generic_arguments {
            if !allow_empty_generic_arguments {
                return Err(ParseError::expected(
                    self.get_span_from(&start),
                    TokenType::Identifier,
                ));
            }

            let error = ParseError::expected(self.get_span_from(&start), TokenType::Identifier);
            self.error(&error);

            let argument_start = self.mark_span();
            vec![self.insert_node(GenericArgument::Error, self.get_span_from(&argument_start))]
        }
        // regular generic arguments: type or value only
        else {
            let mut ambient_context = self.options.nested().with_static(true);
            if self.options.is_in_type()
                || self.options.is_in_decorator()
                || self.language.is_destack()
                || self.language.is_typescript()
            {
                ambient_context = ambient_context.with_type(true);
            }
            let expression_context = self.options.nested();
            self.with_options(
                self.options
                    .with_ambient_context(ambient_context)
                    .with_expression_context(expression_context),
                |parser| parser.eat_generic_arguments_body(),
            )?
        };

        // committed type-like contexts can consume glued right-angle tails
        let allow_glued_type_close =
            self.options.is_in_type() || self.options.is_in_decorator() || used_shift_left_start;
        let allow_missing_type_close = self.options.is_in_type() || self.options.is_in_decorator();

        self.eat_newlines_maybe()?;
        if allow_glued_type_close {
            if allow_missing_type_close {
                self.eat_type_angle_close_or_recover_missing(NodeType::Expression)?;
            } else {
                self.eat_type_angle_close()?;
            }
        } else {
            self.eat_expression_type_angle_close()?;
        }
        Ok(generic_arguments)
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
        let ambient_context = self.options.nested();
        let expression_context = self.options.nested();
        let dynamic_arguments = self.with_options(
            self.options
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
            |parser| parser.eat_positional_arguments_body(TokenType::CloseParenthesis),
        )?;

        self.eat_newlines_maybe()?;
        self.eat_list_close_token_or_recover_missing(
            TokenType::CloseParenthesis,
            NodeType::Expression,
        )?;

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

            // one argument slot
            let argument_start = self.mark_span();
            let is_recovered_argument;
            let argument_id = match self.eat_positional_argument() {
                Ok(argument_id) => {
                    is_recovered_argument = self.argument_has_recovered_slot(argument_id);
                    argument_id
                }
                Err(error) => {
                    self.try_recover_in_item_list(&argument_start, terminator, Some(error))?;
                    let argument_id =
                        self.insert_node(Argument::Error, self.get_span_from(&argument_start));
                    is_recovered_argument = true;
                    argument_id
                }
            };

            self.eat_newlines_maybe()?;
            arguments.push(argument_id);

            // continue regular positional argument lists after a real separator
            if self.peek_is(TokenType::Comma) {
                self.eat_item_stop_with_newlines()?;

                if !is_recovered_argument {
                    continue;
                }

                if self
                    .should_end_recovered_argument_list_at_statement_boundary(is_recovered_argument)
                {
                    break;
                }

                if !self.can_continue_after_recovered_item(terminator, is_recovered_argument) {
                    break;
                }

                continue;
            }
            // recovered statement calls should stop before the next newline led statement
            // recovered slots may continue across newline separators only
            else if self
                .should_end_recovered_argument_list_at_statement_boundary(is_recovered_argument)
                || !self.can_continue_after_recovered_item(terminator, is_recovered_argument)
            {
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

            // one argument slot
            let argument_start = self.mark_span();
            let is_recovered_argument;
            let argument_id = match self.eat_tree_argument() {
                Ok(argument_id) => {
                    is_recovered_argument = self.argument_has_recovered_slot(argument_id);
                    argument_id
                }
                Err(error) => {
                    self.try_recover_in_item_list(&argument_start, terminator, Some(error))?;
                    let argument_id =
                        self.insert_node(Argument::Error, self.get_span_from(&argument_start));
                    is_recovered_argument = true;
                    argument_id
                }
            };

            self.eat_newlines_maybe()?;
            arguments.push(argument_id);

            // continue regular argument lists after a real separator
            if self.peek_is(TokenType::Comma) {
                self.eat_item_stop_with_newlines()?;

                if !is_recovered_argument {
                    continue;
                }

                if self
                    .should_end_recovered_argument_list_at_statement_boundary(is_recovered_argument)
                {
                    break;
                }

                if !self.can_continue_after_recovered_item(terminator, is_recovered_argument) {
                    break;
                }

                continue;
            }
            // recovered statement calls should stop before the next newline led statement
            // recovered slots may continue across newline separators only
            else if self
                .should_end_recovered_argument_list_at_statement_boundary(is_recovered_argument)
                || !self.can_continue_after_recovered_item(terminator, is_recovered_argument)
            {
                break;
            }
        }
        Ok(arguments.into_vec())
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        Argument, Asynchrony, ClassDeclaration, CommentKind, Declaration, Decorator,
        DecoratorPosition, Expression, FunctionDeclaration, FunctionMode, GenericArgument,
        GenericParameter, IfKind, IntType, Member, Name, NodeType, Parameter, Pattern,
        PatternField, ScalarLiteral, TokenType, TupleElement, TypeExpression, TypeLiteral,
        Visibility,
    };
    use destack_source::LanguageType;

    use crate::{
        TestParser, assert_comment, assert_expression_path, assert_name, assert_node, assert_path,
        assert_string,
    };

    /// Return the source text covered by one parser node span.
    fn span_text(source: &str, start: u32, end: u32) -> &str {
        &source[start as usize..end as usize]
    }

    #[test]
    fn test_parse_parameter_type_only() {
        // T
        let mut test = TestParser::new("T");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Named { name, declared_type, default, .. } => {
            assert_string!(parser, *name, "T");
            assert!(declared_type.is_none());
            assert!(default.is_none());
        });
    }

    #[test]
    fn test_parse_parameter_with_type() {
        // x: int32
        let mut test = TestParser::new("x: int32");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Named { name, declared_type, default, .. } => {
            assert_string!(parser, *name, "x");
            assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Literal { value: TypeLiteral::Int(IntType::Arbitrary {
                width: Some(32),
                is_signed: true
            }) });
            assert!(default.is_none());
        });
    }

    #[test]
    fn test_parse_parameter_with_maybe_type() {
        // x?: int32
        let mut test = TestParser::new("x?: int32");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Named { name, is_optional, declared_type: Some(declared_type), default: None, .. } => {
            assert_string!(parser, *name, "x");
            assert!(*is_optional);
            assert_node!(parser.tree, *declared_type, TypeExpression::Literal { value: TypeLiteral::Int(IntType::Arbitrary {
                width: Some(32),
                is_signed: true
            }) });
        });
    }

    #[test]
    fn test_parse_parameter_with_default() {
        // validate: boolean = false
        let mut test = TestParser::new("validate: boolean = false");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Named { name, declared_type, default, .. } => {
            assert_string!(parser, *name, "validate");
            assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Literal { value: TypeLiteral::Boolean });
            assert!(default.is_some());
        });
    }

    #[test]
    fn test_parse_parameter_missing_type_expression() {
        // x:
        let mut test = TestParser::new("x:");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Parameter), None, "")]);

        // x:
        assert_node!(parser.tree, parameter_id, Parameter::Named { name, declared_type: Some(declared_type), default: None, .. } => {
            assert_string!(parser, *name, "x");
            assert_node!(parser.tree, *declared_type, TypeExpression::Missing);
        });
    }

    #[test]
    fn test_parse_parameter_default_async_lambda_with_await_body() {
        let mut test = TestParser::new_with_options(
            "loadFonts: () => Promise<void> = async () => { await Fonts.loadElementsFonts(elements); }",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();

        // parameter default should parse as an async lambda value
        assert_node!(parser.tree, parameter_id, Parameter::Named { name, declared_type: Some(_), default: Some(default), .. } => {
            assert_string!(parser, *name, "loadFonts");
            assert_node!(parser.tree, *default, Expression::Declaration(default_declaration_id) => {
                assert_node!(parser.tree, *default_declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
                    assert_eq!(signature.asynchrony, Asynchrony::Async);
                    assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                        let block = parser.tree.get(*block_id);
                        assert_eq!(block.leading_expressions.len(), 1);
                        assert!(block.tail_expression.is_none());
                        assert_node!(parser.tree, block.leading_expressions[0], Expression::Await { expression } => {
                            assert_node!(parser.tree, *expression, Expression::Call { .. });
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
        assert_node!(parser.tree, parameter_id, Parameter::Pattern { pattern, declared_type: Some(declared_type), default: Some(default), .. } => {
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
            assert_node!(parser.tree, *declared_type, TypeExpression::Literal { value: TypeLiteral::Boolean });
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
        assert_node!(parser.tree, parameter_id, Parameter::Pattern { pattern, is_optional, .. } => {
            assert!(*is_optional);
            assert_node!(parser.tree, *pattern, Pattern::Array { .. } => {});
        });
    }

    #[test]
    fn test_parse_parameter_underscore_name() {
        // _ in TypeScript parameters is a normal name
        let mut test = TestParser::new_with_options("_", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Named { name, declared_type, default, .. } => {
            assert_string!(parser, *name, "_");
            assert!(declared_type.is_none());
            assert!(default.is_none());
        });
    }

    #[test]
    fn test_parse_parameter_variadic() {
        // ...args
        let mut test = TestParser::new("...args");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::VariadicNamed { name, declared_type, .. } => {
            assert_string!(parser, *name, "args");
            assert!(declared_type.is_none());
        });
    }

    #[test]
    fn test_parse_parameter_variadic_with_type() {
        // ...args: int32[]
        let mut test = TestParser::new("...args: int32[]");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::VariadicNamed { name, declared_type, .. } => {
            assert_string!(parser, *name, "args");
            assert!(declared_type.is_some());
        });
    }

    #[test]
    fn test_parse_parameter_optional_variadic() {
        // ...args? optional rest parameter
        let mut test = TestParser::new_with_options("...args?", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::VariadicNamed { name, .. } => {
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
        assert_node!(parser.tree, parameter_id, Parameter::VariadicNamed { name, declared_type, .. } => {
            assert_string!(parser, *name, "value");
            assert!(declared_type.is_some());
        });
    }

    #[test]
    fn test_parse_parameter_variadic_array_pattern() {
        // ...[first, second]
        let mut test = TestParser::new_with_options("...[first, second]", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::VariadicPattern { pattern, declared_type, .. } => {
            assert!(declared_type.is_none());
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
        assert_node!(parser.tree, parameter_id, Parameter::VariadicPattern { pattern, declared_type, .. } => {
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
            let declared_type = declared_type.expect("expected variadic tuple type annotation");
            assert_node!(parser.tree, declared_type, TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "ConstructorParameters");
                assert_eq!(generic_arguments.len(), 1);

                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                        assert_node!(parser.tree, *value, TypeExpression::TypeOfValue { value } => {
                            assert_expression_path!(parser, parser.tree.get(*value), "Response");
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
        assert_node!(parser.tree, parameter_id, Parameter::VariadicPattern { pattern, declared_type: Some(declared_type), .. } => {
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

                    assert_node!(parser.tree, *default, Expression::As { expression, target_type } => {
                        assert_node!(parser.tree, *expression, Expression::ObjectExpression { properties, .. } => {
                            assert!(properties.is_empty());
                        });
                        assert_node!(parser.tree, *target_type, TypeExpression::Literal { value: TypeLiteral::Any });
                    });
                });
            });

            // SpawnArguments<TContext, TExpressionEvent, TEvent, TActor>
            assert_node!(parser.tree, *declared_type, TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "SpawnArguments");
                assert_eq!(generic_arguments.len(), 4);
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
        assert_node!(parser.tree, parameter_id, Parameter::VariadicPattern { pattern, declared_type, .. } => {
            assert!(declared_type.is_none());
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
        assert_node!(parser.tree, parameter_id, Parameter::Named { name, declared_type: Some(declared_type), default: None, .. } => {
            assert_string!(parser, *name, "x");
            assert_node!(parser.tree, *declared_type, TypeExpression::Literal { value: TypeLiteral::Int(IntType::Arbitrary {
                width: Some(32),
                is_signed: true
            }) });
        });
    }

    #[test]
    fn test_parse_parameter_with_modifiers() {
        // private readonly const x: 1
        let mut test = TestParser::new("private readonly const x: 1");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();

        assert_node!(parser.tree, parameter_id, Parameter::Named { visibility, is_readonly, declared_type: Some(declared_type), .. } => {
            assert_eq!(*visibility, Some(Visibility::Private));
            assert!(*is_readonly);
            assert_node!(parser.tree, *declared_type, TypeExpression::ScalarLiteral { value: ScalarLiteral::Integer(1) });
        });
    }

    #[test]
    fn test_parse_generic_parameters_multiline_union_constraint_with_default() {
        let mut test = TestParser::new_with_options(
            r#"<
  Return extends ReturnType<onRequestHookHandler<RawServer>>
    | ReturnType<onRequestAsyncHookHandler<RawServer>>
    = ReturnType<onRequestHookHandler<RawServer>>
>"#,
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        let generic_parameters = parser.eat_generic_parameters(true).unwrap();

        // Return extends ReturnType<onRequestHookHandler<RawServer>> | ReturnType<onRequestAsyncHookHandler<RawServer>> = ReturnType<onRequestHookHandler<RawServer>>
        assert_eq!(generic_parameters.len(), 1);
        assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint: Some(constraint), default: Some(default), .. } => {
            assert_string!(parser, *name, "Return");

            // ReturnType<onRequestHookHandler<RawServer>> | ReturnType<onRequestAsyncHookHandler<RawServer>>
            assert_node!(parser.tree, *constraint, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 2);

                // ReturnType<onRequestHookHandler<RawServer>>
                assert_node!(parser.tree, elements[0], TypeExpression::Reference { path, generic_arguments } => {
                    assert_path!(parser, *path, "ReturnType");
                    assert_eq!(generic_arguments.len(), 1);

                    assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                            assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                                assert_path!(parser, *path, "onRequestHookHandler");
                                assert_eq!(generic_arguments.len(), 1);

                                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                                        assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                                            assert_path!(parser, *path, "RawServer");
                                            assert!(generic_arguments.is_empty());
                                        });
                                });
                            });
                    });
                });

                // ReturnType<onRequestAsyncHookHandler<RawServer>>
                assert_node!(parser.tree, elements[1], TypeExpression::Reference { path, generic_arguments } => {
                    assert_path!(parser, *path, "ReturnType");
                    assert_eq!(generic_arguments.len(), 1);

                    assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                            assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                                assert_path!(parser, *path, "onRequestAsyncHookHandler");
                                assert_eq!(generic_arguments.len(), 1);

                                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                                        assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                                            assert_path!(parser, *path, "RawServer");
                                            assert!(generic_arguments.is_empty());
                                        });
                                });
                            });
                    });
                });
            });

            // ReturnType<onRequestHookHandler<RawServer>>
            assert_node!(parser.tree, *default, TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "ReturnType");
                assert_eq!(generic_arguments.len(), 1);

                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                        assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                            assert_path!(parser, *path, "onRequestHookHandler");
                            assert_eq!(generic_arguments.len(), 1);
                            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                                    assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                                        assert_path!(parser, *path, "RawServer");
                                        assert!(generic_arguments.is_empty());
                                    });
                            });
                        });
                });
            });
        });
    }

    #[test]
    fn test_parse_generic_parameters_missing_close_angle() {
        // <T
        let mut test = TestParser::new("<T");
        let mut parser = test.prepare();
        let parameters = parser.eat_generic_parameters(true).unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

        // <T
        assert_eq!(parameters.len(), 1);
        assert_node!(parser.tree, parameters[0], GenericParameter::Type { name, constraint: None, default: None, .. } => {
            assert_string!(parser, *name, "T");
        });
    }

    #[test]
    fn test_parse_generic_arguments_missing_close_angle_in_type_context() {
        // <string, number
        let mut test = TestParser::new("<string, number");
        let mut parser = test.prepare();
        parser.options.set_in_type(true);
        let arguments = parser.eat_generic_arguments().unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

        // <string, number
        assert_eq!(arguments.len(), 2);
        assert_node!(parser.tree, arguments[0], GenericArgument::Type { value } => {
                assert_node!(parser.tree, *value, TypeExpression::Literal { value: TypeLiteral::String });
        });
        assert_node!(parser.tree, arguments[1], GenericArgument::Type { value } => {
                assert_node!(parser.tree, *value, TypeExpression::Literal { value: TypeLiteral::Number });
        });
    }

    #[test]
    fn test_parse_generic_arguments_empty_in_type_context_recovers_error_slot() {
        // <>
        let mut test = TestParser::new("<>");
        let mut parser = test.prepare();
        parser.options.set_in_type(true);
        let arguments = parser.eat_generic_arguments().unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(None, Some(TokenType::Identifier), "<")]);

        // <>
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], GenericArgument::Error);
    }

    #[test]
    fn test_parse_generic_arguments_first_value_with_boundary_comment() {
        let source = "<\n  // first-type-arg\n  string | number\n>";
        let mut test = TestParser::new_with_options(source, LanguageType::TypeScriptDeclaration);
        let mut parser = test.prepare();
        parser.options.set_in_type(true);
        let arguments = parser.eat_generic_arguments().unwrap();
        parser.attach_comments();

        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], GenericArgument::Type { value } => {
                assert_node!(parser.tree, *value, TypeExpression::Union { .. });
        });
        assert_eq!(parser.tree.comments().len(), 1);
        assert_comment!(parser, 0, CommentKind::Line, "first-type-arg");
    }

    #[test]
    fn test_parse_generic_arguments_following_value_with_boundary_comment() {
        let source = "<string,\n  // second-type-arg\n  number>";
        let mut test = TestParser::new_with_options(source, LanguageType::TypeScriptDeclaration);
        let mut parser = test.prepare();
        parser.options.set_in_type(true);
        let arguments = parser.eat_generic_arguments().unwrap();
        parser.attach_comments();

        assert_eq!(arguments.len(), 2);
        assert_node!(parser.tree, arguments[1], GenericArgument::Type { value } => {
                assert_node!(parser.tree, *value, TypeExpression::Literal { value: TypeLiteral::Number });
        });
        assert_eq!(parser.tree.comments().len(), 1);
        assert_comment!(parser, 0, CommentKind::Line, "second-type-arg");
    }

    #[test]
    fn test_reject_generic_arguments_missing_close_angle_in_value_context() {
        // <string, number
        let mut test = TestParser::new_with_options("<string, number", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let result = parser.eat_generic_arguments();

        assert!(result.is_err());
    }

    #[test]
    fn test_reject_spread_generic_argument() {
        // <...T>
        let mut test = TestParser::new_with_options("<...T>", LanguageType::Destack);
        let mut parser = test.prepare();
        let arguments = parser.eat_generic_arguments().unwrap();

        test.assert_error_leaves(&parser, &[(None, None, "...")]);

        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], GenericArgument::Error);
    }

    #[test]
    fn test_parse_parameter_with_readonly_public_modifier_order_reports_error() {
        // readonly public x: number
        let mut test =
            TestParser::new_with_options("readonly public x: number", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(None, None, "public")]);

        // readonly public x: number
        assert_node!(parser.tree, parameter_id, Parameter::Named { visibility, is_readonly, name, declared_type: Some(declared_type), default: None, .. } => {
            assert_string!(parser, *name, "x");
            assert_eq!(*visibility, Some(Visibility::Public));
            assert!(*is_readonly);
            assert_node!(parser.tree, *declared_type, TypeExpression::Literal { value: TypeLiteral::Number });
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

        // diagnostics
        test.assert_error_leaves(&parser, &[(None, None, "public")]);

        // class D { constructor(readonly public x: number) {} }
        assert_eq!(expressions.len(), 1);
        let expression_id = parser.unwrap_labelled_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { members, .. }) => {
                assert_eq!(members.len(), 1);

                assert_node!(parser.tree, members[0], Member::Method { signature, .. } => {
                    assert_eq!(signature.mode, Some(FunctionMode::Constructor));
                    assert_eq!(signature.parameters.len(), 1);

                    assert_node!(parser.tree, signature.parameters[0], Parameter::Named { visibility, is_readonly, name, declared_type: Some(declared_type), default: None, .. } => {
                        assert_string!(parser, *name, "x");
                        assert_eq!(*visibility, Some(Visibility::Public));
                        assert!(*is_readonly);
                        assert_node!(parser.tree, *declared_type, TypeExpression::Literal { value: TypeLiteral::Number });
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
        assert_node!(parser.tree, parameter_id, Parameter::Named { name, declared_type: Some(declared_type), default: None, .. } => {
            assert_string!(parser, *name, "readonly");
            assert_node!(parser.tree, *declared_type, TypeExpression::Literal { value: TypeLiteral::Int(IntType::Arbitrary {
                width: Some(32),
                is_signed: true
            }) });
        });
    }

    #[test]
    fn test_parse_parameter_comptime() {
        // comptime n: int32
        let mut test = TestParser::new("comptime n: int32");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Named { name, declared_type: Some(declared_type), .. } => {
            assert_string!(parser, *name, "n");
            assert_node!(parser.tree, *declared_type, TypeExpression::Literal { value: TypeLiteral::Int(IntType::Arbitrary {
                width: Some(32),
                is_signed: true
            }) });
        });
    }

    /// Parse TypeScript parameter decorators in constructors and methods.
    #[test]
    fn test_parse_parameter_decorators() {
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
        let expression_id = parser.unwrap_labelled_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { members, .. }) => {
                assert_eq!(members.len(), 2);

                // constructor(@p1 t1, @p2 t2, @p3 ...t3)
                assert_node!(parser.tree, members[0], Member::Method { signature, .. } => {
                    assert_eq!(signature.mode, Some(FunctionMode::Constructor));
                    assert_eq!(signature.parameters.len(), 3);

                    // @p1 t1
                    assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: None, default: None, .. } => {
                        assert_string!(parser, *name, "t1");
                    });
                    let t1_annotations = parser.tree.get_decorators(signature.parameters[0].id);
                    assert_eq!(t1_annotations.len(), 1);
                    assert_node!(parser.tree, t1_annotations[0], Decorator { expression, position } => {
                        assert_eq!(*position, DecoratorPosition::BlockPrefix);
                        assert_expression_path!(parser, parser.tree.get(*expression), "p1");
                    });

                    // @p2 t2
                    assert_node!(parser.tree, signature.parameters[1], Parameter::Named { visibility, name, declared_type: None, default: None, .. } => {
                        assert_string!(parser, *name, "t2");
                        assert_eq!(*visibility, Some(Visibility::Private));
                    });
                    let t2_annotations = parser.tree.get_decorators(signature.parameters[1].id);
                    assert_eq!(t2_annotations.len(), 1);
                    assert_node!(parser.tree, t2_annotations[0], Decorator { expression, position } => {
                        assert_eq!(*position, DecoratorPosition::BlockPrefix);
                        assert_expression_path!(parser, parser.tree.get(*expression), "p2");
                    });

                    // @p3 ...t3
                    assert_node!(parser.tree, signature.parameters[2], Parameter::VariadicNamed { name, declared_type: None, .. } => {
                        assert_string!(parser, *name, "t3");
                    });
                    let t3_annotations = parser.tree.get_decorators(signature.parameters[2].id);
                    assert_eq!(t3_annotations.len(), 1);
                    assert_node!(parser.tree, t3_annotations[0], Decorator { expression, position } => {
                        assert_eq!(*position, DecoratorPosition::BlockPrefix);
                        assert_expression_path!(parser, parser.tree.get(*expression), "p3");
                    });
                });

                // method(@p1 t1, @p2 ...t2)
                assert_node!(parser.tree, members[1], Member::Method { signature, .. } => {
                    assert_eq!(signature.mode, None);
                    assert_eq!(signature.parameters.len(), 2);

                    // @p1 t1
                    assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: None, default: None, .. } => {
                        assert_string!(parser, *name, "t1");
                    });
                    let method_t1_annotations = parser.tree.get_decorators(signature.parameters[0].id);
                    assert_eq!(method_t1_annotations.len(), 1);
                    assert_node!(parser.tree, method_t1_annotations[0], Decorator { expression, position } => {
                        assert_eq!(*position, DecoratorPosition::BlockPrefix);
                        assert_expression_path!(parser, parser.tree.get(*expression), "p1");
                    });

                    // @p2 ...t2
                    assert_node!(parser.tree, signature.parameters[1], Parameter::VariadicNamed { name, declared_type: None, .. } => {
                        assert_string!(parser, *name, "t2");
                    });
                    let method_t2_annotations = parser.tree.get_decorators(signature.parameters[1].id);
                    assert_eq!(method_t2_annotations.len(), 2);
                    assert_node!(parser.tree, method_t2_annotations[0], Decorator { expression, position } => {
                        assert_eq!(*position, DecoratorPosition::BlockPrefix);
                        assert_expression_path!(parser, parser.tree.get(*expression), "p1");
                    });
                    assert_node!(parser.tree, method_t2_annotations[1], Decorator { expression, position } => {
                        assert_eq!(*position, DecoratorPosition::BlockPrefix);
                        assert_expression_path!(parser, parser.tree.get(*expression), "p2");
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
        assert_node!(parser.tree, argument_id, Argument::Named { name: Name::Identifier(name), value } => {
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
        assert_node!(parser.tree, argument_id, Argument::Named { name: Name::String(name), value } => {
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
        assert_node!(parser.tree, argument_id, Argument::Named { name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "title");
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string)) => {
                assert_string!(parser, *string, "hello");
            });
        });
    }

    #[test]
    fn test_parse_named_argument_with_newline_before_assign_before_tree() {
        let mut test = TestParser::new_with_options(
            "onBroadcastSelected\n    = { this._onYouTubeBroadcastIDSelected }",
            LanguageType::TypeScriptXml,
        );
        let mut parser = test.prepare();
        let argument_id = parser.eat_tree_literal_argument().unwrap();

        assert_node!(parser.tree, argument_id, Argument::Named { name: Name::Identifier(name), value } => {
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
        assert_node!(parser.tree, argument_id, Argument::Named { name: Name::Identifier(name), value } => {
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
        assert_node!(parser.tree, argument_id, Argument::Named { name: Name::Identifier(name), value } => {
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

        assert_node!(parser.tree, argument_id, Argument::Positional { value } => {
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
    fn test_parse_dynamic_parameters_recover_error_slot() {
        // (x, =, y)
        let mut test = TestParser::new("(x, =, y)");
        let mut parser = test.prepare();
        let parameters = parser.eat_dynamic_parameters().unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Parameter), None, "=")]);

        // (x, =, y)
        assert_eq!(parameters.len(), 3);
        assert_node!(parser.tree, parameters[0], Parameter::Named { name, declared_type: None, default: None, .. } => {
            assert_string!(parser, *name, "x");
        });
        assert_node!(parser.tree, parameters[1], Parameter::Error);
        assert_node!(parser.tree, parameters[2], Parameter::Named { name, declared_type: None, default: None, .. } => {
            assert_string!(parser, *name, "y");
        });
    }

    #[test]
    fn test_parse_dynamic_arguments_recover_error_slot() {
        // (1, , 3)
        let mut test = TestParser::new("(1, , 3)");
        let mut parser = test.prepare();
        let arguments = parser.eat_dynamic_arguments().unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(None, None, ",")]);

        // (1, , 3)
        assert_eq!(arguments.len(), 3);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
        assert_node!(parser.tree, arguments[1], Argument::Error);
        assert_node!(parser.tree, arguments[2], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
        });
    }

    #[test]
    fn test_parse_dynamic_arguments_recover_missing_close_before_next_statement() {
        // (a,b var
        let source = "(a,b var";
        let mut test = TestParser::new(source);
        let mut parser = test.prepare();
        let arguments = parser.eat_dynamic_arguments().unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "var")]);

        // (a,b var
        assert_eq!(arguments.len(), 2);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_expression_path!(parser, parser.tree.get(*value), "a");
        });
        assert_node!(parser.tree, arguments[1], Argument::Positional { value, .. } => {
            assert_expression_path!(parser, parser.tree.get(*value), "b");

            // `b`
            let argument_span = parser.tree.get_span(arguments[1]);
            let value_span = parser.tree.get_span(*value);

            assert_eq!(span_text(source, argument_span.start, argument_span.end), "b");
            assert_eq!(span_text(source, value_span.start, value_span.end), "b");
        });

        // the next statement starter stays for the caller
        assert!(parser.peek_is(TokenType::Identifier));
    }

    #[test]
    fn test_parse_dynamic_arguments_recover_trailing_spread_error_slot() {
        // (a, ...)
        let mut test = TestParser::new("(a, ...)");
        let mut parser = test.prepare();
        let arguments = parser.eat_dynamic_arguments().unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(None, None, ")")]);

        // (a, ...)
        assert_eq!(arguments.len(), 2);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_expression_path!(parser, parser.tree.get(*value), "a");
        });
        assert_node!(parser.tree, arguments[1], Argument::Error);
    }

    #[test]
    fn test_parse_dynamic_arguments_recover_missing_close_before_semicolon() {
        // (a,b;
        let source = "(a,b;";
        let mut test = TestParser::new(source);
        let mut parser = test.prepare();
        let arguments = parser.eat_dynamic_arguments().unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, ";")]);

        // (a,b;
        assert_eq!(arguments.len(), 2);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_expression_path!(parser, parser.tree.get(*value), "a");
        });
        assert_node!(parser.tree, arguments[1], Argument::Positional { value, .. } => {
            assert_expression_path!(parser, parser.tree.get(*value), "b");

            // `b`
            let argument_span = parser.tree.get_span(arguments[1]);
            let value_span = parser.tree.get_span(*value);

            assert_eq!(span_text(source, argument_span.start, argument_span.end), "b");
            assert_eq!(span_text(source, value_span.start, value_span.end), "b");
        });

        // the semicolon stays for the caller
        assert!(parser.peek_is(TokenType::Semicolon));
    }

    #[test]
    fn test_parse_dynamic_arguments_recover_leading_empty_slots() {
        // (,,b)
        let mut test = TestParser::new("(,,b)");
        let mut parser = test.prepare();
        let arguments = parser.eat_dynamic_arguments().unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(None, None, ","), (None, None, ",")]);

        // (,,b)
        assert_eq!(arguments.len(), 3);
        assert_node!(parser.tree, arguments[0], Argument::Error);
        assert_node!(parser.tree, arguments[1], Argument::Error);
        assert_node!(parser.tree, arguments[2], Argument::Positional { value, .. } => {
            assert_expression_path!(parser, parser.tree.get(*value), "b");
        });
    }

    #[test]
    fn test_parse_malformed_call_statement_missing_close_keeps_call_shape() {
        let mut test = TestParser::new_with_options("foo(a,b;", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, ";")]);

        // foo(a,b;
        assert_eq!(expressions.len(), 1);
        let call_id = parser.unwrap_labelled_expression(expressions[0]);
        assert_node!(parser.tree, call_id, Expression::Call { dynamic_arguments, .. } => {
            assert_eq!(dynamic_arguments.len(), 2);
        });
    }

    #[test]
    fn test_parse_malformed_call_statement_before_var_keeps_call_shape() {
        let mut test = TestParser::new_with_options("foo(a,b var;", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // diagnostics
        test.assert_error_leaves(
            &parser,
            &[
                (Some(NodeType::Expression), None, "var"),
                (None, None, "var"),
                (Some(NodeType::Expression), None, ";"),
            ],
        );

        // foo(a,b var;
        assert_eq!(expressions.len(), 2);
        let call_id = parser.unwrap_labelled_expression(expressions[0]);
        assert_node!(parser.tree, call_id, Expression::Call { dynamic_arguments, .. } => {
            assert_eq!(dynamic_arguments.len(), 2);
        });
        assert_node!(parser.tree, expressions[1], Expression::Error);
    }

    #[test]
    fn test_parse_malformed_call_statement_with_leading_empty_slots_keeps_call_shape() {
        let mut test = TestParser::new_with_options("foo (,,b);", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // diagnostics
        test.assert_error_leaves(&parser, &[(None, None, ","), (None, None, ",")]);

        // foo (,,b);
        assert_eq!(expressions.len(), 1);
        let call_id = parser.unwrap_labelled_expression(expressions[0]);
        assert_node!(parser.tree, call_id, Expression::Call { dynamic_arguments, .. } => {
            assert_eq!(dynamic_arguments.len(), 3);
            assert_node!(parser.tree, dynamic_arguments[0], Argument::Error);
            assert_node!(parser.tree, dynamic_arguments[1], Argument::Error);
        });
    }

    #[test]
    fn test_parse_malformed_call_statement_with_trailing_spread_keeps_call_shape() {
        let mut test = TestParser::new_with_options("foo (a, ...);", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // diagnostics
        test.assert_error_leaves(&parser, &[(None, None, ")")]);

        // foo (a, ...);
        assert_eq!(expressions.len(), 1);
        let call_id = parser.unwrap_labelled_expression(expressions[0]);
        assert_node!(parser.tree, call_id, Expression::Call { dynamic_arguments, .. } => {
            assert_eq!(dynamic_arguments.len(), 2);
            assert_node!(parser.tree, dynamic_arguments[1], Argument::Error);
        });
    }

    #[test]
    fn test_parse_malformed_call_before_empty_slots_call_preserves_following_statement_shape() {
        let source = r#"
foo(a,b var;
foo (,,b);
"#;
        let mut test = TestParser::new_with_options(source, LanguageType::JavaScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // diagnostics
        test.assert_error_leaves(
            &parser,
            &[
                (Some(NodeType::Expression), None, "var"),
                (None, None, "var"),
                (Some(NodeType::Expression), None, ";"),
                (None, None, ","),
                (None, None, ","),
            ],
        );

        // foo(a,b var;
        // Error
        // foo (,,b);
        assert_eq!(expressions.len(), 3);

        let first_call_id = parser.unwrap_labelled_expression(expressions[0]);
        assert_node!(parser.tree, first_call_id, Expression::Call { dynamic_arguments, .. } => {
            assert_eq!(dynamic_arguments.len(), 2);
        });

        assert_node!(parser.tree, expressions[1], Expression::Error);

        let second_call_id = parser.unwrap_labelled_expression(expressions[2]);
        assert_node!(parser.tree, second_call_id, Expression::Call { dynamic_arguments, .. } => {
            assert_eq!(dynamic_arguments.len(), 3);
        });
    }

    #[test]
    fn test_parse_malformed_call_before_trailing_spread_call_preserves_following_statement_shape() {
        let source = r#"
foo(a,b var;
foo (a, ...);
"#;
        let mut test = TestParser::new_with_options(source, LanguageType::JavaScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // diagnostics
        test.assert_error_leaves(
            &parser,
            &[
                (Some(NodeType::Expression), None, "var"),
                (None, None, "var"),
                (Some(NodeType::Expression), None, ";"),
                (None, None, ")"),
            ],
        );

        assert_eq!(expressions.len(), 3);

        // foo(a,b var;
        let first_call_id = parser.unwrap_labelled_expression(expressions[0]);
        assert_node!(parser.tree, first_call_id, Expression::Call { dynamic_arguments, .. } => {
            assert_eq!(dynamic_arguments.len(), 2);
        });

        // Error
        assert_node!(parser.tree, expressions[1], Expression::Error);

        // foo (a, ...);
        let second_call_id = parser.unwrap_labelled_expression(expressions[2]);
        assert_node!(parser.tree, second_call_id, Expression::Call { dynamic_arguments, .. } => {
            assert_eq!(dynamic_arguments.len(), 2);
        });
    }

    #[test]
    fn test_parse_malformed_call_with_empty_slot_before_following_call_keeps_statement_shape() {
        let source = r#"
foo(,
bar();
"#;
        let mut test = TestParser::new_with_options(source, LanguageType::JavaScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // diagnostics
        test.assert_error_leaves(
            &parser,
            &[(None, None, ","), (Some(NodeType::Expression), None, "bar")],
        );

        assert_eq!(expressions.len(), 2);

        // foo(,
        let first_call_id = parser.unwrap_labelled_expression(expressions[0]);
        assert_node!(parser.tree, first_call_id, Expression::Call { dynamic_arguments, .. } => {
            assert_eq!(dynamic_arguments.len(), 1);
            assert_node!(parser.tree, dynamic_arguments[0], Argument::Error);
        });

        // bar();
        let second_call_id = parser.unwrap_labelled_expression(expressions[1]);
        assert_node!(parser.tree, second_call_id, Expression::Call { dynamic_arguments, .. } => {
            assert_eq!(dynamic_arguments.len(), 0);
        });
    }

    #[test]
    fn test_parse_malformed_call_with_empty_slot_before_following_const_keeps_statement_shape() {
        let source = r#"
foo(,
const value = 1;
"#;
        let mut test = TestParser::new_with_options(source, LanguageType::JavaScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // diagnostics
        test.assert_error_leaves(
            &parser,
            &[
                (None, None, ","),
                (Some(NodeType::Expression), None, "const"),
            ],
        );

        assert_eq!(expressions.len(), 2);

        // foo(,
        let first_call_id = parser.unwrap_labelled_expression(expressions[0]);
        assert_node!(parser.tree, first_call_id, Expression::Call { dynamic_arguments, .. } => {
            assert_eq!(dynamic_arguments.len(), 1);
            assert_node!(parser.tree, dynamic_arguments[0], Argument::Error);
        });

        // const value = 1;
        let second_expression_id = parser.unwrap_labelled_expression(expressions[1]);
        assert_node!(parser.tree, second_expression_id, Expression::Let { declarators, .. } => {
            assert_eq!(declarators.len(), 1);
        });
    }

    #[test]
    fn test_parse_spread_argument() {
        // ...args
        let mut test = TestParser::new("...args");
        let mut parser = test.prepare();
        let argument_id = parser.eat_positional_argument().unwrap();
        assert_node!(parser.tree, argument_id, Argument::Spread { label, value } => {
            // ...args
            assert!(label.is_none());
            assert_node!(parser.tree, *value, Expression::Identifier { name } => {
                assert_string!(parser, *name, "args");
            });
        });
    }

    #[test]
    fn test_parse_spread_argument_with_doc_block_comment_newline() {
        // .../** comment */\nargs
        let mut test =
            TestParser::new_with_options(".../** comment */\nargs", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let argument_id = parser.eat_positional_argument().unwrap();

        assert_node!(parser.tree, argument_id, Argument::Spread { label, value } => {
            assert!(label.is_none());
            assert_node!(parser.tree, *value, Expression::Identifier { name } => {
                assert_string!(parser, *name, "args");
            });
        });
    }

    #[test]
    fn test_parse_type_tuple_spread_label_element() {
        let mut test = TestParser::new("[...args: number]");
        let mut parser = test.prepare();
        parser.eat_token(TokenType::OpenBracket).unwrap();
        let elements = parser
            .eat_type_tuple_elements_body(TokenType::CloseBracket)
            .unwrap();

        assert_eq!(elements.len(), 1);
        assert_node!(parser.tree, elements[0], TupleElement::Spread { label, value } => {
            assert_string!(parser, label.unwrap(), "args");
            assert_node!(parser.tree, *value, TypeExpression::Literal { value: TypeLiteral::Number });
        });
    }

    #[test]
    fn test_parse_type_tuple_label_element_span() {
        let mut test = TestParser::new("[label: number]");
        let mut parser = test.prepare();
        parser.eat_token(TokenType::OpenBracket).unwrap();
        let elements = parser
            .eat_type_tuple_elements_body(TokenType::CloseBracket)
            .unwrap();

        assert_eq!(elements.len(), 1);
        assert_node!(parser.tree, elements[0], TupleElement::Element { label, value, is_optional, is_readonly } => {
            assert_eq!(*is_optional, false);
            assert_eq!(*is_readonly, false);
            assert_string!(parser, label.unwrap(), "label");
            assert_node!(parser.tree, *value, TypeExpression::Literal { value: TypeLiteral::Number });
        });
    }

    #[test]
    fn test_parse_type_tuple_label_element_multiline_union_type() {
        let mut test = TestParser::new("[options?:\n  | SkipToken\n  | OtherOption]");
        let mut parser = test.prepare();
        parser.eat_token(TokenType::OpenBracket).unwrap();
        let elements = parser
            .eat_type_tuple_elements_body(TokenType::CloseBracket)
            .unwrap();

        assert_eq!(elements.len(), 1);
        assert_node!(parser.tree, elements[0], TupleElement::Element { label, value, is_optional, is_readonly } => {
            assert_eq!(*is_optional, true);
            assert_eq!(*is_readonly, false);
            assert_string!(parser, label.unwrap(), "options");
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 2);
                assert_expression_path!(parser, parser.tree.get(elements[0]), "SkipToken");
                assert_expression_path!(parser, parser.tree.get(elements[1]), "OtherOption");
            });
        });
    }

    #[test]
    fn test_parse_generic_arguments_with_nested_generics_and_union() {
        let mut test = TestParser::new_with_options(
            "<keyof ServerReservedEventsMap<never, never, never, never> | keyof NamespaceReservedEventsMap<never, never, never, never>>",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let generic_arguments = parser.eat_generic_arguments().unwrap();

        assert_eq!(generic_arguments.len(), 1);
        assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                    assert_eq!(elements.len(), 2);
                    assert!(matches!(parser.tree.get(elements[0]), TypeExpression::KeyOf { .. }));
                    assert!(matches!(parser.tree.get(elements[1]), TypeExpression::KeyOf { .. }));
                });
        });
    }
}
