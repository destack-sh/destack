//! destack.basics.script.run@2025.08.15.1

#![destack::generated(destack.basics.script.run, file)]

use crate::{ActionType, FunctionOperator, MethodType, RunStatus};

#[destack::generated(FunctionOperator, Debug, block)]
impl std::fmt::Debug for FunctionOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FunctionOperator::Eq => write!(f, "EQ"),
            FunctionOperator::Neq => write!(f, "NEQ"),
            FunctionOperator::Gt => write!(f, "GT"),
            FunctionOperator::Lt => write!(f, "LT"),
            FunctionOperator::Gte => write!(f, "GTE"),
            FunctionOperator::Lte => write!(f, "LTE"),
            FunctionOperator::In => write!(f, "IN"),
            FunctionOperator::NotIn => write!(f, "NOT_IN"),
            FunctionOperator::Add => write!(f, "ADD"),
            FunctionOperator::Sub => write!(f, "SUB"),
            FunctionOperator::Mul => write!(f, "MUL"),
            FunctionOperator::Truediv => write!(f, "TRUEDIV"),
            FunctionOperator::Floordiv => write!(f, "FLOORDIV"),
            FunctionOperator::Mod => write!(f, "MOD"),
            FunctionOperator::Pow => write!(f, "POW"),
            FunctionOperator::Divmod => write!(f, "DIVMOD"),
            FunctionOperator::Pos => write!(f, "POS"),
            FunctionOperator::Neg => write!(f, "NEG"),
            FunctionOperator::Abs => write!(f, "ABS"),
            FunctionOperator::Invert => write!(f, "INVERT"),
            FunctionOperator::And => write!(f, "AND"),
            FunctionOperator::Or => write!(f, "OR"),
            FunctionOperator::Xor => write!(f, "XOR"),
            FunctionOperator::Lshift => write!(f, "LSHIFT"),
            FunctionOperator::Rshift => write!(f, "RSHIFT"),
            FunctionOperator::Bool => write!(f, "BOOL"),
            FunctionOperator::Not => write!(f, "NOT"),
            FunctionOperator::Len => write!(f, "LEN"),
            FunctionOperator::Getitem => write!(f, "GETITEM"),
            FunctionOperator::Setitem => write!(f, "SETITEM"),
            FunctionOperator::Delitem => write!(f, "DELITEM"),
            FunctionOperator::Contains => write!(f, "CONTAINS"),
            FunctionOperator::Iter => write!(f, "ITER"),
            FunctionOperator::Next => write!(f, "NEXT"),
        }
    }
}

#[destack::generated(MethodType, Debug, block)]
impl std::fmt::Debug for MethodType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MethodType::Instance => write!(f, "INSTANCE"),
            MethodType::Class => write!(f, "CLASS"),
            MethodType::Static => write!(f, "STATIC"),
        }
    }
}

#[destack::generated(ActionType, Debug, block)]
impl std::fmt::Debug for ActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionType::UnaryInUnaryOut => write!(f, "UNARY_IN_UNARY_OUT"),
            ActionType::UnaryInStreamOut => write!(f, "UNARY_IN_STREAM_OUT"),
            ActionType::StreamInUnaryOut => write!(f, "STREAM_IN_UNARY_OUT"),
            ActionType::StreamInStreamOut => write!(f, "STREAM_IN_STREAM_OUT"),
        }
    }
}

#[destack::generated(RunStatus, Debug, block)]
impl std::fmt::Debug for RunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunStatus::Scheduled => write!(f, "SCHEDULED"),
            RunStatus::Running => write!(f, "RUNNING"),
            RunStatus::Paused => write!(f, "PAUSED"),
            RunStatus::Yielded => write!(f, "YIELDED"),
            RunStatus::Cancelled => write!(f, "CANCELLED"),
            RunStatus::Aborted => write!(f, "ABORTED"),
            RunStatus::Failed => write!(f, "FAILED"),
            RunStatus::Completed => write!(f, "COMPLETED"),
        }
    }
}
