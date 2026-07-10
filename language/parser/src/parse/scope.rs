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
    /// Whether newline calls belong to an outer statement boundary.
    owns_newline_call_boundary: bool,
}

/// Type expression parse scope.
#[derive(Clone, Copy)]
pub(super) struct TypeScope {
    /// The parser flags active while this type is parsed.
    pub(super) flags: ParserFlags,
    /// The minimum infix precedence accepted here.
    pub(super) minimum_precedence: Option<u16>,
}

impl ExpressionScope {
    /// Create a value expression scope from parser flags.
    pub(super) fn from_flags(flags: ParserFlags) -> Self {
        Self {
            flags,
            minimum_precedence: None,
            owns_newline_call_boundary: false,
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

    /// Return whether this expression is parsed in statement position.
    #[inline]
    pub(super) fn is_statement_position(self) -> bool {
        self.flags.is_in_statement_position()
    }

    /// Return whether an outer ternary or match case owns `:`.
    #[inline]
    pub(super) fn owns_colon_boundary(self) -> bool {
        self.flags.is_in_ternary_condition() || self.flags.is_in_match_case()
    }

    /// Return whether newline terminates a match case body expression.
    #[inline]
    pub(super) fn is_match_case_body(self) -> bool {
        self.flags.is_in_match_case_body()
    }

    /// Return whether the expression is the receiver of `new`.
    #[inline]
    pub(super) fn is_new_receiver(self) -> bool {
        self.flags.is_in_new_receiver()
    }

    /// Return whether tree syntax owns `<` continuations.
    #[inline]
    pub(super) fn is_tree_literal(self) -> bool {
        self.flags.is_in_tree_literal()
    }

    /// Return whether an outer for-each construct owns its continuation.
    #[inline]
    pub(super) fn owns_for_each_boundary(self) -> bool {
        self.flags.is_in_for_each()
    }

    /// Return whether this expression is the operand of `typeof`.
    #[inline]
    pub(super) fn is_typeof_query(self) -> bool {
        self.flags.is_in_typeof_query()
    }

    /// Return whether an outer statement boundary owns newline calls.
    #[inline]
    pub(super) fn owns_newline_call_boundary(self) -> bool {
        self.owns_newline_call_boundary
    }

    /// Return whether newline terminates decorator target parsing.
    #[inline]
    pub(super) fn owns_decorator_line_boundary(self) -> bool {
        self.flags.is_in_decorator()
    }

    /// Return whether static closers terminate this expression.
    #[inline]
    pub(super) fn is_static(self) -> bool {
        self.flags.is_in_static()
    }

    /// Return true when this scope stops before an operator.
    pub(super) fn stops_before(self, operator: ExpressionInfixOperator) -> bool {
        self.stops_before_precedence(operator.precedence(), operator.is_right_associative())
    }

    /// Return true when this scope stops before a binding power.
    #[inline]
    pub(super) fn stops_before_precedence(
        self,
        precedence: u16,
        is_right_associative: bool,
    ) -> bool {
        let Some(minimum_precedence) = self.minimum_precedence else {
            return false;
        };

        if is_right_associative {
            minimum_precedence > precedence
        } else {
            minimum_precedence >= precedence
        }
    }
}

impl TypeScope {
    /// Create a type expression scope from parser flags.
    pub(super) fn from_flags(flags: ParserFlags) -> Self {
        Self {
            flags,
            minimum_precedence: None,
        }
    }

    /// Return this scope with a different minimum precedence.
    pub(super) fn at_precedence(mut self, minimum_precedence: Option<u16>) -> Self {
        self.minimum_precedence = minimum_precedence;

        self
    }

    /// Return whether an outer construct owns `:`.
    #[inline]
    pub(super) fn owns_colon_boundary(self) -> bool {
        self.flags.is_in_ternary_condition()
            || self.flags.is_in_match_case()
            || self.flags.is_in_type_conditional_right()
    }

    /// Return whether conditional types are disabled in this scope.
    #[inline]
    pub(super) fn disallows_conditional(self) -> bool {
        self.flags.is_disallow_type_conditional()
    }

    /// Return whether an outer heritage clause owns `implements`.
    #[inline]
    pub(super) fn stops_before_implements(self) -> bool {
        self.flags.is_in_before_block() || self.flags.is_in_super_type()
    }

    /// Return whether static closers terminate this type expression.
    #[inline]
    pub(super) fn is_static(self) -> bool {
        self.flags.is_in_static()
    }

    /// Return whether constructor arguments belong to an enclosing `new` expression.
    #[inline]
    pub(super) fn is_new_receiver(self) -> bool {
        self.flags.is_in_new_receiver()
    }

    /// Return true when this scope stops before an operator.
    pub(super) fn stops_before(self, operator: TypeInfixOperator) -> bool {
        self.minimum_precedence
            .is_some_and(|minimum_precedence| minimum_precedence >= operator.precedence())
    }
}
