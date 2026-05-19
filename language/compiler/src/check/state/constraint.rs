use destack_dir as dir;

use super::{InferId, StaticInferId};

/// Operand used to bind a type inference value.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeOperand {
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
    Infer(InferId),
    /// Value type of a resolved symbol.
    ///
    /// ```ds
    /// source
    /// // the path expression reads the value type of symbol source
    /// ```
    SymbolValue(dir::GlobalSymbolId),
    /// Type produced by static evaluation.
    ///
    /// ```ds
    /// type Element = Vector<T>.Element;
    /// // Element is computed from a static type expression
    /// ```
    Static(StaticInferId),
}

/// Relation or selection the solver must resolve.
#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
    /// Make two type variables equal.
    ///
    /// ```ds
    /// let value: int32 = source;
    /// // type(source) == int32
    /// ```
    Equals {
        /// The left type.
        left: InferId,
        /// The right type.
        right: InferId,
    },
    /// Bind a type variable to a type operand.
    ///
    /// ```ds
    /// 1
    /// // type(1) has type literal 1
    /// ```
    BindType {
        /// The type variable to bind.
        result: InferId,
        /// The type operand.
        operand: TypeOperand,
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
        result: InferId,
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
        result: InferId,
        /// The input types.
        values: Vec<InferId>,
    },
    /// Require a variable to be at least as wide as a bound.
    ///
    /// ```ds
    /// const value = id("x");
    /// // "x" flows upward into T
    /// ```
    LowerBound {
        /// The inferred variable.
        variable: InferId,
        /// The lower bound.
        bound: InferId,
    },
    /// Require a variable to fit within a bound.
    ///
    /// ```ds
    /// const value: string | number = id("x");
    /// // T must stay within string | number
    /// ```
    UpperBound {
        /// The inferred variable.
        variable: InferId,
        /// The upper bound.
        bound: InferId,
    },
    /// Resolve a member access.
    ///
    /// ```ds
    /// point.x
    /// // resolve x against type(point)
    /// ```
    Member {
        /// The member access node.
        node: dir::GlobalNodeIdAny,
        /// The receiver type.
        receiver: InferId,
        /// The member key.
        key: dir::StaticKey,
        /// The selected member type.
        result: InferId,
    },
    /// Resolve a call expression.
    ///
    /// ```ds
    /// parse(text)
    /// // resolve the callable selected by parse(text)
    /// ```
    Call {
        /// The call node.
        node: dir::GlobalNodeIdAny,
        /// The callee type.
        callee: InferId,
        /// The argument types.
        arguments: Vec<InferId>,
        /// The call result type.
        result: InferId,
    },
    /// Resolve a unary operator expression.
    ///
    /// ```ds
    /// !enabled
    /// // resolve the logical not operation
    /// ```
    UnaryOperator {
        /// The operator node.
        node: dir::GlobalNodeIdAny,
        /// The source operator.
        operator: dir::UnaryOperator,
        /// The operand type.
        operand: InferId,
        /// The operation result type.
        result: InferId,
    },
    /// Resolve a binary operator expression.
    ///
    /// ```ds
    /// left + right
    /// // resolve the add operation
    /// ```
    BinaryOperator {
        /// The operator node.
        node: dir::GlobalNodeIdAny,
        /// The source operator.
        operator: dir::BinaryOperator,
        /// The left operand type.
        left: InferId,
        /// The right operand type.
        right: InferId,
        /// The operation result type.
        result: InferId,
    },
}
