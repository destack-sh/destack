mod awaited;
mod conditional;
mod intersection;
mod intrinsic;
mod key;
mod literal;
mod memory;
mod narrow;
mod operation;
mod scalar;
mod substitute;
mod r#try;
mod r#type;
mod r#typeof;
mod union;

pub(in crate::check) use key::{InvalidOperation, OperationReduction};
pub(in crate::check) use scalar::ScalarFamily;
pub(in crate::check) use substitute::TypeSubstitution;
