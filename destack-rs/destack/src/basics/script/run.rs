//! destack.basics.script.run@2025.08.15.1

#![destack::partial(destack.basics.script.run, file)]

#[destack::generated(FunctionOperator, -, block)]
/// The overridable builtin operators for Functions (depends on runtime language).
pub enum FunctionOperator {
    /// =
    Eq = 1,
    /// !=
    Neq = 2,
    /// >
    Gt = 3,
    /// <
    Lt = 4,
    /// >=
    Gte = 5,
    /// <=
    Lte = 6,
    /// in
    In = 7,
    /// not in
    NotIn = 8,
    /// +
    Add = 20,
    /// -
    Sub = 21,
    /// *
    Mul = 22,
    /// /
    Truediv = 23,
    /// //
    Floordiv = 24,
    /// %
    Mod = 25,
    /// **
    Pow = 26,
    /// divmod
    Divmod = 27,
    /// +
    Pos = 30,
    /// -
    Neg = 31,
    /// abs
    Abs = 32,
    /// ~
    Invert = 33,
    /// &
    And = 40,
    /// |
    Or = 41,
    /// ^
    Xor = 42,
    /// <<
    Lshift = 43,
    /// >>
    Rshift = 44,
    /// bool
    Bool = 50,
    /// not
    Not = 51,
    /// len
    Len = 60,
    /// []
    Getitem = 61,
    /// []=
    Setitem = 62,
    /// del []
    Delitem = 63,
    /// in
    Contains = 64,
    /// iter
    Iter = 65,
    /// next
    Next = 66,
}

#[destack::generated(MethodType, -, block)]
/// MethodType
pub enum MethodType {
    /// Instance method
    Instance = 1,
    /// Class method
    Class = 2,
    /// Static method
    Static = 3,
}

#[destack::generated(ActionType, -, block)]
/// ActionType
pub enum ActionType {
    /// Single in, single out
    UnaryInUnaryOut = 1,
    /// Single in, stream out
    UnaryInStreamOut = 2,
    /// Stream in, single out
    StreamInUnaryOut = 3,
    /// Stream in, stream out
    StreamInStreamOut = 4,
}

#[destack::generated(RunStatus, -, block)]
/// RunStatus
pub enum RunStatus {
    /// Scheduled for sometime
    Scheduled = 2,
    /// Actively running
    Running = 10,
    /// Paused manually
    Paused = 21,
    /// Yielded to someone
    Yielded = 23,
    /// Cancelled before running
    Cancelled = 51,
    /// Aborted while running
    Aborted = 52,
    /// Failed due to an error
    Failed = 53,
    /// Completed successfully
    Completed = 54,
}
