mod image;
mod large;
mod location;
mod reference;
pub(crate) mod space;
mod span;

pub use image::SharedManagedSpaceImage;
pub use large::SharedManagedLargeEntryImage;
pub(crate) use large::*;
pub(crate) use location::*;
pub use reference::SharedManagedReference;
pub use space::SharedManagedSpace;
pub use span::SharedManagedSmallSpanImage;
pub(crate) use span::*;
