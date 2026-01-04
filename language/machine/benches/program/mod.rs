pub(crate) mod arithmetic;
pub(crate) mod calls;
pub(crate) mod dispatch;
pub(crate) mod intrinsics;
pub(crate) mod memory;
mod program;

#[allow(unused_imports)]
pub(crate) use program::{Program, print_stats, quick_bench, quick_check, validate_all};
