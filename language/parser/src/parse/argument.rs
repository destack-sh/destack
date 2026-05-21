use destack_dir::{
    Argument, Expression, GenericArgument, GenericParameter, Keyword, LocalNodeId,
    MethodAbstraction, Name, NodeType, Parameter, Pattern, ScalarLiteral, StringId, TokenLiteral,
    TokenSpan, TokenType, TypeExpression, VarianceModifier, Visibility,
};
use destack_source::{NodeSpanRegion, NodeSpanType, Span};

use crate::lex::decode_html_entities;
use crate::parse::flags::ParserFlags;
use crate::parse::mode::ContextualLexMode;
use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserSpanStart};

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
    /// Whether `virtual` was present.
    pub is_virtual: bool,
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

    /// Return the method abstraction represented by these modifiers.
    pub(crate) fn method_abstraction(self) -> MethodAbstraction {
        if self.is_abstract {
            MethodAbstraction::Abstract
        } else if self.is_virtual {
            MethodAbstraction::Virtual
        } else {
            MethodAbstraction::Concrete
        }
    }
}

impl Parser {
    /// Eat a generic argument close token or recover one missing `>`.
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
        let is_recoverable_boundary =
            // allow end and generic closers
            token_type == TokenType::End
                || self.flags.is_in_static() && Self::starts_type_angle_close(token_type)

                // allow ternary and arrow continuations
                || self.flags.is_in_ternary_condition() && token_type == TokenType::Colon
                || self.flags.is_in_type()
                    && matches!(token_type, TokenType::ArrowWide)

                // allow stops and delimiters
                || self.current_token_is_on_new_line()
                || matches!(
                    token_type,
                    TokenType::Comma
                        | TokenType::Semicolon
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
                || self.flags.is_in_super_type()
                    && (token_type == TokenType::OpenBrace
                        || token_type == TokenType::Identifier
                            && matches!(
                                self.current_keyword(),
                                Some(Keyword::Implements | Keyword::With | Keyword::Where)
                            ))

                // allow operator continuations
                || self.current_token_can_start_infix_or_assign_operator();

        self.recover_missing_token_here(TokenType::GreaterThan, owner, is_recoverable_boundary)
    }

    /// Return the common flags for non-sequence argument values.
    #[inline]
    fn argument_value_flags(&self) -> ParserFlags {
        self.flags
            .not_in_sequence_expression()
            .not_in_arrow_return_type()
    }

    /// Return whether the current token ends one generic argument.
    #[inline]
    fn generic_argument_has_boundary(&mut self) -> bool {
        if self.peek_is(TokenType::Comma) || self.peek_starts_type_angle_close() {
            return true;
        }

        // allow one final type argument before one missing close angle at eof
        if self.peek_is(TokenType::End) {
            return true;
        }

        false
    }

    /// Return whether the current generic argument parses cleanly as one type.
    fn generic_argument_parses_as_type(&mut self, context: ParserFlags) -> bool {
        // value contexts parse unmarked arguments as expressions
        if !self.flags.is_in_type() {
            return false;
        }

        // parse one type and require it to own the whole argument
        let speculative_start = self.checkpoint();
        let speculative_start_idx = self.tree.next_id();
        let parsed_type_expression =
            self.with_flags(context, |parser| parser.eat_type_expression());
        let type_expression_id = parsed_type_expression.ok();
        let prefers_type_expression = type_expression_id.is_some_and(|type_expression_id| {
            self.generic_argument_allows_implicit_type_marker(type_expression_id)
        }) && self.generic_argument_has_boundary();

        self.restore(speculative_start, speculative_start_idx);

        prefers_type_expression
    }

    /// Return whether one type generic argument can omit its `type` marker.
    fn generic_argument_allows_implicit_type_marker(
        &self,
        type_expression_id: LocalNodeId<TypeExpression>,
    ) -> bool {
        // typescript generic arguments are always types
        if !self.language.is_destack() {
            return true;
        }

        // destack object shaped types overlap with value literals
        !matches!(
            self.tree.get(type_expression_id),
            TypeExpression::Object { .. } | TypeExpression::Mapped { .. }
        )
    }

    /// Return whether the current generic argument starts as an unambiguous type.
    fn generic_argument_starts_unambiguous_type(&mut self) -> bool {
        if !self.flags.is_in_type() {
            return false;
        }

        // typescript has no value generic arguments
        if !self.language.is_destack() {
            return true;
        }

        // destack `type T` is an explicit marker, not a direct reference
        if self.current_keyword() == Some(Keyword::Type) {
            return false;
        }

        // destack keeps object-shaped arguments ambiguous without `type`
        let token_type = self.peek_token_type();
        if token_type == TokenType::OpenBrace {
            return false;
        }

        // simple type names and type paths dominate real generic type traffic
        if matches!(
            token_type,
            TokenType::Identifier | TokenType::Literal | TokenType::OpenParenthesis
        ) {
            return self.generic_argument_plain_type_has_boundary();
        }

        matches!(
            self.current_keyword(),
            Some(
                Keyword::Keyof
                    | Keyword::Readonly
                    | Keyword::Infer
                    | Keyword::Import
                    | Keyword::Typeof
            )
        )
    }

    /// Return whether a plain type starter is followed by a type boundary.
    fn generic_argument_plain_type_has_boundary(&mut self) -> bool {
        matches!(
            self.token_type_at_offset(1),
            TokenType::Comma
                | TokenType::Dot
                | TokenType::LessThan
                | TokenType::OpenBracket
                | TokenType::Maybe
                | TokenType::Not
                | TokenType::GreaterThan
                | TokenType::ShiftRight
                | TokenType::UnsignedShiftRight
        )
    }

    /// Eat one mixed generic argument node.
    fn eat_generic_argument(
        &mut self,
        context: ParserFlags,
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<GenericArgument>> {
        let is_spread = self.peek_is(TokenType::Spread);
        if is_spread && !self.language.is_destack() {
            return Err(ParseError::unexpected(self.peek()?.span));
        }
        if is_spread {
            self.eat_token(TokenType::Spread)?;
        }

        // parse obvious type arguments directly
        if self.generic_argument_starts_unambiguous_type() {
            let value = self.eat_type_expression_in_flags(context)?;
            let argument = if is_spread {
                GenericArgument::SpreadType { value }
            } else {
                GenericArgument::Type { value }
            };

            return Ok(self.insert_node(argument, self.get_span_from(start)));
        }

        // explicit type-space argument
        if self.is_keyword(Keyword::Type) {
            self.eat_keyword(Keyword::Type)?;
            let value = self.eat_type_expression_in_flags(context.with_type(true))?;
            let argument = if is_spread {
                GenericArgument::SpreadType { value }
            } else {
                GenericArgument::Type { value }
            };

            return Ok(self.insert_node(argument, self.get_span_from(start)));
        }

        // parse ambiguous arguments as types only when the whole argument is type shaped
        if self.generic_argument_parses_as_type(context) {
            let value = self.eat_type_expression_in_flags(context)?;
            let argument = if is_spread {
                GenericArgument::SpreadType { value }
            } else {
                GenericArgument::Type { value }
            };

            return Ok(self.insert_node(argument, self.get_span_from(start)));
        }

        // otherwise parse the argument in value space
        let value_ambient_context = self.flags.with_type(false);
        let value = self.eat_expression(
            self.flags
                .with_ambient_context(value_ambient_context)
                .with_expression_context(context),
        )?;
        let argument = if is_spread {
            GenericArgument::SpreadValue { value }
        } else {
            GenericArgument::Value { value }
        };

        Ok(self.insert_node(argument, self.get_span_from(start)))
    }

    /// Return the common flags for positional argument values.
    #[inline]
    fn positional_argument_flags(&self) -> ParserFlags {
        self.argument_value_flags().not_in_position()
    }

    /// Return true when the current keyword should end a malformed parameter list after a newline.
    fn current_keyword_starts_parameter_recovery_boundary(&mut self) -> bool {
        // only statement scoped dynamic parameters should stop at newline led keywords
        if self.flags.is_in_static() || !self.flags.is_in_statement_context() {
            return false;
        }

        // only newline led keywords can start a following statement
        if !self.current_token_is_on_new_line() {
            return false;
        }

        // parameter heads win when followed by a parameter continuation
        if matches!(
            self.next_token_type(),
            TokenType::Assign
                | TokenType::CloseParenthesis
                | TokenType::Colon
                | TokenType::Comma
                | TokenType::Maybe
        ) {
            return false;
        }

        // statement keywords
        let Some(keyword) = self.current_keyword() else {
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
                | Keyword::Return
                | Keyword::Switch
                | Keyword::Throw
                | Keyword::Type
                | Keyword::Try
                | Keyword::Using
                | Keyword::While
        )
    }

    /// Return true when one recovered argument list should stop at the current statement boundary.
    fn should_end_recovered_argument_list_at_statement_boundary(
        &mut self,
        is_recovered_argument: bool,
    ) -> bool {
        is_recovered_argument
            && self.flags.is_in_statement_context()
            && self.current_token_is_on_new_line()
    }

    /// Return true when one parsed argument was recovered as missing or malformed.
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
        let ambient_context = self.flags.with_type(true);
        let mut expression_context = self.flags.not_in_position();
        if self.flags.is_in_type_conditional_right() {
            expression_context = expression_context.in_type_conditional_right();
        }
        self.eat_type_expression_or_recover_missing(
            self.flags
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
            NodeType::Parameter,
        )
    }

    /// Parse a parameter default value expression.
    #[inline]
    fn eat_parameter_default_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let ambient_context = self
            .flags
            .with_type(self.flags.is_in_static())
            .with_forbid_await(true)
            .with_variant(false);
        let expression_context = self.positional_argument_flags();
        self.eat_expression(
            self.flags
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
        )
    }

    /// Return true when the next token can start a member name.
    pub(crate) fn next_token_starts_member_name(&mut self) -> bool {
        let next_token = self.next_token();

        self.token_starts_member_name(next_token)
    }

    /// Return true when the next same-line token can start a member name.
    pub(crate) fn next_same_line_token_starts_member_name(&mut self) -> bool {
        let next_token = self.next_token();
        if next_token.token.is_on_new_line {
            return false;
        }

        self.token_starts_member_name(next_token)
    }

    /// Return true when one token can start a member name.
    fn token_starts_member_name(&self, token: TokenSpan) -> bool {
        // check for common member name starters
        if matches!(
            token.token.ty,
            TokenType::Identifier
                | TokenType::Hash
                | TokenType::OpenBracket
                | TokenType::OpenBrace
                | TokenType::Spread
                | TokenType::Multiply
        ) {
            return true;
        }
        if token.token.ty != TokenType::Literal {
            return false;
        }

        // check for valid literal member names
        matches!(
            token.token.literal,
            Some(TokenLiteral::String {
                is_terminated: true,
                has_invalid_escape: false,
            }) | Some(TokenLiteral::Boolean { .. })
                | Some(TokenLiteral::Int { .. })
                | Some(TokenLiteral::Float { .. })
        )
    }

    /// Eat a binding modifiers prefix when present.
    pub(crate) fn eat_binding_modifiers_prefix_maybe(
        &mut self,
        allow_readonly_key: bool,
        allow_accessor_modifier: bool,
        allow_virtual_modifier: bool,
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
            let current_keyword = self.current_keyword();
            let is_out_variance_modifier = allow_variance_modifier
                && self.current_identifier_str_is("out")
                && !self.next_token().token.is_on_new_line
                && (self.token_type_at_offset(1) == TokenType::Identifier
                    || self.keyword_at_offset(1) == Some(Keyword::In));
            let can_start_modifier = current_keyword.is_some_and(|keyword| {
                let is_standard_modifier = matches!(
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
                );
                let is_virtual_modifier = allow_virtual_modifier
                    && self.language.is_destack()
                    && keyword == Keyword::Virtual;
                let is_comptime_modifier =
                    self.language.is_destack() && keyword == Keyword::Comptime;

                is_standard_modifier || is_virtual_modifier || is_comptime_modifier
            }) || is_out_variance_modifier;
            if !can_start_modifier {
                break;
            }

            let mut progress = false;

            // modifier disambiguation for abstraction keywords
            let abstraction_is_modifier = !matches!(
                self.token_type_at_offset(1),
                TokenType::Colon | TokenType::Maybe | TokenType::LessThan
            );

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
            if let Some(visibility) = self.peek_visibility_is() {
                if !self.next_same_line_token_starts_member_name() {
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
                if !self.next_same_line_token_starts_member_name() {
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
                if !self.next_same_line_token_starts_member_name() {
                    break;
                }
                self.bump(); // eat abstract
                modifiers.is_abstract = true;
                has_modifiers = true;
                progress = true;
            }

            // abstraction modifiers (virtual)
            if allow_virtual_modifier
                && self.language.is_destack()
                && self.is_keyword(Keyword::Virtual)
                && abstraction_is_modifier
            {
                if !self.next_same_line_token_starts_member_name() {
                    break;
                }
                self.bump(); // eat virtual
                modifiers.is_virtual = true;
                has_modifiers = true;
                progress = true;
            }

            // abstraction modifiers (override)
            if self.is_keyword(Keyword::Override) && abstraction_is_modifier {
                if !self.next_same_line_token_starts_member_name() {
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
                self.next_same_line_token_starts_member_name()
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
                if !self.next_same_line_token_starts_member_name() {
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
                && !matches!(
                    self.token_type_at_offset(1),
                    TokenType::Colon
                        | TokenType::Maybe
                        | TokenType::LessThan
                        | TokenType::OpenParenthesis
                );
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
            if self.language.is_destack()
                && !modifiers.is_comptime
                && self.is_keyword(Keyword::Comptime)
            {
                let next_token = self.next_token();
                let target_starts_after_comptime = next_token.token.ty == TokenType::OpenBrace
                    || !next_token.token.is_on_new_line
                        && self.token_starts_member_name(next_token);
                if !target_starts_after_comptime {
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
        let decorators = if !self.flags.is_in_type() {
            self.eat_decorators_maybe()?
        } else {
            smallvec::SmallVec::new()
        };

        let start = self.span_start();

        let mut modifiers = self.eat_binding_modifiers_prefix_maybe(
            true,
            false,
            false,
            self.flags.is_in_static(),
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
        let (pattern, name, name_span) = self.eat_parameter_pattern_or_name()?;
        // ? maybe
        if self.peek_is(TokenType::Maybe) {
            self.bump(); // eat maybe
            let mut modifier_set = modifiers.unwrap_or_default();
            modifier_set.is_optional = true;
            modifiers = Some(modifier_set);
        }

        // type annotation marker
        let (declared_type, ty_span) = {
            // type markers can start on the next line
            let annotation_token_type = self.peek_token_type();
            let annotation_keyword = self.current_keyword();
            let has_type_annotation_marker = annotation_token_type == TokenType::Colon
                || self.flags.is_in_static()
                    && matches!(
                        annotation_keyword,
                        Some(Keyword::Extends | Keyword::Implements)
                    );

            if has_type_annotation_marker {
                let type_start = self.span_start();
                self.bump(); // eat colon or keyword
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
        let is_comptime = modifiers.is_some_and(|modifier_set| modifier_set.is_comptime);

        // = value
        let parameter = {
            let has_default_assign = self.peek_is(TokenType::Assign)
                || self.current_token_is_on_new_line() && self.peek_is(TokenType::Assign);
            if !is_variadic && has_default_assign {
                self.bump(); // eat assign

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
                        is_comptime,
                        declared_type,
                        default: Some(value),
                    }
                }
                // pattern with default
                else {
                    let pattern = pattern
                        .ok_or_else(|| ParseError::unexpected(self.get_span_from(&start)))?;

                    Parameter::Pattern {
                        pattern,
                        is_optional,
                        is_comptime,
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
                        is_comptime,
                        declared_type,
                    }
                } else {
                    let pattern = pattern
                        .ok_or_else(|| ParseError::unexpected(self.get_span_from(&start)))?;

                    Parameter::VariadicPattern {
                        pattern,
                        is_comptime,
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
                        is_comptime,
                        declared_type,
                        default: None,
                    }
                }
                // pattern without default
                else {
                    let pattern = pattern
                        .ok_or_else(|| ParseError::unexpected(self.get_span_from(&start)))?;

                    Parameter::Pattern {
                        pattern,
                        is_optional,
                        is_comptime,
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
            self.tree.set_side_span(
                parameter_id,
                NodeSpanType::Region(NodeSpanRegion::Type),
                span,
            );
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
            let ambient_context = self.flags.with_before_type(true);
            let expression_context = self.flags;
            let pattern = self.with_flags(
                self.flags
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
        while self.has_more_tokens() {
            if self.peek_is(TokenType::CloseParenthesis) || self.peek_starts_type_angle_close() {
                break;
            }

            // newline led statement keywords should stay outside malformed parameter lists
            if self.current_keyword_starts_parameter_recovery_boundary() {
                break;
            }

            // eat one parameter
            let parameter_start = self.span_start();
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
                            if self.flags.is_in_static() {
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

            // reject cast tails inside parameter heads
            if self.language.is_typescript()
                && matches!(
                    self.current_keyword(),
                    Some(Keyword::As | Keyword::Satisfies)
                )
            {
                let error =
                    ParseError::unexpected(self.peek()?.span).for_node_type(NodeType::Parameter);
                self.try_recover_in_item_list(
                    &parameter_start,
                    if self.flags.is_in_static() {
                        TokenType::GreaterThan
                    } else {
                        TokenType::CloseParenthesis
                    },
                    Some(error.clone()),
                )?;
                return Err(error);
            }

            // untyped parameter lists reject trailing separators after rest parameters
            if in_js && has_variadic_parameter && self.is_item_stop() {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            // continue regular parameter lists after a real separator
            if self.peek_is(TokenType::Comma) {
                self.eat_item_stop()?;

                if !is_recovered_parameter {
                    continue;
                }

                if self.current_keyword_starts_parameter_recovery_boundary() {
                    break;
                }

                if !self.can_continue_after_recovered_item(
                    if self.flags.is_in_static() {
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
            // stop recovered lists before keyword boundaries
            let recovered_parameter_hits_boundary = is_recovered_parameter
                && (self.current_keyword_starts_parameter_recovery_boundary()
                    || !self.can_continue_after_recovered_item(
                        if self.flags.is_in_static() {
                            TokenType::GreaterThan
                        } else {
                            TokenType::CloseParenthesis
                        },
                        is_recovered_parameter,
                    ));

            // require a separator between adjacent parameter heads
            let adjacent_parameter_heads_without_separator = !self.current_token_is_on_new_line();
            if recovered_parameter_hits_boundary || adjacent_parameter_heads_without_separator {
                break;
            }
        }
        Ok(parameters)
    }

    /// Parse one generic parameter in a generic parameter list.
    fn eat_generic_parameter(&mut self) -> ParseResult<LocalNodeId<GenericParameter>> {
        let start = self.span_start();

        // generic parameters accept only the dedicated generic modifiers
        let mut variance = None;
        let mut is_const = false;
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

            let is_out_modifier = self.peek_is(TokenType::Identifier)
                && self.current_identifier_str_is("out")
                && self.token_type_at_offset(1) == TokenType::Identifier;
            if is_out_modifier {
                self.bump(); // eat out
                variance = Some(match variance {
                    Some(VarianceModifier::In) => VarianceModifier::InOut,
                    Some(VarianceModifier::InOut) => VarianceModifier::InOut,
                    _ => VarianceModifier::Out,
                });
                continue;
            }

            if self.is_keyword(Keyword::Const) {
                self.bump(); // eat const
                is_const = true;
                continue;
            }

            if self.language.is_destack() && self.is_keyword(Keyword::Comptime) {
                self.bump(); // eat comptime
                is_comptime = true;
                continue;
            }

            break;
        }

        let is_variadic = self.peek_is(TokenType::Spread);
        if is_variadic && !self.language.is_destack() {
            return Err(ParseError::unexpected(self.peek()?.span));
        }
        if is_variadic {
            self.eat_token(TokenType::Spread)?;
        }

        // generic parameters are always named
        let (name, name_span) = self.eat_binding_identifier_with_span()?;

        // generic parameters are type parameters by default
        // value parameters must opt in with `comptime` or `const`
        let annotation_token_type = self.peek_token_type();
        let annotation_keyword = self.current_keyword();
        let has_colon_annotation = annotation_token_type == TokenType::Colon;
        let has_type_constraint = matches!(annotation_keyword, Some(Keyword::Extends));
        let has_annotation = has_colon_annotation || has_type_constraint;
        let is_value_parameter = is_comptime;

        let (declared_type, declared_type_span) = if has_annotation {
            let type_start = self.span_start();
            self.bump(); // eat colon or extends
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
            || self.current_token_is_on_new_line() && self.peek_is(TokenType::Assign)
        {
            self.bump(); // eat =

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
            if is_variadic {
                GenericParameter::VariadicValue {
                    name,
                    declared_type,
                    default: default.0,
                    is_comptime,
                }
            } else {
                GenericParameter::Value {
                    name,
                    declared_type,
                    default: default.0,
                    is_comptime,
                }
            }
        } else if is_variadic {
            GenericParameter::VariadicType {
                name,
                is_const,
                variance,
                constraint: declared_type,
                default: default.1,
            }
        } else {
            GenericParameter::Type {
                name,
                is_const,
                variance,
                constraint: declared_type,
                default: default.1,
            }
        };

        let parameter_id = self.insert_node(parameter, self.get_span_from(&start));
        self.tree.set_main_span(parameter_id, name_span);

        if let Some(span) = declared_type_span {
            self.tree.set_side_span(
                parameter_id,
                NodeSpanType::Region(NodeSpanRegion::Type),
                span,
            );
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
        if !self.peek_is(TokenType::LessThan) {
            return Ok(None);
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
        let start = self.span_start();
        self.eat_token(TokenType::LessThan)?;

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
        let mut ambient_context = self.flags.nested().with_static(true);
        if self.flags.is_in_type() || self.flags.is_in_decorator() || self.language.is_typescript()
        {
            ambient_context = ambient_context.with_type(true);
        }
        let expression_context = self.flags.nested();
        let parameters = self.with_flags(
            self.flags
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
            |parser| {
                let mut parameters = Vec::new();
                while parser.has_more_tokens() {
                    if parser.peek_starts_type_angle_close() {
                        break;
                    }

                    let parameter = parser
                        .eat_generic_parameter()
                        .for_node_type(NodeType::GenericParameter)?;
                    parameters.push(parameter);

                    if parser.peek_starts_type_angle_close() {
                        break;
                    }

                    if parser.peek_is(TokenType::Comma) {
                        parser.bump(); // eat comma
                        continue;
                    }

                    break;
                }

                Ok(parameters)
            },
        )?;

        let container_start = self.get_span_from(&start).start;
        if let Some(first_parameter_id) = parameters.first().copied() {
            self.set_node_leading_span(first_parameter_id, container_start);
        }

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

        // empty dynamic parameters
        if self.peek_is(TokenType::CloseParenthesis) {
            self.bump(); // eat close parenthesis
            return Ok(vec![]);
        }

        // regular dynamic parameters
        let mut ambient_context = self.flags.nested();
        if self.flags.is_in_type() {
            ambient_context = ambient_context.with_type(true);
        }
        if self.flags.is_in_variant() {
            ambient_context = ambient_context.with_variant(true);
        }
        let expression_context = self.flags.nested();
        let parameters = self.with_flags(
            self.flags
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
            // recover empty arguments as list errors, not expression errors
            if Self::is_expression_slot_boundary_token(self.peek_token_type()) {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            let value = self.eat_expression(self.positional_argument_flags())?;
            let value_span = self.tree.get_span(value);
            let argument_id = self.insert_node(Argument::Positional { value }, value_span);
            return Ok(argument_id);
        }

        let start = self.span_start();
        let decorators = if !self.flags.is_in_type() {
            self.eat_decorators_maybe()?
        } else {
            smallvec::SmallVec::new()
        };

        // spread argument
        if self.peek_is(TokenType::Spread) {
            self.bump(); // eat spread
            let value = self.eat_expression(self.positional_argument_flags())?;

            // build spread argument
            let argument_id = self.insert_node(
                Argument::Spread { label: None, value },
                self.get_span_from(&start),
            );
            self.attach_decorators(argument_id.id, decorators);
            return Ok(argument_id);
        }

        // positional value expression
        let value = self.eat_expression(self.positional_argument_flags())?;

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
        self.eat_tree_argument_with_follow(ContextualLexMode::Normal)
    }

    /// Eat one tree child argument and advance in the requested tree mode after delimiters.
    ///
    /// Examples:
    /// ```
    /// x: 1
    /// ...args
    /// {value}
    /// <Widget prop=value />
    /// ```
    pub(crate) fn eat_tree_argument_with_follow(
        &mut self,
        follow_mode: ContextualLexMode,
    ) -> ParseResult<LocalNodeId<Argument>> {
        let start = self.span_start();
        // named argument (name: value)
        if self.peek_name_is() && self.token_type_at_offset(1) == TokenType::Colon {
            let (name, name_span) = self
                .eat_name_with_span()
                .for_node_type(NodeType::Argument)?;
            self.bump(); // eat colon
            // value
            let value = self.eat_expression(self.positional_argument_flags())?;
            let argument_id =
                self.insert_node(Argument::Named { name, value }, self.get_span_from(&start));
            self.tree.set_main_span(argument_id, name_span);
            Ok(argument_id)
        }
        // spread argument (...expr)
        else if self.peek_is(TokenType::Spread) {
            self.bump(); // eat spread
            let value = self.eat_expression(self.positional_argument_flags())?;
            let argument_id = self.insert_node(
                Argument::Spread { label: None, value },
                self.get_span_from(&start),
            );
            Ok(argument_id)
        }
        // expression container ({expr}): braces are delimiters, not part of the expression
        else if self.peek_is(TokenType::OpenBrace) {
            let wrapper_start = self.span_start();
            self.bump_with_contextual_lex_mode(ContextualLexMode::Normal); // eat {

            // empty container (including comment-only containers)
            if self.peek_is(TokenType::CloseBrace) {
                let value = self
                    .tree
                    .insert(Expression::Stub, self.get_span_from(&start));

                self.bump_with_contextual_lex_mode(follow_mode); // eat }
                self.set_node_wrapper_span(value, self.get_span_from(&wrapper_start));
                let argument_id =
                    self.insert_node(Argument::Positional { value }, self.get_span_from(&start));
                return Ok(argument_id);
            }

            // spread child: {...expr}
            if self.peek_is(TokenType::Spread) {
                self.bump(); // eat spread
                let value_ambient_context = self.flags.with_tree_literal(false);
                let value_expression_context =
                    self.flags.not_in_position().not_in_ternary_condition();
                let value = self.eat_expression(
                    self.flags
                        .with_ambient_context(value_ambient_context)
                        .with_expression_context(value_expression_context),
                )?;

                self.eat_tree_argument_close_brace(follow_mode)?;
                self.set_node_wrapper_span(value, self.get_span_from(&wrapper_start));
                let argument_id = self.insert_node(
                    Argument::Spread { label: None, value },
                    self.get_span_from(&start),
                );
                return Ok(argument_id);
            }

            let value_ambient_context = self.flags.with_tree_literal(false);
            let value_expression_context = self.flags.not_in_position().not_in_ternary_condition();
            let value = self.eat_expression(
                self.flags
                    .with_ambient_context(value_ambient_context)
                    .with_expression_context(value_expression_context),
            )?;

            self.eat_tree_argument_close_brace(follow_mode)?;
            self.set_node_wrapper_span(value, self.get_span_from(&wrapper_start));
            let argument_id =
                self.insert_node(Argument::Positional { value }, self.get_span_from(&start));
            Ok(argument_id)
        }
        // positional argument (bare expression like nested <Element />)
        else {
            // jsx content without braces must be text or nested tags
            if self.language.supports_jsx() && self.flags.is_in_tree_literal() {
                let token = *self.peek()?;
                let is_tree_text = token.token.ty == TokenType::Literal
                    && matches!(
                        token.token.literal,
                        Some(TokenLiteral::TreeString)
                            | Some(TokenLiteral::Character {
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
                    let value = self.eat_tree_child_scalar_expression(follow_mode)?;
                    let argument_id = self
                        .insert_node(Argument::Positional { value }, self.get_span_from(&start));
                    return Ok(argument_id);
                }
            }

            let value = if self.peek_is(TokenType::LessThan) && self.peek_tree_literal().is_ok() {
                self.eat_tree_literal_with_follow(follow_mode)?
            } else {
                let value_expression_context =
                    self.flags.not_in_position().not_in_sequence_expression();
                self.eat_expression(value_expression_context)?
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
        let start = self.span_start();
        // spread argument
        if self.peek_is(TokenType::Spread) {
            self.bump(); // eat spread
            let value_expression_context = self.flags.with_statement_position(true);
            let value = self.eat_expression(value_expression_context)?;
            let argument_id = self.insert_node(
                Argument::Spread { label: None, value },
                self.get_span_from(&start),
            );
            Ok(argument_id)
        }
        // spread expression container
        else if self.peek_is(TokenType::OpenBrace) {
            let wrapper_start = self.span_start();
            self.bump_with_contextual_lex_mode(ContextualLexMode::Normal); // eat open brace
            if !self.peek_is(TokenType::Spread) {
                return Err(ParseError::unexpected(self.peek()?.span));
            }
            self.bump(); // eat spread
            let value_ambient_context = self.flags.with_tree_literal(false);
            let value_expression_context = self
                .flags
                .not_in_position()
                .not_in_ternary_condition()
                .not_in_sequence_expression();
            let value = self.eat_expression(
                self.flags
                    .with_ambient_context(value_ambient_context)
                    .with_expression_context(value_expression_context),
            )?;
            self.eat_tree_argument_close_brace(ContextualLexMode::TreeTag)?;
            self.set_node_wrapper_span(value, self.get_span_from(&wrapper_start));
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
                self.bump(); // eat colon or assign

                // tree expression container: attr={expr}
                if self.peek_is(TokenType::OpenBrace) {
                    let wrapper_start = self.span_start();
                    self.bump_with_contextual_lex_mode(ContextualLexMode::Normal); // eat {
                    let value_ambient_context = self.flags.with_tree_literal(false);
                    let value_expression_context =
                        self.flags.not_in_position().not_in_ternary_condition();
                    let value = self.eat_expression(
                        self.flags
                            .with_ambient_context(value_ambient_context)
                            .with_expression_context(value_expression_context),
                    )?;
                    self.eat_tree_argument_close_brace(ContextualLexMode::TreeTag)?;
                    self.set_node_wrapper_span(value, self.get_span_from(&wrapper_start));
                    value
                }
                // string literal attribute
                else if self.peek_string_literal_is() {
                    self.set_tree_tag_follow();
                    let (string, span) = self.eat_tree_attribute_string_literal()?;
                    self.insert_node(
                        Expression::ScalarLiteral(ScalarLiteral::String(string)),
                        span,
                    )
                }
                // shorthand array attribute
                else if self.peek_is(TokenType::OpenBracket) {
                    let value_start = self.span_start();
                    let value_ambient_context = self.flags.with_tree_literal(false);
                    let value_expression_context = self.flags.not_in_position();
                    self.with_flags(
                        self.flags
                            .with_ambient_context(value_ambient_context)
                            .with_expression_context(value_expression_context),
                        |parser| parser.eat_bracket_literal_expression(&value_start),
                    )?
                }
                // tree literal attribute value
                else if self.language.supports_jsx() && self.peek_is(TokenType::LessThan) {
                    let value_ambient_context = self.flags.with_tree_literal(true);
                    let value_expression_context = self.flags.not_in_position();
                    self.with_flags(
                        self.flags
                            .with_ambient_context(value_ambient_context)
                            .with_expression_context(value_expression_context),
                        |parser| parser.eat_tree_literal_with_follow(ContextualLexMode::TreeTag),
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

    /// Eat a quoted tree attribute string literal.
    ///
    /// Examples:
    /// ```ds
    /// title="Hello"
    /// title='Hello'
    /// title="A&nbsp;B"
    /// title="A&#160;&#xA0;B"
    /// ```
    fn eat_tree_attribute_string_literal(&mut self) -> ParseResult<(StringId, Span)> {
        let token = *self.peek_string_literal()?;
        let content = self.get_string_literal_str(token);

        // match JSX transforms by decoding attribute entities
        let string_id = if let Some(decoded) = decode_html_entities(content) {
            self.strings.intern(&decoded)
        } else {
            self.strings.intern(content)
        };

        self.bump();

        Ok((string_id, token.span))
    }

    /// Eat a tree argument expression-container close and advance in the requested mode.
    fn eat_tree_argument_close_brace(&mut self, follow_mode: ContextualLexMode) -> ParseResult<()> {
        if self.peek_is(TokenType::CloseBrace) {
            self.bump_with_contextual_lex_mode(follow_mode);

            return Ok(());
        }

        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Argument)
    }

    /// Return true when the current tree argument has an explicit value separator.
    pub(crate) fn tree_literal_argument_has_value_separator(&mut self) -> bool {
        self.peek_is(TokenType::Colon)
            || self.peek_is(TokenType::Assign)
            || self.current_token_is_on_new_line()
                && (self.peek_is(TokenType::Colon) || self.peek_is(TokenType::Assign))
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

    /// Eat type generic arguments, including the angle tokens.
    ///
    /// Examples:
    /// ```ds
    /// <T>
    /// <K, V>
    /// <<T>() => T>
    /// ```
    pub(crate) fn eat_type_generic_arguments(
        &mut self,
    ) -> ParseResult<Vec<LocalNodeId<GenericArgument>>> {
        let start = self.span_start();

        self.eat_generic_angle_open()?;

        let first_argument_boundary_start = self.prev_token_end();
        let recovers_empty_argument = true;
        let generic_arguments = self.eat_generic_arguments_after_open(
            &start,
            first_argument_boundary_start,
            recovers_empty_argument,
            self.type_generic_argument_flags(),
        )?;

        self.eat_type_angle_close_or_recover_missing(NodeType::Expression)?;

        Ok(generic_arguments)
    }

    /// Eat one generic argument opening angle and split `<<` when needed.
    fn eat_generic_angle_open(&mut self) -> ParseResult<bool> {
        if self.peek_is(TokenType::LessThan) {
            self.bump_with_contextual_lex_mode(ContextualLexMode::Normal);

            return Ok(false);
        }

        if self.peek_is(TokenType::ShiftLeft) {
            if !self.re_lex_generic_l_angle() {
                return Err(ParseError::expected(self.peek()?.span, TokenType::LessThan));
            }

            self.bump_with_contextual_lex_mode(ContextualLexMode::Normal);

            return Ok(true);
        }

        Err(ParseError::expected(self.peek()?.span, TokenType::LessThan))
    }

    /// Eat generic argument contents after the opening angle.
    fn eat_generic_arguments_after_open(
        &mut self,
        start: &ParserSpanStart,
        first_argument_boundary_start: u32,
        recovers_empty_argument: bool,
        flags: ParserFlags,
    ) -> ParseResult<Vec<LocalNodeId<GenericArgument>>> {
        let is_empty = if recovers_empty_argument {
            self.peek_starts_type_angle_close()
        } else {
            self.peek_starts_expression_type_angle_close()
        };
        if !is_empty {
            return self.with_flags(flags, |parser| {
                parser.eat_generic_arguments_body(first_argument_boundary_start)
            });
        }

        if recovers_empty_argument {
            Ok(vec![self.recover_empty_generic_argument(start)])
        } else {
            Err(ParseError::expected(
                self.get_span_from(start),
                TokenType::Identifier,
            ))
        }
    }

    /// Return the flags for type generic arguments.
    fn type_generic_argument_flags(&self) -> ParserFlags {
        let ambient_context = self.flags.nested().with_static(true).with_type(true);
        let expression_context = self.flags.nested();

        self.flags
            .with_ambient_context(ambient_context)
            .with_expression_context(expression_context)
    }

    /// Return the flags for generic arguments that may be types or values.
    fn mixed_generic_argument_flags(&self) -> ParserFlags {
        let mut ambient_context = self.flags.nested().with_static(true);
        if self.flags.is_in_type()
            || self.flags.is_in_decorator()
            || self.language.is_destack()
            || self.language.is_typescript()
        {
            ambient_context = ambient_context.with_type(true);
        }
        let expression_context = self.flags.nested();

        self.flags
            .with_ambient_context(ambient_context)
            .with_expression_context(expression_context)
    }

    /// Recover one empty generic argument list as an error argument.
    fn recover_empty_generic_argument(
        &mut self,
        start: &ParserSpanStart,
    ) -> LocalNodeId<GenericArgument> {
        let error = ParseError::expected(self.get_span_from(start), TokenType::Identifier);
        self.error(&error);

        let argument_start = self.span_start();

        self.insert_node(GenericArgument::Error, self.get_span_from(&argument_start))
    }

    /// Eat top-level generic arguments.
    #[inline]
    fn eat_generic_arguments_body(
        &mut self,
        mut next_argument_boundary_start: u32,
    ) -> ParseResult<Vec<LocalNodeId<GenericArgument>>> {
        let mut arguments = smallvec::SmallVec::<[LocalNodeId<GenericArgument>; 4]>::new();

        while self.has_more_tokens() {
            // stop on closing `>`
            if self.peek_starts_type_angle_close() {
                break;
            }

            // eat one argument
            let argument_start = self.span_start();
            let mut is_recovered_argument = false;
            let argument_id = match self
                .eat_generic_argument(self.argument_value_flags(), &argument_start)
            {
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
            self.set_node_leading_span(argument_id, next_argument_boundary_start);

            arguments.push(argument_id);

            // continue through separators
            if self.peek_is(TokenType::Comma) {
                self.bump();
                next_argument_boundary_start = self.prev_token_end();
            }
            // let recovered arguments continue across newline separators only
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
        let start = self.span_start();
        let used_shift_left_start = self.eat_generic_angle_open()?;
        let first_argument_boundary_start = self.prev_token_end();

        let recovers_empty_argument = self.flags.is_in_type() || self.flags.is_in_decorator();
        let generic_arguments = self.eat_generic_arguments_after_open(
            &start,
            first_argument_boundary_start,
            recovers_empty_argument,
            self.mixed_generic_argument_flags(),
        )?;

        // type-like contexts can consume glued right-angle tails
        let allow_glued_type_close =
            self.flags.is_in_type() || self.flags.is_in_decorator() || used_shift_left_start;
        let allow_missing_type_close = self.flags.is_in_type() || self.flags.is_in_decorator();

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
        self.eat_token(TokenType::OpenParenthesis)?;

        // empty dynamic arguments
        if self.peek_is(TokenType::CloseParenthesis) {
            self.bump(); // eat close parenthesis
            return Ok(vec![]);
        }

        // regular dynamic arguments
        let ambient_context = self.flags.nested();
        let expression_context = self.flags.nested();
        let arguments = self.with_flags(
            self.flags
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
            |parser| parser.eat_positional_arguments_body(TokenType::CloseParenthesis),
        )?;

        self.eat_list_close_token_or_recover_missing(
            TokenType::CloseParenthesis,
            NodeType::Expression,
        )?;

        Ok(arguments)
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
        self.eat_argument_list_body(terminator, Parser::eat_positional_argument)
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
        self.eat_argument_list_body(terminator, Parser::eat_tree_argument)
    }

    /// Eat one argument list body with caller-selected item syntax.
    ///
    /// Examples:
    /// ```ds
    /// first, second
    /// first
    /// second
    /// broken, recovered
    /// ```
    #[inline]
    fn eat_argument_list_body(
        &mut self,
        terminator: TokenType,
        mut eat_argument: impl FnMut(&mut Self) -> ParseResult<LocalNodeId<Argument>>,
    ) -> ParseResult<Vec<LocalNodeId<Argument>>> {
        let mut arguments = smallvec::SmallVec::<[LocalNodeId<Argument>; 4]>::new();

        while self.has_more_tokens() {
            if self.peek_is(terminator) {
                break;
            }

            // eat one argument
            let argument_start = self.span_start();
            let is_recovered_argument;
            let argument_id = match eat_argument(self) {
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

            arguments.push(argument_id);

            // continue regular lists after a real separator
            if self.peek_is(TokenType::Comma) {
                self.eat_item_stop()?;

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
