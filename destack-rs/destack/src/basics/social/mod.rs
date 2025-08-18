//! destack.basics.social

#![destack::partial(destack.basics.social, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::basics::social::notification::NotificationStatus;

pub mod _gen;
pub mod follow;
pub mod notification;
pub mod reaction;
pub mod star;
