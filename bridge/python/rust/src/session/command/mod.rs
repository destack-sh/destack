#[path = "check.generated.rs"]
mod check;
#[path = "format.generated.rs"]
mod format;
#[path = "lint.generated.rs"]
mod lint;
#[path = "parse.generated.rs"]
mod parse;
mod registry;

pub use check::*;
pub use format::*;
pub use lint::*;
pub use parse::*;
pub(crate) use registry::register;
