/// One type relation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum Relation {
    /// Both operands solve to the same type.
    Equal,
    /// The left operand is assignable to the right operand.
    Assignable,
    /// The left operand is assignable to the right operand without influencing it.
    Writable,
    /// The left operand is castable to the right operand.
    Castable,
    /// The left operand satisfies the right operand without influencing it.
    Satisfies,
    /// The left operand extends the right operand.
    Extends,
    /// The left operand implements the right operand.
    Implements,
}
