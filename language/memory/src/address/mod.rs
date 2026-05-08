mod page;
mod space;
mod table;

pub use space::*;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use table::watch_page_write;
