mod associated;
mod dynamic;
pub(in crate::lower) mod form;
mod intrinsic;
mod lower;
mod nominal;
mod object;
mod scalar;
mod signature;
mod template;

pub(in crate::lower) use lower::TypeLowerer;
pub(in crate::lower) use nominal::*;
pub(in crate::lower) use template::{BoundReceiver, GenericScope};
