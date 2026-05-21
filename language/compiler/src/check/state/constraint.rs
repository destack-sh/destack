use destack_dir as dir;

use super::{StaticInferId, TypeInferId};

/// Term used to bind a type inference value.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum TypeTerm {
    /// Concrete type.
    ///
    /// ```ds
    /// 1
    /// // the literal can directly produce type 1
    /// ```
    Type(dir::Type),
    /// Type inferred somewhere else in the same check state.
    ///
    /// ```ds
    /// let value = source;
    /// // type(value) can point at type(source)
    /// ```
    Infer(TypeInferId),
    /// Type of a resolved symbol.
    ///
    /// ```ds
    /// source
    /// // the path expression reads the value type of symbol source
    /// ```
    Symbol(dir::GlobalSymbolId),
    /// Type produced by static evaluation.
    ///
    /// ```ds
    /// type Element = Vector<T>.Element;
    /// // Element is computed from a static type expression
    /// ```
    Static(StaticInferId),
    /// Source type expression normalized by the solver.
    ///
    /// ```ds
    /// let value: Box<T>;
    /// // the type expression is walked now and evaluated later
    /// ```
    TypeExpression(dir::GlobalNodeId<dir::TypeExpression>),
    /// Function type assembled from solved signature pieces.
    ///
    /// ```ds
    /// function add(left: int32): int32
    /// // the function type is built after parameter types are solved
    /// ```
    Function {
        /// The source node for the constructed type.
        source: dir::GlobalNodeIdAny,
        /// The function asynchrony.
        asynchrony: dir::Asynchrony,
        /// The parameter types.
        parameters: Vec<TypeInferId>,
        /// The return type.
        return_type: Option<TypeInferId>,
        /// Whether this function is a generator.
        is_generator: bool,
    },
}

/// Relation or selection the solver must resolve.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Constraint {
    /// Make two type variables equal.
    ///
    /// ```ds
    /// let value: int32 = source;
    /// // type(source) == int32
    /// ```
    Equals {
        /// The left type.
        left: TypeInferId,
        /// The right type.
        right: TypeInferId,
    },
    /// Bind a type variable to a type term.
    ///
    /// ```ds
    /// 1
    /// // type(1) has type literal 1
    /// ```
    Bind {
        /// The type variable to bind.
        result: TypeInferId,
        /// The type term.
        term: TypeTerm,
    },
    /// Bind a static variable to a static value.
    ///
    /// ```ds
    /// Array<T, 4>
    /// // bind N to 4
    /// ```
    BindStatic {
        /// The static result.
        result: StaticInferId,
        /// The static value.
        value: dir::StaticTerm,
    },
    /// Instantiate a generic declaration with static arguments.
    ///
    /// ```ds
    /// Box<int32>
    /// // instantiate Box with int32
    /// ```
    Instantiate {
        /// The instantiated type.
        result: TypeInferId,
        /// The generic symbol.
        symbol: dir::GlobalSymbolId,
        /// The static arguments.
        arguments: Vec<StaticInferId>,
    },
    /// Join branch-local types into one type.
    ///
    /// ```ds
    /// condition ? left : right
    /// // type(result) is the join of type(left) and type(right)
    /// ```
    Join {
        /// The joined result type.
        result: TypeInferId,
        /// The input types.
        values: Vec<TypeInferId>,
    },
    /// Require a variable to be at least as wide as a bound.
    ///
    /// ```ds
    /// const value = id("x");
    /// // "x" flows upward into T
    /// ```
    LowerBound {
        /// The inferred variable.
        variable: TypeInferId,
        /// The lower bound.
        bound: TypeInferId,
    },
    /// Require a variable to fit within a bound.
    ///
    /// ```ds
    /// const value: string | number = id("x");
    /// // T must stay within string | number
    /// ```
    UpperBound {
        /// The inferred variable.
        variable: TypeInferId,
        /// The upper bound.
        bound: TypeInferId,
    },
    /// Resolve a member access.
    ///
    /// ```ds
    /// point.x
    /// // resolve x against type(point)
    /// ```
    SelectMember {
        /// The member access node.
        node: dir::GlobalNodeIdAny,
        /// The receiver type.
        receiver: TypeInferId,
        /// The member key.
        key: dir::StaticKey,
        /// The selected member type.
        result: TypeInferId,
    },
    /// Resolve a call expression.
    ///
    /// ```ds
    /// parse(text)
    /// // resolve the callable selected by parse(text)
    /// ```
    SelectCall {
        /// The call node.
        node: dir::GlobalNodeIdAny,
        /// The callee type.
        callee: TypeInferId,
        /// The argument types.
        arguments: Vec<TypeInferId>,
        /// The call result type.
        result: TypeInferId,
    },
    /// Resolve a unary operator expression.
    ///
    /// ```ds
    /// !enabled
    /// // resolve the logical not operation
    /// ```
    SelectUnaryOperator {
        /// The operator node.
        node: dir::GlobalNodeIdAny,
        /// The source operator.
        operator: dir::UnaryOperator,
        /// The operand type.
        operand: TypeInferId,
        /// The operation result type.
        result: TypeInferId,
    },
    /// Resolve a binary operator expression.
    ///
    /// ```ds
    /// left + right
    /// // resolve the add operation
    /// ```
    SelectBinaryOperator {
        /// The operator node.
        node: dir::GlobalNodeIdAny,
        /// The source operator.
        operator: dir::BinaryOperator,
        /// The left operand type.
        left: TypeInferId,
        /// The right operand type.
        right: TypeInferId,
        /// The operation result type.
        result: TypeInferId,
    },
}
