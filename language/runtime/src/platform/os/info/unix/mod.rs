#[cfg(target_vendor = "apple")]
mod apple;
mod backend;
#[cfg(not(target_vendor = "apple"))]
mod posix;

pub(crate) use backend::*;
