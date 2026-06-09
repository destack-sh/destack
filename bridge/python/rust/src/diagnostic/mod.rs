#[path = "diagnostic.generated.rs"]
mod diagnostic;
#[path = "edit.generated.rs"]
mod edit;
mod registry;

pub use diagnostic::*;
pub use edit::*;
pub(crate) use registry::register;
