mod awaited;
mod conditional;
mod instantiation;
mod intersection;
mod intrinsic;
mod key;
mod literal;
mod memory;
mod narrow;
mod newtype;
mod operation;
mod scalar;
mod r#static;
mod substitute;
mod r#try;
mod r#type;
mod r#typeof;
mod union;

pub(in crate::sema) use union::NullishPart;

pub(in crate::sema) use key::{InvalidOperation, OperationReduction};
pub(in crate::sema) use memory::BorrowConversion;
pub(in crate::sema) use substitute::{InferSubstitution, TypeSubstitution};
pub(in crate::sema) use r#try::TryProjection;
pub(in crate::sema) use r#type::ProjectionMemo;
