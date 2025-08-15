//! imagination/style@2025.08.15.1

#![destack::partial(imagination/style, file)]

pub use gradient::*;
pub use color::*;
pub use style::*;
pub use palette::*;
pub use font::*;
pub use stroke::*;
pub use theme::*;
pub use fill::*;
pub use shadow::*;
pub use border::*;

mod gradient;
mod color;
mod style;
mod palette;
mod font;
mod stroke;
mod theme;
mod fill;
mod shadow;
mod border;