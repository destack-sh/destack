//! presentation/view@2025.08.15.1

#![destack::partial(presentation/view, file)]

pub use relative::*;
pub use label::*;
pub use text::*;
pub use slider::*;
pub use input::*;
pub use frame::*;
pub use split::*;
pub use number::*;
pub use layout::*;
pub use content::*;
pub use view::*;

mod relative;
mod label;
mod text;
mod slider;
mod input;
mod frame;
mod split;
mod number;
mod layout;
mod content;
mod view;