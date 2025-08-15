//! destack.basics.social@2025.08.15.1

#![destack::partial(destack.basics.social, file)]
#![allow(unused_imports)]

pub use crate::basics::social::_gen::*;
pub use crate::basics::social::follow::*;
pub use crate::basics::social::notification::*;
pub use crate::basics::social::reaction::*;
pub use crate::basics::social::star::*;

mod _gen;
mod follow;
mod notification;
mod reaction;
mod star;
