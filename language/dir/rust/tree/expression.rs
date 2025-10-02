#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Assignment (e.g., `x = y`).
    Assign,
    /// Assignment with operator (except direct assignment, e.g., `x += y`).
    AssignOp,
    /// Unary operation (except reference/dereference, e.g., `-x`).
    Unary,
	/// Reference operation (e.g., `&x`).
	Reference,
	/// Dereference operation (e.g., `*x`).
	Dereference,
    /// Binary operation.
    Binary,
    /// Drop locals.
    Drop,
    /// Field member access.
    Field,
    /// Call to a function.
    Call,
    /// Cast to a type.
    Cast,
    /// --------------------------------
    /// Literals.
    /// --------------------------------
    /// Literal value.
    Literal,
    /// Struct creation.
    Struct,
    /// Tuple creation.
    Tuple,
    /// Array creation.
    Array,
    /// --------------------------------
    /// Control flow.
    /// --------------------------------
    /// If expression.
    If,
	/// Loop expression.
	Loop,
    /// Match expression.
    Match,
    /// Closure expression.
    Closure,
}
