use destack_dir::{Asynchrony, OperatorPrecedence};

/// The interpretation of `yield` in one function position.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub(crate) enum YieldContext {
    /// Treat `yield` as an identifier.
    #[default]
    Identifier,
    /// Parse `yield` as a generator expression.
    Expression,
    /// Reject `yield` in this position.
    Forbidden,
}

/// The interpretation of `await` in one function position.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub(crate) enum AwaitContext {
    /// Parse `await` as an asynchronous expression.
    #[default]
    Expression,
    /// Reject `await` in this position.
    Forbidden,
}

/// Function rules inherited by nested grammar positions.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub(crate) struct FunctionContext {
    /// The active `yield` interpretation.
    pub(crate) yield_context: YieldContext,
    /// The active `await` interpretation.
    pub(crate) await_context: AwaitContext,
}

impl FunctionContext {
    /// Return the rules active in parameter initializers.
    pub(crate) fn enter_parameters(self, asynchrony: Asynchrony, is_generator: bool) -> Self {
        Self {
            yield_context: if is_generator {
                YieldContext::Forbidden
            } else {
                self.yield_context
            },
            await_context: if asynchrony == Asynchrony::Async {
                AwaitContext::Forbidden
            } else {
                self.await_context
            },
        }
    }

    /// Create the rules active in a function body.
    pub(crate) fn body(asynchrony: Asynchrony, is_generator: bool) -> Self {
        Self {
            yield_context: if is_generator {
                YieldContext::Expression
            } else {
                YieldContext::Identifier
            },
            await_context: if asynchrony == Asynchrony::Async {
                AwaitContext::Expression
            } else {
                AwaitContext::Forbidden
            },
        }
    }
}

/// The relationship between an expression and its surrounding statement.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub(crate) enum StatementPosition {
    /// The expression is not owned by a statement parse.
    #[default]
    None,
    /// The expression is nested within a statement parse.
    Nested,
    /// The expression directly occupies a statement slot.
    Direct,
}

impl StatementPosition {
    /// Return the position inherited by one nested expression.
    pub(crate) fn nested(self) -> Self {
        match self {
            StatementPosition::None => StatementPosition::None,
            StatementPosition::Nested | StatementPosition::Direct => StatementPosition::Nested,
        }
    }
}

/// The interpretation of an opening brace at an expression head.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub(crate) enum BraceContext {
    /// Parse an object expression.
    #[default]
    Object,
    /// Parse a block expression.
    Block,
}

/// The grammar mode of one value expression.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub(crate) enum ExpressionMode {
    /// Parse an ordinary value expression.
    #[default]
    Value,
    /// Parse a value embedded in tree syntax.
    Tree,
    /// Parse the constructor receiver before its argument list.
    NewReceiver,
    /// Parse the operand of a `typeof` query.
    TypeofQuery,
}

/// Token ownership inherited by infix operands.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub(crate) struct ExpressionStops(u8);

impl ExpressionStops {
    /// A colon owned by an enclosing match case.
    pub(crate) const MATCH_COLON: Self = Self(1 << 0);
    /// An `in` or `of` token owned by an enclosing iteration clause.
    pub(crate) const FOR_EACH: Self = Self(1 << 1);
    /// A newline owned by an enclosing match case.
    pub(crate) const MATCH_LINE: Self = Self(1 << 2);
    /// An angle close owned by an enclosing generic argument list.
    pub(crate) const ANGLE_CLOSE: Self = Self(1 << 3);
    /// A newline call owned by an enclosing statement expression.
    pub(crate) const NEWLINE_CALL: Self = Self(1 << 4);
    /// A brace reserved for an enclosing control body during recovery.
    pub(crate) const BODY_BRACE: Self = Self(1 << 5);
    /// A colon owned by an enclosing conditional expression.
    pub(crate) const CONDITIONAL_COLON: Self = Self(1 << 6);
    /// A question owned by an enclosing iterative conditional ladder.
    pub(crate) const CONDITIONAL_QUESTION: Self = Self(1 << 7);

    /// Return these stops with one stop enabled.
    pub(crate) const fn with(self, stop: Self) -> Self {
        Self(self.0 | stop.0)
    }

    /// Return these stops with one stop disabled.
    pub(crate) const fn without(self, stop: Self) -> Self {
        Self(self.0 & !stop.0)
    }

    /// Return whether these stops contain one stop.
    pub(crate) const fn contains(self, stop: Self) -> bool {
        self.0 & stop.0 != 0
    }
}

/// Decorator ownership surrounding one grammar position.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub(crate) enum DecoratorContext {
    /// No decorator is being parsed.
    #[default]
    None,
    /// A decorator value is being parsed.
    Value,
    /// The first decorator path segment is being parsed.
    Head,
}

impl DecoratorContext {
    /// Return the decorator context inherited by one nested expression.
    const fn nested(self) -> Self {
        match self {
            DecoratorContext::Head => DecoratorContext::Value,
            context => context,
        }
    }
}

/// Rules for one value-expression parse.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub(crate) struct ExpressionContext {
    /// Function rules inherited by this expression.
    pub(crate) function: FunctionContext,
    /// The relationship to the surrounding statement.
    pub(crate) statement: StatementPosition,
    /// The interpretation of an opening brace at the expression head.
    pub(crate) brace: BraceContext,
    /// Decorator ownership surrounding this expression.
    pub(crate) decorator: DecoratorContext,
    /// The grammar mode of this expression.
    pub(crate) mode: ExpressionMode,
    /// Token ownership inherited by operator operands.
    pub(crate) stops: ExpressionStops,
    /// The minimum accepted infix precedence.
    pub(crate) minimum_precedence: OperatorPrecedence,
}

impl ExpressionContext {
    /// Create context for one for-each iterator value.
    pub(crate) fn for_each(
        function: FunctionContext,
        has_header_close: bool,
        is_block_start: bool,
    ) -> Self {
        let stops = if has_header_close || is_block_start {
            ExpressionStops::default()
        } else {
            ExpressionStops::BODY_BRACE
        };

        Self {
            function,
            stops,
            ..Self::default()
        }
    }

    /// Return context for an expression nested by an explicit grammar construct.
    pub(crate) fn nested(self) -> Self {
        Self {
            function: self.function,
            statement: self.statement.nested(),
            decorator: self.decorator.nested(),
            ..Self::default()
        }
    }

    /// Return context for the right operand of one infix operation.
    pub(crate) fn right(self, minimum_precedence: OperatorPrecedence) -> Self {
        Self {
            statement: self.statement.nested(),
            brace: BraceContext::Object,
            decorator: self.decorator.nested(),
            minimum_precedence,
            ..self
        }
    }
}

/// The grammar mode of one type expression.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub(crate) enum TypeMode {
    /// Parse an ordinary type expression.
    #[default]
    Type,
    /// Parse the receiver of a `new` expression.
    NewReceiver,
    /// Parse an arrow return annotation.
    ArrowReturn,
}

/// Conditional-type behavior in one type position.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub(crate) enum ConditionalTypeContext {
    /// Parse conditional types.
    #[default]
    Allowed,
    /// Leave conditional continuations to the enclosing grammar.
    Forbidden,
}

/// The interpretation of `extends` following an inferred type.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub(crate) enum InferExtends {
    /// Allow `extends` to open a surrounding conditional type.
    #[default]
    Conditional,
    /// Attach `extends` as the inferred type constraint.
    Constraint,
}

/// Token ownership inherited by type infix operands.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub(crate) struct TypeStops(u8);

impl TypeStops {
    /// A colon owned by an enclosing conditional type.
    pub(crate) const CONDITIONAL_COLON: Self = Self(1 << 0);
    /// An `implements` token owned by an enclosing heritage clause.
    pub(crate) const IMPLEMENTS: Self = Self(1 << 1);
    /// An angle close owned by an enclosing generic argument list.
    pub(crate) const ANGLE_CLOSE: Self = Self(1 << 2);

    /// Return these stops with one stop enabled.
    pub(crate) const fn with(self, stop: Self) -> Self {
        Self(self.0 | stop.0)
    }

    /// Return whether these stops contain one stop.
    pub(crate) const fn contains(self, stop: Self) -> bool {
        self.0 & stop.0 != 0
    }
}

/// Rules for one type-expression parse.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub(crate) struct TypeContext {
    /// Function rules inherited by embedded value expressions.
    pub(crate) function: FunctionContext,
    /// Decorator ownership surrounding this type.
    pub(crate) decorator: DecoratorContext,
    /// The grammar mode of this type expression.
    pub(crate) mode: TypeMode,
    /// Conditional-type behavior in this position.
    pub(crate) conditional: ConditionalTypeContext,
    /// The interpretation of `extends` following `infer`.
    pub(crate) infer_extends: InferExtends,
    /// Token ownership inherited by operator operands.
    pub(crate) stops: TypeStops,
    /// The minimum accepted infix precedence.
    pub(crate) minimum_precedence: OperatorPrecedence,
}

impl TypeContext {
    /// Return context for a type nested by an explicit grammar construct.
    pub(crate) fn nested(self) -> Self {
        Self {
            function: self.function,
            decorator: self.decorator.nested(),
            conditional: self.conditional,
            ..Self::default()
        }
    }

    /// Return context for the right operand of one infix operation.
    pub(crate) const fn right(self, minimum_precedence: OperatorPrecedence) -> Self {
        Self {
            decorator: self.decorator.nested(),
            minimum_precedence,
            ..self
        }
    }
}

impl From<ExpressionContext> for TypeContext {
    /// Convert value-expression rules into an embedded type-expression context.
    fn from(context: ExpressionContext) -> Self {
        let stops = if context.stops.contains(ExpressionStops::ANGLE_CLOSE) {
            TypeStops::default().with(TypeStops::ANGLE_CLOSE)
        } else {
            TypeStops::default()
        };

        Self {
            function: context.function,
            stops,
            ..Self::default()
        }
    }
}

/// Rules for one pattern parse.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub(crate) struct PatternContext {
    /// Function rules inherited by embedded default expressions.
    pub(crate) function: FunctionContext,
    /// Whether `|` ends this pattern operand.
    pub(crate) stops_at_union: bool,
    /// Whether the pattern belongs to a match case.
    pub(crate) is_match_case: bool,
    /// Whether the pattern is immediately followed by a type annotation.
    pub(crate) is_before_type: bool,
}

impl PatternContext {
    /// Return context for a pattern nested by explicit delimiters.
    pub(crate) const fn nested(self) -> Self {
        Self {
            function: self.function,
            is_match_case: self.is_match_case,
            stops_at_union: false,
            is_before_type: false,
        }
    }
}

/// The language space surrounding one dynamic parameter list.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub(crate) enum ParameterSpace {
    /// Parameters belong to a value declaration.
    #[default]
    Value,
    /// Parameters belong to a type expression.
    Type,
}

/// Rules for one dynamic parameter list.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub(crate) struct ParameterContext {
    /// Function rules inherited by parameter defaults.
    pub(crate) function: FunctionContext,
    /// The relationship to a surrounding statement.
    pub(crate) statement: StatementPosition,
    /// The language space surrounding the parameters.
    pub(crate) space: ParameterSpace,
    /// The type grammar mode used by annotations.
    pub(crate) type_mode: TypeMode,
}
