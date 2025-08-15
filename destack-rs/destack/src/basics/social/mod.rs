//! basics/social@2025.08.15.1

#![destack::partial(basics/social, file)]

pub use star::*;
pub use notification::*;
pub use reaction::*;
pub use follow::*;

mod star;
mod notification;
mod reaction;
mod follow;