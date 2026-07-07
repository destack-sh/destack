use destack_dir::{
    Keyword, MethodAbstraction, Token, TokenLiteral, TokenType, VarianceModifier, Visibility,
};

use crate::{Parser, ParserError};

/// The keywords that can appear before a binding.
pub static BINDING_MODIFIERS: [Keyword; 9] = [
    Keyword::Static,
    Keyword::Abstract,
    Keyword::Virtual,
    Keyword::Override,
    Keyword::Readonly,
    Keyword::Public,
    Keyword::Protected,
    Keyword::Private,
    Keyword::Comptime,
];

/// The binding grammar that governs one modifier prefix.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum BindingModifierGrammar {
    /// A function parameter.
    Parameter,
    /// An object property.
    Property,
    /// A declared member.
    Member,
}

impl BindingModifierGrammar {
    /// Return whether this grammar accepts accessor modifiers.
    const fn accepts_accessor(self) -> bool {
        matches!(self, Self::Property | Self::Member)
    }

    /// Return whether this grammar accepts virtual modifiers.
    const fn accepts_virtual(self) -> bool {
        matches!(self, Self::Member)
    }

    /// Return whether this grammar accepts comptime modifiers.
    const fn accepts_comptime(self) -> bool {
        matches!(self, Self::Property | Self::Member)
    }

    /// Return whether this grammar accepts variance modifiers.
    const fn accepts_variance(self) -> bool {
        matches!(self, Self::Member)
    }

    /// Return whether this grammar accepts declaration modifiers.
    const fn accepts_declaration(self) -> bool {
        matches!(self, Self::Member)
    }

    /// Return whether this grammar validates modifier order.
    const fn validates_order(self) -> bool {
        !matches!(self, Self::Property)
    }
}

/// Modifiers consumed from one binding head.
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
    /// Return the visibility keyword when present.
    #[inline]
    fn peek_visibility(&self) -> Option<Visibility> {
        match self.peek_keyword() {
            Some(Keyword::Public) => Some(Visibility::Public),
            Some(Keyword::Protected) => Some(Visibility::Protected),
            Some(Keyword::Private) => Some(Visibility::Private),
            _ => None,
        }
    }

    /// Return true when the next token can start a member name.
    pub(crate) fn peek_next_member_name(&self) -> bool {
        let peek_next_token = self.peek_next_token();

        Self::is_member_name_start(peek_next_token)
    }

    /// Return true when the next same-line token can start a member name.
    pub(crate) fn peek_next_same_line_member_name(&self) -> bool {
        let peek_next_token = self.peek_next_token();
        if peek_next_token.is_on_new_line() {
            return false;
        }

        Self::is_member_name_start(peek_next_token)
    }

    /// Return true when one token can start a member name.
    pub(crate) fn is_member_name_start(token: Token) -> bool {
        // check for common member name starters
        if matches!(
            token.ty(),
            TokenType::Identifier
                | TokenType::Hash
                | TokenType::OpenBracket
                | TokenType::OpenBrace
                | TokenType::Spread
                | TokenType::Multiply
        ) {
            return true;
        }
        if !token.is(TokenType::Literal) {
            return false;
        }

        // check for valid literal member names
        matches!(
            token.literal(),
            Some(TokenLiteral::String {
                is_terminated: true,
                has_invalid_escape: false,
            }) | Some(TokenLiteral::Boolean { .. })
                | Some(TokenLiteral::Int { .. })
                | Some(TokenLiteral::Float { .. })
        )
    }

    /// Parse a binding modifiers prefix when present.
    pub(crate) fn parse_binding_modifiers(
        &mut self,
        grammar: BindingModifierGrammar,
    ) -> BindingModifiers {
        // initialize modifier state
        let mut modifiers = BindingModifiers::default();

        // modifier ordering for typed member forms
        let mut seen_static = false;
        let mut seen_override = false;
        let mut seen_readonly = false;
        let mut seen_variance_in = false;
        let mut seen_variance_out = false;
        let mut seen_accessor = false;

        // eat modifiers in any order
        loop {
            // modifiers only start on identifiers and known modifier keywords
            if !self.peek_is(TokenType::Identifier) {
                break;
            }
            let peek_keyword = self.peek_keyword();
            let is_out_variance_modifier = grammar.accepts_variance()
                && self.peek_identifier_is("out")
                && !self.peek_next_token().is_on_new_line()
                && (self.peek_token_type_at(1) == TokenType::Identifier
                    || self.peek_keyword_at(1) == Some(Keyword::In));
            let can_start_modifier = peek_keyword.is_some_and(|keyword| {
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
                let is_virtual_modifier = grammar.accepts_virtual() && keyword == Keyword::Virtual;
                let is_comptime_modifier =
                    grammar.accepts_comptime() && keyword == Keyword::Comptime;

                is_standard_modifier || is_virtual_modifier || is_comptime_modifier
            }) || is_out_variance_modifier;
            if !can_start_modifier {
                break;
            }

            // modifier disambiguation for abstraction keywords
            let is_abstraction_modifier = !matches!(
                self.peek_token_type_at(1),
                TokenType::Colon | TokenType::Maybe | TokenType::LessThan
            );

            // consume an input variance modifier
            if grammar.accepts_variance() && peek_keyword == Some(Keyword::In) {
                let range = self.peek_token().range();
                self.bump();
                if grammar.validates_order() && (seen_variance_in || seen_variance_out) {
                    self.report_error(ParserError::unexpected(range));
                }
                modifiers.variance = Some(match modifiers.variance {
                    Some(VarianceModifier::Out | VarianceModifier::InOut) => {
                        VarianceModifier::InOut
                    }
                    _ => VarianceModifier::In,
                });
                seen_variance_in = true;

                continue;
            }

            // consume an output variance modifier
            if is_out_variance_modifier {
                let range = self.peek_token().range();
                self.bump();
                if grammar.validates_order() && seen_variance_out {
                    self.report_error(ParserError::unexpected(range));
                }
                modifiers.variance = Some(match modifiers.variance {
                    Some(VarianceModifier::In | VarianceModifier::InOut) => VarianceModifier::InOut,
                    _ => VarianceModifier::Out,
                });
                seen_variance_out = true;

                continue;
            }

            // visibility modifiers
            if let Some(visibility) = self.peek_visibility() {
                if !self.peek_next_same_line_member_name() {
                    break;
                }
                let range = self.peek_token().range();
                self.bump();
                if modifiers.visibility.is_some() {
                    if grammar.validates_order() {
                        self.report_error(ParserError::unexpected(range));
                    }
                } else {
                    if grammar.validates_order() && (seen_static || seen_override || seen_readonly)
                    {
                        self.report_error(ParserError::unexpected(range));
                    }
                    modifiers.visibility = Some(visibility);
                }
                continue;
            }

            // declaration modifiers
            if grammar.accepts_declaration() && self.peek_is_keyword(Keyword::Declare) {
                if !self.peek_next_same_line_member_name() {
                    break;
                }
                let range = self.peek_token().range();
                self.bump();
                if modifiers.is_ambient {
                    if grammar.validates_order() {
                        self.report_error(ParserError::unexpected(range));
                    }
                } else {
                    modifiers.is_ambient = true;
                }
                continue;
            }

            // scope modifiers (static)
            if !modifiers.is_static && self.peek_is_keyword(Keyword::Static) {
                if !self.peek_next_member_name() {
                    break;
                }
                let range = self.peek_token().range();
                self.bump();
                modifiers.is_static = true;
                if grammar.validates_order() && seen_override {
                    self.report_error(ParserError::unexpected(range));
                }
                if grammar.validates_order() && seen_accessor {
                    self.report_error(ParserError::unexpected(range));
                }
                seen_static = true;
                continue;
            }

            // abstraction modifiers (abstract)
            if self.peek_is_keyword(Keyword::Abstract) && is_abstraction_modifier {
                if !self.peek_next_same_line_member_name() {
                    break;
                }
                self.bump();
                modifiers.is_abstract = true;
                continue;
            }

            // abstraction modifiers (virtual)
            if grammar.accepts_virtual()
                && self.peek_is_keyword(Keyword::Virtual)
                && is_abstraction_modifier
            {
                if !self.peek_next_same_line_member_name() {
                    break;
                }
                self.bump();
                modifiers.is_virtual = true;
                continue;
            }

            // abstraction modifiers (override)
            if self.peek_is_keyword(Keyword::Override) && is_abstraction_modifier {
                if !self.peek_next_same_line_member_name() {
                    break;
                }
                let range = self.peek_token().range();
                self.bump();
                modifiers.is_override = true;
                if grammar.validates_order() && seen_readonly {
                    self.report_error(ParserError::unexpected(range));
                }
                seen_override = true;
                continue;
            }

            // mutability modifiers (readonly)
            // allow treating readonly as a key in property contexts
            let is_readonly_modifier = self.peek_next_same_line_member_name();
            if !modifiers.is_readonly
                && self.peek_is_keyword(Keyword::Readonly)
                && is_readonly_modifier
            {
                self.bump();
                modifiers.is_readonly = true;
                seen_readonly = true;
                continue;
            }

            // operator modifiers (const)
            if !modifiers.is_const_asserted && self.peek_is_keyword(Keyword::Const) {
                if !self.peek_next_same_line_member_name() {
                    break;
                }
                self.bump();
                modifiers.is_const_asserted = true;
                continue;
            }

            // accessor modifiers
            let is_accessor_modifier = grammar.accepts_accessor()
                && self.peek_is_keyword(Keyword::Accessor)
                && !matches!(
                    self.peek_token_type_at(1),
                    TokenType::Colon
                        | TokenType::Maybe
                        | TokenType::LessThan
                        | TokenType::OpenParenthesis
                );
            if !modifiers.is_accessor && is_accessor_modifier {
                if !self.peek_next_member_name() {
                    break;
                }
                self.bump();
                modifiers.is_accessor = true;
                seen_accessor = true;
                continue;
            }

            // timing modifiers (comptime)
            if grammar.accepts_comptime()
                && !modifiers.is_comptime
                && self.peek_is_keyword(Keyword::Comptime)
            {
                let peek_next_token = self.peek_next_token();
                let is_target_after_comptime = peek_next_token.ty() == TokenType::OpenBrace
                    || !peek_next_token.is_on_new_line()
                        && Self::is_member_name_start(peek_next_token);
                if !is_target_after_comptime {
                    break;
                }
                self.bump();
                modifiers.is_comptime = true;
                continue;
            }

            break;
        }

        modifiers
    }

    /// Parse one optional postfix binding modifier.
    pub(crate) fn parse_postfix_binding_modifier(
        &mut self,
        mut modifiers: BindingModifiers,
    ) -> BindingModifiers {
        if self.peek_is(TokenType::Maybe) {
            self.bump();
            modifiers.is_optional = true;
        }

        modifiers
    }
}
