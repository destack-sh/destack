mod handle;
mod options;
mod root;

pub use destack_heap::{GcState, GcStats};
pub use handle::*;
pub use options::*;
pub(crate) use root::*;
pub use root::{RootSet, RootSink};
