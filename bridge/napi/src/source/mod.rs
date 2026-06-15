#[path = "component.generated.rs"]
mod component;
#[path = "file.generated.rs"]
mod file;
#[path = "module.generated.rs"]
mod module;
#[path = "package.generated.rs"]
mod package;
#[path = "product.generated.rs"]
mod product;
#[path = "profile.generated.rs"]
mod profile;
#[path = "span.generated.rs"]
mod span;
#[path = "target.generated.rs"]
mod target;

pub use component::*;
pub use file::*;
pub use module::*;
pub use package::*;
pub use product::*;
pub use profile::*;
pub use span::*;
pub use target::*;
