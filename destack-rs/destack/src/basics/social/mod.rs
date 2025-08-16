//! destack.basics.social@2025.08.15.1

#![destack::partial(destack.basics.social, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::basics::social::notification::NotificationStatus;

pub mod _gen;
pub mod follow;
pub mod notification;
pub mod reaction;
pub mod star;
