#[cfg(target_vendor = "apple")]
mod apple;
#[cfg(not(target_vendor = "apple"))]
mod posix;
mod target;

pub(crate) use target::*;
