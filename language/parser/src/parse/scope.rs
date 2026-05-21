use crate::parse::expression::operator::ExpressionInfixOperator;
use crate::parse::flags::ParserFlags;
use crate::parse::r#type::operator::TypeInfixOperator;

pub(super) const CONDITIONAL_PRECEDENCE: u16 = 900;

/// Value expression parse scope.
#[derive(Clone, Copy)]
pub(crate) struct ExpressionScope {
    /// The parser flags active while this expression is parsed.
    pub(super) flags: ParserFlags,
    /// The minimum infix precedence accepted here.
    pub(super) minimum_precedence: Option<u16>,
    /// Whether sequence expressions may be parsed.
    pub(super) allows_sequence: bool,
    /// Whether the expression is parsed in statement position.
    pub(super) is_statement_position: bool,
    /// Whether `:` is owned by an outer ternary or match case.
    pub(super) owns_colon_boundary: bool,
    /// Whether newline terminates continuation in match case bodies.
    pub(super) is_match_case_body: bool,
    /// Whether the receiver is the operand of `new`.
    pub(super) is_new_receiver: bool,
    /// Whether tree syntax owns `<` continuations.
    pub(super) is_tree_literal: bool,
    /// Whether `in`, `of`, or the body block belongs to an outer for-each parser.
    pub(super) owns_for_each_boundary: bool,
    /// Whether generic postfix syntax is disabled after typeof.
    pub(super) is_typeof_query: bool,
    /// Whether newline calls belong to an outer statement boundary.
    pub(super) owns_newline_call_boundary: bool,
    /// Whether newline terminates decorator target parsing.
    pub(super) owns_decorator_line_boundary: bool,
    /// Whether static closers terminate expression parsing.
    pub(super) is_static: bool,
}

/// Type expression parse scope.
#[derive(Clone, Copy)]
pub(super) struct TypeScope {
    /// The parser flags active while this type is parsed.
    pub(super) flags: ParserFlags,
    /// The minimum infix precedence accepted here.
    pub(super) minimum_precedence: Option<u16>,
    /// Whether `:` is owned by an outer ternary, match case, or conditional type.
    pub(super) owns_colon_boundary: bool,
    /// Whether `extends` belongs to an outer conditional type.
    pub(super) disallows_conditional: bool,
    /// Whether heritage clauses own `implements`.
    pub(super) stops_before_implements: bool,
    /// Whether type predicate syntax is valid.
    pub(super) allows_type_predicate: bool,
    /// Whether static type closers stop parsing.
    pub(super) is_static: bool,
}

impl ExpressionScope {
    /// Create a value expression scope from parser flags.
    pub(super) fn from_flags(flags: ParserFlags) -> Self {
        Self {
            flags,
            minimum_precedence: None,
            allows_sequence: flags.allows_sequence_expression(),
            is_statement_position: flags.is_in_statement_position(),
            owns_colon_boundary: flags.is_in_ternary_condition() || flags.is_in_match_case(),
            is_match_case_body: flags.is_in_match_case_body(),
            is_new_receiver: flags.is_in_new_receiver(),
            is_tree_literal: flags.is_in_tree_literal(),
            owns_for_each_boundary: flags.is_in_for_each(),
            is_typeof_query: flags.is_in_typeof_query(),
            owns_newline_call_boundary: false,
            owns_decorator_line_boundary: flags.is_in_decorator(),
            is_static: flags.is_in_static(),
        }
    }

    /// Return this scope with a different minimum precedence.
    pub(super) fn at_precedence(mut self, minimum_precedence: Option<u16>) -> Self {
        self.minimum_precedence = minimum_precedence;

        self
    }

    /// Return this scope with newline calls owned by the caller.
    pub(super) fn with_newline_call_boundary(mut self, enabled: bool) -> Self {
        self.owns_newline_call_boundary = enabled;

        self
    }

    /// Return true when this scope stops before an operator.
    pub(super) fn stops_before(self, operator: ExpressionInfixOperator) -> bool {
        let Some(minimum_precedence) = self.minimum_precedence else {
            return false;
        };

        if operator.is_right_associative() {
            minimum_precedence > operator.precedence()
        } else {
            minimum_precedence >= operator.precedence()
        }
    }
}

impl TypeScope {
    /// Create a type expression scope from parser flags.
    pub(super) fn from_flags(flags: ParserFlags) -> Self {
        Self {
            flags,
            minimum_precedence: None,
            owns_colon_boundary: flags.is_in_ternary_condition()
                || flags.is_in_match_case()
                || flags.is_in_type_conditional_right(),
            disallows_conditional: flags.is_disallow_type_conditional(),
            stops_before_implements: flags.is_in_before_block() || flags.is_in_super_type(),
            allows_type_predicate: flags.allows_type_predicate(),
            is_static: flags.is_in_static(),
        }
    }

    /// Return this scope with a different minimum precedence.
    pub(super) fn at_precedence(mut self, minimum_precedence: Option<u16>) -> Self {
        self.minimum_precedence = minimum_precedence;

        self
    }

    /// Return true when this scope stops before an operator.
    pub(super) fn stops_before(self, operator: TypeInfixOperator) -> bool {
        self.minimum_precedence
            .is_some_and(|minimum_precedence| minimum_precedence >= operator.precedence())
    }
}
